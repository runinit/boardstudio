#![allow(unused_imports)]
use super::*;
use super::tests_support::*;
use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
use boardstudio_model::model::*;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::{
    test_role::{all_by_role_name, by_role_name},
    test_wait::wait_until,
};
use boardstudio_web_ui_shared::ui::button::Button;
use crate::mechanical_pair::{MechanicalPairFeatureRow, MechanicalPairLinkOption, feature_label};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

fn test_page() -> Element {
    test_page_with_previous(false, false)
}

fn previous_test_page() -> Element {
    test_page_with_previous(true, false)
}

fn test_page_with_previous(is_previous: bool, pending: bool) -> Element {
    let mut selected_layer = use_signal(|| "plate".to_owned());
    let mut shown_finding = use_signal(String::new);
    let request_sequence = use_signal(|| 0_u64);
    let identity = test_identity();
    let values = test_values(HardwareTransport::Wired, None);
    let mut process_targets = vec![
        test_process("plate", PlateMethod::Printed, "PLA", 1.5),
        test_process("plate-foam", PlateMethod::CutSheet, "EVA", 0.0),
        test_process("bottom-foam", PlateMethod::CutSheet, "EVA", 0.5),
        test_process("bottom", PlateMethod::Printed, "PLA", 2.0),
    ];
    process_targets.push(MechanicalProcessTarget {
        part_id: "part-1".into(),
        name: "U1 · Fixture part".into(),
        default_thickness: 1.5,
        process: None,
    });
    let layers = Rc::from([
        MechanicalLayerRow {
            id: "plate".into(),
            label: "Plate".into(),
            z: 0.0,
            thickness: 1.5,
            resolved_body_thickness: Some(1.5),
            is_previous,
        },
        MechanicalLayerRow {
            id: "pcb".into(),
            label: "PCB".into(),
            z: 0.0,
            thickness: 1.6,
            resolved_body_thickness: None,
            is_previous: false,
        },
        MechanicalLayerRow {
            id: "plate-foam".into(),
            label: "Plate foam".into(),
            z: 0.0,
            thickness: 0.5,
            resolved_body_thickness: None,
            is_previous: false,
        },
        MechanicalLayerRow {
            id: "bottom-foam".into(),
            label: "Bottom foam".into(),
            z: 0.0,
            thickness: 0.5,
            resolved_body_thickness: None,
            is_previous: false,
        },
        MechanicalLayerRow {
            id: "bottom".into(),
            label: "Bottom".into(),
            z: 0.0,
            thickness: 3.0,
            resolved_body_thickness: Some(3.0),
            is_previous: false,
        },
        MechanicalLayerRow {
            id: "retainer".into(),
            label: "Top case".into(),
            z: 0.0,
            thickness: 0.0,
            resolved_body_thickness: None,
            is_previous: false,
        },
    ]);
    let findings = Rc::from([
        MechanicalFindingRow {
            id: "current-error".into(),
            severity: Severity::Error,
            message: "Current clearance error".into(),
        },
        MechanicalFindingRow {
            id: "current-warning".into(),
            severity: Severity::Warning,
            message: "Current warning".into(),
        },
    ]);
    rsx! {
        MechanicalSettings {
            identity,
            request_sequence,
            values: Some(values),
            profiles: Rc::from([]),
            profile_targets: Rc::from([]),
            process_targets: Rc::from(process_targets),
            stabilizer_fits: Rc::from([]),
            layers,
            gasket_supports: Rc::from([]),
            fit_parts: Rc::from([]),
            fit_parts_resolved: false,
            hardware_mounts: Rc::from([]),
            suggested_mounts: Rc::from([]),
            findings,
            selected_layer: selected_layer(),
            mismatch: None,
            editable: true,
            disabled_reason: None,
            feedback: if pending { Rc::from([MechanicalSettingsFeedback {
                identity: test_identity(), request_id: 1, field_id: "process:plate:remove".into(),
                state: MechanicalSettingsFeedbackState::Pending, message: None,
            }]) } else { Rc::from([]) },
            summary_feedback: None,
            on_request: move |_| {},
            on_select_layer: move |id| selected_layer.set(id),
            on_show_finding: move |id| shown_finding.set(id),
        }
        div { id: "case-contextual-selected-finding", "{shown_finding}" }
    }
}

