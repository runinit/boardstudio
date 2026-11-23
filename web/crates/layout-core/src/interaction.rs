//! Layout canvas admission, modifier state and pointer transitions.

use boardstudio_model::model::EditOperation;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanvasMode {
    Panning,
    Marquee,
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
                | Self::Marquee
                | Self::DraggingParts
                | Self::DraggingMatrixRail
                | Self::DraggingPartRail
                | Self::DraggingOutlineHandle
        )
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Pointer {
        mode: CanvasMode,
        id: i32,
        client: (f64, f64),
        marquee_end: (f64, f64),
        moved: bool,
        quick_offset: bool,
        preview: Option<EditOperation>,
    },
    Mode {
        mode: CanvasMode,
        pointer: Option<i32>,
    },
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
        mode: CanvasMode,
        id: i32,
        client: (f64, f64),
        control: bool,
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarqueeBounds {
    pub start: (f64, f64),
    pub end: (f64, f64),
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
    PreviewMarquee(MarqueeBounds),
    SelectMarquee(MarqueeBounds),
    ModifiersChanged,
    PreventDefault,
}

impl CanvasInteraction {
    pub fn mode(&self) -> Option<CanvasMode> {
        match self.state {
            State::Idle => None,
            State::Pointer { mode, .. } | State::Mode { mode, .. } => Some(mode),
        }
    }

    pub fn pointer(&self) -> Option<i32> {
        match self.state {
            State::Pointer { id, .. } => Some(id),
            State::Mode { pointer, .. } => pointer,
            State::Idle => None,
        }
    }

    pub fn quick_offset(&self) -> bool {
        matches!(
            self.state,
            State::Pointer {
                quick_offset: true,
                ..
            }
        )
    }

