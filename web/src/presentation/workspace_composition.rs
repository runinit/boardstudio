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

pub(super) enum WorkspaceObjectsInput<'a> {
    Layout(Box<super::layout_workspace::ObjectsInput>),
    Pcb(SharedObjectsInput),
    Keymap(SharedObjectsInput),
    Keycaps(SharedObjectsInput),
    Case(Box<super::case_workspace::ObjectsInput<'a>>),
    Parts(super::parts_workspace::ObjectsInput),
}

pub(super) enum WorkspaceToolbarInput {
    Layout(Box<super::layout_workspace::ToolbarInput>),
    Pcb,
    Keymap(super::shared_viewer::DesignViewToolbarProps),
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
    Pcb(Box<super::pcb_workspace::CanvasInput>),
    Keymap(Box<super::keymap_workspace::CanvasInput>),
    Keycaps(Box<super::keycaps_workspace::CanvasInput>),
    Case(Box<super::case_workspace::CanvasInput>),
    Parts(super::parts_workspace::CanvasInput),
    Other(PlaceholderInput),
}

pub(super) enum WorkspaceInspectorInput {
    Layout(Box<super::layout_workspace::InspectorInput>),
    Pcb(Option<Box<super::pcb_wiring::PcbWiringInspectorProps>>),
    Keymap(Box<super::keymap_workspace::InspectorInput>),
    Keycaps(Box<super::keycaps_workspace::InspectorInput>),
    Case(Box<super::case_workspace::InspectorInput>),
    Parts(Box<super::parts_workspace::InspectorInput>),
}

pub(super) fn objects(input: WorkspaceObjectsInput<'_>) -> Element {
    match input {
        WorkspaceObjectsInput::Layout(input) => super::layout_workspace::objects(*input),
        WorkspaceObjectsInput::Pcb(input) => super::pcb_workspace::objects(input),
        WorkspaceObjectsInput::Keymap(input) => super::keymap_workspace::objects(input),
        WorkspaceObjectsInput::Keycaps(input) => super::keycaps_workspace::objects(input),
        WorkspaceObjectsInput::Case(input) => super::case_workspace::objects(*input),
        WorkspaceObjectsInput::Parts(input) => super::parts_workspace::objects(input),
    }
}

pub(super) fn toolbar(input: WorkspaceToolbarInput) -> Element {
    match input {
        WorkspaceToolbarInput::Layout(input) => super::layout_workspace::toolbar(*input),
        WorkspaceToolbarInput::Pcb => super::pcb_workspace::toolbar(),
        WorkspaceToolbarInput::Keymap(input) => super::keymap_workspace::toolbar(input),
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
        WorkspaceCanvasInput::Pcb(input) => super::pcb_workspace::canvas(*input),
        WorkspaceCanvasInput::Keymap(input) => super::keymap_workspace::canvas(*input),
        WorkspaceCanvasInput::Keycaps(input) => super::keycaps_workspace::canvas(*input),
        WorkspaceCanvasInput::Case(input) => super::case_workspace::canvas(*input),
        WorkspaceCanvasInput::Parts(input) => super::parts_workspace::preview(input),
        WorkspaceCanvasInput::Other(input) => super::parts_workspace::canvas(input),
    }
}

pub(super) fn inspector(input: WorkspaceInspectorInput) -> Element {
    match input {
        WorkspaceInspectorInput::Layout(input) => super::layout_workspace::inspector(*input),
        WorkspaceInspectorInput::Pcb(Some(input)) => rsx! {
            super::pcb_wiring::PcbWiringInspector {
                source: input.source,
                resolution: input.resolution,
                firmware_positions: input.firmware_positions,
                firmware_feedback: input.firmware_feedback,
                firmware_controls: input.firmware_controls,
                part_net_actions: input.part_net_actions,
                part_input_actions: input.part_input_actions,
                on_firmware_edit: input.on_firmware_edit,
                on_resolve: input.on_resolve,
                on_edit_board_wiring: input.on_edit_board_wiring,
                mode_actions: input.mode_actions,
                pin_actions: input.pin_actions,
                apply_actions: input.apply_actions,
                protected_remap_actions: input.protected_remap_actions,
            }
        },
        WorkspaceInspectorInput::Pcb(None) => super::pcb_workspace::inspector(),
        WorkspaceInspectorInput::Keymap(input) => super::keymap_workspace::inspector(*input),
        WorkspaceInspectorInput::Keycaps(input) => super::keycaps_workspace::inspector(*input),
        WorkspaceInspectorInput::Case(input) => super::case_workspace::inspector(*input),
        WorkspaceInspectorInput::Parts(input) => super::parts_workspace::inspector(*input),
    }
}
