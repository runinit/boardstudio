//! Layout's findings page and guarded navigation request.
use super::keycaps_fit::{self, FindingNavigationTarget};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Finding, ProjectDoc, Severity};
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

pub struct InspectorMount {
    pub open: bool,
    pub document: Rc<ProjectDoc>,
    pub findings: Vec<Finding>,
    pub source: Source,
    pub on_close: EventHandler<()>,
    pub on_navigate: EventHandler<Request>,
}

#[component]
pub fn LayoutFindingsFooterButton(count: usize, on_toggle: EventHandler<()>) -> Element {
    rsx! {
        Button {
            variant: ButtonVariant::Secondary,
            class: "ml-auto h-auto min-h-[26px] w-auto gap-1.5 rounded-[3px] border-0 bg-transparent px-1.5 py-0.5 text-warning-dark [font:inherit] hover:bg-accent",
            id: "m1-layout-findings-trigger",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() {
                        raw.stop_propagation();
                    }
                }
            },
            button_type: "button",
            onclick: move |_| on_toggle.call(()),

            svg {
                view_box: "0 0 20 20",
                fill: "none",
                stroke: "currentColor",
                stroke_width: "1.5",
                "aria-hidden": "true",
                path { d: "M10 2.5 18 17H2zM10 7v4m0 3v.2" }
            }
            "Layout findings: " Badge { variant: BadgeVariant::Secondary, "{count}" }

        }
    }
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