fn pending_action_test_page() -> Element {
    test_page_with_previous(false, true)
}

#[wasm_bindgen_test]
async fn pending_action_disables_only_its_control() {
    let _mounted =
        mount_battery_test_page("case-pending-action-root", pending_action_test_page);
    rendered().await;
    let root = element("#case-pending-action-root");
    by_role_name(&root, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;
    let disabled = |name: &str| {
        js_sys::Reflect::get(
            &by_role_name(&root, "button", name).unwrap(),
            &"disabled".into(),
        )
        .unwrap()
        .as_bool()
        .unwrap_or(false)
    };
    assert!(
        disabled("Remove process override for plate"),
        "only the pending plate removal is disabled"
    );
    for name in [
        "Remove process override for plate foam",
        "Remove process override for bottom foam",
        "Remove process override for bottom",
        "Add process override for U1 · Fixture part",
    ] {
        assert!(!disabled(name), "another action stays available: {name}");
    }
    for name in ["Plate thickness", "Wall thickness"] {
        let field: web_sys::HtmlInputElement = by_role_name(&root, "spinbutton", name)
            .unwrap()
            .dyn_into()
            .unwrap();
        assert!(!field.disabled(), "unrelated fields stay editable: {name}");
    }
    root.remove();
}

fn empty_profile_guidance_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    rsx! {
        ProfileGuidance {
            identity: test_identity(),
            profiles: Rc::from([]),
            targets: Rc::from([]),
            plate_thickness: 2.0,
            plate_to_pcb: 3.0,
            editable: true,
            request_sequence,
            on_request: move |_| {},
        }
    }
}

const CUSTOM_PROFILE_FOOTPRINT: &str = r#"(footprint "Custom tactile switch"
    (fp_rect (start -9.5 9.5) (end 9.5 -9.5)
        (stroke (width 0.1) (type solid)) (fill none) (layer "Dwgs.User")))"#;

fn custom_profile_choice() -> MechanicalProfileChoice {
    MechanicalProfileChoice {
        definition_id: "custom-part".into(),
        name: "Custom tactile switch".into(),
        source: Some("KiCad Custom tactile switch".into()),
        family: None,
        plate_to_pcb: Some(3.5),
        supported_thickness: None,
        switch_family_selectable: false,
        profile: MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: None,
            definition_id: "custom-part".into(),
            source: "KiCad Custom tactile switch".into(),
            cutouts: vec![],
            plate_to_pcb: 3.5,
        },
        kicad_source: Some(CUSTOM_PROFILE_FOOTPRINT.into()),
    }
}

