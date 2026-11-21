//! Ownership presentation tests: the Inspector names Core's persisted ownership,
//! shows a linked refinement's located finding and affected exports, and never
//! presents a legacy fixed version as linked (ADR-0002/0003).
use super::*;
use boardstudio_model::model::{OutlineOwnership, OutlineSettings};
use boardstudio_web_ui_shared::test_mount::MountedFixture;

/// Ownership presentation: the Inspector names Core's persisted ownership, shows a
/// linked refinement's located finding and affected exports, and never presents a
/// legacy fixed version as linked (ADR-0002/0003).
const BOARD: &str = "ownership-board";

fn polygon(id: &str, x: f64) -> OutlineFeature {
    OutlineFeature::Polygon {
        anchor_part_id: None,
        id: id.into(),
        points: vec![
            Vec2 { x, y: 0.0 },
            Vec2 {
                x: x + 20.0,
                y: 0.0,
            },
            Vec2 {
                x: x + 20.0,
                y: 20.0,
            },
            Vec2 { x, y: 20.0 },
        ],
        operation: boardstudio_model::model::Operation::Add,
    }
}

fn version(
    id: &str,
    name: &str,
    ownership: OutlineOwnership,
    features: Vec<OutlineFeature>,
) -> OutlineVersion {
    OutlineVersion {
        id: id.into(),
        name: name.into(),
        source: OutlineProvenance {
            revision: 0,
            version_id: None,
        },
        geometry: OutlineSnapshot {
            features,
            settings: OutlineSettings::default(),
            expected_regions: 1,
            bridges: vec![],
            protected_gaps: vec![],
        },
        ownership,
    }
}

