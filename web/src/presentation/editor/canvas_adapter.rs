use super::super::*;
use crate::presentation::{
    layout_canvas::{MatrixPointerDown, PartPointerDown},
    layout_workspace::LayoutWorkspaceState,
    owner_checks::canvas_owner_is_current,
    pending_picks::{apply_pending_anchor_pick, apply_pending_splay_origin_pick},
    tree_cell_anchor::{OwnedTreeCellAnchor, tree_cell_anchor_for_owner, update_tree_cell_anchor},
    view_helpers::matrix_snap_parameters,
    workspace_composition::{CanvasEventHandlers, WorkspaceFrame, WorkspaceInteraction},
};
use CanvasMode as Mode;
use boardstudio_application::resolved_document;
use boardstudio_web_layout::canvas_interaction::{
    ActivePan, ActivePartDrag, begin_body_drag, release_capture, submit_gesture_step,
};
use boardstudio_web_layout::matrix_transform_operation::MatrixTransformFields as Fields;
use boardstudio_web_layout::part_placement::{
    ActivePartPlacement, PlacementSnapOptions, placement_gap, pointer_release_commits,
    snap_placement_at,
};
use boardstudio_web_ui_model::{Workspace, split_focus::context_half};
use boardstudio_web_ui_shared::canvas_navigation::ZoomTarget;
use objects::{ScopedTreeContext, TreeContext, context_for_cell, context_for_part};
use selection::{
    MatrixCellSelection, eligible_live_ids, layout_owner_is_current as owner_ok,
    owns_session_gesture, resolve_context, submit_canvas_selection as submit_sel,
    submit_matrix_cell_selection as submit_cell,
};

pub(in crate::presentation) type SvgRef = Rc<std::cell::RefCell<Option<SvgElement>>>;
pub(in crate::presentation) type AnchorRef = Rc<std::cell::RefCell<Option<OwnedTreeCellAnchor>>>;
pub(in crate::presentation) type CanvasBounds = ((f64, f64, f64, f64), f64, (f64, f64, f64, f64));

#[derive(Clone)]
pub(in crate::presentation) struct CanvasSurface {
    pub runtime: Rc<Runtime>,
    pub svg: SvgRef,
    pub adapter: SelectionAdapter,
    pub interaction: LayoutInteraction,
    pub scope: Scope,
    pub generation: u64,
    pub owner: LayoutOwnerIdentity,
    pub workspace: Signal<Workspace>,
    pub bounds: CanvasBounds,
    pub tree_cell_anchor: AnchorRef,
    pub layout_state: LayoutWorkspaceState,
    pub transform_inspector: objects::MatrixTransformInspectorMount,
    pub range_ids: Rc<Vec<String>>,
}

#[derive(Clone)]
pub(in crate::presentation) struct CanvasAdapter {
    pub surface: CanvasSurface,
    pub navigation: CanvasNavigationState,
    pub part_placement: part_placement::PartPlacementSession,
    pub matrix_placement: objects::MatrixPlacementSession,
    pub mirrored_pair: objects::MirroredPairSession,
    pub interaction_version: Signal<u64>,
    pub active_workspace: Workspace,
}

pub(in crate::presentation) struct LayoutPointer {
    pub matrix_pointer_down: EventHandler<MatrixPointerDown>,
    pub part_pointer_down: EventHandler<PartPointerDown>,
}

enum LiveGesture {
    Pan(ActivePan),
    Parts(ActivePartDrag),
}
enum CanvasHit {
    Matrix(MatrixPointerDown),
    Part(PartPointerDown),
}

