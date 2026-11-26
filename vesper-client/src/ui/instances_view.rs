//! The instances screen: the library, the create form, and per-instance management.

use std::sync::{Arc, Mutex};

use gpui::{
    Context, Div, FocusHandle, Hsla, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, SharedString, Styled, Window, div, px,
};
use tokio::sync::mpsc;

use crate::ui::components::badge::{BadgeTone, badge};
use crate::ui::components::button::{Button, ButtonVariant};
use crate::ui::components::card::Card;
use crate::ui::components::empty_state::{caption, empty_state, error_message};
use crate::ui::theme::{ControlSize, Radius, TextSize, Theme, radius, text_size, weights};

use vesper_core::bridge::commands::{BackendCommand, ModSource};
use vesper_core::bridge::updates::{InstalledModInfo, UiUpdate};
use vesper_core::config::LoaderType;
use vesper_core::config::instance::InstanceInfo;
use vesper_core::mods::ModInfo;

/// Which field a physical keyboard is currently filling.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Field {
    Rename,
    Search,
    CreateName,
    CreateVersion,
}

/// The instances screen: library list, create form, instance management.
pub struct InstancesView {
    instances: Vec<InstanceInfo>,
    is_loading: bool,
    error: SharedString,
    selected: Option<InstanceInfo>,
    did_load: bool,
    theme: Theme,

    cmd_tx: mpsc::Sender<BackendCommand>,
    latest_update: Arc<Mutex<Option<UiUpdate>>>,

    // The create form.
    show_create: bool,
    form_name: SharedString,
    form_mc_version: SharedString,
    form_loader: SharedString,
    form_loader_version: SharedString,

    // The rename and search fields.
    rename_value: SharedString,
    rename_focus: Option<FocusHandle>,
    search_query: SharedString,
    search_focus: Option<FocusHandle>,
    create_focus: Option<FocusHandle>,
    search_source: ModSource,
    search_results: Vec<ModInfo>,
    is_searching: bool,
    downloading: bool,
    installed_mods: Vec<InstalledModInfo>,
    mod_error: SharedString,
    pending_rename: Option<String>,
}

impl InstancesView {
    /// A screen wired to the backend's command and update channels.
    pub fn new(
        cmd_tx: mpsc::Sender<BackendCommand>,
        latest_update: Arc<Mutex<Option<UiUpdate>>>,
    ) -> Self {
        Self {
            instances: Vec::new(),
            is_loading: true,
            error: SharedString::default(),
            selected: None,
            did_load: false,
            theme: Theme::native(),
            cmd_tx,
            latest_update,
            show_create: false,
            form_name: SharedString::default(),
            form_mc_version: SharedString::default(),
            form_loader: SharedString::from("vanilla"),
            form_loader_version: SharedString::default(),
            rename_value: SharedString::default(),
            rename_focus: None,
            search_query: SharedString::default(),
            search_focus: None,
            create_focus: None,
            search_source: ModSource::Modrinth,
            search_results: Vec::new(),
            is_searching: false,
            downloading: false,
            installed_mods: Vec::new(),
            mod_error: SharedString::default(),
            pending_rename: None,
        }
    }

