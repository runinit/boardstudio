//! Layout canvas admission, modifier state and pointer transitions.

use boardstudio_application::Scope;
use boardstudio_model::model::{EditOperation, Position, Vec2};

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

    pub fn is_pick(self) -> bool {
        matches!(
            self,
            Self::PickingSplayOrigin | Self::PickingMatrixAnchor | Self::PickingRailAnchor
        )
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PartPlacementPayload {
    pub part_id: String,
    pub definition_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MatrixPlacementPayload {
    pub matrix_id: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MirroredPairPayload {
    pub open_id: u64,
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
    pub active: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ActiveMarquee {
    pub pointer_id: i32,
    pub origin_px: (f64, f64),
    pub current_px: (f64, f64),
    pub additive: bool,
}

#[derive(Debug, Default)]
enum State {
    #[default]
    Idle,
    Panning {
        id: i32,
        scope: Scope,
        generation: u64,
        origin_px: (f64, f64),
        start_camera: Vec2,
    },
    Marquee {
        id: i32,
        origin_px: (f64, f64),
        current_px: (f64, f64),
        additive: bool,
    },
    DraggingParts {
        id: i32,
        scope: Scope,
        generation: u64,
        gesture_generation: Option<u64>,
        origin_px: (f64, f64),
        origin_mm: Vec2,
        positions: Vec<Position>,
        last_sample: Option<(Vec2, f64, bool)>,
        preview: Option<EditOperation>,
        active: bool,
    },
    DraggingMatrixRail {
        id: i32,
        #[allow(dead_code)]
        origin_px: (f64, f64),
        quick_offset: bool,
        preview: Option<EditOperation>,
    },
    DraggingPartRail {
        id: i32,
        #[allow(dead_code)]
        origin_px: (f64, f64),
        quick_offset: bool,
        preview: Option<EditOperation>,
    },
    DraggingOutlineHandle {
        id: i32,
        #[allow(dead_code)]
        origin_px: (f64, f64),
        preview: Option<EditOperation>,
    },
    PickingSplayOrigin {
        pointer: Option<i32>,
    },
    PickingMatrixAnchor {
        pointer: Option<i32>,
    },
    PickingRailAnchor {
        pointer: Option<i32>,
    },
    PlacingPart {
        pointer: Option<i32>,
        payload: PartPlacementPayload,
    },
    PlacingMatrix {
        pointer: Option<i32>,
        payload: MatrixPlacementPayload,
    },
    PlacingMirroredPair {
        pointer: Option<i32>,
        payload: MirroredPairPayload,
    },
}

impl State {
    fn mode(&self) -> Option<CanvasMode> {
        match self {
            State::Idle => None,
            State::Panning { .. } => Some(CanvasMode::Panning),
            State::Marquee { .. } => Some(CanvasMode::Marquee),
            State::DraggingParts { .. } => Some(CanvasMode::DraggingParts),
            State::DraggingMatrixRail { .. } => Some(CanvasMode::DraggingMatrixRail),
            State::DraggingPartRail { .. } => Some(CanvasMode::DraggingPartRail),
            State::DraggingOutlineHandle { .. } => Some(CanvasMode::DraggingOutlineHandle),
            State::PickingSplayOrigin { .. } => Some(CanvasMode::PickingSplayOrigin),
            State::PickingMatrixAnchor { .. } => Some(CanvasMode::PickingMatrixAnchor),
            State::PickingRailAnchor { .. } => Some(CanvasMode::PickingRailAnchor),
            State::PlacingPart { .. } => Some(CanvasMode::PlacingPart),
            State::PlacingMatrix { .. } => Some(CanvasMode::PlacingMatrix),
            State::PlacingMirroredPair { .. } => Some(CanvasMode::PlacingMirroredPair),
        }
    }

    fn pointer(&self) -> Option<i32> {
        match self {
            State::Idle => None,
            State::Panning { id, .. }
            | State::Marquee { id, .. }
            | State::DraggingParts { id, .. }
            | State::DraggingMatrixRail { id, .. }
            | State::DraggingPartRail { id, .. }
            | State::DraggingOutlineHandle { id, .. } => Some(*id),
            State::PickingSplayOrigin { pointer }
            | State::PickingMatrixAnchor { pointer }
            | State::PickingRailAnchor { pointer }
            | State::PlacingPart { pointer, .. }
            | State::PlacingMatrix { pointer, .. }
            | State::PlacingMirroredPair { pointer, .. } => *pointer,
        }
    }
}

#[derive(Default)]
pub struct CanvasInteraction {
    state: State,
    pending_pan: Option<PendingPan>,
    pending_part_drag: Option<PendingPartDrag>,
    pending_marquee_additive: Option<bool>,
    pub space_down: bool,
    pub control_down: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct PendingPan {
    scope: Scope,
    generation: u64,
    start_camera: Vec2,
}

#[derive(Clone, Debug, PartialEq)]
struct PendingPartDrag {
    scope: Scope,
    generation: u64,
    origin_mm: Vec2,
    positions: Vec<Position>,
}

pub enum CanvasEvent {
    Begin(CanvasMode),
    End(CanvasMode),
    BeginPartPlacement(PartPlacementPayload),
    BeginMatrixPlacement(MatrixPlacementPayload),
    BeginMirroredPairPlacement(MirroredPairPayload),
    SeedPan {
        scope: Scope,
        generation: u64,
        start_camera: Vec2,
    },
    SeedPartDrag {
        scope: Scope,
        generation: u64,
        origin_mm: Vec2,
        positions: Vec<Position>,
    },
    SeedMarquee {
        additive: bool,
    },
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
            State::Panning {
                id,
                scope,
                generation,
                origin_px,
                start_camera,
            } => Some(ActivePan {
                pointer_id: *id,
                scope: scope.clone(),
                generation: *generation,
                origin_px: *origin_px,
                start_camera: *start_camera,
            }),
            _ => None,
        }
    }

    pub fn part_drag(&self) -> Option<ActivePartDrag> {
        match &self.state {
            State::DraggingParts {
                id,
                scope,
                generation,
                gesture_generation,
                origin_px,
                origin_mm,
                positions,
                last_sample,
                preview: _,
                active,
            } => Some(ActivePartDrag {
                pointer_id: *id,
                scope: scope.clone(),
                generation: *generation,
                gesture_generation: *gesture_generation,
                origin_px: *origin_px,
                origin_mm: *origin_mm,
                positions: positions.clone(),
                last_sample: *last_sample,
                active: *active,
            }),
            _ => None,
        }
    }

    pub fn marquee(&self) -> Option<ActiveMarquee> {
        match &self.state {
            State::Marquee {
                id,
                origin_px,
                current_px,
                additive,
            } => Some(ActiveMarquee {
                pointer_id: *id,
                origin_px: *origin_px,
                current_px: *current_px,
                additive: *additive,
            }),
            _ => None,
        }
    }

    pub fn placing_part(&self) -> Option<PartPlacementPayload> {
        match &self.state {
            State::PlacingPart { payload, .. } => Some(payload.clone()),
            _ => None,
        }
    }

    pub fn placing_matrix(&self) -> Option<MatrixPlacementPayload> {
        match &self.state {
            State::PlacingMatrix { payload, .. } => Some(payload.clone()),
            _ => None,
        }
    }

    pub fn placing_pair(&self) -> Option<MirroredPairPayload> {
        match &self.state {
            State::PlacingMirroredPair { payload, .. } => Some(payload.clone()),
            _ => None,
        }
    }

    pub fn set_part_drag_gesture_generation(&mut self, generation: Option<u64>) {
        if let State::DraggingParts {
            gesture_generation, ..
        } = &mut self.state
        {
            *gesture_generation = generation;
        }
    }

    pub fn note_part_drag_sample(&mut self, sample: Option<(Vec2, f64, bool)>) {
        if let State::DraggingParts { last_sample, .. } = &mut self.state {
            *last_sample = sample;
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
            CanvasEvent::SeedPan {
                scope,
                generation,
                start_camera,
            } => {
                self.pending_pan = Some(PendingPan {
                    scope,
                    generation,
                    start_camera,
                });
                Vec::new()
            }
            CanvasEvent::SeedPartDrag {
                scope,
                generation,
                origin_mm,
                positions,
            } => {
                self.pending_part_drag = Some(PendingPartDrag {
                    scope,
                    generation,
                    origin_mm,
                    positions,
                });
                Vec::new()
            }
            CanvasEvent::SeedMarquee { additive } => {
                self.pending_marquee_additive = Some(additive);
                Vec::new()
            }
            CanvasEvent::Begin(mode) if self.mode().is_none() && mode.is_pick() => {
                self.state = match mode {
                    CanvasMode::PickingSplayOrigin => State::PickingSplayOrigin { pointer: None },
                    CanvasMode::PickingMatrixAnchor => State::PickingMatrixAnchor { pointer: None },
                    _ => State::PickingRailAnchor { pointer: None },
                };
                Vec::new()
            }
            CanvasEvent::BeginPartPlacement(payload) if self.mode().is_none() => {
                self.state = State::PlacingPart {
                    pointer: None,
                    payload,
                };
                Vec::new()
            }
            CanvasEvent::BeginMatrixPlacement(payload) if self.mode().is_none() => {
                self.state = State::PlacingMatrix {
                    pointer: None,
                    payload,
                };
                Vec::new()
            }
            CanvasEvent::BeginMirroredPairPlacement(payload) if self.mode().is_none() => {
                self.state = State::PlacingMirroredPair {
                    pointer: None,
                    payload,
                };
                Vec::new()
            }
            CanvasEvent::End(mode) if self.mode() == Some(mode) => {
                self.clear_state();
                Vec::new()
            }
            CanvasEvent::PointerDown {
                mode,
                id,
                client,
                control,
            } => {
                let (pan_seed, part_seed, marquee_seed) = (
                    self.pending_pan.take(),
                    self.pending_part_drag.take(),
                    self.pending_marquee_additive.take(),
                );
                if !matches!(self.state, State::Idle) {
                    return self.capture_for_armed_mode(mode, id);
                }
                if !mode.is_pointer_gesture() {
                    return Vec::new();
                }
                match mode {
                    CanvasMode::Panning if !self.space_down => Vec::new(),
                    CanvasMode::Panning => {
                        let Some(seed) = pan_seed else {
                            return Vec::new();
                        };
                        self.state = State::Panning {
                            id,
                            scope: seed.scope,
                            generation: seed.generation,
                            origin_px: client,
                            start_camera: seed.start_camera,
                        };
                        vec![CapturePointer(id)]
                    }
                    CanvasMode::DraggingParts => {
                        let Some(seed) = part_seed else {
                            return Vec::new();
                        };
                        self.state = State::DraggingParts {
                            id,
                            scope: seed.scope,
                            generation: seed.generation,
                            gesture_generation: None,
                            origin_px: client,
                            origin_mm: seed.origin_mm,
                            positions: seed.positions,
                            last_sample: None,
                            preview: None,
                            active: false,
                        };
                        vec![CapturePointer(id)]
                    }
                    CanvasMode::Marquee => {
                        let additive = marquee_seed.unwrap_or(false);
                        self.state = State::Marquee {
                            id,
                            origin_px: client,
                            current_px: client,
                            additive,
                        };
                        vec![CapturePointer(id)]
                    }
                    CanvasMode::DraggingMatrixRail | CanvasMode::DraggingPartRail => {
                        let quick_offset = mode != CanvasMode::DraggingOutlineHandle
                            && (control || self.control_down);
                        self.state = if mode == CanvasMode::DraggingMatrixRail {
                            State::DraggingMatrixRail {
                                id,
                                origin_px: client,
                                quick_offset,
                                preview: None,
                            }
                        } else {
                            State::DraggingPartRail {
                                id,
                                origin_px: client,
                                quick_offset,
                                preview: None,
                            }
                        };
                        vec![CapturePointer(id)]
                    }
                    CanvasMode::DraggingOutlineHandle => {
                        self.state = State::DraggingOutlineHandle {
                            id,
                            origin_px: client,
                            preview: None,
                        };
                        vec![CapturePointer(id)]
                    }
                    _ => Vec::new(),
                }
            }
            CanvasEvent::PointerMove {
                id,
                client: point,
                operation,
            } => {
                if self.pointer().is_some_and(|owner| id != owner) {
                    return Vec::new();
                }
                match &mut self.state {
                    State::Marquee {
                        current_px,
                        origin_px,
                        ..
                    } => {
                        if point == *current_px {
                            return Vec::new();
                        }
                        *current_px = point;
                        return vec![PreviewMarquee(MarqueeBounds {
                            start: *origin_px,
                            end: point,
                        })];
                    }
                    State::DraggingParts {
                        id: owner,
                        origin_px,
                        active,
                        preview,
                        ..
                    } => {
                        if id != *owner {
                            return Vec::new();
                        }
                        let mut effects = Vec::new();
                        if !*active && point != *origin_px {
                            *active = true;
                            effects.push(StartPartDrag);
                        }
                        if let Some(operation) = operation
                            && preview.as_ref() != Some(&operation)
                        {
                            *preview = Some(operation.clone());
                            effects.push(Preview(operation));
                        }
                        return effects;
                    }
                    State::DraggingMatrixRail {
                        id: owner, preview, ..
                    }
                    | State::DraggingPartRail {
                        id: owner, preview, ..
                    }
                    | State::DraggingOutlineHandle {
                        id: owner, preview, ..
                    } => {
                        if id != *owner {
                            return Vec::new();
                        }
                        if let Some(operation) = operation
                            && preview.as_ref() != Some(&operation)
                        {
                            *preview = Some(operation.clone());
                            return vec![Preview(operation)];
                        }
                        return Vec::new();
                    }
                    _ => Vec::new(),
                }
            }
            CanvasEvent::PointerUp { id, operation } if self.pointer() == Some(id) => {
                match &mut self.state {
                    State::PickingSplayOrigin { pointer }
                    | State::PickingMatrixAnchor { pointer }
                    | State::PickingRailAnchor { pointer }
                    | State::PlacingPart { pointer, .. }
                    | State::PlacingMatrix { pointer, .. }
                    | State::PlacingMirroredPair { pointer, .. } => {
                        *pointer = None;
                        return Vec::new();
                    }
                    _ => {}
                }
                match std::mem::replace(&mut self.state, State::Idle) {
                    State::Panning { .. } => vec![ReleasePointer(id), FinishPan],
                    State::Marquee {
                        origin_px,
                        current_px,
                        ..
                    } => {
                        let mut effects = vec![ReleasePointer(id)];
                        if origin_px != current_px {
                            effects.push(SelectMarquee(MarqueeBounds {
                                start: origin_px,
                                end: current_px,
                            }));
                        }
                        effects
                    }
                    State::DraggingParts { preview, .. }
                    | State::DraggingMatrixRail { preview, .. }
                    | State::DraggingPartRail { preview, .. }
                    | State::DraggingOutlineHandle { preview, .. } => {
                        release_commit(id, operation, preview)
                    }
                    _ => Vec::new(),
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

    fn capture_for_armed_mode(&mut self, mode: CanvasMode, id: i32) -> Vec<InteractionEffect> {
        if self.state.mode() != Some(mode) {
            return Vec::new();
        }
        let slot = match &mut self.state {
            State::PickingSplayOrigin { pointer }
            | State::PickingMatrixAnchor { pointer }
            | State::PickingRailAnchor { pointer }
            | State::PlacingPart { pointer, .. }
            | State::PlacingMatrix { pointer, .. }
            | State::PlacingMirroredPair { pointer, .. } => pointer,
            _ => return Vec::new(),
        };
        if slot.is_none() {
            *slot = Some(id);
        }
        Vec::new()
    }

    fn clear_state(&mut self) {
        self.state = State::Idle;
        self.pending_pan.take();
        self.pending_part_drag.take();
        self.pending_marquee_additive.take();
    }

    fn cancel(&mut self) -> Vec<InteractionEffect> {
        use InteractionEffect::*;
        let (mode, pointer) = (self.state.mode(), self.state.pointer());
        let had_preview = matches!(
            self.state,
            State::DraggingParts {
                preview: Some(_),
                ..
            } | State::DraggingMatrixRail {
                preview: Some(_),
                ..
            } | State::DraggingPartRail {
                preview: Some(_),
                ..
            } | State::DraggingOutlineHandle {
                preview: Some(_),
                ..
            }
        );
        self.clear_state();
        let mut effects = Vec::new();
        if let Some(mode) = mode {
            effects.push(Cancel(mode));
        }
        if had_preview {
            effects.push(ClearPreview);
        }
        if let Some(id) = pointer {
            effects.push(ReleasePointer(id));
        }
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
            canvas: true,
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
    fn mv(id: i32, x: f64, op: Option<EditOperation>) -> CanvasEvent {
        CanvasEvent::PointerMove {
            id,
            client: (x, 200.0),
            operation: op,
        }
    }
    fn up(id: i32, op: Option<EditOperation>) -> CanvasEvent {
        CanvasEvent::PointerUp { id, operation: op }
    }
    fn scope() -> Scope {
        Scope {
            session_epoch: boardstudio_application::SessionEpoch(7),
            document_id: "p".into(),
            board_id: "b".into(),
            instance_id: None,
        }
    }
    fn op(id: &str) -> EditOperation {
        EditOperation::RemoveMatrix { id: id.into() }
    }
    fn spacedown(t: bool, y: bool, p: bool) -> CanvasEvent {
        CanvasEvent::KeyDown {
            key: CanvasKey::Space,
            layout: t,
            canvas: t,
            typing: y,
            prevented: p,
        }
    }
    fn pan_seed(generation: u64) -> CanvasEvent {
        CanvasEvent::SeedPan {
            scope: scope(),
            generation,
            start_camera: Vec2 { x: 7.0, y: 8.0 },
        }
    }
    fn drag_seed(generation: u64) -> CanvasEvent {
        CanvasEvent::SeedPartDrag {
            scope: scope(),
            generation,
            origin_mm: Vec2 { x: 4.0, y: 5.0 },
            positions: vec![Position {
                id: "part".into(),
                at: Vec2 { x: 4.0, y: 5.0 },
            }],
        }
    }
    fn part_payload() -> PartPlacementPayload {
        PartPlacementPayload {
            part_id: "p".into(),
            definition_id: "d".into(),
        }
    }
    fn ctrldown() -> CanvasEvent {
        CanvasEvent::PointerDown {
            mode: DraggingPartRail,
            id: 1,
            client: (0.0, 0.0),
            control: true,
        }
    }
    fn start(c: &mut CanvasInteraction, mode: CanvasMode, id: i32) {
        if mode.is_pointer_gesture() {
            match mode {
                Panning => c.handle(pan_seed(3)),
                DraggingParts => c.handle(drag_seed(3)),
                Marquee => c.handle(CanvasEvent::SeedMarquee { additive: false }),
                _ => Vec::new(),
            };
            if mode == Panning {
                c.handle(key(CanvasKey::Space));
            }
            assert_eq!(c.handle(down(mode, id)), [CapturePointer(id)]);
        } else if mode.is_pick() {
            c.handle(CanvasEvent::Begin(mode));
            c.handle(down(mode, id));
        } else if mode == PlacingPart {
            c.handle(CanvasEvent::BeginPartPlacement(part_payload()));
            c.handle(down(mode, id));
        } else if mode == PlacingMatrix {
            c.handle(CanvasEvent::BeginMatrixPlacement(MatrixPlacementPayload {
                matrix_id: "m".into(),
            }));
            c.handle(down(mode, id));
        } else {
            c.handle(CanvasEvent::BeginMirroredPairPlacement(
                MirroredPairPayload { open_id: 9 },
            ));
            c.handle(down(mode, id));
        }
        assert_eq!((c.mode(), c.pointer()), (Some(mode), Some(id)));
    }
    #[test]
    fn gestures_admit_reject_cancel_and_restart() {
        for active in MODES {
            let mut c = CanvasInteraction::default();
            c.handle(key(CanvasKey::Space));
            start(&mut c, active, 1);
            for contender in MODES {
                c.handle(CanvasEvent::Begin(contender));
                c.handle(down(contender, 2));
                assert_eq!((c.mode(), c.pointer()), (Some(active), Some(1)));
            }
            for event in [
                mv(2, 101.0, Some(op("other"))),
                up(2, Some(op("other"))),
                CanvasEvent::LostPointerCapture(2),
            ] {
                assert!(c.handle(event).is_empty() && c.pointer() == Some(1));
            }
            let drop = c.handle(up(1, None));
            if active.is_pointer_gesture() {
                assert!(drop.contains(&ReleasePointer(1)));
                assert_eq!(drop.contains(&FinishPan), active == Panning);
                assert_eq!(c.mode(), None);
            } else {
                assert_eq!((c.pointer(), c.mode()), (None, Some(active)));
                if active == PlacingPart {
                    assert_eq!(c.placing_part(), Some(part_payload()));
                }
                if active == PlacingMirroredPair {
                    assert_eq!(c.placing_pair(), Some(MirroredPairPayload { open_id: 9 }));
                }
                c.handle(CanvasEvent::End(active));
            }
        }
        for mode in MODES {
            for (event, reset, escape) in [
                (key(CanvasKey::Escape), false, true),
                (CanvasEvent::Blur, true, false),
                (CanvasEvent::WorkspaceChanged, true, false),
                (CanvasEvent::ScopeChanged, false, false),
                (CanvasEvent::LostPointerCapture(1), false, false),
            ] {
                let mut c = CanvasInteraction::default();
                c.handle(key(CanvasKey::Space));
                c.handle(key(CanvasKey::Control));
                start(&mut c, mode, 1);
                if mode.is_pointer_gesture() {
                    c.handle(mv(1, 101.0, Some(op("preview"))));
                }
                let effects = c.handle(event);
                let mut want = vec![Cancel(mode)];
                if matches!(
                    mode,
                    DraggingParts | DraggingMatrixRail | DraggingPartRail | DraggingOutlineHandle
                ) {
                    want.push(ClearPreview);
                }
                want.push(ReleasePointer(1));
                if escape {
                    want.push(PreventDefault);
                }
                if reset {
                    want.push(ModifiersChanged);
                }
                assert_eq!(effects, want);
                assert_eq!(
                    (c.mode(), c.space_down, c.control_down),
                    (None, !reset, !reset)
                );
            }
        }
    }
    #[test]
    fn modifiers_seeds_payloads_and_commits() {
        let mut c = CanvasInteraction::default();
        for (t, y, p) in [
            (false, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            assert!(
                c.handle(spacedown(t, y, p)).is_empty()
                    && c.handle(down(Panning, 1)).is_empty()
                    && c.handle(down(DraggingParts, 1)).is_empty()
            );
        }
        assert!(c.handle(key(CanvasKey::Space)).contains(&PreventDefault));
        c.handle(pan_seed(4));
        assert_eq!(c.handle(down(Panning, 1)), [CapturePointer(1)]);
        let pan = c.pan().expect("pan payload");
        assert_eq!((pan.pointer_id, pan.scope, pan.generation), (1, scope(), 4));
        assert_eq!(pan.origin_px, (100.0, 200.0));
        assert_eq!(pan.start_camera, Vec2 { x: 7.0, y: 8.0 });
        assert_eq!(c.handle(up(1, None)), [ReleasePointer(1), FinishPan]);
        c.handle(drag_seed(5));
        assert_eq!(c.handle(down(DraggingParts, 2)), [CapturePointer(2)]);
        let drag = c.part_drag().expect("drag payload");
        assert_eq!(drag.pointer_id, 2);
        assert_eq!(drag.generation, 5);
        assert_eq!(drag.active, false);
        assert_eq!(drag.origin_mm, Vec2 { x: 4.0, y: 5.0 });
        c.set_part_drag_gesture_generation(Some(11));
        c.note_part_drag_sample(Some((Vec2 { x: 6.0, y: 7.0 }, 2.0, false)));
        assert_eq!(c.part_drag().unwrap().gesture_generation, Some(11));
        assert!(c.handle(mv(2, 100.0, None)).is_empty());
        assert_eq!(c.handle(mv(2, 101.0, None)), [StartPartDrag]);
        assert!(c.handle(mv(2, 102.0, None)).is_empty());
        assert_eq!(c.handle(up(2, None)), [ReleasePointer(2)]);
        c.handle(CanvasEvent::SeedMarquee { additive: true });
        assert_eq!(c.handle(down(Marquee, 3)), [CapturePointer(3)]);
        assert!(c.marquee().is_some_and(|marquee| marquee.additive));
        let bounds = MarqueeBounds {
            start: (100.0, 200.0),
            end: (50.0, 200.0),
        };
        assert_eq!(c.handle(mv(3, 50.0, None)), [PreviewMarquee(bounds)]);
        assert_eq!(
            c.handle(up(3, None)),
            [ReleasePointer(3), SelectMarquee(bounds)]
        );
        for from_keydown in [false, true] {
            if from_keydown {
                assert_eq!(c.handle(key(CanvasKey::Control)), [ModifiersChanged]);
                c.handle(down(DraggingMatrixRail, 1));
            } else {
                c.handle(ctrldown());
            }
            assert!(c.quick_offset() && c.sync_control(false) == from_keydown);
            assert!(c.quick_offset() && !c.control_down);
            c.handle(CanvasEvent::Blur);
            assert!(!c.quick_offset());
        }
        for (release, preview) in [(None, false), (Some("release"), true)] {
            start(&mut c, DraggingMatrixRail, 1);
            assert_eq!(
                c.handle(mv(1, 100.0, Some(op("preview")))),
                [Preview(op("preview"))]
            );
            assert_eq!(
                c.handle(up(1, release.map(op))),
                vec![
                    ReleasePointer(1),
                    Commit {
                        operation: op(release.unwrap_or("preview")),
                        needs_preview: preview
                    }
                ]
            );
        }
    }
}