pub(in crate::presentation) fn use_canvas_surface(
    ctx: CanvasAdapter,
    mount: EventHandler<MountedEvent>,
    keyboard: impl FnMut(KeyboardEvent) + Clone + 'static,
    key_up: impl FnMut(KeyboardEvent) + Clone + 'static,
) -> CanvasEventHandlers {
    let ws = ctx.active_workspace;
    let prev = use_hook(move || Rc::new(Cell::new(ws)));
    let (s, ver) = (&ctx.surface, ctx.interaction_version);
    let (sc, inter) = (s.scope.clone(), s.interaction);
    use_effect(use_reactive((&ws,), move |(c,)| {
        if prev.replace(c) == Workspace::Pcb && c != Workspace::Pcb {
            inter.cancel_scoped(&sc);
        }
    }));
    let wh = wheel_handler(
        s.runtime.clone(),
        s.svg.clone(),
        s.scope.clone(),
        s.adapter.clone(),
        s.generation,
        s.bounds,
    );
    let inp = use_context::<layout_input::LayoutInput>();
    let cancel_pointer = use_cancel_callback(inp, ver, false);
    let lost_capture = use_cancel_callback(inp, ver, true);
    let c1 = Rc::new(ctx);
    let (c2, c3) = (c1.clone(), c1.clone());
    CanvasEventHandlers {
        mount,
        start_pan: use_callback(move |e| c1.handle_start_pan(e)),
        move_pointer: use_callback(move |e| c2.handle_move(e)),
        end_pointer: use_callback(move |e| c3.handle_end(e)),
        cancel_pointer,
        lost_capture,
        keyboard: use_callback(keyboard),
        key_up: use_callback(key_up),
        wheel: use_callback(wh),
    }
}

fn use_cancel_callback(
    inp: layout_input::LayoutInput,
    mut ver: Signal<u64>,
    lost: bool,
) -> EventHandler<PointerEvent> {
    use_callback(move |e: PointerEvent| {
        if let Some(p) = e.data().try_as_web_event() {
            let id = p.pointer_id();
            let ev = if lost {
                CanvasEvent::LostPointerCapture(id)
            } else {
                CanvasEvent::PointerCancel(id)
            };
            if inp
                .dispatch(ev)
                .iter()
                .any(|x| matches!(x, InteractionEffect::Cancel(_)))
            {
                ver += 1;
            }
        }
    })
}

pub(in crate::presentation) fn build_canvas_pointer(f: &WorkspaceFrame) -> LayoutPointer {
    let ed = use_context::<boardstudio_web_ui_model::editor::EditorContext>();
    let svc = f.layout_services();
    let wi = use_context::<WorkspaceInteraction>();
    let s = Rc::new(CanvasSurface {
        runtime: ed.runtime(),
        svg: f.svg.clone(),
        adapter: use_context(),
        interaction: use_context(),
        scope: f.scope.clone(),
        generation: f.owner.generation,
        owner: f.owner.clone(),
        workspace: ed.workspace,
        bounds: f.camera.canvas_bounds(),
        tree_cell_anchor: wi.tree_cell_anchor,
        layout_state: use_context(),
        transform_inspector: svc.matrix_transform_inspector.clone(),
        range_ids: svc.visible_ids.clone(),
    });
    let m = s.clone();
    LayoutPointer {
        matrix_pointer_down: use_callback(move |r| m.on_down(CanvasHit::Matrix(r))),
        part_pointer_down: use_callback(move |r| s.on_down(CanvasHit::Part(r))),
    }
}