    pub fn handle(&mut self, event: CanvasEvent) -> Vec<InteractionEffect> {
        use InteractionEffect::*;
        match event {
            CanvasEvent::Begin(mode) if self.mode().is_none() && !mode.is_pointer_gesture() => {
                self.state = State::Mode {
                    mode,
                    pointer: None,
                };
                Vec::new()
            }
            CanvasEvent::End(mode) if self.mode() == Some(mode) => {
                std::mem::take(&mut self.state);
                Vec::new()
            }
            CanvasEvent::PointerDown {
                mode,
                id,
                client,
                control,
            } => match &mut self.state {
                State::Idle
                    if mode.is_pointer_gesture()
                        && (mode != CanvasMode::Panning || self.space_down) =>
                {
                    self.state = State::Pointer {
                        mode,
                        id,
                        client,
                        marquee_end: client,
                        moved: false,
                        quick_offset: matches!(
                            mode,
                            CanvasMode::DraggingMatrixRail | CanvasMode::DraggingPartRail
                        ) && (control || self.control_down),
                        preview: None,
                    };
                    vec![CapturePointer(id)]
                }
                State::Mode {
                    mode: active,
                    pointer,
                } if *active == mode && pointer.is_none() => {
                    *pointer = Some(id);
                    Vec::new()
                }
                _ => Vec::new(),
            },
            CanvasEvent::PointerMove {
                id,
                client: point,
                operation,
            } => {
                let State::Pointer {
                    mode,
                    id: owner,
                    client,
                    marquee_end,
                    moved,
                    preview,
                    ..
                } = &mut self.state
                else {
                    return Vec::new();
                };
                if id != *owner {
                    return Vec::new();
                }
                if *mode == CanvasMode::Marquee && point != *marquee_end {
                    *marquee_end = point;
                    return vec![PreviewMarquee(MarqueeBounds {
                        start: *client,
                        end: point,
                    })];
                }
                let mut effects = Vec::new();
                if *mode == CanvasMode::DraggingParts && !*moved && point != *client {
                    *moved = true;
                    effects.push(StartPartDrag);
                }
                if let Some(operation) = operation
                    && *mode != CanvasMode::Marquee
                    && preview.as_ref() != Some(&operation)
                {
                    *preview = Some(operation.clone());
                    effects.push(Preview(operation));
                }
                effects
            }
            CanvasEvent::PointerUp { id, operation } if self.pointer() == Some(id) => {
                if let State::Mode { pointer, .. } = &mut self.state {
                    *pointer = None;
                    return Vec::new();
                }
                let State::Pointer {
                    mode,
                    preview,
                    client,
                    marquee_end,
                    ..
                } = std::mem::take(&mut self.state)
                else {
                    unreachable!();
                };
                let mut effects = vec![ReleasePointer(id)];
                if mode == CanvasMode::Panning {
                    effects.push(FinishPan);
                }
                if mode == CanvasMode::Marquee {
                    if client != marquee_end {
                        effects.push(SelectMarquee(MarqueeBounds {
                            start: client,
                            end: marquee_end,
                        }));
                    }
                    return effects;
                }
                let needs_preview = operation.is_some() && operation != preview;
                if let Some(operation) = operation.or(preview) {
                    effects.push(Commit {
                        operation,
                        needs_preview,
                    });
                }
                effects
            }
            CanvasEvent::PointerCancel(id) | CanvasEvent::LostPointerCapture(id)
                if self.pointer() == Some(id) =>
            {
                self.cancel()
            }
            CanvasEvent::KeyDown {
                key,
                layout: true,
                typing,
                prevented,
            } => match key {
                CanvasKey::Control if !self.control_down => {
                    self.control_down = true;
                    vec![ModifiersChanged]
                }
                CanvasKey::Space if !typing && !prevented => {
                    let changed = !std::mem::replace(&mut self.space_down, true);
                    let mut effects = vec![PreventDefault];
                    if changed {
                        effects.push(ModifiersChanged);
                    }
                    effects
                }
                CanvasKey::Escape if !typing && !prevented && self.mode().is_some() => {
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
        let mut effects = Vec::new();
        match std::mem::take(&mut self.state) {
            State::Idle => {}
            State::Mode { mode, .. } => effects.push(Cancel(mode)),
            State::Pointer {
                mode, id, preview, ..
            } => {
                effects.push(Cancel(mode));
                if preview.is_some() {
                    effects.push(ClearPreview);
                }
                effects.push(ReleasePointer(id));
            }
        }
        effects
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use CanvasMode::*;
    use InteractionEffect::*;

    const MODES: [CanvasMode; 12] = [
        Panning,
        Marquee,
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
            typing: false,
            prevented: false,
        }
    }

    fn down(mode: CanvasMode, id: i32) -> CanvasEvent {
        CanvasEvent::PointerDown {
            mode,
            id,
            client: (100.0, 200.0),
            control: false,
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
                    mode.is_pointer_gesture() && mode != Marquee
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
    fn space_admission_and_control_latching() {
        let mut canvas = CanvasInteraction::default();
        for (layout, typing, prevented) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            assert!(
                canvas
                    .handle(CanvasEvent::KeyDown {
                        key: CanvasKey::Space,
                        layout,
                        typing,
                        prevented
                    })
                    .is_empty()
            );
            assert!(canvas.handle(down(Panning, 1)).is_empty());
        }
        assert!(
            canvas
                .handle(key(CanvasKey::Space))
                .contains(&PreventDefault)
        );
        assert!(
            canvas
                .handle(key(CanvasKey::Space))
                .contains(&PreventDefault)
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
                    mode: DraggingPartRail,
                    id: 1,
                    client: (0.0, 0.0),
                    control: true,
                });
            }
            assert!(canvas.quick_offset());
            canvas.handle(CanvasEvent::KeyUp(CanvasKey::Control));
            assert!(!canvas.control_down);
            assert!(canvas.quick_offset());
            canvas.handle(CanvasEvent::Blur);
            assert!(!canvas.quick_offset());
        }
    }

    #[test]
    fn marquee_publishes_and_selects_only_the_admitted_rectangle() {
        let mut canvas = CanvasInteraction::default();
        start(&mut canvas, Marquee, 1);
        let bounds = MarqueeBounds {
            start: (100.0, 200.0),
            end: (50.0, 220.0),
        };
        let sample = CanvasEvent::PointerMove {
            id: 1,
            client: bounds.end,
            operation: None,
        };
        assert_eq!(canvas.handle(sample), [PreviewMarquee(bounds)]);
        assert_eq!(
            canvas.handle(CanvasEvent::PointerUp {
                id: 1,
                operation: None
            }),
            [ReleasePointer(1), SelectMarquee(bounds)]
        );
    }

    fn operation(id: &str) -> EditOperation {
        EditOperation::RemoveMatrix { id: id.into() }
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
