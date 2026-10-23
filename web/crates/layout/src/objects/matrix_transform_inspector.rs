//! Private form for matrix, row, column, and cell-local transform properties.
use crate::matrix_transform_operation::{
    MatrixTransformField, MatrixTransformFields, MatrixTransformValue,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{MatrixSplayAffect, Mirror, Vec2};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::checkbox::{Checkbox, CheckboxState};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use boardstudio_web_ui_shared::ui::label::Label;
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
    let affect_choice = use_memo(move || {
        Some(
            if splay_affect() == MatrixSplayAffect::Column {
                "column"
            } else {
                "following"
            }
            .to_owned(),
        )
    });
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
                        feedback: props.mount.feedback.clone(),
                        on_edit,
                        splay_affect,
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
            },
        ),
        MatrixTransformFields::Row { row, offset } => (
            "Position & rotation",
            rsx! {
                div { class: "m1-matrix-inspector-fields",
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
            convert,
        } => (
            "Column properties",
            rsx! {
                        Collapsible {
                            class: "m1-matrix-transform-section",
                            aria_label: "Position and rotation",
                            keep_mounted: true,
                            open: Some(transform_open()),
                            on_open_change: move |next| transform_open.set(next),
                            CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Position and rotation",  "Transform"   ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                            CollapsibleContent {
                                class: "data-[open=true]:overflow-visible",
                                hidden: !transform_open(),
                                inert: (!transform_open()).then_some(""),
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
                                div { class: "m1-matrix-inspector-fields",
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
                                NumericTransformField {
                                    owner: owner.clone(),
                                    snapshot_token,
                                    revision,
                                    field: MatrixTransformField::ColumnSplay,
                                    label: "Splay",
                                    unit: "°",
                                    draft: props.mount.numeric_field(MatrixTransformField::ColumnSplay).0,
                                    failure: props.mount.numeric_field(MatrixTransformField::ColumnSplay).1,
                                    value: *splay_angle,
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

                                        Field { class: "m1-matrix-field",
                                            FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                            Select::<String> {
                                                id: control_id,
                                                aria_label: control_aria_label.clone(),
                                                value: Some(affect_choice.into()),
                                                disabled: !props.mount.editable,
                                                on_value_change,
                                                for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                    SelectOption::<String> {
                                                        index,
                                                        value: value.clone(),
                                                        text_value: label.clone(),
                                                        disabled: *disabled,
                                                        "{label}"
                                                    }
                                                }
                                            }
                                        }
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
                                span { class: "m1-matrix-field-context", {format!("Column {}", column + 1)} }
                            }
                         }
                        Collapsible {
                            class: "m1-matrix-transform-section",
                            aria_label: "Splay and origin",
                            keep_mounted: true,
                            open: Some(origin_open()),
                            on_open_change: move |next| origin_open.set(next),
                            CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", aria_label: "Splay and origin",  "Origin & scope"   ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                            CollapsibleContent {
                                hidden: !origin_open(),
                                inert: (!origin_open()).then_some(""),
                                div { class: "m1-matrix-inspector-fields",
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
                                    class: "m1-secondary",
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
        } => (
            "Key properties",
            rsx! {
                Collapsible {
                    class: "m1-matrix-transform-section",
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
                        div { class: "m1-matrix-inspector-fields",
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
                    class: "m1-matrix-transform-section",
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
        section {
            key: "{owner_key}",
            class: "m1-matrix-inspector m1-matrix-transform-inspector",
            aria_label: "Matrix transform properties",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() {
                        raw.stop_propagation();
                    }
                }
            },
            header { class: "m1-matrix-inspector-heading",
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
                    class: "m1-matrix-transform-section",
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
            div { class: "m1-selection-delete",
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
            class: "m1-canvas-delete",
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
    #[props(default)]
    display_precision: Option<usize>,
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
    use_effect(use_reactive(
        (&accepted_value, &draft(), &props.owner),
        move |(value, current_draft, owner)| {
            if *observed_owner.peek() != owner {
                observed_owner.set(owner);
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
    rsx! {
        {
            let input_id = format!("layout-number-{}", dioxus::core::current_scope_id().0);
            let unit_id = format!("{input_id}-unit");
            let error_id = format!("{input_id}-error");
            let described_by = match (props.unit.is_empty(), error_message.is_some()) {
                (false, true) => Some(format!("{unit_id} {error_id}")),
                (false, false) => Some(unit_id.clone()),
                (true, true) => Some(error_id.clone()),
                (true, false) => None,
            };
            rsx! {
                Field { class: "m1-matrix-field", invalid: error_message.is_some(),
                    FieldLabel { html_for: input_id.clone(), "{props.label}" }
                    InputGroup { class: "max-[760px]:h-control-touch",
                        InputGroupInput { class: "max-[760px]:h-control-touch",
                            id: input_id,
                            r#type: InputType::Number,
                            step: "any",
                            value: current,
                            readonly: !props.editable,
                            aria_label: props.label,
                            aria_invalid: error_message.is_some(),
                            aria_describedby: described_by,
                            oninput: move |event: FormEvent| {
                                if !dirty() {
                                    baseline.set(accepted_value);
                                }
                                draft.set(event.value());
                                dirty.set(true);
                                error.set(None);
                            },
                            attributes: vec![
                                dioxus::events::onblur({
                                    let mut commit = commit.clone();
                                    move |_| commit()
                                }),
                                dioxus::events::onkeydown({
                                    let mut commit = commit.clone();
                                    move |event: KeyboardEvent| match event.data().key() {
                                        Key::Enter => {
                                            event.prevent_default();
                                            commit();
                                        }
                                        Key::Escape => {
                                            event.prevent_default();
                                            draft.set(display(accepted_value));
                                            baseline.set(accepted_value);
                                            dirty.set(false);
                                            error.set(None);
                                        }
                                        _ => {}
                                    }
                                }),
                            ],
                        }
                        if !props.unit.is_empty() {
                            InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                                InputGroupText {
                                    span { id: unit_id, "{props.unit}" }
                                }
                            }
                        }
                    }
                    if let Some(message) = error_message {
                        div { id: error_id,
                            FieldError { "{message}" }
                        }
                    }
                }
            }
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
    let mirror_choice = use_memo(use_reactive((&props.value,), |(value,)| {
        Some(mirror_value(value).to_owned())
    }));
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
        {
            let control_label = "Mirror".to_owned();
            let control_aria_label = "Mirror matrix".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let error: Option<String> = message;
            let error_id = format!("{control_id}-error");
            let control_choices = [
                ("none".to_owned(), "None".to_owned(), false),
                ("x".to_owned(), "X axis".to_owned(), false),
                ("y".to_owned(), "Y axis".to_owned(), props.mirror_y_locked),
            ];
            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        let next = match choice.as_str() {
            "x" => Some(Mirror::X),
            "y" => Some(Mirror::Y),
            _ => None,
        };
        if next == value || (next == Some(Mirror::Y) && props.mirror_y_locked) {
            return;
        }
        let mut request = request;
        let Some(id) = request().checked_add(1) else {
            return;
        };
        request.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::MatrixMirror,
            baseline: MatrixTransformValue::Mirror(value),
            value: MatrixTransformValue::Mirror(next),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {

                Field { class: "m1-matrix-field", invalid: error.is_some(),
                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                    Select::<String> {
                        id: control_id,
                        aria_label: control_aria_label.clone(),
                        aria_invalid: error.is_some(),
                        aria_describedby: error.as_ref().map(|_| error_id.clone()),
                        value: Some(mirror_choice.into()),
                        disabled: !props.editable,
                        on_value_change,
                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                            SelectOption::<String> {
                                index,
                                value: value.clone(),
                                text_value: label.clone(),
                                disabled: *disabled,
                                "{label}"
                            }
                        }
                    }
                    if let Some(message) = error {
                        div { id: error_id,
                            FieldError { "{message}" }
                        }
                    }
                }
            }
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
    let origin_choice = use_memo(use_reactive((&props.custom,), |(custom,)| {
        Some(if custom { "custom" } else { "base" }.to_owned())
    }));
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
        {
            let control_label = "Origin".to_owned();
            let control_aria_label = "Splay origin".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let error: Option<String> = error;
            let error_id = format!("{control_id}-error");
            let control_choices = [
                ("base".to_owned(), "Column base".to_owned(), false),
                ("custom".to_owned(), "Custom point".to_owned(), false),
            ];
            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        let next = choice == "custom";
        if next == custom {
            return;
        }
        let mut request = request;
        let Some(id) = request().checked_add(1) else {
            return;
        };
        request.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::SplayOriginMode,
            baseline: MatrixTransformValue::OriginMode(custom),
            value: MatrixTransformValue::OriginMode(next),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {

                Field { class: "m1-matrix-field", invalid: error.is_some(),
                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                    Select::<String> {
                        id: control_id,
                        aria_label: control_aria_label.clone(),
                        aria_invalid: error.is_some(),
                        aria_describedby: error.as_ref().map(|_| error_id.clone()),
                        value: Some(origin_choice.into()),
                        disabled: !props.editable,
                        on_value_change,
                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                            SelectOption::<String> {
                                index,
                                value: value.clone(),
                                text_value: label.clone(),
                                disabled: *disabled,
                                "{label}"
                            }
                        }
                    }
                    if let Some(message) = error {
                        div { id: error_id,
                            FieldError { "{message}" }
                        }
                    }
                }
            }
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
        {
            let check_id = format!(
                "layout-check-{}-{}",
                dioxus::core::current_scope_id().0,
                ("Key enabled").replace(' ', "-"),
            );
            rsx! {
                Label { html_for: check_id.clone(), class: "m1-matrix-key-enabled",
                    Checkbox {
                        id: check_id.clone(),
                        aria_label: "Key enabled",
                        disabled,
                        checked: Some(
                            if current {
                                CheckboxState::Checked
                            } else {
                                CheckboxState::Unchecked
                            },
                        ),
                        on_checked_change: move |checked: CheckboxState| {
                            let Some(id) = sequence().checked_add(1) else {
                                return;
                            };
                            sequence.set(id);
                            submitted.set(Some(id));
                            on_edit
                                .call(MatrixTransformRequest {
                                    owner: owner.clone(),
                                    request_id: id,
                                    snapshot_token: token,
                                    revision,
                                    field: MatrixTransformField::KeyEnabled,
                                    baseline: MatrixTransformValue::Bool(current),
                                    value: MatrixTransformValue::Bool(bool::from(checked)),
                                    splay_affect: affect(),
                                    draft: None,
                                    failure: None,
                                    submitted_text: None,
                                    one_shot: false,
                                });
                        },
                    }
                    " Enabled"
                }
            }
        }
        if let Some(message) = error.as_deref() {
            FieldError { class: "m1-matrix-transform-error", "{message}"  }
        }
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
    let assembly_choice = use_memo(use_reactive((&props.value,), |(value,)| Some(value)));
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
        {
            let control_label = "Key Assembly".to_owned();
            let control_aria_label = "Key Assembly".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let mut control_choices: Vec<(String, String, bool)> = Vec::new();
            for (id, label) in props.choices.iter() {
                control_choices.push((id.to_string(), label.to_string(), false));
            }

            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
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
            field: MatrixTransformField::KeyAssembly,
            baseline: MatrixTransformValue::Text(current.clone()),
            value: MatrixTransformValue::Text(choice),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {

                Field { class: "m1-matrix-key-assembly",
                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                    Combobox::<String> {
                        id: control_id,
                        aria_label: control_aria_label.clone(),
                        list_aria_label: control_aria_label.clone(),
                        value: Some(assembly_choice.into()),
                        disabled,
                        on_value_change,
                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                            ComboboxOption::<String> {
                                index,
                                value: value.clone(),
                                text_value: label.clone(),
                                disabled: *disabled,
                                "{label}"
                            }
                        }
                    }
                }
            }
        }
        if let Some(message) = error.as_deref() {
            FieldError { class: "m1-matrix-transform-error", "{message}"  }
        }
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
    let attachment_choices = use_memo(use_reactive((&props.value,), |(values,)| {
        values
            .into_iter()
            .map(|(assembly, definition)| (assembly, Some(definition)))
            .collect::<std::collections::BTreeMap<_, _>>()
    }));
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
                        label: "Use mirrored components",
                        owner: props.owner.clone(),
                        snapshot_token: props.snapshot_token,
                        revision: props.revision,
                        field: MatrixTransformField::KeyAssembliesLocal,
                        pending_disabled: props.mirror_reset_disabled,
                        baseline: MatrixTransformValue::Bool(true),
                        value: MatrixTransformValue::Bool(false),
                        request_sequence: props.request_sequence,
                        editable: props.editable,
                        feedback: props.feedback.clone(),
                        splay_affect: props.splay_affect,
                        on_edit: props.on_edit,
                    }
                } else {
                    Callout { variant: CalloutVariant::Info,
                        "The key assembly and attached components follow the paired half. Replacing one here keeps this key local."
                    }
                }
            }
            if props.value.is_empty() {
                Callout { variant: CalloutVariant::Info, "No attached components. Apply a component in Parts." }
            }
            for (assembly_id, definition_id) in props.value.iter().cloned() {
                {
                    let choices = crate::matrix_transform_operation::attachment_component_choices(
                        &props.choices,
                        &definition_id,
                    );
                    rsx! {
                        div { key: "{assembly_id}", class: "m1-matrix-attached-row",
                            {
                                let control_label = format!("Replace {}", assembly_id);
                                let control_aria_label = format!("Replace {}", assembly_id);
                                let control_id = format!(
                                    "layout-choice-{}-{}",
                                    dioxus::core::current_scope_id().0,
                                    control_aria_label.replace(' ', "-"),
                                );

                                let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                                for (id, label) in choices.iter() {
                                    control_choices.push((id.to_string(), label.to_string(), false));
                                }

                                let on_value_change = EventHandler::new({
            let assembly_id = assembly_id.clone();
            let all = props.value.clone();
            let mut send = send.clone();
            move |choice: Option<String>| {
                let Some(choice) = choice else {
                    return;
                };
                send(
                    all.iter()
                        .map(|(id, def)| {
                            if *id == assembly_id {
                                (id.clone(), choice.clone())
                            } else {
                                (id.clone(), def.clone())
                            }
                        })
                        .collect(),
                    false,
                );
            }
        });
        rsx! {

                                    Field { class: "",
                                        FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                        Combobox::<String> {
                                            id: control_id,
                                            aria_label: control_aria_label.clone(),
                                            list_aria_label: control_aria_label.clone(),
                                            value: Some(attachment_choices.map({ let id = assembly_id.clone(); move |values| values.get(&id).unwrap_or(&None) }).into()),
                                            disabled,
                                            on_value_change,
                                            for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                ComboboxOption::<String> {
                                                    index,
                                                    value: value.clone(),
                                                    text_value: label.clone(),
                                                    disabled: *disabled,
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    class: "m1-inspector-secondary",
    disabled: remove_disabled,
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
            if let Some(message) = error.as_deref() {
                FieldError { class: "m1-matrix-transform-error", "{message}"  }
            }
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
            Button { variant: ButtonVariant::Secondary,
    button_type: "button",
    class: "m1-inspector-secondary",
    disabled: disabled,
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
            if let Some(reason) = props.blocked.as_deref() {
                small { class: "m1-empty-note m1-matrix-transform-unavailable", "{reason}" }
            }
            if let Some(message) = error.as_deref() {
                FieldError { class: "m1-matrix-transform-error", "{message}"  }
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
