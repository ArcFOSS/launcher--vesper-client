//! The application shell: navigation sidebar plus the active page.

use std::sync::{Arc, Mutex};

use gpui::{AppContext, Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div, px};
use gpui::prelude::FluentBuilder;
use tokio::sync::mpsc;

use crate::ui::components::{ButtonVariant};
use crate::ui::instances_view::InstancesView;
use crate::ui::login_view::LoginView;
use crate::ui::navigation::Page;
use crate::ui::sidebar;
use crate::ui::theme::{TextSize, Theme, text_size, weights};

use vesper_core::bridge::commands::BackendCommand;
use vesper_core::bridge::updates::UiUpdate;

/// The app: owns the current page and hands state to whichever screen is on it.
pub struct App {
    page: Page,
    cmd_tx: mpsc::Sender<BackendCommand>,
    latest_update: Arc<Mutex<Option<UiUpdate>>>,
    login: Option<Entity<LoginView>>,
    instances: Option<Entity<InstancesView>>,
}

impl App {
    pub fn new(
        cmd_tx: mpsc::Sender<BackendCommand>,
        latest_update: Arc<Mutex<Option<UiUpdate>>>,
    ) -> Self {
        Self {
            page: Page::Login,
            cmd_tx,
            latest_update,
            login: None,
            instances: None,
        }
    }

    /// Switches the screen. Called from anywhere that holds a context for this
    /// view, which is how the sidebar and the sign-in screen both get here.
    pub fn navigate(&mut self, page: Page, cx: &mut Context<Self>) {
        self.page = page;
        cx.notify();
    }

    /// The sidebar: brand, nav entries, and nothing else.
    fn sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = Theme::native();
        div()
            .flex()
            .flex_col()
            .w(px(224.0))
            .h_full()
            .px(px(12.0))
            .pt(px(12.0))
            .pb(px(12.0))
            .gap(px(4.0))
            .bg(theme.colors.bg)
            .child(self.brand(theme))
            .when(self.page == Page::Instances, |this| {
                this.child(self.nav(cx, Page::Instances, "Instances", ButtonVariant::Outline))
            })
            .when(self.page == Page::Login, |this| {
                this.child(self.nav(cx, Page::Login, "Sign In", ButtonVariant::Outline))
            })
    }

    /// The wordmark: the product name, set at display size and extra-bold.
    fn brand(&self, theme: Theme) -> impl IntoElement {
        use gpui::Hsla;

        div()
            .text_size(px(text_size(TextSize::Display)))
            .font_weight(weights::EXTRABOLD)
            .text_color(Hsla::from(theme.colors.fg))
            .child("Vesper")
    }

    /// One nav entry; the active page reads as filled, the rest as outlined.
    fn nav<'a>(
        &mut self,
        cx: &'a mut Context<Self>,
        page: Page,
        label: &str,
        resting_variant: ButtonVariant,
    ) -> impl IntoElement + use<'a> {
        let theme = Theme::native();
        let variant = if self.page == page {
            ButtonVariant::Secondary
        } else {
            resting_variant
        };
        let id = if self.page == page {
            "sidebar-nav-active"
        } else {
            "sidebar-nav"
        };
        sidebar::nav_button(
            id,
            label.to_string(),
            theme,
            variant,
            cx.listener(move |this, _event, _window, cx| this.navigate(page, cx)),
        )
    }

    fn login_entity(&mut self, cx: &mut Context<Self>) -> Entity<LoginView> {
        if self.login.is_none() {
            let cmd_tx = self.cmd_tx.clone();
            let latest_update = self.latest_update.clone();
            let app = cx.entity();
            self.login = Some(cx.new::<LoginView>(|_cx| LoginView::new(cmd_tx, latest_update, app)));
        }
        self.login.clone().expect("the login entity was just created")
    }

    fn instances_entity(&mut self, cx: &mut Context<Self>) -> Entity<InstancesView> {
        if self.instances.is_none() {
            let cmd_tx = self.cmd_tx.clone();
            let latest_update = self.latest_update.clone();
            self.instances = Some(cx.new::<InstancesView>(|_cx| InstancesView::new(cmd_tx, latest_update)));
        }
        self.instances.clone().expect("the instances entity was just created")
    }

    fn content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        match self.page {
            Page::Login => self.login_entity(cx).into_any_element(),
            Page::Instances => self.instances_entity(cx).into_any_element(),
        }
    }
}

impl Render for App {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = Theme::native();
        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.bg)
            .child(self.sidebar(cx))
            .child(self.content(cx))
    }
}
