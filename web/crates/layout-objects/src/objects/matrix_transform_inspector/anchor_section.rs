//! Anchor section of the transform Inspector: detach, pick, aggregate, follow.
use super::*;

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
pub(super) struct DetachAnchorButtonProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub detach_pending: Signal<bool>,
    pub confirm_pending: Signal<bool>,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
}

/// Detach an anchored matrix. A plain click clears the anchor in one atomic
/// Core intent. When Core refuses because the detach would move the linked
/// partner, the refusal names the required Local exception and the button
/// offers Confirm/Cancel in place: Confirm applies the detach and the
/// exception atomically, Cancel leaves the whole edit unchanged. A stale
/// confirmation is refused by Core and surfaces as an error, never a silent
/// change.
#[component]
pub(super) fn DetachAnchorButton(props: DetachAnchorButtonProps) -> Element {
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
            div { "data-detach-confirm": "true", class: "grid gap-2",
                p { class: "text-xs text-destructive", role: "alert", "{error.as_deref().unwrap_or_default()}" }
                p { class: "text-xs text-muted-foreground",
                    "Confirming clears both anchors and keeps every pose in one Undo entry."
                }
                Button {
                    button_type: "button", variant: ButtonVariant::Secondary,
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
                Button {
                    button_type: "button", variant: ButtonVariant::Secondary,
                    onclick: move |_| submitted.set(None),
                    "Cancel"
                }
            }
        } else {
            Button {
                button_type: "button", variant: ButtonVariant::Secondary,
                disabled: !props.editable || busy,
                onclick: move |_| send(
                    MatrixTransformField::MatrixDetach,
                    MatrixTransformValue::Bool(false),
                    MatrixTransformValue::Bool(true),
                ),
                "Detach"
            }
            if let Some(message) = error.as_deref() {
                FieldError { "{message}" }
            }
        }
    }
}

/// Retarget the anchor by picking a key on the canvas. Arming the pick is a
/// presentation concern (the Inspector only fires `on_pick_anchor`); the
/// canvas hit funnels into `MatrixTransformInspectorMount::pick_matrix_anchor`
/// with the picked key identity and Core validates the target.
#[derive(Props, Clone, PartialEq)]
pub(super) struct PickAnchorButtonProps {
    pub editable: bool,
    pub pending: bool,
    pub on_pick_anchor: EventHandler<()>,
    pub on_cancel_anchor_pick: EventHandler<()>,
}

