//! Layout's findings page and guarded navigation request.
pub use super::keycaps_fit::target_has_live_layout_destination;
use super::keycaps_fit::{self, FindingNavigationTarget};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{Finding, ProjectDoc, Severity};
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    empty::{Empty, EmptyDescription},
    item::{Item, ItemSize},
};

use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Source {
    pub workspace: &'static str,
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub source: Source,
    pub finding: Finding,
    pub target: FindingNavigationTarget,
}

#[derive(Clone)]
pub struct InspectorMount {
    pub open: bool,
    pub document: Rc<ProjectDoc>,
    pub findings: Vec<Finding>,
    pub source: Source,
    pub on_close: EventHandler<()>,
    pub on_navigate: EventHandler<Request>,
}

#[component]
pub fn LayoutFindingsInspector(
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
            class: "grid min-w-0 gap-2.5 px-4 pt-3 pb-4 text-foreground",
            aria_label: "Layout findings",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() { raw.stop_propagation(); }
                }
                if event.key() == Key::Escape {
                    event.prevent_default();
                    event.stop_propagation();
                    on_close.call(());
                }
            },
            div { class: "flex items-center gap-2 text-muted-foreground text-sm",
                Button {
                    variant: ButtonVariant::Ghost,
                    size: ButtonSize::Sm,
                    class: "max-[760px]:min-h-control-touch",
                    onclick: move |_| on_close.call(()),

                    "Back to selection"

                }
                span { "Layout findings" }
            }
            div { class: "flex items-baseline justify-between gap-2",
                h2 { class: "m-0 text-[16px]! font-[650]", id: "m1-layout-findings-heading", tabindex: "-1", "Layout findings" }
                Badge { variant: BadgeVariant::Secondary, "{keycaps_fit::presented_findings(&findings, &document).len()}" }
            }
            if groups.is_empty() {
                Empty { class: "p-4", EmptyDescription { "No active findings for this board." } }
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
    let (severity, severity_variant) = match &finding.severity {
        Severity::Error => ("Error", BadgeVariant::Destructive),
        Severity::Warning => ("Warning", BadgeVariant::Warning),
        Severity::Info => ("Information", BadgeVariant::Info),
    };
    let preview_electrical_notice = finding.id.starts_with("preview:electrical:");
    let target = (!preview_electrical_notice)
        .then(|| keycaps_fit::finding_navigation_target(&finding, &document))
        .flatten()
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
        li { class: "my-1.5",
            Item { size: ItemSize::Sm, class: "items-start",
                Badge { variant: severity_variant, "{severity}" }
                p { "{finding.message}" }
                if let (Some(label), Some(request)) = (
                    action_label.filter(|label| !label.is_empty()),
                    request,
                )
                {
                    Button {
                        variant: ButtonVariant::Secondary,
                        class: "wb-finding-action",
                        button_type: "button",
                        onclick: move |_| on_navigate.call(request.clone()),

                        "{label}"

                    }
                }
            }
        }
    }
}
