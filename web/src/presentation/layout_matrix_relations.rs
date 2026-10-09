//! The matrix-context Relations tab: Structure, Local adjustments and Authored
//! constraints, rendered from the pure `matrix_relations` projection.
use super::objects;
use boardstudio_web_layout::matrix_relations::{
    ConstraintItem, ConstraintKind, Departure, DepartureAction, MatrixRelations, PartReference,
    RelationItem,
};
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct MatrixRelationsPanelProps {
    pub relations: Option<MatrixRelations>,
    pub mount: objects::MatrixTransformInspectorMount,
    /// Select the referenced object. The owner guards currency and Escape return.
    pub on_navigate: EventHandler<objects::TreeContext>,
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
            section { class: "m1-layout-component-inspector", aria_label: "Relationships",
                h2 { "Relationships" }
                p { class: "m1-layout-component-relation-summary", "This selection is no longer available." }
            }
        };
    };
    rsx! {
        section { class: "m1-layout-component-inspector", aria_label: "Relationships",
            h2 { "Relationships" }
            section { class: "m1-relations-structure", aria_label: "Structure",
                h3 { "Structure" }
                ul {
                    for item in relations.structure.iter() {
                        StructureRow { item: item.clone(), on_navigate }
                    }
                }
            }
            if !relations.departures.is_empty() {
                section { class: "m1-relations-departures", aria_label: "Local adjustments",
                    h3 { "Local adjustments" }
                    ul {
                        for departure in relations.departures.iter() {
                            DepartureRow { departure: departure.clone(), mount: props.mount.clone(), on_navigate }
                        }
                    }
                }
            }
            if !relations.constraints.is_empty() {
                section { class: "m1-relations-constraints", aria_label: "Authored constraints",
                    h3 { "Authored constraints" }
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
fn StructureRow(item: RelationItem, on_navigate: EventHandler<objects::TreeContext>) -> Element {
    rsx! {
        li { class: "m1-relations-row", "data-relation-label": "{item.label}",
            span { class: "m1-layout-component-matrix-note", "{item.label}" }
            " "
            if let Some(target) = item.target.clone() {
                button {
                    r#type: "button", class: "m1-inspector-secondary m1-relations-reference",
                    onclick: move |_| on_navigate.call(target.clone()),
                    "{item.value}"
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
    on_navigate: EventHandler<objects::TreeContext>,
) -> Element {
    rsx! {
        li { class: "m1-relations-row", "data-relation-departure": "{departure.parameter}",
            span { class: "m1-layout-component-matrix-note", "{departure.parameter}" }
            " "
            if let Some(target) = departure.target.clone() {
                button {
                    r#type: "button", class: "m1-inspector-secondary m1-relations-reference",
                    onclick: move |_| on_navigate.call(target.clone()),
                    "{departure.value}"
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
    let Some(control) = mount.departure_control(action) else {
        return rsx! {};
    };
    let (label, kind) = match action {
        DepartureAction::Reset => ("Reset", "reset"),
        DepartureAction::Convert => ("Convert to stagger", "convert"),
    };
    rsx! {
        button {
            r#type: "button", class: "m1-inspector-secondary", disabled: control.disabled,
            "data-relation-action": "{kind}", aria_label: "{label}: {parameter}",
            onclick: move |_| mount.run_departure(action),
            "{label}"
        }
        if let Some(reason) = control.reason.as_deref() {
            small { class: "m1-empty-note m1-matrix-transform-unavailable", "{reason}" }
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
        ConstraintKind::Mirror => "Mirror",
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
        button {
            r#type: "button", class: "m1-inspector-secondary m1-relations-reference",
            "data-relation-role": "{role}", "data-part-id": "{reference.part_id}",
            onclick: move |_| on_navigate.call(context.clone()),
            "{reference.label}"
        }
    }
}
