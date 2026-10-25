//! Private form for matrix, row, column, and cell-local transform properties.
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue, column_angle,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{AggregateMethod, Follow, MatrixAnchor, MatrixSplayAffect, Mirror, Vec2};
use boardstudio_web_ui_shared::controls::{
    BoardButton, BoardCheckbox, BoardDisclosure, BoardNumberField, BoardSelectField,
};
use dioxus::prelude::*;

use super::TreeContext;
use super::matrix_transform_controller::MatrixTransformInspectorMount;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "matrix_transform_inspector_tests.rs"]
mod mounted_tests;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixTransformInspectorOwner {
    pub editor_instance_id: u64,
    pub workspace: &'static str,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub matrix_id: String,
    pub context: TreeContext,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixTransformFeedback {
    pub owner: MatrixTransformInspectorOwner,
    pub request_id: u64,
    pub field: MatrixTransformField,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTransformRequest {
    pub owner: MatrixTransformInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixTransformField,
    pub baseline: MatrixTransformValue,
    pub value: MatrixTransformValue,
    pub splay_affect: MatrixSplayAffect,
    pub(super) draft: Option<Signal<String>>,
    pub(super) failure: Option<Signal<Option<String>>>,
    pub(super) submitted_text: Option<String>,
    pub(super) one_shot: bool,
}

impl MatrixTransformRequest {
    /// The request for a one-shot action (reset, convert, delete), shared by the
    /// Inspector buttons and the Relations panel so there is one construction path.
    /// `scope` is the snapshot token and revision; `change` the field, baseline and value.
    pub(super) fn one_shot(
        owner: MatrixTransformInspectorOwner,
        scope: (SnapshotToken, u64),
        request_id: u64,
        change: (
            MatrixTransformField,
            MatrixTransformValue,
            MatrixTransformValue,
        ),
        splay_affect: MatrixSplayAffect,
    ) -> Self {
        let (field, baseline, value) = change;
        Self {
            owner,
            request_id,
            snapshot_token: scope.0,
            revision: scope.1,
            field,
            baseline,
            value,
            splay_affect,
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTransformProjection {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub label: String,
    pub fields: MatrixTransformFields,
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixTransformInspectorProps {
    pub mount: MatrixTransformInspectorMount,
    pub on_pick_splay_origin: EventHandler<()>,
    pub on_pick_anchor: EventHandler<()>,
    pub anchor_pick_pending: bool,
    pub on_cancel_anchor_pick: EventHandler<()>,
    /// Navigate to an aggregate anchor part's key. The handler selects the
    /// part key without writing, so the revision and history stay unchanged.
    pub on_navigate_anchor_part: EventHandler<TreeContext>,
}

#[component]
pub fn MatrixTransformInspector(props: MatrixTransformInspectorProps) -> Element {
    let mut mounted = props.mount.inspector_mounted;
    let visible = props.mount.projection.is_some();
    use_effect(use_reactive((&visible,), move |(visible,)| {
        mounted.set(visible)
    }));
    use_drop(move || mounted.set(false));
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
            rotation_followed,
            anchor,
        } => (
            "Position & orientation",
            rsx! {
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::OriginX, label: "Origin X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::OriginX).0, failure: props.mount.numeric_field(MatrixTransformField::OriginX).1,
                        value: origin.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::OriginY, label: "Origin Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::OriginY).0, failure: props.mount.numeric_field(MatrixTransformField::OriginY).1,
                        value: origin.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::MatrixRotation, label: "Rotation", unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::MatrixRotation).0, failure: props.mount.numeric_field(MatrixTransformField::MatrixRotation).1,
                        value: *rotation, request_sequence, editable: props.mount.editable, disabled: !rotation_followed, feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    if !rotation_followed {
                        p { class: "m1-empty-note m1-matrix-transform-note", role: "note",
                            "Rotation is not followed, so turning it changes nothing."
                        }
                    }
                }
                MirrorTransformField {
                    owner: owner.clone(), snapshot_token, revision, value: *mirror,
                    mirror_y_locked: *mirror_y_locked, request_sequence,
                    editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
                h3 { class: "m1-matrix-subtitle", "Anchor" }
                if let Some(anchor) = anchor {
                    if anchor.aggregate.is_some() {
                        AggregateAnchorSection {
                            owner: owner.clone(), snapshot_token, revision,
                            anchor: anchor.clone(),
                            request_sequence, editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                            on_navigate_anchor_part: props.on_navigate_anchor_part,
                        }
                    } else {
                        p { class: "m1-matrix-anchor-target",
                            "Anchored to {anchor.matrix_id} · row {anchor.row} · column {anchor.column}"
                        }
                    }
                    p { class: "m1-matrix-anchor-follow", "{follows_label(&anchor.follow)}" }
                    FollowTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        anchor: anchor.clone(),
                        request_sequence, editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                    }
                    DetachAnchorButton {
                        owner: owner.clone(), snapshot_token, revision,
                        request_sequence, editable: props.mount.editable,
                        detach_pending: props.mount.one_shot_disabled(MatrixTransformField::MatrixDetach),
                        confirm_pending: props.mount.one_shot_disabled(MatrixTransformField::MatrixDetachConfirm),
                        feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                    }
                    if anchor.aggregate.is_none() {
                        PickAnchorButton {
                            editable: props.mount.editable,
                            pending: props.anchor_pick_pending,
                            on_pick_anchor: props.on_pick_anchor,
                            on_cancel_anchor_pick: props.on_cancel_anchor_pick,
                        }
                    }
                    if let Some(message) = props.mount.feedback.iter().rev().find(|item| {
                        item.owner == owner
                            && item.field == MatrixTransformField::MatrixAnchorPick
                            && item.message.is_some()
                    }).and_then(|item| item.message.clone()) {
                        p { role: "alert", class: "m1-matrix-transform-error", "{message}" }
                    }
                } else {
                    p { class: "m1-empty-note", "No anchor. The matrix places itself." }
                }
            },
        ),
        MatrixTransformFields::Row {
            row,
            offset,
            shift,
            bend,
            padding,
            pitch,
        } => (
            "Position & rotation",
            rsx! {
                h3 { class: "m1-matrix-subtitle", "Carried" }
                p { class: "m1-matrix-field-context", "Moves or turns this row and later rows." }
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowShiftX, label: "Shift X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowShiftX).0, failure: props.mount.numeric_field(MatrixTransformField::RowShiftX).1,
                        value: shift.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowShiftY, label: "Shift Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowShiftY).0, failure: props.mount.numeric_field(MatrixTransformField::RowShiftY).1,
                        value: shift.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowBend, label: "Bend", unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::RowBend).0, failure: props.mount.numeric_field(MatrixTransformField::RowBend).1,
                        value: *bend, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    if *row == 0 {
                        p { class: "m1-empty-note m1-matrix-transform-note", role: "note",
                            "Padding starts at row 1. Row 0 sits at the column base."
                        }
                    } else {
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::RowPadding, label: "Padding", unit: "mm",
                            draft: props.mount.numeric_field(MatrixTransformField::RowPadding).0, failure: props.mount.numeric_field(MatrixTransformField::RowPadding).1,
                            value: padding.unwrap_or(*pitch), request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                            badge: padding.is_none().then(|| "pitch".to_owned()),
                        }
                    }
                }
                h3 { class: "m1-matrix-subtitle", "This row only" }
                p { class: "m1-matrix-field-context", "Nothing else moves." }
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowOffsetX, label: "Offset X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowOffsetX).0, failure: props.mount.numeric_field(MatrixTransformField::RowOffsetX).1,
                        value: offset.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::RowOffsetY, label: "Offset Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowOffsetY).0, failure: props.mount.numeric_field(MatrixTransformField::RowOffsetY).1,
                        value: offset.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                ResetTransformButton {
                    label: "Reset offsets", owner: owner.clone(), snapshot_token, revision,
                    field: MatrixTransformField::RowOffsetReset,
                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::RowOffsetReset),
                    baseline: MatrixTransformValue::Offset(*offset),
                    value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                    request_sequence, editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                }
                span { class: "m1-matrix-field-context", "Row {row}" }
            },
        ),
        MatrixTransformFields::Column {
            column,
            offset,
            stagger,
            spread,
            pitch,
            splay_angle,
            rotation,
            splay_origin,
            custom_origin,
            convert,
        } => (
            "Column properties",
            rsx! {
                BoardDisclosure { class: "m1-matrix-transform-section", open: true, aria_label: "Position and rotation",
                    heading: rsx! { summary { "Transform" } },
                    h3 { class: "m1-matrix-subtitle", "Carried" }
                    p { class: "m1-matrix-field-context", "Moves or turns this column and the columns after it." }
                    if *column == 0 {
                        p { class: "m1-empty-note m1-matrix-transform-note", role: "note",
                            "Spread starts at column 1. Column 0 steps by pitch."
                        }
                    } else {
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnSpread, label: "Spread", unit: "mm",
                            draft: props.mount.numeric_field(MatrixTransformField::ColumnSpread).0, failure: props.mount.numeric_field(MatrixTransformField::ColumnSpread).1,
                            value: spread.unwrap_or(*pitch), request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                            badge: spread.is_none().then(|| "pitch".to_owned()),
                        }
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnStagger, label: "Stagger", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::ColumnStagger).0, failure: props.mount.numeric_field(MatrixTransformField::ColumnStagger).1,
                        value: *stagger, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnSplay, label: "Splay", unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::ColumnSplay).0, failure: props.mount.numeric_field(MatrixTransformField::ColumnSplay).1,
                        value: column_angle(*splay_angle, *rotation, &splay_affect()), request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    BoardSelectField {
                            class: "m1-matrix-field", label: "Splay affects",
                            value: if splay_affect() == MatrixSplayAffect::Column { "column" } else { "following" },
                            disabled: !props.mount.editable,
                            onchange: move |event: FormEvent| {
                                splay_affect.set(if event.value() == "column" { MatrixSplayAffect::Column } else { MatrixSplayAffect::Following });
                            },
                            option { value: "column", "This column" }
                            option { value: "following", "This and following" }
                    }
                    h3 { class: "m1-matrix-subtitle", "This column only" }
                    p { class: "m1-matrix-field-context", "Nothing else moves." }
                    div { class: "m1-matrix-inspector-fields",
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnOffsetX, label: "Offset X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::ColumnOffsetX).0, failure: props.mount.numeric_field(MatrixTransformField::ColumnOffsetX).1,
                            value: offset.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnOffsetY, label: "Offset Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::ColumnOffsetY).0, failure: props.mount.numeric_field(MatrixTransformField::ColumnOffsetY).1,
                            value: offset.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                    }
                    OriginModeTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        custom: *custom_origin, request_sequence,
                        editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                    }
                    ResetTransformButton {
                        label: "Reset offsets", owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::ColumnOffsetReset,
                        pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::ColumnOffsetReset),
                        baseline: MatrixTransformValue::Offset(*offset),
                        value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                        request_sequence, editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                    }
                    if offset.y != 0.0 {
                        ResetTransformButton {
                            label: "Convert to stagger", owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::ColumnOffsetConvert,
                            pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::ColumnOffsetConvert),
                            baseline: MatrixTransformValue::Number(offset.y),
                            value: MatrixTransformValue::Number(0.0),
                            blocked: convert.clone().err(),
                            request_sequence, editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                        }
                    }
                    span { class: "m1-matrix-field-context", {format!("Column {}", column + 1)} }
                }
                BoardDisclosure { class: "m1-matrix-transform-section", open: false, aria_label: "Splay and origin",
                    heading: rsx! { summary { "Origin & scope" } },
                    div { class: "m1-matrix-inspector-fields",
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::SplayOriginX, label: "Origin X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::SplayOriginX).0, failure: props.mount.numeric_field(MatrixTransformField::SplayOriginX).1,
                            value: splay_origin.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                        NumericTransformField {
                            owner: owner.clone(), snapshot_token, revision,
                            field: MatrixTransformField::SplayOriginY, label: "Origin Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::SplayOriginY).0, failure: props.mount.numeric_field(MatrixTransformField::SplayOriginY).1,
                            value: splay_origin.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                            on_edit, splay_affect,
                        }
                    }
                    BoardButton {
                        r#type: "button",
                        class: "m1-secondary",
                        disabled: !props.mount.editable,
                        onclick: move |_| props.on_pick_splay_origin.call(()),
                        "Pick origin"
                    }
                    p { class: "m1-empty-note", "Move the origin without moving the keys." }
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
            shift,
            bend,
        } => (
            "Key properties",
            rsx! {
                BoardDisclosure {
                    class: "m1-matrix-transform-section", open: true,
                    heading: rsx! { summary { "Position & rotation" } },
                EnabledTransformField {
                    owner: owner.clone(), snapshot_token, revision, value: *enabled,
                    request_sequence, editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
                h3 { class: "m1-matrix-subtitle", "Carried" }
                p { class: "m1-matrix-field-context", "Moves or turns this key and the keys above it." }
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyShiftX, label: "Shift X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyShiftX).0, failure: props.mount.numeric_field(MatrixTransformField::KeyShiftX).1,
                        value: shift.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyShiftY, label: "Shift Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyShiftY).0, failure: props.mount.numeric_field(MatrixTransformField::KeyShiftY).1,
                        value: shift.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyBend, label: "Bend", unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyBend).0, failure: props.mount.numeric_field(MatrixTransformField::KeyBend).1,
                        value: *bend, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                h3 { class: "m1-matrix-subtitle", "This key only" }
                p { class: "m1-matrix-field-context", "Nothing else moves." }
                div { class: "m1-matrix-inspector-fields",
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyOffsetX, label: "Local X", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyOffsetX).0, failure: props.mount.numeric_field(MatrixTransformField::KeyOffsetX).1,
                        value: offset.x, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyOffsetY, label: "Local Y", unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyOffsetY).0, failure: props.mount.numeric_field(MatrixTransformField::KeyOffsetY).1,
                        value: offset.y, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(), snapshot_token, revision,
                        field: MatrixTransformField::KeyRotation, label: "Key rotation", unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::KeyRotation).0, failure: props.mount.numeric_field(MatrixTransformField::KeyRotation).1,
                        value: *rotation, request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(),
                        on_edit, splay_affect,
                    }
                }
                ResetTransformButton {
                    label: "Reset local transform", owner: owner.clone(), snapshot_token, revision,
                    field: MatrixTransformField::KeyTransformReset,
                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyTransformReset),
                    baseline: MatrixTransformValue::CellTransform { offset: *offset, rotation: *rotation },
                    value: MatrixTransformValue::CellTransform { offset: Vec2 { x: 0.0, y: 0.0 }, rotation: 0.0 },
                    request_sequence, editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                }
                ResetTransformButton {
                    label: "Add anchored matrix", owner: owner.clone(), snapshot_token, revision,
                    field: MatrixTransformField::KeyAddAnchoredMatrix,
                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyAddAnchoredMatrix),
                    baseline: MatrixTransformValue::Text(String::new()),
                    value: MatrixTransformValue::Text(String::new()),
                    request_sequence, editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                }
                }
                BoardDisclosure {
                    class: "m1-matrix-transform-section", open: true,
                    heading: rsx! { summary { "Key assembly" } },
                KeyAssemblyField {
                    owner: owner.clone(), snapshot_token, revision, value: definition_id.clone(),
                    choices: choices.clone(), request_sequence, editable: props.mount.editable,  feedback: props.mount.feedback.clone(), on_edit,
                    splay_affect,
                }
                AttachedComponentsField {
                    owner: owner.clone(), snapshot_token, revision, value: assemblies.clone(),
                    remove_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyAttached),
                    mirror_reset_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyAssembliesLocal),
                    choices: component_choices.clone(), mirror_target: *mirror_target,
                    assemblies_local: *assemblies_local, request_sequence,
                    editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                }
                }
                span { class: "m1-matrix-field-context", "Key {row}, {column}" }
            },
        ),
    };
    let delete_label = match &projection.fields {
        MatrixTransformFields::Matrix { .. } => "Delete matrix",
        MatrixTransformFields::Row { .. } => "Delete row",
        MatrixTransformFields::Column { .. } => "Delete column",
        MatrixTransformFields::Key { .. } => "Delete selected keys",
    };
    rsx! {
        section { key: "{owner_key}", class: "m1-matrix-inspector m1-matrix-transform-inspector", aria_label: "Matrix transform properties",
            header { class: "m1-matrix-inspector-heading",
                h2 { "{title}" }
                span { "{projection.label}" }
            }
            if matches!(projection.fields, MatrixTransformFields::Column { .. } | MatrixTransformFields::Key { .. }) {
                {body}
            } else {
                BoardDisclosure {
                    class: "m1-matrix-transform-section", open: true,
                    heading: rsx! { summary { "{title}" } },
                    {body}
                }
            }
            div { class: "m1-selection-delete",
            ResetTransformButton {
                label: delete_label, owner: owner.clone(), snapshot_token, revision,
                field: MatrixTransformField::DeleteSelection,
                pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::DeleteSelection),
                baseline: MatrixTransformValue::Bool(false), value: MatrixTransformValue::Bool(true),
                request_sequence, editable: props.mount.editable,
                feedback: props.mount.feedback.clone(), splay_affect, on_edit,
            }
            }
            if let Some(announcement) = (props.mount.change_status)().filter(|announcement| announcement.owner == projection.owner) {
                p { class: "m1-matrix-edit-status m1-matrix-change-status", role: "status", "{announcement.message}" }
            }
        }
    }
}

