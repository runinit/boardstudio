//! Layout canvas admission, modifier state and pointer transitions.

use crate::outline::{OutlineActionEnvelope, OutlinePointTarget, PerimeterAnchor};
use crate::outline_snapping;
use crate::rail_gesture::RailGesture;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{EditOperation, Position, Vec2};
use boardstudio_web_ui_model::transform_feedback::FeedbackDelta;

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

    pub fn is_placing(self) -> bool {
        matches!(
            self,
            Self::PlacingPart | Self::PlacingMatrix | Self::PlacingMirroredPair
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailBodyDrag {
    pub part_id: String,
    pub start_at: Vec2,
    pub last_sample: Option<(Vec2, f64, bool)>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RailDragKind {
    Gesture(RailGesture),
    Body(RailBodyDrag),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RailDragSeed {
    pub transaction_id: String,
    pub tool_label: &'static str,
    pub kind: RailDragKind,
    pub token: SnapshotToken,
    pub revision: u64,
    pub board_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutlineHandleSeed {
    pub point_index: usize,
    pub points: Vec<Vec2>,
    pub preview_points: Vec<Vec2>,
    pub original_world: Vec2,
    pub target: OutlinePointTarget,
    pub anchor: Option<PerimeterAnchor>,
    pub envelope: OutlineActionEnvelope,
    pub transaction_id: String,
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
    MatrixRail(RailDragSeed),
    PartRail(RailDragSeed),
    OutlineHandle(OutlineHandleSeed),
}

impl GestureSeed {
    pub fn mode(&self) -> CanvasMode {
        match self {
            Self::Pan { .. } => CanvasMode::Panning,
            Self::PartDrag { .. } => CanvasMode::DraggingParts,
            Self::MatrixRail(_) => CanvasMode::DraggingMatrixRail,
            Self::PartRail(_) => CanvasMode::DraggingPartRail,
            Self::OutlineHandle(_) => CanvasMode::DraggingOutlineHandle,
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

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveRailDrag {
    pub pointer_id: i32,
    pub transaction_id: String,
    pub tool_label: &'static str,
    pub kind: RailDragKind,
    pub token: SnapshotToken,
    pub revision: u64,
    pub board_id: String,
    pub start_client: (f64, f64),
    pub last_preview: Option<(String, FeedbackDelta)>,
    pub quick_offset: bool,
    pub preview: Option<EditOperation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveOutlineDrag {
    pub pointer_id: i32,
    pub start_client: (f64, f64),
    pub point_index: usize,
    pub points: Vec<Vec2>,
    pub pending: Vec<Vec2>,
    pub preview_points: Vec<Vec2>,
    pub original_world: Vec2,
    pub target: OutlinePointTarget,
    pub anchor: Option<PerimeterAnchor>,
    pub envelope: OutlineActionEnvelope,
    pub transaction_id: String,
    pub snap: Option<outline_snapping::Snap>,
    pub preview_submitted: bool,
    pub moved: bool,
    pub preview: Option<EditOperation>,
}

impl ActiveOutlineDrag {
    pub fn apply_sample(
        &mut self,
        world: Vec2,
        paths: &[Vec<Vec2>],
        origins: &[outline_snapping::Origin],
        options: outline_snapping::Options,
    ) {
        let count = self.points.len();
        let index = self.point_index;
        if count == 0 || index >= count || self.preview_points.len() != count {
            return;
        }
        let context = outline_snapping::Context {
            anchor: Some(self.preview_points[(index + count - 1) % count]),
            previous: Some(self.original_world),
            exclude: Some(self.original_world),
            neighbor: Some(self.preview_points[(index + 1) % count]),
        };
        let mut result = outline_snapping::snap_outline_point(
            world,
            context,
            paths,
            options,
            self.snap.as_ref(),
        );
        if !options.free
            && options.enabled
            && result.guides.is_empty()
            && let Some((at, _)) = outline_snapping::snap_origin(world, origins, options.tolerance)
        {
            result.at = at;
        }
        let mut preview_points = self.preview_points.clone();
        preview_points[index] = result.at;
        let mut pending = self.points.clone();
        pending[index] = self
            .anchor
            .map_or(result.at, |anchor| anchor.local(result.at));
        self.moved = pending != self.points;
        self.pending = pending;
        self.preview_points = preview_points;
        self.snap = Some(result);
    }
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Panning(ActivePan),
    DraggingParts(ActivePartDrag),
    DraggingMatrixRail(ActiveRailDrag),
    DraggingPartRail(ActiveRailDrag),
    DraggingOutlineHandle(ActiveOutlineDrag),
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
            State::DraggingMatrixRail(_) => Some(CanvasMode::DraggingMatrixRail),
            State::DraggingPartRail(_) => Some(CanvasMode::DraggingPartRail),
            State::DraggingOutlineHandle(_) => Some(CanvasMode::DraggingOutlineHandle),
            State::Armed { mode, .. } => Some(*mode),
        }
    }

    fn pointer(&self) -> Option<i32> {
        match self {
            State::Idle => None,
            State::Panning(pan) => Some(pan.pointer_id),
            State::DraggingParts(drag) => Some(drag.pointer_id),
            State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) => {
                Some(drag.pointer_id)
            }
            State::DraggingOutlineHandle(drag) => Some(drag.pointer_id),
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
            &self.state,
            State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) if drag.quick_offset
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

    pub fn rail_drag(&self) -> Option<ActiveRailDrag> {
        match &self.state {
            State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) => Some(drag.clone()),
            _ => None,
        }
    }

    pub fn outline_drag(&self) -> Option<ActiveOutlineDrag> {
        match &self.state {
            State::DraggingOutlineHandle(drag) => Some(drag.clone()),
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

    pub fn note_rail_preview(&mut self, preview: Option<(String, FeedbackDelta)>) {
        if let State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) = &mut self.state {
            drag.last_preview = preview;
        }
    }

    pub fn note_rail_body_sample(&mut self, sample: Option<(Vec2, f64, bool)>) {
        if let State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) = &mut self.state
            && let RailDragKind::Body(body) = &mut drag.kind
        {
            body.last_sample = sample;
        }
    }

    pub fn sample_outline_drag(
        &mut self,
        world: Vec2,
        paths: &[Vec<Vec2>],
        origins: &[outline_snapping::Origin],
        options: outline_snapping::Options,
    ) {
        if let State::DraggingOutlineHandle(drag) = &mut self.state {
            drag.apply_sample(world, paths, origins, options);
        }
    }

    pub fn note_outline_preview_submitted(&mut self) {
        if let State::DraggingOutlineHandle(drag) = &mut self.state {
            drag.preview_submitted = true;
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
                        matches!(seed, GestureSeed::MatrixRail(_) | GestureSeed::PartRail(_))
                            && (control || self.control_down);
                    let rail_active = |seed: RailDragSeed| ActiveRailDrag {
                        pointer_id: id,
                        transaction_id: seed.transaction_id,
                        tool_label: seed.tool_label,
                        kind: seed.kind,
                        token: seed.token,
                        revision: seed.revision,
                        board_id: seed.board_id,
                        start_client: client,
                        last_preview: None,
                        quick_offset,
                        preview: None,
                    };
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
                        GestureSeed::MatrixRail(seed) => {
                            State::DraggingMatrixRail(rail_active(seed))
                        }
                        GestureSeed::PartRail(seed) => State::DraggingPartRail(rail_active(seed)),
                        GestureSeed::OutlineHandle(seed) => {
                            State::DraggingOutlineHandle(ActiveOutlineDrag {
                                pointer_id: id,
                                start_client: client,
                                point_index: seed.point_index,
                                pending: seed.points.clone(),
                                points: seed.points,
                                preview_points: seed.preview_points,
                                original_world: seed.original_world,
                                target: seed.target,
                                anchor: seed.anchor,
                                envelope: seed.envelope,
                                transaction_id: seed.transaction_id,
                                snap: None,
                                preview_submitted: false,
                                moved: false,
                                preview: None,
                            })
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
                State::DraggingMatrixRail(ActiveRailDrag {
                    pointer_id: owner,
                    preview,
                    ..
                })
                | State::DraggingPartRail(ActiveRailDrag {
                    pointer_id: owner,
                    preview,
                    ..
                })
                | State::DraggingOutlineHandle(ActiveOutlineDrag {
                    pointer_id: owner,
                    preview,
                    ..
                }) if *owner == id => {
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
                    State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) => {
                        release_commit(id, operation, drag.preview)
                    }
                    State::DraggingOutlineHandle(drag) => {
                        release_commit(id, operation, drag.preview)
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
            State::DraggingMatrixRail(drag) | State::DraggingPartRail(drag) => (
                drag.pointer_id,
                drag.preview.is_some() || drag.last_preview.is_some(),
            ),
            State::DraggingOutlineHandle(drag) => (
                drag.pointer_id,
                drag.preview.is_some() || drag.preview_submitted,
            ),
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

    macro_rules! is {
        ($c:expr, $e:expr) => { assert!($c.handle($e).is_empty()) };
        ($c:expr, $e:expr, $($w:expr),+ $(,)?) => { assert_eq!($c.handle($e), [$($w),+]) };
    }
    fn kd(k: CanvasKey, l: bool, c: bool, t: bool, p: bool) -> CanvasEvent {
        CanvasEvent::KeyDown {
            key: k,
            layout: l,
            canvas: c,
            typing: t,
            prevented: p,
        }
    }
    fn key(k: CanvasKey) -> CanvasEvent {
        kd(k, true, true, false, false)
    }
    fn mv(id: i32, client: (f64, f64), o: Option<&str>) -> CanvasEvent {
        CanvasEvent::PointerMove {
            id,
            client,
            operation: o.map(op),
        }
    }
    fn up(id: i32, o: Option<&str>) -> CanvasEvent {
        CanvasEvent::PointerUp {
            id,
            operation: o.map(op),
        }
    }
    fn pt(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
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
        let rail = |body| {
            let kind = if body {
                RailDragKind::Body(RailBodyDrag {
                    part_id: "p1".into(),
                    start_at: pt(1.0, 2.0),
                    last_sample: None,
                })
            } else {
                RailDragKind::Gesture(RailGesture::PartRotate {
                    part_id: "m".into(),
                    center: Vec2::default(),
                    start_angle: 0.0,
                    start_rotation: 0.0,
                })
            };
            RailDragSeed {
                transaction_id: "tx-rail".into(),
                tool_label: if body { "Move" } else { "Rotate" },
                kind,
                token: SnapshotToken(9),
                revision: 4,
                board_id: "b".into(),
            }
        };
        let pts = vec![pt(0.0, 0.0), pt(20.0, 0.0), pt(20.0, 20.0), pt(0.0, 20.0)];
        let envelope = OutlineActionEnvelope {
            scope: scope(),
            token: SnapshotToken(9),
            revision: 4,
            generation: 3,
            board_id: "b".into(),
        };
        Some(match mode {
            Panning => GestureSeed::Pan {
                scope: scope(),
                generation,
                start_camera: pt(7.0, 8.0),
            },
            DraggingParts => GestureSeed::PartDrag {
                scope: scope(),
                generation,
                origin_mm: pt(4.0, 5.0),
                positions: vec![Position {
                    id: "part".into(),
                    at: pt(4.0, 5.0),
                }],
            },
            DraggingMatrixRail => GestureSeed::MatrixRail(rail(false)),
            DraggingPartRail => GestureSeed::PartRail(rail(true)),
            DraggingOutlineHandle => GestureSeed::OutlineHandle(OutlineHandleSeed {
                point_index: 1,
                original_world: pts[1],
                preview_points: pts.clone(),
                points: pts,
                target: OutlinePointTarget::Generated { contour: 0 },
                anchor: None,
                transaction_id: "tx-outline".into(),
                envelope,
            }),
            _ => return None,
        })
    }
    fn dn(id: i32, control: bool, seed: Option<GestureSeed>) -> CanvasEvent {
        CanvasEvent::PointerDown {
            id,
            client: (100.0, 200.0),
            control,
            seed,
        }
    }
    fn down(mode: CanvasMode, id: i32) -> CanvasEvent {
        dn(id, false, seed_for(mode, 3))
    }
    fn start(c: &mut CanvasInteraction, mode: CanvasMode, id: i32) {
        if mode.is_pointer_gesture() {
            is!(c, down(mode, id), CapturePointer(id));
        } else {
            c.handle(CanvasEvent::Begin(mode));
            c.handle(down(mode, id));
        }
        assert_eq!((c.mode(), c.pointer()), (Some(mode), Some(id)));
    }
    fn op(id: &str) -> EditOperation {
        EditOperation::RemoveMatrix { id: id.into() }
    }

    #[test]
    fn every_mode_rejects_competing_starts_and_second_pointers() {
        for active in MODES {
            let mut c = CanvasInteraction::default();
            c.handle(key(CanvasKey::Space));
            start(&mut c, active, 1);
            for contender in MODES {
                c.handle(CanvasEvent::Begin(contender));
                c.handle(down(contender, 2));
                assert_eq!((c.mode(), c.pointer()), (Some(active), Some(1)));
            }
            for e in [
                mv(2, (101.0, 200.0), Some("other")),
                up(2, Some("other")),
                CanvasEvent::PointerCancel(2),
                CanvasEvent::LostPointerCapture(2),
            ] {
                is!(c, e);
                assert_eq!(c.pointer(), Some(1));
            }
            let drop = c.handle(up(1, None));
            if active.is_pointer_gesture() {
                assert!(
                    drop.contains(&ReleasePointer(1))
                        && drop.contains(&FinishPan) == (active == Panning)
                );
                assert_eq!(c.mode(), None);
            } else {
                assert_eq!((c.pointer(), c.mode()), (None, Some(active)));
                c.handle(CanvasEvent::End(active));
                assert_eq!(c.mode(), None);
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
                let mut c = CanvasInteraction::default();
                c.handle(key(CanvasKey::Space));
                c.handle(key(CanvasKey::Control));
                start(&mut c, mode, 1);
                if mode.is_pointer_gesture() {
                    c.handle(mv(1, (101.0, 200.0), Some("preview")));
                }
                let fx = c.handle(event);
                assert_eq!(
                    (
                        fx.contains(&Cancel(mode)),
                        fx.contains(&ClearPreview),
                        fx.contains(&ReleasePointer(1)),
                        fx.iter().any(|e| matches!(e, Commit { .. })),
                        c.mode(),
                        c.space_down,
                        c.control_down
                    ),
                    (
                        true,
                        mode.is_pointer_gesture() && mode != Panning,
                        mode.is_pointer_gesture(),
                        false,
                        None,
                        !reset_modifiers,
                        !reset_modifiers
                    )
                );
                start(&mut c, DraggingParts, 2);
            }
        }
    }

    #[test]
    fn uncaptured_armed_mode_cancel_emits_no_release_pointer() {
        for mode in MODES.into_iter().filter(|m| !m.is_pointer_gesture()) {
            for (event, escape) in [
                (key(CanvasKey::Escape), true),
                (CanvasEvent::Blur, false),
                (CanvasEvent::WorkspaceChanged, false),
                (CanvasEvent::ScopeChanged, false),
            ] {
                let mut c = CanvasInteraction::default();
                c.handle(CanvasEvent::Begin(mode));
                assert_eq!((c.mode(), c.pointer()), (Some(mode), None));
                is!(c, CanvasEvent::PointerCancel(1));
                is!(c, CanvasEvent::LostPointerCapture(1));
                // Competing seeded PointerDown must not capture an armed mode's pointer slot.
                is!(c, down(DraggingParts, 2));
                assert_eq!((c.mode(), c.pointer()), (Some(mode), None));
                let mut expected = vec![Cancel(mode)];
                if escape {
                    expected.push(PreventDefault);
                }
                assert_eq!(c.handle(event), expected);
                assert_eq!(c.mode(), None);
            }
        }
    }

    #[test]
    fn space_admission_and_control_latching() {
        let mut c = CanvasInteraction::default();
        for (a, t, p) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            is!(c, kd(CanvasKey::Space, a, a, t, p));
            is!(c, down(Panning, 1));
        }
        assert!(
            c.handle(kd(CanvasKey::Space, false, true, false, false))
                .contains(&PreventDefault)
        );
        assert!(c.space_down);
        is!(c, kd(CanvasKey::Control, false, true, false, false));
        start(&mut c, Panning, 9);
        assert!(
            c.handle(kd(CanvasKey::Escape, false, true, false, false))
                .contains(&Cancel(Panning))
        );
        is!(c, CanvasEvent::KeyUp(CanvasKey::Space), ModifiersChanged);
        assert!(!c.space_down);
        for from_keydown in [false, true] {
            if from_keydown {
                is!(c, key(CanvasKey::Control), ModifiersChanged);
                c.handle(down(DraggingMatrixRail, 1));
            } else {
                c.handle(dn(1, true, seed_for(DraggingPartRail, 3)));
            }
            assert!(c.quick_offset());
            if from_keydown {
                is!(c, CanvasEvent::KeyUp(CanvasKey::Control), ModifiersChanged);
                assert!(c.sync_control(true));
            }
            assert_eq!(c.sync_control(false), from_keydown);
            assert!(!c.control_down && c.quick_offset());
            c.handle(CanvasEvent::Blur);
            assert!(!c.quick_offset());
        }
    }

    #[test]
    fn atomic_gesture_seed_pointer_down_transitions_and_payloads() {
        let mut c = CanvasInteraction::default();
        let com = |id, needs_preview| Commit {
            operation: op(id),
            needs_preview,
        };
        let ds = |id, ctl, m, g| dn(id, ctl, seed_for(m, g));
        // Seedless PointerDown while Idle does nothing.
        is!(c, dn(1, false, None));
        assert_eq!(c.mode(), None);

        // Pan seed requires space_down and atomically populates ActivePan.
        is!(c, down(Panning, 1));
        assert_eq!(c.mode(), None);
        assert!(c.handle(key(CanvasKey::Space)).contains(&PreventDefault));
        is!(c, ds(1, false, Panning, 4), CapturePointer(1));
        let p = c.pan().expect("pan payload");
        assert!((p.pointer_id, p.generation, p.origin_px) == (1, 4, (100.0, 200.0)));
        assert!((p.start_camera, p.scope) == (pt(7.0, 8.0), scope()));
        is!(c, up(1, None), ReleasePointer(1), FinishPan);

        // PartDrag seed atomically populates ActivePartDrag and tracks generation/sample updates.
        is!(c, ds(2, false, DraggingParts, 5), CapturePointer(2));
        let d = c.part_drag().expect("drag payload");
        assert!((d.pointer_id, d.generation, d.active, d.origin_mm) == (2, 5, false, pt(4.0, 5.0)));
        c.set_part_drag_gesture_generation(Some(11));
        c.note_part_drag_sample(Some((pt(6.0, 7.0), 2.0, false)));
        let d = c.part_drag().unwrap();
        assert!(
            (d.gesture_generation, d.last_sample) == (Some(11), Some((pt(6.0, 7.0), 2.0, false)))
        );
        is!(c, up(2, None), ReleasePointer(2));

        // Rail and outline seeds populate payloads and support preview, commit, and cancel.
        is!(c, ds(3, true, DraggingMatrixRail, 3), CapturePointer(3));
        assert!((c.rail_drag().unwrap().tool_label, c.quick_offset()) == ("Rotate", true));
        c.note_rail_preview(Some(("15°".into(), FeedbackDelta::Deg(15.0))));
        assert!(c.handle(mv(3, (15.0, 20.0), Some("r-prev"))) == [Preview(op("r-prev"))]);
        is!(c, up(3, None), ReleasePointer(3), com("r-prev", false));
        assert_eq!(c.rail_drag(), None);

        is!(c, down(DraggingPartRail, 4), CapturePointer(4));
        c.note_rail_body_sample(Some((pt(3.0, 4.0), 1.0, false)));
        c.note_rail_preview(Some(("X 3".into(), FeedbackDelta::Mm2(2.0, 2.0))));
        assert!(matches!(
            c.rail_drag().unwrap().kind,
            RailDragKind::Body(RailBodyDrag {
                last_sample: Some(_),
                ..
            })
        ));
        is!(
            c,
            key(CanvasKey::Escape),
            Cancel(DraggingPartRail),
            ClearPreview,
            ReleasePointer(4),
            PreventDefault
        );

        is!(c, down(DraggingOutlineHandle, 5), CapturePointer(5));
        let opts = outline_snapping::Options {
            grid: pt(1.0, 1.0),
            tolerance: 1.0,
            enabled: false,
            free: true,
        };
        c.sample_outline_drag(pt(25.0, 3.0), &[], &[], opts);
        c.note_outline_preview_submitted();
        let o = c.outline_drag().unwrap();
        assert!(o.moved && o.preview_submitted && o.pending[1] == pt(25.0, 3.0));
        is!(
            c,
            mv(5, (55.0, 57.0), Some("o-prev")),
            Preview(op("o-prev"))
        );
        is!(
            c,
            up(5, Some("o-done")),
            ReleasePointer(5),
            com("o-done", true)
        );
        start(&mut c, DraggingOutlineHandle, 6);
        c.note_outline_preview_submitted();
        is!(
            c,
            CanvasEvent::LostPointerCapture(6),
            Cancel(DraggingOutlineHandle),
            ClearPreview,
            ReleasePointer(6)
        );
        assert_eq!(c.outline_drag(), None);
    }

    #[test]
    fn part_drag_requires_real_client_motion_and_activates_once() {
        let mut c = CanvasInteraction::default();
        start(&mut c, DraggingParts, 1);
        is!(c, mv(1, (100.0, 200.0), None));
        is!(c, mv(1, (101.0, 200.0), None), StartPartDrag);
        is!(c, mv(1, (102.0, 200.0), None));
    }

    #[test]
    fn drop_reuses_the_preview_unless_release_supplies_a_new_operation() {
        for r in [None, Some("preview"), Some("release")] {
            let mut c = CanvasInteraction::default();
            start(&mut c, DraggingMatrixRail, 1);
            is!(
                c,
                mv(1, (100.0, 200.0), Some("preview")),
                Preview(op("preview"))
            );
            is!(c, mv(1, (100.0, 200.0), Some("preview")));
            is!(
                c,
                up(1, r),
                ReleasePointer(1),
                Commit {
                    operation: op(r.unwrap_or("preview")),
                    needs_preview: r == Some("release")
                }
            );
        }
    }
}
