//! Private routing for the six independently owned workspace surfaces.
use super::objects;
use boardstudio_application::Scope;
use dioxus::prelude::*;

pub(super) struct SharedObjectsInput {
    pub(super) selected_context: Signal<Option<objects::ScopedTreeContext>>,
    pub(super) on_select: EventHandler<objects::TreeSelectRequest>,
    pub(super) on_navigate: EventHandler<(Scope, String, Option<String>)>,
    pub(super) on_nudge: EventHandler<objects::TreeNudgeRequest>,
    pub(super) board_setup: objects::BoardSetupMount,
}

pub(super) enum WorkspaceObjectsInput<'a> {
    Layout(Box<super::layout_workspace::ObjectsInput>),
    Pcb(Box<super::pcb_workspace::ObjectsInput>),
    Keymap(super::keymap_workspace::NavigationInput),
    Keycaps(super::keycaps_workspace::NavigationInput),
    Case(Box<super::case_workspace::ObjectsInput<'a>>),
    Parts(super::parts_workspace::ObjectsInput),
}

pub(super) enum WorkspaceToolbarInput {
    Layout(Box<super::layout_workspace::ToolbarInput>),
    Pcb(Box<super::pcb_workspace::ToolbarInput>),
    Keymap(super::shared_viewer::DesignViewToolbarProps),
    Keycaps(super::shared_viewer::DesignViewToolbarProps),
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
    PcbModule(Box<super::pcb_module_inspector::InspectorInput>),
    Keymap(Box<super::keymap_workspace::InspectorInput>),
    Keycaps(Box<super::keycaps_workspace::InspectorInput>),
    Case(Box<super::case_workspace::InspectorInput>),
    Parts(Box<super::parts_workspace::InspectorInput>),
}

pub(super) fn objects(i: WorkspaceObjectsInput<'_>) -> Element {
    match i {
        WorkspaceObjectsInput::Layout(i) => super::layout_workspace::objects(*i),
        WorkspaceObjectsInput::Pcb(i) => super::pcb_workspace::objects(*i),
        WorkspaceObjectsInput::Keymap(i) => super::keymap_workspace::objects(i),
        WorkspaceObjectsInput::Keycaps(i) => super::keycaps_workspace::objects(i),
        WorkspaceObjectsInput::Case(i) => super::case_workspace::objects(*i),
        WorkspaceObjectsInput::Parts(i) => super::parts_workspace::objects(i),
    }
}

pub(super) fn toolbar(i: WorkspaceToolbarInput) -> Element {
    match i {
        WorkspaceToolbarInput::Layout(i) => super::layout_workspace::toolbar(*i),
        WorkspaceToolbarInput::Pcb(i) => super::pcb_workspace::toolbar(*i),
        WorkspaceToolbarInput::Keymap(i) => super::keymap_workspace::toolbar(i),
        WorkspaceToolbarInput::Keycaps(i) => super::keycaps_workspace::toolbar(i),
        WorkspaceToolbarInput::Case => super::case_workspace::toolbar(),
        WorkspaceToolbarInput::Parts => super::parts_workspace::toolbar(),
        WorkspaceToolbarInput::Export | WorkspaceToolbarInput::Other => rsx! {},
    }
}

pub(super) fn canvas(i: WorkspaceCanvasInput) -> Element {
    match i {
        WorkspaceCanvasInput::Pcb(i) => super::pcb_workspace::canvas(*i),
        WorkspaceCanvasInput::Keymap(i) => super::keymap_workspace::canvas(*i),
        WorkspaceCanvasInput::Keycaps(i) => super::keycaps_workspace::canvas(*i),
        WorkspaceCanvasInput::Case(i) => super::case_workspace::canvas(*i),
        WorkspaceCanvasInput::Parts(i) => super::parts_workspace::preview(i),
        WorkspaceCanvasInput::Other(i) => super::parts_workspace::canvas(i),
    }
}

pub(super) fn inspector(i: WorkspaceInspectorInput) -> Element {
    match i {
        WorkspaceInspectorInput::Layout(i) => super::layout_workspace::inspector(*i),
        WorkspaceInspectorInput::Pcb(Some(i)) => rsx! {
            super::pcb_workspace::PcbInspector { selected: i.source.active_part_id.is_some(),
                super::pcb_wiring::PcbWiringInspector {
                    board_overview: true, source: i.source, resolution: i.resolution,
                    firmware_positions: i.firmware_positions, firmware_feedback: i.firmware_feedback,
                    firmware_controls: i.firmware_controls, part_net_actions: i.part_net_actions,
                    part_input_actions: i.part_input_actions, on_firmware_edit: i.on_firmware_edit,
                    on_resolve: i.on_resolve, on_choose_controller: i.on_choose_controller,
                    on_edit_board_wiring: i.on_edit_board_wiring, mode_actions: i.mode_actions,
                    pin_actions: i.pin_actions, apply_actions: i.apply_actions,
                    protected_remap_actions: i.protected_remap_actions,
                }
            }
        },
        WorkspaceInspectorInput::Pcb(None) => super::pcb_workspace::inspector(),
        WorkspaceInspectorInput::PcbModule(i) => super::pcb_module_inspector::inspector(*i),
        WorkspaceInspectorInput::Keymap(i) => super::keymap_workspace::inspector(*i),
        WorkspaceInspectorInput::Keycaps(i) => super::keycaps_workspace::inspector(*i),
        WorkspaceInspectorInput::Case(i) => super::case_workspace::inspector(*i),
        WorkspaceInspectorInput::Parts(i) => super::parts_workspace::inspector(*i),
    }
}