impl CanvasSurface {
    fn world_pt(&self, p: &web_sys::PointerEvent) -> Option<Vec2> {
        let (x, y, w, h) = self.bounds.0;
        coordinates(&self.svg, p, x, y, w, h)
    }
    fn world_scaled(&self, p: &web_sys::PointerEvent) -> Option<(Vec2, f64)> {
        let (x, y, w, h) = self.bounds.0;
        coordinates_with_scale(&self.svg, p, x, y, w, h)
    }
    fn scope_ok(&self) -> bool {
        self.runtime.scope().as_ref() == Some(&self.scope)
            && (self.adapter.generation)() == self.generation
    }
    fn preempt(&self, ptr: &web_sys::PointerEvent, up: bool) -> bool {
        self.interaction.sync_control(ptr.ctrl_key());
        let id = ptr.pointer_id();
        self.interaction.route_pointer(ptr, up)
            || self.interaction.pointer().is_some_and(|p| p != id)
    }
    fn pan(&self, p: &web_sys::PointerEvent) -> bool {
        let start_camera = self.runtime.with_model(|m| m.camera.center);
        let (scope, generation) = (self.scope.clone(), self.generation);
        let seed = GestureSeed::Pan {
            scope,
            generation,
            start_camera,
        };
        capture_canvas_drag(
            self.interaction,
            p.pointer_id(),
            client_point(p),
            p.ctrl_key(),
            seed,
            &self.runtime,
            &self.svg,
        )
    }
    fn try_splay_pick(&self, ptr: &web_sys::PointerEvent, focus: bool) -> bool {
        let mut pend = self.layout_state.pending_splay_origin_pick();
        let (Some(_), Some(pt)) = (pend.peek().as_ref(), self.world_pt(ptr)) else {
            return false;
        };
        if focus {
            prevent_and_focus(ptr, &self.svg);
        }
        self.interaction.end(Mode::PickingSplayOrigin);
        let own = (&*self.runtime, self.workspace, &self.adapter, &self.owner);
        apply_pending_splay_origin_pick(
            own,
            &mut pend,
            &self.transform_inspector,
            (pt, ptr.ctrl_key()),
        )
    }
    fn try_anchor_pick(&self, ptr: &web_sys::PointerEvent, hit: &CanvasHit) -> bool {
        let cell = match hit {
            CanvasHit::Matrix(r) => (r.matrix_id.clone(), r.row, r.column),
            CanvasHit::Part(r) => {
                match self.runtime.with_model(|m| context_for_part(m, &r.part_id)) {
                    Some(TreeContext::Key {
                        matrix_id,
                        row,
                        column,
                    }) => (matrix_id, row, column),
                    _ => return true,
                }
            }
        };
        let mut pend = self.layout_state.pending_anchor_pick();
        if pend.peek().is_none() {
            return false;
        }
        prevent_and_focus(ptr, &self.svg);
        let ins = &self.transform_inspector;
        if ins.projection().is_some_and(|p| matches!(&p.fields, Fields::Matrix { anchor: Some(a), .. } if a.aggregate.is_some())) {
            if !ins.pick_aggregate_part(cell.0, cell.1, cell.2) {
                self.runtime.report("Matrix selection changed. Pick the anchor parts again.");
            }
            return true;
        }
        self.interaction.end(Mode::PickingMatrixAnchor);
        let own = (&*self.runtime, self.workspace, &self.adapter, &self.owner);
        apply_pending_anchor_pick(own, &mut pend, ins, cell)
    }
    fn live_gesture(&self, ptr: i32) -> Option<LiveGesture> {
        let live =
            canvas_owner_is_current(&self.runtime, self.workspace, &self.adapter, &self.owner)
                && self.scope_ok();
        let ok = |id: i32, s: &Scope, g: u64| id == ptr && s == &self.scope && g == self.generation;
        if let Some(p) = self
            .interaction
            .pan()
            .filter(|p| ok(p.pointer_id, &p.scope, p.generation))
        {
            if live {
                return Some(LiveGesture::Pan(p));
            }
        } else {
            let d = self
                .interaction
                .part_drag()
                .filter(|d| ok(d.pointer_id, &d.scope, d.generation))?;
            let owned = owns_session_gesture(
                &self.runtime,
                i64::from(d.pointer_id),
                &d.scope,
                d.gesture_generation,
            );
            if live && (!d.active || owned) {
                return Some(LiveGesture::Parts(d));
            }
        }
        self.interaction.cancel(ptr);
        None
    }
    fn select_hit(
        &self,
        cur: &ReadModel,
        ptr: &web_sys::PointerEvent,
        hit: &CanvasHit,
        hit_context: &TreeContext,
    ) -> Option<SelectionMode> {
        let (rt, ad, sc, gn) = (&self.runtime, &self.adapter, &self.scope, self.generation);
        if !owner_ok(rt, self.workspace, ad, &self.owner) || !self.scope_ok() {
            return None;
        }
        if let Some(a) = &cur.accepted
            && let Some(h) = context_half(&resolved_document(&a.document), hit_context)
        {
            let mut sf = self.layout_state.split_focus();
            if h != sf() {
                sf.set(h);
            }
        }
        update_tree_cell_anchor(&self.tree_cell_anchor, &self.owner, cur, Some(hit_context));
        let ret = tree_cell_anchor_for_owner(&self.tree_cell_anchor, &self.owner);
        let proj = objects::context_for_selection_kind(
            cur,
            hit_context,
            (self.layout_state.selection_kind())(),
            ret.as_ref(),
        );
        let mode = match (ptr.shift_key(), ptr.ctrl_key() || ptr.meta_key()) {
            (true, _) => SelectionMode::Range,
            (false, true) => SelectionMode::Toggle,
            (false, false) => SelectionMode::Replace,
        };
        let context = proj
            .as_ref()
            .map(|p| p.context.clone())
            .unwrap_or_else(|| hit_context.clone());
        let cell = match (hit, hit_context) {
            (CanvasHit::Matrix(r), _) => {
                r.part_id.as_ref().map(|p| (r.matrix_id.clone(), p.clone()))
            }
            (
                CanvasHit::Part(r),
                TreeContext::Key {
                    matrix_id: m,
                    row,
                    column,
                },
            ) if r.part_id == format!("matrix/{m}/r{row}c{column}") => {
                Some((m.clone(), r.part_id.clone()))
            }
            _ => None,
        };
        if let Some((matrix_id, target_part_id)) = cell {
            submit_cell(
                rt,
                ad,
                sc,
                gn,
                MatrixCellSelection {
                    matrix_id,
                    target_part_id,
                    hit_context,
                    context,
                    mode,
                },
            );
            return Some(mode);
        }
        let mut eff = mode;
        if let CanvasHit::Part(r) = hit
            && mode == SelectionMode::Replace
            && cur.selected_part_ids.contains(&r.part_id)
        {
            let ids = proj.map_or_else(
                || resolve_context(cur, hit_context).unwrap_or_default(),
                |x| x.part_ids,
            );
            if ids == cur.selected_part_ids {
                if (ad.anchor_scope)().as_ref() == Some(sc)
                    && cur.selection_anchor_id.as_ref() == Some(&r.part_id)
                    && eligible_live_ids(cur).contains(&r.part_id)
                {
                    let mut s = ad.selected_context;
                    s.set(Some(ScopedTreeContext {
                        scope: sc.clone(),
                        context,
                    }));
                    return Some(mode);
                }
                eff = SelectionMode::Add;
            }
        }
        let rng = if eff == SelectionMode::Range {
            self.range_ids.to_vec()
        } else {
            Vec::new()
        };
        submit_sel(rt, ad, sc, gn, context, eff, rng);
        Some(mode)
    }
    fn on_down(&self, hit: CanvasHit) {
        let (ev, is_part) = match &hit {
            CanvasHit::Matrix(r) => (&r.event, false),
            CanvasHit::Part(r) => (&r.event, true),
        };
        let Some(ptr) = ev.data().try_as_web_event().filter(|p| p.button() == 0) else {
            return;
        };
        self.interaction.sync_control(ptr.ctrl_key());
        let ready = !is_part || (self.scope_ok() && self.runtime.model().gesture.is_none());
        let mode = self.interaction.current();
        if mode == Some(Mode::PickingMatrixAnchor) && self.try_anchor_pick(&ptr, &hit) {
            return;
        }
        if mode == Some(Mode::PickingSplayOrigin) {
            if ready {
                let _ = self.try_splay_pick(&ptr, true);
            }
            return;
        }
        if mode.is_none() && self.interaction.space_down() {
            if is_part && ready {
                prevent_and_focus(&ptr, &self.svg);
                self.pan(&ptr);
            }
            return;
        }
        if !matches!(mode, Some(Mode::PickingMatrixAnchor) | None)
            || !ready
            || self.try_splay_pick(&ptr, true)
        {
            return;
        }
        prevent_and_focus(&ptr, &self.svg);
        let cur = self.runtime.model();
        let (origin, hit_ctx) = match &hit {
            CanvasHit::Matrix(r) => (None, context_for_cell(&cur, &r.matrix_id, r.row, r.column)),
            CanvasHit::Part(r) => match self.world_pt(&ptr) {
                Some(o) => (Some(o), context_for_part(&cur, &r.part_id)),
                None => {
                    release_capture(&self.svg, ptr.pointer_id());
                    return;
                }
            },
        };
        if let Some(sm) = hit_ctx.and_then(|hc| self.select_hit(&cur, &ptr, &hit, &hc))
            && let (CanvasHit::Part(r), Some(origin_mm)) = (&hit, origin)
            && sm == SelectionMode::Replace
        {
            let cur = self.runtime.model();
            if let Some(snap) = cur
                .accepted
                .filter(|_| cur.selected_part_ids.contains(&r.part_id))
            {
                let positions: Vec<_> = snap
                    .document
                    .parts
                    .iter()
                    .filter(|p| cur.selected_part_ids.contains(&p.id))
                    .map(|p| Position {
                        id: p.id.clone(),
                        at: p.pose.at,
                    })
                    .collect();
                if !positions.is_empty() {
                    let seed = GestureSeed::PartDrag {
                        scope: self.scope.clone(),
                        generation: self.generation,
                        origin_mm,
                        positions,
                    };
                    capture_canvas_drag(
                        self.interaction,
                        ptr.pointer_id(),
                        client_point(&ptr),
                        ptr.ctrl_key(),
                        seed,
                        &self.runtime,
                        &self.svg,
                    );
                }
            }
        }
    }
}

