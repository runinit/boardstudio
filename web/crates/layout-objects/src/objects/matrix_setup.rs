//! Private immediate-origin Matrix Setup form, shared by Layout and the project guide.
use super::matrix_preparation::{
    matrix_source, owner_session_current, prepare_matrix_definitions, reversible_layout_param,
};
use super::matrix_setup_fields::{MatrixAssemblyPicker, MatrixDimensionSliders, MatrixPreview};
use super::mirrored_pair::MatrixFieldSignals;
use super::{ScopedTreeContext, TreeContext};
use crate::matrix_setup_operation::{MatrixSetupPreset, MatrixSetupRequest, next_matrix_id};
pub use crate::presentation::parts::MatrixPlacementSource;
use crate::presentation::parts::{
    SwitchOrientation, matrix_setup_preset as part_preset, matrix_with_assembly,
    normalize_matrix_definition,
};
use crate::{
    presentation::canvas_interaction::{CanvasMode, LayoutInteraction},
    runtime::Runtime,
};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Event, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{
    EditOperation, Matrix, MatrixCell, MatrixScene, PartDefinition, Vec2,
};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::ui::{alert::*, button::*, callout::*};
use dioxus::prelude::*;
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupOwner {
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupProjection {
    pub owner: MatrixSetupOwner,
    pub editable: bool,
    pub can_cancel: bool,
    pub status: Option<String>,
    pub error: Option<String>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixSetupCreateRequest {
    pub owner: MatrixSetupOwner,
    pub rows: u32,
    pub columns: u32,
    pub preset: MatrixSetupPreset,
}
#[derive(Clone, PartialEq)]
pub struct MatrixSetupMount {
    pub projection: Option<MatrixSetupProjection>,
    pub can_open: bool,
    pub on_open: EventHandler<()>,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixPlacementOwner {
    // `scope_generation` is read by the canvas placement adapter outside this
    // crate; retire it together with that consumer instead of here.
    pub scope_generation: u64,
    pub scope: Scope,
    pub board_id: String,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementProjection {
    pub owner: MatrixPlacementOwner,
    pub matrix: Matrix,
    pub scene: MatrixScene,
    pub definitions: Vec<PartDefinition>,
}
#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPlacementMove {
    pub owner: MatrixPlacementOwner,
    pub center: Vec2,
}
#[derive(Clone, PartialEq)]
pub struct MatrixPlacementSession {
    pub cancel_owner: Option<MatrixPlacementOwner>,
    pub placement: Option<MatrixPlacementProjection>,
    pub busy: bool,
    pub error: Option<String>,
    pub on_place: EventHandler<MatrixPlacementSource>,
    pub on_move: EventHandler<MatrixPlacementMove>,
    pub on_commit: EventHandler<MatrixPlacementMove>,
    pub on_cancel: EventHandler<MatrixPlacementOwner>,
}
#[derive(Clone)]
pub struct MatrixPlacementInput {
    pub version: Signal<u64>,
    pub selected_context: Signal<Option<ScopedTreeContext>>,
    pub anchor_scope: Signal<Option<Scope>>,
    pub workspace: Signal<Workspace>,
    pub scope_generation: Signal<u64>,
    pub assembly_orientation: crate::presentation::parts::PartsAssemblyOrientation,
}
#[derive(Props, Clone, PartialEq)]
pub struct MatrixSetupProps {
    pub projection: MatrixSetupProjection,
    pub on_cancel: EventHandler<MatrixSetupOwner>,
    pub on_create: EventHandler<MatrixSetupCreateRequest>,
}

fn focus_el(doc: &web_sys::Document, selector: &str) {
    if let Ok(Some(el)) = doc.query_selector(selector)
        && let Ok(h) = el.dyn_into::<web_sys::HtmlElement>()
    {
        let _ = h.focus();
    }
}

#[component]
pub fn MatrixSetup(props: MatrixSetupProps) -> Element {
    let f = use_hook(|| MatrixFieldSignals::new("5".into(), "6".into(), (false, false, false)));
    let mut ret_focus = use_signal(|| None::<web_sys::HtmlElement>);
    let (rv, cv, preset) = ((f.rows)(), (f.columns)(), f.preset());
    let (owner, create_cb, cancel_cb) = (
        props.projection.owner.clone(),
        props.on_create,
        props.on_cancel,
    );
    let (so, sr, sc) = (owner.clone(), rv.clone(), cv.clone());
    rsx! {
        section {
            class: "absolute inset-x-2 top-[100px] bottom-2 z-6 flex flex-col gap-2.5 overflow-auto rounded-md border border-border-strong bg-card p-3 text-foreground shadow-md [&_form]:grid [&_form]:gap-2 [&_footer]:flex [&_footer]:justify-end [&_footer]:gap-2",
            aria_label: "New matrix", onkeydown: super::stop_space_pan,
            header { class: "flex items-baseline justify-between gap-2 [&_h2]:m-0 [&_h2]:text-[16px]!", h2 { "New matrix" } }
            form {
                onsubmit: move |e: FormEvent| { e.prevent_default(); if let (Ok(rows), Ok(columns)) = (sr.trim().parse(), sc.trim().parse()) { create_cb.call(MatrixSetupCreateRequest { owner: so.clone(), rows, columns, preset }); } },
                div { id: "m1-matrix-rows-focus",
                    onmounted: move |_| async move {
                        let Some(d) = web_sys::window().and_then(|w| w.document()) else { return; };
                        if let Some(el) = d.active_element().and_then(|t| t.dyn_into::<web_sys::HtmlElement>().ok()) { ret_focus.set(Some(el)); }
                        focus_el(&d, "#m1-matrix-rows-focus [role='slider'][aria-label='Rows']");
                    },
                    MatrixDimensionSliders { rows: f.rows, columns: f.columns, row_slider: f.row_slider, column_slider: f.column_slider, rows_label: "Rows", columns_label: "Columns", editable: props.projection.editable }
                }
                MatrixAssemblyPicker { family: f.family, mounting: f.mounting, lighting: f.lighting, editable: props.projection.editable }
                MatrixPreview { rows: rv, columns: cv }
                if let Some(m) = props.projection.error.clone() { Alert { variant: AlertVariant::Destructive, AlertDescription { "{m}" } } }
                if let Some(m) = props.projection.status.clone() { div { role: "status", Callout { variant: CalloutVariant::Info, "{m}" } } }
                footer {
                    if props.projection.can_cancel {
                        Button { variant: ButtonVariant::Secondary, button_type: "button", onclick: move |_| {
                            cancel_cb.call(owner.clone());
                            let prev = ret_focus();
                            spawn_local(async move {
                                gloo_timers::future::TimeoutFuture::new(0).await;
                                let Some(d) = web_sys::window().and_then(|w| w.document()) else { return; };
                                if let Some(t) = prev.filter(|t| t.is_connected()) { let _ = t.focus(); } else { focus_el(&d, "#m1-setup-guide-heading"); }
                            });
                        }, "Cancel" }
                    }
                    Button { variant: ButtonVariant::Secondary, button_type: "submit", disabled: !props.projection.editable, "Create matrix" }
                }
            }
        }
    }
}

/// Resolve placing a prepared matrix against the accepted document at execution time.
/// Only the definitions the accepted document does not yet have are sent, so a placement
/// running after an edit that already added one still lands.
fn place_matrix_resolver(matrix: Matrix, definitions: Vec<PartDefinition>) -> EditResolver {
    EditResolver::new("layout-matrix-placement", move |acc: &AcceptedSnapshot| {
        let doc = &acc.document;
        if matrix
            .board_id
            .as_deref()
            .is_some_and(|id| !doc.boards.iter().any(|b| b.id == id))
        {
            return Resolution::Retire("The board no longer exists.".into());
        }
        if doc.matrices.iter().any(|e| e.id == matrix.id) {
            return Resolution::Retire("The matrix identity is already in use.".into());
        }
        let req = definitions
            .iter()
            .filter(|d| !doc.definitions.iter().any(|e| e.id == d.id))
            .cloned()
            .collect();
        Resolution::submit(
            vec![matrix.id.clone()],
            EditOperation::SetMatrix {
                matrix: matrix.clone(),
                definitions: Some(req),
            },
        )
    })
}

#[derive(Clone, Default)]
struct PlacementUi {
    preparing: Option<MatrixPlacementOwner>,
    placement: Option<MatrixPlacementProjection>,
    error: Option<String>,
}

#[derive(Clone)]
struct PlaceCtx {
    rt: Rc<Runtime>,
    inter: LayoutInteraction,
    ui: Signal<PlacementUi>,
    alive: Rc<Cell<bool>>,
    input: MatrixPlacementInput,
}

impl PlaceCtx {
    fn live(&self, o: &MatrixPlacementOwner) -> bool {
        (self.input.workspace)() == Workspace::Layout
            && (self.input.scope_generation)() == o.scope_generation
            && owner_session_current(
                &self.rt,
                &o.scope,
                &o.board_id,
                o.snapshot_token,
                o.revision,
            )
    }

    fn clear(&self) {
        let mut ui = self.ui;
        let mut s = ui.write();
        (s.preparing, s.placement, s.error) = (None, None, None);
        self.inter.end(CanvasMode::PlacingMatrix);
    }
}

pub fn use_matrix_placement(
    runtime: Rc<Runtime>,
    input: MatrixPlacementInput,
) -> MatrixPlacementSession {
    let inter = use_context::<LayoutInteraction>();
    let ui = use_signal(PlacementUi::default);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let a = alive.clone();
        move || {
            a.set(false);
            inter.end(CanvasMode::PlacingMatrix);
        }
    });
    let ctx = PlaceCtx {
        rt: runtime,
        inter,
        ui,
        alive,
        input,
    };

    use_effect(use_reactive(
        (
            &(ctx.input.version)(),
            &(ctx.input.workspace)(),
            &(ctx.input.scope_generation)(),
        ),
        {
            let c = ctx.clone();
            move |_| {
                let act = {
                    let s = c.ui.peek();
                    s.preparing
                        .clone()
                        .or_else(|| s.placement.as_ref().map(|p| p.owner.clone()))
                };
                if let Some(o) = act
                    && !c.live(&o)
                {
                    c.clear();
                }
            }
        },
    ));

    let on_place = use_callback({
        let c = ctx.clone();
        move |src| start_place(&c, src)
    });
    let on_move = use_callback({
        let c = ctx.clone();
        move |m: MatrixPlacementMove| move_place(&c, m)
    });
    let on_commit = use_callback({
        let c = ctx.clone();
        move |m| commit_place(&c, m)
    });
    let on_cancel = use_callback({
        let c = ctx;
        move |o: MatrixPlacementOwner| {
            let act = {
                let s = c.ui.read();
                s.preparing.as_ref() == Some(&o)
                    || s.placement.as_ref().is_some_and(|p| p.owner == o)
            };
            if act {
                c.clear();
            }
        }
    });

    let s = ui.read();
    MatrixPlacementSession {
        cancel_owner: s
            .preparing
            .clone()
            .or_else(|| s.placement.as_ref().map(|p| p.owner.clone())),
        placement: s.placement.clone(),
        busy: s.preparing.is_some(),
        error: s.error.clone(),
        on_place,
        on_move,
        on_commit,
        on_cancel,
    }
}

fn move_place(c: &PlaceCtx, m: MatrixPlacementMove) {
    let ok = c.inter.current() == Some(CanvasMode::PlacingMatrix)
        && m.center.x.is_finite()
        && m.center.y.is_finite()
        && c.live(&m.owner);
    if !ok {
        return;
    }
    let mut ui = c.ui;
    if let Some(a) = ui.write().placement.as_mut().filter(|p| p.owner == m.owner) {
        a.matrix.origin = m.center;
    }
}

fn start_place(c: &PlaceCtx, source: MatrixPlacementSource) {
    let mut ui = c.ui;
    if ui.read().preparing.is_some() || ui.read().placement.is_some() {
        return;
    }
    let (m, scope) = (c.rt.model(), c.rt.scope());
    let Some((snap, scope, board_id)) = matrix_source(&c.rt, &m, scope.as_ref(), Workspace::Layout)
        .filter(|(s, _, _)| {
            m.durability
                == (Durability::Saved {
                    revision: s.document.revision,
                })
        })
    else {
        ui.write().error =
            Some("Open a saved, editable board before placing a key assembly.".into());
        return;
    };
    let owner = MatrixPlacementOwner {
        scope_generation: (c.input.scope_generation)(),
        scope,
        board_id,
        snapshot_token: snap.token,
        revision: snap.document.revision,
    };
    if !c.inter.begin(CanvasMode::PlacingMatrix) {
        return;
    }
    {
        let mut s = ui.write();
        (s.preparing, s.error) = (Some(owner.clone()), None);
    }
    let (c, snap) = (c.clone(), snap.clone());
    let orient = if (c.input.assembly_orientation.0)() == SwitchOrientation::South {
        super::MatrixSwitchOrientation::South
    } else {
        super::MatrixSwitchOrientation::North
    };
    spawn_local(async move {
        let res = prepare_placement(&c.rt, &snap, &owner, source, orient, &c.alive).await;
        if !c.alive.get() || c.ui.read().preparing.as_ref() != Some(&owner) {
            return;
        }
        // A cancellation may already have ended the canvas mode; never re-arm placement.
        if c.inter.current() != Some(CanvasMode::PlacingMatrix) || !c.live(&owner) {
            let mut ui = c.ui;
            ui.write().preparing = None;
            c.inter.end(CanvasMode::PlacingMatrix);
            return;
        }
        let mut ui = c.ui;
        let mut s = ui.write();
        s.preparing = None;
        match res {
            Ok(Some((matrix, definitions, scene))) => {
                s.placement = Some(MatrixPlacementProjection {
                    owner,
                    matrix,
                    scene,
                    definitions,
                })
            }
            Ok(None) => c.inter.end(CanvasMode::PlacingMatrix),
            Err(msg) => {
                s.error = Some(msg);
                c.inter.end(CanvasMode::PlacingMatrix);
            }
        }
    });
}

fn commit_place(c: &PlaceCtx, m: MatrixPlacementMove) {
    let ok = c.inter.current() == Some(CanvasMode::PlacingMatrix)
        && m.center.x.is_finite()
        && m.center.y.is_finite()
        && c.live(&m.owner);
    if !ok {
        return;
    }
    let Some(mut act) = c.ui.read().placement.clone().filter(|p| p.owner == m.owner) else {
        return;
    };
    let model = c.rt.model();
    if model.accepted.is_none()
        || model.lifecycle != Lifecycle::Ready
        || model.durability
            != (Durability::Saved {
                revision: m.owner.revision,
            })
    {
        return;
    }
    let (mut sel, mut anchor) = (c.input.selected_context, c.input.anchor_scope);
    let (prev_ctx, prev_parts) = (sel.peek().clone(), model.selected_part_ids.clone());
    act.matrix.origin = m.center;
    let mid = act.matrix.id.clone();
    c.clear();
    let res = c.rt.commit(
        "layout-matrix-placement",
        place_matrix_resolver(act.matrix, act.definitions),
    );
    let vis = (c.input.workspace)() == Workspace::Layout
        && (c.input.scope_generation)() == m.owner.scope_generation
        && c.rt.model().selected_part_ids == prev_parts
        && sel.peek().as_ref() == prev_ctx.as_ref();
    let mut ui = c.ui;
    match res {
        Ok(snap) if vis && snap.document.matrices.iter().any(|x| x.id == mid) => {
            let context = TreeContext::Matrix { matrix_id: mid };
            match super::resolve_selection(&c.rt.model(), &context) {
                None => {
                    ui.write().error = Some(
                        "The matrix was saved, but its Layout selection could not be restored."
                            .into(),
                    )
                }
                Some(part_ids) => {
                    sel.set(Some(ScopedTreeContext {
                        scope: m.owner.scope,
                        context,
                    }));
                    anchor.set(None);
                    c.rt.submit(Event::SelectParts {
                        operation_id: c.rt.operation(),
                        part_ids,
                        range_part_ids: vec![],
                        mode: boardstudio_application::SelectionMode::Replace,
                    });
                }
            }
        }
        Err(msg) if vis => ui.write().error = Some(msg),
        _ => {}
    }
}

async fn prepare_placement(
    rt: &Runtime,
    acc: &AcceptedSnapshot,
    o: &MatrixPlacementOwner,
    src: MatrixPlacementSource,
    orient: super::MatrixSwitchOrientation,
    alive: &Cell<bool>,
) -> Result<Option<(Matrix, Vec<PartDefinition>, MatrixScene)>, String> {
    let rev = reversible_layout_param(&acc.document);
    let key = match &src {
        MatrixPlacementSource::Preset(p) => part_preset(*p).as_str().to_owned(),
        MatrixPlacementSource::Assembly { assembly, .. } => {
            format!("saved-assembly-{}", assembly.id)
        }
    };
    let mid = next_matrix_id(
        &acc.document.matrices,
        &acc.document.definitions,
        &acc.document.parts,
        &key,
        rev,
    );
    let (mut matrix, defs) = match src {
        MatrixPlacementSource::Preset(pid) => {
            let preset = part_preset(pid);
            let Some(p) = prepare_matrix_definitions(
                mid,
                o.board_id.clone(),
                MatrixSetupRequest {
                    rows: 1,
                    columns: 1,
                    preset,
                },
                rev,
                || alive.get(),
            )
            .await?
            else {
                return Ok(None);
            };
            let dir = if orient == super::MatrixSwitchOrientation::South {
                "south"
            } else {
                "north"
            };
            let var = format!(
                "preset/{}/{}{}",
                preset.as_str(),
                if rev { "reversible/" } else { "" },
                dir
            );
            (
                crate::presentation::objects::matrix_with_preset(
                    &p.matrix, &p.matrix, &var, orient,
                ),
                p.definitions,
            )
        }
        MatrixPlacementSource::Assembly {
            assembly,
            definitions: srcs,
        } => {
            let cell = MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: Some(0.0),
                assemblies: vec![],
                assemblies_local: Some(true),
            };
            let seed = Matrix {
                id: mid.clone(),
                name: Some(assembly.name.clone()),
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.05, y: 19.05 },
                origin: Vec2::default(),
                definition_id: String::new(),
                part_ids: vec![],
                board_id: Some(o.board_id.clone()),
                mirror: None,
                rotation: None,
                edge_gap: Some(Vec2 { x: 1.0, y: 1.0 }),
                diode_direction: None,
                row_layout: vec![],
                column_layout: vec![],
                anchor: None,
                cells: vec![cell],
            };
            let (m, mut defs) = matrix_with_assembly(
                &seed,
                &assembly,
                &srcs,
                &acc.document,
                &format!("placement-{mid}"),
            )?;
            for d in &mut defs {
                *d = normalize_matrix_definition(d.clone()).await?;
            }
            (m, defs)
        }
    };
    matrix.origin = Vec2::default();
    let mut scenes = rt.project_matrices(acc.document.revision, vec![matrix.clone()])?;
    if scenes.len() != 1 {
        return Err("Core returned an incomplete matrix placement preview.".into());
    }
    Ok(Some((matrix, defs, scenes.remove(0))))
}
