//! The sign-in screen.
//!
//! Rebuilt in Ely's visual language: a warm page, one elevated panel, one
//! primary action. The auth bridge below is unchanged — the `!Send` `AuthManager`
//! still runs on a dedicated OS thread with its own tokio runtime, and results
//! still arrive through `Arc<Mutex<Option<UiUpdate>>>`.
//!
//! Nothing here owns UI state the backend also owns: the view holds only what
//! it has actually been told.

use std::sync::{Arc, Mutex};

use anyhow::{Context as _, Result};
use gpui::{
    Context, Entity, Hsla, IntoElement, ParentElement, Render, SharedString, Styled, Window,
    div, px,
    prelude::FluentBuilder,
};
use tokio::sync::mpsc;

use vesper_core::auth::AuthManager;
use vesper_core::bridge::commands::BackendCommand;
use vesper_core::bridge::updates::UiUpdate;

use crate::ui::components::badge::{BadgeTone, badge};
use crate::ui::components::{Button, ButtonVariant, Card, Dots, caption, error_message};
use crate::ui::theme::{ControlSize, TextSize, Theme, text_size, weights};

/// Microsoft's public client id for the Minecraft launcher.
const MS_CLIENT_ID: &str = "c36a9fb6-4f2a-41ff-90bd-ae7cc92031eb";

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
}

impl LoginView {
    /// A signed-out screen wired to the backend's update channel.
    pub fn new(
        cmd_tx: mpsc::Sender<BackendCommand>,
        latest_update: Arc<Mutex<Option<UiUpdate>>>,
    ) -> Self {
        Self {
            stage: Stage::Idle,
            username: SharedString::default(),
            auth_error: SharedString::default(),
            theme: Theme::default(),
            loader: None,
            cmd_tx,
            latest_update,
        }
    }

    /// Folds in one backend update, if one is waiting. Returns true if it
    /// changed anything, so the caller can skip a redundant repaint.
    fn poll_updates(&mut self) -> bool {
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
        self.apply(update);
        true
    }

    /// Moves to the stage `update` describes.
    fn apply(&mut self, update: UiUpdate) {
        match update {
            UiUpdate::AuthBrowserLoginStarted => {
                self.stage = Stage::AwaitingBrowser;
                self.auth_error = SharedString::default();
            }
            UiUpdate::AuthSuccess { profile } => {
                self.stage = Stage::SignedIn;
                self.username = profile.display_name.into();
                self.auth_error = SharedString::default();
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

        let update = self.latest_update.clone();

        // The auth flow runs on a dedicated thread with its own current-thread
        // runtime: `AuthManager` is `!Send`, and GPUI 0.2.2's `spawn` cannot
        // carry it across the async boundary. See STACK.md.
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build the auth runtime");

            runtime.block_on(async move {
                if let Err(error) = run_auth_flow(&update).await {
                    set_update(
                        &update,
                        UiUpdate::AuthError {
                            message: format!("Auth failed: {error:#}"),
                        },
                    );
                }
            });
        });
    }

    /// Drops the session on screen and asks the backend to drop its tokens.
    fn handle_logout(&mut self, cx: &mut Context<Self>) {
        let _ = self.cmd_tx.try_send(BackendCommand::AuthLogout);
        self.stage = Stage::Idle;
        self.username = SharedString::default();
        self.auth_error = SharedString::default();
        cx.notify();
    }

    /// The dots entity, started on first use.
    fn loader(&mut self, cx: &mut Context<Self>) -> Entity<Dots> {
        self.loader
            .get_or_insert_with(|| Dots::new(cx))
            .clone()
    }
}

fn set_update(target: &Arc<Mutex<Option<UiUpdate>>>, update: UiUpdate) {
    if let Ok(mut guard) = target.lock() {
        *guard = Some(update);
    }
}

async fn run_auth_flow(update: &Arc<Mutex<Option<UiUpdate>>>) -> Result<()> {
    let auth_manager = AuthManager::new(MS_CLIENT_ID.to_string())
        .context("Failed to initialize auth manager")?;

    let (auth_url, csrf_state, pkce_verifier, port) = auth_manager
        .start_browser_login()
        .await
        .context("Failed to start browser login")?;

    open::that(auth_url.as_str()).context("Failed to open browser")?;

    let tokens = auth_manager
        .complete_browser_login(port, &pkce_verifier, &csrf_state)
        .await
        .context("Browser login completion failed")?;

    let mc_profile = tokens
        .mc_profile
        .as_ref()
        .context("No Minecraft profile in auth response")?;

    let profile = vesper_core::auth::models::AuthProfile {
        id: mc_profile.id.clone(),
        display_name: mc_profile.name.clone(),
        email: None,
        account_type: vesper_core::auth::models::AccountType::Java,
        minecraft_username: Some(mc_profile.name.clone()),
        minecraft_uuid: Some(mc_profile.id.clone()),
        xbox_gamertag: None,
        bedrock_profile: None,
        device_keys: None,
        identity_keys: None,
    };

    set_update(update, UiUpdate::AuthSuccess { profile });
    Ok(())
}

/// The page: a warm field with `content` centered on it.
fn page(theme: Theme, content: impl gpui::IntoElement) -> impl gpui::IntoElement {
    div()
        .flex()
        .size_full()
        .items_center()
        .justify_center()
        .bg(theme.colors.bg)
        .child(content)
}

/// The sign-in panel: an elevated card, one column, evenly spaced.
fn panel(theme: Theme, content: impl gpui::IntoElement) -> impl gpui::IntoElement {
    Card::elevated()
        .render(
            theme,
            div()
                .flex()
                .flex_col()
                .w(px(PANEL_WIDTH))
                .gap(px(16.0))
                .p(px(32.0))
                .child(content),
        )
}

/// The wordmark. Ely sets display type extra-bold and close-tracked.
fn wordmark(theme: Theme) -> impl gpui::IntoElement {
    div()
        .text_size(px(text_size(TextSize::Xxl)))
        .font_weight(weights::EXTRABOLD)
        .text_color(Hsla::from(theme.colors.fg))
        .child("Vesper Client")
}

impl Render for LoginView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let changed = self.poll_updates();
        if changed {
            cx.notify();
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
    fn sign_in(&mut self, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .child(wordmark(theme))
            .child(
                div()
                    .text_size(px(text_size(TextSize::Base)))
                    .text_color(Hsla::from(theme.colors.fg_muted))
                    .child("Sign in with your Microsoft account."),
            )
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

    /// Either the call to action, or the wait for the browser to come back.
    fn primary_action(&mut self, cx: &mut Context<Self>) -> impl gpui::IntoElement {
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
    fn awaiting(&mut self, cx: &mut Context<Self>) -> impl gpui::IntoElement {
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
            .child(caption(
                "Finish signing in there, then come back here.",
                theme,
            ))
    }

    /// The signed-in state: who is here, and the way out.
    fn signed_in(&mut self, cx: &mut Context<Self>) -> impl gpui::IntoElement {
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