impl CanvasAdapter {
    fn snap_part(&self, active: &ActivePartPlacement, pt: Vec2, free: bool) -> Vec2 {
        let s = &self.surface;
        let model = s.runtime.model();
        if model.accepted.is_none() {
            return pt;
        }
        let cfg = s.layout_state.snap_settings().read().clone();
        let pitch = matrix_snap_parameters(&model, &active.owner.scope, &s.adapter, None).0;
        let opts = PlacementSnapOptions {
            snap_fraction: cfg.snap_fraction,
            geometry_snap: cfg.geometry_snap,
            gap: placement_gap(&cfg),
            free,
            pitch,
        };
        snap_placement_at(
            &active.snap_document,
            &active.owner.board_id,
            &active.pending,
            pt,
            opts,
        )
    }
    fn place_ok(&self, scope: &Scope, gen_id: u64) -> bool {
        let s = &self.surface;
        (s.workspace)() == Workspace::Layout
            && s.runtime.scope().as_ref() == Some(scope)
            && (s.adapter.generation)() == gen_id
    }
    fn place_pointer(&self, mode: Mode, ptr: &web_sys::PointerEvent, commit: bool) {
        let s = &self.surface;
        match mode {
            Mode::PlacingMatrix => {
                if let Some(a) = &self.matrix_placement.placement {
                    let (owner, m) = (a.owner.clone(), &self.matrix_placement);
                    if !self.place_ok(&owner.scope, owner.scope_generation) {
                        m.on_cancel.call(owner);
                    } else if let Some(center) = s.world_pt(ptr) {
                        (if commit { m.on_commit } else { m.on_move })
                            .call(objects::MatrixPlacementMove { owner, center });
                    }
                }
            }
            Mode::PlacingMirroredPair => {
                if let Some(a) = &self.mirrored_pair.placement {
                    let (owner, m) = (a.owner.clone(), &self.mirrored_pair);
                    if !self.place_ok(&owner.scope, owner.scope_generation) {
                        m.on_cancel.call(owner);
                    } else if let Some(center) = s.world_pt(ptr) {
                        (if commit { m.on_commit } else { m.on_move })
                            .call(objects::MirroredPairMove { owner, center });
                    }
                }
            }
            Mode::PlacingPart => {
                if let (Some(a), Some(pt)) = (&self.part_placement.projection, s.world_pt(ptr)) {
                    let m = &self.part_placement;
                    (if commit { m.on_commit } else { m.on_move }).call(self.snap_part(
                        a,
                        pt,
                        ptr.alt_key(),
                    ));
                }
            }
            _ => {}
        }
    }
    fn handle_move(&self, event: PointerEvent) {
        let Some(ptr) = event.data().try_as_web_event() else {
            return;
        };
        let s = &self.surface;
        if s.preempt(&ptr, false) {
            return;
        }
        if let Some(m) = s.interaction.current().filter(|m| m.is_placing()) {
            self.place_pointer(m, &ptr, false);
            return;
        }
        if let (Some(LiveGesture::Parts(cur)), Some((pt, sc))) =
            (s.live_gesture(ptr.pointer_id()), s.world_scaled(&ptr))
            && (cur.active || self.begin_part_drag(&cur, &ptr, sc))
        {
            let sample = (pt, sc, ptr.alt_key());
            s.interaction.note_part_drag_sample(Some(sample));
            step_parts(&s.runtime, &cur, sample, false);
        }
    }
    fn begin_part_drag(&self, cur: &ActivePartDrag, ptr: &web_sys::PointerEvent, sc: f64) -> bool {
        let s = &self.surface;
        let ev = CanvasEvent::PointerMove {
            id: ptr.pointer_id(),
            client: client_point(ptr),
            operation: None,
        };
        if !s
            .interaction
            .handle(ev)
            .contains(&InteractionEffect::StartPartDrag)
        {
            return false;
        }
        let model = s.runtime.model();
        if model.gesture.is_some() {
            s.interaction.cancel(ptr.pointer_id());
            return false;
        }
        let ret = tree_cell_anchor_for_owner(&s.tree_cell_anchor, &s.owner);
        let (pitch, gap) = matrix_snap_parameters(&model, &cur.scope, &s.adapter, ret.as_ref());
        let snap = objects::gesture_snap_inputs(
            &s.layout_state.snap_settings().read().clone(),
            pitch,
            gap,
        );
        let (rt, inter, id) = (&s.runtime, s.interaction, cur.pointer_id);
        let Some(g) = begin_body_drag(
            rt,
            inter,
            id,
            None,
            &cur.positions,
            snap,
            (sc, ptr.alt_key()),
        ) else {
            return false;
        };
        if rt.scope().as_ref() == Some(&cur.scope) && (s.adapter.generation)() == cur.generation {
            inter.set_part_drag_gesture_generation(Some(g));
            return true;
        }
        inter.cancel(ptr.pointer_id());
        false
    }
    fn handle_end(&self, event: PointerEvent) {
        let Some(ptr) = event.data().try_as_web_event() else {
            return;
        };
        let s = &self.surface;
        if s.preempt(&ptr, true) {
            return;
        }
        let id = ptr.pointer_id();
        if let Some(m) = s.interaction.current().filter(|m| m.is_placing()) {
            if s.interaction.pointer() != Some(id) {
                return;
            }
            s.interaction.pointer_up(id);
            ptr.prevent_default();
            ptr.stop_propagation();
            if (m == Mode::PlacingMatrix && ptr.button() == 0)
                || (m == Mode::PlacingPart && pointer_release_commits(ptr.button()))
            {
                self.place_pointer(m, &ptr, true);
            }
            return;
        }
        let Some(g) = s.live_gesture(id) else {
            return;
        };
        s.interaction.pointer_up(id);
        let mut ver = self.interaction_version;
        ver += 1;
        match g {
            LiveGesture::Pan(p) => {
                self.navigation.finish_pan(
                    &s.runtime,
                    &s.svg,
                    p.start_camera,
                    p.origin_px,
                    client_point(&ptr),
                    (s.bounds.0.2, s.bounds.0.3),
                    id,
                );
            }
            LiveGesture::Parts(cur) => {
                if cur.active {
                    let smp = s
                        .world_scaled(&ptr)
                        .map(|(p, sc)| (p, sc, ptr.alt_key()))
                        .or(cur.last_sample);
                    if let Some(smp) = smp {
                        step_parts(&s.runtime, &cur, smp, true);
                    } else {
                        selection::cancel_drag_if_owned(
                            &s.runtime,
                            i64::from(id),
                            &cur.scope,
                            cur.gesture_generation,
                        );
                    }
                }
                release_capture(&s.svg, id);
            }
        }
    }
    fn handle_start_pan(&self, event: PointerEvent) {
        let Some(ptr) = event
            .data()
            .try_as_web_event()
            .filter(|p| !p.default_prevented() && p.button() == 0)
        else {
            return;
        };
        let s = &self.surface;
        s.interaction.sync_control(ptr.ctrl_key());
        let no_g = s.runtime.with_model(|m| m.gesture.is_none());
        let idle =
            s.interaction.current().is_none() && s.scope_ok() && no_g && s.interaction.space_down();
        let ws = (s.workspace)();
        if ws != Workspace::Layout {
            if matches!(ws, Workspace::Pcb | Workspace::Keymap | Workspace::Keycaps)
                && idle
                && s.pan(&ptr)
            {
                ptr.prevent_default();
            }
            return;
        }
        let id = ptr.pointer_id();
        if s.interaction.pointer().is_some_and(|o| o != id) {
            return;
        }
        match s.interaction.current() {
            Some(Mode::PickingSplayOrigin) => {
                s.try_splay_pick(&ptr, false);
            }
            Some(m) => {
                s.interaction
                    .pointer_down(id, client_point(&ptr), ptr.ctrl_key());
                if m == Mode::PlacingMirroredPair {
                    self.place_pointer(m, &ptr, true);
                }
            }
            None if owner_ok(&s.runtime, s.workspace, &s.adapter, &s.owner) && idle => {
                let _ = s.pan(&ptr);
            }
            None => return,
        }
        ptr.prevent_default();
    }
}