fn custom_profile_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut request_count = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    let mut profile = use_signal(custom_profile_choice);
    let mut saved_profile = use_signal(|| {
        let mut choice = custom_profile_choice();
        choice.definition_id = "saved-part".into();
        choice.name = "Saved profile without extraction state".into();
        choice.source = Some("Authored profile".into());
        choice.kicad_source = None;
        choice.profile.definition_id = "saved-part".into();
        choice.profile.source = "Authored profile".into();
        choice.profile.cutouts = vec![vec![
            Vec2 { x: -9.5, y: 9.5 },
            Vec2 { x: 9.5, y: 9.5 },
            Vec2 { x: 9.5, y: -9.5 },
            Vec2 { x: -9.5, y: -9.5 },
        ]];
        choice
    });
    let extracted_definition_ids = use_signal(Vec::<String>::new);
    let mut owner_key = use_signal(|| "test-owner-a".to_owned());
    let extraction =
        MechanicalProfileExtractionPort(Rc::new(move |definition_id, mappings| {
            let mut extracted_definition_ids = extracted_definition_ids;
            extracted_definition_ids.with_mut(|ids| ids.push(definition_id.clone()));
            Box::pin(async move {
                let request = boardstudio_model::model::ArtifactRequest::ExtractMechanical {
                    id: "profile-extraction-test".to_owned(),
                    source: CUSTOM_PROFILE_FOOTPRINT.to_owned(),
                    mappings,
                    max_deviation_mm: 0.005,
                };
                let reply = boardstudio_core_artifact::artifact::request(
                    &serde_json::to_string(&request).unwrap(),
                );
                match serde_json::from_str::<boardstudio_model::model::ArtifactReply>(&reply)
                    .unwrap()
                {
                    boardstudio_model::model::ArtifactReply::ExtractMechanical {
                        result,
                        ..
                    } => Ok(result),
                    boardstudio_model::model::ArtifactReply::Error { error, .. } => {
                        Err(error.message)
                    }
                    reply => panic!("unexpected mechanical extraction reply: {reply:?}"),
                }
            })
        }));
    let extracted_ids_label = extracted_definition_ids().join(",");
    let owner_key_value = owner_key();
    rsx! {
        div { id: "case-custom-profile-test-root",
            ProfileGuidance {
                identity: test_identity(),
                owner_key: owner_key_value,
                profiles: Rc::from([profile(), saved_profile()]),
                targets: Rc::from([]),
                plate_thickness: 1.6,
                plate_to_pcb: 3.5,
                editable: true,
                request_sequence,
                extract_profile: Some(extraction),
                on_request: move |request: MechanicalSettingsRequest| {
                    request_count += 1;
                    if let MechanicalSettingsPatch::UpdateProfile(updated) = request.patch.clone() {
                        if updated.definition_id == "custom-part" {
                            let mut choice = profile();
                            choice.profile = updated;
                            profile.set(choice);
                        } else {
                            let mut choice = saved_profile();
                            choice.profile = updated;
                            saved_profile.set(choice);
                        }
                    }
                    last_request.set(format!(
                        "{:?}|reviewed={:?}",
                        request.patch, request.reviewed_fingerprint
                    ));
                },
            }
            p { id: "case-custom-profile-last-request", "{last_request}" }
            p { id: "case-custom-profile-request-count", "{request_count}" }
            p { id: "case-custom-profile-extracted-definition-ids", "{extracted_ids_label}" }
            Button { id: "case-profile-switch-owner", onclick: move |_| owner_key.set("test-owner-b".to_owned()), "Switch Case owner" }
        }
    }
}

