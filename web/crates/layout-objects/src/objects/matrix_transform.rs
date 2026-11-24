//! The matrix, row, column and key transform inspector: every field edit builds one
//! `EditOperation` against the accepted matrix and commits it synchronously.
pub use super::workspace_state::MatrixTransformInspectorOwner;
use super::{
    ScopedTreeContext, TreeContext,
    layout_rail::LayoutTransformRuntime,
    matrix_preset::{missing_definitions, switch_choices},
    mirror_links::matrix_is_linked,
};
use crate::matrix_relations::{DepartureAction, field_preview};
use crate::matrix_setup_operation::next_matrix_id;
use crate::matrix_transform_operation::{
    MatrixTransformField as Field, MatrixTransformFields, MatrixTransformValue as Value,
    attachment_component_choices, build_operation, column_angle, key_half_exception,
    key_half_exception_at_resolver, key_half_exception_omitted, set_cells_enabled,
};
use crate::presentation::parts::{load_all_catalogue_definitions, reversible_layout};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{
    AggregateMethod, AnchorPart, EditOperation, Follow, Matrix, MatrixAnchor, MatrixSplayAffect,
    Mirror, PartDefinition, Vec2, convert_offset_available,
};
use boardstudio_web_layout_core::{
    canonical_matrix_member, matrix_by_id, rail_drag::nudge_session_is_ready, scene_by_id,
    scene_cell_in,
};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, ToggleField, format_numeric,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTransformProjection {
    pub owner: MatrixTransformInspectorOwner,
    pub fields: MatrixTransformFields,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepartureControl {
    pub label: &'static str,
    pub disabled: bool,
    pub reason: Option<String>,
}

#[derive(Clone, PartialEq)]
pub struct MatrixTransformInspectorMount {
    pub runtime: LayoutTransformRuntime,
    pub selected: Option<ScopedTreeContext>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub aggregate_pick_draft: Signal<Option<(MatrixTransformInspectorOwner, Vec<AnchorPart>)>>,
    pub catalogue: Signal<Vec<PartDefinition>>,
}

pub fn use_workspace_matrix_transform(
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected: Signal<Option<ScopedTreeContext>>,
    splay_affect: Signal<MatrixSplayAffect>,
) -> MatrixTransformInspectorMount {
    let catalogue = use_signal(Vec::<PartDefinition>::new);
    let rev = runtime
        .model()
        .accepted
        .as_ref()
        .map(|s| reversible_layout(&s.document));
    use_effect(use_reactive((&rev,), move |(rev,)| {
        let Some(rev) = rev else { return };
        let mut catalogue = catalogue;
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(defs) = load_all_catalogue_definitions(rev).await {
                let _ = catalogue.try_write().map(|mut slot| *slot = defs);
            }
        });
    }));
    MatrixTransformInspectorMount {
        runtime: LayoutTransformRuntime(runtime),
        selected: selected.read().clone(),
        splay_affect,
        aggregate_pick_draft: use_signal(|| None),
        catalogue,
    }
}

impl MatrixTransformInspectorMount {
    pub(super) fn parts(&self) -> Option<(String, TreeContext, MatrixTransformFields)> {
        let selected = self.selected.as_ref()?;
        let scope = self.runtime.0.scope().filter(|s| s == &selected.scope)?;
        let snapshot = self.runtime.0.model().accepted?;
        let matrix_id = match &selected.context {
            TreeContext::Matrix { matrix_id }
            | TreeContext::Row { matrix_id, .. }
            | TreeContext::Column { matrix_id, .. }
            | TreeContext::Key { matrix_id, .. } => matrix_id,
            _ => return None,
        };
        let matrix = matrix_by_id(&snapshot.document, matrix_id)?;
        let fields = transform_fields(
            &snapshot,
            &scope.board_id,
            &selected.context,
            matrix,
            &self.catalogue.read(),
        )?;
        Some((matrix_id.clone(), selected.context.clone(), fields))
    }