pub(in crate::presentation) fn capture_canvas_drag(
    interaction: LayoutInteraction,
    id: i32,
    client: (f64, f64),
    control: bool,
    seed: GestureSeed,
    runtime: &Rc<Runtime>,
    svg: &SvgRef,
) -> bool {
    let sc = match &seed {
        GestureSeed::Pan { scope, .. } | GestureSeed::PartDrag { scope, .. } => Some(scope.clone()),
        _ => None,
    };
    let Some(el) = svg.borrow().clone() else {
        return false;
    };
    if !interaction.capture_drag(id, client, control, seed.clone(), &el, None) {
        return false;
    }
    let (rt, svg) = (runtime.clone(), svg.clone());
    interaction.on_cancel(move || {
        if let Some(d) = interaction.take_canceled_drag() {
            selection::cancel_scoped_drag(&rt, &svg, sc.as_ref(), &d);
        }
    });
    true
}

fn step_parts(rt: &Rc<Runtime>, cur: &ActivePartDrag, (pt, sc, alt): (Vec2, f64, bool), end: bool) {
    let (dx, dy) = (pt.x - cur.origin_mm.x, pt.y - cur.origin_mm.y);
    let pos = cur
        .positions
        .iter()
        .map(|p| Position {
            id: p.id.clone(),
            at: Vec2 {
                x: p.at.x + dx,
                y: p.at.y + dy,
            },
        })
        .collect();
    submit_gesture_step(rt, cur.pointer_id, pos, sc, alt, end);
}

