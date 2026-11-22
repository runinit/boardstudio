use super::*;

#[wasm_bindgen_test]
async fn mounted_component_position_untouched_blur_and_escape_cancel_submit_no_edit() {
    let (probe, root) = mounted_probe_configured(
        "layout-component-position-untouched-blur-test-root",
        |document| {
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap();
            part.reference = "J1".into();
            part.pose.at = Vec2 {
                x: 66.675,
                y: -47.625,
            };
        },
    )
    .await;
    settle_mounted_component_inspector(&probe).await;
    let initial_revision = accepted_revision(&probe);
    let x = position_input(probe.root_id, "X mm");
    let y = position_input(probe.root_id, "Y mm");
    let bubbling = web_sys::EventInit::new();
    bubbling.set_bubbles(true);
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);

    // Merely moving through both fields must not turn their two-decimal
    // presentation into a coordinate edit on blur.
    x.focus().unwrap();
    y.focus().unwrap();
    x.focus().unwrap();

    // Escape cancels a real draft. The ensuing blur must not submit the
    // restored, formatted display value as a replacement for raw geometry.
    x.set_value("71.125");
    x.dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &bubbling).unwrap())
        .unwrap();
    x.dispatch_event(
        &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
    )
    .unwrap();
    let _ = x.blur();
    accept_pending(&probe).await;

    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 {
            x: 66.675,
            y: -47.625
        },
        "untouched focus/blur and Escape-cancel must not move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "untouched focus/blur and Escape-cancel must accept no edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_group_position_blur_moves_all_selected_parts_from_first_anchor() {
    let (probe, root) = mounted_group_probe("layout-component-group-position-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    let document = web_sys::window().unwrap().document().unwrap();
    assert!(
        document
            .query_selector(&format!("#{}", probe.root_id))
            .unwrap()
            .unwrap()
            .text_content()
            .unwrap_or_default()
            .contains("2 parts selected. Position edits apply to the selection.")
    );
    let x = position_input(probe.root_id, "X mm");
    assert!(!x.disabled());
    try_commit_position(&x, "5.5");
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 5.5, y: 3.0 },
        "the group position edit moves the anchor part"
    );
    assert_eq!(
        accepted_position(&probe, "source-part"),
        Vec2 { x: 11.5, y: 3.0 },
        "the group position edit translates every selected part by the anchor delta"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_locked_component_presents_xy_unavailable_and_suppresses_commit() {
    let (probe, root) =
        mounted_probe_configured("layout-component-position-locked-test-root", |document| {
            document
                .parts
                .iter_mut()
                .find(|part| part.id == "selected-part")
                .unwrap()
                .locked = Some(true);
        })
        .await;
    settle_mounted_component_inspector(&probe).await;
    assert!(probe.projection.borrow().as_ref().unwrap().locked);
    let initial_revision = accepted_revision(&probe);
    for (label, value) in [("X mm", "50"), ("Y mm", "60")] {
        let input = position_input(probe.root_id, label);
        assert!(
            input.disabled(),
            "{label} must be unavailable for a locked part"
        );
        try_commit_position(&input, value);
    }
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 4.0, y: 3.0 },
        "locked position commits never move the part"
    );
    assert_eq!(
        accepted_revision(&probe),
        initial_revision,
        "locked position commits accept no edits"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_offset_form_follows_the_selection_a_conversion_its_undo_and_the_reset() {
    const KEY: &str = "matrix/component-matrix/r0c0";
    let (probe, root) = mounted_probe_configured("layout-offset-form-test-root", |document| {
        // The key sits between the two standalone parts, so the first part that is not
        // the selected one is a key for one selection and not for the other.
        document.boards[0].part_ids =
            vec!["selected-part".into(), KEY.into(), "source-part".into()];
        let mut key = document.parts[0].clone();
        key.id = KEY.into();
        key.reference = "SW1".into();
        key.outline = None;
        key.pose.rotation = 30.0;
        document.parts.push(key);
        document.matrices.push(Matrix {
            id: "component-matrix".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "component-definition".into(),
            part_ids: vec![KEY.into()],
            board_id: Some("board".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![boardstudio_core::model::ColumnLayout::default()],
            anchor: None,
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: Some(30.0),
                assemblies: vec![],
                assemblies_local: None,
            }],
        });
        document.constraints.push(Constraint::PartOffset {
            id: "held".into(),
            source_part_id: KEY.into(),
            target_part_id: "selected-part".into(),
            offset: Vec2 { x: 1.0, y: -4.0 },
            rotation: 5.0,
        });
    })
    .await;
    settle_mounted_component_inspector(&probe).await;
    let order: Vec<(String, bool)> = probe
        .projection
        .borrow()
        .as_ref()
        .expect("the selected part projects")
        .board_parts
        .iter()
        .map(|part| (part.id.clone(), part.matrix_key))
        .collect();
    assert_eq!(
        order[..3],
        [
            ("selected-part".to_owned(), false),
            (KEY.to_owned(), true),
            ("source-part".to_owned(), false),
        ],
        "the fixture's board order and key flags"
    );
    let root_id = probe.root_id;

    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .expect("fixture snapshot")
            .document
            .parts
            .iter()
            .find(|part| part.id == KEY)
            .expect("the source key is accepted")
            .pose
            .rotation,
        30.0,
        "the accepted source key must retain its authored 30 degree heading before conversion"
    );

    // First render: the saved part-offset constraint.
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "Offset along part axes".into(), "SW1".into()),
        "first render"
    );

    // A selection change to a part without a constraint: its first other part is the
    // selected-part, which is not a key, so the default is board axes and zeros.
    click_component_inspector(root_id, "button", "Select source");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "Offset on board axes".into(), "U1".into()),
        "after selecting an unconstrained part"
    );

    // Back to the constrained part: its saved kind, source and numbers again.
    click_component_inspector(root_id, "button", "Select original");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "Offset along part axes".into(), "SW1".into()),
        "after selecting the constrained part again"
    );

    // A conversion re-expresses the numbers on the board axes at the key's 30 degree turn.
    choose_offset_axes_in_form(root_id, "Offset on board axes").await;
    accept_pending(&probe).await;
    let (x, axes, source) = offset_form(root_id);
    assert!(
        (x.parse::<f64>().unwrap() - 2.866_025_403_784_439).abs() < 1e-6,
        "converted X draft {x}"
    );
    assert_eq!(
        (axes.as_str(), source.as_str()),
        ("Offset on board axes", "SW1")
    );

    // Undoing the conversion restores the saved part-offset constraint and its numbers.
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("1".into(), "Offset along part axes".into(), "SW1".into()),
        "after undoing the conversion"
    );

    // The reset: an unconstrained part's unsaved axes choice goes back to the default when
    // Properties is shown again.
    click_component_inspector(root_id, "button", "Select source");
    settle_mounted_component_inspector(&probe).await;
    choose_offset_axes_in_form(root_id, "Offset along part axes").await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id).1,
        "Offset along part axes",
        "the unsaved choice shows"
    );
    click_component_inspector(root_id, "tab", "Relations");
    settle_mounted_component_inspector(&probe).await;
    click_component_inspector(root_id, "tab", "Properties");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        offset_form(root_id),
        ("0".into(), "Offset on board axes".into(), "U1".into()),
        "after the reset"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn companion_placement_shows_mixed_modes_and_tweaks() {
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;

    let mut projection = companion_placement_component_projection(None);
    projection.mode = None;
    projection.mixed_mode = true;
    projection.tweak = None;
    projection.mixed_tweak = true;
    let actions = Rc::new(RefCell::new(None::<inspector::CompanionPlacementAction>));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(companion_placement_component_host);
    dom.provide_root_context(projection);
    dom.provide_root_context(actions);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    wait_until("the mixed companion placement section mounts", || {
        by_role_name(&root, "button", "Companion placement").is_some()
    })
    .await;
    let trigger = by_role_name(&root, "button", "Companion placement").unwrap();
    assert!(trigger.text_content().unwrap().contains("Mixed"));
    trigger.click();
    let mode = by_role_name(&root, "button", "Mode").unwrap();
    assert!(mode.text_content().unwrap().contains("Mixed values"));
    choose_inspector_option(&root, "Mode", "Mirror with tweak").await;
    wait_until("mixed tweak fields mount", || {
        ["X tweak (mm)", "Y tweak (mm)", "Rotation tweak (degrees)"]
            .iter()
            .all(|name| by_role_name(&root, "spinbutton", name).is_some())
    })
    .await;
    for name in ["X tweak (mm)", "Y tweak (mm)", "Rotation tweak (degrees)"] {
        assert_eq!(
            by_role_name(&root, "spinbutton", name)
                .unwrap()
                .dyn_into::<web_sys::HtmlInputElement>()
                .unwrap()
                .placeholder(),
            "Mixed values"
        );
    }
    root.remove();
}

#[wasm_bindgen_test]
async fn stabiliser_companion_placement_shows_a_disabled_mode_and_core_reason() {
    let projection = companion_placement_component_projection(Some(
        "Stabilisers keep their mirrored-X position and unchanged heading in every mode.",
    ));
    let actions = Rc::new(RefCell::new(None::<inspector::CompanionPlacementAction>));
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(companion_placement_component_host);
    dom.provide_root_context(projection);
    dom.provide_root_context(actions);
    dioxus_web::launch::launch_virtual_dom(
        dom,
        dioxus_web::Config::new().rootnode(root.clone().into()),
    );
    use boardstudio_web_ui_shared::test_wait::wait_until;
    wait_until("the stabiliser placement reason mounts", || {
        by_role_name(&root, "button", "Mode").is_some()
    })
    .await;
    let mode = by_role_name(&root, "button", "Mode").expect("disabled mode control");
    assert!(mode.has_attribute("disabled"));
    assert!(
        root.text_content()
            .unwrap()
            .contains("Stabilisers keep their mirrored-X position")
    );
    assert!(by_role_name(&root, "button", "Apply placement").is_none());
    root.remove();
}
