//! Contextual Properties and Relations for one accepted standalone Layout part.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Constraint, MirrorAxis, PartOutline, Vec2};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwnerKey {
    pub scope: Option<Scope>,
    pub workspace: &'static str,
    pub part_id: Option<String>,
}

#[derive(Default)]
pub struct LayoutComponentInspectorLifetime {
    current: Cell<u64>,
    key: RefCell<Option<LayoutComponentInspectorOwnerKey>>,
}

impl LayoutComponentInspectorLifetime {
    pub fn update(&self, key: Option<LayoutComponentInspectorOwnerKey>) -> u64 {
        let mut current_key = self.key.borrow_mut();
        if *current_key != key {
            self.current.set(self.current.get().wrapping_add(1).max(1));
            *current_key = key;
        }
        self.current.get()
    }

    pub fn current_generation(&self) -> u64 {
        self.current.get()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutComponentInspectorOwner {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub part_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutComponentInspectorProjection {
    pub owner: LayoutComponentInspectorOwner,
    pub reference: String,
    pub definition_name: String,
    pub definition_kind: String,
    pub envelope_notice: Option<String>,
    pub locked: bool,
    pub position: Vec2,
    pub layout_id: Option<String>,
    pub layouts: Vec<LayoutChoice>,
    pub outline: PartOutline,
    pub board_parts: Vec<BoardPartChoice>,
    pub active_constraint: Option<Constraint>,
    pub relationship_summary: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutChoice {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoardPartChoice {
    pub id: String,
    pub reference: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutConstraintValues {
    Offset { offset: Vec2, rotation: f64 },
    Mirror { axis: MirrorAxis, coordinate: f64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentPositionAxis {
    X,
    Y,
}

#[derive(Clone, Debug, PartialEq)]
pub enum LayoutComponentInspectorAction {
    SetPosition {
        owner: LayoutComponentInspectorOwner,
        axis: ComponentPositionAxis,
        value: f64,
    },
    AssignLayout {
        owner: LayoutComponentInspectorOwner,
        layout_id: Option<String>,
    },
    SetOutline {
        owner: LayoutComponentInspectorOwner,
        outline: PartOutline,
    },
    SetConstraint {
        owner: LayoutComponentInspectorOwner,
        source_part_id: String,
        values: LayoutConstraintValues,
    },
    RemoveConstraint {
        owner: LayoutComponentInspectorOwner,
        constraint_id: String,
    },
    NavigateElectrical {
        owner: LayoutComponentInspectorOwner,
    },
}

#[derive(Props, Clone, PartialEq)]
pub struct LayoutComponentInspectorProps {
    pub projection: LayoutComponentInspectorProjection,
    pub on_action: EventHandler<LayoutComponentInspectorAction>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum InspectorTab {
    Properties,
    Relations,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConstraintKind {
    Offset,
    Mirror,
}

#[component]
pub fn LayoutComponentInspector(props: LayoutComponentInspectorProps) -> Element {
    let projection = props.projection.clone();
    let owner = projection.owner.clone();
    // Selection lifetime is independent from the accepted snapshot. A newer
    // accepted revision must refresh action admission without discarding
    // unrelated dirty fields or the user's selected Inspector tab.
    let identity = (
        owner.scope.clone(),
        owner.context_generation,
        owner.scope_generation,
        owner.part_id.clone(),
    );
    let capture_identity = (identity.clone(), owner.snapshot_token, owner.revision);
    let mut latest_capture = use_signal(|| owner.clone());
    use_effect(use_reactive((&capture_identity,), {
        let owner = owner.clone();
        move |_| latest_capture.set(owner.clone())
    }));
    let mut tab = use_signal(|| InspectorTab::Properties);
    let mut constraint_open = use_signal(|| projection.active_constraint.is_some());
    let mut x = use_signal(|| format!("{:.2}", projection.position.x));
    let mut y = use_signal(|| format!("{:.2}", projection.position.y));
    let mut margin = use_signal(|| {
        projection
            .outline
            .margin
            .map(|value| value.to_string())
            .unwrap_or_default()
    });
    let mut constraint_kind = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { .. }) => ConstraintKind::Mirror,
        _ => ConstraintKind::Offset,
    });
    let initial_source = projection
        .active_constraint
        .as_ref()
        .map(Constraint::source)
        .filter(|source| projection.board_parts.iter().any(|part| part.id == *source))
        .map(str::to_owned)
        .or_else(|| {
            projection
                .board_parts
                .iter()
                .find(|part| part.id != owner.part_id)
                .map(|part| part.id.clone())
        })
        .unwrap_or_default();
    let mut source_part_id = use_signal(|| initial_source.clone());
    let mut offset_x = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Offset { offset, .. }) => offset.x.to_string(),
        _ => "0".to_owned(),
    });
    let mut offset_y = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Offset { offset, .. }) => offset.y.to_string(),
        _ => "0".to_owned(),
    });
    let mut rotation = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Offset { rotation, .. }) => rotation.to_string(),
        _ => "0".to_owned(),
    });
    let mut mirror_axis = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { axis, .. }) => axis.clone(),
        _ => MirrorAxis::Vertical,
    });
    let mut mirror_coordinate = use_signal(|| match projection.active_constraint.as_ref() {
        Some(Constraint::Mirror { coordinate, .. }) => coordinate.to_string(),
        _ => "0".to_owned(),
    });
    let mut error = use_signal(|| None::<String>);

    let initial_position = projection.position;
    let initial_outline = projection.outline.clone();
    let initial_constraint = projection.active_constraint.clone();
    let has_initial_constraint = initial_constraint.is_some();
    let board_parts = projection.board_parts.clone();
    let part_id = owner.part_id.clone();
    use_effect(use_reactive((&identity,), {
        move |_| {
            tab.set(InspectorTab::Properties);
            constraint_open.set(has_initial_constraint);
            error.set(None);
        }
    }));
    let accepted_x = (identity.clone(), initial_position.x);
    use_effect(use_reactive((&accepted_x,), move |((_, value),)| {
        x.set(format!("{value:.2}"));
    }));
    let accepted_y = (identity.clone(), initial_position.y);
    use_effect(use_reactive((&accepted_y,), move |((_, value),)| {
        y.set(format!("{value:.2}"));
    }));
    let accepted_margin = (identity.clone(), initial_outline.margin);
    use_effect(use_reactive((&accepted_margin,), move |((_, value),)| {
        margin.set(value.map(|value| value.to_string()).unwrap_or_default());
    }));
    let accepted_constraint = (identity.clone(), initial_constraint.clone());
    use_effect(use_reactive((&accepted_constraint,), {
        move |((_, constraint),)| {
            constraint_open.set(constraint.is_some());
            constraint_kind.set(if matches!(constraint, Some(Constraint::Mirror { .. })) {
                ConstraintKind::Mirror
            } else {
                ConstraintKind::Offset
            });
            let source = constraint
                .as_ref()
                .map(Constraint::source)
                .filter(|source| board_parts.iter().any(|part| part.id == *source))
                .map(str::to_owned)
                .or_else(|| {
                    board_parts
                        .iter()
                        .find(|part| part.id != part_id)
                        .map(|part| part.id.clone())
                })
                .unwrap_or_default();
            source_part_id.set(source);
            match constraint {
                Some(Constraint::Offset {
                    offset,
                    rotation: degrees,
                    ..
                }) => {
                    offset_x.set(offset.x.to_string());
                    offset_y.set(offset.y.to_string());
                    rotation.set(degrees.to_string());
                }
                _ => {
                    offset_x.set("0".to_owned());
                    offset_y.set("0".to_owned());
                    rotation.set("0".to_owned());
                }
            }
            match constraint {
                Some(Constraint::Mirror {
                    axis, coordinate, ..
                }) => {
                    mirror_axis.set(axis.clone());
                    mirror_coordinate.set(coordinate.to_string());
                }
                _ => {
                    mirror_axis.set(MirrorAxis::Vertical);
                    mirror_coordinate.set("0".to_owned());
                }
            }
            error.set(None);
        }
    }));
    let reset_properties_drafts = {
        let mut x = x;
        let mut y = y;
        let mut margin = margin;
        let mut constraint_open = constraint_open;
        let mut constraint_kind = constraint_kind;
        let mut source_part_id = source_part_id;
        let mut offset_x = offset_x;
        let mut offset_y = offset_y;
        let mut rotation = rotation;
        let mut mirror_axis = mirror_axis;
        let mut mirror_coordinate = mirror_coordinate;
        let mut error = error;
        let initial_position = initial_position;
        let initial_margin = initial_outline.margin;
        let initial_constraint = initial_constraint.clone();
        let initial_source = initial_source.clone();
        move || {
            x.set(format!("{:.2}", initial_position.x));
            y.set(format!("{:.2}", initial_position.y));
            margin.set(
                initial_margin
                    .map(|value| value.to_string())
                    .unwrap_or_default(),
            );
            constraint_open.set(initial_constraint.is_some());
            constraint_kind.set(
                if matches!(initial_constraint.as_ref(), Some(Constraint::Mirror { .. })) {
                    ConstraintKind::Mirror
                } else {
                    ConstraintKind::Offset
                },
            );
            source_part_id.set(initial_source.clone());
            match initial_constraint.as_ref() {
                Some(Constraint::Offset {
                    offset,
                    rotation: degrees,
                    ..
                }) => {
                    offset_x.set(offset.x.to_string());
                    offset_y.set(offset.y.to_string());
                    rotation.set(degrees.to_string());
                }
                _ => {
                    offset_x.set("0".to_owned());
                    offset_y.set("0".to_owned());
                    rotation.set("0".to_owned());
                }
            }
            match initial_constraint.as_ref() {
                Some(Constraint::Mirror {
                    axis, coordinate, ..
                }) => {
                    mirror_axis.set(axis.clone());
                    mirror_coordinate.set(coordinate.to_string());
                }
                _ => {
                    mirror_axis.set(MirrorAxis::Vertical);
                    mirror_coordinate.set("0".to_owned());
                }
            }
            error.set(None);
        }
    };

    let commit_position: Rc<dyn Fn(ComponentPositionAxis)> = {
        let latest_capture = latest_capture;
        let position = projection.position;
        let action = props.on_action;
        let error = error;
        Rc::new(move |axis| {
            let mut error = error;
            let (draft, current) = match axis {
                ComponentPositionAxis::X => (x(), position.x),
                ComponentPositionAxis::Y => (y(), position.y),
            };
            let Ok(value) = draft.parse::<f64>() else {
                error.set(Some("Enter a finite position coordinate.".to_owned()));
                return;
            };
            if !value.is_finite() {
                error.set(Some("Enter a finite position coordinate.".to_owned()));
                return;
            }
            error.set(None);
            if value != current {
                action.call(LayoutComponentInspectorAction::SetPosition {
                    owner: latest_capture(),
                    axis,
                    value,
                });
            }
        })
    };
    let set_outline: Rc<dyn Fn(PartOutline)> = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        Rc::new(move |outline| {
            action.call(LayoutComponentInspectorAction::SetOutline {
                owner: latest_capture(),
                outline,
            })
        })
    };
    let save_constraint = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        let mut error = error;
        move |_| {
            let source = source_part_id();
            let owner = latest_capture();
            if source.is_empty() || source == owner.part_id {
                error.set(Some("Choose another part on this board.".to_owned()));
                return;
            }
            let values = match constraint_kind() {
                ConstraintKind::Offset => {
                    let (Ok(x), Ok(y), Ok(rotation)) = (
                        offset_x().parse::<f64>(),
                        offset_y().parse::<f64>(),
                        rotation().parse::<f64>(),
                    ) else {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    };
                    if !x.is_finite() || !y.is_finite() || !rotation.is_finite() {
                        error.set(Some("Enter finite constraint values.".to_owned()));
                        return;
                    }
                    LayoutConstraintValues::Offset {
                        offset: Vec2 { x, y },
                        rotation,
                    }
                }
                ConstraintKind::Mirror => {
                    let Ok(coordinate) = mirror_coordinate().parse::<f64>() else {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    };
                    if !coordinate.is_finite() {
                        error.set(Some("Enter a finite axis coordinate.".to_owned()));
                        return;
                    }
                    LayoutConstraintValues::Mirror {
                        axis: mirror_axis(),
                        coordinate,
                    }
                }
            };
            error.set(None);
            action.call(LayoutComponentInspectorAction::SetConstraint {
                owner: owner.clone(),
                source_part_id: source,
                values,
            });
        }
    };
    let active_constraint_id = projection
        .active_constraint
        .as_ref()
        .map(|constraint| constraint.id().to_owned());
    let remove_constraint = {
        let latest_capture = latest_capture;
        let action = props.on_action;
        move |_| {
            if let Some(constraint_id) = active_constraint_id.as_ref() {
                action.call(LayoutComponentInspectorAction::RemoveConstraint {
                    owner: latest_capture(),
                    constraint_id: constraint_id.clone(),
                });
            }
        }
    };
    let position = projection.position;
    let outline = projection.outline.clone();
    let active_constraint = projection.active_constraint.clone();
    let board_parts = projection.board_parts.clone();
    let constraint_source_reference = active_constraint
        .as_ref()
        .and_then(|constraint| {
            board_parts
                .iter()
                .find(|part| part.id == constraint.source())
        })
        .map(|part| part.reference.clone())
        .unwrap_or_else(|| "A part".to_owned());

    rsx! {
        section { class: "m1-layout-component-inspector", aria_label: "Component inspector",
            header { class: "m1-layout-component-inspector-heading",
                h2 { "{projection.reference}" }
                if projection.locked { span { class: "m1-inspector-lock", "Locked" } }
            }
            div { role: "tablist", aria_label: "Inspector details", class: "m1-layout-component-tabs",
                button { r#type: "button", role: "tab", aria_selected: "{tab() == InspectorTab::Properties}", onclick: { let mut reset = reset_properties_drafts; move |_| { reset(); tab.set(InspectorTab::Properties); } }, "Properties" }
                button { r#type: "button", role: "tab", aria_selected: "{tab() == InspectorTab::Relations}", onclick: move |_| tab.set(InspectorTab::Relations), "Relations" }
            }
            if tab() == InspectorTab::Properties {
                p { class: "m1-layout-component-definition", "{projection.definition_name}", span { "{projection.definition_kind}" } }
                if let Some(notice) = projection.envelope_notice.as_ref() { p { class: "m1-layout-component-notice", "{notice}" } }
                if !projection.layouts.is_empty() {
                    label { class: "m1-layout-component-layout", "Layout"
                        select {
                            aria_label: "Component layout",
                            value: projection.layout_id.as_deref().unwrap_or(""),
                            onchange: {
                                let latest_capture = latest_capture;
                                let action = props.on_action;
                                move |event: FormEvent| action.call(LayoutComponentInspectorAction::AssignLayout {
                                    owner: latest_capture(),
                                    layout_id: (!event.value().is_empty()).then_some(event.value()),
                                })
                            },
                            option { value: "", selected: projection.layout_id.is_none(), "Board / ungrouped" }
                            for layout in &projection.layouts {
                                option {
                                    value: "{layout.id}",
                                    selected: projection.layout_id.as_deref() == Some(layout.id.as_str()),
                                    "{layout.name}"
                                }
                            }
                        }
                    }
                }
                h3 { class: "m1-layout-component-subtitle", "Position", small { "millimetres" } }
                div { class: "m1-layout-component-position",
                    label { "X (mm)" input {
                        r#type: "number", step: "0.1", value: "{x}", aria_label: "X mm",
                        oninput: move |event| x.set(event.value()),
                        onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::X) },
                        onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                            match event.data().key().to_string().as_str() {
                                "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::X); }
                                "Escape" => { x.set(format!("{:.2}", position.x)); error.set(None); }
                                _ => {}
                            }
                        } },
                    } }
                    label { "Y (mm)" input {
                        r#type: "number", step: "0.1", value: "{y}", aria_label: "Y mm",
                        oninput: move |event| y.set(event.value()),
                        onblur: { let commit = commit_position.clone(); move |_| commit(ComponentPositionAxis::Y) },
                        onkeydown: { let commit = commit_position.clone(); move |event: KeyboardEvent| {
                            match event.data().key().to_string().as_str() {
                                "Enter" => { event.prevent_default(); commit(ComponentPositionAxis::Y); }
                                "Escape" => { y.set(format!("{:.2}", position.y)); error.set(None); }
                                _ => {}
                            }
                        } },
                    } }
                }
                details { class: "m1-layout-component-outline",
                    summary { span { "Board outline" } small { if projection.outline.excluded { "Excluded" } else { "Included" } } }
                    fieldset { disabled: projection.locked,
                    label { input {
                        r#type: "checkbox", aria_label: "Include in outline", checked: !projection.outline.excluded,
                        onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.excluded = !event.checked(); submit(outline); } },
                    } "Include in outline" }
                    if !projection.outline.excluded {
                        label { input {
                            r#type: "checkbox", aria_label: "Use board margin", checked: projection.outline.margin.is_none(),
                            onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.margin = if event.checked() { None } else { Some(0.0) }; margin.set(outline.margin.map(|value| value.to_string()).unwrap_or_default()); submit(outline); } },
                        } "Use board margin" }
                        if outline.margin.is_some() {
                            label { "Part edge margin" input {
                                r#type: "number", step: "any", min: "0", value: "{margin}", aria_label: "Part edge margin",
                                oninput: move |event| margin.set(event.value()),
                                onblur: {
                                    let margin_outline = outline.clone();
                                    let save_outline = set_outline.clone();
                                    move |_| {
                                        match margin().parse::<f64>() {
                                            Ok(value) if value.is_finite() && value >= 0.0 => {
                                                if value != margin_outline.margin.unwrap_or(value) {
                                                    let mut next = margin_outline.clone(); next.margin = Some(value); save_outline(next);
                                                }
                                                error.set(None);
                                            }
                                            _ => error.set(Some("Enter 0 or greater for the part edge margin.".to_owned())),
                                        }
                                    }
                                },
                                onkeydown: {
                                    let accepted_margin = outline.margin.unwrap_or_default().to_string();
                                    move |event: KeyboardEvent| match event.data().key().to_string().as_str() {
                                        "Enter" => {
                                            event.prevent_default();
                                            if let Some(input) = event.data().try_as_web_event()
                                                .and_then(|event| event.target())
                                                .and_then(|target| target.dyn_into::<web_sys::HtmlInputElement>().ok())
                                            {
                                                let _ = input.blur();
                                            }
                                        }
                                        "Escape" => {
                                            event.prevent_default();
                                            margin.set(accepted_margin.clone());
                                            error.set(None);
                                        }
                                        _ => {}
                                    }
                                },
                            } }
                        }
                    }
                    label { input {
                        r#type: "checkbox", aria_label: "Allow component body overhang", checked: projection.outline.allow_body_overhang,
                        onchange: { let baseline = outline.clone(); let submit = set_outline.clone(); move |event| { let mut outline = baseline.clone(); outline.allow_body_overhang = event.checked(); submit(outline); } },
                    } "Allow component body overhang" }
                    small { "Pad and drill support remains required when a part is excluded or body overhang is allowed." }
                    }
                }
                details { class: "m1-layout-component-constraint", open: constraint_open(),
                    summary { onclick: move |_| constraint_open.set(!constraint_open()), span { "Layout constraint" } small { if active_constraint.is_some() { "Active" } else { "Optional" } } }
                    if board_parts.iter().filter(|part| part.id != owner.part_id).count() > 0 {
                        label { "Relationship"
                            select { aria_label: "Constraint type", value: if constraint_kind() == ConstraintKind::Offset { "offset" } else { "mirror" }, onchange: move |event| constraint_kind.set(if event.value() == "mirror" { ConstraintKind::Mirror } else { ConstraintKind::Offset }),
                                option { value: "offset", selected: constraint_kind() == ConstraintKind::Offset, "Offset from part" }
                                option { value: "mirror", selected: constraint_kind() == ConstraintKind::Mirror, "Mirror placement across axis" }
                            }
                        }
                        label { "Source part"
                            select { aria_label: "Constraint source part", value: "{source_part_id}", onchange: move |event| source_part_id.set(event.value()),
                                for part in board_parts.iter().filter(|part| part.id != owner.part_id) {
                                    option {
                                        value: "{part.id}",
                                        selected: source_part_id().as_str() == part.id.as_str(),
                                        "{part.reference}"
                                    }
                                }
                            }
                        }
                        if constraint_kind() == ConstraintKind::Offset {
                            div { class: "m1-layout-component-constraint-grid",
                                label { "Offset X (mm)" input { r#type: "number", step: "any", aria_label: "Offset X (mm)", value: "{offset_x}", oninput: move |event| offset_x.set(event.value()) } }
                                label { "Offset Y (mm)" input { r#type: "number", step: "any", aria_label: "Offset Y (mm)", value: "{offset_y}", oninput: move |event| offset_y.set(event.value()) } }
                            }
                            label { "Rotation (degrees)" input { r#type: "number", step: "any", aria_label: "Rotation (degrees)", value: "{rotation}", oninput: move |event| rotation.set(event.value()) } }
                        } else {
                            label { "Axis"
                                select { aria_label: "Mirror axis", value: if mirror_axis() == MirrorAxis::Vertical { "vertical" } else { "horizontal" }, onchange: move |event| mirror_axis.set(if event.value() == "horizontal" { MirrorAxis::Horizontal } else { MirrorAxis::Vertical }),
                                    option { value: "vertical", selected: mirror_axis() == MirrorAxis::Vertical, "Vertical" }
                                    option { value: "horizontal", selected: mirror_axis() == MirrorAxis::Horizontal, "Horizontal" }
                                }
                            }
                            label { "Axis coordinate (mm)" input { r#type: "number", step: "any", aria_label: "Axis coordinate (mm)", value: "{mirror_coordinate}", oninput: move |event| mirror_coordinate.set(event.value()) } }
                            p { "Footprint geometry stays unchanged. Use a handed definition where needed." }
                        }
                        if active_constraint.is_some() {
                            p { class: "m1-layout-component-constraint-note", "{constraint_source_reference} drives {projection.reference}." }
                        }
                        div { class: "m1-layout-component-constraint-actions",
                            button { r#type: "button", onclick: save_constraint, if active_constraint.is_some() { "Save constraint" } else { "Add constraint" } }
                            if active_constraint.is_some() { button { r#type: "button", onclick: remove_constraint, "Remove" } }
                        }
                    } else { p { "Add another part on this board to create a layout constraint." } }
                }
                if let Some(message) = error() { p { role: "alert", "{message}" } }
                button { class: "m1-layout-component-electrical", r#type: "button", onclick: {
                    let latest_capture = latest_capture; let action = props.on_action;
                    move |_| action.call(LayoutComponentInspectorAction::NavigateElectrical { owner: latest_capture() })
                }, "Edit electrical connections" }
            } else {
                h2 { "Relationships" }
                p { class: "m1-layout-component-relation-summary", "{projection.relationship_summary}" }
                button { r#type: "button", onclick: move |_| tab.set(InspectorTab::Properties), "Edit placement relationship" }
                p { class: "m1-layout-component-matrix-note", "Matrix rows and columns share pitch, stagger and splay. Edit those in Properties." }
            }
        }
    }
}
