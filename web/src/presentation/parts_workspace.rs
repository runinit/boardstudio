//! Parts-owned Objects, canvas placeholder and Inspector composition.
use super::parts::{PartsInspectorPanel, PartsLibraryPanel, PartsPreviewWorkspace};
use super::parts::{PartsQuery, PartsSelection};
use super::workspace_composition::PlaceholderInput;
use boardstudio_application::{AcceptedSnapshot, Scope};
use dioxus::prelude::*;

pub(super) struct ObjectsInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) workspace: Signal<&'static str>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) on_select: EventHandler<()>,
}

pub(super) struct InspectorInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
    pub(super) on_place_controller: EventHandler<String>,
    pub(super) controller_placement_enabled: bool,
    pub(super) placement_busy: bool,
    pub(super) placement_error: Option<String>,
    pub(super) layout_target: Signal<Option<String>>,
}

pub(super) struct CanvasInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Option<Scope>,
    pub(super) query: PartsQuery,
    pub(super) selected: PartsSelection,
}

pub(super) fn objects(input: ObjectsInput) -> Element {
    rsx! {
        PartsLibraryPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            workspace: input.workspace,
            query: input.query,
            selected: input.selected,
            on_select: input.on_select,
        }
    }
}

pub(super) fn toolbar() -> Element {
    rsx! {}
}

pub(super) fn canvas(mut input: PlaceholderInput) -> Element {
    rsx! {
        section { class: "m1-placeholder-workspace",
            h1 { "{input.name}" }
            p { "{input.message}" }
            button { onclick: move |_| input.workspace.set("Layout"), "Back to Layout" }
        }
    }
}

pub(super) fn preview(input: CanvasInput) -> Element {
    rsx! {
        PartsPreviewWorkspace {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
        }
    }
}

pub(super) fn inspector(input: InspectorInput) -> Element {
    rsx! {
        PartsInspectorPanel {
            snapshot: input.snapshot,
            scope: input.scope,
            query: input.query,
            selected: input.selected,
            on_place_controller: input.on_place_controller,
            controller_placement_enabled: input.controller_placement_enabled,
            placement_busy: input.placement_busy,
            placement_error: input.placement_error,
            layout_target: input.layout_target,
        }
    }
}
