use super::TreeContext;
use super::matrix_transform_controller::MatrixTransformInspectorMount;
use crate::matrix_transform_operation::{
    MatrixTransformField as Field, MatrixTransformFields, MatrixTransformValue as Value,
    attachment_component_choices, column_angle,
};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{AggregateMethod, MatrixAnchor, MatrixSplayAffect, Mirror, Vec2};
use boardstudio_web_ui_model::state::{FieldPreview, FieldPreviewSource, FieldPreviewState};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, ToggleField, format_numeric,
};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use dioxus::prelude::*;

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
    pub field: Field,
    pub message: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixTransformRequest {
    pub owner: MatrixTransformInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: Field,
    pub baseline: Value,
    pub value: Value,
    pub splay_affect: MatrixSplayAffect,
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

pub(super) fn submit(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    baseline: Value,
    value: Value,
) -> Option<u64> {
    let projection = mount.projection.as_ref().filter(|_| mount.editable)?;
    let mut sequence = mount.request_sequence;
    let request_id = sequence().checked_add(1)?;
    sequence.set(request_id);
    mount.on_edit.call(MatrixTransformRequest {
        owner: projection.owner.clone(),
        request_id,
        snapshot_token: projection.snapshot_token,
        revision: projection.revision,
        field,
        baseline,
        value,
        splay_affect: (mount.splay_affect)(),
    });
    Some(request_id)
}

pub(super) fn number(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &'static str,
    unit: &'static str,
    value: f64,
    available: bool,
) -> Element {
    rsx! {
        TransformNumber { key: "{field:?}", mount: mount.clone(), field, label, unit, value, available }
    }
}

#[component]
fn TransformNumber(
    mount: MatrixTransformInspectorMount,
    field: Field,
    label: &'static str,
    unit: &'static str,
    value: f64,
    available: bool,
) -> Element {
    let preview = try_use_context::<FieldPreview>();
    let source = try_use_context::<Option<FieldPreviewSource>>().flatten();
    let context = mount
        .projection
        .as_ref()
        .map(|projection| projection.owner.context.clone());
    let mut shown = use_signal(Vec::<String>::new);
    let affect = mount.splay_affect;
    let show = move || {
        let (Some(preview), Some(source), Some(context)) = (preview, &source, &context) else {
            return;
        };
        if let Some(facts) = crate::matrix_relations::field_preview(
            &source.document,
            &source.scene.matrix_scenes,
            context,
            field,
            affect(),
        ) {
            shown.set(facts.parts.clone());
            preview.show(FieldPreviewState {
                parts: facts.parts,
                hint: facts.hint,
            });
        }
    };
    let release = move || {
        if let Some(preview) = preview {
            preview.release(&shown.peek());
        }
    };
    use_drop(release);
    let precision = 4;
    let displayed = format_numeric(value, precision)
        .parse::<f64>()
        .unwrap_or(value);
    rsx! {
        div {
            "data-transform-number": "true",
            onpointerenter: { let mut show = show.clone(); move |_| show() },
            onpointerleave: move |_| release(),
            onfocusin: { let mut show = show; move |_| show() },
            onfocusout: move |_| release(),
            NumericField {
                label, unit, value: displayed, precision,
                disabled_reason: (!(mount.editable && available)).then(|| "This transform is unavailable".to_owned()),
                on_commit: move |next| { submit(&mount, field, Value::Number(value), Value::Number(next)); },
            }
        }
    }
}

fn toggle(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &str,
    value: bool,
) -> Element {
    let mount = mount.clone();
    rsx! {
        ToggleField {
            key: "{field:?}", label: label.to_owned(), checked: value,
            disabled_reason: (!mount.editable).then(|| "This transform is unavailable".to_owned()),
            on_commit: move |next| { submit(&mount, field, Value::Bool(value), Value::Bool(next)); },
        }
    }
}

fn action(
    mount: &MatrixTransformInspectorMount,
    field: Field,
    label: &str,
    baseline: Value,
    value: Value,
) -> Element {
    let disabled = !mount.editable;
    let mount = mount.clone();
    rsx! {
        Button {
            button_type: "button", variant: ButtonVariant::Secondary, disabled,
            onclick: move |_| { submit(&mount, field, baseline.clone(), value.clone()); },
            "{label}"
        }
    }
}

#[component]
pub fn MatrixTransformInspector(props: MatrixTransformInspectorProps) -> Element {
    let mount = &props.mount;
    let mut mounted = mount.inspector_mounted;
    let visible = mount.projection.is_some();
    use_effect(use_reactive((&visible,), move |(visible,)| {
        mounted.set(visible)
    }));
    use_drop(move || mounted.set(false));
    let Some(projection) = &mount.projection else {
        return rsx! {};
    };
    let num = |field, label, unit, value| number(mount, field, label, unit, value, true);
    let mm = |field, label, value| num(field, label, "mm", value);
    let deg = |field, label, value| num(field, label, "°", value);
    let act = |field, label, baseline, value| action(mount, field, label, baseline, value);
    let owner = format!("{:?}", projection.owner);
    let content = match &projection.fields {
        MatrixTransformFields::Matrix {
            origin,
            rotation,
            mirror,
            mirror_y_locked,
            rotation_followed,
            anchor,
        } => {
            let mirror = *mirror;
            let mut options = vec![(None, "None".to_owned()), (Some(Mirror::X), "X".to_owned())];
            if !mirror_y_locked || mirror == Some(Mirror::Y) {
                options.push((Some(Mirror::Y), "Y".to_owned()));
            }
            let edit = mount.clone();
            rsx! {
                InspectorSection { title: "Position & orientation",
                    {mm(Field::OriginX, "Origin X", origin.x)}
                    {mm(Field::OriginY, "Origin Y", origin.y)}
                    {number(mount, Field::MatrixRotation, "Rotation", "°", *rotation, *rotation_followed)}
                    SelectField::<Option<Mirror>> {
                        label: "Mirror", value: mirror, options, disabled: !mount.editable,
                        on_commit: move |next| { submit(&edit, Field::MatrixMirror, Value::Mirror(mirror), Value::Mirror(next)); },
                    }
                }
                {anchor_controls(&props, anchor.as_ref())}
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
                {act(Field::RowOffsetReset, "Reset offsets", Value::Offset(*offset), Value::Offset(Vec2::default()))}
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
            let custom = *custom_origin;
            let edit = mount.clone();
            rsx! {
                InspectorSection { title: "Column transform",
                    {mm(Field::ColumnSpread, "Spread", spread.unwrap_or(*pitch))}
                    {mm(Field::ColumnStagger, "Stagger", *stagger)}
                    {mm(Field::ColumnOffsetX, "Offset X", offset.x)}
                    {mm(Field::ColumnOffsetY, "Offset Y", offset.y)}
                    {act(Field::ColumnOffsetReset, "Reset offsets", Value::Offset(*offset), Value::Offset(Vec2::default()))}
                    if convert.is_ok() {
                        {act(Field::ColumnOffsetConvert, "Convert offset to stagger", Value::Number(offset.y), Value::Number(0.0))}
                    }
                    SelectField::<MatrixSplayAffect> {
                        label: "Splay affects", value: affect(),
                        options: vec![(MatrixSplayAffect::Column, "This column".to_owned()), (MatrixSplayAffect::Following, "Following columns".to_owned())],
                        on_commit: move |value| affect.set(value),
                    }
                    {deg(Field::ColumnSplay, "Splay", column_angle(*splay_angle, *rotation, &affect()))}
                }
                InspectorSection { title: "Splay origin", default_open: false,
                    ToggleField {
                        label: "Custom origin", checked: custom,
                        disabled_reason: (!mount.editable).then(|| "This transform is unavailable".to_owned()),
                        on_commit: move |next| { submit(&edit, Field::SplayOriginMode, Value::OriginMode(custom), Value::OriginMode(next)); },
                    }
                    if custom {
                        {mm(Field::SplayOriginX, "Origin X", splay_origin.x)}
                        {mm(Field::SplayOriginY, "Origin Y", splay_origin.y)}
                        Button {
                            variant: ButtonVariant::Secondary, disabled: !mount.editable,
                            onclick: move |_| props.on_pick_splay_origin.call(()), "Pick origin"
                        }
                    }
                }
            }
        }
        MatrixTransformFields::Key {
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
            ..
        } => {
            let baseline = definition_id.clone();
            let edit = mount.clone();
            rsx! {
                InspectorSection { title: "Key transform",
                    {toggle(mount, Field::KeyEnabled, "Enabled", *enabled)}
                    if let Some(exception) = half_exception {
                        {toggle(mount, Field::KeyOmit, "Omit mirrored key", exception.omitted)}
                    }
                    {mm(Field::KeyShiftX, "Shift X", shift.x)}
                    {mm(Field::KeyShiftY, "Shift Y", shift.y)}
                    {deg(Field::KeyBend, "Bend", *bend)}
                    {mm(Field::KeyOffsetX, "Local X", offset.x)}
                    {mm(Field::KeyOffsetY, "Local Y", offset.y)}
                    {deg(Field::KeyRotation, "Key rotation", *rotation)}
                    {act(Field::KeyTransformReset, "Reset local transform",
                        Value::CellTransform { offset: *offset, rotation: *rotation },
                        Value::CellTransform { offset: Vec2::default(), rotation: 0.0 })}
                }
                InspectorSection { title: "Key assembly",
                    SelectField::<String> {
                        label: "Key assembly", value: definition_id.clone(), options: choices.clone(), disabled: !mount.editable,
                        on_commit: move |next| { submit(&edit, Field::KeyAssembly, Value::Text(baseline.clone()), Value::Text(next)); },
                    }
                    if *mirror_target && *assemblies_local {
                        {act(Field::KeyAssembliesLocal, "Use mirrored components", Value::Bool(true), Value::Bool(false))}
                    }
                    for (index, (id, definition)) in assemblies.iter().enumerate() {
                        {
                            let edit = mount.clone();
                            let baseline = assemblies.clone();
                            let mut remaining = assemblies.clone();
                            remaining.remove(index);
                            rsx! {
                                div { key: "{id}",
                                    SelectField::<String> {
                                        label: format!("Replace {id}"), value: definition.clone(),
                                        options: attachment_component_choices(component_choices, definition), disabled: !mount.editable,
                                        on_commit: move |next| {
                                            let mut replacements = baseline.clone();
                                            replacements[index].1 = next;
                                            submit(&edit, Field::KeyAttached, Value::Attached(baseline.clone()), Value::Attached(replacements));
                                        },
                                    }
                                    {action(mount, Field::KeyAttached, &format!("Remove {id}"), Value::Attached(assemblies.clone()), Value::Attached(remaining))}
                                }
                            }
                        }
                    }
                }
                {act(Field::KeyAddAnchoredMatrix, "Add anchored matrix", Value::Text(String::new()), Value::Text(String::new()))}
            }
        }
    };
    let error = mount
        .feedback
        .iter()
        .rev()
        .find(|feedback| feedback.owner == projection.owner)
        .and_then(|feedback| feedback.message.as_deref());
    rsx! {
        section { key: "{owner}", class: "grid min-w-0 gap-3 px-4 py-3", aria_label: "Matrix transform properties",
            onkeydown: move |event: KeyboardEvent| { if event.key() == Key::Character(" ".into()) { event.stop_propagation(); } },
            {content}
            SelectionDeleteControl { mount: mount.clone() }
            if let Some(error) = error {
                p { role: "alert", class: "text-destructive", "{error}" }
            }
            if let Some(status) = (mount.change_status)().filter(|status| status.owner == projection.owner) {
                p { "data-matrix-change-status": "true", role: "status", "{status.message}" }
            }
        }
    }
}

fn anchor_controls(
    props: &MatrixTransformInspectorProps,
    anchor: Option<&MatrixAnchor>,
) -> Element {
    let mount = &props.mount;
    let Some(projection) = mount.projection.as_ref() else {
        return rsx! {};
    };
    let draft = (mount.aggregate_pick_draft)();
    let aggregate = anchor.and_then(|anchor| anchor.aggregate.as_ref());
    let mut pending = draft
        .filter(|(owner, _)| owner == &projection.owner)
        .map(|(_, parts)| parts)
        .unwrap_or_default();
    if let Some(aggregate) = aggregate {
        pending.retain(|part| !aggregate.parts.contains(part));
    }
    let edit_method = mount.clone();
    let add = mount.clone();
    let cancel = mount.clone();
    let on_cancel = props.on_cancel_anchor_pick;
    let on_pick = props.on_pick_anchor;
    let on_navigate = props.on_navigate_anchor_part;
    let detach = mount
        .feedback
        .iter()
        .rev()
        .find(|feedback| {
            feedback.owner == projection.owner && feedback.field == Field::MatrixDetach
        })
        .and_then(|feedback| feedback.message.as_deref())
        .and_then(|message| {
            message.split_once(&format!(
                "Detaching {} would move its linked partner ",
                projection.owner.matrix_id
            ))
        })
        .and_then(|(_, suffix)| suffix.split_once(';'))
        .filter(|(partner, tail)| {
            tail.trim_start()
                == format!("make {partner} Local to keep its pose and confirm the detach")
        })
        .map(|(partner, _)| partner.to_owned());
    rsx! {
        InspectorSection { title: "Anchor",
            if let Some(anchor) = anchor {
                if let Some(aggregate) = aggregate {
                    {
                        let method = aggregate.method;
                        rsx! {
                            SelectField::<AggregateMethod> {
                                label: "Method", value: method, disabled: !mount.editable,
                                options: vec![(AggregateMethod::Average, "Average".to_owned()), (AggregateMethod::Intersect, "Intersection".to_owned())],
                                on_commit: move |next| { submit(&edit_method, Field::MatrixAnchorMethod, Value::AnchorMethod(method), Value::AnchorMethod(next)); },
                            }
                        }
                    }
                    for (index, part) in aggregate.parts.iter().enumerate() {
                        {
                            let tree = TreeContext::Key { matrix_id: part.matrix_id.clone(), row: part.row, column: part.column };
                            let edit = mount.clone();
                            let baseline = aggregate.parts.clone();
                            let mut remaining = baseline.clone();
                            remaining.remove(index);
                            rsx! {
                                div { key: "{part.matrix_id}-{part.row}-{part.column}",
                                    Button {
                                        variant: ButtonVariant::Secondary, onclick: move |_| on_navigate.call(tree.clone()),
                                        "{part.matrix_id} · row {part.row + 1} · column {part.column + 1}"
                                    }
                                    Button {
                                        variant: ButtonVariant::Secondary, disabled: !mount.editable || remaining.is_empty(),
                                        onclick: move |_| { submit(&edit, Field::MatrixAnchorParts, Value::AnchorParts(baseline.clone()), Value::AnchorParts(remaining.clone())); },
                                        "Remove"
                                    }
                                }
                            }
                        }
                    }
                    if props.anchor_pick_pending && !pending.is_empty() {
                        {
                            let baseline = aggregate.parts.clone();
                            let mut next = baseline.clone();
                            next.extend(pending.clone());
                            rsx! {
                                Button {
                                    variant: ButtonVariant::Secondary, disabled: !mount.editable,
                                    onclick: move |_| {
                                        submit(&add, Field::MatrixAnchorParts, Value::AnchorParts(baseline.clone()), Value::AnchorParts(next.clone()));
                                        add.clear_aggregate_pick();
                                        on_cancel.call(());
                                    },
                                    "Add {pending.len()} picked parts"
                                }
                            }
                        }
                    }
                } else {
                    p { "data-anchor-target": "true", "Anchored to {anchor.matrix_id} · row {anchor.row} · column {anchor.column}" }
                }
                {toggle(mount, Field::MatrixFollowX, "Follow X", anchor.follow.x)}
                {toggle(mount, Field::MatrixFollowY, "Follow Y", anchor.follow.y)}
                {toggle(mount, Field::MatrixFollowRotation, "Follow rotation", anchor.follow.rotation)}
                if let Some(partner) = detach {
                    {action(mount, Field::MatrixDetachConfirm, "Confirm detach", Value::Text(String::new()), Value::Text(partner))}
                } else {
                    {action(mount, Field::MatrixDetach, "Detach", Value::Bool(false), Value::Bool(true))}
                }
            }
            if props.anchor_pick_pending {
                Button {
                    variant: ButtonVariant::Secondary, onclick: move |_| {
                        cancel.clear_aggregate_pick();
                        on_cancel.call(());
                    },
                    "Cancel anchor pick"
                }
            } else {
                Button {
                    variant: ButtonVariant::Secondary, disabled: !mount.editable, onclick: move |_| on_pick.call(()),
                    if aggregate.is_some() { "Pick parts" } else { "Pick anchor" }
                }
            }
        }
    }
}

#[component]
pub fn SelectionDeleteControl(mount: MatrixTransformInspectorMount) -> Element {
    let Some(projection) = &mount.projection else {
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
            onpointerdown: move |event| event.stop_propagation(),
            onkeydown: move |event| event.stop_propagation(),
            {action(&mount, Field::DeleteSelection, label, Value::Bool(false), Value::Bool(true))}
        }
    }
}
