//! Private form for the real selected-matrix name, size, and pitch fields.
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixNameTarget {
    Matrix,
    Layout { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixEditField {
    Name,
    Rows,
    Columns,
    PitchX,
    PitchY,
}

impl MatrixEditField {
    fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Rows => "rows",
            Self::Columns => "columns",
            Self::PitchX => "pitch-x",
            Self::PitchY => "pitch-y",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) enum MatrixEditValue {
    Name(Option<String>),
    Rows(u32),
    Columns(u32),
    PitchX(f64),
    PitchY(f64),
}

/// Draft owner excludes accepted token/revision so unrelated accepted edits do not erase text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixInspectorOwner {
    pub editor_instance_id: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub matrix_id: String,
    pub name_target: MatrixNameTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MatrixInspectorProjection {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub matrix_label: String,
    pub name_label: &'static str,
    pub name_value: String,
    pub name_baseline: MatrixEditValue,
    pub rows: u32,
    pub columns: u32,
    pub pitch_x: f64,
    pub pitch_y: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MatrixEditRequest {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixEditField,
    pub baseline: MatrixEditValue,
    pub value: MatrixEditValue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixEditState {
    Pending,
    Saved,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixEditFeedback {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub field: MatrixEditField,
    pub state: MatrixEditState,
    pub message: Option<String>,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct MatrixInspectorProps {
    pub projection: MatrixInspectorProjection,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
pub(in crate::presentation) fn MatrixInspector(props: MatrixInspectorProps) -> Element {
    let projection = &props.projection;
    let name_feedback = props.feedback.clone();
    let rows_feedback = props.feedback.clone();
    let columns_feedback = props.feedback.clone();
    let pitch_x_feedback = props.feedback.clone();
    let pitch_y_feedback = props.feedback.clone();
    let name_key = owner_key(&projection.owner, MatrixEditField::Name);
    let rows_key = owner_key(&projection.owner, MatrixEditField::Rows);
    let columns_key = owner_key(&projection.owner, MatrixEditField::Columns);
    let pitch_x_key = owner_key(&projection.owner, MatrixEditField::PitchX);
    let pitch_y_key = owner_key(&projection.owner, MatrixEditField::PitchY);
    rsx! {
        section { class: "m1-matrix-inspector", aria_label: "Matrix inspector",
            header { class: "m1-matrix-inspector-heading",
                h2 { "Matrix" }
                span { "{projection.matrix_label}" }
            }
            div { class: "m1-matrix-inspector-fields",
                MatrixFieldEditor {
                    key: "{name_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Name,
                    label: projection.name_label, value: projection.name_value.clone(),
                    baseline: projection.name_baseline.clone(), kind: MatrixFieldKind::Name,
                    request_sequence: props.request_sequence, editable: props.editable, busy: props.busy,
                    feedback: name_feedback, on_edit: props.on_edit,
                }
                MatrixFieldEditor {
                    key: "{rows_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Rows,
                    label: "Rows", value: projection.rows.to_string(),
                    baseline: MatrixEditValue::Rows(projection.rows), kind: MatrixFieldKind::PositiveInteger,
                    request_sequence: props.request_sequence, editable: props.editable, busy: props.busy,
                    feedback: rows_feedback, on_edit: props.on_edit,
                }
                MatrixFieldEditor {
                    key: "{columns_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Columns,
                    label: "Columns", value: projection.columns.to_string(),
                    baseline: MatrixEditValue::Columns(projection.columns), kind: MatrixFieldKind::PositiveInteger,
                    request_sequence: props.request_sequence, editable: props.editable, busy: props.busy,
                    feedback: columns_feedback, on_edit: props.on_edit,
                }
                MatrixFieldEditor {
                    key: "{pitch_x_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::PitchX,
                    label: "Pitch X", value: projection.pitch_x.to_string(),
                    baseline: MatrixEditValue::PitchX(projection.pitch_x), kind: MatrixFieldKind::PositiveNumber,
                    request_sequence: props.request_sequence, editable: props.editable, busy: props.busy,
                    feedback: pitch_x_feedback, on_edit: props.on_edit,
                }
                MatrixFieldEditor {
                    key: "{pitch_y_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::PitchY,
                    label: "Pitch Y", value: projection.pitch_y.to_string(),
                    baseline: MatrixEditValue::PitchY(projection.pitch_y), kind: MatrixFieldKind::PositiveNumber,
                    request_sequence: props.request_sequence, editable: props.editable, busy: props.busy,
                    feedback: pitch_y_feedback, on_edit: props.on_edit,
                }
            }
            if props.busy {
                p { class: "m1-matrix-edit-status", role: "status", "Saving matrix change…" }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatrixFieldKind {
    Name,
    PositiveInteger,
    PositiveNumber,
}

#[derive(Props, Clone, PartialEq)]
struct MatrixFieldEditorProps {
    owner: MatrixInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixEditField,
    label: &'static str,
    value: String,
    baseline: MatrixEditValue,
    kind: MatrixFieldKind,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixEditFeedback>,
    on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
fn MatrixFieldEditor(props: MatrixFieldEditorProps) -> Element {
    let mut draft = use_signal(|| props.value.clone());
    let mut draft_baseline = use_signal(|| props.baseline.clone());
    let mut dirty = use_signal(|| false);
    let mut stale = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitted_request_id = use_signal(|| None::<u64>);
    let mut status = use_signal(|| None::<String>);
    let accepted_value = props.value.clone();
    let accepted_baseline = props.baseline.clone();
    let owner = props.owner.clone();
    let field = props.field;
    let feedback = props.feedback.clone();
    let editable = props.editable;
    let busy = props.busy;
    let mut draft_for_effect = draft;
    let mut baseline_for_effect = draft_baseline;
    let mut dirty_for_effect = dirty;
    let mut stale_for_effect = stale;
    let mut error_for_effect = error;
    let mut submitted_for_effect = submitted_request_id;
    let mut status_for_effect = status;
    let submitted_id = submitted_request_id();
    let matching_feedback = feedback.into_iter().find(|feedback| {
        feedback.owner == owner
            && feedback.field == field
            && submitted_id == Some(feedback.request_id)
    });
    use_effect(use_reactive(
        (&accepted_value, &accepted_baseline, &matching_feedback),
        move |(value, baseline, feedback)| {
            match feedback.as_ref().map(|feedback| feedback.state) {
                Some(MatrixEditState::Pending) => {
                    status_for_effect.set(Some("Saving…".to_owned()));
                    return;
                }
                Some(MatrixEditState::Saved) => {
                    draft_for_effect.set(value.clone());
                    baseline_for_effect.set(baseline.clone());
                    dirty_for_effect.set(false);
                    stale_for_effect.set(false);
                    error_for_effect.set(None);
                    submitted_for_effect.set(None);
                    status_for_effect.set(Some("Saved".to_owned()));
                    return;
                }
                Some(MatrixEditState::Failed) => {
                    submitted_for_effect.set(None);
                    status_for_effect.set(None);
                    error_for_effect.set(Some(
                        feedback
                            .as_ref()
                            .and_then(|item| item.message.clone())
                            .unwrap_or_else(|| {
                                "This matrix change was not saved. Review the value and retry."
                                    .to_owned()
                            }),
                    ));
                }
                None => {}
            }

            if baseline_for_effect() != baseline {
                if dirty_for_effect() {
                    stale_for_effect.set(true);
                } else {
                    draft_for_effect.set(value.clone());
                    baseline_for_effect.set(baseline.clone());
                    stale_for_effect.set(false);
                    error_for_effect.set(None);
                    status_for_effect.set(None);
                    submitted_for_effect.set(None);
                }
            }
        },
    ));

    let commit = {
        let owner = props.owner.clone();
        let field = props.field;
        let kind = props.kind;
        let sequence = props.request_sequence;
        let snapshot_token = props.snapshot_token;
        let revision = props.revision;
        let on_edit = props.on_edit;
        let accepted_display = props.value.clone();
        move || {
            if busy || !editable || submitted_request_id().is_some() {
                return;
            }
            if stale() {
                error.set(Some(
                    "The accepted value changed. Press Escape to reload it before editing."
                        .to_owned(),
                ));
                return;
            }
            let text = draft();
            let value = match kind {
                MatrixFieldKind::Name => {
                    let name = text.trim();
                    if name.is_empty() {
                        error.set(Some("Enter a name.".to_owned()));
                        return;
                    }
                    MatrixEditValue::Name(Some(name.to_owned()))
                }
                MatrixFieldKind::PositiveInteger => match text.trim().parse::<u32>() {
                    Ok(value) if value > 0 => match field {
                        MatrixEditField::Rows => MatrixEditValue::Rows(value),
                        MatrixEditField::Columns => MatrixEditValue::Columns(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a positive whole number.".to_owned()));
                        return;
                    }
                },
                MatrixFieldKind::PositiveNumber => match text.trim().parse::<f64>() {
                    Ok(value) if value.is_finite() && value > 0.0 => match field {
                        MatrixEditField::PitchX => MatrixEditValue::PitchX(value),
                        MatrixEditField::PitchY => MatrixEditValue::PitchY(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a finite number greater than zero.".to_owned()));
                        return;
                    }
                },
            };
            let current = match field {
                MatrixEditField::Name => match kind {
                    MatrixFieldKind::Name => value.clone(),
                    _ => return,
                },
                MatrixEditField::Rows => value.clone(),
                MatrixEditField::Columns => value.clone(),
                MatrixEditField::PitchX => value.clone(),
                MatrixEditField::PitchY => value.clone(),
            };
            let baseline = draft_baseline();
            let displayed_name_is_unchanged = field == MatrixEditField::Name
                && matches!(&current, MatrixEditValue::Name(Some(name)) if name == accepted_display.trim());
            if current == baseline || displayed_name_is_unchanged {
                draft.set(accepted_display.clone());
                dirty.set(false);
                stale.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
                return;
            }
            let mut sequence = sequence;
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Matrix request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MatrixEditRequest {
                owner: owner.clone(),
                request_id,
                snapshot_token,
                revision,
                field,
                baseline,
                value,
            };
            submitted_request_id.set(Some(request_id));
            dirty.set(true);
            error.set(None);
            status.set(Some("Saving…".to_owned()));
            on_edit.call(request);
        }
    };

    let on_keydown = {
        let commit = commit.clone();
        move |event: KeyboardEvent| match event.data().key().as_str() {
            "Enter" => {
                event.prevent_default();
                commit();
            }
            "Escape" => {
                event.prevent_default();
                draft.set(props.value.clone());
                draft_baseline.set(props.baseline.clone());
                dirty.set(false);
                stale.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
            }
            _ => {}
        }
    };
    let feedback_error = props
        .feedback
        .iter()
        .find(|feedback| {
            feedback.owner == props.owner
                && feedback.field == props.field
                && submitted_request_id() == Some(feedback.request_id)
                && feedback.state == MatrixEditState::Failed
        })
        .and_then(|feedback| feedback.message.clone());
    let error_text = error().or(feedback_error);
    let current_status = status();
    let input_value = draft();
    let current_baseline = props.baseline.clone();
    let input_type = if props.kind == MatrixFieldKind::Name {
        "text"
    } else {
        "number"
    };
    let step = match props.kind {
        MatrixFieldKind::Name => "any",
        MatrixFieldKind::PositiveInteger => "1",
        MatrixFieldKind::PositiveNumber => "any",
    };
    let min = match props.kind {
        MatrixFieldKind::Name => None,
        MatrixFieldKind::PositiveInteger => Some("1"),
        MatrixFieldKind::PositiveNumber => Some("0"),
    };
    rsx! {
        label { class: if error_text.is_some() || stale() { "m1-matrix-field has-error" } else { "m1-matrix-field" },
            span { "{props.label}" }
            span { class: "m1-matrix-field-input",
                input {
                    r#type: input_type,
                    step: step,
                    min: min,
                    value: "{input_value}",
                    readonly: !props.editable || props.busy || stale(),
                    "aria-label": props.label,
                    "aria-invalid": error_text.is_some() || stale(),
                    oninput: move |event: FormEvent| {
                        if !dirty() {
                            draft_baseline.set(current_baseline.clone());
                        }
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        status.set(None);
                        submitted_request_id.set(None);
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: on_keydown,
                }
                if matches!(props.kind, MatrixFieldKind::PositiveNumber) { small { "mm" } }
            }
            if stale() {
                small { role: "alert", "The accepted value changed. Press Escape to reload it." }
            } else if let Some(message) = error_text.as_deref() {
                small { role: "alert", "{message}" }
            }
            if let Some(message) = current_status.as_deref() {
                small { role: "status", "{message}" }
            }
        }
    }
}

fn owner_key(owner: &MatrixInspectorOwner, field: MatrixEditField) -> String {
    fn encode(value: &str) -> String {
        use std::fmt::Write as _;
        let mut result = String::with_capacity(value.len() * 2);
        for byte in value.as_bytes() {
            let _ = write!(result, "{byte:02x}");
        }
        result
    }
    let instance = owner
        .scope
        .instance_id
        .as_deref()
        .map(encode)
        .unwrap_or_default();
    let target = match &owner.name_target {
        MatrixNameTarget::Matrix => "matrix".to_owned(),
        MatrixNameTarget::Layout { id } => format!("layout-{}", encode(id)),
    };
    format!(
        "{}-{}-{}-{}-{}-{}-{}-{}-{}",
        owner.editor_instance_id,
        owner.context_generation,
        owner.scope_generation,
        owner.scope.session_epoch.0,
        encode(&owner.scope.document_id),
        encode(&owner.scope.board_id),
        instance,
        encode(&owner.matrix_id),
        format!("{target}-{}", field.key()),
    )
}
