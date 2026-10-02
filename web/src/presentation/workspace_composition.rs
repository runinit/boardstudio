//! Private routing for the six independently owned workspace surfaces.
use super::objects;
use boardstudio_application::Scope;
use dioxus::prelude::*;

pub(super) struct SharedObjectsInput {
    pub(super) selected_context: Signal<Option<objects::ScopedTreeContext>>,
    pub(super) on_select: EventHandler<objects::TreeSelectRequest>,
    pub(super) on_navigate: EventHandler<(Scope, String, Option<String>)>,
    pub(super) on_nudge: EventHandler<objects::TreeNudgeRequest>,
}

pub(super) enum WorkspaceObjectsInput {
    Layout(SharedObjectsInput),
    Pcb(SharedObjectsInput),
    Keymap(SharedObjectsInput),
    Keycaps(SharedObjectsInput),
    Case(SharedObjectsInput),
    Parts(super::parts_workspace::ObjectsInput),
}

pub(super) enum WorkspaceToolbarInput {
    Layout(super::layout_workspace::ToolbarInput),
    Pcb,
    Keymap,
    Keycaps,
    Case,
    Parts,
    Export,
    Other,
}

pub(super) struct CanvasEventHandlers {
    pub(super) mount: EventHandler<MountedEvent>,
    pub(super) start_pan: EventHandler<PointerEvent>,
    pub(super) move_pointer: EventHandler<PointerEvent>,
    pub(super) end_pointer: EventHandler<PointerEvent>,
    pub(super) cancel_pointer: EventHandler<PointerEvent>,
    pub(super) keyboard: EventHandler<KeyboardEvent>,
    pub(super) key_up: EventHandler<KeyboardEvent>,
    pub(super) wheel: EventHandler<WheelEvent>,
}

pub(super) struct PlaceholderInput {
    pub(super) workspace: Signal<&'static str>,
    pub(super) name: &'static str,
    pub(super) message: &'static str,
}

pub(super) enum WorkspaceCanvasInput {
    Pcb(PlaceholderInput),
    Keymap(Box<super::keymap_workspace::CanvasInput>),
    Keycaps(PlaceholderInput),
    Case(Box<super::case_workspace::CanvasInput>),
    Parts(PlaceholderInput),
    Other(PlaceholderInput),
}

pub(super) enum WorkspaceInspectorInput {
    Layout(super::layout_workspace::InspectorInput),
    Pcb,
    Keymap(Box<super::keymap_workspace::InspectorInput>),
    Keycaps,
    Case(super::case_workspace::InspectorInput),
    Parts(super::parts_workspace::InspectorInput),
}

pub(super) fn objects(input: WorkspaceObjectsInput) -> Element {
    match input {
        WorkspaceObjectsInput::Layout(input) => super::layout_workspace::objects(input),
        WorkspaceObjectsInput::Pcb(input) => super::pcb_workspace::objects(input),
        WorkspaceObjectsInput::Keymap(input) => super::keymap_workspace::objects(input),
        WorkspaceObjectsInput::Keycaps(input) => super::keycaps_workspace::objects(input),
        WorkspaceObjectsInput::Case(input) => super::case_workspace::objects(input),
        WorkspaceObjectsInput::Parts(input) => super::parts_workspace::objects(input),
    }
}

pub(super) fn toolbar(input: WorkspaceToolbarInput) -> Element {
    match input {
        WorkspaceToolbarInput::Layout(input) => super::layout_workspace::toolbar(input),
        WorkspaceToolbarInput::Pcb => super::pcb_workspace::toolbar(),
        WorkspaceToolbarInput::Keymap => super::keymap_workspace::toolbar(),
        WorkspaceToolbarInput::Keycaps => super::keycaps_workspace::toolbar(),
        WorkspaceToolbarInput::Case => super::case_workspace::toolbar(),
        WorkspaceToolbarInput::Parts => super::parts_workspace::toolbar(),
        WorkspaceToolbarInput::Export | WorkspaceToolbarInput::Other => rsx! {},
    }
}

/// Layout's pointer-transaction SVG stays inline with its existing Editor-owned
/// handlers. Other workspaces mount their current content here.
pub(super) fn canvas(input: WorkspaceCanvasInput) -> Element {
    match input {
        WorkspaceCanvasInput::Pcb(input) => super::pcb_workspace::canvas(input),
        WorkspaceCanvasInput::Keymap(input) => super::keymap_workspace::canvas(*input),
        WorkspaceCanvasInput::Keycaps(input) => super::keycaps_workspace::canvas(input),
        WorkspaceCanvasInput::Case(input) => super::case_workspace::canvas(*input),
        WorkspaceCanvasInput::Parts(input) => super::parts_workspace::canvas(input),
        WorkspaceCanvasInput::Other(input) => super::parts_workspace::canvas(input),
    }
}

pub(super) fn inspector(input: WorkspaceInspectorInput) -> Element {
    match input {
        WorkspaceInspectorInput::Layout(input) => super::layout_workspace::inspector(input),
        WorkspaceInspectorInput::Pcb => super::pcb_workspace::inspector(),
        WorkspaceInspectorInput::Keymap(input) => super::keymap_workspace::inspector(*input),
        WorkspaceInspectorInput::Keycaps => super::keycaps_workspace::inspector(),
        WorkspaceInspectorInput::Case(input) => super::case_workspace::inspector(input),
        WorkspaceInspectorInput::Parts(input) => super::parts_workspace::inspector(input),
    }
}
