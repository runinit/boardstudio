//! Layout canvas admission, modifier state and pointer transitions.

use boardstudio_application::Scope;
use boardstudio_model::model::{EditOperation, Position, Vec2};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasMode {
    Panning,
    DraggingParts,
    DraggingMatrixRail,
    DraggingPartRail,
    PickingSplayOrigin,
    PickingMatrixAnchor,
    PickingRailAnchor,
    PlacingPart,
    PlacingMatrix,
    PlacingMirroredPair,
    DraggingOutlineHandle,
}

impl CanvasMode {
    fn is_pointer_gesture(self) -> bool {
        matches!(
            self,
            Self::Panning
                | Self::DraggingParts
                | Self::DraggingMatrixRail
                | Self::DraggingPartRail
                | Self::DraggingOutlineHandle
        )
    }

    pub fn is_pick(self) -> bool {
        matches!(
            self,
            Self::PickingSplayOrigin | Self::PickingMatrixAnchor | Self::PickingRailAnchor
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum GestureSeed {
    Pan {
        scope: Scope,
        generation: u64,
        start_camera: Vec2,
    },
    PartDrag {
        scope: Scope,
        generation: u64,
        origin_mm: Vec2,
        positions: Vec<Position>,
    },
    MatrixRail,
    PartRail,
    OutlineHandle,
}

impl GestureSeed {
    pub fn mode(&self) -> CanvasMode {
        match self {
            Self::Pan { .. } => CanvasMode::Panning,
            Self::PartDrag { .. } => CanvasMode::DraggingParts,
            Self::MatrixRail => CanvasMode::DraggingMatrixRail,
            Self::PartRail => CanvasMode::DraggingPartRail,
            Self::OutlineHandle => CanvasMode::DraggingOutlineHandle,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivePan {
    pub pointer_id: i32,
    pub scope: Scope,
    pub generation: u64,
    pub origin_px: (f64, f64),
    pub start_camera: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActivePartDrag {
    pub pointer_id: i32,
    pub scope: Scope,
    pub generation: u64,
    pub gesture_generation: Option<u64>,
    pub origin_px: (f64, f64),
    pub origin_mm: Vec2,
    pub positions: Vec<Position>,
    pub last_sample: Option<(Vec2, f64, bool)>,
    pub preview: Option<EditOperation>,
    pub active: bool,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Panning(ActivePan),
    DraggingParts(ActivePartDrag),
    DraggingMatrixRail {
        id: i32,
        quick_offset: bool,
        preview: Option<EditOperation>,
    },
    DraggingPartRail {
        id: i32,
        quick_offset: bool,
        preview: Option<EditOperation>,
    },
    DraggingOutlineHandle {
        id: i32,
        preview: Option<EditOperation>,
    },
    Armed {
        mode: CanvasMode,
        pointer: Option<i32>,
    },
}

impl State {
    fn mode(&self) -> Option<CanvasMode> {
        match self {
            State::Idle => None,
            State::Panning(_) => Some(CanvasMode::Panning),
            State::DraggingParts(_) => Some(CanvasMode::DraggingParts),
            State::DraggingMatrixRail { .. } => Some(CanvasMode::DraggingMatrixRail),
            State::DraggingPartRail { .. } => Some(CanvasMode::DraggingPartRail),
            State::DraggingOutlineHandle { .. } => Some(CanvasMode::DraggingOutlineHandle),
            State::Armed { mode, .. } => Some(*mode),
        }
    }

    fn pointer(&self) -> Option<i32> {
        match self {
            State::Idle => None,
            State::Panning(pan) => Some(pan.pointer_id),
            State::DraggingParts(drag) => Some(drag.pointer_id),
            State::DraggingMatrixRail { id, .. }
            | State::DraggingPartRail { id, .. }
            | State::DraggingOutlineHandle { id, .. } => Some(*id),
            State::Armed { pointer, .. } => *pointer,
        }
    }
}

#[derive(Default)]
pub struct CanvasInteraction {
    state: State,
    pub space_down: bool,
    pub control_down: bool,
}

pub enum CanvasEvent {
    Begin(CanvasMode),
    End(CanvasMode),
    PointerDown {
        id: i32,
        client: (f64, f64),
        control: bool,
        seed: Option<GestureSeed>,
    },
    PointerMove {
        id: i32,
        client: (f64, f64),
        operation: Option<EditOperation>,
    },
    PointerUp {
        id: i32,
        operation: Option<EditOperation>,
    },
    PointerCancel(i32),
    LostPointerCapture(i32),
    KeyDown {
        key: CanvasKey,
        layout: bool,
        canvas: bool,
        typing: bool,
        prevented: bool,
    },
    KeyUp(CanvasKey),
    Blur,
    ScopeChanged,
    WorkspaceChanged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasKey {
    Space,
    Control,
    Escape,
    Other,
}

impl CanvasKey {
    pub fn new(key: &str, code: &str) -> Self {
        match key {
            "Control" => Self::Control,
            "Escape" => Self::Escape,
            " " => Self::Space,
            _ if code == "Space" => Self::Space,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, PartialEq)]
pub enum InteractionEffect {
    CapturePointer(i32),
    ReleasePointer(i32),
    StartPartDrag,
    Preview(EditOperation),
    Commit {
        operation: EditOperation,
        needs_preview: bool,
    },
    ClearPreview,
    Cancel(CanvasMode),
    FinishPan,
    ModifiersChanged,
    PreventDefault,
}

impl CanvasInteraction {
    pub fn mode(&self) -> Option<CanvasMode> {
        self.state.mode()
    }

    pub fn pointer(&self) -> Option<i32> {
        self.state.pointer()
    }

    pub fn quick_offset(&self) -> bool {
        matches!(
            self.state,
            State::DraggingMatrixRail {
                quick_offset: true,
                ..
            } | State::DraggingPartRail {
                quick_offset: true,
                ..
            }
        )
    }

    pub fn pan(&self) -> Option<ActivePan> {
        match &self.state {
            State::Panning(pan) => Some(pan.clone()),
            _ => None,
        }
    }

    pub fn part_drag(&self) -> Option<ActivePartDrag> {
        match &self.state {
            State::DraggingParts(drag) => Some(drag.clone()),
            _ => None,
        }
    }

    pub fn set_part_drag_gesture_generation(&mut self, generation: Option<u64>) {
        if let State::DraggingParts(drag) = &mut self.state {
            drag.gesture_generation = generation;
        }
    }

    pub fn note_part_drag_sample(&mut self, sample: Option<(Vec2, f64, bool)>) {
        if let State::DraggingParts(drag) = &mut self.state {
            drag.last_sample = sample;
        }
    }

    pub fn sync_control(&mut self, down: bool) -> bool {
        if self.control_down == down {
            return false;
        }
        self.control_down = down;
        true
    }

    pub fn handle(&mut self, event: CanvasEvent) -> Vec<InteractionEffect> {
        use InteractionEffect::*;
        match event {
            CanvasEvent::Begin(mode) if self.mode().is_none() && !mode.is_pointer_gesture() => {
                self.state = State::Armed {
                    mode,
                    pointer: None,
                };
                Vec::new()
            }
            CanvasEvent::End(mode) if self.mode() == Some(mode) => {
                self.state = State::Idle;
                Vec::new()
            }
            CanvasEvent::PointerDown {
                id,
                client,
                control,
                seed,
            } => match &mut self.state {
                State::Idle => {
                    let Some(seed) = seed else {
                        return Vec::new();
                    };
                    if seed.mode() == CanvasMode::Panning && !self.space_down {
                        return Vec::new();
                    }
                    let quick_offset =
                        matches!(seed, GestureSeed::MatrixRail | GestureSeed::PartRail)
                            && (control || self.control_down);
                    self.state = match seed {
                        GestureSeed::Pan {
                            scope,
                            generation,
                            start_camera,
                        } => State::Panning(ActivePan {
                            pointer_id: id,
                            scope,
                            generation,
                            origin_px: client,
                            start_camera,
                        }),
                        GestureSeed::PartDrag {
                            scope,
                            generation,
                            origin_mm,
                            positions,
                        } => State::DraggingParts(ActivePartDrag {
                            pointer_id: id,
                            scope,
                            generation,
                            gesture_generation: None,
                            origin_px: client,
                            origin_mm,
                            positions,
                            last_sample: None,
                            preview: None,
                            active: false,
                        }),
                        GestureSeed::MatrixRail => State::DraggingMatrixRail {
                            id,
                            quick_offset,
                            preview: None,
                        },
                        GestureSeed::PartRail => State::DraggingPartRail {
                            id,
                            quick_offset,
                            preview: None,
                        },
                        GestureSeed::OutlineHandle => {
                            State::DraggingOutlineHandle { id, preview: None }
                        }
                    };
                    vec![CapturePointer(id)]
                }
                State::Armed { pointer, .. } if seed.is_none() && pointer.is_none() => {
                    *pointer = Some(id);
                    Vec::new()
                }
                _ => Vec::new(),
            },
            CanvasEvent::PointerMove {
                id,
                client: point,
                operation,
            } => match &mut self.state {
                State::DraggingParts(drag) if drag.pointer_id == id => {
                    let mut effects = Vec::new();
                    if !drag.active && point != drag.origin_px {
                        drag.active = true;
                        effects.push(StartPartDrag);
                    }
                    if let Some(operation) = operation
                        && drag.preview.as_ref() != Some(&operation)
                    {
                        drag.preview = Some(operation.clone());
                        effects.push(Preview(operation));
                    }
                    effects
                }
                State::DraggingMatrixRail {
                    id: owner, preview, ..
                }
                | State::DraggingPartRail {
                    id: owner, preview, ..
                }
                | State::DraggingOutlineHandle {
                    id: owner, preview, ..
                } if *owner == id => {
                    if let Some(operation) = operation
                        && preview.as_ref() != Some(&operation)
                    {
                        *preview = Some(operation.clone());
                        vec![Preview(operation)]
                    } else {
                        Vec::new()
                    }
                }
                _ => Vec::new(),
            },
            CanvasEvent::PointerUp { id, operation } if self.pointer() == Some(id) => {
                if let State::Armed { pointer, .. } = &mut self.state {
                    *pointer = None;
                    return Vec::new();
                }
                match std::mem::take(&mut self.state) {
                    State::Panning(_) => vec![ReleasePointer(id), FinishPan],
                    State::DraggingParts(drag) => release_commit(id, operation, drag.preview),
                    State::DraggingMatrixRail { preview, .. }
                    | State::DraggingPartRail { preview, .. }
                    | State::DraggingOutlineHandle { preview, .. } => {
                        release_commit(id, operation, preview)
                    }
                    State::Idle | State::Armed { .. } => Vec::new(),
                }
            }
            CanvasEvent::PointerCancel(id) | CanvasEvent::LostPointerCapture(id)
                if self.pointer() == Some(id) =>
            {
                self.cancel()
            }
            CanvasEvent::KeyDown {
                key,
                layout,
                canvas,
                typing,
                prevented,
            } => match key {
                CanvasKey::Control if layout && !self.control_down => {
                    self.control_down = true;
                    vec![ModifiersChanged]
                }
                CanvasKey::Space if canvas && !typing && !prevented => {
                    let changed = !std::mem::replace(&mut self.space_down, true);
                    let mut effects = vec![PreventDefault];
                    if changed {
                        effects.push(ModifiersChanged);
                    }
                    effects
                }
                CanvasKey::Escape if canvas && !typing && !prevented && self.mode().is_some() => {
                    let mut effects = self.cancel();
                    effects.push(PreventDefault);
                    effects
                }
                _ => Vec::new(),
            },
            CanvasEvent::KeyUp(key) => {
                let changed = match key {
                    CanvasKey::Space => std::mem::take(&mut self.space_down),
                    CanvasKey::Control => std::mem::take(&mut self.control_down),
                    _ => false,
                };
                if changed {
                    vec![ModifiersChanged]
                } else {
                    Vec::new()
                }
            }
            CanvasEvent::ScopeChanged => self.cancel(),
            CanvasEvent::Blur | CanvasEvent::WorkspaceChanged => {
                let mut effects = self.cancel();
                if std::mem::take(&mut self.space_down) | std::mem::take(&mut self.control_down) {
                    effects.push(ModifiersChanged);
                }
                effects
            }
            _ => Vec::new(),
        }
    }

    fn cancel(&mut self) -> Vec<InteractionEffect> {
        use InteractionEffect::*;
        let Some(mode) = self.state.mode() else {
            return Vec::new();
        };
        let (id, had_preview) = match std::mem::take(&mut self.state) {
            State::Idle => return Vec::new(),
            State::Armed { .. } => return vec![Cancel(mode)],
            State::Panning(pan) => (pan.pointer_id, false),
            State::DraggingParts(drag) => (drag.pointer_id, drag.preview.is_some()),
            State::DraggingMatrixRail { id, preview, .. }
            | State::DraggingPartRail { id, preview, .. }
            | State::DraggingOutlineHandle { id, preview, .. } => (id, preview.is_some()),
        };
        let mut effects = vec![Cancel(mode)];
        if had_preview {
            effects.push(ClearPreview);
        }
        effects.push(ReleasePointer(id));
        effects
    }
}

fn release_commit(
    id: i32,
    operation: Option<EditOperation>,
    preview: Option<EditOperation>,
) -> Vec<InteractionEffect> {
    use InteractionEffect::*;
    let mut effects = vec![ReleasePointer(id)];
    let needs_preview = operation.is_some() && operation != preview;
    if let Some(operation) = operation.or(preview) {
        effects.push(Commit {
            operation,
            needs_preview,
        });
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use CanvasMode::*;
    use InteractionEffect::*;

    const MODES: [CanvasMode; 11] = [
        Panning,
        DraggingParts,
        DraggingMatrixRail,
        DraggingPartRail,
        PickingSplayOrigin,
        PickingMatrixAnchor,
        PickingRailAnchor,
        PlacingPart,
        PlacingMatrix,
        PlacingMirroredPair,
        DraggingOutlineHandle,
    ];

    fn key(key: CanvasKey) -> CanvasEvent {
        CanvasEvent::KeyDown {
            key,
            layout: true,
            canvas: true,
            typing: false,
            prevented: false,
        }
    }

    fn scope() -> Scope {
        Scope {
            session_epoch: boardstudio_application::SessionEpoch(7),
            document_id: "p".into(),
            board_id: "b".into(),
            instance_id: None,
        }
    }

    fn seed_for(mode: CanvasMode, generation: u64) -> Option<GestureSeed> {
        match mode {
            Panning => Some(GestureSeed::Pan {
                scope: scope(),
                generation,
                start_camera: Vec2 { x: 7.0, y: 8.0 },
            }),
            DraggingParts => Some(GestureSeed::PartDrag {
                scope: scope(),
                generation,
                origin_mm: Vec2 { x: 4.0, y: 5.0 },
                positions: vec![Position {
                    id: "part".into(),
                    at: Vec2 { x: 4.0, y: 5.0 },
                }],
            }),
            DraggingMatrixRail => Some(GestureSeed::MatrixRail),
            DraggingPartRail => Some(GestureSeed::PartRail),
            DraggingOutlineHandle => Some(GestureSeed::OutlineHandle),
            _ => None,
        }
    }

    fn down(mode: CanvasMode, id: i32) -> CanvasEvent {
        CanvasEvent::PointerDown {
            id,
            client: (100.0, 200.0),
            control: false,
            seed: seed_for(mode, 3),
        }
    }

    fn start(canvas: &mut CanvasInteraction, mode: CanvasMode, id: i32) {
        if mode.is_pointer_gesture() {
            assert_eq!(canvas.handle(down(mode, id)), [CapturePointer(id)]);
        } else {
            canvas.handle(CanvasEvent::Begin(mode));
            canvas.handle(down(mode, id));
        }
        assert_eq!(canvas.mode(), Some(mode));
        assert_eq!(canvas.pointer(), Some(id));
    }

    fn operation(id: &str) -> EditOperation {
        EditOperation::RemoveMatrix { id: id.into() }
    }

    #[test]
    fn every_mode_rejects_competing_starts_and_second_pointers() {
        for active in MODES {
            let mut canvas = CanvasInteraction::default();
            canvas.handle(key(CanvasKey::Space));
            start(&mut canvas, active, 1);
            for contender in MODES {
                canvas.handle(CanvasEvent::Begin(contender));
                canvas.handle(down(contender, 2));
                assert_eq!(canvas.mode(), Some(active));
                assert_eq!(canvas.pointer(), Some(1));
            }
            for event in [
                CanvasEvent::PointerMove {
                    id: 2,
                    client: (101.0, 200.0),
                    operation: Some(operation("other")),
                },
                CanvasEvent::PointerUp {
                    id: 2,
                    operation: Some(operation("other")),
                },
                CanvasEvent::PointerCancel(2),
                CanvasEvent::LostPointerCapture(2),
            ] {
                assert!(canvas.handle(event).is_empty());
                assert_eq!(canvas.pointer(), Some(1));
            }
            let drop = canvas.handle(CanvasEvent::PointerUp {
                id: 1,
                operation: None,
            });
            if active.is_pointer_gesture() {
                assert!(drop.contains(&ReleasePointer(1)));
                assert_eq!(drop.contains(&FinishPan), active == Panning);
                assert_eq!(canvas.mode(), None);
            } else {
                assert_eq!(canvas.pointer(), None);
                assert_eq!(canvas.mode(), Some(active));
                canvas.handle(CanvasEvent::End(active));
                assert_eq!(canvas.mode(), None);
            }
        }
    }

    #[test]
    fn every_mode_cancels_without_committing_and_accepts_a_new_gesture() {
        for mode in MODES {
            for (event, reset_modifiers) in [
                (key(CanvasKey::Escape), false),
                (CanvasEvent::Blur, true),
                (CanvasEvent::WorkspaceChanged, true),
                (CanvasEvent::ScopeChanged, false),
                (CanvasEvent::PointerCancel(1), false),
                (CanvasEvent::LostPointerCapture(1), false),
            ] {
                let mut canvas = CanvasInteraction::default();
                canvas.handle(key(CanvasKey::Space));
                canvas.handle(key(CanvasKey::Control));
                start(&mut canvas, mode, 1);
                if mode.is_pointer_gesture() {
                    canvas.handle(CanvasEvent::PointerMove {
                        id: 1,
                        client: (101.0, 200.0),
                        operation: Some(operation("preview")),
                    });
                }
                let effects = canvas.handle(event);
                assert!(effects.contains(&Cancel(mode)));
                assert_eq!(
                    effects.contains(&ClearPreview),
                    mode.is_pointer_gesture() && mode != Panning
                );
                assert_eq!(
                    effects.contains(&ReleasePointer(1)),
                    mode.is_pointer_gesture()
                );
                assert!(!effects.iter().any(|effect| matches!(effect, Commit { .. })));
                assert_eq!(canvas.mode(), None);
                assert_eq!(canvas.space_down, !reset_modifiers);
                assert_eq!(canvas.control_down, !reset_modifiers);
                start(&mut canvas, DraggingParts, 2);
            }
        }
    }

    #[test]
    fn uncaptured_armed_mode_cancel_emits_no_release_pointer() {
        for mode in MODES.into_iter().filter(|mode| !mode.is_pointer_gesture()) {
            for (event, escape) in [
                (key(CanvasKey::Escape), true),
                (CanvasEvent::Blur, false),
                (CanvasEvent::WorkspaceChanged, false),
                (CanvasEvent::ScopeChanged, false),
            ] {
                let mut canvas = CanvasInteraction::default();
                canvas.handle(CanvasEvent::Begin(mode));
                assert_eq!((canvas.mode(), canvas.pointer()), (Some(mode), None));
                assert!(canvas.handle(CanvasEvent::PointerCancel(1)).is_empty());
                assert!(canvas.handle(CanvasEvent::LostPointerCapture(1)).is_empty());
                // Competing seeded PointerDown must not capture an armed mode's pointer slot.
                assert!(canvas.handle(down(DraggingParts, 2)).is_empty());
                assert_eq!((canvas.mode(), canvas.pointer()), (Some(mode), None));

                let effects = canvas.handle(event);
                let mut expected = vec![Cancel(mode)];
                if escape {
                    expected.push(PreventDefault);
                }
                assert_eq!(effects, expected);
                assert_eq!(canvas.mode(), None);
            }
        }
    }

    #[test]
    fn space_admission_and_control_latching() {
        let mut canvas = CanvasInteraction::default();
        for (active_canvas, typing, prevented) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            assert!(
                canvas
                    .handle(CanvasEvent::KeyDown {
                        key: CanvasKey::Space,
                        layout: active_canvas,
                        canvas: active_canvas,
                        typing,
                        prevented,
                    })
                    .is_empty()
            );
            assert!(canvas.handle(down(Panning, 1)).is_empty());
        }
        assert!(
            canvas
                .handle(CanvasEvent::KeyDown {
                    key: CanvasKey::Space,
                    layout: false,
                    canvas: true,
                    typing: false,
                    prevented: false,
                })
                .contains(&PreventDefault)
        );
        assert!(canvas.space_down);
        assert!(
            canvas
                .handle(CanvasEvent::KeyDown {
                    key: CanvasKey::Control,
                    layout: false,
                    canvas: true,
                    typing: false,
                    prevented: false,
                })
                .is_empty()
        );
        start(&mut canvas, Panning, 9);
        assert!(
            canvas
                .handle(CanvasEvent::KeyDown {
                    key: CanvasKey::Escape,
                    layout: false,
                    canvas: true,
                    typing: false,
                    prevented: false,
                })
                .contains(&Cancel(Panning))
        );
        assert_eq!(
            canvas.handle(CanvasEvent::KeyUp(CanvasKey::Space)),
            [ModifiersChanged]
        );
        assert!(!canvas.space_down);
        for from_keydown in [false, true] {
            if from_keydown {
                assert_eq!(canvas.handle(key(CanvasKey::Control)), [ModifiersChanged]);
                canvas.handle(down(DraggingMatrixRail, 1));
            } else {
                canvas.handle(CanvasEvent::PointerDown {
                    id: 1,
                    client: (0.0, 0.0),
                    control: true,
                    seed: Some(GestureSeed::PartRail),
                });
            }
            assert!(canvas.quick_offset());
            if from_keydown {
                assert_eq!(
                    canvas.handle(CanvasEvent::KeyUp(CanvasKey::Control)),
                    [ModifiersChanged]
                );
                assert!(canvas.sync_control(true));
            }
            assert_eq!(canvas.sync_control(false), from_keydown);
            assert!(!canvas.control_down);
            assert!(canvas.quick_offset());
            canvas.handle(CanvasEvent::Blur);
            assert!(!canvas.quick_offset());
        }
    }

    #[test]
    fn atomic_gesture_seed_pointer_down_transitions_and_payloads() {
        let mut canvas = CanvasInteraction::default();
        // Seedless PointerDown while Idle does nothing.
        assert!(
            canvas
                .handle(CanvasEvent::PointerDown {
                    id: 1,
                    client: (100.0, 200.0),
                    control: false,
                    seed: None,
                })
                .is_empty()
        );
        assert_eq!(canvas.mode(), None);

        // Pan seed requires space_down and atomically populates ActivePan.
        assert!(canvas.handle(down(Panning, 1)).is_empty());
        assert_eq!(canvas.mode(), None);
        assert!(
            canvas
                .handle(key(CanvasKey::Space))
                .contains(&PreventDefault)
        );
        assert_eq!(
            canvas.handle(CanvasEvent::PointerDown {
                id: 1,
                client: (100.0, 200.0),
                control: false,
                seed: seed_for(Panning, 4),
            }),
            [CapturePointer(1)]
        );
        let pan = canvas.pan().expect("pan payload");
        assert_eq!((pan.pointer_id, pan.scope, pan.generation), (1, scope(), 4));
        assert_eq!(pan.origin_px, (100.0, 200.0));
        assert_eq!(pan.start_camera, Vec2 { x: 7.0, y: 8.0 });
        assert_eq!(
            canvas.handle(CanvasEvent::PointerUp {
                id: 1,
                operation: None
            }),
            [ReleasePointer(1), FinishPan]
        );

        // PartDrag seed atomically populates ActivePartDrag and tracks generation/sample updates.
        assert_eq!(
            canvas.handle(CanvasEvent::PointerDown {
                id: 2,
                client: (100.0, 200.0),
                control: false,
                seed: seed_for(DraggingParts, 5),
            }),
            [CapturePointer(2)]
        );
        let drag = canvas.part_drag().expect("drag payload");
        assert_eq!(drag.pointer_id, 2);
        assert_eq!(drag.generation, 5);
        assert!(!drag.active);
        assert_eq!(drag.origin_mm, Vec2 { x: 4.0, y: 5.0 });
        canvas.set_part_drag_gesture_generation(Some(11));
        canvas.note_part_drag_sample(Some((Vec2 { x: 6.0, y: 7.0 }, 2.0, false)));
        let drag = canvas.part_drag().unwrap();
        assert_eq!(drag.gesture_generation, Some(11));
        assert_eq!(
            drag.last_sample,
            Some((Vec2 { x: 6.0, y: 7.0 }, 2.0, false))
        );
        assert_eq!(
            canvas.handle(CanvasEvent::PointerUp {
                id: 2,
                operation: None
            }),
            [ReleasePointer(2)]
        );

        // Rail and outline seeds transition directly to their respective modes.
        for (seed, mode) in [
            (GestureSeed::MatrixRail, DraggingMatrixRail),
            (GestureSeed::PartRail, DraggingPartRail),
            (GestureSeed::OutlineHandle, DraggingOutlineHandle),
        ] {
            assert_eq!(
                canvas.handle(CanvasEvent::PointerDown {
                    id: 3,
                    client: (10.0, 20.0),
                    control: false,
                    seed: Some(seed),
                }),
                [CapturePointer(3)]
            );
            assert_eq!((canvas.mode(), canvas.pointer()), (Some(mode), Some(3)));
            assert_eq!(
                canvas.handle(CanvasEvent::PointerUp {
                    id: 3,
                    operation: None
                }),
                [ReleasePointer(3)]
            );
        }
    }

    #[test]
    fn part_drag_requires_real_client_motion_and_activates_once() {
        let mut canvas = CanvasInteraction::default();
        start(&mut canvas, DraggingParts, 1);
        let sample = |x| CanvasEvent::PointerMove {
            id: 1,
            client: (x, 200.0),
            operation: None,
        };
        assert!(canvas.handle(sample(100.0)).is_empty());
        assert_eq!(canvas.handle(sample(101.0)), [StartPartDrag]);
        assert!(canvas.handle(sample(102.0)).is_empty());
    }

    #[test]
    fn drop_reuses_the_preview_unless_release_supplies_a_new_operation() {
        for release in [None, Some("preview"), Some("release")] {
            let mut canvas = CanvasInteraction::default();
            start(&mut canvas, DraggingMatrixRail, 1);
            let sample = || CanvasEvent::PointerMove {
                id: 1,
                client: (100.0, 200.0),
                operation: Some(operation("preview")),
            };
            assert_eq!(canvas.handle(sample()), [Preview(operation("preview"))]);
            assert!(canvas.handle(sample()).is_empty());
            assert_eq!(
                canvas.handle(CanvasEvent::PointerUp {
                    id: 1,
                    operation: release.map(operation)
                }),
                [
                    ReleasePointer(1),
                    Commit {
                        operation: operation(release.unwrap_or("preview")),
                        needs_preview: release == Some("release"),
                    }
                ]
            );
        }
    }
}