#[wasm_bindgen_test]
async fn custom_profile_geometry_controls_read_apply_edit_and_remove_assigned_profile() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-custom-profile-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(custom_profile_test_page));
    rendered().await;
    let element = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
    };
    let saved_contour: web_sys::HtmlInputElement = by_role_name(
        &root,
        "spinbutton",
        "Plate cutout geometry contour 1 vertex 1 X",
    )
    .unwrap()
    .dyn_into()
    .unwrap();
    saved_contour.set_value("-9.4");
    let input_event = web_sys::EventInit::new();
    input_event.set_bubbles(true);
    saved_contour
        .dispatch_event(
            &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
        )
        .unwrap();
    rendered().await;
    assert!(
        element("#case-custom-profile-last-request")
            .text_content()
            .unwrap()
            .contains("-9.4")
    );
    assert!(
        by_role_name(&root, "button", "Remove profile Custom tactile switch")
            .unwrap()
            .is_connected()
    );
    by_role_name(&root, "button", "Read KiCad layers")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        element("#case-custom-profile-extracted-definition-ids")
            .text_content()
            .unwrap(),
        "custom-part"
    );
    assert!(
        by_role_name(&root, "button", "Purpose for Dwgs.User")
            .unwrap()
            .is_connected()
    );
    let geometry = boardstudio_core_artifact::artifact::request(
        &serde_json::to_string(
            &boardstudio_model::model::ArtifactRequest::ExtractMechanical {
                id: "ids".into(),
                source: CUSTOM_PROFILE_FOOTPRINT.into(),
                mappings: vec![],
                max_deviation_mm: 0.005,
            },
        )
        .unwrap(),
    );
    let boardstudio_model::model::ArtifactReply::ExtractMechanical { result, .. } =
        serde_json::from_str(&geometry).unwrap()
    else {
        panic!("geometry")
    };
    let checkbox = by_role_name(
        &root,
        "checkbox",
        &format!("Select geometry {}", result.geometry.primitives[0].id),
    )
    .unwrap();
    checkbox.click();
    rendered().await;
    let request_count = || {
        element("#case-custom-profile-request-count")
            .text_content()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };
    let before_apply = request_count();
    by_role_name(&root, "button", "Apply selected geometry")
        .unwrap()
        .click();
    rendered().await;
    let extraction_error = root
        .query_selector("[aria-label='Custom KiCad geometry'] [role='alert']")
        .unwrap()
        .and_then(|alert| alert.text_content());
    assert!(
        extraction_error.is_none(),
        "real Core rejected the UI mapping: {extraction_error:?}"
    );
    assert_eq!(
        request_count(),
        before_apply + 1,
        "Apply must submit one newly extracted profile"
    );
    assert_eq!(
        element("#case-custom-profile-extracted-definition-ids")
            .text_content()
            .unwrap(),
        "custom-part,custom-part"
    );
    assert!(
        element("#case-custom-profile-last-request")
            .text_content()
            .unwrap()
            .contains("UpdateProfile")
    );
    assert!(
        element("#case-custom-profile-last-request")
            .text_content()
            .unwrap()
            .contains("-9.5")
    );
    let custom = by_role_name(&root, "group", "Fit profile Custom tactile switch").unwrap();
    let cutout_x: web_sys::HtmlInputElement = by_role_name(
        &custom,
        "spinbutton",
        "Plate cutout geometry contour 1 vertex 1 X",
    )
    .unwrap()
    .dyn_into()
    .unwrap();
    assert_eq!(
        cutout_x.value(),
        "-9.5",
        "Apply must replace the accepted contour with the real extraction"
    );
    cutout_x.set_value("-9.4");
    cutout_x
        .dispatch_event(
            &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
        )
        .unwrap();
    rendered().await;
    assert!(
        element("#case-custom-profile-last-request")
            .text_content()
            .unwrap()
            .contains("-9.4")
    );
    by_role_name(&root, "button", "Remove profile Custom tactile switch")
        .unwrap()
        .click();
    rendered().await;
    assert!(
        element("#case-custom-profile-last-request")
            .text_content()
            .unwrap()
            .contains("RemoveProfile")
    );
    by_role_name(&root, "button", "Switch Case owner")
        .unwrap()
        .click();
    rendered().await;
    assert!(by_role_name(&root, "button", "Purpose for Dwgs.User").is_none());
    root.remove();
}