    /// Folds in one backend update, if one is waiting. Returns true if it
    /// changed anything, so the caller can skip a redundant repaint.
    fn poll(&mut self) -> bool {
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

    fn apply(&mut self, update: UiUpdate) {
        match update {
            UiUpdate::InstanceListResult(list) => {
                self.instances = list;
                self.is_loading = false;
                self.error = SharedString::default();
                self.did_load = true;
            }
            UiUpdate::InstanceCreated { .. } => {
                self.show_create = false;
                self.did_load = true;
            }
            UiUpdate::InstanceDeleted { id } => {
                self.instances.retain(|i| i.id != id);
                if self.selected.as_ref().is_some_and(|i| i.id == id) {
                    self.selected = None;
                }
            }
            UiUpdate::InstanceRenamed { id } => {
                let renamed = self.pending_rename.take();
                if let Some(name) = renamed {
                    for instance in &mut self.instances {
                        if instance.id == id {
                            instance.config.name = name.clone();
                        }
                    }
                    if let Some(instance) = self.selected.as_mut() {
                        if instance.id == id {
                            instance.config.name = name.clone();
                        }
                    }
                }
            }
            UiUpdate::InstanceError { message } => {
                self.error = message.into();
                self.is_loading = false;
            }
            UiUpdate::ModSearchResults(results) => {
                self.search_results = results;
                self.is_searching = false;
                self.mod_error = SharedString::default();
            }
            UiUpdate::ModDownloadComplete { .. } => {
                self.downloading = false;
                self.load_installed();
            }
            UiUpdate::ModListResult(mods) => {
                self.installed_mods = mods;
            }
            UiUpdate::ModError { message } => {
                self.mod_error = message.into();
                self.is_searching = false;
                self.downloading = false;
            }
            _ => {}
        }
    }

    fn load_instances(&mut self) {
        let _ = self.cmd_tx.send(BackendCommand::InstanceList);
    }

    fn load_installed(&mut self) {
        if let Some(instance) = self.selected.clone() {
            let _ = self.cmd_tx.send(BackendCommand::ModListInstalled {
                instance_id: instance.id.clone(),
            });
        }
    }

    fn toggle_create(&mut self) {
        self.show_create = !self.show_create;
    }

    fn submit_create(&mut self) {
        let name = self.form_name.as_str().trim().to_string();
        if name.is_empty() {
            self.error = "Give your instance a name.".into();
            return;
        }
        let mc_version = self.form_mc_version.as_str().trim().to_string();
        if mc_version.is_empty() {
            self.error = "Pick a Minecraft version.".into();
            return;
        }
        let loader = LoaderType::from_str_loose(self.form_loader.as_str());
        let loader_version = if self.form_loader_version.as_str().trim().is_empty() {
            None
        } else {
            Some(self.form_loader_version.as_str().trim().to_string())
        };
        let _ = self.cmd_tx.send(BackendCommand::InstanceCreate {
            name,
            mc_version,
            loader,
            loader_version,
        });
    }

    fn rename_instance(&mut self, instance_id: &str) {
        let name = self.rename_value.as_str().trim().to_string();
        if name.is_empty() || self.selected.as_ref().is_some_and(|i| i.config.name == name) {
            return;
        }
        self.pending_rename = Some(name.clone());
        let _ = self.cmd_tx.send(BackendCommand::InstanceRename {
            id: instance_id.to_string(),
            name,
        });
    }

    fn open_instance(&mut self, instance_id: &str) {
        if let Some(instance) = self.instances.iter().find(|i| i.id == instance_id).cloned() {
            self.selected = Some(instance);
            self.mod_error = SharedString::default();
            self.search_results.clear();
            self.installed_mods.clear();
            self.is_searching = false;
            self.downloading = false;
            self.load_installed();
        }
    }

    fn delete_instance(&mut self, instance_id: &str) {
        let _ = self.cmd_tx.send(BackendCommand::InstanceDelete {
            id: instance_id.to_string(),
        });
    }

    fn search_mods(&mut self) {
        let query = self.search_query.as_str().trim().to_string();
        if query.is_empty() || self.is_searching || self.downloading {
            return;
        }
        self.is_searching = true;
        let _ = self.cmd_tx.send(BackendCommand::ModSearch {
            query,
            source: self.search_source,
        });
    }

    fn install_mod(&mut self, project_id: &str, file_id: &str, file_name: &str, source: ModSource) {
        if self.downloading {
            return;
        }
        self.downloading = true;
        let instance_id = self.selected.clone().map(|i| i.id).unwrap_or_default();
        let _ = self.cmd_tx.send(BackendCommand::ModDownload {
            instance_id,
            project_id: project_id.to_string(),
            file_id: file_id.to_string(),
            file_name: file_name.to_string(),
            source,
        });
    }

    fn toggle_source(&mut self) {
        self.search_source = if self.search_source == ModSource::Modrinth {
            ModSource::CurseForge
        } else {
            ModSource::Modrinth
        };
    }

    fn append_char(&mut self, field: Field, key_char: Option<String>) {
        let Some(ch) = key_char.and_then(|c| c.chars().next().filter(|c| c.is_alphanumeric())) else {
            return;
        };
        let text = match field {
            Field::Rename => self.rename_value.as_str().to_string(),
            Field::Search => self.search_query.as_str().to_string(),
            Field::CreateName => self.form_name.as_str().to_string(),
            Field::CreateVersion => self.form_mc_version.as_str().to_string(),
        };
        let mut text = text;
        text.push(ch);
        match field {
            Field::Rename => self.rename_value = SharedString::from(text),
            Field::Search => self.search_query = SharedString::from(text),
            Field::CreateName => self.form_name = SharedString::from(text),
            Field::CreateVersion => self.form_mc_version = SharedString::from(text),
        }
    }
}

impl Render for InstancesView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = self.theme;
        let changed = self.poll();
        if changed {
            cx.notify();
        }
        if !self.did_load {
            self.did_load = true;
            self.load_instances();
        }

