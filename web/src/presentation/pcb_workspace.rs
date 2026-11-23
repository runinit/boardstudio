//! PCB-owned workspace surface composition.
use super::workbench::editor_session::selection_handlers;
mod interaction;
use super::pcb_scene::{PcbPartHit, PcbPartPointerDown, PcbScene};
use super::workspace_composition::{CanvasEventHandlers, SharedObjectsInput};
use super::workspace_composition::{
    WorkspaceInteraction, WorkspaceSurfaces, use_frame, use_publish_surfaces,
};
use super::*;
use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_web_ui_model::editor::{EditGuard, EditorContext};
use boardstudio_web_ui_shared::inspector_fields::InspectorSection;
use boardstudio_web_ui_shared::ui::toggle_group::{ToggleGroup, ToggleItem};
use dioxus::prelude::*;

pub(super) struct ToolbarInput {
    pub(super) command_label: String,
    pub(super) menu_owner_key: String,
    pub(super) open_menu: Signal<Option<objects::LayoutCommandMenu>>,
    pub(super) selection_kind: objects::LayoutSelectionKind,
    pub(super) snap_settings: objects::LayoutSnapSettings,
    pub(super) transform: objects::LayoutTransformMenuMount,
    pub(super) align: objects::LayoutAlignMount,
    pub(super) on_selection_kind: EventHandler<objects::LayoutSelectionKind>,
    pub(super) on_snap_intent: EventHandler<objects::LayoutSnapIntent>,
    pub(super) has_selection_context: bool,
    pub(super) show_relationships: bool,
    pub(super) on_show_relationships: EventHandler<()>,
}

pub(super) struct CanvasInput {
    pub(super) snapshot: AcceptedSnapshot,
    pub(super) scope: Scope,
    pub(super) view_box: String,
    pub(super) selected_ids: Vec<String>,
    pub(super) generation: u64,
    pub(super) handlers: CanvasEventHandlers,
    pub(super) on_empty_hit: EventHandler<PointerEvent>,
    pub(super) on_part_hit: EventHandler<PcbPartHit>,
    pub(super) on_part_pointer_down: EventHandler<PcbPartPointerDown>,
    pub(super) on_module_select: EventHandler<String>,
}

pub(super) struct ObjectsInput {
    pub(super) shared: SharedObjectsInput,
    pub(super) matrix_setup: objects::MatrixSetupMount,
    pub(super) on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    pub(super) layout_target: Signal<Option<String>>,
    pub(super) parts_query: super::parts::PartsQuery,
    pub(super) on_browse_parts: EventHandler<()>,
    pub(super) placement_error: Option<String>,
}

pub(super) fn objects(i: ObjectsInput) -> Element {
    rsx! {
        div {
            objects::Objects {
                selected_context: i.shared.selected_context, on_select: i.shared.on_select, on_navigate: i.shared.on_navigate,
                on_nudge: i.shared.on_nudge, board_setup: Some(i.shared.board_setup), matrix_setup: Some(i.matrix_setup),
                pair_created: None, on_place_component: Some(i.on_place_component), layout_target: Some(i.layout_target),
                parts_query: Some(i.parts_query), on_browse_parts: Some(i.on_browse_parts), placement_error: i.placement_error,
                matrix_inspector: None, add_object_menu_open: None,
            }
        }
    }
}

pub(super) fn toolbar(i: ToolbarInput) -> Element {
    rsx! {
        div { class: "pointer-events-none absolute inset-0 z-[12] [container:layout-toolbar/inline-size]", "data-workspace-toolbar": "pcb",
            div { class: "pointer-events-none absolute inset-x-3 top-3 max-[760px]:inset-x-2 max-[760px]:top-2",
                objects::LayoutCommandPill {
                    command_label: i.command_label, menu_owner_key: i.menu_owner_key, open_menu: i.open_menu,
                    transform: i.transform, align: i.align, selection_kind: i.selection_kind, snap_settings: i.snap_settings,
                    on_selection_kind: i.on_selection_kind, on_snap_intent: i.on_snap_intent,
                    has_selection_context: i.has_selection_context, show_relationships: i.show_relationships,
                    on_show_relationships: i.on_show_relationships,
                }
            }
        }
    }
}

