//! The selected matrix's name, size, pitch, switch, diode and spacing fields, each committed
//! synchronously as one `SetMatrix` / `SetLayout` edit.
use super::layout_rail::LayoutTransformRuntime;
use super::matrix_preset::{MatrixPreset, SwitchOrientation, apply_preset, duplicate_variant, matrix_preset, switch_choices};
use super::{ScopedTreeContext, TreeContext};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::{DiodeDirection, EditOperation, Matrix, MatrixCell, PartDefinition, Vec2};
use boardstudio_web_layout_core::rail_drag::nudge_session_is_ready;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TextCommitField,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct MatrixInspectorMount {
    pub runtime: LayoutTransformRuntime,
    pub selected: Option<ScopedTreeContext>,
}

pub fn use_matrix_inspector(
    runtime: std::rc::Rc<crate::runtime::Runtime>,
    selected: Signal<Option<ScopedTreeContext>>,
) -> MatrixInspectorMount {
    MatrixInspectorMount { runtime: LayoutTransformRuntime(runtime), selected: selected.read().clone() }
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Field {
    Rows,
    Columns,
    PitchX,
    PitchY,
    EdgeGapX,
    EdgeGapY,
}

impl MatrixInspectorMount {
    fn matrix_id(&self) -> Option<String> {
        let selected = self.selected.as_ref()?;
        let TreeContext::Matrix { matrix_id } = &selected.context else {
            return None;
        };
        (self.runtime.0.scope().as_ref() == Some(&selected.scope)).then(|| matrix_id.clone())
    }

    pub(super) fn selected_matrix_id(&self) -> Option<String> {
        self.matrix_id()
    }

    /// Add one row (`rows`) or column to the named matrix, as the Objects panel's add actions do.
    pub(super) fn grow(&self, matrix_id: &str, rows: bool) -> Result<(), String> {
        let id = matrix_id.to_owned();
        self.commit_on(id, "layout-matrix-inspector", move |_, matrix| {
            let next = if rows { resize(matrix, matrix.rows.saturating_add(1), matrix.columns)? } else { resize(matrix, matrix.rows, matrix.columns.saturating_add(1))? };
            Ok(EditOperation::SetMatrix { matrix: next, definitions: None })
        })
    }

    pub fn is_active(&self) -> bool {
        self.matrix_id().is_some_and(|id| {
            self.runtime
                .0
                .model()
                .accepted
                .is_some_and(|snapshot| snapshot.document.matrices.iter().any(|matrix| matrix.id == id))
        })
    }

    /// Commit one edit built from the matrix accepted when it runs.
    fn commit(&self, label: &str, build: impl Fn(&AcceptedSnapshot, &Matrix) -> Result<EditOperation, String> + 'static) -> Result<(), String> {
        let id = self.matrix_id().ok_or("The selection is no longer a matrix.")?;
        self.commit_on(id, label, build)
    }

    fn commit_on(&self, id: String, label: &str, build: impl Fn(&AcceptedSnapshot, &Matrix) -> Result<EditOperation, String> + 'static) -> Result<(), String> {
        let runtime = &self.runtime.0;
        if !nudge_session_is_ready(&runtime.model()) {
            return Err("The matrix is not ready to edit. Wait for the current operation to finish, then retry.".into());
        }
        let resolver = EditResolver::new("layout-matrix-inspector", move |accepted: &AcceptedSnapshot| {
            let Some(matrix) = accepted.document.matrices.iter().find(|matrix| matrix.id == id) else {
                return Resolution::Retire("The selected matrix no longer exists.".into());
            };
            match build(accepted, matrix) {
                Ok(operation) => Resolution::submit(vec![matrix.id.clone()], operation),
                Err(message) => Resolution::Retire(message),
            }
        });
        runtime.commit(label, resolver).map(|_| ())
    }

    fn edit(&self, field: Field, value: f64) -> Result<(), String> {
        self.commit("layout-matrix-inspector", move |_, matrix| {
            let mut next = matrix.clone();
            let gap = |matrix: &Matrix| matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
            match field {
                Field::Rows | Field::Columns => {
                    let count = value as u32;
                    if value.fract() != 0.0 || count == 0 {
                        return Err("Matrix dimensions must be positive whole numbers.".into());
                    }
                    next = if field == Field::Rows { resize(matrix, count, matrix.columns)? } else { resize(matrix, matrix.rows, count)? };
                }
                Field::PitchX | Field::PitchY if value > 0.0 => {
                    if field == Field::PitchX { next.pitch.x = value } else { next.pitch.y = value }
                }
                Field::EdgeGapX | Field::EdgeGapY if value >= 0.0 => {
                    let mut edge = gap(matrix);
                    if field == Field::EdgeGapX { edge.x = value } else { edge.y = value }
                    next.edge_gap = Some(edge);
                }
                _ => return Err("The matrix field value is invalid.".into()),
            }
            Ok(EditOperation::SetMatrix { matrix: next, definitions: None })
        })
    }
}

/// Resize keeping preset cells: new cells copy the preset template's assemblies.
fn resize(matrix: &Matrix, rows: u32, columns: u32) -> Result<Matrix, String> {
    if rows.checked_mul(columns).is_none_or(|cells| cells > 4096) {
        return Err("Matrix dimensions cannot exceed 4096 cells.".into());
    }
    let template = matrix.cells.iter().find(|cell| {
        cell.definition_id.as_deref() == Some(matrix.definition_id.as_str())
            && cell.variant.as_deref().is_some_and(|variant| variant.starts_with("preset/"))
    });
    let mut next = matrix.clone();
    next.rows = rows;
    next.columns = columns;
    let Some(template) = template else {
        return Ok(next);
    };
    next.cells.retain(|cell| cell.row < rows && cell.column < columns);
    let rotation = Some(if template.variant.as_deref().is_some_and(|variant| variant.ends_with("/north")) { 180.0 } else { 0.0 });
    let assemblies: Vec<_> = template.assemblies.iter().filter(|a| a.id == "diode" || a.id == "led").cloned().collect();
    for row in 0..rows {
        for column in (0..columns).filter(|&column| row >= matrix.rows || column >= matrix.columns) {
            next.cells.push(MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row,
                column,
                enabled: true,
                definition_id: template.definition_id.clone(),
                variant: template.variant.clone(),
                offset: None,
                rotation,
                assemblies: assemblies.clone(),
                assemblies_local: template.assemblies_local,
            });
        }
    }
    Ok(next)
}