fn prevent_and_focus(ptr: &web_sys::PointerEvent, svg: &SvgRef) {
    ptr.prevent_default();
    ptr.stop_propagation();
    if let Some(s) = svg.borrow().as_ref() {
        focus_svg(s);
    }
}

pub(in crate::presentation) fn wheel_handler(
    runtime: Rc<Runtime>,
    svg: SvgRef,
    scope: Scope,
    adapter: SelectionAdapter,
    render_generation: u64,
    bounds: CanvasBounds,
) -> impl FnMut(WheelEvent) + Clone + 'static {
    let pending = Rc::new(std::cell::RefCell::new(
        layout_camera::WheelZoomCoalescer::new(),
    ));
    move |event: WheelEvent| {
        if runtime.scope().as_ref() != Some(&scope) || (adapter.generation)() != render_generation {
            return;
        }
        let Some(w) = event.data().try_as_web_event() else {
            return;
        };
        w.prevent_default();
        let (vx, vy, vw, vh) = bounds.0;
        let rect = svg.borrow().as_ref().map(|e| e.get_bounding_client_rect());
        let loc =
            rect.and_then(|r| pointer_location(&r, w.client_x(), w.client_y(), vx, vy, vw, vh));
        let (Some(l), Some(win)) = (loc, web_sys::window()) else {
            return;
        };
        let anc = layout_camera::WheelAnchor {
            world: l.world,
            x_fraction: l.x_fraction,
            y_fraction: l.y_fraction,
        };
        if !pending.borrow_mut().push(w.delta_y(), anc) {
            return;
        }
        let (pend, rt, sc, gf) = (
            pending.clone(),
            runtime.clone(),
            scope.clone(),
            adapter.generation,
        );
        let cb = Closure::once_into_js(move || {
            let mut p = pend.borrow_mut();
            let start = rt.model().camera.zoom;
            let zoom = p.coalesced_zoom(start, bounds.1).filter(|&z| z != start);
            if let (Some(zoom), Some(b)) = (zoom, p.take_batch())
                && rt.scope().as_ref() == Some(&sc)
                && gf() == render_generation
            {
                let anc = PointerLocation {
                    world: b.anchor.world,
                    x_fraction: b.anchor.x_fraction,
                    y_fraction: b.anchor.y_fraction,
                };
                let center = CanvasNavigationState::zoom_center_at(bounds.2, anc, zoom);
                CanvasNavigationState::apply_zoom_target(&rt, ZoomTarget { center, zoom });
            }
        });
        let _ = win.request_animation_frame(cb.unchecked_ref());
    }
}