pub(super) fn canvas(i: CanvasInput) -> Element {
    let h = i.handlers;
    rsx! {
        section { class: "flex min-h-0 min-w-0 flex-1", "aria-label": "PCB workspace",
            svg {
                class: "block h-full min-h-[250px] min-w-0 w-full flex-1 touch-none bg-canvas max-[760px]:min-h-0",
                view_box: "{i.view_box}", preserve_aspect_ratio: "xMidYMid meet", tabindex: "0", role: "group",
                "aria-label": "PCB layout; select a part with click, Enter, or Space, hold Space and drag to pan, or use the mouse wheel to zoom",
                onmounted: h.mount,
                onpointerdown: move |e: PointerEvent| { h.start_pan.call(e.clone()); i.on_empty_hit.call(e); },
                onpointermove: h.move_pointer, onpointerup: h.end_pointer, onpointercancel: h.cancel_pointer,
                onlostpointercapture: h.cancel_pointer, onkeydown: h.keyboard, onkeyup: h.key_up, onwheel: h.wheel,
                g { transform: "scale(1,-1)",
                    PcbScene {
                        snapshot: i.snapshot, scope: i.scope, selected_ids: i.selected_ids, generation: i.generation,
                        on_part_hit: i.on_part_hit, on_part_pointer_down: i.on_part_pointer_down, on_module_select: i.on_module_select,
                    }
                }
            }
        }
    }
}

#[component]
pub(super) fn PcbInspector(selected: bool, children: Element) -> Element {
    let mut wiring = use_signal(|| true);
    rsx! {
        section { aria_label: "PCB inspector",
            ToggleGroup { class: "mb-4 flex gap-0 border-b border-border", aria_label: "PCB inspector mode", horizontal: true,
                pressed: Some([if wiring() { 1usize } else { 0usize }].into_iter().collect()),
                on_pressed_change: move |p: std::collections::HashSet<usize>| {
                    if p.contains(&1) { wiring.set(true); } else if p.contains(&0) && selected { wiring.set(false); }
                },
                ToggleItem { index: 0usize, disabled: !selected, "Selection" }
                ToggleItem { index: 1usize, "Routing & nets" }
            }
            div { hidden: wiring(), inert: wiring().then_some(""),
                InspectorSection { title: "Position", default_open: true, super::inspector::Inspector {} }
            }
            div { hidden: !wiring(), inert: (!wiring()).then_some(""), {children} }
        }
    }
}

pub(super) fn pcb_source(
    editor: EditorContext,
    model: &ReadModel,
    current_scope: Option<Scope>,
    snapshot: &AcceptedSnapshot,
    scope_generation: u64,
) -> Option<pcb_wiring::PcbWiringSource> {
    let instance_selection = editor.instance_selection;
    current_scope
        .as_ref()
        .filter(|scope| {
            active_board_scope_matches(model, scope) && instance_selection.is_current(model)
        })
        .and_then(|scope| {
            let mut board_scope = scope.clone();
            board_scope.instance_id = None;
            pcb_wiring::PcbWiringSource::new(
                snapshot,
                &board_scope,
                scope,
                model.selected_part_ids.first().map(String::as_str),
                scope_generation,
            )
        })
}

