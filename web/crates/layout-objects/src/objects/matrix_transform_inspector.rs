//! Private form for matrix, row, column, and cell-local transform properties.
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue, column_angle,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{
    AggregateAnchor, AggregateMethod, AnchorPart, Follow, MatrixAnchor, MatrixSplayAffect, Mirror,
    Vec2,
};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::CheckboxState;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use boardstudio_web_ui_shared::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldError, FieldLabel},
    select::{Select, SelectOption},
};
use boardstudio_web_ui_shared::ui::{
    input::InputType,
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupInput, InputGroupText,
    },
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;

use super::TreeContext;
use super::matrix_transform_controller::MatrixTransformInspectorMount;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "matrix_transform_inspector_tests.rs"]
mod mounted_tests;

mod anchor_section;
mod fields;
mod key_section;
mod matrix_section;
mod row_column_section;

use anchor_section::{
    AggregateAnchorSection, DetachAnchorButton, FollowTransformField, PickAnchorButton,
};
use fields::{ResetTransformButton, TransformCheckRow, TransformChoiceRow};
// Shared with `layout_canvas_controls`: the numeric row stays addressable through
// the Inspector module while its definition lives in `fields`.
pub use fields::NumericTransformField;
use key_section::{
    AttachedComponentsField, EnabledTransformField, KeyAssemblyField, OmitTransformField,
};
use matrix_section::MirrorTransformField;
use row_column_section::OriginModeTransformField;

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
    let mut transform_open = use_signal(|| true);
    let mut origin_open = use_signal(|| false);
    let mut assembly_open = use_signal(|| true);

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
    let affect_selected = if splay_affect() == MatrixSplayAffect::Column {
        "column".to_owned()
    } else {
        "following".to_owned()
    };
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
                div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                    NumericTransformField {
                        owner: owner.clone(),
                        snapshot_token,
                        revision,
                        field: MatrixTransformField::OriginX,
                        label: "Origin X",
                        unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::OriginX).0,
                        failure: props.mount.numeric_field(MatrixTransformField::OriginX).1,
                        value: origin.x,
                        request_sequence,
                        editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(),
                        snapshot_token,
                        revision,
                        field: MatrixTransformField::OriginY,
                        label: "Origin Y",
                        unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::OriginY).0,
                        failure: props.mount.numeric_field(MatrixTransformField::OriginY).1,
                        value: origin.y,
                        request_sequence,
                        editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(),
                        snapshot_token,
                        revision,
                        field: MatrixTransformField::MatrixRotation,
                        label: "Rotation",
                        unit: "°",
                        draft: props.mount.numeric_field(MatrixTransformField::MatrixRotation).0,
                        failure: props.mount.numeric_field(MatrixTransformField::MatrixRotation).1,
                        value: *rotation,
                        request_sequence,
                        editable: props.mount.editable,
                        disabled: !rotation_followed,
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
                    }
                    if !rotation_followed {
                        p { class: "text-xs text-muted-foreground", role: "note",
                            "Rotation is not followed, so turning it changes nothing."
                        }
                    }
                }
                MirrorTransformField {
                    owner: owner.clone(),
                    snapshot_token,
                    revision,
                    value: *mirror,
                    mirror_y_locked: *mirror_y_locked,
                    request_sequence,
                    editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(),
                    on_edit,
                    splay_affect,
                }
                h3 { class: "text-xs font-semibold", "Anchor" }
                if let Some(anchor) = anchor {
                    if anchor.aggregate.is_some() {
                        AggregateAnchorSection {
                            owner: owner.clone(), snapshot_token, revision,
                            anchor: anchor.clone(),
                            request_sequence, editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(), splay_affect, on_edit,
                            on_navigate_anchor_part: props.on_navigate_anchor_part,
                            mount: props.mount.clone(),
                            aggregate_pick_pending: props.anchor_pick_pending,
                            on_pick_anchor: props.on_pick_anchor,
                            on_cancel_anchor_pick: props.on_cancel_anchor_pick,
                        }
                    } else {
                        p { "data-anchor-target": "true", class: "text-xs text-muted-foreground",
                            "Anchored to {anchor.matrix_id} · row {anchor.row} · column {anchor.column}"
                        }
                    }
                    p { "data-anchor-follow": "true", class: "text-xs text-muted-foreground", "{follows_label(&anchor.follow)}" }
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
                        p { role: "alert", class: "text-xs text-destructive", "{message}" }
                    }
                } else {
                    p { class: "text-xs text-muted-foreground", "No anchor. The matrix places itself." }
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
                h3 { class: "text-xs font-semibold", "Carried" }
                p { class: "text-xs text-muted-foreground", "Moves or turns this row and later rows." }
                div { class: "grid min-w-0 gap-3",
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
                        p { class: "text-xs text-muted-foreground", role: "note",
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
                h3 { class: "text-xs font-semibold", "This row only" }
                p { class: "text-xs text-muted-foreground", "Nothing else moves." }
                div { class: "grid min-w-0 gap-3",
                    NumericTransformField {
                        owner: owner.clone(),
                        snapshot_token,
                        revision,
                        field: MatrixTransformField::RowOffsetX,
                        label: "Offset X",
                        unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowOffsetX).0,
                        failure: props.mount.numeric_field(MatrixTransformField::RowOffsetX).1,
                        value: offset.x,
                        request_sequence,
                        editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
                    }
                    NumericTransformField {
                        owner: owner.clone(),
                        snapshot_token,
                        revision,
                        field: MatrixTransformField::RowOffsetY,
                        label: "Offset Y",
                        unit: "mm",
                        draft: props.mount.numeric_field(MatrixTransformField::RowOffsetY).0,
                        failure: props.mount.numeric_field(MatrixTransformField::RowOffsetY).1,
                        value: offset.y,
                        request_sequence,
                        editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
                    }
                }
                ResetTransformButton {
                    label: "Reset offsets",
                    owner: owner.clone(),
                    snapshot_token,
                    revision,
                    field: MatrixTransformField::RowOffsetReset,
                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::RowOffsetReset),
                    baseline: MatrixTransformValue::Offset(*offset),
                    value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                    request_sequence,
                    editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(),
                    splay_affect,
                    on_edit,
                }
                span { class: "m-0 text-xs leading-normal text-muted-foreground", "Row {row}" }
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
                        Collapsible {
                            class: "grid min-w-0 gap-2.5 border-t border-border pt-2.5 [&_h3]:m-0 [&_h3]:text-xs [&_h3]:font-[650] [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground",
                            aria_label: "Position and rotation",
                            keep_mounted: true,
                            open: Some(transform_open()),
                            on_open_change: move |next| transform_open.set(next),
                            CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Position and rotation",  "Transform"   ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                            CollapsibleContent {
                                class: "data-[open=true]:overflow-visible",
                                hidden: !transform_open(),
                                inert: (!transform_open()).then_some(""),
                    h3 { class: "text-xs font-semibold", "Carried" }
                    p { class: "text-xs text-muted-foreground", "Moves or turns this column and the columns after it." }
                    if *column == 0 {
                        p { class: "text-xs text-muted-foreground", role: "note",
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
                                    owner: owner.clone(),
                                    snapshot_token,
                                    revision,
                                    field: MatrixTransformField::ColumnStagger,
                                    label: "Stagger",
                                    unit: "mm",
                                    draft: props.mount.numeric_field(MatrixTransformField::ColumnStagger).0,
                                    failure: props.mount.numeric_field(MatrixTransformField::ColumnStagger).1,
                                    value: *stagger,
                                    request_sequence,
                                    editable: props.mount.editable,
                                    feedback: props.mount.feedback.clone(),
                                    on_edit,
                                    splay_affect,
                                }
                                NumericTransformField {
                                    owner: owner.clone(),
                                    snapshot_token,
                                    revision,
                                    field: MatrixTransformField::ColumnSplay,
                                    label: "Splay",
                                    unit: "°",
                                    draft: props.mount.numeric_field(MatrixTransformField::ColumnSplay).0,
                                    failure: props.mount.numeric_field(MatrixTransformField::ColumnSplay).1,
                                    value: column_angle(*splay_angle, *rotation, &splay_affect()),
                                    request_sequence,
                                    editable: props.mount.editable,
                                    feedback: props.mount.feedback.clone(),
                                    on_edit,
                                    splay_affect,
                                }
                                {
                                    let control_label = "Splay affects".to_owned();
                                    let control_aria_label = "Splay affects".to_owned();
                                    let control_id = format!(
                                        "layout-choice-{}-{}",
                                        dioxus::core::current_scope_id().0,
                                        control_aria_label.replace(' ', "-"),
                                    );

                                    let control_choices = [
                                        ("column".to_owned(), "This column".to_owned(), false),
                                        ("following".to_owned(), "This and following".to_owned(), false),
                                    ];
                                    let on_value_change = EventHandler::new(move |choice: Option<String>| {
                let Some(choice) = choice else {
                    return;
                };
                splay_affect.set(if choice == "column" {
                    MatrixSplayAffect::Column
                } else {
                    MatrixSplayAffect::Following
                });
            });
            rsx! {
                                        TransformChoiceRow {
                                            id: control_id,
                                            label: control_label,
                                            aria_label: control_aria_label,
                                            choices: control_choices.into(),
                                            selected: affect_selected,
                                            disabled: !props.mount.editable,
                                            on_value_change,
                                        }
                                    }
                                }
                                h3 { class: "text-xs font-semibold", "This column only" }
                                p { class: "text-xs text-muted-foreground", "Nothing else moves." }
                                div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                                    NumericTransformField {
                                        owner: owner.clone(),
                                        snapshot_token,
                                        revision,
                                        field: MatrixTransformField::ColumnOffsetX,
                                        label: "Offset X",
                                        unit: "mm",
                                        draft: props.mount.numeric_field(MatrixTransformField::ColumnOffsetX).0,
                                        failure: props.mount.numeric_field(MatrixTransformField::ColumnOffsetX).1,
                                        value: offset.x,
                                        request_sequence,
                                        editable: props.mount.editable,
                                        feedback: props.mount.feedback.clone(),
                                        on_edit,
                                        splay_affect,
                                    }
                                    NumericTransformField {
                                        owner: owner.clone(),
                                        snapshot_token,
                                        revision,
                                        field: MatrixTransformField::ColumnOffsetY,
                                        label: "Offset Y",
                                        unit: "mm",
                                        draft: props.mount.numeric_field(MatrixTransformField::ColumnOffsetY).0,
                                        failure: props.mount.numeric_field(MatrixTransformField::ColumnOffsetY).1,
                                        value: offset.y,
                                        request_sequence,
                                        editable: props.mount.editable,
                                        feedback: props.mount.feedback.clone(),
                                        on_edit,
                                        splay_affect,
                                    }
                                }
                                OriginModeTransformField {
                                    owner: owner.clone(),
                                    snapshot_token,
                                    revision,
                                    custom: *custom_origin,
                                    request_sequence,
                                    editable: props.mount.editable,
                                    feedback: props.mount.feedback.clone(),
                                    on_edit,
                                    splay_affect,
                                }
                                ResetTransformButton {
                                    label: "Reset offsets",
                                    owner: owner.clone(),
                                    snapshot_token,
                                    revision,
                                    field: MatrixTransformField::ColumnOffsetReset,
                                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::ColumnOffsetReset),
                                    baseline: MatrixTransformValue::Offset(*offset),
                                    value: MatrixTransformValue::Offset(Vec2 { x: 0.0, y: 0.0 }),
                                    request_sequence,
                                    editable: props.mount.editable,
                                    feedback: props.mount.feedback.clone(),
                                    splay_affect,
                                    on_edit,
                                }
                                if offset.y != 0.0 {
                                    ResetTransformButton {
                                        label: "Convert to stagger",
                                        owner: owner.clone(),
                                        snapshot_token,
                                        revision,
                                        field: MatrixTransformField::ColumnOffsetConvert,
                                        pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::ColumnOffsetConvert),
                                        baseline: MatrixTransformValue::Number(offset.y),
                                        value: MatrixTransformValue::Number(0.0),
                                        blocked: convert.clone().err(),
                                        request_sequence,
                                        editable: props.mount.editable,
                                        feedback: props.mount.feedback.clone(),
                                        splay_affect,
                                        on_edit,
                                    }
                                }
                                span { class: "m-0 text-xs leading-normal text-muted-foreground", {format!("Column {}", column + 1)} }
                            }
                         }
                        Collapsible {
                            class: "grid min-w-0 gap-2.5 border-t border-border pt-2.5 [&_h3]:m-0 [&_h3]:text-xs [&_h3]:font-[650] [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground",
                            aria_label: "Splay and origin",
                            keep_mounted: true,
                            open: Some(origin_open()),
                            on_open_change: move |next| origin_open.set(next),
                            CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Splay and origin",  "Origin & scope"   ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                            CollapsibleContent {
                                hidden: !origin_open(),
                                inert: (!origin_open()).then_some(""),
                                div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                                    NumericTransformField {
                                        owner: owner.clone(),
                                        snapshot_token,
                                        revision,
                                        field: MatrixTransformField::SplayOriginX,
                                        label: "Origin X",
                                        unit: "mm",
                                        draft: props.mount.numeric_field(MatrixTransformField::SplayOriginX).0,
                                        failure: props.mount.numeric_field(MatrixTransformField::SplayOriginX).1,
                                        value: splay_origin.x,
                                        request_sequence,
                                        editable: props.mount.editable,
                                        feedback: props.mount.feedback.clone(),
                                        on_edit,
                                        splay_affect,
                                    }
                                    NumericTransformField {
                                        owner: owner.clone(),
                                        snapshot_token,
                                        revision,
                                        field: MatrixTransformField::SplayOriginY,
                                        label: "Origin Y",
                                        unit: "mm",
                                        draft: props.mount.numeric_field(MatrixTransformField::SplayOriginY).0,
                                        failure: props.mount.numeric_field(MatrixTransformField::SplayOriginY).1,
                                        value: splay_origin.y,
                                        request_sequence,
                                        editable: props.mount.editable,
                                        feedback: props.mount.feedback.clone(),
                                        on_edit,
                                        splay_affect,
                                    }
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    button_type: "button",
                                    disabled: !props.mount.editable,
                                    onclick: move |_| props.on_pick_splay_origin.call(()),

                                    "Pick origin"

                                }
                                Callout { variant: CalloutVariant::Info, "Move the origin without moving the keys." }
                            }
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
            half_exception,
        } => (
            "Key properties",
            rsx! {
                Collapsible {
                    class: "grid min-w-0 gap-2.5 border-t border-border pt-2.5 [&_h3]:m-0 [&_h3]:text-xs [&_h3]:font-[650] [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground",
                    keep_mounted: true,
                    open: Some(transform_open()),
                    on_open_change: move |next| transform_open.set(next),
                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", "Position & rotation"  ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                    CollapsibleContent {
                        hidden: !transform_open(),
                        inert: (!transform_open()).then_some(""),
                        EnabledTransformField {
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            value: *enabled,
                            request_sequence,
                            editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(),
                            on_edit,
                            splay_affect,
                        }
                if let Some(exception) = half_exception {
                    OmitTransformField {
                        owner: owner.clone(), snapshot_token, revision, omitted: exception.omitted,
                        request_sequence, editable: props.mount.editable,
                        feedback: props.mount.feedback.clone(), on_edit, splay_affect,
                    }
                }
                h3 { class: "text-xs font-semibold", "Carried" }
                p { class: "text-xs text-muted-foreground", "Moves or turns this key and the keys above it." }
                div { class: "grid min-w-0 gap-3",
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
                        div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                            NumericTransformField {
                                owner: owner.clone(),
                                snapshot_token,
                                revision,
                                field: MatrixTransformField::KeyOffsetX,
                                label: "Local X",
                                unit: "mm",
                                draft: props.mount.numeric_field(MatrixTransformField::KeyOffsetX).0,
                                failure: props.mount.numeric_field(MatrixTransformField::KeyOffsetX).1,
                                value: offset.x,
                                request_sequence,
                                editable: props.mount.editable,
                                feedback: props.mount.feedback.clone(),
                                on_edit,
                                splay_affect,
                            }
                            NumericTransformField {
                                owner: owner.clone(),
                                snapshot_token,
                                revision,
                                field: MatrixTransformField::KeyOffsetY,
                                label: "Local Y",
                                unit: "mm",
                                draft: props.mount.numeric_field(MatrixTransformField::KeyOffsetY).0,
                                failure: props.mount.numeric_field(MatrixTransformField::KeyOffsetY).1,
                                value: offset.y,
                                request_sequence,
                                editable: props.mount.editable,
                                feedback: props.mount.feedback.clone(),
                                on_edit,
                                splay_affect,
                            }
                            NumericTransformField {
                                owner: owner.clone(),
                                snapshot_token,
                                revision,
                                field: MatrixTransformField::KeyRotation,
                                label: "Key rotation",
                                unit: "°",
                                draft: props.mount.numeric_field(MatrixTransformField::KeyRotation).0,
                                failure: props.mount.numeric_field(MatrixTransformField::KeyRotation).1,
                                value: *rotation,
                                request_sequence,
                                editable: props.mount.editable,
                                feedback: props.mount.feedback.clone(),
                                on_edit,
                                splay_affect,
                            }
                        }
                        ResetTransformButton {
                            label: "Reset local transform",
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            field: MatrixTransformField::KeyTransformReset,
                            pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyTransformReset),
                            baseline: MatrixTransformValue::CellTransform {
                                offset: *offset,
                                rotation: *rotation,
                            },
                            value: MatrixTransformValue::CellTransform {
                                offset: Vec2 { x: 0.0, y: 0.0 },
                                rotation: 0.0,
                            },
                            request_sequence,
                            editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(),
                            splay_affect,
                            on_edit,
                        }
                    }
                }
                Collapsible {
                    class: "grid min-w-0 gap-2.5 border-t border-border pt-2.5 [&_h3]:m-0 [&_h3]:text-xs [&_h3]:font-[650] [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground",
                    keep_mounted: true,
                    open: Some(assembly_open()),
                    on_open_change: move |next| assembly_open.set(next),
                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", "Key assembly"  ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                    CollapsibleContent {
                        class: "data-[open=true]:overflow-visible",
                        hidden: !assembly_open(),
                        inert: (!assembly_open()).then_some(""),
                        KeyAssemblyField {
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            value: definition_id.clone(),
                            choices: choices.clone(),
                            request_sequence,
                            editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(),
                            on_edit,
                            splay_affect,
                        }
                        AttachedComponentsField {
                            owner: owner.clone(),
                            snapshot_token,
                            revision,
                            value: assemblies.clone(),
                            remove_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyAttached),
                            mirror_reset_disabled: props.mount.one_shot_disabled(MatrixTransformField::KeyAssembliesLocal),
                            choices: component_choices.clone(),
                            mirror_target: *mirror_target,
                            assemblies_local: *assemblies_local,
                            request_sequence,
                            editable: props.mount.editable,
                            feedback: props.mount.feedback.clone(),
                            on_edit,
                            splay_affect,
                        }
                    }
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
                span { class: "m-0 text-xs leading-normal text-muted-foreground", "Key {row}, {column}" }
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
        section {
            key: "{owner_key}",
            class: "grid min-w-0 gap-3 border-t border-border px-4 pt-3 pb-4 text-foreground",
            aria_label: "Matrix transform properties",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() {
                        raw.stop_propagation();
                    }
                }
            },
            header { class: "sr-only",
                h2 { "{title}" }
                span { "{projection.label}" }
            }
            if matches!(
                projection.fields,
                MatrixTransformFields::Column { .. } | MatrixTransformFields::Key { .. }
            )
            {
                {body}
            } else {
                Collapsible {
                    class: "grid min-w-0 gap-2.5 border-t border-border pt-2.5 [&_h3]:m-0 [&_h3]:text-xs [&_h3]:font-[650] [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground",
                    keep_mounted: true,
                    open: Some(transform_open()),
                    on_open_change: move |next| transform_open.set(next),
                    CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", "{title}"  ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                    CollapsibleContent {
                        class: "data-[open=true]:overflow-visible",
                        hidden: !transform_open(),
                        inert: (!transform_open()).then_some(""),
                        {body}
                    }
                }
            }
            div { class: "border-t border-border pt-3",
                ResetTransformButton {
                    label: delete_label,
                    owner: owner.clone(),
                    snapshot_token,
                    revision,
                    field: MatrixTransformField::DeleteSelection,
                    pending_disabled: props.mount.one_shot_disabled(MatrixTransformField::DeleteSelection),
                    baseline: MatrixTransformValue::Bool(false),
                    value: MatrixTransformValue::Bool(true),
                    request_sequence,
                    editable: props.mount.editable,
                    feedback: props.mount.feedback.clone(),
                    splay_affect,
                    on_edit,
                }
            }
            if let Some(announcement) = (props.mount.change_status)().filter(|announcement| announcement.owner == projection.owner) {
                p { "data-matrix-change-status": "true", class: "text-xs text-muted-foreground", role: "status", "{announcement.message}" }
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
    rsx! {
        div {
            class: "[&_button]:min-h-8 [&_button]:rounded-md [&_button]:border [&_button]:border-border-strong [&_button]:bg-card [&_button]:text-xs",
            onpointerdown: move |event| event.stop_propagation(),
            onkeydown: move |event: KeyboardEvent| {
                event.stop_propagation();
                if event.key().to_string() == " "
                    && let Some(raw) = event.data().try_as_web_event()
                {
                    raw.stop_propagation();
                }
            },
            ResetTransformButton {
                label,
                owner: projection.owner.clone(),
                snapshot_token: projection.snapshot_token,
                revision: projection.revision,
                field: MatrixTransformField::DeleteSelection,
                pending_disabled: mount.one_shot_disabled(MatrixTransformField::DeleteSelection),
                baseline: MatrixTransformValue::Bool(false),
                value: MatrixTransformValue::Bool(true),
                request_sequence: mount.request_sequence,
                editable: mount.editable,
                feedback: mount.feedback.clone(),
                splay_affect: mount.splay_affect,
                on_edit: mount.on_edit,
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
