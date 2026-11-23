//! The matrix, row, column and key transform inspector: every field edit builds one
//! `EditOperation` against the accepted matrix and commits it synchronously.
use super::layout_rail::LayoutTransformRuntime;
pub use super::workspace_state::MatrixTransformInspectorOwner;
use super::{ScopedTreeContext, TreeContext};
use crate::matrix_relations::DepartureAction;
use crate::matrix_transform_operation::{
    MatrixTransformField as Field, MatrixTransformFields, MatrixTransformValue as Value,
    attachment_component_choices, build_operation, column_angle, key_half_exception,
    key_half_exception_at_resolver, key_half_exception_omitted,
};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{
    AggregateMethod, AnchorPart, EditOperation, Matrix, MatrixAnchor, MatrixSplayAffect, Mirror,
    PartDefinition, ProjectDoc, Vec2, convert_offset_available,
};
use boardstudio_web_layout_core::rail_drag::nudge_session_is_ready;
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, ToggleField, format_numeric,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTransformProjection {
    pub owner: MatrixTransformInspectorOwner,
    pub fields: MatrixTransformFields,
}

/// Whether a departure (a reset or convert listed under Relations) can run now, and why not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DepartureControl {
    pub label: &'static str,
    pub disabled: bool,
    pub reason: Option<String>,
}

/// What the transform inspector and the canvas controls need: the runtime, the selection they
/// edit, the splay scope the user chose and the canvas multi-pick draft.
#[derive(Clone, PartialEq)]
pub struct MatrixTransformInspectorMount {
    pub runtime: LayoutTransformRuntime,
    pub selected: Option<ScopedTreeContext>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    /// The pending canvas multi-Pick draft, owner-tagged so a selection change orphans it.
    pub aggregate_pick_draft: Signal<Option<(MatrixTransformInspectorOwner, Vec<AnchorPart>)>>,
    /// The bundled catalogue's definitions, loaded once per document reversibility, so a key can
    /// pick an assembly or attach a component the document does not hold yet.
    pub catalogue: Signal<Vec<PartDefinition>>,
}

