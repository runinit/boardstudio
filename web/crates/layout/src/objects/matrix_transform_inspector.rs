//! Private form for matrix, row, column, and cell-local transform properties.
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{MatrixSplayAffect, Mirror, Vec2};
use dioxus::prelude::*;

use super::TreeContext;
use super::matrix_transform_controller::MatrixTransformInspectorMount;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "matrix_transform_inspector_tests.rs"]
mod mounted_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixTransformInspectorOwner {
    pub editor_instance_id: u64,
    pub workspace: &'static str,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub matrix_id: String,
    pub context: TreeContext,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::presentation) enum MatrixTransformState {
    Pending,
    Saved,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::presentation) struct MatrixTransformFeedback {
    pub owner: MatrixTransformInspectorOwner,
    pub request_id: u64,
    pub field: MatrixTransformField,
    pub state: MatrixTransformState,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MatrixTransformRequest {
    pub owner: MatrixTransformInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixTransformField,
    pub baseline: MatrixTransformValue,
    pub value: MatrixTransformValue,
    pub splay_affect: MatrixSplayAffect,
}

#[derive(Clone, Debug, PartialEq)]
pub(in crate::presentation) struct MatrixTransformProjection {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub label: String,
    pub fields: MatrixTransformFields,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct MatrixTransformInspectorProps {
    pub mount: MatrixTransformInspectorMount,
    pub on_pick_splay_origin: EventHandler<()>,
}

#[component]
pub(in crate::presentation) fn MatrixTransformInspector(
    props: MatrixTransformInspectorProps,
) -> Element {
    let Some(projection) = props.mount.projection.as_ref() else {
        return rsx! {};
    };
    let request_sequence = props.mount.request_sequence;
    let on_edit = props.mount.on_edit;
    let owner = projection.owner.clone();
    let owner_key = transform_owner_key(&owner);
    let snapshot_token = projection.snapshot_token;
    let revision = projection.revision;
    let mut splay_affect = props.mount.splay_affect;
    let (title, body) = match &projection.fields {
        MatrixTransformFields::Matrix {
            origin,
            rotation,
            mirror,
            mirror_y_locked,
        } => (
            "Position & orientation",
            rsx! {
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::OriginX, label: "Origin X", unit: "mm",
                        value: origin.x, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::OriginY, label: "Origin Y", unit: "mm",
                        value: origin.y, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::MatrixRotation, label: "Rotation", unit: "°",
                        value: *rotation, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                MirrorTransformField {
                    owner: owner.clone(), snapshot_token, revision, value: *mirror,
                    mirror_y_locked: *mirror_y_locked, request_sequence,
                    editable: props.mount.editable, busy: props.mount.busy,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
            },
        ),
        MatrixTransformFields::Row { row, offset } => (
            "Position & rotation",
            rsx! {
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowOffsetX, label: "Offset X", unit: "mm",
                        value: offset.x, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowOffsetY, label: "Offset Y", unit: "mm",
                        value: offset.y, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                ResetTransformButton {
                    label: "Reset offsets", owner: owner.clone(), snapshot_token, revision,
                    field: MatrixTransformField::RowOffsetReset,
                    baseline: MatrixTransformValue::Offset(*offset),
                    value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                    request_sequence, editable: props.mount.editable, busy: props.mount.busy,
                    feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                }
                span { class: "m1-matrix-field-context", "Row {row}" }
            },
        ),
        MatrixTransformFields::Column {
            column,
            offset,
            stagger,
            splay_angle,
            splay_origin,
            custom_origin,
        } => (
            "Column properties",
            rsx! {
                section { class: "m1-matrix-transform-section", aria_label: "Splay and origin",
                    h3 { "Splay & origin" }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnSplay, label: "Splay", unit: "°",
                        value: *splay_angle, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    OriginModeTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        custom: *custom_origin, request_sequence,
                        editable: props.mount.editable, busy: props.mount.busy,
                        feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                    }
                    div { class: "m1-matrix-inspector-fields",
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::SplayOriginX, label: "Origin X", unit: "mm",
                            value: splay_origin.x, request_sequence, editable: props.mount.editable,
                            busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::SplayOriginY, label: "Origin Y", unit: "mm",
                            value: splay_origin.y, request_sequence, editable: props.mount.editable,
                            busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                    }
                    label { class: "m1-matrix-field", "Splay affects"
                        select {
                            "aria-label": "Splay affects",
                            value: if splay_affect() == MatrixSplayAffect::Column { "column" } else { "following" },
                            disabled: !props.mount.editable || props.mount.busy,
                            onchange: move |event: FormEvent| {
                                splay_affect.set(if event.value() == "column" { MatrixSplayAffect::Column } else { MatrixSplayAffect::Following });
                            },
                            option { value: "column", "This column" }
                            option { value: "following", "This and following" }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "m1-secondary",
                        disabled: !props.mount.editable || props.mount.busy,
                        onclick: move |_| props.on_pick_splay_origin.call(()),
                        "Pick origin"
                    }
                    p { class: "m1-empty-note", "Move the origin without moving the keys." }
                }
                section { class: "m1-matrix-transform-section", aria_label: "Position and rotation",
                    h3 { "Position & rotation" }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnStagger, label: "Stagger", unit: "mm",
                        value: *stagger, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    div { class: "m1-matrix-inspector-fields",
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnOffsetX, label: "Offset X", unit: "mm",
                            value: offset.x, request_sequence, editable: props.mount.editable,
                            busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnOffsetY, label: "Offset Y", unit: "mm",
                            value: offset.y, request_sequence, editable: props.mount.editable,
                            busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                    }
                    ResetTransformButton {
                        label: "Reset offsets", owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnOffsetReset,
                        baseline: MatrixTransformValue::Offset(*offset),
                        value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                        request_sequence, editable: props.mount.editable, busy: props.mount.busy,
                        feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                    }
                    span { class: "m1-matrix-field-context", "Column {column}" }
                }
            },
        ),
        MatrixTransformFields::Key {
            row,
            column,
            enabled,
            definition_id,
            choices,
            assemblies,
            component_choices,
            mirror_target,
            assemblies_local,
            offset,
            rotation,
        } => (
            "Key properties",
            rsx! {
                EnabledTransformField {
                    owner: owner.clone(), snapshot_token, revision, value: *enabled,
                    request_sequence, editable: props.mount.editable, busy: props.mount.busy,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyOffsetX, label: "Local X", unit: "mm",
                        value: offset.x, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyOffsetY, label: "Local Y", unit: "mm",
                        value: offset.y, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyRotation, label: "Key rotation", unit: "°",
                        value: *rotation, request_sequence, editable: props.mount.editable,
                        busy: props.mount.busy, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                ResetTransformButton {
                    label: "Reset local transform", owner: owner.clone(), snapshot_token, revision,
                    field: MatrixTransformField::KeyTransformReset,
                    baseline: MatrixTransformValue::CellTransform { offset: *offset, rotation: *rotation },
                    value: MatrixTransformValue::CellTransform { offset: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                    request_sequence, editable: props.mount.editable, busy: props.mount.busy,
                    feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                }
                KeyAssemblyField {
                    owner: owner.clone(), snapshot_token, revision, value: definition_id.clone(),
                    choices: choices.clone(), request_sequence, editable: props.mount.editable,
                    busy: props.mount.busy, feedback: props.mount.feedback.clone(), on_edit,
                    splay_affect,
                }
                AttachedComponentsField {
                    owner: owner.clone(), snapshot_token, revision, value: assemblies.clone(),
                    choices: component_choices.clone(), mirror_target: *mirror_target,
                    assemblies_local: *assemblies_local, request_sequence,
                    editable: props.mount.editable, busy: props.mount.busy,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
                span { class: "m1-matrix-field-context", "Key {row}, {column}" }
            },
        ),
    };
    rsx! {
        section { key: "{owner_key}", class: "m1-matrix-inspector m1-matrix-transform-inspector", aria_label: "Matrix transform properties",
            header { class: "m1-matrix-inspector-heading",
                h2 { "{title}" }
                span { "{projection.label}" }
            }
            {body}
            if props.mount.busy {
                p { class: "m1-matrix-edit-status", role: "status", "Saving transform change…" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct NumericTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixTransformField,
    label: &'static str,
    unit: &'static str,
    value: f64,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn NumericTransformField(props: NumericTransformFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.to_string());
    let mut baseline = use_signal(|| props.value);
    let mut dirty = use_signal(|| false);
    let mut conflict = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| None::<u64>);
    let mut status = use_signal(|| None::<String>);
    let owner = props.owner.clone();
    let field = props.field;
    let accepted_value = props.value;
    let feedback = props.feedback.clone();
    let request_id = submitted();
    let matching_feedback = feedback.into_iter().find(|item| {
        item.owner == owner && item.field == field && Some(item.request_id) == request_id
    });
    let mut draft_effect = draft;
    let mut baseline_effect = baseline;
    let mut dirty_effect = dirty;
    let mut conflict_effect = conflict;
    let mut error_effect = error;
    let mut submitted_effect = submitted;
    let mut status_effect = status;
    use_effect(use_reactive(
        (&accepted_value, &matching_feedback),
        move |(value, feedback)| {
            match feedback.as_ref().map(|item| item.state) {
                Some(MatrixTransformState::Pending) => {
                    status_effect.set(Some("Saving…".to_owned()));
                    return;
                }
                Some(MatrixTransformState::Saved) => {
                    draft_effect.set(value.to_string());
                    baseline_effect.set(value);
                    dirty_effect.set(false);
                    conflict_effect.set(false);
                    error_effect.set(None);
                    submitted_effect.set(None);
                    status_effect.set(Some("Saved".to_owned()));
                    return;
                }
                Some(MatrixTransformState::Failed) => {
                    submitted_effect.set(None);
                    status_effect.set(None);
                    error_effect.set(Some(
                        feedback
                            .as_ref()
                            .and_then(|item| item.message.clone())
                            .unwrap_or_else(|| {
                                "This transform was not saved. Review the value and retry."
                                    .to_owned()
                            }),
                    ));
                }
                None => {}
            }
            if baseline_effect() != value {
                if dirty_effect() {
                    conflict_effect.set(true);
                } else {
                    draft_effect.set(value.to_string());
                    baseline_effect.set(value);
                    conflict_effect.set(false);
                    error_effect.set(None);
                    status_effect.set(None);
                    submitted_effect.set(None);
                }
            }
        },
    ));
    let commit = {
        let owner = props.owner.clone();
        let field = props.field;
        let sequence = props.request_sequence;
        let token = props.snapshot_token;
        let revision = props.revision;
        let on_edit = props.on_edit;
        let affect = props.splay_affect;
        let editable = props.editable;
        let busy = props.busy;
        let mut sequence = sequence;
        move || {
            if !editable || busy || submitted().is_some() {
                return;
            }
            let next = match draft().trim().parse::<f64>() {
                Ok(number) if number.is_finite() => number,
                _ => {
                    error.set(Some("Enter a finite number.".to_owned()));
                    return;
                }
            };
            let accepted_baseline = baseline();
            if next == accepted_baseline {
                draft.set(accepted_value.to_string());
                baseline.set(accepted_value);
                dirty.set(false);
                conflict.set(false);
                error.set(None);
                status.set(None);
                submitted.set(None);
                return;
            }
            let Some(id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Transform request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(id);
            let request = MatrixTransformRequest {
                owner: owner.clone(),
                request_id: id,
                snapshot_token: token,
                revision,
                field,
                baseline: MatrixTransformValue::Number(accepted_baseline),
                value: MatrixTransformValue::Number(next),
                splay_affect: affect(),
            };
            submitted.set(Some(id));
            dirty.set(true);
            error.set(None);
            conflict.set(false);
            status.set(Some("Saving…".to_owned()));
            on_edit.call(request);
        }
    };
    let current = draft();
    let has_error = error().is_some() || conflict();
    let error_message = if conflict() {
        Some("The accepted value changed. Edit this draft to apply it to the current value, or press Escape to reload.".to_owned())
    } else {
        error().or_else(|| {
            props
                .feedback
                .iter()
                .find(|item| {
                    item.owner == props.owner
                        && item.field == props.field
                        && submitted() == Some(item.request_id)
                        && item.state == MatrixTransformState::Failed
                })
                .and_then(|item| item.message.clone())
        })
    };
    rsx! {
        label { class: if has_error { "m1-matrix-field has-error" } else { "m1-matrix-field" },
            span { "{props.label}" }
            span { class: "m1-matrix-field-input",
                input {
                    r#type: "number", step: "any", value: "{current}",
                    readonly: !props.editable || props.busy,
                    "aria-label": props.label,
                    "aria-invalid": has_error,
                    oninput: move |event: FormEvent| {
                        if conflict() {
                            baseline.set(accepted_value);
                            conflict.set(false);
                        } else if !dirty() {
                            baseline.set(accepted_value);
                        }
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        status.set(None);
                        submitted.set(None);
                    },
                    onblur: { let mut commit = commit.clone(); move |_| commit() },
                    onkeydown: {
                        let mut commit = commit.clone();
                        move |event: KeyboardEvent| match event.data().key() {
                            Key::Enter => { event.prevent_default(); commit(); }
                            Key::Escape => {
                                event.prevent_default();
                                draft.set(accepted_value.to_string());
                                baseline.set(accepted_value);
                                dirty.set(false);
                                conflict.set(false);
                                error.set(None);
                                status.set(None);
                                submitted.set(None);
                            }
                            _ => {}
                        }
                    },
                }
                small { "{props.unit}" }
            }
            if let Some(message) = error_message.as_deref() { small { role: "alert", "{message}" } }
            if let Some(message) = status().as_deref() { small { role: "status", "{message}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MirrorTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    value: Option<Mirror>,
    mirror_y_locked: bool,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn MirrorTransformField(props: MirrorTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_value = props.value;
    let observed_value = use_signal(|| current_value);
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_value;
    use_effect(use_reactive((&current_value,), move |(value,)| {
        if observed_effect() != value {
            observed_effect.set(value);
            submitted_effect.set(None);
        }
    }));
    let message = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::MatrixMirror
                && submitted() == Some(item.request_id)
        })
        .filter(|item| item.state == MatrixTransformState::Failed)
        .and_then(|item| item.message.clone());
    let request = props.request_sequence;
    let on_edit = props.on_edit;
    let owner = props.owner.clone();
    let value = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let affect = props.splay_affect;
    rsx! {
        label { class: if message.is_some() { "m1-matrix-field has-error" } else { "m1-matrix-field" },
            span { "Mirror" }
            select {
                "aria-label": "Mirror matrix",
                value: mirror_value(value),
                disabled: !props.editable || props.busy,
                onchange: move |event: FormEvent| {
                    let next = match event.value().as_str() { "x" => Some(Mirror::X), "y" => Some(Mirror::Y), _ => None };
                    if next == value || (next == Some(Mirror::Y) && props.mirror_y_locked) { return; }
                    let mut request = request;
                    let Some(id) = request().checked_add(1) else { return; };
                    request.set(id);
                    submitted.set(Some(id));
                    on_edit.call(MatrixTransformRequest {
                        owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                        field: MatrixTransformField::MatrixMirror,
                        baseline: MatrixTransformValue::Mirror(value), value: MatrixTransformValue::Mirror(next),
                        splay_affect: affect(),
                    });
                },
                option { value: "none", "None" }
                option { value: "x", "X axis" }
                option { value: "y", disabled: props.mirror_y_locked, "Y axis" }
            }
            if let Some(error) = message.as_deref() { small { role: "alert", "{error}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct OriginModeTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    custom: bool,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn OriginModeTransformField(props: OriginModeTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_custom = props.custom;
    let observed_custom = use_signal(|| current_custom);
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_custom;
    use_effect(use_reactive((&current_custom,), move |(custom,)| {
        if observed_effect() != custom {
            observed_effect.set(custom);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::SplayOriginMode
                && submitted() == Some(item.request_id)
                && item.state == MatrixTransformState::Failed
        })
        .and_then(|item| item.message.clone());
    let request = props.request_sequence;
    let on_edit = props.on_edit;
    let owner = props.owner.clone();
    let custom = props.custom;
    let token = props.snapshot_token;
    let revision = props.revision;
    let affect = props.splay_affect;
    rsx! {
        label { class: "m1-matrix-field", "Origin"
            select {
                "aria-label": "Splay origin",
                value: if custom { "custom" } else { "base" },
                disabled: !props.editable || props.busy,
                onchange: move |event: FormEvent| {
                    let next = event.value() == "custom";
                    if next == custom { return; }
                    let mut request = request;
                    let Some(id) = request().checked_add(1) else { return; };
                    request.set(id);
                    submitted.set(Some(id));
                    on_edit.call(MatrixTransformRequest {
                        owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                        field: MatrixTransformField::SplayOriginMode,
                        baseline: MatrixTransformValue::OriginMode(custom), value: MatrixTransformValue::OriginMode(next),
                        splay_affect: affect(),
                    });
                },
                option { value: "base", "Column base" }
                option { value: "custom", "Custom point" }
            }
            if let Some(message) = error.as_deref() { small { role: "alert", "{message}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct EnabledTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    value: bool,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn EnabledTransformField(props: EnabledTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyEnabled
                && submitted() == Some(item.request_id)
                && item.state == MatrixTransformState::Failed
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || props.busy;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
        label { class: "m1-matrix-key-enabled",
            input {
                r#type: "checkbox", aria_label: "Key enabled", checked: current, disabled,
                onchange: move |event| {
                    let Some(id) = sequence().checked_add(1) else { return; };
                    sequence.set(id);
                    submitted.set(Some(id));
                    on_edit.call(MatrixTransformRequest {
                        owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                        field: MatrixTransformField::KeyEnabled,
                        baseline: MatrixTransformValue::Bool(current),
                        value: MatrixTransformValue::Bool(event.checked()),
                        splay_affect: affect(),
                    });
                },
            }
            " Enabled"
        }
        if let Some(message) = error.as_deref() { small { role: "alert", class: "m1-matrix-transform-error", "{message}" } }
    }
}

#[derive(Props, Clone, PartialEq)]
struct KeyAssemblyFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    value: String,
    choices: Vec<(String, String)>,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn KeyAssemblyField(props: KeyAssemblyFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyAssembly
                && submitted() == Some(item.request_id)
                && item.state == MatrixTransformState::Failed
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || props.busy;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.value.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
        label { class: "m1-matrix-key-assembly",
            "Key Assembly"
            select {
                aria_label: "Key Assembly", disabled, value: "{props.value}",
                onchange: move |event| {
                    let Some(id) = sequence().checked_add(1) else { return; };
                    sequence.set(id);
                    submitted.set(Some(id));
                    on_edit.call(MatrixTransformRequest {
                        owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                        field: MatrixTransformField::KeyAssembly,
                        baseline: MatrixTransformValue::Text(current.clone()),
                        value: MatrixTransformValue::Text(event.value()),
                        splay_affect: affect(),
                    });
                },
                for (id, label) in props.choices.iter() {
                    option { key: "{id}", value: "{id}", selected: *id == props.value, "{label}" }
                }
            }
        }
        if let Some(message) = error.as_deref() { small { role: "alert", class: "m1-matrix-transform-error", "{message}" } }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AttachedComponentsFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    value: Vec<(String, String)>,
    choices: Vec<boardstudio_core::model::PartDefinition>,
    mirror_target: bool,
    assemblies_local: bool,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
fn AttachedComponentsField(props: AttachedComponentsFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::KeyAttached
                && submitted() == Some(item.request_id)
                && item.state == MatrixTransformState::Failed
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || props.busy;
    let sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let current = props.value.clone();
    let send = move |next: Vec<(String, String)>| {
        let mut sequence = sequence;
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::KeyAttached,
            baseline: MatrixTransformValue::Attached(current.clone()),
            value: MatrixTransformValue::Attached(next),
            splay_affect: affect(),
        });
    };
    rsx! {
        h3 { class: "m1-matrix-subtitle", "Attached components" }
        if props.mirror_target {
            if props.assemblies_local {
                ResetTransformButton {
                    label: "Use mirrored components", owner: props.owner.clone(),
                    snapshot_token: props.snapshot_token, revision: props.revision,
                    field: MatrixTransformField::KeyAssembliesLocal,
                    baseline: MatrixTransformValue::Bool(true),
                    value: MatrixTransformValue::Bool(false),
                    request_sequence: props.request_sequence, editable: props.editable,
                    busy: props.busy, feedback: props.feedback.clone(),
                    splay_affect: props.splay_affect, on_edit: props.on_edit,
                }
            } else {
                p { class: "m1-matrix-empty-note", "The key assembly and attached components follow the paired half. Replacing one here keeps this key local." }
            }
        }
        if props.value.is_empty() {
            p { class: "m1-matrix-empty-note", "No attached components. Apply a component in Parts." }
        }
        for (assembly_id, definition_id) in props.value.iter().cloned() {
            { let choices = crate::matrix_transform_operation::attachment_component_choices(&props.choices, &definition_id);
              rsx! {
                div { key: "{assembly_id}", class: "m1-matrix-attached-row",
                    select {
                        aria_label: "Replace {assembly_id}", disabled, value: "{definition_id}",
                        onchange: {
                            let assembly_id = assembly_id.clone();
                            let all = props.value.clone();
                            let mut send = send.clone();
                            move |event| {
                                send(
                                    all.iter()
                                        .map(|(id, def)| {
                                            if *id == assembly_id { (id.clone(), event.value()) } else { (id.clone(), def.clone()) }
                                        })
                                        .collect(),
                                );
                            }
                        },
                        for (id, label) in choices.iter() {
                            option { key: "{id}", value: "{id}", selected: *id == definition_id, "{label}" }
                        }
                    }
                    button {
                        r#type: "button", class: "m1-inspector-secondary", disabled,
                        aria_label: "Remove {assembly_id}",
                        onclick: {
                            let assembly_id = assembly_id.clone();
                            let all = props.value.clone();
                            let mut send = send.clone();
                            move |_| send(all.iter().filter(|(id, _)| *id != assembly_id).cloned().collect())
                        },
                        "Remove"
                    }
                }
              }
            }
        }
        if let Some(message) = error.as_deref() { small { role: "alert", class: "m1-matrix-transform-error", "{message}" } }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ResetTransformButtonProps {
    label: &'static str,
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixTransformField,
    baseline: MatrixTransformValue,
    value: MatrixTransformValue,
    request_sequence: Signal<u64>,
    editable: bool,
    busy: bool,
    feedback: Vec<MatrixTransformFeedback>,
    splay_affect: Signal<MatrixSplayAffect>,
    on_edit: EventHandler<MatrixTransformRequest>,
}

#[component]
fn ResetTransformButton(props: ResetTransformButtonProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_baseline = props.baseline.clone();
    let observed_baseline = use_signal(|| current_baseline.clone());
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_baseline;
    use_effect(use_reactive((&current_baseline,), move |(baseline,)| {
        if observed_effect() != baseline {
            observed_effect.set(baseline);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == props.field
                && submitted() == Some(item.request_id)
                && item.state == MatrixTransformState::Failed
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || props.busy;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let baseline = props.baseline.clone();
    let value = props.value.clone();
    let field = props.field;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
        button {
            r#type: "button", class: "m1-inspector-secondary", disabled,
            onclick: move |_| {
                let Some(id) = sequence().checked_add(1) else { return; };
                sequence.set(id);
                submitted.set(Some(id));
                on_edit.call(MatrixTransformRequest {
                    owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                    field, baseline: baseline.clone(), value: value.clone(), splay_affect: affect(),
                });
            },
            "{props.label}"
        }
        if let Some(message) = error.as_deref() { small { role: "alert", class: "m1-matrix-transform-error", "{message}" } }
    }
}

fn mirror_value(value: Option<Mirror>) -> &'static str {
    match value {
        Some(Mirror::X) => "x",
        Some(Mirror::Y) => "y",
        None | Some(Mirror::None) => "none",
    }
}

fn transform_owner_key(owner: &MatrixTransformInspectorOwner) -> String {
    format!(
        "{}-{}-{}-{}-{}-{}-{}-{:?}",
        owner.editor_instance_id,
        owner.context_generation,
        owner.scope_generation,
        owner.scope.session_epoch.0,
        owner.scope.document_id,
        owner.scope.board_id,
        owner.matrix_id,
        owner.context,
    )
}
