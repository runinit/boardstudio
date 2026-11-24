use super::super::*;
use super::EditorContext;

fn keyboard_handler(
    mirrored_pair: objects::MirroredPairSession,
    placement: part_placement::PartPlacementSession,
    matrix_placement: objects::MatrixPlacementSession,
    mut transform_tool: Signal<Option<objects::LayoutTransformTool>>,
    snap_settings: Signal<objects::LayoutSnapSettings>,
) -> impl FnMut(KeyboardEvent) + Clone + 'static {
    let interaction = use_context::<LayoutInteraction>();
    move |event: KeyboardEvent| {
        let key = event.key();
        let delta = match key {
            Key::ArrowLeft => Vec2 { x: -1.0, y: 0.0 },
            Key::ArrowRight => Vec2 { x: 1.0, y: 0.0 },
            Key::ArrowDown => Vec2 { x: 0.0, y: -1.0 },
            Key::ArrowUp => Vec2 { x: 0.0, y: 1.0 },
            Key::Enter | Key::Escape => Vec2::default(),
            _ => return,
        };
        let enter = key == Key::Enter;
        let step = |pitch: f64, default: f64| {
            let fraction = snap_settings.peek().snap_fraction;
            if fraction < 0.0 {
                -fraction
            } else if fraction > 0.0 {
                pitch * fraction
            } else {
                default
            }
        };
        let position = |center: Vec2, pitch: Vec2, default: Vec2| Vec2 {
            x: center.x + delta.x * step(pitch.x, default.x),
            y: center.y + delta.y * step(pitch.y, default.y),
        };
        let center = |at: Vec2, pitch: Vec2| {
            if enter {
                return at;
            }
            position(
                at,
                pitch,
                Vec2 {
                    x: pitch.x * 0.25,
                    y: pitch.y * 0.25,
                },
            )
        };
        match (interaction.current(), key) {
            (None, Key::Escape) if transform_tool.peek().is_some() => transform_tool.set(None),
            (_, Key::Escape) | (None, _) => return,
            (Some(CanvasMode::PlacingMatrix), _) => {
                let Some(active) = matrix_placement.placement.as_ref() else {
                    return;
                };
                let request = objects::MatrixPlacementMove {
                    owner: active.owner.clone(),
                    center: center(active.matrix.origin, active.matrix.pitch),
                };
                if enter {
                    matrix_placement.on_commit
                } else {
                    matrix_placement.on_move
                }
                .call(request);
            }
            (Some(CanvasMode::PlacingMirroredPair), _) => {
                let Some(active) = mirrored_pair.placement.as_ref() else {
                    return;
                };
                let request = objects::MirroredPairMove {
                    owner: active.owner.clone(),
                    center: center(active.pair.center, active.pair.matrix.pitch),
                };
                if enter {
                    mirrored_pair.on_commit
                } else {
                    mirrored_pair.on_move
                }
                .call(request);
            }
            (Some(CanvasMode::PlacingPart), _) => {
                if let Some(active) = placement.projection.as_ref() {
                    if enter {
                        placement.on_commit.call(active.pending.at);
                    } else {
                        placement.on_move.call(position(
                            active.pending.at,
                            Vec2 { x: 19.05, y: 19.05 },
                            Vec2 { x: 0.1, y: 0.1 },
                        ));
                    }
                }
            }
            _ => return,
        }
        event.prevent_default();
    }
}

pub(super) fn build_keyboard_bundle(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
    render_scope: &Scope,
) -> (
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(PointerEvent) + Clone + 'static,
) {
    let editor = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let runtime = &editor.runtime();
    let svg = &cam.svg_surface.svg;
    let mirrored_pair = &state.matrix_pair.mirrored_pair;
    let part_placement = &cam.placement.part_placement;
    let matrix_placement = &state.matrix_pair.matrix_placement;
    let layout_snap_settings = state.layout_state.snap_settings();
    let pending_splay_origin_pick = state.layout_state.pending_splay_origin_pick();
    let matrix_transform_inspector = &state.matrix_transform_inspector;
    let adapter = &state.adapter;
    let layout_owner = &state.view.layout_owner;
    let workspace = editor.workspace;
    let view_x = cam.scene_bounds.view_x;
    let view_y = cam.scene_bounds.view_y;
    let width = cam.scene_bounds.width;
    let height = cam.scene_bounds.height;
    let render_generation = state.render_generation;
    let keyboard = keyboard_handler(
        mirrored_pair.clone(),
        part_placement.clone(),
        matrix_placement.clone(),
        state.layout_state.transform_tool(),
        layout_snap_settings,
    );
    let input = use_context::<layout_input::LayoutInput>();
    let key_up = move |event| input.keyup(event);
    let start_pan =
        super::canvas_adapter::start_pan_handler(super::canvas_adapter::StartPanInput {
            render_scope: render_scope.clone(),
            render_generation,
            runtime: runtime.clone(),
            svg: svg.clone(),
            adapter: adapter.clone(),
            mirrored_pair: mirrored_pair.clone(),
            pending_splay_pick: pending_splay_origin_pick,
            transform_inspector: matrix_transform_inspector.clone(),
            layout_owner_for_pick: layout_owner.clone(),
            workspace,
            view_x,
            view_y,
            width,
            height,
        });
    (keyboard, key_up, start_pan)
}

pub(in crate::presentation) fn build_wheel(
    editor: EditorContext,
    adapter: &SelectionAdapter,
    scope: &Scope,
    generation: u64,
    svg: &Rc<RefCell<Option<SvgElement>>>,
    bounds: &super::camera::SceneBounds,
) -> impl FnMut(WheelEvent) + Clone + 'static {
    super::canvas_adapter::wheel_handler(super::canvas_adapter::WheelInput {
        runtime: editor.runtime(),
        svg: svg.clone(),
        scope: scope.clone(),
        adapter: adapter.clone(),
        render_generation: generation,
        view_x: bounds.view_x,
        view_y: bounds.view_y,
        width: bounds.width,
        height: bounds.height,
        keymap_scale_ratio: bounds.keymap_scale_ratio,
        min_x: bounds.min_x,
        max_x: bounds.max_x,
        min_y: bounds.min_y,
        max_y: bounds.max_y,
    })
}
