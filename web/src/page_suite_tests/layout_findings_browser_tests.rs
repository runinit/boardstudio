use boardstudio_application::SessionEpoch;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{Board, Part, Pose2, Scope as FindingScope, Side, Vec2};
use boardstudio_core::model::{Finding, ProjectDoc, Severity};
use boardstudio_web_keycaps::keycaps_fit::{self, FindingNavigationTarget};
use boardstudio_web_layout::layout_findings::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
use dioxus::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen_test::wasm_bindgen_test;

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
        include_str!("../../assets/workbench-base.css"),
        include_str!("../../assets/tailwind.css")
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
    stylesheet.set_text_content(Some(include_str!("../../assets/tailwind.css")));
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
