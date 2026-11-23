//! The matrix-context Relationships section: Structure, Local adjustments and
//! Authored constraints, rendered from the pure `matrix_relations` projection.
use super::RelationHighlight;
use super::objects;
use boardstudio_web_layout::matrix_relations::{
    ConstraintItem, ConstraintKind, Departure, DepartureAction, MatrixRelations, PartReference,
    RelationItem,
};
use boardstudio_web_ui_model::state::LayerVisibility;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    item::Item,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;

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
    let Some(relations) = props.relations.as_ref() else {
        return rsx! {
            section {
                class: "grid min-w-0 content-start gap-3 px-4 pt-3 pb-5 text-foreground [&_h2]:m-0 [&_h3]:m-0 [&_p]:m-0",
                aria_labelledby: "m1-relations-heading",
                h2 { id: "m1-relations-heading", "Relationships" }
                p { class: "text-xs leading-normal",
                    "This selection is no longer available."
                }
            }
        };
    };
    rsx! {
        section {
            class: "grid min-w-0 content-start gap-3 px-4 pt-3 pb-5 text-foreground [&_h2]:m-0 [&_h3]:m-0 [&_p]:m-0",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() {
                        raw.stop_propagation();
                    }
                }
            },
            aria_labelledby: "m1-relations-heading",
            h2 { id: "m1-relations-heading", "Relationships" }
            if let Some(note) = relations.selection_note.as_ref() {
                p { class: "text-xs text-muted-foreground", role: "note", "{note}" }
            }
            section {
                aria_labelledby: "m1-relations-structure-heading",
                h3 { id: "m1-relations-structure-heading", "How it is placed" }
                ul {
                    for item in relations.structure.iter() {
                        StructureRow { item: item.clone(), on_navigate, on_open_owner: props.on_open_owner }
                    }
                }
            }
            if !relations.departures.is_empty() {
                section {
                    aria_labelledby: "m1-relations-departures-heading",
                    h3 { id: "m1-relations-departures-heading", "Independent adjustments" }
                    ul {
                        for departure in relations.departures.iter() {
                            DepartureRow {
                                departure: departure.clone(),
                                mount: props.mount.clone(),
                                on_open_owner: props.on_open_owner,
                            }
                        }
                    }
                }
            }
            if !relations.constraints.is_empty() {
                section {
                    aria_labelledby: "m1-relations-constraints-heading",
                    h3 { id: "m1-relations-constraints-heading", "Authored constraints" }
                    ul {
                        for constraint in relations.constraints.iter() {
                            ConstraintRow {
                                constraint: constraint.clone(),
                                on_navigate,
                            }
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
    // references navigate while staying beside the selection.
    let on_select = if item.opens_editor {
        on_open_owner
    } else {
        on_navigate
    };
    rsx! {
        li { "data-relation-label": "{item.label}",
            Item { class: "p-0",
                span { class: "text-xs leading-[1.45] text-muted-foreground", "{item.label}" }
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
}

#[component]
fn DepartureRow(
    departure: Departure,
    mount: objects::MatrixTransformInspectorMount,
    on_open_owner: EventHandler<objects::TreeContext>,
) -> Element {
    rsx! {
        li {
            "data-relation-departure": "{departure.parameter}",
            Item { class: "p-0",
                span { class: "text-xs leading-[1.45] text-muted-foreground", "{departure.parameter}" }
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
                    DepartureActionButton {
                        action,
                        parameter: departure.parameter.clone(),
                        mount: mount.clone(),
                    }
                }
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
    let _ = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>().map(|d| d.0());
    let mut error = use_signal(|| None::<String>);
    let Some(control) = mount.departure_control(action) else {
        return rsx! {};
    };
    let kind = match action {
        DepartureAction::Reset => "reset",
        DepartureAction::Convert => "convert",
    };
    let label = control.label;
    let reason_id = format!(
        "m1-relations-reason-{kind}-{}",
        dioxus::core::current_scope_id().0
    );
    let described_by = control.reason.is_some().then(|| reason_id.clone());
    rsx! {
            div { "data-relation-action": "{kind}",
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    disabled: control.disabled,
                    aria_label: "{label}: {parameter}",
                    aria_describedby: described_by,
                    onclick: move |_| {
                        if let Some(result) = mount.run_departure(action) {
                            error.set(result.err());
                        }
                    },

                    "{label}"

                }
            }
            if let Some(reason) = control.reason.as_deref() {
                small {
                    id: "{reason_id}",
                    "{reason}"
                }
            }
            if let Some(message) = error().as_deref() {
                Alert {
    variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
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
        li {
            "data-relation-constraint": "{constraint.id}",
            Item { class: "p-0",
                span { class: "text-xs leading-[1.45] text-muted-foreground", "{kind}" }
                " "
                ReferenceButton {
                    reference: constraint.source.clone(),
                    role: "source",
                    on_navigate,
                }
                " drives "
                ReferenceButton {
                    reference: constraint.target.clone(),
                    role: "target",
                    on_navigate,
                }
                if constraint.takes_precedence {
                    small { " Takes priority over the attachment to its key." }
                }
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

/// Labels whose rows name what the selection carries downstream: later rows and
/// columns inherit this selection's steps, or another matrix follows it.
/// Variant B renders these after the chain as dashed "Carries …" chips.
fn carries_label(label: &str) -> bool {
    matches!(label, "Inherited by" | "Followed by")
}

/// Labels that name the anchor reference the selection is placed from.
/// Variant B renders these as violet dashed chips, matching the hover outline.
fn anchor_reference_label(label: &str) -> bool {
    matches!(
        label,
        "Anchor frame" | "Anchor" | "Anchor part" | "Follows" | "Attached to"
    )
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct PlacementBreadcrumbProps {
    pub relations: MatrixRelations,
    pub on_navigate: EventHandler<objects::TreeContext>,
    pub on_open_owner: EventHandler<objects::TreeContext>,
}

/// Variant B "How it is placed" breadcrumb above the Inspector fields: the
/// chain in kernel order, then dashed "Carries …" chips for what the
/// selection carries. Chips navigate exactly like the Relationships rows
/// (Replace selection, never a document write) and hover with the same violet
/// reference highlight.
#[component]
pub(super) fn PlacementBreadcrumb(props: PlacementBreadcrumbProps) -> Element {
    rsx! {
        nav { class: "flex flex-wrap items-center gap-1 border-b border-border bg-field px-4 py-2.5", aria_label: "How it is placed",
            span { class: "w-full text-xs font-semibold text-muted-foreground", "How it is placed" }
            for (index, item) in props.relations.structure.iter().cloned().enumerate() {
                if index > 0 {
                    span { class: "text-xs text-border-strong", aria_hidden: "true", "›" }
                }
                BreadcrumbChip {
                    item,
                    on_navigate: props.on_navigate,
                    on_open_owner: props.on_open_owner,
                }
            }
        }
    }
}

#[component]
fn BreadcrumbChip(
    item: RelationItem,
    on_navigate: EventHandler<objects::TreeContext>,
    on_open_owner: EventHandler<objects::TreeContext>,
) -> Element {
    // Parameter entries open their owner's matching tool/editor; receiver
    // references navigate while staying beside the selection.
    let on_select = if item.opens_editor {
        on_open_owner
    } else {
        on_navigate
    };
    let carries = carries_label(&item.label);
    let anchor = anchor_reference_label(&item.label);
    let text = if carries {
        format!("Carries {}", item.value)
    } else {
        item.value.clone()
    };
    let chip_class = if carries {
        "inline-flex max-w-full [&_button]:h-auto [&_button]:rounded-full [&_button]:border [&_button]:border-dashed [&_button]:border-border-strong [&_button]:px-2 [&_button]:py-0.5 [&_button]:text-xs [&_button]:text-muted-foreground"
    } else if anchor {
        "inline-flex max-w-full [&_button]:h-auto [&_button]:rounded-full [&_button]:border [&_button]:border-dashed [&_button]:border-geometry-part [&_button]:px-2 [&_button]:py-0.5 [&_button]:text-xs [&_button]:text-geometry-part"
    } else {
        "inline-flex max-w-full [&_button]:h-auto [&_button]:rounded-full [&_button]:border [&_button]:border-border-strong [&_button]:px-2 [&_button]:py-0.5 [&_button]:text-xs"
    };
    rsx! {
        span { class: "{chip_class}", "data-crumb-label": "{item.label}",
            if let Some(target) = item.target.clone() {
                RelationReference {
                    text: text.clone(),
                    parts: item.highlight.clone(),
                    what: format!("{} {}", item.label, item.value),
                    on_select: EventHandler::new(move |()| on_select.call(target.clone())),
                }
            } else {
                span { title: "{item.label} {item.value}", "{text}" }
            }
        }
    }
}

/// Whether the amber at-rest field preview (ticket 16) currently marks
/// `part_id`. Distinct from the violet dashed Relations reference: the
/// preview names the keys one Inspector number field would move, and it
/// wins ties so the hovered field reads through a reference.
pub(super) fn field_previews(part_id: Option<&String>) -> bool {
    use super::FieldPreview;
    let preview = try_use_context::<FieldPreview>();
    part_id.is_some_and(|id| {
        preview.is_some_and(|target| {
            target.0()
                .as_ref()
                .is_some_and(|state| state.parts.contains(id))
        })
    })
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
        div { class: "inline-flex", "data-relation-role": role, "data-part-id": part_id,
            Button {
                variant: ButtonVariant::Secondary,
                button_type: "button",
                aria_describedby: described_by,
                onpointerenter: move |_| highlight.show(&enter),
                onpointerleave: move |_| highlight.release(&leave),
                onfocus: move |_| highlight.show(&focus),
                onblur: move |_| highlight.release(&blur),
                onclick: move |_| on_select.call(()),

                "{text}"

            }
        }
        if hidden {
            small { class: "text-xs text-muted-foreground", " (hidden layer)" }
        }
        if !parts.is_empty() {
            span { id: "{description_id}", class: "sr-only",
                "{description}"
            }
        }
    }
}

#[component]
pub(super) fn RelationMatrixKey(
    part_id: Option<String>,
    selected: bool,
    #[props(default)] dimmed: bool,
    /// Ticket 17: the canvas anchor badge target. The violet dashed reference
    /// stands while the selection does, so it wins over dimming like the
    /// selection itself; the amber field preview still wins every tie.
    #[props(default)]
    anchor_target: bool,
    /// Ticket 02: the pointer rests on this key (prototype variant D). The
    /// caller only sets it while no amber preview marks the key, so the
    /// preview keeps winning every tie.
    #[props(default)]
    hovered: bool,
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
    // Ticket 14: selection stays solid on the dimmed half; only unselected
    // unfocused keys fade to the grey dashed outline. Ticket 16: the amber
    // at-rest field preview wins ties so the hovered field reads through
    // selection, dimming and violet references alike. Ticket 17: the badge
    // target is selection chrome, so it wins over dimming like the selection.
    // Ticket 02: key hover (prototype variant D) tints the key but never
    // overrides the amber preview; the caller only sets `hovered` then.
    let previewed = field_previews(part_id.as_ref());
    let base = if previewed {
        "fill-geometry-fill-muted/55 stroke-warning stroke-[0.42]"
    } else if selected {
        "fill-selected stroke-primary stroke-[0.42]"
    } else if hovered && dimmed {
        "fill-muted-foreground stroke-muted-foreground stroke-[1.5px] [vector-effect:non-scaling-stroke]"
    } else if hovered {
        "fill-selected/60 stroke-primary stroke-[1.5px] [vector-effect:non-scaling-stroke]"
    } else if anchor_target || highlighted {
        "fill-geometry-fill-muted/55 stroke-geometry-part stroke-[0.42] [stroke-dasharray:1.2_0.8]"
    } else if dimmed {
        "fill-none stroke-border-strong stroke-[0.3] [stroke-dasharray:1.2_0.8]"
    } else {
        "fill-geometry-fill-muted/55 stroke-geometry-outline stroke-[0.28]"
    };
    let visible_highlight = highlighted && !previewed && !selected;
    let visible_dimmed = dimmed && !selected && !anchor_target && !highlighted && !previewed;
    let visible_anchor = anchor_target && !selected && !previewed;
    rsx! {
        rect {
            class: "pointer-events-none {base}",
            "data-selected": "{selected}",
            "data-relation-highlight": "{visible_highlight}",
            "data-field-preview": "{previewed}",
            "data-dimmed": "{visible_dimmed}",
            "data-anchor-target": "{visible_anchor}",
            "data-hovered": "{hovered}",
            x: "{x}", y: "{y}", width: "{width}", height: "{height}", rx: "0.9",
            transform: "{transform}", "data-matrix-id": "{matrix_id}", "data-row": "{row}", "data-column": "{column}",
            onpointerdown: move |event| on_pointer_down.call(event)
        }
    }
}

#[component]
pub(super) fn RelationPartOutline(
    part_id: String,
    selected: bool,
    points: String,
    #[props(default)] dimmed: bool,
    /// Ticket 17: the canvas anchor badge target, like the key rect above.
    #[props(default)]
    anchor_target: bool,
    /// Ticket 02: the pointer rests on this part (prototype variant D). The
    /// caller only sets it while no amber preview marks the part, so the
    /// preview keeps winning every tie.
    #[props(default)]
    hovered: bool,
) -> Element {
    let RelationHighlight(highlight) = use_context::<RelationHighlight>();
    let highlighted = highlight()
        .as_ref()
        .is_some_and(|part_ids| part_ids.contains(&part_id));
    let previewed = field_previews(Some(&part_id));
    let base = if previewed {
        "fill-geometry-body stroke-warning stroke-[0.8]"
    } else if selected {
        "fill-selected stroke-primary stroke-[0.8]"
    } else if hovered && dimmed {
        "fill-muted-foreground stroke-muted-foreground stroke-[1.5px] [vector-effect:non-scaling-stroke]"
    } else if hovered {
        "fill-selected/60 stroke-primary stroke-[1.5px] [vector-effect:non-scaling-stroke]"
    } else if anchor_target || highlighted {
        "fill-geometry-body stroke-geometry-part stroke-[0.8] [stroke-dasharray:1.2_0.8]"
    } else if dimmed {
        "fill-none stroke-border-strong stroke-[0.4] [stroke-dasharray:1.2_0.8]"
    } else {
        "fill-geometry-body stroke-geometry-part stroke-[0.4]"
    };
    let visible_highlight = highlighted && !previewed && !selected;
    let visible_dimmed = dimmed && !selected && !anchor_target && !highlighted && !previewed;
    let visible_anchor = anchor_target && !selected && !previewed;
    rsx! {
        polygon {
            points: "{points}",
            class: "cursor-grab {base}",
            "data-geometry": "part-outline",
            "data-selected": "{selected}",
            "data-relation-highlight": "{visible_highlight}",
            "data-field-preview": "{previewed}",
            "data-dimmed": "{visible_dimmed}",
            "data-anchor-target": "{visible_anchor}",
            "data-hovered": "{hovered}",
        }
    }
}
