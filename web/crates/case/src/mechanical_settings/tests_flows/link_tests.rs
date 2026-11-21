//! Pair-link flow tests for mechanical settings.
use super::*;

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
