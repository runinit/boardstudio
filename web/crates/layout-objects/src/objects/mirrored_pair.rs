//! Private, source-parity projection for the new mirrored-pair placement.
use super::matrix_preparation::{
    matrix_dimensions_valid, matrix_source, owner_source_matches, prepare_matrix_definitions,
    reversible_layout_param, session_scope_matches,
};
use super::matrix_setup_fields::{
    MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview, matrix_setup_preset,
};
pub use super::workspace_state::{MirroredPairCreated, MirroredPairOwner};
use crate::matrix_setup_operation::{MatrixSetupPreset, MatrixSetupRequest, next_matrix_id};
use crate::{
    mirrored_pair_geometry::{MirroredPairGeometryInput, MirroredPairIds, project_mirrored_pair},
    mirrored_pair_lifecycle::{PairFormStage, PairFormState},
    presentation::canvas_interaction::{CanvasMode, LayoutInteraction},
    runtime::Runtime,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{EditOperation, Layout, Matrix, MatrixScene, PartDefinition, Vec2};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{NumericField, TextCommitField};
use boardstudio_web_ui_shared::ui::{alert::*, button::*, callout::*};
use dioxus::prelude::*;
use std::{
    cell::Cell,
    collections::{BTreeSet, HashSet},
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

mod canvas_overlay;
pub use canvas_overlay::MirroredPairCanvasOverlay;

#[derive(Clone, Copy)]
pub(super) struct MatrixFieldSignals {
    pub rows: Signal<String>,
    pub columns: Signal<String>,
    pub row_slider: Signal<Option<f64>>,
    pub column_slider: Signal<Option<f64>>,
    pub family: Signal<Option<HashSet<usize>>>,
    pub mounting: Signal<Option<HashSet<usize>>>,
    pub lighting: Signal<Option<HashSet<usize>>>,
}

impl MatrixFieldSignals {
    pub(super) fn new(rows: String, columns: String, c: (bool, bool, bool)) -> Self {
        let parse = |s: &str, max: f64, def: f64| {
            s.trim()
                .parse::<f64>()
                .ok()
                .filter(|v| (1.0..=max).contains(v))
                .unwrap_or(def)
        };
        let (r, col) = (parse(&rows, 12.0, 5.0), parse(&columns, 18.0, 6.0));
        Self {
            rows: Signal::new(rows),
            columns: Signal::new(columns),
            row_slider: Signal::new(Some(r)),
            column_slider: Signal::new(Some(col)),
            family: Signal::new(Some(HashSet::from([usize::from(c.0)]))),
            mounting: Signal::new(Some(HashSet::from([usize::from(c.1)]))),
            lighting: Signal::new(Some(HashSet::from([usize::from(c.2)]))),
        }
    }
    pub(super) fn preset(self) -> MatrixSetupPreset {
        let on = |s: Signal<Option<HashSet<usize>>>| s().is_some_and(|c| c.contains(&1));
        matrix_setup_preset(on(self.family), on(self.mounting), on(self.lighting))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairRequest {
    pub owner: MirroredPairOwner,
    pub left_name: String,
    pub right_name: String,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
    pub gap_mm: f64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormProjection {
    pub owner: MirroredPairOwner,
    pub values: MirroredPairFormValues,
    pub editable: bool,
    pub error: Option<String>,
    pub status: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MirroredPairFormValues {
    pub left_name: String,
    pub right_name: String,
    pub rows: String,
    pub columns: String,
    pub preset: MatrixSetupPreset,
    pub gap_mm: String,
}
impl Default for MirroredPairFormValues {
    fn default() -> Self {
        Self {
            left_name: "Left half".into(),
            right_name: "Right half".into(),
            rows: "3".into(),
            columns: "5".into(),
            preset: MatrixSetupPreset::MxSolder,
            gap_mm: "40".into(),
        }
    }
}
#[derive(Clone, PartialEq)]
pub struct MirroredPairSession {
    pub form: Option<MirroredPairFormProjection>,
    pub placement: Option<MirroredPairPlacement>,
    pub can_open: bool,
    pub owns_canvas: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
    pub on_move: EventHandler<MirroredPairMove>,
    pub on_commit: EventHandler<MirroredPairMove>,
    pub on_created: EventHandler<MirroredPairCreated>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairPlacement {
    pub owner: MirroredPairOwner,
    pub pair: crate::mirrored_pair_geometry::MirroredPairProjection,
    pub left_scene: MatrixScene,
    pub right_scene: MatrixScene,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MirroredPairMove {
    pub owner: MirroredPairOwner,
    pub center: Vec2,
}
#[derive(Props, Clone, PartialEq)]
pub struct MirroredPairFormProps {
    pub projection: MirroredPairFormProjection,
    pub on_cancel: EventHandler<MirroredPairOwner>,
    pub on_preview: EventHandler<MirroredPairRequest>,
}

#[component]
pub fn MirroredPairForm(props: MirroredPairFormProps) -> Element {
    let v = props.projection.values.clone();
    let mut text = use_signal(|| (v.left_name, v.right_name, v.gap_mm));
    let f = use_hook(|| MatrixFieldSignals::new(v.rows, v.columns, preset_choices(v.preset)));
    let (rv, cv, (left, right, gap), preset) = ((f.rows)(), (f.columns)(), text(), f.preset());
    let gap_mm = gap
        .trim()
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite() && *v >= 0.0);
    let dims = super::matrix_setup_fields::parse_matrix_dimensions(&rv, &cv);
    let valid_names =
        !left.trim().is_empty() && !right.trim().is_empty() && left.trim() != right.trim();
    let can_preview =
        props.projection.editable && dims.is_some() && gap_mm.is_some() && valid_names;
    let (owner, on_preview, on_cancel) = (
        props.projection.owner.clone(),
        props.on_preview,
        props.on_cancel,
    );
    let (so, sl, sr) = (owner.clone(), left.clone(), right.clone());
    let sid = dioxus::core::current_scope_id().0;
    let dis = (!props.projection.editable).then(|| "Placement is unavailable".to_owned());
    rsx! {
        section {
            class: "pointer-events-auto relative box-border flex max-h-full w-[min(380px,100%)] flex-col gap-0 overflow-y-auto rounded-md border border-border-strong bg-card p-6 text-foreground shadow-md max-[760px]:w-[min(380px,calc(100%-8px))] max-[760px]:p-[18px]",
            aria_label: "New mirrored pair", onkeydown: super::stop_space_pan,
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-xl!", h2 { "Mirrored pair" } }
            p { class: "mt-2.5 mb-5 text-[13px] leading-normal text-muted-foreground", "Linked key assemblies, diode settings and components. Substitute a component on either half when needed." }
            form {
                class: "grid gap-2",
                onsubmit: move |e: FormEvent| {
                    e.prevent_default();
                    let (Some((rows, columns)), Some(gap_mm)) = (dims, gap_mm) else { return; };
                    let (left_name, right_name) = (sl.trim().to_owned(), sr.trim().to_owned());
                    if !left_name.is_empty() && !right_name.is_empty() && left_name != right_name {
                        on_preview.call(MirroredPairRequest { owner: so.clone(), left_name, right_name, rows, columns, preset, gap_mm });
                    }
                },
                div { class: "grid grid-cols-2 gap-x-4 gap-y-3", role: "group", aria_label: "Mirrored pair dimensions",
                    TextCommitField { label: "Left layout", value: left, id: format!("layout-input-{sid}-left"), aria_label: "Left layout name", disabled_reason: dis.clone(), on_commit: move |n| text.write().0 = n }
                    TextCommitField { label: "Right layout", value: right, id: format!("layout-input-{sid}-right"), aria_label: "Right layout name", disabled_reason: dis.clone(), on_commit: move |n| text.write().1 = n }
                }
                MatrixDimensionSliders { rows: f.rows, columns: f.columns, row_slider: f.row_slider, column_slider: f.column_slider, rows_label: "Rows per half", columns_label: "Columns per half", editable: props.projection.editable }
                MatrixAssemblyPicker { family: f.family, mounting: f.mounting, lighting: f.lighting, editable: props.projection.editable }
                MatrixPreview { rows: rv, columns: cv }
                NumericField { label: "Gap between key edges", unit: "mm", value: gap_mm.unwrap_or(0.0), min: 0.0, id: format!("layout-input-{sid}-gap"), aria_label: "Gap between key edges (mm)", disabled_reason: dis, on_commit: move |n: f64| text.write().2 = n.to_string() }
                p { class: "mt-2.5 mb-5 border-t border-border pt-3.5 text-[13px] leading-normal text-muted-foreground", "Edit either half to update both. Unlink in the inspector for independent geometry." }
                if let Some(error) = props.projection.error.clone() { Alert { variant: AlertVariant::Destructive, AlertDescription { "{error}" } } }
                if let Some(status) = props.projection.status.clone() { div { role: "status", Callout { variant: CalloutVariant::Info, "{status}" } } }
                footer { class: "flex justify-end gap-2",
                    Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| on_cancel.call(owner.clone()), "Cancel" }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !can_preview, "Preview placement" }
                }
            }
        }
    }
}

fn preset_choices(p: MatrixSetupPreset) -> (bool, bool, bool) {
    use MatrixSetupPreset::*;
    (
        matches!(p, ChocSolder | ChocHotswap | ChocRgb | ChocHotswapRgb),
        matches!(p, MxHotswap | ChocHotswap | MxHotswapRgb | ChocHotswapRgb),
        matches!(p, MxRgb | ChocRgb | MxHotswapRgb | ChocHotswapRgb),
    )
}

fn mirrored_pair_resolver(
    board_id: String,
    left: Layout,
    right: Layout,
    matrix: Matrix,
    right_mid: String,
    definitions: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new("layout-mirrored-pair", move |acc: &AcceptedSnapshot| {
        let doc = &acc.document;
        if !doc.boards.iter().any(|b| b.id == board_id) {
            return Resolution::Retire("The board no longer exists.".into());
        }
        if [&matrix.id, &right_mid, &left.id, &right.id]
            .into_iter()
            .any(|id| {
                doc.matrices.iter().any(|m| m.id == *id) || doc.layouts.iter().any(|l| l.id == *id)
            })
        {
            return Resolution::Retire("The mirrored pair's identity is already in use.".into());
        }
        Resolution::submit(
            vec![
                matrix.id.clone(),
                right_mid.clone(),
                left.id.clone(),
                right.id.clone(),
            ],
            EditOperation::CreateMirroredPair {
                left: left.clone(),
                right: right.clone(),
                matrix: Box::new(matrix.clone()),
                definitions: Some(definitions.clone()),
            },
        )
    })
}

#[derive(Clone)]
struct ActivePair {
    request: MirroredPairRequest,
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
    ids: MirroredPairIds,
    placement: MirroredPairPlacement,
}

#[derive(Clone)]
struct PairUi {
    next_open_id: u64,
    open: Option<MirroredPairOwner>,
    form_state: PairFormState<MirroredPairFormValues>,
    preparing: Option<MirroredPairOwner>,
    placement: Option<ActivePair>,
    error: Option<String>,
    status: Option<String>,
}

impl Default for PairUi {
    fn default() -> Self {
        Self {
            next_open_id: 0,
            open: None,
            form_state: PairFormState::new(MirroredPairFormValues::default()),
            preparing: None,
            placement: None,
            error: None,
            status: None,
        }
    }
}

impl PairUi {
    fn reset(&mut self) {
        self.open = None;
        self.preparing = None;
        self.placement = None;
        self.error = None;
        self.status = None;
        self.form_state = PairFormState::new(MirroredPairFormValues::default());
    }
    fn fail(&mut self, msg: String) {
        self.form_state.stage = PairFormStage::Setup;
        self.preparing = None;
        self.error = Some(msg);
        self.status = None;
    }
    fn clear_stale(&mut self, owner: &MirroredPairOwner) {
        if self.preparing.as_ref() == Some(owner) {
            self.preparing = None;
            self.form_state.stage = PairFormStage::Setup;
            self.status = None;
            if self.open.as_ref() == Some(owner) {
                self.error = Some("The accepted board changed. Cancel this setup and reopen it before placing a pair.".into());
            }
        }
    }
}

fn owner_live(
    rt: &Runtime,
    owner: &MirroredPairOwner,
    ws: Workspace,
    gen_id: u64,
    open: Option<&MirroredPairOwner>,
) -> bool {
    if open != Some(owner)
        || ws != Workspace::Layout
        || gen_id != owner.scope_generation
        || rt.scope().as_ref() != Some(&owner.scope)
    {
        return false;
    }
    matrix_source(rt, &rt.model(), rt.scope().as_ref(), ws).is_some_and(|(s, sc, _)| {
        owner_source_matches(&owner.scope, owner.snapshot_token, owner.revision, &sc, s)
    })
}

pub fn use_mirrored_pair(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<Workspace>,
    scope_generation: Signal<u64>,
    on_created: EventHandler<MirroredPairCreated>,
) -> MirroredPairSession {
    let inter = use_context::<LayoutInteraction>();
    let mut ui = use_signal(PairUi::default);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let a = alive.clone();
        move || a.set(false)
    });

    use_effect(use_reactive(
        (&version(), &workspace(), &scope_generation()),
        {
            let rt = runtime.clone();
            move |(_, ws, generation)| {
                let Some(o) = ui.read().open.clone() else {
                    return;
                };
                if ws != Workspace::Layout
                    || o.scope_generation != generation
                    || !session_scope_matches(&rt, &o.scope)
                {
                    ui.write().reset();
                    inter.end(CanvasMode::PlacingMirroredPair);
                } else if rt.model().accepted.as_ref().is_none_or(|s| {
                    !owner_source_matches(&o.scope, o.snapshot_token, o.revision, &o.scope, s)
                }) {
                    let mut s = ui.write();
                    s.preparing = None;
                    s.placement = None;
                    s.form_state.stage = PairFormStage::Setup;
                    s.status = None;
                    s.error = Some("The accepted board changed. Cancel this setup and reopen it before placing a pair.".into());
                }
            }
        },
    ));

    let on_open = EventHandler::new({
        let rt = runtime.clone();
        move |_| {
            if ui.read().open.is_some()
                || ui.read().preparing.is_some()
                || ui.read().placement.is_some()
            {
                return;
            }
            let (m, sc) = (rt.model(), rt.scope());
            let Some((snap, scope, board_id)) = matrix_source(&rt, &m, sc.as_ref(), workspace())
            else {
                ui.write().error = Some(
                    "Open a saved, editable board in Layout before creating a mirrored pair."
                        .into(),
                );
                return;
            };
            if !inter.begin(CanvasMode::PlacingMirroredPair) {
                return;
            }
            let mut s = ui.write();
            s.next_open_id = s.next_open_id.saturating_add(1);
            s.form_state = PairFormState::new(MirroredPairFormValues::default());
            s.error = None;
            s.status = None;
            s.open = Some(MirroredPairOwner {
                editor_instance_id: 0,
                open_id: s.next_open_id,
                scope_generation: scope_generation(),
                scope,
                board_id,
                snapshot_token: snap.token,
                revision: snap.document.revision,
            });
        }
    });

    let on_cancel = EventHandler::new(move |o: MirroredPairOwner| {
        let active = {
            let s = ui.read();
            s.open.as_ref() == Some(&o)
                || s.placement.as_ref().is_some_and(|p| p.placement.owner == o)
        };
        if crate::mirrored_pair_lifecycle::pair_cancel_is_allowed(active, false) {
            ui.write().reset();
            inter.end(CanvasMode::PlacingMirroredPair);
        }
    });

    let on_preview = EventHandler::new({
        let (rt, alive) = (runtime.clone(), alive.clone());
        move |req: MirroredPairRequest| {
            start_preview(&rt, inter, ui, &alive, workspace, scope_generation, req)
        }
    });

    let on_move = EventHandler::new({
        let rt = runtime.clone();
        move |req: MirroredPairMove| {
            if inter.current() != Some(CanvasMode::PlacingMirroredPair) {
                return;
            }
            let Some(mut act) = ui
                .read()
                .placement
                .clone()
                .filter(|a| a.placement.owner == req.owner)
            else {
                return;
            };
            if !owner_live(
                &rt,
                &req.owner,
                workspace(),
                scope_generation(),
                ui.read().open.as_ref(),
            ) {
                return;
            }
            match project_pair(&act.matrix, &act.ids, &act.request, req.center) {
                Ok(pair) => {
                    act.placement.pair = pair;
                    let mut s = ui.write();
                    s.placement = Some(act);
                    s.error = None;
                }
                Err(msg) => ui.write().error = Some(msg),
            }
        }
    });

    let on_commit = EventHandler::new({
        let rt = runtime.clone();
        move |req: MirroredPairMove| {
            let owner = req.owner;
            if inter.current() != Some(CanvasMode::PlacingMirroredPair)
                || !owner_live(
                    &rt,
                    &owner,
                    workspace(),
                    scope_generation(),
                    ui.read().open.as_ref(),
                )
            {
                return;
            }
            let Some(act) = ui
                .read()
                .placement
                .clone()
                .filter(|a| a.placement.owner == owner)
            else {
                return;
            };
            let pair = match project_pair(&act.matrix, &act.ids, &act.request, req.center) {
                Ok(pair) => pair,
                Err(msg) => {
                    ui.write().error = Some(msg);
                    return;
                }
            };
            let (left_id, right_id, left_mid, right_mid) = (
                pair.left.id.clone(),
                pair.right.id.clone(),
                pair.matrix.id.clone(),
                act.ids.right_matrix_id.clone(),
            );
            match rt.commit(
                "layout-mirrored-pair",
                mirrored_pair_resolver(
                    owner.board_id.clone(),
                    pair.left,
                    pair.right,
                    pair.matrix,
                    right_mid.clone(),
                    act.definitions,
                ),
            ) {
                Ok(snap) => {
                    let on_board = rt.model().active_board_id == owner.board_id;
                    ui.write().reset();
                    inter.end(CanvasMode::PlacingMirroredPair);
                    if on_board {
                        on_created.call(MirroredPairCreated {
                            scope: owner.scope.clone(),
                            owner,
                            result_token: snap.token,
                            result_revision: snap.document.revision,
                            left_layout_id: left_id,
                            right_layout_id: right_id,
                            left_matrix_id: left_mid,
                            right_matrix_id: right_mid,
                        });
                    }
                }
                Err(msg) => {
                    let mut s = ui.write();
                    s.placement = None;
                    s.form_state.stage = PairFormStage::Setup;
                    s.error = Some(msg);
                    s.status = None;
                }
            }
        }
    });

    let s = ui.read();
    let form = s
        .form_state
        .setup_is_visible()
        .then(|| {
            s.open.clone().map(|owner| MirroredPairFormProjection {
                editable: s.form_state.setup_is_editable()
                    && s.preparing.is_none()
                    && owner_live(
                        &runtime,
                        &owner,
                        workspace(),
                        scope_generation(),
                        s.open.as_ref(),
                    ),
                owner,
                values: s.form_state.values.clone(),
                error: s.error.clone(),
                status: s.status.clone(),
            })
        })
        .flatten();
    let owns_canvas = s.open.is_some() || s.preparing.is_some() || s.placement.is_some();
    let can_open = inter.current().is_none()
        && !owns_canvas
        && matrix_source(
            &runtime,
            &runtime.model(),
            runtime.scope().as_ref(),
            workspace(),
        )
        .is_some();
    MirroredPairSession {
        form,
        placement: s.placement.as_ref().map(|a| a.placement.clone()),
        can_open,
        owns_canvas,
        on_open,
        on_cancel,
        on_preview,
        on_move,
        on_commit,
        on_created,
    }
}

fn start_preview(
    rt: &Rc<Runtime>,
    inter: LayoutInteraction,
    mut ui: Signal<PairUi>,
    alive: &Rc<Cell<bool>>,
    ws: Signal<Workspace>,
    gen_sig: Signal<u64>,
    req: MirroredPairRequest,
) {
    let ok = {
        let s = ui.read();
        inter.current() == Some(CanvasMode::PlacingMirroredPair)
            && s.open.as_ref() == Some(&req.owner)
            && s.preparing.is_none()
            && s.placement.is_none()
            && s.form_state.setup_is_editable()
            && ws() == Workspace::Layout
            && gen_sig() == req.owner.scope_generation
    };
    if !ok {
        return;
    }
    if !matrix_dimensions_valid(req.rows, req.columns)
        || req.gap_mm < 0.0
        || !req.gap_mm.is_finite()
        || req.left_name.trim().is_empty()
        || req.right_name.trim().is_empty()
        || req.left_name.trim() == req.right_name.trim()
    {
        ui.write().error = Some(
            "Choose distinct names, a non-negative gap, and dimensions up to 4096 cells.".into(),
        );
        return;
    }
    let (model, live_scope) = (rt.model(), rt.scope());
    let Some((snap, scope, board_id)) = matrix_source(rt, &model, live_scope.as_ref(), ws()) else {
        ui.write().error =
            Some("The selected board is no longer ready for mirrored-pair placement.".into());
        return;
    };
    if !owner_source_matches(
        &req.owner.scope,
        req.owner.snapshot_token,
        req.owner.revision,
        &scope,
        snap,
    ) {
        ui.write().error = Some(
            "The accepted board changed. Close this setup and reopen it before placing a pair."
                .into(),
        );
        return;
    }
    let rev = reversible_layout_param(&snap.document);
    let left_mid = next_matrix_id(
        &snap.document.matrices,
        &snap.document.definitions,
        &snap.document.parts,
        req.preset.as_str(),
        rev,
    );
    {
        let mut s = ui.write();
        s.form_state = PairFormState {
            stage: PairFormStage::Preparing,
            values: MirroredPairFormValues {
                left_name: req.left_name.clone(),
                right_name: req.right_name.clone(),
                rows: req.rows.to_string(),
                columns: req.columns.to_string(),
                preset: req.preset,
                gap_mm: req.gap_mm.to_string(),
            },
        };
        s.preparing = Some(req.owner.clone());
        s.error = None;
        s.status = Some("Preparing mirrored pair definitions…".into());
    }
    let (rt, alive, acc, owner) = (rt.clone(), alive.clone(), snap.clone(), req.owner.clone());
    spawn_local(async move {
        let cur =
            || alive.get() && owner_live(&rt, &owner, ws(), gen_sig(), ui.read().open.as_ref());
        let prep = prepare_matrix_definitions(
            left_mid,
            board_id,
            MatrixSetupRequest {
                rows: req.rows,
                columns: req.columns,
                preset: req.preset,
            },
            rev,
            cur,
        )
        .await;
        if !alive.get() {
            return;
        }
        let prep = match prep {
            Ok(Some(p)) if cur() => p,
            Err(msg) if cur() => {
                ui.write().fail(msg);
                return;
            }
            _ => {
                ui.write().clear_stale(&owner);
                return;
            }
        };
        match build_active_pair(&rt, &acc, owner.clone(), req, prep.matrix, prep.definitions) {
            Ok(active) if cur() => {
                let mut s = ui.write();
                s.placement = Some(active);
                s.form_state.stage = PairFormStage::Placement;
                s.preparing = None;
                s.error = None;
                s.status = Some("Place linked halves · Click to place · Esc cancels".into());
            }
            Err(msg) if cur() => ui.write().fail(msg),
            _ => ui.write().clear_stale(&owner),
        }
    });
}

fn project_pair(
    matrix: &Matrix,
    ids: &MirroredPairIds,
    req: &MirroredPairRequest,
    center: Vec2,
) -> Result<crate::mirrored_pair_geometry::MirroredPairProjection, String> {
    project_mirrored_pair(
        matrix.clone(),
        ids,
        &MirroredPairGeometryInput {
            left_name: req.left_name.clone(),
            right_name: req.right_name.clone(),
            gap_mm: req.gap_mm,
        },
        center,
    )
}

fn build_active_pair(
    rt: &Runtime,
    acc: &AcceptedSnapshot,
    owner: MirroredPairOwner,
    mut req: MirroredPairRequest,
    matrix: Matrix,
    definitions: Vec<PartDefinition>,
) -> Result<ActivePair, String> {
    let mut matrices = acc.document.matrices.clone();
    matrices.push(matrix.clone());
    let mut defs = acc.document.definitions.clone();
    defs.extend(definitions.iter().cloned());
    let rev = reversible_layout_param(&acc.document);
    let ids = MirroredPairIds {
        right_matrix_id: next_matrix_id(
            &matrices,
            &defs,
            &acc.document.parts,
            req.preset.as_str(),
            rev,
        ),
        left_layout_id: next_layout_id(&acc.document.layouts, "mirrored-left"),
        right_layout_id: next_layout_id(&acc.document.layouts, "mirrored-right"),
    };
    let initial = project_pair(&matrix, &ids, &req, Vec2::default())?;
    // Ticket 11: propose the stored gap before preview (translation-invariant across placement moves).
    let adapted_gap = boardstudio_core_kernel::matrix::layout::proposed_creation_gap(
        &acc.document,
        &initial.matrix,
        initial.axis_x,
    )?;
    let prev = req.gap_mm;
    req.gap_mm = adapted_gap;
    let pair = if adapted_gap > prev {
        project_pair(&matrix, &ids, &req, Vec2::default())?
    } else {
        initial
    };
    let scenes = rt.project_matrices(
        acc.document.revision,
        vec![pair.matrix.clone(), pair.right_preview.clone()],
    )?;
    let [left_scene, right_scene] = scenes.as_slice() else {
        return Err("Core returned an incomplete mirrored-pair preview.".into());
    };
    Ok(ActivePair {
        request: req,
        matrix,
        definitions,
        ids,
        placement: MirroredPairPlacement {
            owner,
            pair,
            left_scene: left_scene.clone(),
            right_scene: right_scene.clone(),
        },
    })
}

fn next_layout_id(layouts: &[Layout], prefix: &str) -> String {
    let used: BTreeSet<&str> = layouts.iter().map(|l| l.id.as_str()).collect();
    (1u64..)
        .map(|n| format!("{prefix}-{n}"))
        .find(|c| !used.contains(c.as_str()))
        .unwrap_or_else(|| format!("{prefix}-next"))
}