pub fn use_workspace_matrix_transform(
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected: Signal<Option<ScopedTreeContext>>,
    splay_affect: Signal<MatrixSplayAffect>,
) -> MatrixTransformInspectorMount {
    let catalogue = use_signal(Vec::<PartDefinition>::new);
    let reversible = runtime
        .model()
        .accepted
        .as_ref()
        .map(|snapshot| crate::presentation::parts::reversible_layout(&snapshot.document));
    use_effect(use_reactive((&reversible,), move |(reversible,)| {
        let Some(reversible) = reversible else { return };
        let mut catalogue = catalogue;
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(definitions) =
                crate::presentation::parts::load_all_catalogue_definitions(reversible).await
            {
                let _ = catalogue.try_write().map(|mut slot| *slot = definitions);
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
    /// The selected matrix id and its field projection, while the selection is live.
    pub(super) fn parts(&self) -> Option<(String, TreeContext, MatrixTransformFields)> {
        let selected = self.selected.as_ref()?;
        let scope = self
            .runtime
            .0
            .scope()
            .filter(|scope| scope == &selected.scope)?;
        let snapshot = self.runtime.0.model().accepted?;
        let matrix_id = match &selected.context {
            TreeContext::Matrix { matrix_id }
            | TreeContext::Row { matrix_id, .. }
            | TreeContext::Column { matrix_id, .. }
            | TreeContext::Key { matrix_id, .. } => matrix_id,
            _ => return None,
        };
        let matrix = snapshot
            .document
            .matrices
            .iter()
            .find(|matrix| matrix.id == *matrix_id)?;
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

    /// Transform edits queue behind each other, so the inspector stays editable while applying.
    pub fn editable(&self) -> bool {
        nudge_session_is_ready(&self.runtime.0.model())
    }

    /// A canvas pick of the anchor key retargets the anchor, keeping the Follow subset. Core
    /// validates the target (same Layout, no cycle, not constraint-driven).
    pub fn pick_matrix_anchor(&self, matrix_id: String, row: u32, column: u32) -> bool {
        let Some(MatrixTransformFields::Matrix { .. }) = self.parts().map(|(_, _, fields)| fields)
        else {
            return false;
        };
        self.edit_reported(
            Field::MatrixAnchorPick,
            Value::AnchorKey {
                matrix_id,
                row,
                column,
            },
        )
    }

    /// A canvas multi-Pick of one aggregate part appends to the pending draft without writing.
    pub fn pick_aggregate_part(&self, matrix_id: String, row: u32, column: u32) -> bool {
        let Some(projection) = self.projection() else {
            return false;
        };
        let MatrixTransformFields::Matrix {
            anchor: Some(anchor),
            ..
        } = &projection.fields
        else {
            return false;
        };
        if anchor.aggregate.is_none() {
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
            .filter(|(owner, _)| *owner == projection.owner)
            .map(|(_, parts)| parts)
            .unwrap_or_default();
        if !parts.contains(&part) {
            parts.push(part);
        }
        draft.set(Some((projection.owner, parts)));
        true
    }

    pub fn clear_aggregate_pick(&self) {
        let mut draft = self.aggregate_pick_draft;
        draft.set(None);
    }

    /// A canvas pick of the splay origin; with Ctrl held it swings the keys instead of holding them.
    pub fn pick_splay_origin(&self, point: Vec2, swing: bool) -> bool {
        let Some(MatrixTransformFields::Column { .. }) = self.parts().map(|(_, _, fields)| fields)
        else {
            return false;
        };
        point.x.is_finite()
            && point.y.is_finite()
            && self.edit_reported(
                if swing {
                    Field::SplayOriginSwingPoint
                } else {
                    Field::SplayOriginPoint
                },
                Value::Point(point),
            )
    }

    fn edit_reported(&self, field: Field, value: Value) -> bool {
        match self.edit(field, value) {
            Ok(()) => true,
            Err(message) => {
                self.runtime.0.report(message);
                false
            }
        }
    }

    fn departure_plan(&self, action: DepartureAction) -> Option<(Field, Value, Option<String>)> {
        let zero = Vec2 { x: 0.0, y: 0.0 };
        Some(match (self.parts()?.2, action) {
            (MatrixTransformFields::Row { .. }, DepartureAction::Reset) => {
                (Field::RowOffsetReset, Value::Offset(zero), None)
            }
            (MatrixTransformFields::Column { .. }, DepartureAction::Reset) => {
                (Field::ColumnOffsetReset, Value::Offset(zero), None)
            }
            (MatrixTransformFields::Column { convert, .. }, DepartureAction::Convert) => (
                Field::ColumnOffsetConvert,
                Value::Number(0.0),
                convert.err(),
            ),
            (MatrixTransformFields::Key { .. }, DepartureAction::Reset) => (
                Field::KeyTransformReset,
                Value::CellTransform {
                    offset: zero,
                    rotation: 0.0,
                },
                None,
            ),
            _ => return None,
        })
    }

    /// The state of `action` for the selected row, column or key; `None` when it has no such action.
    pub fn departure_control(&self, action: DepartureAction) -> Option<DepartureControl> {
        let (field, _, blocked) = self.departure_plan(action)?;
        let label = match field {
            Field::ColumnOffsetConvert => "Convert to stagger",
            Field::KeyTransformReset => "Reset local transform",
            _ => "Reset offsets",
        };
        Some(DepartureControl {
            label,
            disabled: !self.editable() || blocked.is_some(),
            reason: blocked,
        })
    }

    /// Run `action` through the Inspector's edit path; the error is why it did not land.
    pub fn run_departure(&self, action: DepartureAction) -> Option<Result<(), String>> {
        let (field, value, blocked) = self.departure_plan(action)?;
        (blocked.is_none() && self.editable()).then(|| self.edit(field, value))
    }

    /// Commit one field edit; the error is the reason it did not land.
    pub fn edit(&self, field: Field, value: Value) -> Result<(), String> {
        let runtime = &self.runtime.0;
        let model = runtime.model();
        if !nudge_session_is_ready(&model) {
            return Err("The matrix transform is not ready to edit. Wait for the current operation to finish, then retry.".into());
        }
        let (matrix_id, context, _) = self.parts().ok_or("The selection is no longer editable.")?;
        let board_id = runtime
            .scope()
            .map(|scope| scope.board_id)
            .unwrap_or_default();
        let snapshot = model.accepted.as_ref().ok_or("No accepted keyboard.")?;
        let cells = selected_cells(snapshot, &matrix_id, &model.selected_part_ids, &context);
        let affect = (self.splay_affect)();
        let resolver = match (field, &value, &context) {
            (Field::DeleteSelection, _, TreeContext::Key { .. }) => {
                remove_cells_resolver(matrix_id, cells)
            }
            (Field::KeyEnabled, Value::Bool(enabled), TreeContext::Key { .. }) => {
                enable_cells_resolver(matrix_id, board_id, cells, *enabled)
            }
            (Field::KeyOmit, Value::Bool(omit), TreeContext::Key { row, column, .. }) => {
                key_half_exception_at_resolver(matrix_id, *row, *column, *omit)
            }
            (Field::KeyAddAnchoredMatrix, _, TreeContext::Key { row, column, .. }) => {
                add_anchored_resolver(matrix_id, *row, *column)
            }
            _ => edit_resolver(
                matrix_id,
                board_id,
                context,
                field,
                value,
                affect,
                self.catalogue.read().clone(),
            ),
        };
        runtime
            .commit("layout-matrix-transform", resolver)
            .map(|_| ())
    }
}

fn matrix_of<'a>(accepted: &'a AcceptedSnapshot, id: &str) -> Option<&'a Matrix> {
    accepted
        .document
        .matrices
        .iter()
        .find(|matrix| matrix.id == id)
}

/// Resolve one field edit against the accepted document when the edit runs.
fn edit_resolver(
    matrix_id: String,
    board_id: String,
    context: TreeContext,
    field: Field,
    value: Value,
    affect: MatrixSplayAffect,
    catalogue: Vec<PartDefinition>,
) -> EditResolver {
    EditResolver::new(
        "layout-matrix-transform",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = matrix_of(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let Some(fields) = transform_fields(accepted, &board_id, &context, matrix, &[]) else {
                return Resolution::Retire(
                    "The selected row, column or key no longer exists.".into(),
                );
            };
            let operation =
                match build_operation(matrix, &fields, field, value.clone(), affect.clone()) {
                    Ok(operation) => operation,
                    Err(message) => return Resolution::Retire(message),
                };
            // An assembly or attachment the document lacks arrives with its catalogue definition.
            let wanted: Vec<&str> = match &value {
                Value::Text(id) if field == Field::KeyAssembly => vec![id.as_str()],
                Value::Attached(list) if field == Field::KeyAttached => {
                    list.iter().map(|(_, id)| id.as_str()).collect()
                }
                _ => Vec::new(),
            };
            let missing =
                super::matrix_preset::missing_definitions(&accepted.document, &catalogue, &wanted);
            if wanted.iter().any(|id| {
                !accepted.document.definitions.iter().any(|d| d.id == *id)
                    && !missing.iter().any(|d| d.id == *id)
            }) {
                return Resolution::Retire("The component catalogue is not ready yet. Try this change again when it finishes loading.".into());
            }
            let operation = match operation {
                EditOperation::SetMatrix { matrix, .. } if !missing.is_empty() => {
                    EditOperation::SetMatrix {
                        matrix,
                        definitions: Some(missing),
                    }
                }
                other => other,
            };
            Resolution::submit(vec![matrix.id.clone()], operation)
        },
    )
}

fn set_matrix(matrix: Matrix) -> EditOperation {
    EditOperation::SetMatrix {
        matrix,
        definitions: None,
    }
}

fn enable_cells_resolver(
    matrix_id: String,
    board_id: String,
    cells: Vec<(u32, u32)>,
    enabled: bool,
) -> EditResolver {
    EditResolver::new(
        "layout-matrix-transform",
        move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = matrix_of(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            let mut next = matrix.clone();
            for &(row, column) in &cells {
                let context = TreeContext::Key {
                    matrix_id: matrix_id.clone(),
                    row,
                    column,
                };
                let Some(fields) = transform_fields(accepted, &board_id, &context, &next, &[])
                else {
                    return Resolution::Retire("A selected key no longer exists.".into());
                };
                match build_operation(
                    &next,
                    &fields,
                    Field::KeyEnabled,
                    Value::Bool(enabled),
                    MatrixSplayAffect::Column,
                ) {
                    Ok(EditOperation::SetMatrix { matrix, .. }) => next = matrix,
                    _ => {
                        return Resolution::Retire(
                            "The key enabled edit could not be built.".into(),
                        );
                    }
                }
            }
            if &next == matrix {
                return Resolution::Unchanged;
            }
            Resolution::submit(vec![matrix.id.clone()], set_matrix(next))
        },
    )
}

fn remove_cells_resolver(matrix_id: String, cells: Vec<(u32, u32)>) -> EditResolver {
    EditResolver::new("layout-matrix-transform", move |_: &AcceptedSnapshot| {
        Resolution::submit(
            vec![matrix_id.clone()],
            EditOperation::RemoveMatrixCells {
                matrix_id: matrix_id.clone(),
                cells: cells.clone(),
            },
        )
    })
}

/// Core validates the anchor target and assigns the Origin, the Layout and the mirror.
fn add_anchored_resolver(matrix_id: String, row: u32, column: u32) -> EditResolver {
    EditResolver::new(
        "layout-matrix-transform",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(matrix) = matrix_of(accepted, &matrix_id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            if row >= matrix.rows || column >= matrix.columns {
                return Resolution::Retire("The selected key no longer exists.".into());
            }
            let id = crate::matrix_setup_operation::next_matrix_id(
                &document.matrices,
                &document.definitions,
                &document.parts,
                "anchored",
                crate::presentation::parts::reversible_layout(document),
            );
            Resolution::submit(
                vec![matrix_id.clone(), id.clone()],
                EditOperation::AddAnchoredMatrix {
                    id,
                    anchor: boardstudio_model::model::MatrixAnchor {
                        matrix_id: matrix_id.clone(),
                        row,
                        column,
                        follow: boardstudio_model::model::Follow::default(),
                        aggregate: None,
                    },
                },
            )
        },
    )
}

/// The (row, column) cells of the key selection, including a lone selected key.
fn selected_cells(
    snapshot: &AcceptedSnapshot,
    matrix_id: &str,
    selected_ids: &[String],
    context: &TreeContext,
) -> Vec<(u32, u32)> {
    let mut cells: Vec<_> = snapshot
        .scene
        .matrix_scenes
        .iter()
        .find(|scene| scene.matrix_id == matrix_id)
        .into_iter()
        .flat_map(|scene| &scene.cells)
        .filter(|cell| {
            selected_ids.iter().any(|id| {
                cell.member_id.as_ref() == Some(id)
                    || *id == format!("matrix/{matrix_id}/r{}c{}", cell.row, cell.column)
            })
        })
        .map(|cell| (cell.row, cell.column))
        .collect();
    if let TreeContext::Key { row, column, .. } = context
        && !cells.contains(&(*row, *column))
    {
        cells.push((*row, *column));
    }
    cells
}

fn linked_layout(document: &ProjectDoc, board_id: &str, matrix_id: &str) -> bool {
    super::mirror_links::layout_for_matrix(&document.layouts, board_id, matrix_id)
        .is_some_and(|layout| super::mirror_links::layout_is_linked(&document.layouts, layout))
}

/// The field projection of the selected matrix, row, column or key.
pub(super) fn transform_fields(
    snapshot: &AcceptedSnapshot,
    board_id: &str,
    context: &TreeContext,
    matrix: &Matrix,
    catalogue: &[PartDefinition],
) -> Option<MatrixTransformFields> {
    let zero = Vec2 { x: 0.0, y: 0.0 };
    let column_record = |column: u32| matrix.column_layout.get(column as usize);
    let scene = || {
        snapshot
            .scene
            .matrix_scenes
            .iter()
            .find(|scene| scene.matrix_id == matrix.id)
    };
    Some(match context {
        TreeContext::Matrix { .. } => MatrixTransformFields::Matrix {
            origin: matrix.origin,
            rotation: matrix.rotation.unwrap_or(0.0),
            mirror: matrix.mirror,
            mirror_y_locked: linked_layout(&snapshot.document, board_id, &matrix.id),
            rotation_followed: matrix
                .anchor
                .as_ref()
                .is_none_or(|anchor| anchor.follow.rotation),
            anchor: matrix.anchor.clone(),
        },
        TreeContext::Row { row, .. } => {
            let record = matrix.row_layout.get(*row as usize);
            MatrixTransformFields::Row {
                row: *row,
                offset: record.map_or(zero, |record| record.offset),
                shift: record.map_or(zero, |record| record.shift),
                bend: record.map_or(0.0, |record| record.bend),
                padding: record.and_then(|record| record.padding),
                pitch: matrix.pitch.y,
            }
        }
        TreeContext::Column { column, .. } => {
            let basis = scene()?
                .columns
                .iter()
                .find(|basis| basis.column == *column)?;
            MatrixTransformFields::Column {
                column: *column,
                offset: column_record(*column).map_or(zero, |record| record.offset),
                stagger: column_record(*column).map_or(0.0, |record| record.stagger),
                spread: column_record(*column).and_then(|record| record.spread),
                pitch: matrix.pitch.x,
                splay_angle: basis.splay_angle,
                rotation: basis.rotation,
                splay_origin: basis.splay_origin,
                custom_origin: basis.custom_origin,
                convert: convert_offset_available(matrix, *column),
            }
        }
        TreeContext::Key { row, column, .. } => {
            let member_id = scene()?
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column)
                .and_then(|cell| cell.member_id.clone())
                .or_else(|| {
                    key_half_exception_omitted(&snapshot.document, &matrix.id, *row, *column)
                        .map(|exception| exception.mirror_id)
                });
            let cell = matrix
                .cells
                .iter()
                .find(|cell| cell.row == *row && cell.column == *column);
            let definition_id = cell
                .and_then(|cell| cell.definition_id.clone())
                .unwrap_or_else(|| matrix.definition_id.clone());
            let choices =
                super::matrix_preset::switch_choices(&snapshot.document, catalogue, &definition_id);
            MatrixTransformFields::Key {
                row: *row,
                column: *column,
                enabled: cell.is_none_or(|cell| cell.enabled),
                definition_id,
                choices,
                assemblies: cell
                    .map(|cell| {
                        cell.assemblies
                            .iter()
                            .map(|a| (a.id.clone(), a.definition_id.clone()))
                            .collect()
                    })
                    .unwrap_or_default(),
                component_choices: snapshot.document.definitions.iter().chain(catalogue).fold(
                    Vec::new(),
                    |mut all: Vec<PartDefinition>, definition| {
                        if !all.iter().any(|existing| existing.id == definition.id) {
                            all.push(definition.clone());
                        }
                        all
                    },
                ),
                mirror_target: snapshot.document.layouts.iter().any(|layout| {
                    layout.matrix_ids.iter().any(|id| id == &matrix.id)
                        && layout.mirror_link.is_some()
                }),
                assemblies_local: cell.and_then(|cell| cell.assemblies_local).unwrap_or(false),
                offset: cell.and_then(|cell| cell.offset).unwrap_or(zero),
                rotation: cell.and_then(|cell| cell.rotation).unwrap_or(0.0),
                shift: cell.and_then(|cell| cell.shift).unwrap_or(zero),
                bend: cell.and_then(|cell| cell.bend).unwrap_or(0.0),
                half_exception: member_id
                    .as_deref()
                    .and_then(|member| key_half_exception(&snapshot.document, member)),
            }
        }
        _ => return None,
    })
}

/// One numeric transform field; hovering or focusing it lights the keys its edit would move.
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
    error: Signal<Option<String>>,
) -> Element {
    let preview = try_use_context::<FieldPreview>();
    let source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let shown = use_signal(Vec::<String>::new);
    let show = {
        let mount = mount.clone();
        move || {
            let mut shown = shown;
            let (Some(preview), Some(source), Some((_, context, _))) =
                (preview, &source, mount.parts())
            else {
                return;
            };
            if let Some(facts) = crate::matrix_relations::field_preview(
                &source.document,
                &source.scene.matrix_scenes,
                &context,
                field,
                (mount.splay_affect)(),
            ) {
                shown.set(facts.parts.clone());
                preview.show(FieldPreviewState {
                    parts: facts.parts,
                    hint: facts.hint,
                });
            }
        }
    };
    let release = move || {
        if let Some(preview) = preview {
            preview.release(&shown.peek());
        }
    };
    use_drop(release);
    let displayed = format_numeric(value, 4).parse::<f64>().unwrap_or(value);
    let mut error = error;
    rsx! {
        div { "data-transform-number": "true",
            onpointerenter: { let show = show.clone(); move |_| show() },
            onpointerleave: move |_| release(),
            onfocusin: move |_| show(),
            onfocusout: move |_| release(),
            NumericField {
                label, unit, value: displayed, precision: 4,
                on_commit: move |next| error.set(mount.edit(field, Value::Number(next)).err()),
            }
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
    rsx! {
        Button {
            button_type: "button", variant: ButtonVariant::Secondary,
            onclick: move |_| error.set(mount.edit(field, value.clone()).err()),
            "{label}"
        }
    }
}

fn toggle(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &str,
    checked: bool,
    mut error: Signal<Option<String>>,
) -> Element {
    let mount = mount.clone();
    rsx! {
        ToggleField { key: "{field:?}", label: label.to_owned(), checked,
            on_commit: move |next| error.set(mount.edit(field, Value::Bool(next)).err()) }
    }
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
    rsx! {
        div { onpointerdown: move |event| event.stop_propagation(), onkeydown: move |event| event.stop_propagation(),
            {action(&mount, Field::DeleteSelection, label, Value::Bool(true), error)}
        }
    }
}

#[component]
pub fn MatrixTransformInspector(
    mount: MatrixTransformInspectorMount,
    on_pick_splay_origin: EventHandler<()>,
    on_pick_anchor: EventHandler<()>,
    anchor_pick_pending: bool,
    on_cancel_anchor_pick: EventHandler<()>,
    /// Navigate to an aggregate anchor part's key without writing.
    on_navigate_anchor_part: EventHandler<TreeContext>,
) -> Element {
    let _ = use_context::<Signal<u64>>()();
    let _ = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>().map(|d| d.0());
    let error = use_signal(|| None::<String>);
    let Some((_, _, fields)) = mount.parts() else {
        return rsx! {};
    };
    let num = |field, label, unit, value| number(&mount, field, label, unit, value, error);
    let mm = |field, label, value| num(field, label, "mm", value);
    let deg = |field, label, value| num(field, label, "°", value);
    let act = |field, label: &str, value| action(&mount, field, label, value, error);
    let content = match &fields {
        MatrixTransformFields::Matrix {
            origin,
            rotation,
            mirror,
            mirror_y_locked,
            anchor,
            ..
        } => {
            let mirror = *mirror;
            let mut options = vec![(None, "None".to_owned()), (Some(Mirror::X), "X".to_owned())];
            if !mirror_y_locked || mirror == Some(Mirror::Y) {
                options.push((Some(Mirror::Y), "Y".to_owned()));
            }
            let edit = mount.clone();
            let mut error = error;
            rsx! {
                InspectorSection { title: "Position & orientation",
                    {mm(Field::OriginX, "Origin X", origin.x)}
                    {mm(Field::OriginY, "Origin Y", origin.y)}
                    {deg(Field::MatrixRotation, "Rotation", *rotation)}
                    SelectField::<Option<Mirror>> {
                        label: "Mirror", value: mirror, options,
                        on_commit: move |next| error.set(edit.edit(Field::MatrixMirror, Value::Mirror(next)).err()),
                    }
                }
                {anchor_controls(&mount, anchor.as_ref(), error, AnchorPick { pending: anchor_pick_pending, on_pick: on_pick_anchor, on_cancel: on_cancel_anchor_pick, on_navigate: on_navigate_anchor_part })}
            }
        }
        MatrixTransformFields::Row {
            offset,
            shift,
            bend,
            padding,
            pitch,
            ..
        } => rsx! {
            InspectorSection { title: "Row transform",
                {mm(Field::RowShiftX, "Shift X", shift.x)}
                {mm(Field::RowShiftY, "Shift Y", shift.y)}
                {deg(Field::RowBend, "Bend", *bend)}
                {mm(Field::RowPadding, "Padding", padding.unwrap_or(*pitch))}
                {mm(Field::RowOffsetX, "Offset X", offset.x)}
                {mm(Field::RowOffsetY, "Offset Y", offset.y)}
                {act(Field::RowOffsetReset, "Reset offsets", Value::Offset(Vec2::default()))}
            }
        },
        MatrixTransformFields::Column {
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
            let mut affect = mount.splay_affect;
            let edit = mount.clone();
            let custom = *custom_origin;
            let mut error = error;
            rsx! {
                InspectorSection { title: "Column transform",
                    {mm(Field::ColumnSpread, "Spread", spread.unwrap_or(*pitch))}
                    {mm(Field::ColumnStagger, "Stagger", *stagger)}
                    {mm(Field::ColumnOffsetX, "Offset X", offset.x)}
                    {mm(Field::ColumnOffsetY, "Offset Y", offset.y)}
                    {act(Field::ColumnOffsetReset, "Reset offsets", Value::Offset(Vec2::default()))}
                    if convert.is_ok() {
                        {act(Field::ColumnOffsetConvert, "Convert offset to stagger", Value::Number(0.0))}
                    }
                    SelectField::<MatrixSplayAffect> {
                        label: "Splay affects", value: affect(),
                        options: vec![(MatrixSplayAffect::Column, "This column".to_owned()), (MatrixSplayAffect::Following, "Following columns".to_owned())],
                        on_commit: move |value| affect.set(value),
                    }
                    {deg(Field::ColumnSplay, "Splay", column_angle(*splay_angle, *rotation, &affect()))}
                }
                InspectorSection { title: "Splay origin", default_open: false,
                    ToggleField { label: "Custom origin", checked: custom,
                        on_commit: move |next| error.set(edit.edit(Field::SplayOriginMode, Value::OriginMode(next)).err()) }
                    if custom {
                        {mm(Field::SplayOriginX, "Origin X", splay_origin.x)}
                        {mm(Field::SplayOriginY, "Origin Y", splay_origin.y)}
                        Button { variant: ButtonVariant::Secondary, onclick: move |_| on_pick_splay_origin.call(()), "Pick origin" }
                    }
                }
            }
        }
        MatrixTransformFields::Key {
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
            let mut error = error;
            rsx! {
                InspectorSection { title: "Key transform",
                    {toggle(&mount, Field::KeyEnabled, "Enabled", *enabled, error)}
                    if let Some(exception) = half_exception {
                        {toggle(&mount, Field::KeyOmit, "Omit mirrored key", exception.omitted, error)}
                    }
                    {mm(Field::KeyShiftX, "Shift X", shift.x)}
                    {mm(Field::KeyShiftY, "Shift Y", shift.y)}
                    {deg(Field::KeyBend, "Bend", *bend)}
                    {mm(Field::KeyOffsetX, "Local X", offset.x)}
                    {mm(Field::KeyOffsetY, "Local Y", offset.y)}
                    {deg(Field::KeyRotation, "Key rotation", *rotation)}
                    {act(Field::KeyTransformReset, "Reset local transform", Value::CellTransform { offset: Vec2::default(), rotation: 0.0 })}
                }
                InspectorSection { title: "Key assembly",
                    SelectField::<String> {
                        label: "Key assembly", value: definition_id.clone(), options: choices.clone(),
                        on_commit: move |next| error.set(edit.edit(Field::KeyAssembly, Value::Text(next)).err()),
                    }
                    if *mirror_target && *assemblies_local {
                        {act(Field::KeyAssembliesLocal, "Use mirrored components", Value::Bool(false))}
                    }
                    for (index, (id, definition)) in assemblies.iter().enumerate() {
                        {
                            let (replace, remove) = (mount.clone(), mount.clone());
                            let (baseline, mut remaining) = (assemblies.clone(), assemblies.clone());
                            remaining.remove(index);
                            let mut error = error;
                            rsx! {
                                div { key: "{id}",
                                    SelectField::<String> {
                                        label: format!("Replace {id}"), value: definition.clone(),
                                        options: attachment_component_choices(component_choices, definition),
                                        on_commit: move |next| {
                                            let mut replacements = baseline.clone();
                                            replacements[index].1 = next;
                                            error.set(replace.edit(Field::KeyAttached, Value::Attached(replacements)).err());
                                        },
                                    }
                                    Button { variant: ButtonVariant::Secondary, onclick: move |_| error.set(remove.edit(Field::KeyAttached, Value::Attached(remaining.clone())).err()), "Remove {id}" }
                                }
                            }
                        }
                    }
                }
                {act(Field::KeyAddAnchoredMatrix, "Add anchored matrix", Value::Text(String::new()))}
            }
        }
    };
    rsx! {
        section { class: "grid min-w-0 gap-3 px-4 py-3", aria_label: "Matrix transform properties",
            onkeydown: move |event: KeyboardEvent| { if event.key() == Key::Character(" ".into()) { event.stop_propagation(); } },
            {content}
            SelectionDeleteControl { mount: mount.clone() }
            if let Some(message) = error() {
                p { role: "alert", class: "text-destructive", "{message}" }
            }
        }
    }
}

struct AnchorPick {
    pending: bool,
    on_pick: EventHandler<()>,
    on_cancel: EventHandler<()>,
    on_navigate: EventHandler<TreeContext>,
}

fn anchor_controls(
    mount: &MatrixTransformInspectorMount,
    anchor: Option<&MatrixAnchor>,
    error: Signal<Option<String>>,
    pick: AnchorPick,
) -> Element {
    let Some(projection) = mount.projection() else {
        return rsx! {};
    };
    let aggregate = anchor.and_then(|anchor| anchor.aggregate.as_ref());
    let mut picked = (mount.aggregate_pick_draft)()
        .filter(|(owner, _)| *owner == projection.owner)
        .map(|(_, parts)| parts)
        .unwrap_or_default();
    if let Some(aggregate) = aggregate {
        picked.retain(|part| !aggregate.parts.contains(part));
    }
    // A refused detach names the linked partner; the next press confirms it.
    let partner = error().and_then(|message| {
        let (_, rest) = message.split_once(&format!(
            "Detaching {} would move its linked partner ",
            projection.owner.matrix_id
        ))?;
        let (partner, tail) = rest.split_once(';')?;
        (tail.trim_start()
            == format!("make {partner} Local to keep its pose and confirm the detach"))
        .then(|| partner.to_owned())
    });
    rsx! {
        InspectorSection { title: "Anchor",
            if let Some(anchor) = anchor {
                if let Some(aggregate) = aggregate {
                    {
                        let method = aggregate.method;
                        let mount = mount.clone();
                        let mut error = error;
                        rsx! {
                            SelectField::<AggregateMethod> {
                                label: "Method", value: method,
                                options: vec![(AggregateMethod::Average, "Average".to_owned()), (AggregateMethod::Intersect, "Intersection".to_owned())],
                                on_commit: move |next| { error.set(mount.edit(Field::MatrixAnchorMethod, Value::AnchorMethod(next)).err()); },
                            }
                        }
                    }
                    for (index, part) in aggregate.parts.iter().enumerate() {
                        {
                            let tree = TreeContext::Key { matrix_id: part.matrix_id.clone(), row: part.row, column: part.column };
                            let mut remaining = aggregate.parts.clone();
                            remaining.remove(index);
                            let last = remaining.is_empty();
                            let (on_navigate, mount, mut error) = (pick.on_navigate, mount.clone(), error);
                            rsx! {
                                div { key: "{part.matrix_id}-{part.row}-{part.column}",
                                    Button { variant: ButtonVariant::Secondary, onclick: move |_| on_navigate.call(tree.clone()), "{part.matrix_id} · row {part.row + 1} · column {part.column + 1}" }
                                    Button { variant: ButtonVariant::Secondary, disabled: last,
                                        onclick: move |_| error.set(mount.edit(Field::MatrixAnchorParts, Value::AnchorParts(remaining.clone())).err()), "Remove" }
                                }
                            }
                        }
                    }
                    if pick.pending && !picked.is_empty() {
                        {
                            let mut next = aggregate.parts.clone();
                            next.extend(picked.clone());
                            let (mount, mut error, on_cancel) = (mount.clone(), error, pick.on_cancel);
                            rsx! {
                                Button { variant: ButtonVariant::Secondary,
                                    onclick: move |_| {
                                        error.set(mount.edit(Field::MatrixAnchorParts, Value::AnchorParts(next.clone())).err());
                                        mount.clear_aggregate_pick();
                                        on_cancel.call(());
                                    },
                                    "Add {picked.len()} picked parts"
                                }
                            }
                        }
                    }
                } else {
                    p { "data-anchor-target": "true", "Anchored to {anchor.matrix_id} · row {anchor.row} · column {anchor.column}" }
                }
                {toggle(mount, Field::MatrixFollowX, "Follow X", anchor.follow.x, error)}
                {toggle(mount, Field::MatrixFollowY, "Follow Y", anchor.follow.y, error)}
                {toggle(mount, Field::MatrixFollowRotation, "Follow rotation", anchor.follow.rotation, error)}
                if let Some(partner) = partner {
                    {action(mount, Field::MatrixDetachConfirm, "Confirm detach", Value::Text(partner), error)}
                } else {
                    {action(mount, Field::MatrixDetach, "Detach", Value::Bool(true), error)}
                }
            }
            if pick.pending {
                {
                    let (mount, on_cancel) = (mount.clone(), pick.on_cancel);
                    rsx! { Button { variant: ButtonVariant::Secondary, onclick: move |_| { mount.clear_aggregate_pick(); on_cancel.call(()); }, "Cancel anchor pick" } }
                }
            } else {
                {
                    let on_pick = pick.on_pick;
                    rsx! { Button { variant: ButtonVariant::Secondary, onclick: move |_| on_pick.call(()), if aggregate.is_some() { "Pick parts" } else { "Pick anchor" } } }
                }
            }
        }
    }
}