#[wasm_bindgen_test]
async fn profile_volume_height_escape_discards_and_enter_blur_commits_once() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-profile-volume-draft-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(custom_profile_test_page));
    rendered().await;
    let custom_profile =
        by_role_name(&root, "group", "Fit profile Custom tactile switch").unwrap();
    let query = |selector: &str| root.query_selector(selector).unwrap().unwrap();
    by_role_name(&custom_profile, "button", "Add volume to Clearance volumes")
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    rendered().await;
    let input = by_role_name(&root, "spinbutton", "Clearance volumes volume 1 Height")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    let dispatch_input = |value: &str| {
        input.set_value(value);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
    };
    let dispatch_key = |key: &str| {
        let init = web_sys::KeyboardEventInit::new();
        init.set_key(key);
        init.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &init)
                    .unwrap(),
            )
            .unwrap();
    };
    let dispatch_change = || {
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("change", &init).unwrap())
            .unwrap();
    };
    let count = || {
        query("#case-custom-profile-request-count")
            .text_content()
            .unwrap()
            .parse::<u64>()
            .unwrap()
    };

    // Establish the accepted 6 mm height in the retained reference journey.
    input.focus().unwrap();
    dispatch_input("6");
    input.blur().unwrap();
    dispatch_change();
    rendered().await;
    assert_eq!(input.value(), "6");
    let before = count();
    assert_eq!(before, 2, "one Add volume and one accepted height edit");

    input.focus().unwrap();
    dispatch_input("7.2");
    dispatch_key("Escape");
    input.blur().unwrap();
    dispatch_change();
    rendered().await;
    assert_eq!(
        count(),
        before,
        "Escape then blur must not submit the abandoned profile height"
    );
    assert_eq!(
        input.value(),
        "6",
        "Escape must restore the accepted height"
    );

    input.focus().unwrap();
    dispatch_input("6.5");
    dispatch_key("Enter");
    input.blur().unwrap();
    dispatch_change();
    rendered().await;
    assert_eq!(
        count(),
        before + 1,
        "Enter followed by blur/change must submit exactly once"
    );
    assert_eq!(input.value(), "6.5");

    input.focus().unwrap();
    dispatch_input("0.05");
    input.blur().unwrap();
    dispatch_change();
    rendered().await;
    assert_eq!(
        count(),
        before + 1,
        "invalid height below 0.1 mm must not submit"
    );
    input.focus().unwrap();
    dispatch_key("Escape");
    rendered().await;
    assert_eq!(
        input.value(),
        "6.5",
        "invalid input must leave the accepted height unchanged"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn assigned_profile_adds_component_clearance_polygon_without_extraction() {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-profile-clearance-test-host");
    document.body().unwrap().append_child(&root).unwrap();
    let _mounted = MountedFixture::new(root.clone(), test_page_dom(custom_profile_test_page));
    rendered().await;
    let custom_profile =
        by_role_name(&root, "group", "Fit profile Custom tactile switch").unwrap();
    let element = |selector: &str| {
        root.query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
            .unwrap()
    };

    by_role_name(
        &custom_profile,
        "button",
        "Add polygon to Component clearance zones",
    )
    .unwrap()
    .click();
    rendered().await;

    let request = element("#case-custom-profile-last-request")
        .text_content()
        .unwrap();
    assert!(request.contains("UpdateProfile"));
    assert!(request.contains("clearances: Some"));
    assert!(request.contains("x: -2.5"));
    by_role_name(&custom_profile, "button", "Add volume to Access openings")
        .unwrap()
        .click();
    rendered().await;
    let request = element("#case-custom-profile-last-request")
        .text_content()
        .unwrap();
    assert!(request.contains("openings: Some"));
    assert!(request.contains("height: 10.0"));
    by_role_name(&custom_profile, "button", "Add volume to Clearance volumes")
        .unwrap()
        .click();
    rendered().await;
    let request = element("#case-custom-profile-last-request")
        .text_content()
        .unwrap();
    assert!(request.contains("clearance_volumes: Some"));
    assert!(request.contains("height: 10.0"));
    assert_eq!(
        element("#case-custom-profile-extracted-definition-ids")
            .text_content()
            .unwrap(),
        ""
    );
    root.remove();
}

fn text_commit_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut count = use_signal(|| 0_u32);
    let mut accepted = use_signal(|| "M2".to_owned());
    rsx! {
        TextDraftField {
            identity: test_identity(), request_sequence, feedback: Rc::from([]),
            label: "Thread", value: accepted(), intent: MechanicalTextIntent::ClosureThread,
            editable: true, on_request: move |_| count += 1,
        }
        output { role: "status", aria_label: "Submitted drafts", "{count}" }
        Button { onclick: move |_| accepted.set("M4".into()), "Land previous edit" }
    }
}

fn text_multiline_test_page() -> Element {
    let request_sequence = use_signal(|| 0_u64);
    let mut count = use_signal(|| 0_u32);
    rsx! {
        TextDraftField {
            identity: test_identity(), request_sequence, feedback: Rc::from([]),
            label: "Screw lengths", value: "3, 4".to_owned(),
            intent: MechanicalTextIntent::ClosureScrewLengths,
            editable: true, multiline: true, on_request: move |_| count += 1,
        }
        output { role: "status", aria_label: "Submitted drafts", "{count}" }
    }
}

