//! Search and navigation only; commands remain owned by their mounted controls.
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::DomEventListener;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    combobox::{Combobox, ComboboxEmpty, ComboboxOption},
    command::{Command, CommandRegistry, use_commands},
    dialog::{Dialog, DialogDescription, DialogTitle},
    kbd::Kbd,
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

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

impl PaletteState {
    pub fn show(mut self) {
        if (self.open)() {
            return;
        }
        let active = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.active_element())
            .and_then(|e| e.dyn_into::<web_sys::HtmlElement>().ok());
        self.opener.set(active.clone());
        self.pending.set(None);
        if let Some(el) = active
            && matches!(el.tag_name().as_str(), "INPUT" | "TEXTAREA" | "SELECT")
        {
            let _ = el.blur();
        }
        self.open.set(true);
    }

    fn close(mut self, command: Option<Command>) {
        if !*self.open.peek() {
            return;
        }
        self.pending.set(command);
        self.open.set(false);
        let opener = (self.opener)();
        spawn(async move {
            gloo_timers::future::TimeoutFuture::new(0).await;
            if *self.open.peek() {
                return;
            }
            if let Some(target) = opener.filter(|e| e.is_connected()) {
                let _ = target.focus();
            }
            if let Some(cmd) = self.pending.write().take() {
                cmd.on_select.call(());
            }
        });
    }
}