#[component]
pub(super) fn PickAnchorButton(props: PickAnchorButtonProps) -> Element {
    rsx! {
        if props.pending {
            p { class: "text-xs text-muted-foreground",
                "Click a key on the canvas to retarget the anchor."
            }
            Button {
                button_type: "button", variant: ButtonVariant::Secondary,
                onclick: move |_| props.on_cancel_anchor_pick.call(()),
                "Cancel anchor pick"
            }
        } else {
            Button {
                button_type: "button", variant: ButtonVariant::Secondary,
                disabled: !props.editable,
                onclick: move |_| props.on_pick_anchor.call(()),
                "Pick anchor"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct AggregateAnchorSectionProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub anchor: MatrixAnchor,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub on_navigate_anchor_part: EventHandler<TreeContext>,
    pub mount: MatrixTransformInspectorMount,
    pub aggregate_pick_pending: bool,
    pub on_pick_anchor: EventHandler<()>,
    pub on_cancel_anchor_pick: EventHandler<()>,
}

/// The aggregate arm of the Anchor section: the Core-admitted method over N
/// single-key parts, displayed from Core-published values only. Each part
/// renders as a link that navigates to the part key without writing; the
/// revision and history stay unchanged.
#[component]
pub(super) fn AggregateAnchorSection(props: AggregateAnchorSectionProps) -> Element {
    let Some(aggregate) = props.anchor.aggregate.clone() else {
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
    let method_choice = use_memo(use_reactive((&is_average,), |(average,)| {
        Some(if average { "average" } else { "intersect" }.to_owned())
    }));
    let submitted = use_signal(|| None::<(MatrixTransformField, u64)>);
    let mut submitted_effect = submitted;
    let observed = use_signal(|| (current_method, aggregate.parts.clone()));
    let mut observed_effect = observed;
    use_effect(use_reactive(
        (&current_method, &aggregate.parts),
        move |(method, parts)| {
            if observed_effect() != (method, parts.clone()) {
                observed_effect.set((method, parts.clone()));
                submitted_effect.set(None);
            }
        },
    ));
    let error_for = |field: MatrixTransformField| {
        props
            .feedback
            .iter()
            .rev()
            .find(|item| {
                props.owner == item.owner
                    && submitted() == Some((field, item.request_id))
                    && item.message.is_some()
            })
            .and_then(|item| item.message.clone())
    };
    let error = error_for(MatrixTransformField::MatrixAnchorMethod);
    let parts_error = error_for(MatrixTransformField::MatrixAnchorParts);
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
                format!(
                    "{} · row {} · column {}",
                    part.matrix_id, part.row, part.column
                ),
                format!("{}-{}-{}", part.matrix_id, part.row, part.column),
            )
        })
        .collect();
    let sequence = props.request_sequence;
    let owner = props.owner.clone();
    let token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let affect = props.splay_affect;
    let editable = props.editable;
    rsx! {
        p { "data-anchor-target": "true", class: "text-xs text-muted-foreground",
            "{method} of {count} {noun}"
        }
        Field {
          FieldLabel { html_for: "matrix-anchor-method", "Method" }
          Select::<String> {
            id: "matrix-anchor-method", aria_label: "Method",
            value: Some(method_choice.into()),
            disabled: !editable,
            on_value_change: {
                let owner = owner.clone();
                let mut sequence = sequence;
                let mut submitted = submitted;
                move |choice: Option<String>| {
                    let Some(choice) = choice else { return; };
                    let next = if choice == "intersect" {
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
                    submitted.set(Some((MatrixTransformField::MatrixAnchorMethod, request_id)));
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
                }
            },
            SelectOption::<String> { index: 0usize, value: "average".to_owned(), text_value: "Average", "Average" }
            SelectOption::<String> { index: 1usize, value: "intersect".to_owned(), text_value: "Intersection", "Intersection" }
          }
        }
        if let Some(message) = error.as_deref() {
            FieldError { "{message}" }
        }
        ul { class: "grid gap-2",
            for (index, (context, label, key)) in parts.iter().enumerate() {
                li { key: "{key}",
                    Button {
                        button_type: "button", variant: ButtonVariant::Secondary,
                        onclick: {
                            let context = context.clone();
                            let on_navigate = props.on_navigate_anchor_part;
                            move |_| on_navigate.call(context.clone())
                        },
                        "{label}"
                    }
                    Button {
                        button_type: "button", variant: ButtonVariant::Secondary,
                        aria_label: format!("Remove {label}"),
                        disabled: !editable || parts.len() <= 1,
                        onclick: {
                            let owner = owner.clone();
                            let mut sequence = sequence;
                            let mut submitted = submitted;
                            let mut next_parts = aggregate.parts.clone();
                            next_parts.remove(index);
                            let baseline_parts = aggregate.parts.clone();
                            move |_| {
                                let Some(request_id) = sequence().checked_add(1) else {
                                    return;
                                };
                                sequence.set(request_id);
                                submitted.set(Some((MatrixTransformField::MatrixAnchorParts, request_id)));
                                on_edit.call(MatrixTransformRequest {
                                    owner: owner.clone(),
                                    request_id,
                                    snapshot_token: token,
                                    revision,
                                    field: MatrixTransformField::MatrixAnchorParts,
                                    baseline: MatrixTransformValue::AnchorParts(baseline_parts.clone()),
                                    value: MatrixTransformValue::AnchorParts(next_parts.clone()),
                                    splay_affect: affect(),
                                    draft: None,
                                    failure: None,
                                    submitted_text: None,
                                    one_shot: false,
                                });
                            }
                        },
                        "Remove"
                    }
                }
            }
        }
        if let Some(message) = parts_error.as_deref() {
            FieldError { "{message}" }
        }
        AggregateMultiPick {
            owner: owner.clone(), snapshot_token: token, revision,
            aggregate: aggregate.clone(),
            request_sequence: sequence, editable,
            splay_affect: affect, on_edit,
            mount: props.mount.clone(),
            armed: props.aggregate_pick_pending,
            on_pick_anchor: props.on_pick_anchor,
            on_cancel_anchor_pick: props.on_cancel_anchor_pick,
            submitted,
        }
    }
}

/// The canvas multi-Pick for aggregate parts, reusing the ticket-23 armed-pick
/// pattern: arming is presentation state, the draft lives on the mount, and
/// committing writes one `SetMatrixAnchor` Core intent. Cancelling or Esc
/// drops the draft with nothing written.
#[derive(Props, Clone, PartialEq)]
pub(super) struct AggregateMultiPickProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub aggregate: AggregateAnchor,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub mount: MatrixTransformInspectorMount,
    pub armed: bool,
    pub on_pick_anchor: EventHandler<()>,
    pub on_cancel_anchor_pick: EventHandler<()>,
    pub submitted: Signal<Option<(MatrixTransformField, u64)>>,
}

#[component]
pub(super) fn AggregateMultiPick(props: AggregateMultiPickProps) -> Element {
    let draft_parts = props
        .mount
        .aggregate_pick_draft
        .read()
        .clone()
        .filter(|(draft_owner, _)| *draft_owner == props.owner)
        .map(|(_, parts)| parts)
        .unwrap_or_default();
    let fresh_parts: Vec<AnchorPart> = draft_parts
        .into_iter()
        .filter(|part| !props.aggregate.parts.contains(part))
        .collect();
    if !props.armed {
        return rsx! {
            Button {
                button_type: "button", variant: ButtonVariant::Secondary,
                disabled: !props.editable,
                onclick: move |_| props.on_pick_anchor.call(()),
                "Pick parts"
            }
        };
    }
    let count = fresh_parts.len();
    let noun = if count == 1 { "part" } else { "parts" };
    rsx! {
        p { "data-anchor-pick-note": "true", class: "text-xs text-muted-foreground",
            "Click keys on the canvas to add parts."
        }
        if count > 0 {
            p { "data-anchor-pick-note": "true", class: "text-xs text-muted-foreground",
                "{count} picked"
            }
            Button {
                button_type: "button", variant: ButtonVariant::Secondary,
                disabled: !props.editable,
                onclick: {
                    let owner = props.owner.clone();
                    let on_edit = props.on_edit;
                    let mut sequence = props.request_sequence;
                    let mut submitted = props.submitted;
                    let affect = props.splay_affect;
                    let token = props.snapshot_token;
                    let revision = props.revision;
                    let mount = props.mount.clone();
                    let aggregate = props.aggregate.clone();
                    move |_| {
                        let mut next_parts = aggregate.parts.clone();
                        next_parts.extend(fresh_parts.clone());
                        let Some(request_id) = sequence().checked_add(1) else {
                            return;
                        };
                        sequence.set(request_id);
                        submitted.set(Some((MatrixTransformField::MatrixAnchorParts, request_id)));
                        on_edit.call(MatrixTransformRequest {
                            owner: owner.clone(),
                            request_id,
                            snapshot_token: token,
                            revision,
                            field: MatrixTransformField::MatrixAnchorParts,
                            baseline: MatrixTransformValue::AnchorParts(aggregate.parts.clone()),
                            value: MatrixTransformValue::AnchorParts(next_parts),
                            splay_affect: affect(),
                            draft: None,
                            failure: None,
                            submitted_text: None,
                            one_shot: false,
                        });
                        mount.clear_aggregate_pick();
                    }
                },
                "Add {count} {noun}"
            }
        }
        Button {
            button_type: "button", variant: ButtonVariant::Secondary,
            onclick: {
                let mount = props.mount.clone();
                let on_cancel = props.on_cancel_anchor_pick;
                move |_| {
                    mount.clear_aggregate_pick();
                    on_cancel.call(());
                }
            },
            "Cancel anchor pick"
        }
    }
}
#[derive(Props, Clone, PartialEq)]
pub(super) struct FollowTransformFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub anchor: MatrixAnchor,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub splay_affect: Signal<MatrixSplayAffect>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
}

/// The anchor's Follow subset as three toggles. Each toggle submits its own
/// field in one atomic Core intent; a newer accepted anchor retires the
/// pending error display.
#[component]
pub(super) fn FollowTransformField(props: FollowTransformFieldProps) -> Element {
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
            Label { html_for: format!("matrix-anchor-{}", label.replace(' ', "-")),
                Checkbox {
                    id: format!("matrix-anchor-{}", label.replace(' ', "-")),
                    checked: Some(if current { CheckboxState::Checked } else { CheckboxState::Unchecked }),
                    disabled: !props.editable,
                    aria_label: label,
                    on_checked_change: move |next| send(field, current, next == CheckboxState::Checked),
                }
                "{label}"
            }
        }
    };
    rsx! {
        div { class: "flex flex-wrap gap-3",
            {toggle((MatrixTransformField::MatrixFollowX, "Follow X", current_follow.x))}
            {toggle((MatrixTransformField::MatrixFollowY, "Follow Y", current_follow.y))}
            {toggle((MatrixTransformField::MatrixFollowRotation, "Follow rotation", current_follow.rotation))}
        }
        if let Some(message) = error.as_deref() {
            FieldError { "{message}" }
        }
    }
}