    pub fn projection(&self) -> Option<MatrixTransformProjection> {
        let (matrix_id, context, fields) = self.parts()?;
        let scope = self.selected.as_ref()?.scope.clone();
        Some(MatrixTransformProjection {
            owner: MatrixTransformInspectorOwner {
                scope,
                matrix_id,
                context,
            },
            fields,
        })
    }

    pub fn is_active(&self) -> bool {
        self.parts().is_some()
    }

    pub fn editable(&self) -> bool {
        nudge_session_is_ready(&self.runtime.0.model())
    }

    pub fn pick_matrix_anchor(&self, matrix_id: String, row: u32, column: u32) -> bool {
        let ok = matches!(
            self.parts(),
            Some((_, _, MatrixTransformFields::Matrix { .. }))
        );
        let val = Value::AnchorKey {
            matrix_id,
            row,
            column,
        };
        ok && self.edit_reported(Field::MatrixAnchorPick, val)
    }

    pub fn pick_aggregate_part(&self, matrix_id: String, row: u32, column: u32) -> bool {
        let Some(proj) = self.projection() else {
            return false;
        };
        let MatrixTransformFields::Matrix {
            anchor: Some(a), ..
        } = &proj.fields
        else {
            return false;
        };
        if a.aggregate.is_none() {
            return false;
        }
        let part = AnchorPart {
            matrix_id,
            row,
            column,
        };
        let mut draft = self.aggregate_pick_draft;
        let mut parts = draft
            .read()
            .clone()
            .filter(|(o, _)| *o == proj.owner)
            .map(|(_, p)| p)
            .unwrap_or_default();
        if !parts.contains(&part) {
            parts.push(part);
        }
        draft.set(Some((proj.owner, parts)));
        true
    }

    pub fn clear_aggregate_pick(&self) {
        let mut draft = self.aggregate_pick_draft;
        draft.set(None);
    }

    pub fn pick_splay_origin(&self, point: Vec2, swing: bool) -> bool {
        let f = if swing {
            Field::SplayOriginSwingPoint
        } else {
            Field::SplayOriginPoint
        };
        let ok = matches!(
            self.parts(),
            Some((_, _, MatrixTransformFields::Column { .. }))
        );
        ok && point.x.is_finite()
            && point.y.is_finite()
            && self.edit_reported(f, Value::Point(point))
    }

    fn edit_reported(&self, field: Field, value: Value) -> bool {
        self.edit(field, value)
            .map_err(|msg| self.runtime.0.report(msg))
            .is_ok()
    }

    fn departure_plan(
        &self,
        action: DepartureAction,
    ) -> Option<(Field, Value, &'static str, Option<String>)> {
        use DepartureAction as A;
        use MatrixTransformFields as F;
        let z = Vec2::default();
        Some(match (self.parts()?.2, action) {
            (F::Row { .. }, A::Reset) => (
                Field::RowOffsetReset,
                Value::Offset(z),
                "Reset offsets",
                None,
            ),
            (F::Column { .. }, A::Reset) => (
                Field::ColumnOffsetReset,
                Value::Offset(z),
                "Reset offsets",
                None,
            ),
            (F::Column { convert, .. }, A::Convert) => (
                Field::ColumnOffsetConvert,
                Value::Number(0.0),
                "Convert to stagger",
                convert.err(),
            ),
            (F::Key { .. }, A::Reset) => (
                Field::KeyTransformReset,
                Value::CellTransform {
                    offset: z,
                    rotation: 0.0,
                },
                "Reset local transform",
                None,
            ),
            _ => return None,
        })
    }

    pub fn departure_control(&self, action: DepartureAction) -> Option<DepartureControl> {
        let (_, _, label, blocked) = self.departure_plan(action)?;
        Some(DepartureControl {
            label,
            disabled: !self.editable() || blocked.is_some(),
            reason: blocked,
        })
    }

    pub fn run_departure(&self, action: DepartureAction) -> Option<Result<(), String>> {
        let (field, value, _, blocked) = self.departure_plan(action)?;
        (blocked.is_none() && self.editable()).then(|| self.edit(field, value))
    }