#[component]
pub(super) fn CommandPalette(state: PaletteState) -> Element {
    let registry = use_context::<CommandRegistry>();
    let browser = use_context::<super::library::ProjectBrowserState>().0;
    let mut query = use_signal(String::new);
    let open = (state.open)();
    use_effect(move || {
        if !(state.open)() {
            query.set(String::new());
        }
    });
    let _palette_guard = use_hook(move || {
        Rc::new(
            web_sys::window()
                .and_then(|w| w.document())
                .and_then(|doc| {
                    DomEventListener::listen(&doc, "keydown", true, move |e: web_sys::Event| {
                        let e: web_sys::KeyboardEvent = e.unchecked_into();
                        if super::workbench_shortcuts::is_palette_shortcut(&e) {
                            e.prevent_default();
                            e.stop_immediate_propagation();
                            state.show();
                        } else if *state.open.peek() && e.key() == "Escape" {
                            e.prevent_default();
                            e.stop_immediate_propagation();
                            state.close(None);
                        }
                    })
                }),
        )
    });

    let commands: Vec<_> = if open {
        registry
            .commands()
            .into_iter()
            .filter(|c| !browser() || matches!(c.group, "Go to" | "Project" | "Edit"))
            .collect()
    } else {
        vec![]
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
    // Index-keyed for the `String`-only `Combobox`: positions here must match the
    // render order below so a committed key decodes to the visible command.
    let grouped: Vec<(&str, Vec<Command>)> = groups
        .into_iter()
        .map(|group| {
            let matching: Vec<_> = commands
                .iter()
                .filter(|c| {
                    c.group == group
                        && format!("{} {}", c.group, c.label)
                            .to_lowercase()
                            .contains(&needle)
                })
                .cloned()
                .collect();
            (group, matching)
        })
        .collect();
    let ordered: Vec<Command> = grouped
        .iter()
        .flat_map(|(_, matching)| matching.iter().cloned())
        .collect();
    rsx! {
        Dialog { open: Some(open), on_open_change: move |next: bool| if !next { state.close(None); },
            class: "command-palette",
            DialogTitle { id: "command-palette-title", "Command palette" }
            DialogDescription { id: "command-palette-description", "Search existing actions and objects. Unavailable actions explain why." }
            Combobox {
                open: Some(true), query: Some(query()),
                aria_label: "Search commands and objects", list_aria_label: "Commands and objects",
                placeholder: "Search commands and objects…",
                on_query_change: move |next| query.set(next),
                on_value_change: move |key: Option<String>| {
                    let Some(cmd) = key
                        .and_then(|key| key.parse::<usize>().ok())
                        .and_then(|index| ordered.get(index).cloned()) else { return; };
                    if !browser() || matches!(cmd.group, "Go to" | "Project" | "Edit") { state.close(Some(cmd)); }
                },
                filter: move |_: (String, String)| true,
                {
                    let mut idx = 0usize;
                    rsx! {
                        for (group, matching) in grouped {
                            {
                                rsx! {
                                    if !matching.is_empty() {
                                        div { role: "group", aria_label: group,
                                            div { class: "px-2 py-1 text-xs text-muted-foreground", "{group}" }
                                            for cmd in matching {
                                                {
                                                    let i = idx; idx += 1;
                                                    rsx! {
                                                        ComboboxOption { key: "{i}", index: i, value: i.to_string(), text_value: cmd.label.clone(), aria_label: cmd.label.clone(), disabled: cmd.reason.is_some(),
                                                            div { class: "flex flex-col text-left",
                                                                span { "{cmd.label}" }
                                                                if let Some(reason) = &cmd.reason { small { "{reason}" } }
                                                            }
                                                            if let Some(hint) = cmd.hint { Kbd { "{hint}" } }
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

pub(super) fn use_navigation_commands(
    mut workspace: Signal<Workspace>,
    mut browser: Signal<bool>,
    has_project: bool,
    on_undo: EventHandler<()>,
    on_redo: EventHandler<()>,
    on_export: EventHandler<()>,
) {
    let no_proj = (!has_project).then(|| "Open a project first.".to_owned());
    let mut cmds: Vec<_> = Workspace::TABS
        .into_iter()
        .map(|name| Command {
            group: "Go to",
            label: name.as_str().into(),
            reason: no_proj.clone(),
            hint: None,
            on_select: EventHandler::new(move |()| {
                browser.set(false);
                workspace.set(name);
            }),
        })
        .collect();
    cmds.push(Command {
        group: "Go to",
        label: Workspace::Export.as_str().into(),
        reason: no_proj.clone(),
        hint: None,
        on_select: EventHandler::new(move |()| {
            browser.set(false);
            on_export.call(());
        }),
    });
    cmds.push(Command {
        group: "Go to",
        label: "Browse projects".into(),
        reason: None,
        hint: None,
        on_select: EventHandler::new(move |()| browser.set(true)),
    });
    cmds.push(Command {
        group: "Edit",
        label: "Undo".into(),
        reason: no_proj.clone(),
        hint: Some("Ctrl/Cmd Z"),
        on_select: on_undo,
    });
    cmds.push(Command {
        group: "Edit",
        label: "Redo".into(),
        reason: no_proj,
        hint: Some("Ctrl/Cmd Shift Z / Ctrl Y"),
        on_select: on_redo,
    });
    use_commands(cmds);
}

pub(super) fn editor_commands(
    model: &boardstudio_application::ReadModel,
    workspace: Workspace,
    scope: &boardstudio_application::Scope,
    generation: u64,
    on_select: EventHandler<super::objects::TreeSelectRequest>,
) -> Vec<Command> {
    use super::objects::{self, TreeContext, TreeSelectRequest};
    use boardstudio_application::SelectionMode;
    if !matches!(
        workspace,
        Workspace::Layout
            | Workspace::Pcb
            | Workspace::Keycaps
            | Workspace::Keymap
            | Workspace::Case
    ) {
        return vec![];
    }
    let Some(snapshot) = &model.accepted else {
        return vec![];
    };
    let resolved = boardstudio_application::resolved_document(&snapshot.document);
    let doc = resolved.as_ref();
    let mut targets = Vec::new();
    for m in doc
        .matrices
        .iter()
        .filter(|m| objects::matrix_visible_on_board(doc, &scope.board_id, &m.id))
    {
        let ctx = TreeContext::Matrix {
            matrix_id: m.id.clone(),
        };
        if let Some(lbl) = objects::context_label(model, &ctx) {
            targets.push((format!("Matrix {lbl}"), ctx));
        }
        for r in 0..m.rows {
            for c in 0..m.columns {
                if let Some(ctx) = objects::context_for_cell(model, &m.id, r, c)
                    && let Some(lbl) = objects::context_label(model, &ctx)
                {
                    targets.push((format!("Key {lbl}"), ctx));
                }
            }
        }
    }
    if let Some(b) = doc.boards.iter().find(|b| b.id == scope.board_id) {
        for id in &b.part_ids {
            if let Some(ctx) = objects::component_context_for_finding_part(model, id)
                && let Some(p) = doc.parts.iter().find(|p| p.id == *id)
            {
                targets.push((format!("Part {}", p.reference), ctx));
            }
        }
        targets.push((
            format!("{} Outline", b.name),
            TreeContext::Outline {
                board_id: b.id.clone(),
            },
        ));
        if let Some(o) = doc.board_outlines.iter().find(|o| o.board_id == b.id) {
            for v in &o.versions {
                targets.push((
                    v.name.clone(),
                    TreeContext::OutlineVersion {
                        board_id: b.id.clone(),
                        version_id: Some(v.id.clone()),
                    },
                ));
            }
        }
    }
    targets
        .into_iter()
        .map(|(label, context)| {
            let outline_action =
                super::outline::OutlineAction::for_tree(snapshot, scope, generation, &context);
            let sc = scope.clone();
            Command {
                group: "Objects",
                label,
                reason: None,
                hint: None,
                on_select: EventHandler::new(move |()| {
                    on_select.call(TreeSelectRequest {
                        scope: sc.clone(),
                        context: context.clone(),
                        mode: SelectionMode::Replace,
                        outline_action: outline_action.clone(),
                    })
                }),
            }
        })
        .collect()
}
