//! The sign-in screen.
//!
//! Rebuilt in Ely's visual language: a warm page, one elevated panel, one
//! primary action. The auth bridge now lives in the backend command worker; the
//! screen only sends [`BackendCommand::AuthStartBrowserLogin`] /
//! [`BackendCommand::AuthLogout`] and folds [`UiUpdate`]s into stages.

use std::sync::{Arc, Mutex};

use gpui::{
    Context, Entity, Hsla, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    px, prelude::FluentBuilder,
};
use tokio::sync::mpsc;

use crate::ui::app::App;
use crate::ui::components::badge::{BadgeTone, badge};
use crate::ui::components::{Button, ButtonVariant, Card, Dots, caption, error_message};
use crate::ui::navigation::Page;
use crate::ui::theme::{ControlSize, TextSize, Theme, text_size, weights};

use vesper_core::bridge::commands::BackendCommand;
use vesper_core::bridge::updates::UiUpdate;

/// The panel's width, in px. Wide enough for a sentence, narrow enough to read
/// as a dialog rather than a page.
const PANEL_WIDTH: f32 = 400.0;

/// Where sign-in currently stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Stage {
    /// Nothing attempted yet.
    #[default]
    Idle,
    /// The browser is open; waiting for the callback.
    AwaitingBrowser,
    /// A profile came back.
    SignedIn,
}

/// The sign-in screen.
pub struct LoginView {
    stage: Stage,
    username: SharedString,
    auth_error: SharedString,
    theme: Theme,
    /// Created on first render so the animation only runs while it is on screen.
    loader: Option<Entity<Dots>>,
    cmd_tx: mpsc::Sender<BackendCommand>,
    latest_update: Arc<Mutex<Option<UiUpdate>>>,
    /// Lets the screen hand off to the instance library once signed in.
    entity: Entity<App>,
    did_load: bool,
}

impl LoginView {
    /// A signed-out screen wired to the backend's update channel.
    pub fn new(
        cmd_tx: mpsc::Sender<BackendCommand>,
        latest_update: Arc<Mutex<Option<UiUpdate>>>,
        entity: Entity<App>,
    ) -> Self {
        Self {
            stage: Stage::Idle,
            username: SharedString::default(),
            auth_error: SharedString::default(),
            theme: Theme::native(),
            loader: None,
            cmd_tx,
            latest_update,
            entity,
            did_load: false,
        }
    }

    /// The polling state: one backend update per frame cycle, if any.
    fn poll_updates(&mut self, cx: &mut Context<Self>) -> bool {
        // The guard is released before `self` is touched: holding it across the
        // mutation would borrow `self` twice.
        let pending = {
            let Ok(mut guard) = self.latest_update.lock() else {
                return false;
            };
            guard.take()
        };
        let Some(update) = pending else {
            return false;
        };
        self.apply(update, cx);
        true
    }

    /// Asks the backend whether a persisted session exists.
    fn check_session(&mut self) {
        let _ = self.cmd_tx.try_send(BackendCommand::AuthGetStatus);
    }

    /// Moves to the stage `update` describes, and where the app should be.
    fn apply(&mut self, update: UiUpdate, cx: &mut Context<Self>) {
        match update {
            UiUpdate::AuthBrowserLoginStarted => {
                self.stage = Stage::AwaitingBrowser;
                self.auth_error = SharedString::default();
            }
            UiUpdate::AuthStatus { is_logged_in, profile } => {
                if is_logged_in {
                    self.stage = Stage::SignedIn;
                    if let Some(profile) = profile {
                        self.username = profile.display_name.into();
                    }
                    // A persisted session is the fastest way back in.
                    let _ = self.entity.update(cx, |app, cx| app.navigate(Page::Instances, cx));
                }
            }
            UiUpdate::AuthSuccess { profile } => {
                self.stage = Stage::SignedIn;
                self.username = profile.display_name.into();
                self.auth_error = SharedString::default();
                // Signed in: the library is the destination from here on.
                let _ = self.entity.update(cx, |app, cx| app.navigate(Page::Instances, cx));
            }
            UiUpdate::AuthError { message } => {
                self.stage = Stage::Idle;
                self.auth_error = message.into();
            }
            UiUpdate::AuthLoggedOut => {
                self.stage = Stage::Idle;
                self.username = SharedString::default();
            }
            _ => {}
        }
    }

    /// Opens the browser and starts waiting for the callback.
    fn handle_login(&mut self, _cx: &mut Context<Self>) {
        self.stage = Stage::AwaitingBrowser;
        self.auth_error = SharedString::default();

        let _ = self.cmd_tx.try_send(BackendCommand::AuthStartBrowserLogin {
            account_type: "Java".into(),
        });
    }