/// A linked refinement that resolves from its own authored contour, plus a legacy
/// fixed version (written before ownership existed, so its serde default is Fixed).
async fn mounted(
    versions: Vec<OutlineVersion>,
    active: &str,
) -> (InspectorProbe, web_sys::Element, MountedFixture) {
    let mut document = ProjectDoc::empty("ownership-doc", "Ownership fixture");
    document.revision = 5;
    document.boards.push(Board {
        id: BOARD.into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.board_outlines.push(BoardOutline {
        board_id: BOARD.into(),
        active_version_id: Some(active.into()),
        versions,
        generated_last_valid: None,
        retired_version_ids: vec![],
    });
    let probe = open_inspector_probe(document).await;
    let (root, mounted) = mount_probe(
        "outline-inspector-mount",
        VirtualDom::new(mounted_outline_inspector_host),
        &probe,
    );
    (probe, root, mounted)
}

fn input(root: &web_sys::Element, name: &str) -> web_sys::HtmlInputElement {
    let role = if name == "Outline version name" {
        "textbox"
    } else {
        "spinbutton"
    };
    by_role_name(root, role, name)
        .unwrap_or_else(|| panic!("{role} {name} is mounted"))
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap()
}

/// The DOM `disabled` property, so a `<select>` and an `<input>` are read the same way.
fn is_disabled(element: &web_sys::Element) -> bool {
    js_sys::Reflect::get(element, &"disabled".into())
        .ok()
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

#[wasm_bindgen_test]
async fn mounted_linked_and_legacy_fixed_versions_are_distinct_ownership_choices() {
    let (_probe, root, _mounted) = mounted(
        vec![
            version(
                "linked-v1",
                "Linked 1",
                OutlineOwnership::Linked,
                vec![polygon("linked-contour", 0.0)],
            ),
            version(
                "legacy-fixed",
                "Edited outline 1",
                OutlineOwnership::Fixed,
                vec![polygon("legacy-contour", 60.0)],
            ),
        ],
        "linked-v1",
    )
    .await;
    settle_dimension().await;

    let labels = active_outline_option_labels(&root).await;
    assert!(
        labels.iter().any(|label| label.contains("Generated")),
        "the permanent Generated outline stays a distinct choice: {labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|label| label == "Linked refinement \u{b7} Linked 1"),
        "a persisted linked refinement is named as linked: {labels:?}"
    );
    assert!(
        labels
            .iter()
            .any(|label| label == "Edited outline \u{b7} Edited outline 1"),
        "a legacy fixed version is presented as an edited outline, never as linked: {labels:?}"
    );
    // Settings stay editable while a linked refinement is selected.
    assert!(
        !is_disabled(
            &by_role_name(&root, "button", "Outline corners")
                .unwrap()
                .into()
        ),
        "the linked refinement's settings remain editable"
    );
    assert!(
        !input(&root, "Outline version name").disabled(),
        "the linked refinement's own choices stay editable"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_linked_refinement_edits_its_own_envelope_settings_and_a_fixed_copy_has_none() {
    let envelope = |id: &str, margin: f64| OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: id.into(),
        part_ids: vec!["missing-part".into()],
        margin,
        operation: boardstudio_model::model::Operation::Add,
    };
    let (_probe, root, _mounted) = mounted(
        vec![
            version(
                "linked-v1",
                "Linked 1",
                OutlineOwnership::Linked,
                vec![envelope("linked-envelope", 4.0)],
            ),
            version(
                "fixed-v1",
                "Edited outline 1",
                OutlineOwnership::Fixed,
                vec![polygon("fixed-contour", 60.0)],
            ),
        ],
        "linked-v1",
    )
    .await;
    settle_dimension().await;

    // The linked refinement shows and edits its own margin, bridge width and cleanup,
    // so a source change never forces a Freeze just to adjust them.
    let margin = input(&root, "Outline margin");
    assert_eq!(margin.value().parse::<f64>().unwrap(), 4.0);
    assert!(
        !margin.disabled(),
        "the linked refinement's margin is editable"
    );
    for selector in ["Bridge width", "Automatic gap cleanup", "Maximum gap span"] {
        assert!(
            !by_role_name(
                &root,
                if selector == "Automatic gap cleanup" {
                    "checkbox"
                } else {
                    "spinbutton"
                },
                selector
            )
            .unwrap()
            .has_attribute("disabled"),
            "{selector} is editable on a linked refinement"
        );
    }
    root.remove();

    // A fixed edited copy has no envelope, so those controls are not offered.
    let (_probe, root, _mounted) = mounted(
        vec![version(
            "fixed-v1",
            "Edited outline 1",
            OutlineOwnership::Fixed,
            vec![polygon("fixed-contour", 60.0)],
        )],
        "fixed-v1",
    )
    .await;
    settle_dimension().await;
    for selector in [
        "Outline margin",
        "Bridge width",
        "Automatic gap cleanup",
        "Maximum gap span",
    ] {
        assert!(
            by_role_name(
                &root,
                if selector == "Automatic gap cleanup" {
                    "checkbox"
                } else {
                    "spinbutton"
                },
                selector
            )
            .is_none(),
            "{selector} is not offered on a fixed copy"
        );
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_freeze_creates_one_fixed_copy_retaining_the_link_and_one_undo_restores_both() {
    let (probe, root, _mounted) = mounted(
        vec![version(
            "linked-v1",
            "Linked 1",
            OutlineOwnership::Linked,
            vec![polygon("linked-contour", 0.0)],
        )],
        "linked-v1",
    )
    .await;
    settle_dimension().await;
    let revision_before = accepted_revision(&probe);
    let freeze = button(&root, "Freeze into an edited copy");
    freeze.click();
    accept(&probe).await;

    let accepted = accepted_document(&probe);
    let state = &accepted.board_outlines[0];
    assert_eq!(
        state.versions.len(),
        2,
        "one action creates exactly one copy"
    );
    let linked = state
        .versions
        .iter()
        .find(|version| version.id == "linked-v1")
        .expect("the linked refinement is retained");
    assert_eq!(
        linked.ownership,
        OutlineOwnership::Linked,
        "Freeze keeps the linked refinement linked"
    );
    let frozen = state
        .versions
        .iter()
        .find(|version| version.id != "linked-v1")
        .expect("the frozen copy exists");
    assert_eq!(frozen.ownership, OutlineOwnership::Fixed);
    assert!(
        frozen.name.starts_with("Edited outline "),
        "the copy is named as an edited outline: {}",
        frozen.name
    );
    assert_eq!(
        state.active_version_id.as_deref(),
        Some(frozen.id.as_str()),
        "the frozen copy becomes the selected version"
    );
    assert!(
        accepted.revision > revision_before,
        "Freeze is one accepted revision"
    );
    settle_dimension().await;
    let labels = active_outline_option_labels(&root).await;
    assert!(
        labels
            .iter()
            .any(|label| label == "Linked refinement \u{b7} Linked 1"),
        "the linked refinement stays visibly available after the freeze copy: {labels:?}"
    );
    assert!(
        root.query_selector("[data-finding-id] [role=alert]")
            .unwrap()
            .is_none(),
        "a resolvable linked refinement reports no problem"
    );

    // One Undo restores both the document state and the selection.
    undo(&probe.runtime).await;
    accept(&probe).await;
    let restored = accepted_document(&probe);
    assert_eq!(
        restored.board_outlines[0].versions.len(),
        1,
        "one Undo removes only the freeze copy"
    );
    assert_eq!(
        restored.board_outlines[0].active_version_id.as_deref(),
        Some("linked-v1"),
        "one Undo restores the selected version as well"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_linked_refinement_that_cannot_resolve_shows_its_located_finding() {
    // The selected refinement owns a generated envelope whose included components are
    // gone, so it cannot resolve and Core keeps its last valid shape.
    let (_probe, root, _mounted) = mounted(
        vec![version(
            "linked-v1",
            "Linked 1",
            OutlineOwnership::Linked,
            vec![OutlineFeature::PartEnvelope {
                connections: vec![],
                settings: OutlineSettings::default(),
                id: "linked-envelope".into(),
                part_ids: vec!["missing-part".into()],
                margin: 4.0,
                operation: boardstudio_model::model::Operation::Add,
            }],
        )],
        "linked-v1",
    )
    .await;
    settle_dimension().await;

    let finding_id = format!("board:{BOARD}:outline:linked:linked-v1");
    let problem = root
        .query_selector(&format!("[data-finding-id='{finding_id}']"))
        .unwrap()
        .unwrap_or_else(|| {
            panic!(
                "the located finding is presented by its id contract, not by message text; body: {}",
                root.inner_html()
            )
        });
    assert!(
        problem.get_attribute("class").is_none(),
        "the located finding does not depend on a legacy styling class"
    );
    let announcement = problem
        .query_selector("[role=alert]")
        .unwrap()
        .expect("the located finding is announced");
    assert_eq!(
        announcement.get_attribute("data-slot").as_deref(),
        Some("alert"),
        "the actual finding adopts the copied Alert"
    );
    assert_eq!(
        problem.query_selector_all("[role=alert]").unwrap().length(),
        1
    );
    let text = announcement.text_content().unwrap_or_default();
    assert!(
        text.contains("cannot resolve"),
        "Core's message is shown as-is: {text}"
    );
    assert!(
        text.contains("PCB"),
        "the affected export is named (only the affected exports are disabled): {text}"
    );
    assert!(
        text.contains("Saving and further editing stay available"),
        "saving and editing stay available: {text}"
    );
    // The linked choice stays selectable and its settings stay editable.
    let labels = active_outline_option_labels(&root).await;
    assert!(
        labels
            .iter()
            .any(|label| label == "Linked refinement \u{b7} Linked 1"),
        "a broken refinement remains an available choice: {labels:?}"
    );
    assert!(
        !input(&root, "Outline version name").disabled(),
        "editing stays possible while the refinement cannot resolve"
    );
    root.remove();
}
