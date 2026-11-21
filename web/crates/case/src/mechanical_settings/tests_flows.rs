#![allow(unused_imports)]
use super::tests_support::*;
use super::*;
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_model::model::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::ui::button::Button;
use boardstudio_web_ui_shared::{
    test_role::{all_by_role_name, by_role_name},
    test_wait::wait_until,
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

#[component]
fn PairFeatureTestPage() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let rows: Rc<[MechanicalPairFeatureRow]> = Rc::from([
        MechanicalPairFeatureRow {
            feature_id: "mount-1-left".into(),
            kind: MechanicalFeatureKind::Mount,
            state: MechanicalFeatureState::Linked,
            instance_id: "left".into(),
            counterpart_instance_id: "right".into(),
            counterpart_feature_id: "mount-1-right".into(),
            preview: Some("Mount at (2.0, 3.0)".into()),
            own_geometry: Some("Mount at (2.0, 3.0)".into()),
            rejoin_fingerprint: Some("reviewed-mount".into()),
        },
        MechanicalPairFeatureRow {
            feature_id: "opening-1-left".into(),
            kind: MechanicalFeatureKind::Opening,
            state: MechanicalFeatureState::Local,
            instance_id: "left".into(),
            counterpart_instance_id: "right".into(),
            counterpart_feature_id: "opening-1-right".into(),
            preview: Some("Opening with 3 vertices".into()),
            own_geometry: Some("Opening with 4 vertices".into()),
            rejoin_fingerprint: Some("reviewed-opening".into()),
        },
        MechanicalPairFeatureRow {
            feature_id: "support-1-left".into(),
            kind: MechanicalFeatureKind::GasketSupport,
            state: MechanicalFeatureState::Omitted,
            instance_id: "left".into(),
            counterpart_instance_id: "right".into(),
            counterpart_feature_id: "support-1-right".into(),
            preview: None,
            own_geometry: None,
            rejoin_fingerprint: Some("reviewed-support".into()),
        },
    ]);
    rsx! {
        div { id: "case-pair-feature-test-root",
            MechanicalSettings {
                identity: test_identity(),
                request_sequence,
                values: Some(test_values(HardwareTransport::Wired, None)),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from([]),
                stabilizer_fits: Rc::from([]),
                layers: Rc::from([]),
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings: Rc::from([]),
                pair_features: Some(rows),
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    last_request.set(format!(
                        "{:?}|reviewed={:?}",
                        request.patch, request.reviewed_fingerprint
                    ));
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
            p { id: "case-pair-feature-last-request", "{last_request}" }
        }
    }
}

#[wasm_bindgen_test]
async fn pair_feature_controls_are_reachable_and_submit_typed_pair_changes() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-pair-feature-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(PairFeatureTestPage));
    rendered().await;

    // The panel renders the accepted pair state for every covered kind.
    by_role_name(&root, "button", "Linked halves")
        .unwrap()
        .click();
    rendered().await;
    let panel = root.text_content().unwrap();
    assert!(
        panel.contains("Mirrored pair"),
        "the pair panel renders its heading, got {panel}"
    );
    assert!(
        panel.contains("Mount · linked to the other half"),
        "the linked row states its relationship, got {panel}"
    );

    // Linked: make local, or remove on this half.
    by_role_name(&root, "button", "Make local mount-1-left")
        .unwrap()
        .click();
    rendered().await;
    let local = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    assert!(
        local.contains("MakeFeatureLocal") && local.contains("mount-1-left"),
        "the linked row submits an explicit make-local, got {local}"
    );
    by_role_name(&root, "button", "Remove on this half mount-1-left")
        .unwrap()
        .click();
    rendered().await;
    let omit = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    assert!(
        omit.contains("OmitFeatureMember") && omit.contains("mount-1-left"),
        "the linked row can remove the feature on this half, got {omit}"
    );

    // Local: the preview is shown before the commit, and rejoin names its source.
    assert!(
        root.text_content()
            .unwrap()
            .contains("Rejoin takes Opening with 3 vertices from right"),
        "a rejoin previews the geometry it will take"
    );
    by_role_name(&root, "button", "Rejoin opening-1-left from the other half")
        .unwrap()
        .click();
    rendered().await;
    let rejoin = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    assert!(
        rejoin.contains("RejoinFeatureMember")
            && rejoin.contains("opening-1-left")
            && rejoin.contains("\"right\""),
        "rejoin submits the reviewed source half, got {rejoin}"
    );
    assert!(
        rejoin.contains("reviewed=Some(\"reviewed-opening\")"),
        "the rejoin submission carries the fingerprint of the row the user reviewed, got {rejoin}"
    );

    // A local half can deliberately take its OWN geometry as the shared value.
    by_role_name(&root, "button", "Rejoin source for opening-1-left")
        .unwrap()
        .click();
    wait_until("the rejoin sources open", || {
        by_role_name(&root, "listbox", "Rejoin source for opening-1-left").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "option", "this half · Opening with 4 vertices").is_some(),
        "the review shows this half's own geometry as a source choice"
    );
    tap_option(&by_role_name(&root, "option", "this half · Opening with 4 vertices").unwrap());
    rendered().await;
    assert!(
        root.text_content()
            .unwrap()
            .contains("Rejoin takes Opening with 4 vertices from left"),
        "the preview follows the chosen source, got {}",
        root.text_content().unwrap()
    );
    by_role_name(&root, "button", "Rejoin opening-1-left from this half")
        .unwrap()
        .click();
    rendered().await;
    let own_source = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    assert!(
        own_source.contains("RejoinFeatureMember")
            && own_source.contains("source_instance_id: \"left\""),
        "the deliberate own-source rejoin is submitted as reviewed, got {own_source}"
    );

    // Omitted without counterpart geometry: reachable but not committable.
    let last_reviewed = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    let omitted_rejoin =
        by_role_name(&root, "button", "Rejoin support-1-left from the other half").unwrap();
    assert_eq!(
        js_sys::Reflect::get(&omitted_rejoin, &"disabled".into())
            .unwrap()
            .as_bool(),
        Some(true),
        "an omitted feature whose other half has no geometry cannot be rejoined"
    );
    assert!(
        root.text_content()
            .unwrap()
            .contains("has no geometry for this feature"),
    );
    let after = element("#case-pair-feature-last-request")
        .text_content()
        .unwrap();
    assert_eq!(
        after, last_reviewed,
        "the disabled rejoin submits nothing when clicked or left alone"
    );
    let _ = MechanicalFeatureKind::GasketSupport;
    root.remove();
}

