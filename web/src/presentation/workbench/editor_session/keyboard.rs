use super::super::*;

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

pub(super) fn build_keyboard_handlers(
    state: &super::session_state::SessionState,
    cam: &super::camera::CamNamed,
) -> (
    impl FnMut(KeyboardEvent) + Clone + 'static,
    impl FnMut(KeyboardEvent) + Clone + 'static,
) {
    let keyboard = keyboard_handler(
        state.matrix_pair.mirrored_pair.clone(),
        cam.placement.part_placement.clone(),
        state.matrix_pair.matrix_placement.clone(),
        state.layout_state.transform_tool(),
        state.layout_state.snap_settings(),
    );
    let input = use_context::<layout_input::LayoutInput>();
    let key_up = move |event| input.keyup(event);
    (keyboard, key_up)
}