/// Canvas delete uses exactly the same typed request route as the Inspector action.
#[component]
pub fn SelectionDeleteControl(mount: MatrixTransformInspectorMount) -> Element {
    let Some(projection) = mount.projection.as_ref() else {
        return rsx! {};
    };
    let label = match projection.fields {
        MatrixTransformFields::Matrix { .. } => "Delete matrix",
        MatrixTransformFields::Row { .. } => "Delete row",
        MatrixTransformFields::Column { .. } => "Delete column",
        MatrixTransformFields::Key { .. } => "Delete selected keys",
    };
    rsx! { div { class: "m1-canvas-delete", onpointerdown: move |event| event.stop_propagation(), onkeydown: move |event| event.stop_propagation(),
        ResetTransformButton { label, owner: projection.owner.clone(), snapshot_token: projection.snapshot_token, revision: projection.revision,
            field: MatrixTransformField::DeleteSelection,
            pending_disabled: mount.one_shot_disabled(MatrixTransformField::DeleteSelection),
            baseline: MatrixTransformValue::Bool(false), value: MatrixTransformValue::Bool(true),
            request_sequence: mount.request_sequence, editable: mount.editable,
            feedback: mount.feedback.clone(), splay_affect: mount.splay_affect, on_edit: mount.on_edit,
        }
    } }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct NumericTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixTransformField,
    label: &'static str,
    unit: &'static str,
    value: f64,
    request_sequence: Signal<u64>,
    editable: bool,
    feedback: Vec<MatrixTransformFeedback>,
    on_edit: EventHandler<MatrixTransformRequest>,
    splay_affect: Signal<MatrixSplayAffect>,
    draft: Signal<String>,
    failure: Signal<Option<String>>,
    /// Hard unavailability with its own explained reason, e.g. a rotation the
    /// anchor does not follow. Unlike `editable`, this disables the input.
    #[props(default)]
    disabled: bool,
    #[props(default)]
    display_precision: Option<usize>,
    /// A muted badge beside the value, e.g. the pitch a default step follows.
    #[props(default)]
    badge: Option<String>,
}