#[component]
fn PairedDisableTestPage() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let mut requests = use_signal(|| 0_u32);
    rsx! {
        div { id: "case-paired-disable-test-root",
            MechanicalSettings {
                identity: test_identity(),
                request_sequence,
                values: Some(test_values(HardwareTransport::Wired, None)),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from([]),
                stabilizer_fits: Rc::from([]),
                layers: Rc::from([]),
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings: Rc::from([]),
                pair_counterpart: Some("right".into()),
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    requests += 1;
                    last_request.set(format!("{:?}", request.patch));
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
            p { id: "case-paired-disable-last-request", "{last_request}" }
            p { id: "case-paired-disable-count", "{requests}" }
        }
    }
}

#[component]
fn PairedRemovalTestPage() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let mut requests = use_signal(|| 0_u32);
    rsx! {
        div { id: "case-paired-removal-test-root",
            MechanicalSettings {
                identity: test_identity_with_instance("left"),
                request_sequence,
                values: Some(test_values(HardwareTransport::Wired, None)),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from([]),
                stabilizer_fits: Rc::from([]),
                layers: Rc::from([]),
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings: Rc::from([]),
                pair_counterpart: Some("right".into()),
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    requests += 1;
                    last_request.set(format!("{:?}", request.patch));
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
            p { id: "case-paired-removal-last-request", "{last_request}" }
            p { id: "case-paired-removal-count", "{requests}" }
        }
    }
}

#[component]
fn LegacyConstructionTestPage() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let mut requests = use_signal(|| 0_u32);
    rsx! {
        div { id: "case-legacy-construction-test-root",
            MechanicalSettings {
                identity: test_identity_with_instance("left"),
                request_sequence,
                values: Some(test_values(HardwareTransport::Wired, None)),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from([]),
                stabilizer_fits: Rc::from([]),
                layers: Rc::from([]),
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings: Rc::from([]),
                construction_is_legacy: true,
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    requests += 1;
                    last_request.set(format!("{:?}", request.patch));
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
            p { id: "case-legacy-construction-last-request", "{last_request}" }
            p { id: "case-legacy-construction-count", "{requests}" }
        }
    }
}