#[component]
pub(super) fn PcbWorkspace() -> Element {
    let editor = use_context::<EditorContext>();
    let adapter = use_context::<SelectionAdapter>();
    let frame = use_frame();
    let interaction = use_context::<WorkspaceInteraction>();
    let runtime = editor.runtime();
    let EditorContext {
        version,
        workspace,
        instance_selection,
        scope_generation,
        ..
    } = editor;
    let CompactPanelState {
        mut objects_open,
        inspector_open: mut inspect_open,
    } = editor.panels;
    let model = runtime.model();
    let snapshot = &frame.snapshot;
    let render_scope = &frame.scope;
    let render_generation = frame.owner.generation;
    let inspector_panel_settings = interaction.inspector_settings;
    let layout_owner = &frame.owner;
    let layout_selection_kind = frame.status.layout.selection_kind();
    let layout_snap_settings = frame.status.layout.snap_settings();
    let layout_assembly_3d = frame.status.layout.assembly_3d();
    let part_placement = &frame.part_placement;
    let pcb_wiring_source = pcb_source(
        editor,
        &model,
        layout_owner.scope.clone(),
        snapshot,
        render_generation,
    );
    let guard = EditGuard {
        editor,
        workspace: Workspace::Pcb,
    };
    let wiring = use_context::<pcb_wiring::SharedPcbWiring>();
    let resolution_signal = wiring.resolution_signal;
    let resolution = wiring.current_resolution(&runtime, true);
    let part_net_actions = pcb_wiring::use_pcb_part_net_edits(guard);
    let part_input_actions = pcb_wiring::use_part_input_edits(guard);
    let firmware = pcb_wiring::use_firmware_position_edits(guard, resolution_signal);
    let firmware_feedback = firmware.feedback;
    let firmware_editable = firmware.editable;
    let firmware_on_edit = firmware.on_edit;
    let pcb_wiring_mode_actions = pcb_wiring::use_board_wiring_mode_edits(
        guard,
        pcb_wiring_source.clone(),
        resolution_signal,
    );
    let pcb_wiring_pin_actions =
        pcb_wiring::use_pcb_wiring_pin_edits(guard, pcb_wiring_source.clone(), resolution_signal);
    let pcb_wiring_protected_remap_actions = pcb_wiring::use_protected_remap_review(
        runtime.clone(),
        version,
        workspace,
        scope_generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
    );
    let pcb_wiring_apply_actions = pcb_wiring::use_board_wiring_apply(
        runtime.clone(),
        version,
        workspace,
        scope_generation,
        {
            let runtime = runtime.clone();
            Rc::new(move || instance_selection.is_current(&runtime.model()))
        },
        pcb_wiring_source.clone(),
        resolution_signal,
    );
    let pcb_wiring_edit_board = on_pcb_wiring_edit_board_handler(
        runtime.clone(),
        adapter.clone(),
        render_scope.clone(),
        render_generation,
        snapshot.token,
        workspace,
        instance_selection,
    );
    let selected_tree_context = adapter.selected_context.read().clone().filter(|selected| {
        &selected.scope == render_scope
            && selection::context_is_current(&model, &selected.scope, &selected.context)
    });
    let selected_module = selected_tree_context.as_ref().and_then(|selected| {
        if &selected.scope != render_scope {
            return None;
        }
        match &selected.context {
            objects::TreeContext::MountedModule {
                board_id,
                module_id,
            } if board_id == &render_scope.board_id => Some(module_id.clone()),
            _ => None,
        }
    });
    let inspector = if let Some(module_id) = selected_module {
        pcb_module_inspector::inspector(pcb_module_inspector::InspectorInput {
            runtime: runtime.clone(),
            snapshot: snapshot.clone(),
            scope: render_scope.clone(),
            module_id,
            selected_context: adapter.selected_context,
            on_place_component: part_placement.on_place_component,
        })
    } else if let Some(source) = pcb_wiring_source {
        let firmware_position_projection =
            pcb_wiring::firmware_position_projection(&source, render_generation, &resolution);
        let firmware_feedback = firmware_feedback.filter(|feedback| {
            feedback.target.is_visible(
                &source.ui_scope,
                render_generation,
                &firmware_position_projection,
            )
        });
        let firmware_controls = rsx! {
            firmware_positions::FirmwareKeymapPanel {
                projection: firmware_position_projection.clone(),
                feedback: firmware_feedback.clone(),
                editable: firmware_editable,
                on_change: firmware_on_edit,
            }
        };
        rsx! {
            PcbInspector { selected: source.active_part_id.is_some(),
                pcb_wiring::PcbWiringInspector {
                    board_overview: true, source, resolution,
                    firmware_positions: firmware_position_projection,
                    firmware_feedback, firmware_controls, part_net_actions, part_input_actions,
                    on_firmware_edit: firmware_on_edit, on_resolve: EventHandler::new(move |()| wiring.on_resolve.call(())),
                    on_choose_controller: part_placement.on_choose_controller,
                    on_edit_board_wiring: pcb_wiring_edit_board, mode_actions: pcb_wiring_mode_actions,
                    pin_actions: pcb_wiring_pin_actions, apply_actions: pcb_wiring_apply_actions,
                    protected_remap_actions: pcb_wiring_protected_remap_actions,
                }
            }
        }
    } else {
        rsx! {}
    };

    let mut shared = frame.objects.clone();
    let add_owner = LayoutOwnerIdentity {
        scope: Some(render_scope.clone()),
        token: Some(snapshot.token),
        revision: Some(snapshot.document.revision),
        generation: render_generation,
        workspace: Workspace::Pcb,
    };
    let add_is_current = pcb_add_layout_owner_is_current(&runtime, workspace, &adapter, &add_owner);
    shared.on_select = pcb_add_outline_select_handler(
        runtime.clone(),
        workspace,
        adapter.clone(),
        add_owner.clone(),
        layout_assembly_3d,
        shared.on_select,
    );
    let mut matrix_setup = frame.matrix_setup.clone();
    matrix_setup.can_open = add_is_current
        && matrix_setup.projection.is_none()
        && interaction.arbiter.current().is_none();
    matrix_setup.on_open = pcb_add_layout_open_handler(
        runtime.clone(),
        workspace,
        adapter.clone(),
        add_owner,
        layout_assembly_3d,
        matrix_setup.on_open,
    );

    let pcb_selection_kind = selection_handlers::on_selection_kind_handler(
        selection_handlers::SelectionKindInput {
            runtime: runtime.clone(),
            adapter: adapter.clone(),
            owner: layout_owner.clone(),
            tree_cell_anchor: interaction.tree_cell_anchor.clone(),
            objects_open,
            inspect_open,
            selection_kind: layout_selection_kind,
            workspace,
            inspector_panel_settings,
        },
        Workspace::Pcb,
    );
    let pcb_snap_intent = selection_handlers::on_snap_intent_handler(
        runtime.clone(),
        adapter.clone(),
        layout_owner.clone(),
        layout_snap_settings,
        workspace,
        Workspace::Pcb,
    );
    let align = objects::use_canvas_align(
        runtime.clone(),
        version,
        adapter.selected_context,
        workspace,
        scope_generation,
        Workspace::Pcb,
    );
    let toolbar_selection_availability =
        super::workspace_composition::use_toolbar_selection_availability(
            interaction.tree_cell_anchor.clone(),
            layout_owner.clone(),
            layout_canvas::SceneSource {
                document: snapshot.document.clone(),
                ..frame.camera.accepted_canvas_source.clone()
            },
            snapshot,
            &model,
            selected_tree_context.clone(),
            Some(Workspace::Pcb),
        );
    let show_position_inspector = frame.show_position_inspector;
    let properties_available = selected_tree_context.as_ref().is_some_and(|selected| {
        matches!(
            &selected.context,
            objects::TreeContext::Matrix { .. }
                | objects::TreeContext::Row { .. }
                | objects::TreeContext::Column { .. }
                | objects::TreeContext::Key { .. }
                | objects::TreeContext::Component {
                    part_id: Some(_),
                    ..
                }
        )
    }) && show_position_inspector;
    let on_show_properties = {
        let runtime = runtime.clone();
        let adapter = adapter.clone();
        let owner = layout_owner.clone();
        move |_| {
            if !pcb_owner_is_current(&runtime, workspace, &adapter, &owner) || !properties_available
            {
                return;
            }
            pin_inspector_on_desktop(inspector_panel_settings);
            objects_open.set(false);
            inspect_open.set(true);
        }
    };
    let transform = objects::LayoutTransformMenuMount {
        properties_available,
        column_available: toolbar_selection_availability.0,
        row_available: toolbar_selection_availability.1,
        pointer_tools_visible: false,
        pointer_tools_available: false,
        active_tool: None,
        matrix_rotate_followed: None,
        on_selection_kind: pcb_selection_kind,
        on_show_properties: EventHandler::new(on_show_properties),
        on_transform_tool: EventHandler::new(|_| {}),
    };
    let bounds = &frame.camera;
    let on_part_hit = interaction::part_hit(editor, adapter.clone(), &frame, &interaction);
    let on_part_pointer_down =
        interaction::part_pointer_down(editor, adapter.clone(), &frame, &interaction);
    let on_empty_hit = interaction::empty_hit(editor, adapter, &frame, &interaction);
    let surfaces = WorkspaceSurfaces {
        objects: objects(ObjectsInput {
            shared,
            matrix_setup,
            on_place_component: part_placement.on_place_component,
            layout_target: frame.layout_target,
            parts_query: frame.parts_query,
            on_browse_parts: frame.browse_parts,
            placement_error: frame.placement_error.clone(),
        }),
        inspector,
        toolbar: toolbar(ToolbarInput {
            command_label: "PCB commands".to_owned(),
            menu_owner_key: format!("{layout_owner:?}"),
            open_menu: frame.status.layout.command_menu(),
            selection_kind: layout_selection_kind(),
            snap_settings: layout_snap_settings.read().clone(),
            transform,
            align,
            on_selection_kind: pcb_selection_kind,
            on_snap_intent: pcb_snap_intent,
            show_relationships: false,
            has_selection_context: false,
            on_show_relationships: EventHandler::new(|()| {}),
        }),
        canvas: canvas(CanvasInput {
            snapshot: snapshot.clone(),
            scope: render_scope.clone(),
            view_box: bounds.view_box.clone(),
            selected_ids: model.selected_part_ids,
            generation: render_generation,
            handlers: frame.handlers.clone(),
            on_empty_hit,
            on_part_hit,
            on_part_pointer_down,
            on_module_select: frame.select_module,
        }),
        status: rsx! {},
        layers: rsx! { pcb_layers::PcbLayerControls { initially_open: false, snapshot: snapshot.clone(), scope: render_scope.clone() } },
    };
    use_publish_surfaces(Workspace::Pcb, surfaces);
    rsx! {}
}

pub(super) fn on_pcb_wiring_edit_board_handler(
    runtime: Rc<Runtime>,
    adapter: SelectionAdapter,
    scope: Scope,
    generation: u64,
    token: boardstudio_application::SnapshotToken,
    workspace: Signal<Workspace>,
    instance_selection: InstanceSelection,
) -> EventHandler<()> {
    EventHandler::new(move |()| {
        if workspace() != Workspace::Pcb
            || runtime.scope().as_ref() != Some(&scope)
            || (adapter.generation)() != generation
        {
            return;
        }
        let model = runtime.model();
        if !active_board_scope_matches(&model, &scope) || !instance_selection.is_current(&model) {
            return;
        }
        let Some(snapshot) = model
            .accepted
            .as_ref()
            .filter(|snapshot| snapshot.token == token)
        else {
            return;
        };
        if !snapshot
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return;
        }
        let mut selected_context = adapter.selected_context;
        selected_context.set(None);
        let mut anchor_scope = adapter.anchor_scope;
        anchor_scope.set(None);
        runtime.submit(boardstudio_application::Event::SelectParts {
            operation_id: runtime.operation(),
            part_ids: Vec::new(),
            range_part_ids: Vec::new(),
            mode: SelectionMode::Replace,
        });
    })
}