pub fn target_has_live_layout_destination(
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
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
    use std::cell::RefCell;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn part(id: &str) -> Part {
        Part {
            keycap: None,
            outline: None,
            anchor: None,
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

    fn preview_electrical_pin_finding() -> Finding {
        Finding {
            id: "preview:electrical:pin:net:part:pad-1".into(),
            severity: Severity::Info,
            scope: FindingScope::Pcb,
            message: "Preview removes pad pad-1 on right-U1 (right/U1) from net OMIT_CONTROLLER (right/omit-controller-net).".into(),
            target_ids: vec!["right/omit-controller-net".into(), "right/U1".into()],
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
        let count = keycaps_fit::presented_findings(&host.findings, &host.document).len();
        rsx! {
            LayoutFindingsFooterButton { count, on_toggle: move |_| {} }
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

    async fn mount(root_id: &str, host: Host) -> (web_sys::Element, MountedFixture) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id(root_id);
        document.body().unwrap().append_child(&root).unwrap();
        let dom = VirtualDom::new(findings_host);
        dom.provide_root_context(host);
        let mounted = MountedFixture::new(root.clone(), dom);
        wait_until("the actual findings Inspector mounts", || {
            by_role_name(&root, "button", "Back to selection").is_some()
        })
        .await;
        (root, mounted)
    }

    #[wasm_bindgen_test]
    async fn findings_navigation_and_heading_keep_their_layout_without_legacy_classes() {
        let (root, mut mounted) = mount(
            "findings-layout-test-root",
            Host {
                document: Rc::new(document()),
                findings: vec![],
                requests: Rc::default(),
            },
        )
        .await;
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(&format!(
            "{}\n{}",
            include_str!("../../../assets/workbench-base.css"),
            include_str!("../../../assets/tailwind.css")
        )));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        mounted.own_node(stylesheet.clone());
        let back = by_role_name(&root, "button", "Back to selection").unwrap();
        let navigation = back.parent_element().unwrap();
        let heading = document
            .get_element_by_id("m1-layout-findings-heading")
            .unwrap();
        let heading_row = heading.parent_element().unwrap();
        // Simulate deleting the old stylesheet's selectors, while retaining all
        // new utilities on the actual rendered elements.
        for element in [&navigation, &heading_row] {
            let classes = element.get_attribute("class").unwrap_or_default();
            element
                .set_attribute(
                    "class",
                    &classes
                        .split_whitespace()
                        .filter(|class| !class.starts_with("m1-"))
                        .collect::<Vec<_>>()
                        .join(" "),
                )
                .unwrap();
        }
        let window = web_sys::window().unwrap();
        let navigation_style = window.get_computed_style(&navigation).unwrap().unwrap();
        let heading_style = window.get_computed_style(&heading_row).unwrap().unwrap();
        assert_eq!(
            navigation_style.get_property_value("display").unwrap(),
            "flex"
        );
        assert_eq!(navigation_style.get_property_value("gap").unwrap(), "8px");
        assert_eq!(heading_style.get_property_value("display").unwrap(), "flex");
        assert_eq!(
            heading_style.get_property_value("justify-content").unwrap(),
            "space-between"
        );
        assert_eq!(
            window
                .get_computed_style(&heading)
                .unwrap()
                .unwrap()
                .get_property_value("font-size")
                .unwrap(),
            "16px"
        );
        stylesheet.remove();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn findings_footer_stays_compact_without_legacy_classes() {
        let (root, mut mounted) = mount(
            "findings-footer-layout-test-root",
            Host {
                document: Rc::new(document()),
                findings: vec![],
                requests: Rc::default(),
            },
        )
        .await;
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(include_str!("../../../assets/tailwind.css")));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        mounted.own_node(stylesheet.clone());
        let button = by_role_name(&root, "button", "Layout findings: 0").unwrap();
        let classes = button.get_attribute("class").unwrap_or_default();
        button
            .set_attribute(
                "class",
                &classes
                    .split_whitespace()
                    .filter(|class| !class.starts_with("m1-"))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
            .unwrap();
        let style = web_sys::window()
            .unwrap()
            .get_computed_style(&button)
            .unwrap()
            .unwrap();
        assert_eq!(style.get_property_value("min-height").unwrap(), "26px");
        assert_eq!(style.get_property_value("padding-left").unwrap(), "6px");
        stylesheet.remove();
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn finding_action_is_rendered_only_for_a_live_target_and_a_stale_target_has_none() {
        let requests = Rc::new(RefCell::new(Vec::new()));
        let (live, _live_mounted) = mount(
            "layout-findings-live-test-root",
            Host {
                document: Rc::new(document()),
                findings: vec![finding("left-part")],
                requests: requests.clone(),
            },
        )
        .await;
        let action = by_role_name(&live, "button", "Select affected geometry")
            .expect("a live Part target must offer navigation");
        assert_eq!(
            action.text_content().as_deref(),
            Some("Select affected geometry")
        );
        action.click();
        wait_until("finding navigation reaches its owner", || {
            !requests.borrow().is_empty()
        })
        .await;
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
        let (stale, _stale_mounted) = mount(
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
            by_role_name(&stale, "button", "Select affected geometry").is_none(),
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
        let (moved, _moved_mounted) = mount(
            "layout-findings-moved-test-root",
            Host {
                document: Rc::new(moved_document),
                findings: vec![finding("left-part")],
                requests: moved_requests.clone(),
            },
        )
        .await;
        by_role_name(&moved, "button", "Select affected geometry")
            .expect("a part that moved boards navigates to its current board")
            .click();
        wait_until("moved finding navigation reaches its owner", || {
            !moved_requests.borrow().is_empty()
        })
        .await;
        assert!(matches!(
            moved_requests.borrow().as_slice(),
            [Request { target: FindingNavigationTarget::Part { board_id, .. }, .. }] if board_id == "right"
        ));
        moved.remove();
    }

    #[wasm_bindgen_test]
    async fn preview_electrical_info_finding_is_visible_but_has_no_navigation_action() {
        let mut accepted_document = document();
        accepted_document.boards[1].part_ids = vec!["right/U1".into()];
        accepted_document.parts[1].id = "right/U1".into();
        accepted_document.parts[1].reference = "right-U1".into();
        let (preview, _mounted) = mount(
            "layout-findings-preview-electrical-test-root",
            Host {
                document: Rc::new(accepted_document),
                findings: vec![preview_electrical_pin_finding()],
                requests: Rc::new(RefCell::new(Vec::new())),
            },
        )
        .await;

        let text = preview.text_content().unwrap_or_default();
        assert!(text.contains("Layout findings: 1"));
        assert!(text.contains("Information"));
        assert!(text.contains("Preview removes pad pad-1 on right-U1 (right/U1)"));
        assert!(text.contains("net OMIT_CONTROLLER (right/omit-controller-net)"));
        assert!(
            by_role_name(&preview, "button", "Select affected geometry").is_none(),
            "a preview-only electrical notice remains passive even when its Part target is live"
        );
        preview.remove();
    }
}
