use super::*;
use boardstudio_application::SessionEpoch;
use boardstudio_core::model::{Contour, Vec2};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::*;
use web_sys::{Element as DomElement, HtmlElement};

wasm_bindgen_test_configure!(run_in_browser);

fn fixture_markers() -> Rc<[FindingMarker]> {
    Rc::from([
        FindingMarker {
            finding_id: "finding-a".into(),
            board_id: "board-a".into(),
            contours: vec![contour(1.0), contour(2.0)],
        },
        FindingMarker {
            finding_id: "finding-b".into(),
            board_id: "board-a".into(),
            contours: vec![contour(3.0)],
        },
        FindingMarker {
            finding_id: "finding-a".into(),
            board_id: "board-b".into(),
            contours: vec![contour(4.0)],
        },
    ])
}

fn contour(offset: f64) -> Contour {
    Contour {
        points: vec![
            Vec2 { x: offset, y: 0.0 },
            Vec2 {
                x: offset + 1.0,
                y: 0.0,
            },
            Vec2 {
                x: offset + 1.0,
                y: 1.0,
            },
        ],
        hole: false,
    }
}

fn fixture_scope(board_id: &str) -> Scope {
    Scope {
        session_epoch: SessionEpoch(7),
        document_id: "document-a".into(),
        board_id: board_id.into(),
        instance_id: None,
    }
}

fn focused(finding_id: &str) -> FocusedFinding {
    FocusedFinding {
        scope: fixture_scope("board-a"),
        token: SnapshotToken(11),
        revision: 8,
        finding_id: finding_id.into(),
        navigation_id: 0,
    }
}

fn mounted_probe() -> Element {
    let mut workspace = use_signal(|| "Layout".to_owned());
    let mut scope = use_signal(|| Some(fixture_scope("board-a")));
    let mut token = use_signal(|| Some(SnapshotToken(11)));
    let mut revision = use_signal(|| Some(8_u64));
    let mut board = use_signal(|| "board-a".to_owned());
    let mut finding = use_signal(|| Some(focused("finding-a")));
    let mut markers = use_signal(fixture_markers);

    rsx! {
        button { id: "board-b", onclick: move |_| board.set("board-b".into()), "Board B" }
        button { id: "other-workspace", onclick: move |_| workspace.set("Keycaps".into()), "Keycaps" }
        button { id: "stale-token", onclick: move |_| token.set(Some(SnapshotToken(12))), "New token" }
        button { id: "stale-revision", onclick: move |_| revision.set(Some(9)), "New revision" }
        button { id: "stale-scope", onclick: move |_| scope.set(Some(fixture_scope("document-b-board"))), "New scope" }
        button { id: "missing-marker", onclick: move |_| markers.set(Rc::from([])), "Remove marker" }
        button { id: "focus-b", onclick: move |_| finding.set(Some(focused("finding-b"))), "Focus finding B" }
        button { id: "restore", onclick: move |_| {
            workspace.set("Layout".into());
            scope.set(Some(fixture_scope("board-a")));
            token.set(Some(SnapshotToken(11)));
            revision.set(Some(8));
            board.set("board-a".into());
            finding.set(Some(focused("finding-a")));
            markers.set(fixture_markers());
        }, "Restore" }
        FocusedFindingMarker {
            workspace: workspace(),
            scope: scope(),
            token: token(),
            revision: revision(),
            active_board_id: board(),
            finding: finding(),
            markers: markers(),
        }
    }
}

#[wasm_bindgen_test]
async fn mounted_layout_marker_is_bound_to_focused_identity_and_accepted_owner() {
    let (root, _mounted) = mount_probe();
    settle().await;

    let marker = focused_marker().unwrap();
    assert_eq!(
        marker.get_attribute("data-finding-id").as_deref(),
        Some("finding-a")
    );
    assert_eq!(
        marker.get_attribute("class").as_deref(),
        Some("wb-outline-finding is-focused")
    );
    assert_eq!(marker.query_selector_all("polygon").unwrap().length(), 2);
    assert!(query("g.wb-outline-finding[data-finding-id='finding-b']").is_none());

    click("focus-b");
    settle().await;
    let marker = focused_marker().unwrap();
    assert_eq!(
        marker.get_attribute("data-finding-id").as_deref(),
        Some("finding-b")
    );
    assert_eq!(marker.query_selector_all("polygon").unwrap().length(), 1);

    click("restore");
    for control in [
        "board-b",
        "other-workspace",
        "stale-token",
        "stale-revision",
        "stale-scope",
        "missing-marker",
    ] {
        click("restore");
        click(control);
        settle().await;
        assert!(
            focused_marker().is_none(),
            "obsolete marker remained after {control}"
        );
    }

    root.remove();
}

#[wasm_bindgen_test]
async fn focused_finding_remains_visible_in_both_themes_and_appearances() {
    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let html = document.document_element().unwrap();
    let original_theme = html.get_attribute("data-theme");
    let original_appearance = html.get_attribute("data-appearance");
    let style = document.create_element("style").unwrap();
    style.set_text_content(Some(&format!(
        "{}\n{}",
        include_str!("../../../assets/workbench-base.css"),
        include_str!("../../../assets/tailwind.css")
    )));
    document.head().unwrap().append_child(&style).unwrap();
    let (root, mut mounted) = mount_probe();
    mounted.own_node(style.clone());
    settle().await;
    let polygon = root
        .query_selector("[data-finding-id='finding-a'] polygon")
        .unwrap()
        .expect("the focused finding has an outline");

    for theme in ["original", "shadcn"] {
        for appearance in ["light", "dark"] {
            html.set_attribute("data-theme", theme).unwrap();
            html.set_attribute("data-appearance", appearance).unwrap();
            let computed = window.get_computed_style(&polygon).unwrap().unwrap();
            let stroke = computed.get_property_value("stroke").unwrap();
            let width = computed.get_property_value("stroke-width").unwrap();
            let opacity = computed.get_property_value("fill-opacity").unwrap();
            assert_ne!(stroke, "none", "{theme}/{appearance} finding stroke");
            assert!(
                width.trim_end_matches("px").parse::<f64>().unwrap() > 1.0,
                "{theme}/{appearance} focused finding must be distinguishable"
            );
            assert!(
                opacity.parse::<f64>().unwrap() > 0.0,
                "{theme}/{appearance} finding fill must be visible"
            );
        }
    }
    for (name, value) in [
        ("data-theme", original_theme),
        ("data-appearance", original_appearance),
    ] {
        if let Some(value) = value {
            html.set_attribute(name, &value).unwrap();
        } else {
            html.remove_attribute(name).unwrap();
        }
    }
    root.remove();
    style.remove();
}

fn mount_probe() -> (DomElement, MountedFixture) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("keycaps-focused-marker-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let mounted = MountedFixture::new(root.clone(), VirtualDom::new(mounted_probe));
    (root, mounted)
}

fn focused_marker() -> Option<DomElement> {
    query("g.wb-outline-finding.is-focused")
}

fn query(selector: &str) -> Option<DomElement> {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector(&format!("#keycaps-focused-marker-test-root {selector}"))
        .unwrap()
}

fn click(id: &str) {
    element(id).click();
}

fn element(id: &str) -> HtmlElement {
    web_sys::window()
        .unwrap()
        .document()
        .unwrap()
        .get_element_by_id(id)
        .unwrap()
        .dyn_into()
        .unwrap()
}

async fn settle() {
    gloo_timers::future::TimeoutFuture::new(40).await;
}