        let content = if self.selected.is_some() {
            self.manage(cx).into_any_element()
        } else {
            self.library(cx).into_any_element()
        };

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.colors.bg)
            .child(content)
    }
}

impl InstancesView {
    /// The library: header, then the list, the create form, or an empty state.
    fn library(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.library_header(cx))
            .child(self.library_body(cx))
    }

    fn library_header(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .px(px(24.0))
            .pt(px(24.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Lg)))
                            .font_weight(weights::SEMIBOLD)
                            .text_color(Hsla::from(theme.colors.fg))
                            .child("Instances"),
                    )
                    .child(caption(format!("{} installed", self.instances.len()), theme)),
            )
            .child(
                Button::new(ButtonVariant::Primary)
                    .size(ControlSize::Md)
                    .on_click(
                        "create-instance",
                        "New Instance",
                        theme,
                        cx.listener(|this, _event, _window, _cx| this.toggle_create()),
                    )
                    .into_any_element(),
            )
    }

    fn library_body(&mut self, cx: &mut Context<Self>) -> Div {
        if self.show_create {
            return self.create_form(cx);
        }

        let theme = self.theme;

        if self.is_loading {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .w_full()
                .h_full()
                .child(caption("Loading instances…", theme));
        }

        if !self.error.is_empty() {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .w_full()
                .h_full()
                .gap(px(12.0))
                .child(error_message(self.error.clone(), theme))
                .child(
                    Button::new(ButtonVariant::Outline)
                        .size(ControlSize::Md)
                        .full_width()
                        .on_click(
                            "retry-instances",
                            "Retry",
                            theme,
                            cx.listener(|this, _event, _window, _cx| this.load_instances()),
                        )
                        .into_any_element(),
                );
        }

        if self.instances.is_empty() {
            return div()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .w_full()
                .h_full()
                .child(
                    empty_state(
                        "No instances yet.",
                        "Create your first instance to start playing.",
                        theme,
                    ),
                )
                .child(
                    Button::new(ButtonVariant::Primary)
                        .size(ControlSize::Md)
                        .full_width()
                        .on_click(
                            "create-instance-empty",
                            "Create Instance",
                            theme,
                            cx.listener(|this, _event, _window, _cx| this.toggle_create()),
                        )
                        .into_any_element(),
                );
        }

        let instances = self.instances.clone();
        div()
            .flex()
            .flex_col()
            .w_full()
            .px(px(24.0))
            .py(px(8.0))
            .gap(px(8.0))
            .children(instances.iter().map(|i| self.instance_card(cx, i.clone())))
    }

    /// The create form: one card, a column, the fields, and the actions.
    fn create_form(&mut self, cx: &mut Context<Self>) -> Div {
        let theme = self.theme;
        let focus = self.create_focus.clone().unwrap_or_else(|| cx.focus_handle());
        self.create_focus = Some(focus.clone());
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(
                Card::new().render(
                    theme,
                    div()
                        .flex()
                        .flex_col()
                        .gap(px(16.0))
                        .child(
                            div()
                                .text_size(px(text_size(TextSize::Lg)))
                                .font_weight(weights::SEMIBOLD)
                                .text_color(Hsla::from(theme.colors.fg))
                                .child("New Instance"),
                        )
                        .child(self.create_field(cx, Field::CreateName, &focus))
                        .child(self.create_field(cx, Field::CreateVersion, &focus))
                        .child(self.create_actions(cx)),
                ),
            )
    }

    /// One labelled text field of the create form.
    fn create_field(&mut self, cx: &mut Context<Self>, field: Field, focus: &FocusHandle) -> impl IntoElement {
        let theme = self.theme;
        let value = match field {
            Field::CreateName => self.form_name.clone(),
            Field::CreateVersion => self.form_mc_version.clone(),
            _ => SharedString::default(),
        };
        let label = match field {
            Field::CreateName => "Instance name",
            Field::CreateVersion => "Minecraft version",
            _ => "Version",
        };
        let id = match field {
            Field::CreateName => "create-instance-name",
            Field::CreateVersion => "create-instance-version",
            _ => "create-instance-version",
        };
        div()
            .flex()
            .flex_col()
            .gap(px(6.0))
            .child(
                div()
                    .text_size(px(text_size(TextSize::Sm)))
                    .font_weight(weights::MEDIUM)
                    .text_color(Hsla::from(theme.colors.fg_muted))
                    .child(label),
            )
            .child(
                div()
                    .id(id)
                    .track_focus(focus)
                    .key_context("vesper.instances.create")
                    .on_key_down(
                        cx.listener(move |this, event: &KeyDownEvent, _window, _cx| {
                            this.append_char(field, event.keystroke.key_char.clone())
                        }),
                    )
                    .w_full()
                    .px(px(12.0))
                    .py(px(8.0))
                    .rounded(px(radius(Radius::Md)))
                    .bg(theme.colors.sunken)
                    .border_1()
                    .border_color(Hsla::from(theme.colors.border))
                    .text_size(px(text_size(TextSize::Base)))
                    .text_color(Hsla::from(theme.colors.fg))
                    .child(value),
            )
    }

    fn create_actions(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_row()
            .justify_between()
            .gap(px(8.0))
            .child(
                Button::new(ButtonVariant::Outline)
                    .size(ControlSize::Md)
                    .on_click(
                        "create-instance-cancel",
                        "Cancel",
                        theme,
                        cx.listener(|this, _event, _window, _cx| this.toggle_create()),
                    )
                    .into_any_element(),
            )
            .child(
                Button::new(ButtonVariant::Primary)
                    .size(ControlSize::Md)
                    .on_click(
                        "create-instance-submit",
                        "Create",
                        theme,
                        cx.listener(|this, _event, _window, _cx| this.submit_create()),
                    )
                    .into_any_element(),
            )
    }

    fn instance_card(&mut self, cx: &mut Context<Self>, instance: InstanceInfo) -> impl IntoElement {
        let theme = self.theme;
        let instance_id = instance.id.clone();
        let delete_id = instance.id.clone();
        Card::new().render(
            theme,
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap(px(12.0))
                .w_full()
                .child(self.instance_meta(&instance, theme))
                .child(
                    div()
                        .flex()
                        .flex_row()
                        .gap(px(8.0))
                        .child(
                            Button::new(ButtonVariant::Outline)
                                .size(ControlSize::Sm)
                                .on_click(
                                    "open-instance",
                                    "Manage",
                                    theme,
                                    cx.listener(move |this, _event, _window, _cx| {
                                        this.open_instance(&instance_id)
                                    }),
                                )
                                .into_any_element(),
                        )
                        .child(
                            Button::new(ButtonVariant::Danger)
                                .size(ControlSize::Sm)
                                .on_click(
                                    "delete-instance",
                                    "Delete",
                                    theme,
                                    cx.listener(move |this, _event, _window, _cx| {
                                        this.delete_instance(&delete_id)
                                    }),
                                )
                                .into_any_element(),
                        ),
                ),
        )
    }

    fn instance_meta(&self, instance: &InstanceInfo, theme: Theme) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap(px(6.0))
            .child(badge(
                instance.config.loader_type.to_string(),
                BadgeTone::Neutral,
                theme,
            ))
            .child(badge(
                format!("Minecraft {}", instance.config.mc_version),
                BadgeTone::Neutral,
                theme,
            ))
    }

    /// The instance management screen: back, header, rename, mods.
    fn manage(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .size_full()
            .child(self.back_button(cx))
            .child(self.manage_header())
            .child(self.rename_card(cx))
            .child(self.mods_card(cx))
    }

    fn back_button(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        div()
            .w_full()
            .px(px(24.0))
            .pt(px(16.0))
            .child(
                Button::new(ButtonVariant::Ghost)
                    .size(ControlSize::Md)
                    .on_click(
                        "back-to-instances",
                        "Back to Instances",
                        theme,
                        cx.listener(|this, _event, _window, cx| {
                            this.selected = None;
                            cx.notify();
                        }),
                    )
                    .into_any_element(),
            )
    }

    fn manage_header(&mut self) -> impl IntoElement {
        let theme = self.theme;
        let instance = self.selected.clone().expect("manage renders a selected instance");
        div()
            .w_full()
            .px(px(24.0))
            .pt(px(8.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Lg)))
                            .font_weight(weights::SEMIBOLD)
                            .text_color(Hsla::from(theme.colors.fg))
                            .child(instance.config.name.clone()),
                    )
                    .child(self.instance_meta(&instance, theme)),
            )
    }

    fn rename_card(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        Card::new().render(
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(12.0))
                .child(self.rename_label())
                .child(self.rename_field(cx))
                .child(self.rename_save(cx)),
        )
    }

    fn rename_label(&self) -> impl IntoElement {
        let theme = self.theme;
        div()
            .text_size(px(text_size(TextSize::Sm)))
            .font_weight(weights::MEDIUM)
            .text_color(Hsla::from(theme.colors.fg_muted))
            .child("Name")
    }

    fn rename_field(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let value = self.rename_value.clone();
        let focus = self.rename_focus.clone().unwrap_or_else(|| cx.focus_handle());
        self.rename_focus = Some(focus.clone());
        div()
            .id("instance-rename-field")
            .track_focus(&focus)
            .key_context("vesper.instances.rename")
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _window, _cx| {
                    this.append_char(Field::Rename, event.keystroke.key_char.clone())
                }),
            )
            .w_full()
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(radius(Radius::Md)))
            .bg(theme.colors.sunken)
            .border_1()
            .border_color(Hsla::from(theme.colors.border))
            .text_size(px(text_size(TextSize::Base)))
            .text_color(Hsla::from(theme.colors.fg))
            .child(value)
    }

    fn rename_save(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let instance_id = self.selected.clone().map(|i| i.id).unwrap_or_default();
        Button::new(ButtonVariant::Primary)
            .size(ControlSize::Md)
            .full_width()
            .on_click(
                "save-instance-name",
                "Save Name",
                theme,
                cx.listener(move |this, _event, _window, _cx| {
                    let id = instance_id.clone();
                    this.rename_instance(&id);
                }),
            )
            .into_any_element()
    }

    fn mods_card(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        Card::new().render(
            theme,
            div()
                .flex()
                .flex_col()
                .gap(px(16.0))
                .child(self.mods_header())
                .child(self.search_row(cx))
                .child(self.mod_results(cx))
                .child(self.installed_list(cx)),
        )
    }

    fn mods_header(&mut self) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .w_full()
            .child(
                div()
                    .text_size(px(text_size(TextSize::Lg)))
                    .font_weight(weights::SEMIBOLD)
                    .text_color(Hsla::from(theme.colors.fg))
                    .child("Mods"),
            )
            .child(caption(format!("{} installed", self.installed_mods.len()), theme))
    }

    fn search_row(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_row()
            .gap(px(8.0))
            .child(self.search_field(cx))
            .child(self.source_button(cx))
            .child(self.search_button(cx))
    }

    fn search_field(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        let value = self.search_query.clone();
        let focus = self.search_focus.clone().unwrap_or_else(|| cx.focus_handle());
        self.search_focus = Some(focus.clone());
        div()
            .id("mod-search-field")
            .track_focus(&focus)
            .key_context("vesper.instances.mods")
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _window, _cx| {
                    this.append_char(Field::Search, event.keystroke.key_char.clone())
                }),
            )
            .w(px(220.0))
            .px(px(12.0))
            .py(px(8.0))
            .rounded(px(radius(Radius::Md)))
            .bg(theme.colors.sunken)
            .border_1()
            .border_color(Hsla::from(theme.colors.border))
            .text_size(px(text_size(TextSize::Base)))
            .text_color(Hsla::from(theme.colors.fg))
            .child(value)
    }

    fn source_button(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        Button::new(ButtonVariant::Subtle)
            .size(ControlSize::Sm)
            .full_width()
            .on_click(
                "toggle-mod-source",
                format!("Source: {}", self.search_source),
                theme,
                cx.listener(|this, _event, _window, _cx| this.toggle_source()),
            )
            .into_any_element()
    }

    fn search_button(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        Button::new(ButtonVariant::Primary)
            .size(ControlSize::Sm)
            .on_click(
                "search-mods",
                "Search",
                theme,
                cx.listener(|this, _event, _window, _cx| this.search_mods()),
            )
            .into_any_element()
    }

    fn mod_results(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        if self.downloading {
            return div().child(caption("Downloading…", theme));
        }
        if !self.mod_error.is_empty() {
            return div().child(error_message(self.mod_error.clone(), theme));
        }
        if self.is_searching {
            return div().child(caption("Searching…", theme));
        }
        if self.search_results.is_empty() {
            return div().child(caption("Search for a mod to install.", theme));
        }
        let results = self.search_results.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(results.iter().map(|m| self.mod_result_row(cx, m.clone())))
    }

    fn mod_result_row(&mut self, cx: &mut Context<Self>, m: ModInfo) -> impl IntoElement {
        let theme = self.theme;
        let file_name = format!("{}.jar", m.name);
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Md)))
                            .font_weight(weights::SEMIBOLD)
                            .text_color(Hsla::from(theme.colors.fg))
                            .child(m.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Sm)))
                            .text_color(Hsla::from(theme.colors.fg_muted))
                            .child(m.description.clone()),
                    ),
            )
            .child(
                Button::new(ButtonVariant::Outline)
                    .size(ControlSize::Sm)
                    .on_click(
                        "install-mod",
                        "Install",
                        theme,
                        cx.listener(move |this, _event, _window, _cx| {
                            this.install_mod(&m.project_id, &m.file_id, &file_name, m.source)
                        }),
                    )
                    .into_any_element(),
            )
    }

    fn installed_list(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme;
        if self.installed_mods.is_empty() {
            return div().child(caption("No mods installed yet.", theme));
        }
        let mods = self.installed_mods.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(8.0))
            .children(mods.iter().map(|m| self.installed_row(cx, m)))
    }

    fn installed_row(&mut self, _cx: &mut Context<Self>, m: &InstalledModInfo) -> impl IntoElement {
        let theme = self.theme;
        div()
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(12.0))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(px(2.0))
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Md)))
                            .font_weight(weights::SEMIBOLD)
                            .text_color(Hsla::from(theme.colors.fg))
                            .child(m.name.clone()),
                    )
                    .child(
                        div()
                            .text_size(px(text_size(TextSize::Sm)))
                            .text_color(Hsla::from(theme.colors.fg_muted))
                            .child(m.version.clone()),
                    ),
            )
            .child(caption(format!("via {}", m.source), theme))
    }
}