#[component]
pub(super) fn NumericTransformField(props: NumericTransformFieldProps) -> Element {
    let display = move |value: f64| match props.display_precision {
        Some(0) => format!("{value:.0}"),
        Some(precision) => {
            let text = format!("{value:.precision$}");
            let text = text.trim_end_matches('0').trim_end_matches('.');
            if text == "-0" {
                "0".to_owned()
            } else {
                text.to_owned()
            }
        }
        None => value.to_string(),
    };
    let mut draft = props.draft;
    let mut baseline = use_signal(|| props.value);
    let mut dirty = use_signal(|| false);
    let mut error = props.failure;
    let accepted_value = props.value;
    let mut draft_effect = draft;
    let mut baseline_effect = baseline;
    let mut dirty_effect = dirty;
    let mut error_effect = error;
    let mut observed_owner = use_signal(|| props.owner.clone());
    let field_affect =
        (props.field == MatrixTransformField::ColumnSplay).then(|| (props.splay_affect)());
    let mut observed_affect = use_signal(|| field_affect.clone());
    use_effect(use_reactive(
        (&accepted_value, &draft(), &props.owner, &field_affect),
        move |(value, current_draft, owner, affect)| {
            if *observed_owner.peek() != owner || *observed_affect.peek() != affect {
                observed_owner.set(owner);
                observed_affect.set(affect);
                draft_effect.set(display(value));
                baseline_effect.set(value);
                dirty_effect.set(false);
                error_effect.set(None);
                return;
            }
            if current_draft.is_empty() && !(*dirty_effect.peek()) {
                draft_effect.set(display(value));
                if *baseline_effect.peek() != value {
                    baseline_effect.set(value);
                }
                return;
            }
            // PendingEditSignals restores the accepted value only while the current text
            // still equals the submitted text. This effect handles unrelated accepted edits.
            if *baseline_effect.peek() != value && !(*dirty_effect.peek()) {
                draft_effect.set(display(value));
                baseline_effect.set(value);
                error_effect.set(None);
            } else if current_draft == value.to_string() {
                if current_draft != display(value) {
                    draft_effect.set(display(value));
                }
                if *baseline_effect.peek() != value {
                    baseline_effect.set(value);
                }
                if *dirty_effect.peek() {
                    dirty_effect.set(false);
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
        let mut sequence = sequence;
        move || {
            // Formatting an accepted value must never turn focus/blur into a rounded edit.
            if !editable || !dirty() {
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
            if next == accepted_value {
                draft.set(display(accepted_value));
                baseline.set(accepted_value);
                dirty.set(false);
                error.set(None);
                return;
            }
            let Some(id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Transform request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(id);
            let submitted_text = draft.peek().clone();
            let request = MatrixTransformRequest {
                owner: owner.clone(),
                request_id: id,
                snapshot_token: token,
                revision,
                field,
                baseline: MatrixTransformValue::Number(accepted_baseline),
                value: MatrixTransformValue::Number(next),
                splay_affect: affect(),
                draft: Some(draft),
                failure: Some(error),
                submitted_text: Some(submitted_text),
                one_shot: false,
            };
            dirty.set(true);
            error.set(None);
            on_edit.call(request);
        }
    };
    let current = draft();
    let error_message = error();
    let badge = props.badge.clone();
    rsx! {
        BoardNumberField {
            class: if props.badge.is_some() { "m1-matrix-field m1-matrix-field-default" } else { "m1-matrix-field" },
            label: props.label,
            unit: props.unit,
            value: current,
            readonly: !props.editable,
            disabled: props.disabled,
            error: error_message,
            oninput: move |event: FormEvent| {
                if !dirty() {
                    baseline.set(accepted_value);
                }
                draft.set(event.value());
                dirty.set(true);
                error.set(None);
            },
            onblur: { let mut commit = commit.clone(); move |_| commit() },
            onkeydown: {
                let mut commit = commit.clone();
                move |event: KeyboardEvent| match event.data().key() {
                    Key::Enter => { event.prevent_default(); commit(); }
                    Key::Escape => {
                        event.prevent_default();
                        draft.set(display(accepted_value));
                        baseline.set(accepted_value);
                        dirty.set(false);
                        error.set(None);
                    }
                    _ => {}
                }
            },
        }
        if let Some(badge) = badge {
            small { class: "m1-pitch-badge", "{badge}" }
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
        .filter(|item| item.message.is_some())
        .and_then(|item| item.message.clone());
    let request = props.request_sequence;
    let on_edit = props.on_edit;
    let owner = props.owner.clone();
    let value = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let affect = props.splay_affect;
    rsx! {
        BoardSelectField {
                class: "m1-matrix-field", label: "Mirror", error: message,
                aria_label: "Mirror matrix",
                value: mirror_value(value),
                disabled: !props.editable,
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
                        draft: None, failure: None, submitted_text: None,
            one_shot: false,
                    });
                },
                option { value: "none", "None" }
                option { value: "x", "X axis" }
                option { value: "y", disabled: props.mirror_y_locked, "Y axis" }
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
                && item.message.is_some()
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
        BoardSelectField {
                class: "m1-matrix-field", label: "Origin", error,
                aria_label: "Splay origin",
                value: if custom { "custom" } else { "base" },
                disabled: !props.editable,
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
                        draft: None, failure: None, submitted_text: None,
            one_shot: false,
                    });
                },
                option { value: "base", "Column base" }
                option { value: "custom", "Custom point" }
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
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let current = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    rsx! {
        label { class: "m1-matrix-key-enabled",
            BoardCheckbox {
                aria_label: "Key enabled", checked: current, disabled,
                onchange: move |event: FormEvent| {
                    let Some(id) = sequence().checked_add(1) else { return; };
                    sequence.set(id);
                    submitted.set(Some(id));
                    on_edit.call(MatrixTransformRequest {
                        owner: owner.clone(), request_id: id, snapshot_token: token, revision,
                        field: MatrixTransformField::KeyEnabled,
                        baseline: MatrixTransformValue::Bool(current),
                        value: MatrixTransformValue::Bool(event.checked()),
                        splay_affect: affect(),
                        draft: None, failure: None, submitted_text: None,
            one_shot: false,
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
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
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
                        draft: None, failure: None, submitted_text: None,
            one_shot: false,
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
    remove_disabled: Signal<bool>,
    mirror_reset_disabled: Signal<bool>,
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
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable;
    let remove_disabled = disabled || (props.remove_disabled)();
    let sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let current = props.value.clone();
    let send = move |next: Vec<(String, String)>, one_shot: bool| {
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
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot,
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
                    pending_disabled: props.mirror_reset_disabled,
                    baseline: MatrixTransformValue::Bool(true),
                    value: MatrixTransformValue::Bool(false),
                    request_sequence: props.request_sequence, editable: props.editable,
                    feedback: props.feedback.clone(),
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
                                    false,
                                );
                            }
                        },
                        for (id, label) in choices.iter() {
                            option { key: "{id}", value: "{id}", selected: *id == definition_id, "{label}" }
                        }
                    }
                    BoardButton {
                        r#type: "button", class: "m1-inspector-secondary", disabled: remove_disabled,
                        aria_label: "Remove {assembly_id}",
                        onclick: {
                            let assembly_id = assembly_id.clone();
                            let all = props.value.clone();
                            let mut send = send.clone();
                            move |_| send(all.iter().filter(|(id, _)| *id != assembly_id).cloned().collect(), true)
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
    pending_disabled: Signal<bool>,
    feedback: Vec<MatrixTransformFeedback>,
    splay_affect: Signal<MatrixSplayAffect>,
    on_edit: EventHandler<MatrixTransformRequest>,
    /// Why the action is unavailable; disables the button and is shown beside it.
    #[props(default)]
    blocked: Option<String>,
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
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let disabled = !props.editable || (props.pending_disabled)() || props.blocked.is_some();
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
        BoardButton {
            r#type: "button", class: "m1-inspector-secondary", disabled,
            onclick: move |_| {
                let Some(id) = sequence().checked_add(1) else { return; };
                sequence.set(id);
                submitted.set(Some(id));
                on_edit.call(MatrixTransformRequest::one_shot(
                    owner.clone(), (token, revision), id,
                    (field, baseline.clone(), value.clone()), affect(),
                ));
            },
            "{props.label}"
        }
        if let Some(reason) = props.blocked.as_deref() { small { class: "m1-empty-note m1-matrix-transform-unavailable", "{reason}" } }
        if let Some(message) = error.as_deref() { small { role: "alert", class: "m1-matrix-transform-error", "{message}" } }
    }
}

/// Parse Core's detach proposal ("Detaching {m} would move its linked partner
/// {p}; make {p} Local ...") into the required Local matrix. Anything else is
/// shown verbatim without a Confirm action.
fn detach_proposal(message: &str, matrix_id: &str) -> Option<String> {
    // The transform layer prefixes Core refusals ("The transform change was
    // refused: ..."), so the proposal marker is searched, not anchored.
    let marker = format!("Detaching {matrix_id} would move its linked partner ");
    let at = message.find(&marker)?;
    let (partner, tail) = message[at + marker.len()..].split_once(';')?;
    if tail.trim_start() == format!("make {partner} Local to keep its pose and confirm the detach")
    {
        Some(partner.to_owned())
    } else {
        None
    }
}

#[derive(Props, Clone, PartialEq)]
struct DetachAnchorButtonProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    request_sequence: Signal<u64>,
    editable: bool,
    detach_pending: Signal<bool>,
    confirm_pending: Signal<bool>,
    feedback: Vec<MatrixTransformFeedback>,
    splay_affect: Signal<MatrixSplayAffect>,
    on_edit: EventHandler<MatrixTransformRequest>,
}

/// Detach an anchored matrix. A plain click clears the anchor in one atomic
/// Core intent. When Core refuses because the detach would move the linked
/// partner, the refusal names the required Local exception and the button
/// offers Confirm/Cancel in place: Confirm applies the detach and the
/// exception atomically, Cancel leaves the whole edit unchanged. A stale
/// confirmation is refused by Core and surfaces as an error, never a silent
/// change.
#[component]
fn DetachAnchorButton(props: DetachAnchorButtonProps) -> Element {
    let mut submitted = use_signal(|| None::<(MatrixTransformField, u64)>);
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let matrix_id = props.owner.matrix_id.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let mut send = move |field: MatrixTransformField,
                         baseline: MatrixTransformValue,
                         value: MatrixTransformValue| {
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some((field, id)));
        on_edit.call(MatrixTransformRequest::one_shot(
            owner.clone(),
            (token, revision),
            id,
            (field, baseline, value),
            affect(),
        ));
    };
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            let Some((field, request_id)) = submitted() else {
                return false;
            };
            item.owner == props.owner
                && item.field == field
                && request_id == item.request_id
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let proposal = error
        .as_deref()
        .and_then(|message| detach_proposal(message, &matrix_id));
    let busy = (props.detach_pending)() || (props.confirm_pending)();
    rsx! {
        if let Some(partner) = proposal.as_deref() {
            div { class: "m1-matrix-detach-confirm",
                p { class: "m1-matrix-transform-error", role: "alert", "{error.as_deref().unwrap_or_default()}" }
                p { class: "m1-empty-note",
                    "Confirming clears both anchors and keeps every pose in one Undo entry."
                }
                BoardButton {
                    r#type: "button", class: "m1-inspector-secondary",
                    disabled: !props.editable || busy,
                    onclick: {
                        let partner = partner.to_owned();
                        let mut send = send.clone();
                        move |_| send(
                            MatrixTransformField::MatrixDetachConfirm,
                            MatrixTransformValue::Text(String::new()),
                            MatrixTransformValue::Text(partner.clone()),
                        )
                    },
                    "Confirm detach"
                }
                BoardButton {
                    r#type: "button", class: "m1-inspector-secondary",
                    onclick: move |_| submitted.set(None),
                    "Cancel"
                }
            }
        } else {
            BoardButton {
                r#type: "button", class: "m1-inspector-secondary",
                disabled: !props.editable || busy,
                onclick: move |_| send(
                    MatrixTransformField::MatrixDetach,
                    MatrixTransformValue::Bool(false),
                    MatrixTransformValue::Bool(true),
                ),
                "Detach"
            }
            if let Some(message) = error.as_deref() {
                p { role: "alert", class: "m1-matrix-transform-error", "{message}" }
            }
        }
    }
}

/// Retarget the anchor by picking a key on the canvas. Arming the pick is a
/// presentation concern (the Inspector only fires `on_pick_anchor`); the
/// canvas hit funnels into `MatrixTransformInspectorMount::pick_matrix_anchor`
/// with the picked key identity and Core validates the target.
#[derive(Props, Clone, PartialEq)]
struct PickAnchorButtonProps {
    editable: bool,
    pending: bool,
    on_pick_anchor: EventHandler<()>,
    on_cancel_anchor_pick: EventHandler<()>,
}

#[component]
fn PickAnchorButton(props: PickAnchorButtonProps) -> Element {
    rsx! {
        if props.pending {
            p { class: "m1-empty-note",
                "Click a key on the canvas to retarget the anchor."
            }
            BoardButton {
                r#type: "button", class: "m1-inspector-secondary",
                onclick: move |_| props.on_cancel_anchor_pick.call(()),
                "Cancel anchor pick"
            }
        } else {
            BoardButton {
                r#type: "button", class: "m1-inspector-secondary",
                disabled: !props.editable,
                onclick: move |_| props.on_pick_anchor.call(()),
                "Pick anchor"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct AggregateAnchorSectionProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    anchor: MatrixAnchor,
    request_sequence: Signal<u64>,
    editable: bool,
    feedback: Vec<MatrixTransformFeedback>,
    splay_affect: Signal<MatrixSplayAffect>,
    on_edit: EventHandler<MatrixTransformRequest>,
    on_navigate_anchor_part: EventHandler<TreeContext>,
}

/// The aggregate arm of the Anchor section: the Core-admitted method over N
/// single-key parts, displayed from Core-published values only. Each part
/// renders as a link that navigates to the part key without writing; the
/// revision and history stay unchanged.
#[component]
fn AggregateAnchorSection(props: AggregateAnchorSectionProps) -> Element {
    let Some(aggregate) = props.anchor.aggregate.as_ref() else {
        return rsx! {};
    };
    let method = match aggregate.method {
        AggregateMethod::Average => "Average",
        AggregateMethod::Intersect => "Intersection",
    };
    let count = aggregate.parts.len();
    let noun = if count == 1 { "key" } else { "keys" };
    let current_method = aggregate.method;
    let is_average = matches!(current_method, AggregateMethod::Average);
    let mut submitted = use_signal(|| None::<u64>);
    let mut submitted_effect = submitted;
    let observed_method = use_signal(|| current_method);
    let mut observed_effect = observed_method;
    use_effect(use_reactive((&current_method,), move |(method,)| {
        if observed_effect() != method {
            observed_effect.set(method);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            props.owner == item.owner
                && item.field == MatrixTransformField::MatrixAnchorMethod
                && Some(item.request_id) == submitted()
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let parts: Vec<(TreeContext, String, String)> = aggregate
        .parts
        .iter()
        .map(|part| {
            (
                TreeContext::Key {
                    matrix_id: part.matrix_id.clone(),
                    row: part.row,
                    column: part.column,
                },
                format!("{} · row {} · column {}", part.matrix_id, part.row, part.column),
                format!("{}-{}-{}", part.matrix_id, part.row, part.column),
            )
        })
        .collect();
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let editable = props.editable;
    rsx! {
        p { class: "m1-matrix-anchor-target",
            "{method} of {count} {noun}"
        }
        BoardSelectField {
            label: "Method",
            class: "m1-matrix-field",
            aria_label: "Method",
            value: if is_average { "average".to_owned() } else { "intersect".to_owned() },
            disabled: !editable,
            onchange: move |event: FormEvent| {
                let next = if event.value() == "intersect" {
                    AggregateMethod::Intersect
                } else {
                    AggregateMethod::Average
                };
                if next == current_method {
                    return;
                }
                let Some(request_id) = sequence().checked_add(1) else {
                    return;
                };
                sequence.set(request_id);
                submitted.set(Some(request_id));
                on_edit.call(MatrixTransformRequest {
                    owner: owner.clone(),
                    request_id,
                    snapshot_token: token,
                    revision,
                    field: MatrixTransformField::MatrixAnchorMethod,
                    baseline: MatrixTransformValue::AnchorMethod(current_method),
                    value: MatrixTransformValue::AnchorMethod(next),
                    splay_affect: affect(),
                    draft: None,
                    failure: None,
                    submitted_text: None,
                    one_shot: false,
                });
            },
            option { value: "average", selected: is_average, "Average" }
            option { value: "intersect", selected: !is_average, "Intersection" }
        }
        if let Some(message) = error.as_deref() {
            p { role: "alert", class: "m1-matrix-transform-error", "{message}" }
        }
        ul { class: "m1-matrix-anchor-parts",
            for (context, label, key) in parts.iter() {
                li { key: "{key}",
                    button {
                        r#type: "button",
                        class: "m1-inspector-secondary m1-matrix-anchor-part",
                        onclick: {
                            let context = context.clone();
                            let on_navigate = props.on_navigate_anchor_part;
                            move |_| on_navigate.call(context.clone())
                        },
                        "{label}"
                    }
                }
            }
        }
    }
}

/// The Follow subset an anchor carries, in the Inspector's words.
fn follows_label(follow: &Follow) -> String {
    let mut kept = Vec::new();
    if follow.x {
        kept.push("X");
    }
    if follow.y {
        kept.push("Y");
    }
    if follow.rotation {
        kept.push("rotation");
    }
    match kept.as_slice() {
        [] => "Follows nothing".to_owned(),
        [only] => format!("Follows {only}"),
        [first, second] => format!("Follows {first} and {second}"),
        _ => format!("Follows {}, {} and {}", kept[0], kept[1], kept[2]),
    }
}

#[derive(Props, Clone, PartialEq)]
struct FollowTransformFieldProps {
    owner: MatrixTransformInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    anchor: MatrixAnchor,
    request_sequence: Signal<u64>,
    editable: bool,
    feedback: Vec<MatrixTransformFeedback>,
    splay_affect: Signal<MatrixSplayAffect>,
    on_edit: EventHandler<MatrixTransformRequest>,
}

/// The anchor's Follow subset as three toggles. Each toggle submits its own
/// field in one atomic Core intent; a newer accepted anchor retires the
/// pending error display.
#[component]
fn FollowTransformField(props: FollowTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<(MatrixTransformField, u64)>);
    let current_follow = props.anchor.follow;
    let observed_follow = use_signal(|| current_follow);
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_follow;
    use_effect(use_reactive((&current_follow,), move |(follow,)| {
        if observed_effect() != follow {
            observed_effect.set(follow);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            let Some((field, request_id)) = submitted() else {
                return false;
            };
            item.owner == props.owner
                && item.field == field
                && request_id == item.request_id
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let mut sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let send = move |field: MatrixTransformField, baseline: bool, value: bool| {
        if baseline == value {
            return;
        }
        let Some(id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(id);
        submitted.set(Some((field, id)));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field,
            baseline: MatrixTransformValue::Bool(baseline),
            value: MatrixTransformValue::Bool(value),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    };
    let toggle = move |(field, label, current): (MatrixTransformField, &'static str, bool)| {
        let mut send = send.clone();
        rsx! {
            label {
                BoardCheckbox {
                    checked: current,
                    disabled: !props.editable,
                    aria_label: label,
                    onchange: move |_| send(field, current, !current),
                }
                "{label}"
            }
        }
    };
    rsx! {
        div { class: "m1-matrix-follow",
            {toggle((MatrixTransformField::MatrixFollowX, "Follow X", current_follow.x))}
            {toggle((MatrixTransformField::MatrixFollowY, "Follow Y", current_follow.y))}
            {toggle((MatrixTransformField::MatrixFollowRotation, "Follow rotation", current_follow.rotation))}
        }
        if let Some(message) = error.as_deref() {
            p { role: "alert", class: "m1-matrix-transform-error", "{message}" }
        }
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