#[component]
pub fn MatrixInspector(mount: MatrixInspectorMount) -> Element {
    let _ = use_context::<Signal<u64>>()();
    let mut error = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut templates = use_signal(Vec::<PartDefinition>::new);
    let (preset, orientation) = mount.runtime.0.model().accepted.as_ref().zip(mount.matrix_id()).and_then(|(snapshot, id)| snapshot.document.matrices.iter().find(|matrix| matrix.id == id)).map_or((None, None), matrix_preset);
    let mut preset_draft = use_signal(|| preset.unwrap_or(MatrixPreset::MxSolder));
    let mut orientation_draft = use_signal(|| orientation.unwrap_or(SwitchOrientation::South));
    use_hook(move || {
        wasm_bindgen_futures::spawn_local(async move {
            if let Ok(definitions) = crate::presentation::parts::load_matrix_templates(false).await {
                let _ = templates.try_write().map(|mut slot| *slot = definitions);
            }
        });
    });
    let (Some(matrix_id), Some(snapshot)) = (mount.matrix_id(), mount.runtime.0.model().accepted) else {
        return rsx! {};
    };
    let Some(matrix) = snapshot.document.matrices.iter().find(|matrix| matrix.id == matrix_id).cloned() else {
        return rsx! {};
    };
    let scope = mount.runtime.0.scope();
    let layout = snapshot
        .document
        .layouts
        .iter()
        .find(|layout| scope.as_ref().is_some_and(|scope| layout.board_id == scope.board_id) && layout.matrix_ids.contains(&matrix_id))
        .cloned();
    let linked = layout.as_ref().and_then(|active| {
        let partner = |id: &str| snapshot.document.layouts.iter().find(|candidate| candidate.id == id);
        match &active.mirror_link {
            Some(link) => partner(&link.source_id).map(|partner| (active.id.clone(), partner.name.clone())),
            None => snapshot
                .document
                .layouts
                .iter()
                .find(|other| other.mirror_link.as_ref().is_some_and(|link| link.source_id == active.id))
                .map(|other| (other.id.clone(), other.name.clone())),
        }
    });
    let choices = switch_choices(&snapshot.document, &templates(), &matrix.definition_id);
    let catalogue = templates();
    let gap = matrix.edge_gap.unwrap_or(Vec2 { x: 1.0, y: 1.0 });
    let (name_label, name) = match &layout {
        Some(layout) => ("Layout name", layout.name.clone()),
        None => ("Matrix name", matrix.name.clone().unwrap_or_else(|| format!("Matrix {}", matrix.id))),
    };
    let number = |field: Field, label: &'static str, unit: &'static str, value: f64, integer: bool| {
        let mount = mount.clone();
        rsx! {
            NumericField {
                key: "{field:?}", label, unit, value, precision: if integer { 0 } else { 3 },
                on_commit: move |next: f64| error.set(mount.edit(field, next).err()),
            }
        }
    };
    let rename = mount.clone();
    let pick_switch = mount.clone();
    let diode = mount.clone();
    let unlink = mount.clone();
    let delete = mount.clone();
    let layout_id = layout.as_ref().map(|layout| layout.id.clone());
    let board_id = scope.map(|scope| scope.board_id).unwrap_or_default();
    let unlink_board = board_id.clone();
    rsx! {
        section { class: "grid gap-3 min-w-0 px-4 pt-3 pb-4 text-foreground", aria_label: "Matrix inspector",
            onkeydown: move |event: KeyboardEvent| { if event.key().to_string() == " " { event.stop_propagation(); } },
            header { class: "flex items-baseline justify-between gap-2 border-b border-border pb-2.5", h2 { "Matrix" } span { "{matrix.id}" } }
            TextCommitField { label: name_label, value: name, on_commit: move |next: String| {
                let (layout_id, next) = (layout_id.clone(), next.trim().to_owned());
                error.set(rename.commit("layout-matrix-inspector", move |accepted, matrix| {
                    if next.is_empty() {
                        return Err("Enter a non-empty matrix or layout name.".into());
                    }
                    if let Some(id) = &layout_id {
                        let mut layout = accepted.document.layouts.iter().find(|layout| layout.id == *id).cloned().ok_or("The selected layout no longer exists.")?;
                        layout.name = next.clone();
                        return Ok(EditOperation::SetLayout { layout });
                    }
                    let mut renamed = matrix.clone();
                    renamed.name = Some(next.clone());
                    Ok(EditOperation::SetMatrix { matrix: renamed, definitions: None })
                }).err());
            } }
            {number(Field::Rows, "Rows", "", f64::from(matrix.rows), true)}
            {number(Field::Columns, "Columns", "", f64::from(matrix.columns), true)}
            {number(Field::PitchX, "Pitch X", "mm", matrix.pitch.x, false)}
            {number(Field::PitchY, "Pitch Y", "mm", matrix.pitch.y, false)}
            if let Some((_, partner)) = &linked {
                div { class: "grid gap-2 border-b border-border pb-3",
                    strong { "Linked to {partner}" }
                    p { "Key assemblies, diodes and components mirror across both halves. Replace a component on one half to keep it local." }
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| {
                        let (board_id, link_id) = (unlink_board.clone(), linked.as_ref().map(|(id, _)| id.clone()).unwrap_or_default());
                        error.set(unlink.runtime.0.commit("layout-matrix-unlink", EditResolver::new("layout-matrix-unlink", move |accepted: &AcceptedSnapshot| {
                            let Some(mut layout) = accepted.document.layouts.iter().find(|l| l.id == link_id && l.board_id == board_id).cloned() else {
                                return Resolution::Retire("The selected layout no longer exists.".into());
                            };
                            layout.mirror_link = None;
                            Resolution::submit(vec![link_id.clone()], EditOperation::SetLayout { layout })
                        })).err());
                    }, "Unlink halves" }
                }
            }
            InspectorSection { title: "Key assembly", default_open: false,
                SelectField::<String> { label: "Switch footprint", value: matrix.definition_id.clone(), options: choices,
                    on_commit: move |id: String| error.set(pick_switch.commit("layout-matrix-inspector", {
                        let catalogue = catalogue.clone();
                        move |accepted, matrix| {
                            let mut next = matrix.clone();
                            next.definition_id = id.clone();
                            let missing = super::matrix_preset::missing_definitions(&accepted.document, &catalogue, &[id.as_str()]);
                            if missing.is_empty() && !accepted.document.definitions.iter().any(|definition| definition.id == id) {
                                return Err("The selected switch footprint is not available in the loaded catalogue.".into());
                            }
                            Ok(EditOperation::SetMatrix { matrix: next, definitions: (!missing.is_empty()).then_some(missing) })
                        }
                    }).err()) }
                SelectField::<String> { label: "Assembly preset", value: preset_draft().as_str().to_owned(), disabled: busy(),
                    options: MatrixPreset::ALL.iter().map(|preset| (preset.as_str().to_owned(), preset.label().to_owned())).collect(),
                    on_commit: move |value: String| if let Some(next) = MatrixPreset::parse(&value) { preset_draft.set(next) } }
                SelectField::<String> { label: "Switch orientation", value: orientation_draft().as_str().to_owned(), disabled: busy(),
                    options: vec![("south".to_owned(), "South-facing LED".to_owned()), ("north".to_owned(), "North-facing LED".to_owned())],
                    on_commit: move |value: String| orientation_draft.set(if value == "north" { SwitchOrientation::North } else { SwitchOrientation::South }) }
                Button { variant: ButtonVariant::Secondary, disabled: busy(), onclick: {
                    let (runtime, matrix_id, board_id) = (mount.runtime.0.clone(), matrix_id.clone(), board_id.clone());
                    move |_| {
                        let (runtime, matrix_id, board_id) = (runtime.clone(), matrix_id.clone(), board_id.clone());
                        busy.set(true);
                        wasm_bindgen_futures::spawn_local(async move {
                            let result = apply_preset(&runtime, &matrix_id, &board_id, preset_draft.peek().clone(), orientation_draft.peek().clone()).await;
                            let _ = busy.try_write().map(|mut slot| *slot = false);
                            let _ = error.try_write().map(|mut slot| *slot = result.err());
                        });
                    }
                }, "Update assembly preset" }
                Button { variant: ButtonVariant::Secondary, disabled: busy(), onclick: {
                    let (runtime, matrix_id, scope) = (mount.runtime.0.clone(), matrix_id.clone(), mount.runtime.0.scope());
                    move |_| {
                        let (runtime, matrix_id, Some(scope)) = (runtime.clone(), matrix_id.clone(), scope.clone()) else { return };
                        busy.set(true);
                        wasm_bindgen_futures::spawn_local(async move {
                            let result = duplicate_variant(runtime.clone(), &scope, &matrix_id, preset_draft.peek().clone(), orientation_draft.peek().clone()).await;
                            let _ = busy.try_write().map(|mut slot| *slot = false);
                            if let Err(message) = result { runtime.report(message); }
                        });
                    }
                }, "Duplicate design as variant" }
                SelectField::<DiodeDirection> { label: "Diode direction", value: matrix.diode_direction.unwrap_or(DiodeDirection::Row2col),
                    options: vec![(DiodeDirection::Row2col, "Rows to columns".to_owned()), (DiodeDirection::Col2row, "Columns to rows".to_owned())],
                    on_commit: move |direction| error.set(diode.commit("layout-matrix-inspector", move |_, matrix| {
                        let mut next = matrix.clone();
                        next.diode_direction = Some(direction);
                        Ok(EditOperation::SetMatrix { matrix: next, definitions: None })
                    }).err()) }
            }
            InspectorSection { title: "Keycap spacing", default_open: false,
                {number(Field::EdgeGapX, "Edge gap X", "mm", gap.x, false)}
                {number(Field::EdgeGapY, "Edge gap Y", "mm", gap.y, false)}
            }
            Button { variant: ButtonVariant::Secondary, onclick: move |_| {
                error.set(delete.commit("layout-matrix-delete", |_, matrix| Ok(EditOperation::RemoveMatrix { id: matrix.id.clone() })).err());
            }, "Delete matrix" }
            if let Some(message) = error() { p { role: "alert", class: "text-destructive", "{message}" } }
        }
    }
}