#[wasm_bindgen_test]
async fn multiline_text_draft_is_named_and_describes_its_error() {
    let _mounted =
        mount_battery_test_page("case-text-multiline-root", text_multiline_test_page);
    rendered().await;
    let root = element("#case-text-multiline-root");
    let area = by_role_name(&root, "textbox", "Screw lengths")
        .unwrap()
        .dyn_into::<web_sys::HtmlTextAreaElement>()
        .unwrap();
    assert_eq!(
        area.get_attribute("data-slot").as_deref(),
        Some("input-group-control"),
        "the adopted multiline field keeps the shared input root"
    );
    assert_eq!(area.get_attribute("rows").as_deref(), Some("2"));
    let area_value = |value: &str| {
        area.focus().unwrap();
        area.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        area.dispatch_event(
            &web_sys::Event::new_with_event_init_dict("input", &event).unwrap(),
        )
        .unwrap();
    };
    area_value("abc");
    area.blur().unwrap();
    wait_until("invalid lengths are rejected locally", || {
        area.get_attribute("aria-invalid").as_deref() == Some("true")
    })
    .await;
    assert_eq!(area.value(), "abc", "the field owns the rejected draft");
    let described = area
        .get_attribute("aria-describedby")
        .expect("validation must describe the multiline input");
    let error = root
        .query_selector(&format!("[id='{described}'] [data-slot='field-error']"))
        .unwrap()
        .expect("description resolves to this production field error");
    assert!(!error.text_content().unwrap().is_empty());
    assert_eq!(
        by_role_name(&root, "status", "Submitted drafts")
            .unwrap()
            .text_content()
            .unwrap(),
        "0",
        "an invalid draft must not submit"
    );
    area_value("3, 5");
    area.blur().unwrap();
    wait_until("valid lengths submit", || {
        by_role_name(&root, "status", "Submitted drafts")
            .unwrap()
            .text_content()
            .as_deref()
            == Some("1")
    })
    .await;
    root.remove();
}

#[wasm_bindgen_test]
async fn text_escape_discards_and_new_drafts_queue_before_acknowledgment() {
    let _mounted = mount_battery_test_page("case-text-commit-root", text_commit_test_page);
    rendered().await;
    let root = element("#case-text-commit-root");
    let input = by_role_name(&root, "textbox", "Thread")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    assert_eq!(
        input.get_attribute("data-slot").as_deref(),
        Some("input-group-control"),
        "the adopted text field keeps the shared input root"
    );
    assert!(!input.id().is_empty(), "the field label targets this input");
    let input_value = |value: &str| {
        input.focus().unwrap();
        input.set_value(value);
        let event = web_sys::EventInit::new();
        event.set_bubbles(true);
        input
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &event).unwrap())
            .unwrap();
    };
    input_value("M3");
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape)
                .unwrap(),
        )
        .unwrap();
    input.blur().unwrap();
    rendered().await;
    assert_eq!(
        by_role_name(&root, "status", "Submitted drafts")
            .unwrap()
            .text_content()
            .unwrap(),
        "0",
        "Escape must not submit on blur"
    );
    for value in ["M3", "M4"] {
        input_value(value);
        input.blur().unwrap();
        rendered().await;
    }
    assert_eq!(
        by_role_name(&root, "status", "Submitted drafts")
            .unwrap()
            .text_content()
            .unwrap(),
        "2"
    );
    assert_eq!(input.value(), "M4");
    input_value("M5");
    by_role_name(&root, "button", "Land previous edit")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        input.value(),
        "M5",
        "landing an earlier edit must preserve active typing"
    );
    root.remove();
}

