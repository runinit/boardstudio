//! The matrix-context Relations tab: Structure, Local adjustments and Authored
//! constraints, rendered from the pure `matrix_relations` projection.
use super::RelationHighlight;
use super::objects;
use boardstudio_web_layout::matrix_relations::{
    ConstraintItem, ConstraintKind, Departure, DepartureAction, MatrixRelations, PartReference,
    RelationItem,
};
use boardstudio_web_ui_model::state::LayerVisibility;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct MatrixRelationsPanelProps {
    pub relations: Option<MatrixRelations>,
    pub mount: objects::MatrixTransformInspectorMount,
    /// Select the referenced object. The owner guards currency and Escape return.
    pub on_navigate: EventHandler<objects::TreeContext>,
    /// Select a parameter's owner and open its matching tool/editor.
    pub on_open_owner: EventHandler<objects::TreeContext>,
}

#[component]
pub(super) fn MatrixRelationsPanel(props: MatrixRelationsPanelProps) -> Element {
    let on_navigate = props.on_navigate;
    use_effect({
        let mount = props.mount.clone();
        move || mount.set_inspector_mounted(true)
    });
    use_drop({
        let mount = props.mount.clone();
        move || mount.set_inspector_mounted(false)
    });
    let Some(relations) = props.relations.as_ref() else {
        return rsx! {
            section { class: "m1-layout-component-inspector", aria_labelledby: "m1-relations-heading",
                h2 { id: "m1-relations-heading", "Relationships" }
                p { class: "m1-layout-component-relation-summary", "This selection is no longer available." }
            }
        };
    };
    rsx! {
        section { class: "m1-layout-component-inspector", aria_labelledby: "m1-relations-heading",
            h2 { id: "m1-relations-heading", "Relationships" }
            if let Some(note) = relations.selection_note.as_ref() {
                p { class: "m1-relations-cross-matrix", role: "note", "{note}" }
            }
            section { class: "m1-relations-structure", aria_labelledby: "m1-relations-structure-heading",
                h3 { id: "m1-relations-structure-heading", "How it is placed" }
                ul {
                    for item in relations.structure.iter() {
                        StructureRow { item: item.clone(), on_navigate, on_open_owner: props.on_open_owner }
                    }
                }
            }
            if !relations.departures.is_empty() {
                section { class: "m1-relations-departures", aria_labelledby: "m1-relations-departures-heading",
                    h3 { id: "m1-relations-departures-heading", "Independent adjustments" }
                    ul {
                        for departure in relations.departures.iter() {
                            DepartureRow { departure: departure.clone(), mount: props.mount.clone(), on_open_owner: props.on_open_owner }
                        }
                    }
                }
            }
            if !relations.constraints.is_empty() {
                section { class: "m1-relations-constraints", aria_labelledby: "m1-relations-constraints-heading",
                    h3 { id: "m1-relations-constraints-heading", "Authored constraints" }
                    ul {
                        for constraint in relations.constraints.iter() {
                            ConstraintRow { constraint: constraint.clone(), on_navigate }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StructureRow(
    item: RelationItem,
    on_navigate: EventHandler<objects::TreeContext>,
    on_open_owner: EventHandler<objects::TreeContext>,
) -> Element {
    // Parameter entries open their owner's matching tool/editor; receiver
    // references navigate while staying on the Relations tab.
    let on_select = if item.opens_editor {
        on_open_owner
    } else {
        on_navigate
    };
    rsx! {
        li { class: "m1-relations-row", "data-relation-label": "{item.label}",
            span { class: "m1-layout-component-matrix-note", "{item.label}" }
            " "
            if let Some(target) = item.target.clone() {
                RelationReference {
                    text: item.value.clone(),
                    parts: item.highlight.clone(),
                    what: format!("{} {}", item.label, item.value),
                    on_select: EventHandler::new(move |()| on_select.call(target.clone())),
                }
            } else {
                span { "{item.value}" }
            }
        }
    }
}

#[component]
fn DepartureRow(
    departure: Departure,
    mount: objects::MatrixTransformInspectorMount,
    on_open_owner: EventHandler<objects::TreeContext>,
) -> Element {
    rsx! {
        li { class: "m1-relations-row", "data-relation-departure": "{departure.parameter}",
            span { class: "m1-layout-component-matrix-note", "{departure.parameter}" }
            " "
            if let Some(target) = departure.target.clone() {
                RelationReference {
                    text: departure.value.clone(),
                    parts: departure.highlight.clone(),
                    what: format!("{} {}", departure.parameter, departure.value),
                    on_select: EventHandler::new(move |()| on_open_owner.call(target.clone())),
                }
            } else {
                span { "{departure.value}" }
            }
            for action in departure.actions.iter().copied() {
                DepartureActionButton { action, parameter: departure.parameter.clone(), mount: mount.clone() }
            }
        }
    }
}

#[component]
fn DepartureActionButton(
    action: DepartureAction,
    parameter: String,
    mount: objects::MatrixTransformInspectorMount,
) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let Some(control) = mount.departure_control(action) else {
        return rsx! {};
    };
    let kind = match action {
        DepartureAction::Reset => "reset",
        DepartureAction::Convert => "convert",
    };
    let label = control.label;
    let reason_id = format!("m1-relations-reason-{kind}-{}", parameter.replace(' ', "-"));
    let described_by = control.reason.is_some().then(|| reason_id.clone());
    let error = mount.departure_feedback(action, submitted());
    rsx! {
        button {
            r#type: "button", class: "m1-inspector-secondary", disabled: control.disabled,
            "data-relation-action": "{kind}", aria_label: "{label}: {parameter}",
            aria_describedby: described_by,
            onclick: move |_| {
                if let Some(id) = mount.run_departure(action) {
                    submitted.set(Some(id));
                }
            },
            "{label}"
        }
        if let Some(reason) = control.reason.as_deref() {
            small { id: "{reason_id}", class: "m1-empty-note m1-matrix-transform-unavailable", "{reason}" }
        }
        if let Some(message) = error.as_deref() {
            small { role: "alert", class: "m1-matrix-transform-error", "{message}" }
        }
    }
}

#[component]
fn ConstraintRow(
    constraint: ConstraintItem,
    on_navigate: EventHandler<objects::TreeContext>,
) -> Element {
    let kind = match constraint.kind {
        ConstraintKind::Offset => "Offset",
        ConstraintKind::PartOffset => "Part offset",
        ConstraintKind::Mirror => "Mirror",
        ConstraintKind::MirrorLine => "Mirror line",
    };
    rsx! {
        li { class: "m1-relations-row", "data-relation-constraint": "{constraint.id}",
            span { class: "m1-layout-component-matrix-note", "{kind}" }
            " "
            ReferenceButton { reference: constraint.source.clone(), role: "source", on_navigate }
            " drives "
            ReferenceButton { reference: constraint.target.clone(), role: "target", on_navigate }
            if constraint.takes_precedence {
                small { class: "m1-empty-note", " Takes priority over the attachment to its key." }
            }
        }
    }
}

#[component]
fn ReferenceButton(
    reference: PartReference,
    role: &'static str,
    on_navigate: EventHandler<objects::TreeContext>,
) -> Element {
    let context = reference.context.clone();
    rsx! {
        RelationReference {
            text: reference.label.clone(),
            parts: vec![reference.part_id.clone()],
            what: format!("the {role} {}", reference.label),
            role,
            part_id: reference.part_id.clone(),
            on_select: EventHandler::new(move |()| on_navigate.call(context.clone())),
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct RelationReferenceProps {
    /// The visible text of the reference.
    text: String,
    /// The part IDs the reference highlights on the Layout canvas.
    parts: Vec<String>,
    /// What the highlight names, for the `aria-describedby` text.
    what: String,
    #[props(default)]
    role: Option<&'static str>,
    #[props(default)]
    part_id: Option<String>,
    on_select: EventHandler<()>,
}

/// A Relations reference button. Hover or keyboard focus marks `parts` on the Layout canvas;
/// leaving, blurring or unmounting releases that highlight, and the button names it through
/// `aria-describedby`. Selecting the reference is the button's own action.
#[component]
fn RelationReference(props: RelationReferenceProps) -> Element {
    let highlight = use_context::<RelationHighlight>();
    let RelationReferenceProps {
        text,
        parts,
        what,
        role,
        part_id,
        on_select,
    } = props;
    // Labels repeat across rows, so the id carries a per-instance counter.
    let instance = use_hook(|| {
        thread_local! { static NEXT: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
        NEXT.with(|next| {
            let id = next.get();
            next.set(id.wrapping_add(1));
            id
        })
    });
    let description_id = format!("m1-relations-highlight-{instance}");
    let described_by = (!parts.is_empty()).then(|| description_id.clone());
    let enter = parts.clone();
    let leave = parts.clone();
    let focus = parts.clone();
    let blur = parts.clone();
    // The drop closure runs once, so it reads the parts of the latest render.
    let latest = use_hook(|| std::rc::Rc::new(std::cell::RefCell::new(Vec::<String>::new())));
    *latest.borrow_mut() = parts.clone();
    use_drop({
        let latest = latest.clone();
        move || highlight.release(&latest.borrow())
    });
    // Parts behind a hidden layer toggle have no visible canvas shape. The
    // highlight still names the same parts, but the row says the canvas
    // cannot show them instead of implying it does.
    let hidden = match try_use_context::<LayerVisibility>() {
        Some(visibility) => {
            !parts.is_empty()
                && parts.iter().all(|part| {
                    visibility.hidden.read().contains(part)
                        || visibility.modules_hidden.read().contains(part)
                })
        }
        None => false,
    };
    let description = if hidden {
        format!("{what} is on a hidden layer; the Layout canvas cannot highlight it.")
    } else {
        format!("Highlights the Layout canvas parts for {what}.")
    };
    rsx! {
        button {
            r#type: "button", class: "m1-inspector-secondary m1-relations-reference",
            "data-relation-role": role, "data-part-id": part_id,
            aria_describedby: described_by,
            onpointerenter: move |_| highlight.show(&enter),
            onpointerleave: move |_| highlight.release(&leave),
            onfocus: move |_| highlight.show(&focus),
            onblur: move |_| highlight.release(&blur),
            onclick: move |_| on_select.call(()),
            "{text}"
        }
        if hidden {
            small { class: "m1-empty-note", " (hidden layer)" }
        }
        if !parts.is_empty() {
            span { id: "{description_id}", class: "m1-visually-hidden",
                "{description}"
            }
        }
    }
}

#[component]
pub(super) fn RelationMatrixKey(
    part_id: Option<String>,
    selected: bool,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    transform: String,
    matrix_id: String,
    row: u32,
    column: u32,
    on_pointer_down: EventHandler<PointerEvent>,
) -> Element {
    let RelationHighlight(highlight) = use_context::<RelationHighlight>();
    let highlighted = part_id.as_ref().is_some_and(|id| {
        highlight()
            .as_ref()
            .is_some_and(|part_ids| part_ids.contains(id))
    });
    rsx! {
        rect {
            class: if selected { "m1-matrix-key is-selected" } else if highlighted { "m1-matrix-key is-relation-highlight" } else { "m1-matrix-key" },
            x: "{x}", y: "{y}", width: "{width}", height: "{height}", rx: "0.9",
            transform: "{transform}", "data-matrix-id": "{matrix_id}", "data-row": "{row}", "data-column": "{column}",
            onpointerdown: move |event| on_pointer_down.call(event)
        }
    }
}

#[component]
pub(super) fn RelationPartOutline(part_id: String, selected: bool, points: String) -> Element {
    let RelationHighlight(highlight) = use_context::<RelationHighlight>();
    let highlighted = highlight()
        .as_ref()
        .is_some_and(|part_ids| part_ids.contains(&part_id));
    rsx! {
        polygon {
            points: "{points}",
            class: if selected { "m1-part selected" } else if highlighted { "m1-part is-relation-highlight" } else { "m1-part" }
        }
    }
}