    pub fn edit(&self, field: Field, value: Value) -> Result<(), String> {
        let runtime = &self.runtime.0;
        let model = runtime.model();
        if !nudge_session_is_ready(&model) {
            return Err("The matrix transform is not ready to edit. Wait for the current operation to finish, then retry.".into());
        }
        let (matrix_id, context, _) = self.parts().ok_or("The selection is no longer editable.")?;
        let board_id = runtime.scope().map(|s| s.board_id).unwrap_or_default();
        let snapshot = model.accepted.as_ref().ok_or("No accepted keyboard.")?;
        let cells = selected_cells(snapshot, &matrix_id, &model.selected_part_ids, &context);
        let affect = (self.splay_affect)();
        let cat = self.catalogue.read().clone();
        let resolver = match (field, &value, &context) {
            (Field::DeleteSelection, _, TreeContext::Key { .. }) => resolver(move |_| {
                let op = EditOperation::RemoveMatrixCells {
                    matrix_id: matrix_id.clone(),
                    cells: cells.clone(),
                };
                Resolution::submit(vec![matrix_id.clone()], op)
            }),
            (Field::KeyOmit, Value::Bool(omit), TreeContext::Key { row, column, .. }) => {
                key_half_exception_at_resolver(matrix_id, *row, *column, *omit)
            }
            _ => edit_resolver(
                matrix_id, board_id, context, cells, field, value, affect, cat,
            ),
        };
        runtime
            .commit("layout-matrix-transform", resolver)
            .map(|_| ())
    }
}

fn resolver(f: impl Fn(&AcceptedSnapshot) -> Resolution + 'static) -> EditResolver {
    EditResolver::new("layout-matrix-transform", f)
}

#[allow(clippy::too_many_arguments)]
fn edit_resolver(
    matrix_id: String,
    board_id: String,
    context: TreeContext,
    cells: Vec<(u32, u32)>,
    field: Field,
    value: Value,
    affect: MatrixSplayAffect,
    catalogue: Vec<PartDefinition>,
) -> EditResolver {
    resolver(move |accepted| {
        let doc = &accepted.document;
        let Some(matrix) = matrix_by_id(doc, &matrix_id) else {
            return Resolution::Retire("The selected matrix no longer exists.".into());
        };
        if let (Field::KeyEnabled, Value::Bool(en), TreeContext::Key { .. }) =
            (field, &value, &context)
        {
            if scene_by_id(&accepted.scene.matrix_scenes, &matrix_id).is_none()
                || cells
                    .iter()
                    .any(|&(r, c)| r >= matrix.rows || c >= matrix.columns)
            {
                return Resolution::Retire("A selected key no longer exists.".into());
            }
            return match set_cells_enabled(matrix, &cells, *en) {
                EditOperation::SetMatrix { matrix: n, .. } if n == *matrix => Resolution::Unchanged,
                op => Resolution::submit(vec![matrix.id.clone()], op),
            };
        }
        if let (Field::KeyAddAnchoredMatrix, TreeContext::Key { row, column, .. }) =
            (field, &context)
        {
            if *row >= matrix.rows || *column >= matrix.columns {
                return Resolution::Retire("The selected key no longer exists.".into());
            }
            let id = next_matrix_id(
                &doc.matrices,
                &doc.definitions,
                &doc.parts,
                "anchored",
                reversible_layout(doc),
            );
            let anchor = MatrixAnchor {
                matrix_id: matrix_id.clone(),
                row: *row,
                column: *column,
                follow: Follow::default(),
                aggregate: None,
            };
            let op = EditOperation::AddAnchoredMatrix {
                id: id.clone(),
                anchor,
            };
            return Resolution::submit(vec![matrix_id.clone(), id], op);
        }
        let Some(fields) = transform_fields(accepted, &board_id, &context, matrix, &[]) else {
            return Resolution::Retire("The selected row, column or key no longer exists.".into());
        };
        let op = match build_operation(matrix, &fields, field, value.clone(), affect.clone()) {
            Ok(op) => op,
            Err(msg) => return Resolution::Retire(msg),
        };
        let wanted: Vec<&str> = match &value {
            Value::Text(id) if field == Field::KeyAssembly => vec![id.as_str()],
            Value::Attached(list) if field == Field::KeyAttached => {
                list.iter().map(|(_, id)| id.as_str()).collect()
            }
            _ => Vec::new(),
        };
        let missing = missing_definitions(doc, &catalogue, &wanted);
        if wanted
            .iter()
            .any(|id| !doc.definitions.iter().chain(&missing).any(|d| d.id == *id))
        {
            return Resolution::Retire("The component catalogue is not ready yet. Try this change again when it finishes loading.".into());
        }
        let op = match op {
            EditOperation::SetMatrix { matrix, .. } if !missing.is_empty() => {
                EditOperation::SetMatrix {
                    matrix,
                    definitions: Some(missing),
                }
            }
            other => other,
        };
        Resolution::submit(vec![matrix.id.clone()], op)
    })
}