#[component]
fn BatteryTestPage() -> Element {
    let wired_request_sequence = use_signal(|| 0_u64);
    let wireless_request_sequence = use_signal(|| 0_u64);
    let mut last_request = use_signal(String::new);
    rsx! {
        div { id: "case-battery-wired-test-root",
            MechanicalSettings {
                identity: test_identity(),
                request_sequence: wired_request_sequence,
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
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    last_request.set(format!("{:?}", request.patch))
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
        }
        div { id: "case-battery-wireless-test-root",
            MechanicalSettings {
                identity: test_identity(),
                request_sequence: wireless_request_sequence,
                values: Some(test_values(
                    HardwareTransport::Wireless,
                    Some(MechanicalBattery {
                        cable_width: None,
                        size: Vec3 { x: 30.0, y: 20.0, z: 6.0 },
                        at: Vec2 { x: 12.0, y: -4.0 },
                        cable_exit: Vec2 { x: 29.0, y: -4.0 },
                    }),
                )),
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
                selected_layer: String::new(),
                mismatch: None,
                editable: true,
                disabled_reason: None,
                feedback: Rc::from([]),
                summary_feedback: None,
                on_request: move |request: MechanicalSettingsRequest| {
                    last_request.set(format!("{:?}", request.patch))
                },
                on_select_layer: move |_| {},
                on_show_finding: move |_| {},
            }
        }
        div { id: "case-battery-last-request", "{last_request}" }
    }
}

#[wasm_bindgen_test]
async fn selected_plate_uses_contextual_inspector_and_returns_to_assembly_settings() {
    let document = web_sys::window().unwrap().document().unwrap();
    let stylesheet = document.create_element("style").unwrap();
    stylesheet.set_text_content(Some(include_str!("../../../../assets/workbench-base.css")));
    document.head().unwrap().append_child(&stylesheet).unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("case-contextual-mechanical-settings-test-root");
    document.body().unwrap().append_child(&root).unwrap();
    let mut mounted = MountedFixture::new(root.clone(), test_page_dom(test_page));
    mounted.own_node(stylesheet.clone());
    rendered().await;

    let scope = element("#case-contextual-mechanical-settings-test-root");
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Plate"));
    assert!(text.contains("Assembly settings"));
    assert!(text.contains("Plate thickness"));
    assert!(text.contains("Resolved thickness 1.50 mm."));
    assert!(text.contains("Current clearance error"));
    assert!(!text.contains("Current warning"));
    assert!(!text.contains("Case construction"));
    assert!(!text.contains("Bottom thickness"));
    by_role_name(&scope, "button", "Show finding current-error")
        .unwrap()
        .click();
    rendered().await;
    assert_eq!(
        element("#case-contextual-selected-finding")
            .text_content()
            .as_deref(),
        Some("current-error")
    );

    by_role_name(&scope, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;
    let manufacturing_text = scope.text_content().unwrap_or_default();
    assert!(
        manufacturing_text.contains("Per-part process overrides"),
        "the assembly settings should expose per-part manufacturing controls"
    );
    assert!(manufacturing_text.contains("Per-part process overrides 4"));
    assert!(manufacturing_text.contains("Stabilizer fit 0 wide keys"));
    assert_eq!(
        all_by_role_name(&scope, "button", "Material").len(),
        4,
        "each accepted standard process override should expose its material"
    );
    assert_eq!(
        scope
            .text_content()
            .unwrap_or_default()
            .matches("Add process override")
            .count(),
        1
    );
    assert!(
        scope
            .text_content()
            .unwrap_or_default()
            .contains("Case construction")
    );

    by_role_name(
        &element("#case-contextual-mechanical-settings-test-root"),
        "button",
        "PCB, 1.60 mm, Z 0.00",
    )
    .unwrap()
    .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("PCB thickness"));
    assert!(!text.contains("Plate thickness"));
    assert!(!text.contains("Resolved thickness"));
    by_role_name(&scope, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;

    by_role_name(
        &element("#case-contextual-mechanical-settings-test-root"),
        "button",
        "Plate foam, 0.50 mm, Z 0.00",
    )
    .unwrap()
    .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Plate foam"));
    assert!(!text.contains("PCB thickness"));
    by_role_name(&scope, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;

    by_role_name(
        &element("#case-contextual-mechanical-settings-test-root"),
        "button",
        "Bottom foam, 0.50 mm, Z 0.00",
    )
    .unwrap()
    .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Bottom foam"));
    assert!(!text.contains("PCB thickness"));
    by_role_name(&scope, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;

    by_role_name(
        &element("#case-contextual-mechanical-settings-test-root"),
        "button",
        "Bottom, 3.00 mm, Z 0.00",
    )
    .unwrap()
    .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Bottom case"));
    assert!(text.contains("Bottom thickness"));
    assert!(text.contains("Wall thickness"));
    assert!(text.contains("Clearance"));
    assert!(text.contains("Resolved thickness 3.00 mm."));
    assert!(text.contains("Edit shared closure hardware"));
    assert!(!text.contains("Plate thickness"));
    assert!(!text.contains("Current warning"));

    by_role_name(&scope, "button", "Edit shared closure hardware")
        .unwrap()
        .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Case construction"));

    by_role_name(
        &element("#case-contextual-mechanical-settings-test-root"),
        "button",
        "Top case, 0.00 mm, Z 0.00",
    )
    .unwrap()
    .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Top case"));
    assert!(text.contains("Wall thickness"));
    assert!(text.contains("Clearance"));
    assert!(!text.contains("Resolved thickness"));
    assert!(!text.contains("Plate thickness"));
}

#[wasm_bindgen_test]
async fn unassigned_profile_guidance_shows_saved_plate_to_pcb_gap() {
    let _mounted = mount_battery_test_page(
        "case-empty-profile-guidance-test-root",
        empty_profile_guidance_test_page,
    );
    rendered().await;
    let text = element("#case-empty-profile-guidance-test-root")
        .text_content()
        .unwrap_or_default();
    assert!(
        text.contains("Plate underside to PCB top: 3.00 mm"),
        "show the accepted plate gap even when no explicit profile row is assigned"
    );
}

#[wasm_bindgen_test]
async fn previous_generated_layer_context_stays_visible_and_is_labeled() {
    let _mounted =
        mount_battery_test_page("case-previous-settings-test-root", previous_test_page);
    rendered().await;
    let scope = element("#case-previous-settings-test-root");
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("previous generated geometry"));
    assert!(text.contains("Resolved thickness 1.50 mm."));
    assert!(text.contains("Plate thickness"));

    by_role_name(&scope, "button", "Assembly settings")
        .unwrap()
        .click();
    rendered().await;
    let text = scope.text_content().unwrap_or_default();
    assert!(text.contains("Previous resolved stack"));
    assert!(text.contains("Settings below reflect the current accepted configuration."));
    assert!(text.contains("Previous revision"));
}

#[wasm_bindgen_test]
async fn battery_controls_match_wired_and_wireless_modes_and_dispatch_toggle() {
    let _mounted = mount_battery_test_page("case-battery-settings-test-root", BatteryTestPage);
    rendered().await;

    let document = web_sys::window().unwrap().document().unwrap();
    let wired = document
        .query_selector("#case-battery-wired-test-root")
        .unwrap()
        .unwrap();
    assert!(
        wired
            .text_content()
            .unwrap_or_default()
            .contains("Include a battery envelope"),
        "wired mechanical settings should expose the optional battery envelope toggle"
    );
    let toggle = by_role_name(&wired, "checkbox", "Include a battery envelope").unwrap();
    assert_eq!(
        toggle.get_attribute("aria-checked").as_deref(),
        Some("false")
    );
    assert!(
        !wired.text_content().unwrap_or_default().contains("Width"),
        "wired settings without a battery show no battery fields"
    );
    assert!(!toggle.id().is_empty(), "the toggle keeps its label target");
    let label = document
        .query_selector(&format!("label[for='{}']", toggle.id()))
        .unwrap()
        .expect("the toggle has an associated visible label");
    assert!(
        label
            .text_content()
            .unwrap_or_default()
            .contains("Include a battery envelope"),
        "the associated label names the toggle"
    );

    toggle.click();
    rendered().await;
    assert_eq!(
        element("#case-battery-last-request")
            .text_content()
            .as_deref(),
        Some("SetBatteryEnabled(true)")
    );

    let wireless = document
        .query_selector("#case-battery-wireless-test-root")
        .unwrap()
        .unwrap();
    let text = wireless.text_content().unwrap_or_default();
    assert!(text.contains("Included for wireless."));
    assert!(!text.contains("Include a battery envelope"));
    for (label, minimum) in [
        ("Width", "0.1"),
        ("Depth", "0.1"),
        ("Height", "0.1"),
        ("Cable width", "0.1"),
        ("Position X", "-1000000"),
        ("Position Y", "-1000000"),
        ("Cable exit X", "-1000000"),
        ("Cable exit Y", "-1000000"),
    ] {
        let input: web_sys::HtmlInputElement = by_role_name(&wireless, "spinbutton", label)
            .unwrap_or_else(|| panic!("wireless battery field {label} should be visible"))
            .dyn_into()
            .unwrap();
        assert_eq!(input.step(), "0.1");
        assert_eq!(input.min(), minimum);
    }
    let cable_width: web_sys::HtmlInputElement =
        by_role_name(&wireless, "spinbutton", "Cable width")
            .unwrap()
            .dyn_into()
            .unwrap();
    assert_eq!(cable_width.value().parse::<f64>().unwrap(), 2.0);
}