#[wasm_bindgen_test]
async fn a_legacy_half_converts_its_construction_only_after_confirming_the_change() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-legacy-construction-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(LegacyConstructionTestPage));
    rendered().await;

    // Reachable only for a legacy half, and it submits nothing on its own.
    let make_local = by_role_name(&root, "button", "Make construction local for left").unwrap();
    let construction = make_local.closest("fieldset").unwrap().unwrap();
    assert!(
        construction
            .class_name()
            .split_whitespace()
            .any(|class| class == "grid")
    );
    assert!(make_local.class_name().contains("min-h-control-touch"));
    assert!(
        element("#case-legacy-construction-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "showing the conversion control submits nothing"
    );

    // The confirmation states the whole change before anything is submitted.
    make_local.click();
    rendered().await;
    let confirmation = construction.text_content().unwrap();
    assert!(
        confirmation.contains("left")
            && confirmation.contains("this half's local values")
            && confirmation.contains("Other legacy halves keep their previous baseline")
            && confirmation.contains("One Undo"),
        "the confirmation states what converting construction does, got {confirmation}"
    );
    assert!(
        element("#case-legacy-construction-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "asking for confirmation submits nothing"
    );

    // Cancelling keeps the shared construction.
    by_role_name(&root, "button", "Keep shared construction for left")
        .unwrap()
        .click();
    rendered().await;
    assert!(
        element("#case-legacy-construction-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "cancelling submits nothing"
    );

    // Confirming submits exactly one typed conversion naming one instance.
    by_role_name(&root, "button", "Make construction local for left")
        .unwrap()
        .click();
    rendered().await;
    by_role_name(
        &root,
        "button",
        "Confirm making construction local for left",
    )
    .unwrap()
    .click();
    rendered().await;
    let submitted = element("#case-legacy-construction-last-request")
        .text_content()
        .unwrap();
    assert!(
        submitted.contains("MaterializeLegacyConstruction")
            && submitted.contains("instance_ids: [\"left\"]"),
        "one typed conversion names exactly this half, got {submitted}"
    );
    assert_eq!(
        element("#case-legacy-construction-count")
            .text_content()
            .unwrap(),
        "1",
        "exactly one conversion edit is submitted"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_paired_half_removes_only_after_confirming_what_happens() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-paired-removal-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(PairedRemovalTestPage));
    rendered().await;

    // The control exists for a linked half and nothing is submitted on its own.
    let remove = by_role_name(&root, "button", "Remove assembly left").unwrap();
    assert!(
        element("#case-paired-removal-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "showing the removal control submits nothing"
    );

    // The first click asks for explicit confirmation of the whole change.
    remove.click();
    rendered().await;
    let confirmation = root.text_content().unwrap();
    assert!(
        confirmation.contains("right")
            && confirmation.contains("keeps the geometry and settings")
            && confirmation.contains("boards, PCBs and parts stay")
            && confirmation.contains("One Undo"),
        "the confirmation says what removing this half does, got {confirmation}"
    );
    assert!(
        element("#case-paired-removal-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "asking for confirmation submits nothing"
    );

    // Cancelling keeps everything.
    by_role_name(&root, "button", "Keep assembly left")
        .unwrap()
        .click();
    rendered().await;
    assert!(
        element("#case-paired-removal-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "cancelling submits nothing"
    );

    // Confirming submits exactly one typed physical-instance removal.
    by_role_name(&root, "button", "Remove assembly left")
        .unwrap()
        .click();
    rendered().await;
    by_role_name(&root, "button", "Confirm removing assembly left")
        .unwrap()
        .click();
    rendered().await;
    let submitted = element("#case-paired-removal-last-request")
        .text_content()
        .unwrap();
    assert!(
        submitted.contains("RemovePhysicalInstance"),
        "the confirmation submits the typed removal, got {submitted}"
    );
    assert_eq!(
        element("#case-paired-removal-count")
            .text_content()
            .unwrap(),
        "1",
        "one typed removal is submitted"
    );
    root.remove();
}

fn mount_collection_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    rsx! {
        MountCollectionControls {
            identity: test_identity(),
            request_sequence,
            on_request: move |request: MechanicalSettingsRequest| {
                last_request.set(format!("{:?}", request.patch))
            },
            editable: true,
            feedback: Rc::from([]),
            collection: MechanicalMountCollection::Suspension,
            label: "Suspension mounts",
            mounts: vec![Mount {
                id: "mount-1".into(),
                at: Vec2 { x: 2.0, y: 3.0 },
                kind: MountKind::Hole,
                hole_diameter: 3.2,
                boss_diameter: None,
                height: None,
            }],
            reserved_mount_ids: vec!["mount-1".into()],
            allow_add: true,
        }
        p { id: "case-mount-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn mount_collection_names_its_disclosure_and_submits_typed_mount_changes() {
    let _mounted = mount_battery_test_page("case-mount-test-root", mount_collection_test_page);
    rendered().await;
    let root = element("#case-mount-test-root");
    by_role_name(&root, "button", "Suspension mounts · 1")
        .unwrap()
        .click();
    rendered().await;
    by_role_name(&root, "button", "Mount type").unwrap().click();
    wait_until("the mount types open", || {
        by_role_name(&root, "listbox", "Mount type").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "Boss").unwrap());
    rendered().await;
    let kind = element("#case-mount-last-request").text_content().unwrap();
    assert!(
        kind.contains("SetMountKind") && kind.contains("Boss"),
        "the mount type submits its typed change, got {kind}"
    );
    by_role_name(&root, "button", "Remove Hole 1")
        .unwrap()
        .click();
    rendered().await;
    let removed = element("#case-mount-last-request").text_content().unwrap();
    assert!(
        removed.contains("RemoveMount") && removed.contains("mount-1"),
        "the row removes its own mount, got {removed}"
    );
    root.remove();
}

fn opening_controls_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let mut values = test_values(HardwareTransport::Wired, None);
    values.openings = vec![
        MechanicalOpening {
            id: "opening-1".into(),
            z: 0.0,
            height: 10.0,
            next_vertex_identity: 0,
            vertices: (1..=4)
                .map(|vertex| MechanicalOpeningVertex {
                    id: format!("vertex-{vertex}"),
                    point: Vec2 {
                        x: f64::from(vertex),
                        y: 0.0,
                    },
                })
                .collect(),
        },
        MechanicalOpening {
            id: "opening-2".into(),
            z: 0.0,
            height: 10.0,
            next_vertex_identity: 0,
            vertices: (1..=2)
                .map(|vertex| MechanicalOpeningVertex {
                    id: format!("short-{vertex}"),
                    point: Vec2 {
                        x: f64::from(vertex),
                        y: 0.0,
                    },
                })
                .collect(),
        },
    ];
    rsx! {
        OpeningControls {
            identity: test_identity(),
            request_sequence,
            values,
            editable: true,
            feedback: Rc::from([]),
            on_request: move |request: MechanicalSettingsRequest| {
                last_request.set(format!("{:?}", request.patch))
            },
            owner_key: "opening-test".to_owned(),
        }
        p { id: "case-opening-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn opening_controls_name_their_disclosure_and_submit_typed_opening_changes() {
    let _mounted = mount_battery_test_page("case-opening-test-root", opening_controls_test_page);
    rendered().await;
    let root = element("#case-opening-test-root");
    by_role_name(&root, "button", "Case openings · 2")
        .unwrap()
        .click();
    rendered().await;
    by_role_name(&root, "button", "Remove access opening 1 vertex 1")
        .unwrap()
        .click();
    rendered().await;
    let removed = element("#case-opening-last-request")
        .text_content()
        .unwrap();
    assert!(
        removed.contains("RemoveOpeningVertex")
            && removed.contains("opening-1")
            && removed.contains("vertex-1"),
        "the vertex removal names its opening and vertex, got {removed}"
    );
    by_role_name(&root, "button", "Add vertex to volume 1")
        .unwrap()
        .click();
    rendered().await;
    let added = element("#case-opening-last-request")
        .text_content()
        .unwrap();
    assert!(
        added.contains("AddOpeningVertex") && added.contains("opening-1"),
        "the vertex addition names its opening, got {added}"
    );
    by_role_name(&root, "button", "Remove volume 2")
        .unwrap()
        .click();
    rendered().await;
    let volume = element("#case-opening-last-request")
        .text_content()
        .unwrap();
    assert!(
        volume.contains("RemoveOpening") && volume.contains("opening-2"),
        "the volume removal names its opening, got {volume}"
    );
    assert!(
        root.text_content()
            .unwrap()
            .contains("An opening footprint needs at least three vertices."),
        "a short footprint keeps its blocking validation message"
    );
    root.remove();
}

fn closure_test_hardware() -> InternalClosureHardware {
    InternalClosureHardware {
        drive: ScrewDrive::Hex,
        installation: InsertInstallation::HeatSet,
        thread_diameter: 2.0,
        pitch: 0.4,
        insert_length: 3.0,
        thread_start: 0.5,
        tip_allowance: 0.5,
        seat_lead_depth: 1.0,
        seat_lead_diameter: 2.2,
        bearing_thickness: 1.0,
        fixed_length: None,
        length_datum: ScrewLengthDatum::UnderHead,
        head_profile: ScrewHeadProfile::Flat,
        id: "closure-1".into(),
        thread: "M2".into(),
        screw_lengths: vec![3.0, 4.0],
        head_diameter: 3.8,
        head_height: 2.0,
        hole_diameter: 2.2,
        insert_diameter: 3.2,
        seat_diameter: 4.0,
        seat_depth: 1.0,
        engagement: 2.5,
        bottoming_clearance: 0.5,
        roof: 1.0,
        surround: 1.5,
    }
}

fn closure_controls_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    rsx! {
        GasketClosureControls {
            identity: test_identity(),
            request_sequence,
            hardware: Some(closure_test_hardware()),
            editable: true,
            feedback: Rc::from([]),
            on_request: move |request: MechanicalSettingsRequest| {
                last_request.set(format!("{:?}", request.patch))
            },
            owner_key: "closure-test".to_owned(),
        }
        p { id: "case-closure-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn closure_controls_name_their_disclosures_and_submit_typed_closure_changes() {
    let _mounted = mount_battery_test_page("case-closure-test-root", closure_controls_test_page);
    rendered().await;
    let root = element("#case-closure-test-root");
    assert!(
        by_role_name(&root, "button", "Closure hardware · M2").is_some(),
        "the open closure disclosure names its thread"
    );
    by_role_name(&root, "button", "Insert size")
        .unwrap()
        .click();
    wait_until("the insert sizes open", || {
        by_role_name(&root, "listbox", "Insert size").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "M2 × L4 × OD3.2").unwrap());
    rendered().await;
    let preset = element("#case-closure-last-request")
        .text_content()
        .unwrap();
    assert!(
        preset.contains("SetClosureInsertPreset") && preset.contains("m2-4"),
        "the insert size submits its typed preset, got {preset}"
    );
    by_role_name(&root, "button", "Advanced closure dimensions · Custom · mm")
        .unwrap()
        .click();
    rendered().await;
    by_role_name(&root, "button", "Screw head profile")
        .unwrap()
        .click();
    wait_until("the head profiles open", || {
        by_role_name(&root, "listbox", "Screw head profile").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "Countersunk").unwrap());
    rendered().await;
    let head = element("#case-closure-last-request")
        .text_content()
        .unwrap();
    assert!(
        head.contains("SetClosureHeadProfile") && head.contains("Countersunk"),
        "the head profile submits its typed change, got {head}"
    );
    root.remove();
}

fn critical_fit_controls_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    rsx! {
        CriticalFitControls {
            identity: test_identity(),
            request_sequence,
            fits: vec![MechanicalCriticalFit {
                id: "fit-1".into(),
                part_id: "plate".into(),
                label: "Plate gap".into(),
                from: Vec2 { x: 0.0, y: 0.0 },
                to: Vec2 { x: 10.0, y: 0.0 },
                tolerance: "±0.1".into(),
            }],
            hardware: vec![MechanicalHardwareSpecification {
                id: "hw-1".into(),
                part_id: "bottom".into(),
                feature_id: "mount-1".into(),
                designation: "M2x4".into(),
                thread: "M2".into(),
                length: 4.0,
                quantity: 4,
                notes: None,
                tolerance: None,
            }],
            hardware_mounts: Rc::from([
                MechanicalHardwareMount {
                    part_id: "bottom".into(),
                    part_name: "Bottom".into(),
                    feature_id: "mount-1".into(),
                    kind: MountKind::Hole,
                    x: 2.0,
                    y: 3.0,
                },
                MechanicalHardwareMount {
                    part_id: "bottom".into(),
                    part_name: "Bottom".into(),
                    feature_id: "mount-2".into(),
                    kind: MountKind::Boss,
                    x: 5.0,
                    y: 6.0,
                },
            ]),
            fit_parts: Rc::from([MechanicalFitPart {
                id: "plate".into(),
                name: "Plate".into(),
            }]),
            fit_parts_resolved: true,
            values: test_values(HardwareTransport::Wired, None),
            editable: true,
            feedback: Rc::from([]),
            on_request: move |request: MechanicalSettingsRequest| {
                last_request.set(format!("{:?}", request.patch))
            },
            owner_key: "fits-test".to_owned(),
        }
        p { id: "case-fits-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn critical_fit_controls_name_their_disclosure_and_submit_typed_fit_changes() {
    let _mounted = mount_battery_test_page("case-fits-test-root", critical_fit_controls_test_page);
    rendered().await;
    let root = element("#case-fits-test-root");
    assert!(
        by_role_name(
            &root,
            "button",
            "Hardware & critical fits · 1 hardware · 1 fits"
        )
        .is_some(),
        "the fits disclosure names its counts"
    );
    let outputs = root.query_selector_all("output").unwrap();
    let measurement = (0..outputs.length())
        .filter_map(|index| outputs.item(index))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .find(|output| {
            output
                .text_content()
                .is_some_and(|text| text.contains("Measured 10.00 mm"))
        })
        .expect("the measured fit is displayed");
    let classes = measurement.get_attribute("class").unwrap_or_default();
    for class in ["text-muted-foreground", "text-[11px]", "font-mono"] {
        assert!(
            classes.split_whitespace().any(|name| name == class),
            "measurement uses {class}"
        );
    }
    let paragraphs = root.query_selector_all("p").unwrap();
    let help = (0..paragraphs.length())
        .filter_map(|index| paragraphs.item(index))
        .filter_map(|node| node.dyn_into::<web_sys::Element>().ok())
        .find(|paragraph| {
            paragraph
                .text_content()
                .is_some_and(|text| text.contains("Endpoints are document XY coordinates"))
        })
        .expect("the fit coordinate guidance is displayed");
    let classes = help.get_attribute("class").unwrap_or_default();
    for class in ["text-muted-foreground", "text-sm", "leading-normal"] {
        assert!(
            classes.split_whitespace().any(|name| name == class),
            "guidance uses {class}"
        );
    }
    by_role_name(&root, "button", "Mount link for hardware hw-1")
        .unwrap()
        .click();
    wait_until("the mount links open", || {
        by_role_name(&root, "listbox", "Mount link for hardware hw-1").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "Bottom · boss · 5.0, 6.0 mm").unwrap());
    rendered().await;
    let link = element("#case-fits-last-request").text_content().unwrap();
    assert!(
        link.contains("SetHardwareMount") && link.contains("mount-2"),
        "the mount link submits its typed change, got {link}"
    );
    by_role_name(&root, "button", "Remove hardware hw-1")
        .unwrap()
        .click();
    rendered().await;
    let hardware = element("#case-fits-last-request").text_content().unwrap();
    assert!(
        hardware.contains("RemoveHardware") && hardware.contains("hw-1"),
        "the hardware removal names its item, got {hardware}"
    );
    by_role_name(&root, "button", "Remove fit fit-1")
        .unwrap()
        .click();
    rendered().await;
    let fit = element("#case-fits-last-request").text_content().unwrap();
    assert!(
        fit.contains("RemoveCriticalFit") && fit.contains("fit-1"),
        "the fit removal names its item, got {fit}"
    );
    root.remove();
}

fn manufacturing_controls_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let stabilizer = MechanicalStabilizerOverride {
        profile: None,
        part_id: "plate".into(),
        kind: MechanicalStabilizerKind::PcbMount,
        units: 6.25,
        rotation: None,
    };
    rsx! {
        ManufacturingControls {
            identity: test_identity(),
            targets: Rc::from([
                MechanicalProcessTarget {
                    part_id: "plate".into(),
                    name: "Plate".into(),
                    default_thickness: 1.5,
                    process: Some(MechanicalPartProcess {
                        part_id: "plate".into(),
                        method: PlateMethod::Printed,
                        material: "PLA".into(),
                        thickness: 1.5,
                        constraints_version: String::new(),
                    }),
                },
                MechanicalProcessTarget {
                    part_id: "bottom".into(),
                    name: "Bottom".into(),
                    default_thickness: 2.0,
                    process: None,
                },
            ]),
            stabilizers: Rc::from([MechanicalStabilizerFit {
                part_id: "plate".into(),
                reference: "Space".into(),
                key_units: 6.25,
                default: stabilizer.clone(),
                current: stabilizer,
            }]),
            editable: true,
            feedback: Rc::from([]),
            request_sequence,
            on_request: move |request: MechanicalSettingsRequest| {
                last_request.set(format!("{:?}", request.patch))
            },
        }
        p { id: "case-manufacturing-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn manufacturing_controls_submit_typed_process_and_stabilizer_changes() {
    let _mounted = mount_battery_test_page(
        "case-manufacturing-test-root",
        manufacturing_controls_test_page,
    );
    rendered().await;
    let root = element("#case-manufacturing-test-root");
    by_role_name(&root, "button", "Method").unwrap().click();
    wait_until("the methods open", || {
        by_role_name(&root, "listbox", "Method").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "CNC machined").unwrap());
    rendered().await;
    let method = element("#case-manufacturing-last-request")
        .text_content()
        .unwrap();
    assert!(
        method.contains("SetProcessMethod") && method.contains("Cnc"),
        "the method submits its typed change, got {method}"
    );
    by_role_name(&root, "button", "Material").unwrap().click();
    wait_until("the materials open", || {
        by_role_name(&root, "listbox", "Material").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "ABS").unwrap());
    rendered().await;
    let material = element("#case-manufacturing-last-request")
        .text_content()
        .unwrap();
    assert!(
        material.contains("SetProcessMaterial") && material.contains("ABS"),
        "the material submits its typed change, got {material}"
    );
    by_role_name(&root, "button", "Remove process override for Plate")
        .unwrap()
        .click();
    rendered().await;
    let removed = element("#case-manufacturing-last-request")
        .text_content()
        .unwrap();
    assert!(
        removed.contains("RemoveProcessOverride") && removed.contains("plate"),
        "the process removal names its part, got {removed}"
    );
    by_role_name(&root, "button", "Add process override for Bottom")
        .unwrap()
        .click();
    rendered().await;
    let added = element("#case-manufacturing-last-request")
        .text_content()
        .unwrap();
    assert!(
        added.contains("AddProcessOverride") && added.contains("bottom"),
        "the process addition names its part, got {added}"
    );
    by_role_name(&root, "button", "Stabilizer for Space")
        .unwrap()
        .click();
    wait_until("the stabilizers open", || {
        by_role_name(&root, "listbox", "Stabilizer for Space").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "None").unwrap());
    rendered().await;
    let stabilizer = element("#case-manufacturing-last-request")
        .text_content()
        .unwrap();
    assert!(
        stabilizer.contains("SetStabilizerKind") && stabilizer.contains("None"),
        "the stabilizer submits its typed change, got {stabilizer}"
    );
    root.remove();
}

fn diagnostics_test_page() -> Element {
    let mut shown = use_signal(String::new);
    rsx! {
        Diagnostics {
            findings: Rc::from([
                MechanicalFindingRow {
                    id: "finding-error".into(),
                    severity: Severity::Error,
                    message: "Wall too thin".into(),
                },
                MechanicalFindingRow {
                    id: "finding-warning".into(),
                    severity: Severity::Warning,
                    message: "Foam clearance tight".into(),
                },
            ]),
            on_show_finding: move |id| shown.set(id),
        }
        p { id: "case-diagnostics-shown", "{shown}" }
    }
}

#[wasm_bindgen_test]
async fn diagnostics_name_each_finding_and_show_the_chosen_one() {
    let _mounted = mount_battery_test_page("case-diagnostics-test-root", diagnostics_test_page);
    rendered().await;
    let root = element("#case-diagnostics-test-root");
    let text = root.text_content().unwrap_or_default();
    assert!(text.contains("Mechanical diagnostics · 2"));
    assert!(text.contains("Error"));
    assert!(text.contains("Wall too thin"));
    assert!(text.contains("Warning"));
    assert!(text.contains("Foam clearance tight"));
    by_role_name(&root, "button", "Show finding finding-warning")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        element("#case-diagnostics-shown").text_content().as_deref(),
        Some("finding-warning"),
        "the chosen finding is shown"
    );
    root.remove();
}

fn gasket_context_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut selected = use_signal(|| "gasket:gasket-1:lower".to_owned());
    rsx! {
        MechanicalSettings {
            identity: test_identity(),
            request_sequence,
            values: Some(test_values(HardwareTransport::Wired, None)),
            profiles: Rc::from([]),
            profile_targets: Rc::from([]),
            process_targets: Rc::from([]),
            stabilizer_fits: Rc::from([]),
            layers: Rc::from([]),
            gasket_supports: Rc::from([MechanicalGasketSupportRow {
                id: "gasket-1".into(),
                region_id: "region-1".into(),
                outline_key: "outline-1".into(),
                anchor: 0.0,
                pair_id: None,
                length: 10.0,
                width: 5.0,
                unlinked: true,
                fit_error: None,
                is_previous: false,
            }]),
            fit_parts: Rc::from([]),
            fit_parts_resolved: false,
            hardware_mounts: Rc::from([]),
            suggested_mounts: Rc::from([]),
            findings: Rc::from([]),
            selected_layer: selected(),
            mismatch: None,
            editable: true,
            disabled_reason: None,
            feedback: Rc::from([]),
            summary_feedback: None,
            on_request: move |_| {},
            on_select_layer: move |id| selected.set(id),
            on_show_finding: move |_| {},
        }
        p { id: "case-gasket-selected", "{selected}" }
    }
}

#[wasm_bindgen_test]
async fn gasket_context_names_its_return_and_fields() {
    let _mounted = mount_battery_test_page("case-gasket-test-root", gasket_context_test_page);
    rendered().await;
    let root = element("#case-gasket-test-root");
    let text = root.text_content().unwrap_or_default();
    assert!(text.contains("Gasket 1"));
    assert!(text.contains("Fits at this position."));
    assert!(by_role_name(&root, "spinbutton", "Cut length").is_some());
    assert!(by_role_name(&root, "spinbutton", "Pad width").is_some());
    by_role_name(&root, "button", "All gasket settings")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        element("#case-gasket-selected").text_content().as_deref(),
        Some("gaskets"),
        "the gasket view returns to all gasket settings"
    );
    root.remove();
}

fn setup_basics_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let settings = MechanicalSettingsProps {
        identity: test_identity(),
        request_sequence,
        values: Some(test_values(
            HardwareTransport::Wireless,
            Some(MechanicalBattery {
                cable_width: None,
                size: Vec3 {
                    x: 30.0,
                    y: 20.0,
                    z: 6.0,
                },
                at: Vec2 { x: 12.0, y: -4.0 },
                cable_exit: Vec2 { x: 29.0, y: -4.0 },
            }),
        )),
        preview_values: None,
        profiles: Rc::from([]),
        extract_profile: None,
        profile_targets: Rc::from([]),
        process_targets: Rc::from([]),
        stabilizer_fits: Rc::from([]),
        layers: Rc::from([]),
        gasket_supports: Rc::from([]),
        fit_parts: Rc::from([]),
        fit_parts_resolved: false,
        hardware_mounts: Rc::from([]),
        suggested_mounts: Rc::from([]),
        findings: Rc::from([]),
        pair_features: None,
        pair_link_options: None,
        pair_counterpart: None,
        construction_is_legacy: false,
        pair_projection_port: None,
        selected_layer: String::new(),
        mismatch: None,
        editable: true,
        disabled_reason: None,
        feedback: Rc::from([]),
        summary_feedback: None,
        on_request: EventHandler::new(move |request: MechanicalSettingsRequest| {
            last_request.set(format!("{:?}", request.patch))
        }),
        on_select_layer: EventHandler::new(move |_| {}),
        on_show_finding: EventHandler::new(move |_| {}),
    };
    rsx! {
        MechanicalSetupBasics { settings: settings }
        p { id: "case-setup-basics-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn setup_basics_submit_typed_mount_battery_and_process_changes() {
    let _mounted = mount_battery_test_page("case-setup-basics-test-root", setup_basics_test_page);
    rendered().await;
    let root = element("#case-setup-basics-test-root");
    by_role_name(&root, "button", "Case mounting style")
        .unwrap()
        .click();
    wait_until("the mounting styles open", || {
        by_role_name(&root, "listbox", "Case mounting style").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "Gasket mount").unwrap());
    rendered().await;
    let mount = element("#case-setup-basics-last-request")
        .text_content()
        .unwrap();
    assert!(
        mount.contains("SetMount") && mount.contains("Gasket"),
        "the mounting style submits its typed change, got {mount}"
    );
    by_role_name(&root, "checkbox", "Include wireless battery")
        .unwrap()
        .click();
    rendered().await;
    let battery = element("#case-setup-basics-last-request")
        .text_content()
        .unwrap();
    assert_eq!(
        battery.as_str(),
        "SetBatteryEnabled(false)",
        "the battery toggle submits its typed change, got {battery}"
    );
    by_role_name(&root, "button", "Case fabrication process")
        .unwrap()
        .click();
    wait_until("the processes open", || {
        by_role_name(&root, "listbox", "Case fabrication process").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "CNC machined").unwrap());
    rendered().await;
    let process = element("#case-setup-basics-last-request")
        .text_content()
        .unwrap();
    assert!(
        process.contains("SetMethod") && process.contains("Cnc"),
        "the process submits its typed change, got {process}"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn a_paired_half_disables_with_an_explicit_unlink_explanation_and_one_edit() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-paired-disable-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(PairedDisableTestPage));
    rendered().await;

    let explanation = root.text_content().unwrap();
    assert!(
        explanation.contains("right")
            && explanation.contains("keeps its geometry and settings")
            && explanation.contains("neither PCB is removed")
            && explanation.contains("One Undo"),
        "the panel explains what disabling a linked half does, got {explanation}"
    );
    by_role_name(&root, "button", "Unlink and disable this half")
        .unwrap()
        .click();
    rendered().await;
    let submitted = element("#case-paired-disable-last-request")
        .text_content()
        .unwrap();
    assert!(
        submitted.contains("Disable") && !submitted.contains("Unlink"),
        "one typed Disable carries the whole decision, got {submitted}"
    );
    assert_eq!(
        element("#case-paired-disable-count")
            .text_content()
            .unwrap(),
        "1",
        "exactly one edit is submitted, with no separate unlink item"
    );
    root.remove();
}

fn link_options() -> Rc<[MechanicalPairLinkOption]> {
    Rc::from([
        MechanicalPairLinkOption {
            pair_id: "pair/left".into(),
            instance_id: "left".into(),
            other_instance_id: "right".into(),
            other_name: "Right half".into(),
            derived_mapping: Some(Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            }),
            construction_source: "left".into(),
            correspondences: vec![
                boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1".into(),
                    second_feature_id: "mount-1".into(),
                    source_instance_id: None,
                },
            ],
            matched_features: "Mount mount-1".into(),
            matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                kind: MechanicalFeatureKind::Mount,
                first_feature_id: "mount-1-left".into(),
                second_feature_id: "mount-1-right".into(),
                first_geometry: Some("Mount at (2.0, 3.0)".into()),
                second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                source_instance_id: "left".into(),
            }],
            reviewed_fingerprint: Some("reviewed-link".into()),
            preview: Some(crate::mechanical_pair::MechanicalLinkPreview {
                members: vec![
                    crate::mechanical_pair::MechanicalLinkMemberPreview {
                        instance_id: "left".into(),
                        features: vec!["Mount at (2.0, 3.0)".into()],
                        findings: vec![],
                    },
                    crate::mechanical_pair::MechanicalLinkMemberPreview {
                        instance_id: "right".into(),
                        features: vec!["Mount at (-2.0, 3.0)".into()],
                        findings: vec![],
                    },
                ],
            }),
            preview_note: None,
        },
        MechanicalPairLinkOption {
            pair_id: "pair/left".into(),
            instance_id: "left".into(),
            other_instance_id: "lower".into(),
            other_name: "Lower half".into(),
            derived_mapping: None,
            construction_source: "left".into(),
            correspondences: vec![
                boardstudio_core_mechanical::mechanical::change::MechanicalFeatureCorrespondence {
                    kind: MechanicalFeatureKind::Mount,
                    first_feature_id: "mount-1-left".into(),
                    second_feature_id: "mount-1-right".into(),
                    source_instance_id: Some("left".into()),
                },
            ],
            matched_features: "Mount mount-1-left · mount-1-right".into(),
            matched: vec![crate::mechanical_pair::MechanicalPairMatchedFeature {
                kind: MechanicalFeatureKind::Mount,
                first_feature_id: "mount-1-left".into(),
                second_feature_id: "mount-1-right".into(),
                first_geometry: Some("Mount at (2.0, 3.0)".into()),
                second_geometry: Some("Mount at (-2.0, 3.0)".into()),
                source_instance_id: "left".into(),
            }],
            reviewed_fingerprint: Some("reviewed-link-explicit".into()),
            preview: None,
            preview_note: Some("the reviewed configuration needs a catalogue template".into()),
        },
    ])
}

/// A stand-in for the mount's projection port: it reports the reviewed map it was
/// given, which is what the review displays and commits.
fn echo_projection_port() -> crate::mechanical_pair::MechanicalPairProjectionPort {
    crate::mechanical_pair::MechanicalPairProjectionPort(Rc::new(|change| {
        let boardstudio_core_mechanical::mechanical::change::MechanicalChange::LinkMirrorPair {
            mapping,
            ..
        } = change
        else {
            return Err("not a reviewed link".into());
        };
        let text = format!(
            "offset ({:?}, {:?}) rotation {:?}",
            mapping.at.x, mapping.at.y, mapping.rotation
        );
        Ok(crate::mechanical_pair::MechanicalLinkPreview {
            members: vec![
                crate::mechanical_pair::MechanicalLinkMemberPreview {
                    instance_id: "left".into(),
                    features: vec![text.clone()],
                    findings: vec![],
                },
                crate::mechanical_pair::MechanicalLinkMemberPreview {
                    instance_id: "right".into(),
                    features: vec![text],
                    findings: vec![],
                },
            ],
        })
    }))
}

#[component]
fn PairLinkTestPage() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    rsx! {
        div { id: "case-pair-link-test-root",
            MechanicalSettings {
                identity: test_identity(),
                request_sequence,
                values: Some(test_values(HardwareTransport::Wired, None)),
                profiles: Rc::from([]),
                profile_targets: Rc::from([]),
                process_targets: Rc::from([]),
                stabilizer_fits: Rc::from([]),
                layers: Rc::from([]),
                gasket_supports: Rc::from([]),
                fit_parts: Rc::from([]),
                fit_parts_resolved: false,
                hardware_mounts: Rc::from([]),
                suggested_mounts: Rc::from([]),
                findings: Rc::from([]),
                pair_link_options: Some(link_options()),
                pair_projection_port: Some(echo_projection_port()),
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    last_request.set(format!(
                        "{:?}|reviewed={:?}",
                        request.patch, request.reviewed_fingerprint
                    ));
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
            p { id: "case-pair-link-last-request", "{last_request}" }
        }
    }
}

#[wasm_bindgen_test]
async fn link_controls_review_a_candidate_and_submit_the_reviewed_link() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-pair-link-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(PairLinkTestPage));
    rendered().await;

    // The review shows the map the link will use and the durable matches before
    // anything is submitted.
    by_role_name(&root, "button", "Linked halves")
        .unwrap()
        .click();
    rendered().await;
    let review = root.text_content().unwrap();
    assert!(
        review.contains("Mirror another half"),
        "the link review renders its heading, got {review}"
    );
    assert!(
        review.contains("Derived from the halves' declared transforms"),
        "the review explains the map, got {review}"
    );
    assert!(
        review.contains("Mount mount-1-left · mount-1-right"),
        "the review lists the durable-id matches, got {review}"
    );
    assert!(
        element("#case-pair-link-last-request")
            .text_content()
            .unwrap()
            .is_empty(),
        "the review submits nothing by itself"
    );

    // Both halves of the complete proposed result are rendered before the commit,
    // read from Core rather than computed in the panel.
    let both_halves = root.text_content().unwrap();
    assert!(
        both_halves.contains("left")
            && both_halves.contains("right")
            && both_halves.contains("offset (0.0, 0.0) rotation 0.0"),
        "the review shows both halves of the proposed result, got {both_halves}"
    );

    // Each match chooses its own source half, independently of the construction
    // source and of the other matches.
    by_role_name(&root, "button", "Source for mount-1-left")
        .unwrap()
        .click();
    wait_until("the source choices open", || {
        by_role_name(&root, "listbox", "Source for mount-1-left").is_some()
    })
    .await;
    assert!(
        by_role_name(&root, "option", "right · Mount at (-2.0, 3.0)").is_some(),
        "the other half's geometry is shown for the choice"
    );
    tap_option(&by_role_name(&root, "option", "right · Mount at (-2.0, 3.0)").unwrap());
    rendered().await;

    by_role_name(&root, "button", "Link left with right")
        .unwrap()
        .click();
    rendered().await;
    let linked = element("#case-pair-link-last-request")
        .text_content()
        .unwrap();
    assert!(
        linked.contains("LinkMirrorPair")
            && linked.contains("pair/left")
            && linked.contains("mount-1"),
        "the confirmed link submits the reviewed pair, got {linked}"
    );
    assert!(
        linked.contains("rotation: 0.0"),
        "the submitted link carries the reviewed map, got {linked}"
    );
    assert!(
        linked.contains("reviewed=Some(\"reviewed-link\")"),
        "the link submission carries the fingerprint of the candidate the user reviewed, got {linked}"
    );
    assert!(
        linked.contains("source_instance_id: Some(\"right\")"),
        "the link submission carries the source the user chose for that feature, got {linked}"
    );
    assert!(
        linked.contains("source_instance_id: Some(\"right\")")
            && linked.matches("source_instance_id: Some(").count() == 1,
        "each match carries its own source choice, got {linked}"
    );

    // A translated frame has no derived map: the review asks for one explicitly.
    by_role_name(&root, "button", "Link with").unwrap().click();
    wait_until("the link candidates open", || {
        by_role_name(&root, "listbox", "Link with").is_some()
    })
    .await;
    tap_option(&by_role_name(&root, "option", "Lower half").unwrap());
    rendered().await;
    assert!(
        by_role_name(&root, "spinbutton", "Pair mapping X").is_some(),
        "a translated frame gets explicit mapping inputs"
    );
    assert!(
        root.text_content()
            .unwrap()
            .contains("complete proposed result is unavailable"),
        "a candidate without a projection says so instead of showing a partial result"
    );
    let blocked_link = by_role_name(&root, "button", "Link left with lower").unwrap();
    assert_eq!(
        js_sys::Reflect::get(&blocked_link, &"disabled".into())
            .unwrap()
            .as_bool(),
        Some(true),
        "the explicit map is required before the link can be committed"
    );

    let mapping_input = |name: &str, value: &str| {
        let input: web_sys::HtmlInputElement = by_role_name(&root, "spinbutton", name)
            .unwrap()
            .dyn_into()
            .unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    };
    mapping_input("Pair mapping X", "30");
    mapping_input("Pair mapping Y", "-5");
    mapping_input("Pair mapping rotation", "180");
    rendered().await;
    by_role_name(&root, "button", "Link left with lower")
        .unwrap()
        .click();
    rendered().await;
    let explicit = element("#case-pair-link-last-request")
        .text_content()
        .unwrap();
    assert!(
        explicit.contains("LinkMirrorPair")
            && explicit.contains("lower")
            && explicit.contains("x: 30.0")
            && explicit.contains("y: -5.0")
            && explicit.contains("rotation: 180.0"),
        "the explicit reviewed map is submitted for a translated frame, got {explicit}"
    );
    assert!(
        explicit.contains("reviewed=Some(\"reviewed-link-explicit\")"),
        "the explicit-map candidate's own review fingerprint travels with it, got {explicit}"
    );
    root.remove();
}
