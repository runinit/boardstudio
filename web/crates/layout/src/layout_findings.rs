//! Layout's accepted-scene findings page and guarded navigation request.
use super::keycaps_fit::{self, FindingNavigationTarget};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Finding, ProjectDoc, Severity};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Source {
    pub workspace: &'static str,
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
                if event.key() == Key::Escape {
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{Board, Part, Pose2, Scope as FindingScope, Side, Vec2};
    use std::cell::RefCell;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn part(id: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: "definition".into(),
            reference: id.to_uppercase(),
            pose: Pose2 {
                at: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn board(id: &str, part_ids: &[&str]) -> Board {
        Board {
            id: id.into(),
            name: id.into(),
            outline_ids: vec![],
            part_ids: part_ids.iter().map(|id| (*id).to_owned()).collect(),
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("doc", "Findings fixture");
        document.boards.push(board("left", &["left-part"]));
        document.boards.push(board("right", &["right-part"]));
        document.parts.push(part("left-part"));
        document.parts.push(part("right-part"));
        document
    }

    fn finding(target: &str) -> Finding {
        Finding {
            id: format!("finding-{target}"),
            severity: Severity::Warning,
            scope: FindingScope::Layout,
            message: format!("Check {target}"),
            target_ids: vec![target.into()],
        }
    }

    #[wasm_bindgen_test]
    fn destination_requires_the_target_to_still_live_on_its_named_board() {
        let document = document();
        let live = FindingNavigationTarget::Part {
            board_id: "left".into(),
            part_id: "left-part".into(),
        };
        assert!(target_has_live_layout_destination(&live, &document));

        // Part listed by the board but deleted from the document.
        let mut deleted = document.clone();
        deleted.parts.retain(|part| part.id != "left-part");
        assert!(!target_has_live_layout_destination(&live, &deleted));

        // Part removed from its board (stale target) and part owned by another board.
        let mut unlisted = document.clone();
        unlisted.boards[0].part_ids.clear();
        assert!(!target_has_live_layout_destination(&live, &unlisted));
        let other_board = FindingNavigationTarget::Part {
            board_id: "left".into(),
            part_id: "right-part".into(),
        };
        assert!(!target_has_live_layout_destination(&other_board, &document));

        // Board that no longer exists.
        let gone = FindingNavigationTarget::Board {
            board_id: "removed".into(),
        };
        assert!(!target_has_live_layout_destination(&gone, &document));
        let live_board = FindingNavigationTarget::Board {
            board_id: "right".into(),
        };
        assert!(target_has_live_layout_destination(&live_board, &document));
    }

    #[derive(Clone)]
    struct Host {
        document: Rc<ProjectDoc>,
        findings: Vec<Finding>,
        requests: Rc<RefCell<Vec<Request>>>,
    }

    #[component]
    fn findings_host() -> Element {
        let host = use_context::<Host>();
        let requests = host.requests.clone();
        rsx! {
            LayoutFindingsInspector {
                open: true,
                document: host.document.clone(),
                findings: host.findings.clone(),
                source: Source {
                    workspace: "Layout",
                    scope: Scope {
                        session_epoch: SessionEpoch(1),
                        document_id: "doc".into(),
                        board_id: "left".into(),
                        instance_id: None,
                    },
                    token: SnapshotToken(1),
                    revision: 0,
                    generation: 1,
                },
                on_close: move |_| {},
                on_navigate: move |request| requests.borrow_mut().push(request),
            }
        }
    }

    async fn mount(root_id: &str, host: Host) -> web_sys::Element {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id(root_id);
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(findings_host);
        dom.provide_root_context(host);
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(40).await;
        root
    }

    #[wasm_bindgen_test]
    async fn finding_action_is_rendered_only_for_a_live_target_and_a_stale_target_has_none() {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let live = mount(
            "layout-findings-live-test-root",
            Host {
                document: Rc::new(document()),
                findings: vec![finding("left-part")],
                requests: requests.clone(),
            },
        )
        .await;
        let action = live
            .query_selector("button.wb-finding-action")
            .unwrap()
            .expect("a live Part target must offer navigation");
        assert_eq!(
            action.text_content().as_deref(),
            Some("Select affected geometry")
        );
        action.dyn_into::<web_sys::HtmlElement>().unwrap().click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert!(matches!(
            requests.borrow().as_slice(),
            [Request { target: FindingNavigationTarget::Part { part_id, .. }, .. }] if part_id == "left-part"
        ));
        live.remove();

        // The same finding after the component was deleted keeps the message but
        // exposes no navigation action.
        let mut stale_document = document();
        stale_document.parts.retain(|part| part.id != "left-part");
        stale_document.boards[0].part_ids.clear();
        let stale_requests = Rc::new(RefCell::new(Vec::new()));
        let stale = mount(
            "layout-findings-stale-test-root",
            Host {
                document: Rc::new(stale_document),
                findings: vec![finding("left-part")],
                requests: stale_requests.clone(),
            },
        )
        .await;
        assert!(
            stale
                .text_content()
                .unwrap_or_default()
                .contains("Check left-part"),
            "the finding message stays visible"
        );
        assert!(
            stale
                .query_selector("button.wb-finding-action")
                .unwrap()
                .is_none(),
            "a deleted target must not render a navigation action"
        );
        assert!(stale_requests.borrow().is_empty());
        stale.remove();

        // The part now lives only on the other board: the action must target that
        // board, never the board the stale finding was raised from.
        let mut moved_document = document();
        moved_document.boards[0].part_ids.clear();
        moved_document.boards[1].part_ids.push("left-part".into());
        let moved_requests = Rc::new(RefCell::new(Vec::new()));
        let moved = mount(
            "layout-findings-moved-test-root",
            Host {
                document: Rc::new(moved_document),
                findings: vec![finding("left-part")],
                requests: moved_requests.clone(),
            },
        )
        .await;
        moved
            .query_selector("button.wb-finding-action")
            .unwrap()
            .expect("a part that moved boards navigates to its current board")
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
            .click();
        gloo_timers::future::TimeoutFuture::new(40).await;
        assert!(matches!(
            moved_requests.borrow().as_slice(),
            [Request { target: FindingNavigationTarget::Part { board_id, .. }, .. }] if board_id == "right"
        ));
        moved.remove();
    }
}