fn selected_cells(
    snapshot: &AcceptedSnapshot,
    matrix_id: &str,
    ids: &[String],
    context: &TreeContext,
) -> Vec<(u32, u32)> {
    let mut cells: Vec<_> = scene_by_id(&snapshot.scene.matrix_scenes, matrix_id)
        .into_iter()
        .flat_map(|s| &s.cells)
        .filter(|c| {
            ids.iter().any(|id| {
                c.member_id.as_ref() == Some(id)
                    || *id == canonical_matrix_member(matrix_id, c.row, c.column)
            })
        })
        .map(|c| (c.row, c.column))
        .collect();
    if let TreeContext::Key { row, column, .. } = context
        && !cells.contains(&(*row, *column))
    {
        cells.push((*row, *column));
    }
    cells
}

/// The field projection of the selected matrix, row, column or key.
pub(super) fn transform_fields(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    context: &TreeContext,
    matrix: &Matrix,
    catalogue: &[PartDefinition],
) -> Option<MatrixTransformFields> {
    let (z, doc) = (Vec2::default(), &snapshot.document);
    let scene = || scene_by_id(&snapshot.scene.matrix_scenes, &matrix.id);
    use MatrixTransformFields as F;
    Some(match context {
        TreeContext::Matrix { .. } => F::Matrix {
            origin: matrix.origin,
            rotation: matrix.rotation.unwrap_or(0.0),
            mirror: matrix.mirror,
            mirror_y_locked: matrix_is_linked(&doc.layouts, board_id, &matrix.id),
            rotation_followed: matrix.anchor.as_ref().is_none_or(|a| a.follow.rotation),
            anchor: matrix.anchor.clone(),
        },
        TreeContext::Row { row, .. } => {
            let r = matrix.row_layout.get(*row as usize);
            F::Row {
                row: *row,
                offset: r.map_or(z, |r| r.offset),
                shift: r.map_or(z, |r| r.shift),
                bend: r.map_or(0.0, |r| r.bend),
                padding: r.and_then(|r| r.padding),
                pitch: matrix.pitch.y,
            }
        }
        TreeContext::Column { column, .. } => {
            let b = scene()?.columns.iter().find(|b| b.column == *column)?;
            let r = matrix.column_layout.get(*column as usize);
            F::Column {
                column: *column,
                offset: r.map_or(z, |r| r.offset),
                stagger: r.map_or(0.0, |r| r.stagger),
                spread: r.and_then(|r| r.spread),
                pitch: matrix.pitch.x,
                splay_angle: b.splay_angle,
                rotation: b.rotation,
                splay_origin: b.splay_origin,
                custom_origin: b.custom_origin,
                convert: convert_offset_available(matrix, *column),
            }
        }
        TreeContext::Key { row, column, .. } => {
            let member = scene_cell_in(scene()?, *row, *column)
                .and_then(|c| c.member_id.clone())
                .or_else(|| {
                    key_half_exception_omitted(doc, &matrix.id, *row, *column).map(|e| e.mirror_id)
                });
            let cell = matrix
                .cells
                .iter()
                .find(|c| c.row == *row && c.column == *column);
            let def_id = cell
                .and_then(|c| c.definition_id.clone())
                .unwrap_or_else(|| matrix.definition_id.clone());
            let mut seen = HashSet::new();
            let component_choices: Vec<_> = doc
                .definitions
                .iter()
                .chain(catalogue)
                .filter(|d| seen.insert(d.id.clone()))
                .cloned()
                .collect();
            let assemblies = cell
                .map(|c| {
                    c.assemblies
                        .iter()
                        .map(|a| (a.id.clone(), a.definition_id.clone()))
                        .collect()
                })
                .unwrap_or_default();
            F::Key {
                row: *row,
                column: *column,
                enabled: cell.is_none_or(|c| c.enabled),
                choices: switch_choices(doc, catalogue, &def_id),
                definition_id: def_id,
                assemblies,
                component_choices,
                mirror_target: doc
                    .layouts
                    .iter()
                    .any(|l| l.matrix_ids.contains(&matrix.id) && l.mirror_link.is_some()),
                assemblies_local: cell.and_then(|c| c.assemblies_local).unwrap_or(false),
                offset: cell.and_then(|c| c.offset).unwrap_or(z),
                rotation: cell.and_then(|c| c.rotation).unwrap_or(0.0),
                shift: cell.and_then(|c| c.shift).unwrap_or(z),
                bend: cell.and_then(|c| c.bend).unwrap_or(0.0),
                half_exception: member.as_deref().and_then(|m| key_half_exception(doc, m)),
            }
        }
        _ => return None,
    })
}

