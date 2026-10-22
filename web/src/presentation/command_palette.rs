//! Search and navigation only; commands remain owned by their mounted controls.
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    combobox::{Combobox, ComboboxEmpty, ComboboxOption},
    command::{Command, CommandRegistry, use_commands},
    dialog::{Dialog, DialogDescription, DialogTitle},
    kbd::Kbd,
};
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, closure::Closure};

#[derive(Clone, Copy, PartialEq)]
pub(super) struct PaletteState {
    pub open: Signal<bool>,
    opener: Signal<Option<web_sys::HtmlElement>>,
    pending: Signal<Option<Command>>,
}

pub(super) fn use_palette_state() -> PaletteState {
    PaletteState {
        open: use_signal(|| false),
        opener: use_signal(|| None),
        pending: use_signal(|| None),
    }
}

fn popover_opener(active: &web_sys::HtmlElement) -> Option<web_sys::HtmlElement> {
    let popup = active
        .closest("[data-slot=popover-content], .board-popover-content")
        .ok()??;
    let id = popup.id();
    if id.is_empty() {
        return None;
    }
    let document = active.owner_document()?;
    let controls = document.query_selector_all("[aria-controls]").ok()?;
    let mut found = None;
    for index in 0..controls.length() {
        let Some(control) = controls
            .item(index)
            .and_then(|node| node.dyn_into::<web_sys::HtmlElement>().ok())
        else {
            continue;
        };
        let rect = control.get_bounding_client_rect();
        if !control
            .get_attribute("aria-controls")
            .is_some_and(|ids| ids.split_ascii_whitespace().any(|value| value == id))
            || !control.is_connected()
            || control.matches(":disabled").ok()?
            || !(control.tab_index() >= 0
                || control.has_attribute("tabindex")
                || control.is_content_editable())
            || control
                .closest("[hidden], [inert], [aria-hidden=true]")
                .ok()?
                .is_some()
            || rect.width() <= 0.0
            || rect.height() <= 0.0
            || document
                .default_view()?
                .get_computed_style(&control)
                .ok()??
                .get_property_value("visibility")
                .ok()?
                != "visible"
        {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(control);
    }
    found
}

impl PaletteState {
    pub fn show(mut self) {
        if (self.open)() {
            return;
        }
        let active = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
        // Opening the palette dismisses Popovers and can unmount their focused fields.
        self.opener
            .set(active.as_ref().and_then(popover_opener).or(active.clone()));
        self.pending.set(None);
        if let Some(active) = active
            && matches!(active.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        {
            let _ = active.blur();
        }
        self.open.set(true);
    }

    fn close(mut self, command: Option<Command>) {
        if !*self.open.peek() {
            return;
        }
        self.pending.set(command);
        self.open.set(false);
    }

    fn finish_close(mut self) {
        if *self.open.peek() {
            return;
        }
        let opener = (self.opener)();
        spawn(async move {
            // The content has unmounted; let its DOM removal flush before moving focus.
            gloo_timers::future::TimeoutFuture::new(0).await;
            if *self.open.peek() {
                return;
            }
            let target = opener.filter(|e| e.is_connected()).or_else(|| {
                web_sys::window()
                    .and_then(|w| w.document())
                    .and_then(|d| d.get_element_by_id("header-mobile-trigger"))
                    .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok())
            });
            if let Some(target) = target {
                let _ = target.focus();
            }
            let command = self.pending.write().take();
            if let Some(command) = command {
                command.on_select.call(());
            }
        });
    }
}

#[component]
fn PaletteLifetime(on_closed: EventHandler<()>) -> Element {
    use_drop(move || on_closed.call(()));
    rsx! {}
}

fn sync_palette_host(host: &web_sys::HtmlDialogElement, open: bool) {
    if !host.is_connected() {
        return;
    }
    if open
        && !host.open()
        && host
            .query_selector("[data-slot=combobox-input]")
            .ok()
            .flatten()
            .is_some()
    {
        let _ = host.show_modal();
    } else if !open && host.open() {
        host.close();
    }
}

#[component]
pub(super) fn CommandPalette(state: PaletteState) -> Element {
    let host = use_hook(|| Rc::new(RefCell::new(None::<web_sys::HtmlDialogElement>)));
    use_effect({
        let host = host.clone();
        move || {
            let open = (state.open)();
            if let Some(host) = host.borrow().as_ref() {
                sync_palette_host(host, open);
            }
        }
    });
    use_drop({
        let host = host.clone();
        move || {
            if let Some(host) = host.borrow_mut().take()
                && host.is_connected()
                && host.open()
            {
                host.close();
            }
        }
    });
    let mount_host = host.clone();
    let content_host = host.clone();
    let content_mounted = onmounted(move |event: MountedEvent| {
        if let Some(content) = event.data().downcast::<web_sys::Element>()
            && let Some(host) = content_host.borrow().as_ref()
            && host
                .query_selector("[data-slot=dialog-content]")
                .ok()
                .flatten()
                .as_ref()
                == Some(content)
        {
            sync_palette_host(host, *state.open.peek());
        }
    });
    let registry = use_context::<CommandRegistry>();
    let registered_commands = use_memo(move || {
        if (state.open)() {
            registry.commands()
        } else {
            Vec::new()
        }
    });
    let mut query = use_signal(String::new);
    let open = (state.open)();
    let on_show = use_callback(move |()| state.show());
    let on_escape = use_callback(move |()| state.close(None));
    let on_closed = use_callback(move |()| state.finish_close());
    use_effect(move || {
        if !(state.open)() {
            query.set(String::new());
        }
    });
    let listener = use_hook(move || {
        web_sys::window()
            .and_then(|w| w.document())
            .map(|document| {
                let callback = Closure::wrap(Box::new(move |event: web_sys::KeyboardEvent| {
                    if super::workbench_shortcuts::is_palette_shortcut(&event) {
                        event.prevent_default();
                        event.stop_immediate_propagation();
                        on_show.call(());
                    } else if *state.open.peek() && event.key() == "Escape" {
                        event.prevent_default();
                        event.stop_immediate_propagation();
                        on_escape.call(());
                    }
                }) as Box<dyn FnMut(_)>);
                let _ = document.add_event_listener_with_callback_and_bool(
                    "keydown",
                    callback.as_ref().unchecked_ref(),
                    true,
                );
                Rc::new((document, callback))
            })
    });
    use_drop(move || {
        if let Some(listener) = listener {
            let _ = listener.0.remove_event_listener_with_callback_and_bool(
                "keydown",
                listener.1.as_ref().unchecked_ref(),
                true,
            );
        }
    });
    let browser = use_context::<super::library::ProjectBrowserState>().0;
    let commands = if open {
        registered_commands()
            .into_iter()
            .filter(|c| !browser() || matches!(c.group, "Go to" | "Project" | "Edit"))
            .collect()
    } else {
        Vec::new()
    };
    let needle = query().to_lowercase();
    let groups = [
        "Go to",
        "Project",
        "Edit",
        "View",
        "Current Workspace",
        "Objects",
    ];
    rsx! {
        dialog {
            id: "command-palette", class: "command-palette-host", aria_modal: "true",
            aria_labelledby: "command-palette-title", aria_describedby: "command-palette-description",
            onmounted: move |event: MountedEvent| {
                if let Some(host) = event.data().downcast::<web_sys::Element>().and_then(|element| element.dyn_ref::<web_sys::HtmlDialogElement>()).cloned() {
                    *mount_host.borrow_mut() = Some(host.clone());
                    sync_palette_host(&host, *state.open.peek());
                }
            },
            oncancel: move |event| {
                event.prevent_default();
                event.stop_propagation();
                if host.borrow().as_ref().is_some_and(|host| host.is_connected() && host.open()) && *state.open.peek() {
                    state.close(None);
                }
            },
        Dialog { open: Some(open), on_open_change: move |next: bool| { if !next { state.close(None); } },
            attributes: vec![content_mounted],
            class: "command-palette", is_modal: false, role: "presentation",
            aria_modal: None::<String>, aria_labelledby: None::<String>, aria_describedby: None::<String>,
            PaletteLifetime { on_closed }
            DialogTitle { id: "command-palette-title", "Command palette" }
            DialogDescription { id: "command-palette-description", "Search existing actions and objects. Unavailable actions explain why." }
            Combobox::<Command> {
                open: Some(true),
                query: Some(query()),
                aria_label: "Search commands and objects",
                list_aria_label: "Commands and objects",
                placeholder: "Search commands and objects…",
                on_query_change: move |next| query.set(next),
                on_value_change: move |command: Option<Command>| { if let Some(command) = command
                    && (!browser() || matches!(command.group, "Go to" | "Project" | "Edit")) { state.close(Some(command)); } },
                filter: move |_: (String, String)| true,
                {
                    let mut index = 0usize;
                    rsx! {
                        for group in groups {
                            {
                                let matching: Vec<_> = commands.iter().filter(|c| c.group == group && format!("{} {}", c.group, c.label).to_lowercase().contains(&needle)).cloned().collect();
                                rsx! {
                                    if !matching.is_empty() {
                                        div { role: "group", aria_label: group,
                                            div { class: "px-2 py-1 text-xs text-muted-foreground", "{group}" }
                                            for command in matching {
                                                {
                                                    let option_index = index;
                                                    index += 1;
                                                    rsx! {
                                                        ComboboxOption::<Command> { key: "{option_index}:{command.label}",
                                                            index: option_index, value: command.clone(), text_value: command.label.clone(), aria_label: command.label.clone(),
                                                            disabled: command.reason.is_some(),
                                                            div { class: "flex flex-col text-left",
                                                                span { "{command.label}" }
                                                                if let Some(reason) = &command.reason { small { "{reason}" } }
                                                            }
                                                            if let Some(hint) = command.hint { Kbd { "{hint}" } }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ComboboxEmpty { "No matching commands or objects." }
                    }
                }
            }
            Button { variant: ButtonVariant::Ghost, aria_label: "Close command palette", onclick: move |_| state.close(None), "Close" Kbd { "Esc" } }
        }
        }
    }
}

pub(super) fn use_navigation_commands(
    mut workspace: Signal<&'static str>,
    mut browser: Signal<bool>,
    has_project: bool,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_export: EventHandler<()>,
) {
    let mut commands = Vec::new();
    for name in ["Layout", "PCB", "Keymap", "Keycaps", "Case", "Parts"] {
        commands.push(Command {
            group: "Go to",
            label: name.into(),
            reason: (!has_project).then(|| "Open a project first.".into()),
            hint: None,
            on_select: EventHandler::new(move |()| {
                browser.set(false);
                workspace.set(name);
            }),
        });
    }
    commands.push(Command {
        group: "Go to",
        label: "Export".into(),
        reason: (!has_project).then(|| "Open a project first.".into()),
        hint: None,
        on_select: EventHandler::new(move |()| {
            browser.set(false);
            on_export.call(());
        }),
    });
    commands.push(Command {
        group: "Go to",
        label: "Browse projects".into(),
        reason: None,
        hint: None,
        on_select: EventHandler::new(move |()| browser.set(true)),
    });
    for (label, hint, on_select) in [
        ("Undo", "Ctrl/Cmd Z", on_undo),
        ("Redo", "Ctrl/Cmd Shift Z / Ctrl Y", on_redo),
    ] {
        commands.push(Command {
            group: "Edit",
            label: label.into(),
            reason: (!has_project).then(|| "Open a project first.".into()),
            hint: Some(hint),
            on_select,
        });
    }
    use_commands(commands);
}

pub(super) fn editor_commands(
    model: &boardstudio_application::ReadModel,
    workspace: &'static str,
    scope: &boardstudio_application::Scope,
    generation: u64,
    on_select: EventHandler<super::objects::TreeSelectRequest>,
) -> Vec<Command> {
    use super::objects::{self, TreeContext, TreeSelectRequest};
    use boardstudio_application::SelectionMode;
    let mut commands = Vec::new();
    if !matches!(workspace, "Layout" | "PCB" | "Keycaps" | "Keymap" | "Case") {
        return commands;
    }
    let Some(snapshot) = &model.accepted else {
        return commands;
    };
    let document = &snapshot.document;
    let mut targets = Vec::new();
    for matrix in &document.matrices {
        if objects::matrix_visible_on_board(document, &scope.board_id, &matrix.id) {
            let context = TreeContext::Matrix {
                matrix_id: matrix.id.clone(),
            };
            if let Some(label) = objects::context_label(model, &context) {
                targets.push((format!("Matrix {label}"), context));
            }
            for row in 0..matrix.rows {
                for column in 0..matrix.columns {
                    if let Some(context) = objects::context_for_cell(model, &matrix.id, row, column)
                        && let Some(label) = objects::context_label(model, &context)
                    {
                        targets.push((format!("Key {label}"), context));
                    }
                }
            }
        }
    }
    if let Some(board) = document.boards.iter().find(|b| b.id == scope.board_id) {
        for id in &board.part_ids {
            if let Some(context) = objects::component_context_for_finding_part(model, id)
                && let Some(part) = document.parts.iter().find(|p| p.id == *id)
            {
                targets.push((format!("Part {}", part.reference), context));
            }
        }
        targets.push((
            format!("{} Outline", board.name),
            TreeContext::Outline {
                board_id: board.id.clone(),
            },
        ));
        if let Some(outline) = document
            .board_outlines
            .iter()
            .find(|o| o.board_id == board.id)
        {
            for version in &outline.versions {
                targets.push((
                    version.name.clone(),
                    TreeContext::OutlineVersion {
                        board_id: board.id.clone(),
                        version_id: Some(version.id.clone()),
                    },
                ));
            }
        }
    }
    for (label, context) in targets {
        let outline_action = super::outline_lifecycle::OutlineAction::for_tree(
            snapshot, scope, generation, &context,
        );
        let scope = scope.clone();
        commands.push(Command {
            group: "Objects",
            label,
            reason: None,
            hint: None,
            on_select: EventHandler::new(move |()| {
                on_select.call(TreeSelectRequest {
                    scope: scope.clone(),
                    context: context.clone(),
                    mode: SelectionMode::Replace,
                    outline_action: outline_action.clone(),
                })
            }),
        });
    }
    commands
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn popover_opener_requires_the_unique_nearest_visible_focusable_relationship() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_inner_html(r#"
            <button aria-label="Outer opener" aria-controls="palette-test-outer">Outer</button>
            <div data-slot="popover-content" id="palette-test-outer">
                <button aria-label="Inner opener" aria-controls="other palette-test-inner" tabindex="-1">Inner</button>
                <div class="board-popover-content" id="palette-test-inner">
                    <input aria-label="Popup field">
                </div>
            </div>
            <div hidden><button aria-controls="palette-test-inner">Hidden duplicate</button></div>
            <div inert><button aria-controls="palette-test-inner">Inert duplicate</button></div>
            <span aria-controls="palette-test-inner">Not focusable</span>
            <input aria-label="Ordinary field">
            <div role="dialog" id="palette-test-sheet"><input aria-label="Sheet field"></div>
            <button aria-controls="palette-test-sheet">Sheet opener</button>
        "#);
        document.body().unwrap().append_child(&root).unwrap();
        let field = by_role_name(&root, "textbox", "Popup field")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        let inner = by_role_name(&root, "button", "Inner opener")
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap();
        assert_eq!(popover_opener(&field), Some(inner.clone()));
        for name in ["Ordinary field", "Sheet field"] {
            let field = by_role_name(&root, "textbox", name)
                .unwrap()
                .dyn_into::<web_sys::HtmlElement>()
                .unwrap();
            assert!(popover_opener(&field).is_none());
        }
        let duplicate = document.create_element("button").unwrap();
        duplicate.set_text_content(Some("Ambiguous duplicate"));
        duplicate
            .set_attribute("aria-controls", "palette-test-inner")
            .unwrap();
        root.append_child(&duplicate).unwrap();
        assert!(popover_opener(&field).is_none());
        duplicate.remove();
        for (name, value) in [
            ("disabled", ""),
            ("hidden", ""),
            ("inert", ""),
            ("aria-hidden", "true"),
            ("style", "visibility:hidden"),
        ] {
            inner.set_attribute(name, value).unwrap();
            assert!(popover_opener(&field).is_none(), "must reject {name}");
            inner.remove_attribute(name).unwrap();
        }
        inner.remove();
        assert!(popover_opener(&field).is_none());
        root.remove();
    }
}
