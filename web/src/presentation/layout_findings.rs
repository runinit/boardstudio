//! Layout's accepted-scene findings page and guarded navigation request.
use super::keycaps_fit::{self, FindingNavigationTarget};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Finding, ProjectDoc, Severity};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Source {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Request {
    pub source: Source,
    pub finding: Finding,
    pub target: FindingNavigationTarget,
}

pub(super) struct InspectorMount {
    pub open: bool,
    pub document: Rc<ProjectDoc>,
    pub findings: Vec<Finding>,
    pub source: Source,
    pub on_close: EventHandler<()>,
    pub on_navigate: EventHandler<Request>,
}

#[component]
pub(super) fn LayoutFindingsFooterButton(count: usize, on_toggle: EventHandler<()>) -> Element {
    rsx! {
        button {
            class: "m1-canvas-footer-findings",
            id: "m1-layout-findings-trigger",
            r#type: "button",
            title: "Open Layout findings",
            onclick: move |_| on_toggle.call(()),
            svg { view_box: "0 0 20 20", fill: "none", stroke: "currentColor", stroke_width: "1.5", "aria-hidden": "true",
                path { d: "M10 2.5 18 17H2zM10 7v4m0 3v.2" }
            }
            "Layout findings: {count}"
        }
    }
}

#[component]
pub(super) fn LayoutFindingsInspector(
    open: bool,
    document: Rc<ProjectDoc>,
    findings: Vec<Finding>,
    source: Source,
    on_close: EventHandler<()>,
    on_navigate: EventHandler<Request>,
) -> Element {
    use_effect(use_reactive((&open,), move |(open,)| {
        if open
            && let Some(element) = web_sys::window()
                .and_then(|window| window.document())
                .and_then(|document| document.get_element_by_id("m1-layout-findings-heading"))
                .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
        {
            let _ = element.focus();
        }
    }));

    if !open {
        return rsx! {};
    }

    let groups = keycaps_fit::grouped_findings(&findings, &document);
    rsx! {
        section {
            class: "m1-layout-findings",
            aria_label: "Layout findings",
            onkeydown: move |event: KeyboardEvent| {
                if event.key() == "Escape" {
                    event.prevent_default();
                    event.stop_propagation();
                    on_close.call(());
                }
            },
            div { class: "m1-layout-findings-return",
                button {
                    class: "m1-layout-findings-back",
                    r#type: "button",
                    onclick: move |_| on_close.call(()),
                    "Back to selection"
                }
                span { "Layout findings" }
            }
            div { class: "m1-layout-findings-heading",
                h2 { id: "m1-layout-findings-heading", tabindex: "-1", "Layout findings" }
                span { "{keycaps_fit::presented_findings(&findings, &document).len()}" }
            }
            if groups.is_empty() {
                p { class: "m1-layout-findings-empty", "No active findings for this board." }
            } else {
                div { class: "wb-finding-groups",
                    for group in groups {
                        section { aria_label: "{group.label}",
                            h3 { "{group.label}" }
                            ul { class: "wb-findings",
                                for finding in group.findings {
                                    LayoutFinding {
                                        finding,
                                        document: document.clone(),
                                        source: source.clone(),
                                        on_navigate,
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn LayoutFinding(
    finding: Finding,
    document: Rc<ProjectDoc>,
    source: Source,
    on_navigate: EventHandler<Request>,
) -> Element {
    let (severity, severity_class) = match &finding.severity {
        Severity::Error => ("Error", "error"),
        Severity::Warning => ("Warning", "warning"),
        Severity::Info => ("Information", "info"),
    };
    let target = keycaps_fit::finding_navigation_target(&finding, &document)
        .filter(|target| target_has_live_layout_destination(target, &document));
    let action_label = target.as_ref().map(|target| match target {
        FindingNavigationTarget::Outline { .. } => "Show outline",
        FindingNavigationTarget::Part { .. }
        | FindingNavigationTarget::Matrix { .. }
        | FindingNavigationTarget::Board { .. } => "Select affected geometry",
        FindingNavigationTarget::Body { .. } | FindingNavigationTarget::MechanicalLayer { .. } => {
            ""
        }
    });
    let request = target.map(|target| Request {
        source,
        finding: finding.clone(),
        target,
    });

    rsx! {
        li { class: "m1-keycaps-fit-finding m1-layout-finding is-{severity_class}",
            span { class: "wb-finding-mark", "aria-hidden": "true" }
            div {
                strong { class: "wb-finding-severity", "{severity}" }
                p { "{finding.message}" }
                if let (Some(label), Some(request)) = (action_label.filter(|label| !label.is_empty()), request) {
                    button {
                        class: "wb-finding-action",
                        r#type: "button",
                        onclick: move |_| on_navigate.call(request.clone()),
                        "{label}"
                    }
                }
            }
        }
    }
}

pub(super) fn target_has_live_layout_destination(
    target: &FindingNavigationTarget,
    document: &ProjectDoc,
) -> bool {
    let board_id = keycaps_fit::target_board_id(target);
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return false;
    };
    match target {
        FindingNavigationTarget::Outline { .. } | FindingNavigationTarget::Board { .. } => true,
        FindingNavigationTarget::Part { part_id, .. } => {
            board.part_ids.iter().any(|id| id == part_id)
                && document.parts.iter().any(|part| part.id == *part_id)
        }
        FindingNavigationTarget::Matrix { matrix_id, .. } => {
            document.matrices.iter().any(|matrix| {
                matrix.id == *matrix_id
                    && matrix
                        .board_id
                        .as_deref()
                        .is_none_or(|owner| owner == board_id)
                    && matrix
                        .part_ids
                        .iter()
                        .any(|part_id| board.part_ids.contains(part_id))
            })
        }
        FindingNavigationTarget::Body { .. } | FindingNavigationTarget::MechanicalLayer { .. } => {
            false
        }
    }
}