pub(super) fn number(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &'static str,
    unit: &'static str,
    value: f64,
    error: Signal<Option<String>>,
) -> Element {
    rsx! { TransformNumber { key: "{field:?}", mount: mount.clone(), field, label, unit, value, error } }
}

#[component]
fn TransformNumber(
    mount: MatrixTransformInspectorMount,
    field: Field,
    label: &'static str,
    unit: &'static str,
    value: f64,
    mut error: Signal<Option<String>>,
) -> Element {
    let preview = try_use_context::<FieldPreview>();
    let source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let shown = use_signal(Vec::<String>::new);
    let show = {
        let mount = mount.clone();
        move || {
            let mut shown = shown;
            let (Some(p), Some(s), Some((_, ctx, _))) = (preview, &source, mount.parts()) else {
                return;
            };
            if let Some(f) = field_preview(
                &s.document,
                &s.scene.matrix_scenes,
                &ctx,
                field,
                (mount.splay_affect)(),
            ) {
                shown.set(f.parts.clone());
                p.show(FieldPreviewState {
                    parts: f.parts,
                    hint: f.hint,
                });
            }
        }
    };
    let release = move || {
        if let Some(p) = preview {
            p.release(&shown.peek());
        }
    };
    use_drop(release);
    let displayed = format_numeric(value, 4).parse::<f64>().unwrap_or(value);
    rsx! {
        div { "data-transform-number": "true",
            onpointerenter: { let show = show.clone(); move |_| show() }, onpointerleave: move |_| release(),
            onfocusin: move |_| show(), onfocusout: move |_| release(),
            NumericField { label, unit, value: displayed, precision: 4, on_commit: move |next| error.set(mount.edit(field, Value::Number(next)).err()) }
        }
    }
}

fn action(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &str,
    value: Value,
    mut error: Signal<Option<String>>,
) -> Element {
    let mount = mount.clone();
    rsx! { Button { button_type: "button", variant: ButtonVariant::Secondary, onclick: move |_| error.set(mount.edit(field, value.clone()).err()), "{label}" } }
}

fn toggle(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &str,
    checked: bool,
    mut error: Signal<Option<String>>,
) -> Element {
    let mount = mount.clone();
    rsx! { ToggleField { key: "{field:?}", label: label.to_owned(), checked, on_commit: move |next| error.set(mount.edit(field, Value::Bool(next)).err()) } }
}

