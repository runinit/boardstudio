use super::*;
use crate::interactions::{DragSnapOptions, MagneticSnapState, normalize_drag};

#[derive(Clone, Debug)]
pub(super) struct Gesture {
    pub(super) pointer_id: i64,
    generation: u64,
    transaction_id: String,
    base_revision: u64,
    targets: Vec<String>,
    start: Vec<Position>,
    latest: Vec<Position>,
    sent_positions: Option<Vec<Position>>,
    operation_id: OperationId,
    pitch: Vec2,
    snap_fraction: f64,
    geometry_snap: bool,
    gap: Option<f64>,
    pixels_per_world: Option<f64>,
    snap_state: MagneticSnapState,
    alt: bool,
    frame_pending: bool,
    changed: bool,
}

impl Session {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin_gesture(
        &mut self,
        operation_id: OperationId,
        pointer_id: i64,
        target_ids: Vec<String>,
        transaction_id: String,
        start: Vec<Position>,
        pitch: Vec2,
        snap_fraction: f64,
        geometry_snap: bool,
        gap: Option<f64>,
        pixels_per_world: Option<f64>,
        alt: bool,
        effects: &mut Vec<Effect>,
    ) {
        if self.gesture.is_none() && self.model.lifecycle == Lifecycle::Ready {
            let generation = self.next_gesture;
            self.next_gesture += 1;
            let base_revision = self
                .model
                .accepted
                .as_ref()
                .map_or(0, |s| s.document.revision);
            self.gesture = Some(Gesture {
                operation_id,
                pointer_id,
                generation,
                transaction_id,
                base_revision,
                targets: target_ids,
                start: start.clone(),
                latest: start,
                sent_positions: None,
                pitch,
                snap_fraction,
                geometry_snap,
                gap,
                pixels_per_world,
                snap_state: MagneticSnapState::default(),
                alt,
                frame_pending: false,
                changed: false,
            });
            self.model.gesture = self.gesture.as_ref().map(gesture_view);
            effects.push(Effect::CapturePointer { pointer_id });
        }
    }
    pub(super) fn gesture_sample(
        &mut self,
        pointer_id: i64,
        positions: Vec<Position>,
        pixels_per_world: Option<f64>,
        alt: bool,
        end: bool,
        effects: &mut Vec<Effect>,
    ) {
        let Some((g, snapshot)) = self
            .gesture
            .as_mut()
            .zip(self.model.accepted.as_ref())
            .filter(|(gesture, _)| gesture.pointer_id == pointer_id)
        else {
            return;
        };
        g.pixels_per_world = pixels_per_world.or(g.pixels_per_world);
        g.alt = alt;
        let (positions, guide) = normalize_drag(
            &snapshot.document,
            &g.start,
            positions,
            DragSnapOptions {
                board_id: &self.model.active_board_id,
                pitch: g.pitch,
                fraction: g.snap_fraction,
                geometry_snap: g.geometry_snap,
                gap: g.gap,
                targets: &g.targets,
                alt,
                pixels_per_world: g.pixels_per_world,
                state: &mut g.snap_state,
            },
        );
        self.model.snap_guide = guide;
        g.changed = positions != g.start;
        g.latest = positions;
        let generation = g.generation;
        let changed = g.changed;
        if !g.frame_pending && !end {
            g.frame_pending = true;
            effects.push(Effect::RequestFrame { generation });
        }
        self.model.gesture = self.gesture.as_ref().map(gesture_view);
        if end && !changed {
            self.invalidate_gesture(effects);
        } else if end {
            self.apply_gesture(true, effects);
        }
    }
    pub(super) fn gesture_frame(&mut self, generation: u64, effects: &mut Vec<Effect>) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if g.generation != generation {
            return;
        }
        g.frame_pending = false;
        self.apply_gesture(false, effects);
    }
    fn apply_gesture(&mut self, is_commit: bool, effects: &mut Vec<Effect>) {
        let Some(g) = &mut self.gesture else {
            return;
        };
        if !is_commit && !g.changed && g.sent_positions.is_none() {
            return;
        }
        let positions = g.latest.clone();
        if !is_commit && g.sent_positions.as_ref() == Some(&positions) {
            return;
        }
        let operation_id = g.operation_id;
        let command = EditCommand {
            base_revision: g.base_revision,
            transaction_id: g.transaction_id.clone(),
            phase: if is_commit {
                EditPhase::Commit
            } else {
                EditPhase::Preview
            },
            target_ids: g.targets.clone(),
            operation: EditOperation::MoveParts {
                positions: positions.clone(),
            },
        };
        if !is_commit {
            g.sent_positions = Some(positions);
        }
        self.apply_edit(operation_id, command, true, effects);
    }
    pub(super) fn invalidate_gesture(&mut self, effects: &mut Vec<Effect>) {
        if let Some(operation_id) = self.clear_gesture(effects) {
            self.settle(operation_id, TerminalOutcome::Cancelled, effects);
        }
    }

    pub(super) fn clear_gesture(&mut self, effects: &mut Vec<Effect>) -> Option<OperationId> {
        let gesture = self.gesture.take();
        gesture.map(|g| {
            if g.frame_pending {
                effects.push(Effect::CancelFrame {
                    generation: g.generation,
                });
            }
            effects.push(Effect::ReleasePointer {
                pointer_id: g.pointer_id,
            });
            self.model.gesture = None;
            if self.display_preview_transaction_id.as_deref() == Some(&g.transaction_id) {
                self.model.display_preview = None;
                self.display_preview_transaction_id = None;
            }
            self.model.snap_guide = None;
            g.operation_id
        })
    }
}

fn gesture_view(g: &Gesture) -> GestureView {
    GestureView {
        pointer_id: g.pointer_id,
        generation: g.generation,
        target_ids: g.targets.clone(),
        changed: g.changed,
        snap: g.snap_fraction != 0.0,
        alt: g.alt,
    }
}