    /// Drops the session on screen and asks the backend to drop its tokens.
    fn handle_logout(&mut self, cx: &mut Context<Self>) {
        let _ = self.cmd_tx.try_send(BackendCommand::AuthLogout);
        self.stage = Stage::Idle;
        self.username = SharedString::default();
        self.auth_error = SharedString::default();
        cx.notify();
        let _ = self.entity.update(cx, |app, cx| app.navigate(Page::Login, cx));
    }

    /// The dots entity, started on first use.
    fn loader(&mut self, cx: &mut Context<Self>) -> Entity<Dots> {
        self.loader
            .get_or_insert_with(|| Dots::new(cx))
            .clone()
    }
}

/// The page: a warm field with `content` centered on it.
fn page(theme: Theme, content: impl IntoElement) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .size_full()
        .items_center()
        .justify_center()
        .gap(px(24.0))
        .bg(theme.colors.bg)
        .child(content)
}

/// The sign-in panel: an elevated card, one column, evenly spaced.
fn panel(theme: Theme, content: impl IntoElement) -> impl IntoElement {
    Card::elevated()
        .render(
            theme,
            div()
                .flex()
                .flex_col()
                .items_center()
                .w(px(PANEL_WIDTH))
                .gap(px(16.0))
                .p(px(32.0))
                .child(content),
        )
}

/// The wordmark. Ely sets display type extra-bold and close-tracked.
fn wordmark(theme: Theme) -> impl IntoElement {
    div()
        .text_size(px(text_size(TextSize::Xxl)))
        .font_weight(weights::EXTRABOLD)
        .text_align(gpui::TextAlign::Center)
        .text_color(Hsla::from(theme.colors.fg))
        .child("Vesper Client")
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let changed = self.poll_updates(cx);
        if changed {
            cx.notify();
        }
        if !self.did_load {
            self.did_load = true;
            self.check_session();
        }

        let theme = self.theme;
        let body = match self.stage {
            Stage::SignedIn => self.signed_in(cx).into_any_element(),
            _ => self.sign_in(cx).into_any_element(),
        };
        page(theme, panel(theme, body))
    }
}

impl LoginView {
    /// Idle or awaiting the browser: the wordmark, one action, and whatever the
    /// backend last said.
    fn sign_in(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(wordmark(theme))
            .child(self.subtitle(theme, "Sign in with your Microsoft account."))
            .child(div().h(px(8.0)))
            .child(self.primary_action(cx))
            .when(!self.auth_error.is_empty(), |this| {
                this.child(
                    div()
                        .pt(px(8.0))
                        .child(error_message(self.auth_error.clone(), theme)),
                )
            })
    }

    /// The screen's secondary line: what is happening, in quiet type.
    fn subtitle(&self, theme: Theme, text: impl Into<SharedString>) -> impl IntoElement {
        let text = text.into();
        div()
            .text_size(px(text_size(TextSize::Base)))
            .text_align(gpui::TextAlign::Center)
            .text_color(Hsla::from(theme.colors.fg_muted))
            .child(text)
    }

    /// Either the call to action, or the wait for the browser to come back.
    fn primary_action(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        if self.stage == Stage::AwaitingBrowser {
            return self.awaiting(cx).into_any_element();
        }

        Button::new(ButtonVariant::Primary)
            .size(ControlSize::Lg)
            .full_width()
            .on_click(
                "login",
                "Continue with Microsoft",
                theme,
                cx.listener(|this, _event, _window, cx| this.handle_login(cx)),
            )
            .into_any_element()
    }

    /// The polling state: three dots, one sentence, one hint.
    fn awaiting(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let loader = self.loader(cx);
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap(px(12.0))
            .py(px(8.0))
            .child(div().h(px(8.0)).child(loader))
            .child(
                div()
                    .text_size(px(text_size(TextSize::Base)))
                    .font_weight(weights::MEDIUM)
                    .text_color(Hsla::from(theme.colors.fg))
                    .child("Waiting for your browser…"),
            )
            .child(caption("Finish signing in there, then come back here.", theme))
    }

    /// The signed-in state: who is here, and the way out.
    fn signed_in(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(wordmark(theme))
            .child(
                div()
                    .text_size(px(text_size(TextSize::Base)))
                    .text_color(Hsla::from(theme.colors.fg_muted))
                    .child("You are signed in."),
            )
            .child(
                div()
                    .flex()
                    .justify_center()
                    .child(badge(
                        self.username.clone(),
                        BadgeTone::Success,
                        theme,
                    )),
            )
            .child(
                Button::danger()
                    .size(ControlSize::Md)
                    .full_width()
                    .on_click(
                        "logout",
                        "Sign out",
                        theme,
                        cx.listener(|this, _event, _window, cx| this.handle_logout(cx)),
                    ),
            )
    }
}