#[component]
pub fn SelectionDeleteControl(mount: MatrixTransformInspectorMount) -> Element {
    let error = use_signal(|| None::<String>);
    let Some((_, _, fields)) = mount.parts() else {
        return rsx! {};
    };
    let label = match fields {
        MatrixTransformFields::Matrix { .. } => "Delete matrix",
        MatrixTransformFields::Row { .. } => "Delete row",
        MatrixTransformFields::Column { .. } => "Delete column",
        MatrixTransformFields::Key { .. } => "Delete selected keys",
    };
    rsx! { div { onpointerdown: move |e| e.stop_propagation(), onkeydown: move |e| e.stop_propagation(), {action(&mount, Field::DeleteSelection, label, Value::Bool(true), error)} } }
}

#[component]
pub fn MatrixTransformInspector(
    mount: MatrixTransformInspectorMount,
    on_pick_splay_origin: EventHandler<()>,
    on_pick_anchor: EventHandler<()>,
    anchor_pick_pending: bool,
    on_cancel_anchor_pick: EventHandler<()>,
    on_navigate_anchor_part: EventHandler<TreeContext>,
) -> Element {
    let _ = use_context::<Signal<u64>>()();
    let _ = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>().map(|d| d.0());
    let mut error = use_signal(|| None::<String>);
    let Some(projection) = mount.projection() else {
        return rsx! {};
    };
    let mm = |field, label, value| number(&mount, field, label, "mm", value, error);
    let deg = |field, label, value| number(&mount, field, label, "°", value, error);
    let act = |field, label: &str, value| action(&mount, field, label, value, error);
    use MatrixTransformFields as F;
    let content = match &projection.fields {
        F::Matrix {
            origin,
            rotation,
            mirror,
            mirror_y_locked,
            anchor,
            ..
        } => {
            let mut options = vec![(None, "None".to_owned()), (Some(Mirror::X), "X".to_owned())];
            if !mirror_y_locked || *mirror == Some(Mirror::Y) {
                options.push((Some(Mirror::Y), "Y".to_owned()));
            }
            let edit = mount.clone();
            rsx! {
                InspectorSection { title: "Position & orientation",
                    {mm(Field::OriginX, "Origin X", origin.x)} {mm(Field::OriginY, "Origin Y", origin.y)}
                    {deg(Field::MatrixRotation, "Rotation", *rotation)}
                    SelectField::<Option<Mirror>> { label: "Mirror", value: *mirror, options, on_commit: move |n| error.set(edit.edit(Field::MatrixMirror, Value::Mirror(n)).err()) }
                }
                {anchor_controls(&mount, &projection.owner, anchor.as_ref(), error, anchor_pick_pending, on_pick_anchor, on_cancel_anchor_pick, on_navigate_anchor_part)}
            }
        }
        F::Row {
            offset,
            shift,
            bend,
            padding,
            pitch,
            ..
        } => {
            rsx! {
                InspectorSection { title: "Row transform",
                    {mm(Field::RowShiftX, "Shift X", shift.x)} {mm(Field::RowShiftY, "Shift Y", shift.y)}
                    {deg(Field::RowBend, "Bend", *bend)} {mm(Field::RowPadding, "Padding", padding.unwrap_or(*pitch))}
                    {mm(Field::RowOffsetX, "Offset X", offset.x)} {mm(Field::RowOffsetY, "Offset Y", offset.y)}
                    {act(Field::RowOffsetReset, "Reset offsets", Value::Offset(Vec2::default()))}
                }
            }
        }
        F::Column {
            offset,
            stagger,
            spread,
            pitch,
            splay_angle,
            rotation,
            splay_origin,
            custom_origin,
            convert,
            ..
        } => {
            let (mut affect, edit) = (mount.splay_affect, mount.clone());
            rsx! {
                InspectorSection { title: "Column transform",
                    {mm(Field::ColumnSpread, "Spread", spread.unwrap_or(*pitch))} {mm(Field::ColumnStagger, "Stagger", *stagger)}
                    {mm(Field::ColumnOffsetX, "Offset X", offset.x)} {mm(Field::ColumnOffsetY, "Offset Y", offset.y)}
                    {act(Field::ColumnOffsetReset, "Reset offsets", Value::Offset(Vec2::default()))}
                    if convert.is_ok() { {act(Field::ColumnOffsetConvert, "Convert offset to stagger", Value::Number(0.0))} }
                    SelectField::<MatrixSplayAffect> {
                        label: "Splay affects", value: affect(),
                        options: vec![(MatrixSplayAffect::Column, "This column".to_owned()), (MatrixSplayAffect::Following, "Following columns".to_owned())],
                        on_commit: move |v| affect.set(v),
                    }
                    {deg(Field::ColumnSplay, "Splay", column_angle(*splay_angle, *rotation, &affect()))}
                }
                InspectorSection { title: "Splay origin", default_open: false,
                    ToggleField { label: "Custom origin", checked: *custom_origin, on_commit: move |n| error.set(edit.edit(Field::SplayOriginMode, Value::OriginMode(n)).err()) }
                    if *custom_origin {
                        {mm(Field::SplayOriginX, "Origin X", splay_origin.x)} {mm(Field::SplayOriginY, "Origin Y", splay_origin.y)}
                        Button { variant: ButtonVariant::Secondary, onclick: move |_| on_pick_splay_origin.call(()), "Pick origin" }
                    }
                }
            }
        }
        F::Key {
            enabled,
            definition_id,
            choices,
            offset,
            rotation,
            shift,
            bend,
            half_exception,
            mirror_target,
            assemblies_local,
            assemblies,
            component_choices,
            ..
        } => {
            let edit = mount.clone();
            rsx! {
                InspectorSection { title: "Key transform",
                    {toggle(&mount, Field::KeyEnabled, "Enabled", *enabled, error)}
                    if let Some(ex) = half_exception { {toggle(&mount, Field::KeyOmit, "Omit mirrored key", ex.omitted, error)} }
                    {mm(Field::KeyShiftX, "Shift X", shift.x)} {mm(Field::KeyShiftY, "Shift Y", shift.y)}
                    {deg(Field::KeyBend, "Bend", *bend)} {mm(Field::KeyOffsetX, "Local X", offset.x)}
                    {mm(Field::KeyOffsetY, "Local Y", offset.y)} {deg(Field::KeyRotation, "Key rotation", *rotation)}
                    {act(Field::KeyTransformReset, "Reset local transform", Value::CellTransform { offset: Vec2::default(), rotation: 0.0 })}
                }
                InspectorSection { title: "Key assembly",
                    SelectField::<String> { label: "Key assembly", value: definition_id.clone(), options: choices.clone(), on_commit: move |n| error.set(edit.edit(Field::KeyAssembly, Value::Text(n)).err()) }
                    if *mirror_target && *assemblies_local { {act(Field::KeyAssembliesLocal, "Use mirrored components", Value::Bool(false))} }
                    for (index, (id, def)) in assemblies.iter().enumerate() {
                        div { key: "{id}",
                            SelectField::<String> {
                                label: format!("Replace {id}"), value: def.to_owned(),
                                options: attachment_component_choices(component_choices, def),
                                on_commit: { let (m, mut next) = (mount.clone(), assemblies.to_vec()); move |v| { next[index].1 = v; error.set(m.edit(Field::KeyAttached, Value::Attached(next.clone())).err()); } },
                            }
                            Button { variant: ButtonVariant::Secondary, onclick: { let (m, mut rem) = (mount.clone(), assemblies.to_vec()); rem.remove(index); move |_| error.set(m.edit(Field::KeyAttached, Value::Attached(rem.clone())).err()) }, "Remove {id}" }
                        }
                    }
                }
                {act(Field::KeyAddAnchoredMatrix, "Add anchored matrix", Value::Text(String::new()))}
            }
        }
    };
    rsx! {
        section { class: "grid min-w-0 gap-3 px-4 py-3", aria_label: "Matrix transform properties", onkeydown: super::stop_space_pan,
            {content}
            SelectionDeleteControl { mount: mount.clone() }
            if let Some(message) = error() { p { role: "alert", class: "text-destructive", "{message}" } }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn anchor_controls(
    mount: &MatrixTransformInspectorMount,
    owner: &MatrixTransformInspectorOwner,
    anchor: Option<&MatrixAnchor>,
    mut error: Signal<Option<String>>,
    pending: bool,
    on_pick: EventHandler<()>,
    on_cancel: EventHandler<()>,
    on_navigate: EventHandler<TreeContext>,
) -> Element {
    let aggregate = anchor.and_then(|a| a.aggregate.as_ref());
    let mut picked = (mount.aggregate_pick_draft)()
        .filter(|(o, _)| o == owner)
        .map_or_else(Vec::new, |(_, p)| p);
    picked.retain(|p| aggregate.is_none_or(|a| !a.parts.contains(p)));
    let partner = error().and_then(|msg| {
        let prefix = format!(
            "Detaching {} would move its linked partner ",
            owner.matrix_id
        );
        let (p, tail) = msg.split_once(&prefix)?.1.split_once(';')?;
        let need = format!("make {p} Local to keep its pose and confirm the detach");
        (tail.trim_start() == need).then(|| p.to_owned())
    });
    rsx! {
        InspectorSection { title: "Anchor",
            if let Some(anchor) = anchor {
                if let Some(aggregate) = aggregate {
                    SelectField::<AggregateMethod> {
                        label: "Method", value: aggregate.method,
                        options: vec![(AggregateMethod::Average, "Average".to_owned()), (AggregateMethod::Intersect, "Intersection".to_owned())],
                        on_commit: { let m = mount.clone(); move |n| error.set(m.edit(Field::MatrixAnchorMethod, Value::AnchorMethod(n)).err()) },
                    }
                    for (index, part) in aggregate.parts.iter().enumerate() {
                        div { key: "{part.matrix_id}-{part.row}-{part.column}",
                            Button { variant: ButtonVariant::Secondary,
                                onclick: { let tree = TreeContext::Key { matrix_id: part.matrix_id.clone(), row: part.row, column: part.column }; move |_| on_navigate.call(tree.clone()) },
                                "{part.matrix_id} · row {part.row + 1} · column {part.column + 1}"
                            }
                            Button { variant: ButtonVariant::Secondary, disabled: aggregate.parts.len() <= 1,
                                onclick: { let (m, mut rem) = (mount.clone(), aggregate.parts.clone()); rem.remove(index); move |_| error.set(m.edit(Field::MatrixAnchorParts, Value::AnchorParts(rem.clone())).err()) },
                                "Remove"
                            }
                        }
                    }
                    if pending && !picked.is_empty() {
                        Button { variant: ButtonVariant::Secondary,
                            onclick: { let (m, mut next) = (mount.clone(), aggregate.parts.clone()); next.extend(picked.clone()); move |_| { error.set(m.edit(Field::MatrixAnchorParts, Value::AnchorParts(next.clone())).err()); m.clear_aggregate_pick(); on_cancel.call(()); } },
                            "Add {picked.len()} picked parts"
                        }
                    }
                } else {
                    p { "data-anchor-target": "true", "Anchored to {anchor.matrix_id} · row {anchor.row} · column {anchor.column}" }
                }
                {toggle(mount, Field::MatrixFollowX, "Follow X", anchor.follow.x, error)}
                {toggle(mount, Field::MatrixFollowY, "Follow Y", anchor.follow.y, error)}
                {toggle(mount, Field::MatrixFollowRotation, "Follow rotation", anchor.follow.rotation, error)}
                if let Some(partner) = partner { {action(mount, Field::MatrixDetachConfirm, "Confirm detach", Value::Text(partner), error)} }
                else { {action(mount, Field::MatrixDetach, "Detach", Value::Bool(true), error)} }
            }
            if pending {
                Button { variant: ButtonVariant::Secondary, onclick: { let m = mount.clone(); move |_| { m.clear_aggregate_pick(); on_cancel.call(()); } }, "Cancel anchor pick" }
            } else {
                Button { variant: ButtonVariant::Secondary, onclick: move |_| on_pick.call(()), if aggregate.is_some() { "Pick parts" } else { "Pick anchor" } }
            }
        }
    }
}
