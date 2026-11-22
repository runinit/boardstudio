use super::*;

#[wasm_bindgen_test]
async fn mounted_matrix_key_stays_key_until_explicit_finding_opens_component_inspector() {
    let (probe, _root) = mounted_runtime_probe(
        "layout-matrix-part-inspector-test-root",
        |document| {
            // Core generates matrix members with its own ids; drop the standalone
            // selected part so the generated member takes over as the selection.
            document.parts.retain(|part| part.id == "source-part");
            document
                .boards
                .iter_mut()
                .for_each(|board| board.part_ids.retain(|id| id != "selected-part"));
            document.mirrored_pairs.clear();
            document.layouts.clear();
            document.layouts.push(Layout {
                id: "component-layout".into(),
                name: "Component half".into(),
                board_id: "board".into(),
                matrix_ids: vec!["component-matrix".into()],
                part_ids: vec![],
                mirror_link: None,
            });
            document.layouts.push(Layout {
                id: "component-mirror-layout".into(),
                name: "Component mirror".into(),
                board_id: "board".into(),
                matrix_ids: vec![],
                part_ids: vec![],
                mirror_link: None,
            });
            document.mirrored_pairs = serde_json::from_value(serde_json::json!([{
                "id": "pair",
                "source": {"kind": "sameBoardLayout", "id": "component-layout"},
                "destination": {"kind": "sameBoardLayout", "id": "component-mirror-layout"},
                "mapping": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0},
                "overrides": []
            }]))
            .unwrap();
            document.matrices.clear();
            document.parts.push(
                serde_json::from_value(serde_json::json!({
                    "id": "matrix/component-matrix/r0c0",
                    "definitionId": "component-definition",
                    "reference": "SW1",
                    "pose": {"at": {"x": 0.0, "y": 0.0}, "rotation": 0.0},
                    "side": "front",
                    "locked": false
                }))
                .unwrap(),
            );
            document.boards[0]
                .part_ids
                .push("matrix/component-matrix/r0c0".into());
            document.matrices.push(Matrix {
                id: "component-matrix".into(),
                name: Some("keys".into()),
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "component-definition".into(),
                part_ids: vec!["matrix/component-matrix/r0c0".into()],
                board_id: Some("board".into()),
                mirror: None,
                rotation: None,
                edge_gap: None,
                diode_direction: None,
                row_layout: vec![],
                column_layout: vec![],
                anchor: None,
                cells: vec![MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: 0,
                    column: 0,
                    enabled: true,
                    definition_id: Some("component-definition".into()),
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                }],
            });
        },
        &["matrix/mirror_70616972_component-matrix/r0c0"],
    )
    .await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Key,
        "ordinary matrix primary selection remains in Key mode"
    );
    assert!(probe.projection.borrow().is_none());
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} [aria-label='Component inspector']",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "ordinary matrix primary selection remains in Key mode"
    );

    click_component_inspector(probe.root_id, "button", "Select component from finding");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .map(|projection| projection.reference.as_str()),
        Some("SW1"),
        "explicit finding navigation must project the selected matrix Part (Core numbers matrix members as switches)"
    );
    assert_eq!(
        *probe
            .selection_kind
            .borrow()
            .expect("selection mode")
            .peek(),
        objects::LayoutSelectionKind::Part,
        "explicit finding navigation publishes the matching Part toolbar mode"
    );
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} [aria-label='Component inspector']",
                probe.root_id
            ))
            .unwrap()
            .is_some(),
        "the production component Inspector must mount for an explicit live Part context"
    );
    _root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_inspector_production_handler_rejects_selection_aba_and_unmount() {
    let (probe, root) =
        mounted_probe_configured("layout-component-inspector-aba-test-root", |document| {
            document.layouts[0].part_ids.clear();
        })
        .await;
    settle_mounted_component_inspector(&probe).await;
    let old_projection = probe.projection.borrow().clone().unwrap();
    let old_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "button", "Select source");
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(*probe.selected_part.borrow(), Some("source-part".into()));
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().reference,
        "U2",
        "component selection changes must rerender the mounted Inspector"
    );
    click_component_inspector(probe.root_id, "button", "Select original");
    settle_mounted_component_inspector(&probe).await;
    assert_ne!(
        old_projection.owner.context_generation,
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        "the production owner lifetime must advance through the selection ABA"
    );
    let before = accepted_position(&probe, "selected-part");
    old_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: old_projection.owner.clone(),
        axis: ComponentPositionAxis::X,
        value: 42.0,
        submitted_draft: "42".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        before,
        "a stale owner action changes nothing"
    );

    let current = probe.projection.borrow().clone().unwrap();
    probe
        .action
        .borrow()
        .as_ref()
        .unwrap()
        .call(LayoutComponentInspectorAction::SetPosition {
            owner: current.owner,
            axis: ComponentPositionAxis::X,
            value: 42.0,
            submitted_draft: "42".into(),
        });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part").x,
        42.0,
        "the current owner's position edit lands"
    );

    let stale = probe.projection.borrow().clone().unwrap();
    let stale_action = *probe.action.borrow().as_ref().unwrap();
    click_component_inspector(probe.root_id, "button", "Clear selection");
    settle_mounted_component_inspector(&probe).await;
    let cleared = accepted_position(&probe, "selected-part");
    stale_action.call(LayoutComponentInspectorAction::SetPosition {
        owner: stale.owner,
        axis: ComponentPositionAxis::X,
        value: 43.0,
        submitted_draft: "43".into(),
    });
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        cleared,
        "an action after selection clear changes nothing"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_margin_enter_commits_and_escape_restores_the_accepted_value() {
    let (probe, root) = mounted_probe("layout-component-inspector-margin-test-root").await;
    settle_mounted_component_inspector(&probe).await;
    click_component_inspector(probe.root_id, "button", "Board outline");
    settle_mounted_component_inspector(&probe).await;
    assert!(
        component_inspector_control(probe.root_id, "button", "Board outline")
            .get_attribute("aria-expanded")
            .as_deref()
            == Some("true"),
        "the margin field must be in the expanded outline section"
    );
    let document = web_sys::window().unwrap().document().unwrap();
    let input = position_input(probe.root_id, "Part edge margin");
    input.focus().unwrap();
    assert_eq!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "the visible margin field should receive focus before keyboard input"
    );
    input.set_value("4.25");
    let input_init = web_sys::EventInit::new();
    input_init.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let enter = web_sys::KeyboardEventInit::new();
    enter.set_key("Enter");
    enter.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &enter).unwrap(),
        )
        .unwrap();
    assert_ne!(
        document
            .active_element()
            .and_then(|active| active.get_attribute("aria-label"))
            .as_deref(),
        Some("Part edge margin"),
        "Enter must blur the margin field so the normal commit runs"
    );
    accept_pending(&probe).await;
    let accepted = probe.runtime.model().accepted.unwrap();
    assert_eq!(
        accepted
            .document
            .parts
            .iter()
            .find(|part| part.id == "selected-part")
            .and_then(|part| part.outline.as_ref())
            .and_then(|outline| outline.margin),
        Some(4.25),
        "margin Enter commits the draft to the accepted document"
    );

    input.focus().unwrap();
    input.set_value("8.0");
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_init).unwrap())
        .unwrap();
    let escape = web_sys::KeyboardEventInit::new();
    escape.set_key("Escape");
    escape.set_bubbles(true);
    input
        .dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &escape).unwrap(),
        )
        .unwrap();
    accept_pending(&probe).await;
    assert_eq!(
        input.value(),
        "4.25",
        "Escape restores the accepted value, which the Enter commit just moved"
    );
    assert_eq!(
        probe
            .runtime
            .model()
            .accepted
            .clone()
            .unwrap()
            .document
            .revision,
        accepted.document.revision,
        "Escape cancels the draft without another accepted edit"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_component_position_draft_survives_unrelated_acceptance_and_blur_uses_latest_owner()
{
    let (probe, root) =
        mounted_probe_configured("layout-component-inspector-refresh-test-root", |document| {
            document.layouts[0].part_ids.clear();
        })
        .await;
    settle_mounted_component_inspector(&probe).await;
    let input = |label: &str| position_input(probe.root_id, label);
    let type_value = |field: &web_sys::HtmlInputElement, value: &str| {
        field.focus().unwrap();
        field.set_value(value);
        let init = web_sys::EventInit::new();
        init.set_bubbles(true);
        field
            .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &init).unwrap())
            .unwrap();
    };

    click_component_inspector(probe.root_id, "button", "Board outline");
    click_component_inspector(probe.root_id, "button", "Layout constraint");
    settle_mounted_component_inspector(&probe).await;
    let x = input("X mm");
    type_value(&x, "7.25");

    let initial_revision = accepted_revision(&probe);
    let initial_context_generation = probe
        .projection
        .borrow()
        .as_ref()
        .unwrap()
        .owner
        .context_generation;
    // Advance accepted state without changing focus. Driving a control here would blur and
    // submit the draft before the unrelated acceptance being tested.
    let accepted_revision = probe.accept_unrelated_revision().await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        accepted_revision
    );
    assert_eq!(
        probe
            .projection
            .borrow()
            .as_ref()
            .unwrap()
            .owner
            .context_generation,
        initial_context_generation,
        "accepted revisions refresh action capture without ending component selection"
    );
    assert_eq!(
        x.value(),
        "7.25",
        "dirty focused X draft survives the refresh"
    );
    assert_eq!(
        input("Y mm").value(),
        "4.00",
        "clean Y follows the accepted position"
    );
    for name in ["Board outline", "Layout constraint"] {
        assert_eq!(
            component_inspector_control(probe.root_id, "button", name)
                .get_attribute("aria-expanded")
                .as_deref(),
            Some("true"),
            "disclosure {name} survives unrelated acceptance"
        );
    }

    let _ = x.blur();
    accept_pending(&probe).await;
    assert_eq!(
        accepted_position(&probe, "selected-part"),
        Vec2 { x: 7.25, y: 4.0 },
        "blur lands the retained X draft with the accepted Y against the refreshed owner"
    );
    assert!(accepted_revision > initial_revision);

    // The Relations tab holds its selection across a further unrelated acceptance on the
    // same mounted Inspector these draft checks use.
    click_component_inspector(probe.root_id, "tab", "Relations");
    settle_mounted_component_inspector(&probe).await;
    let relations_revision = probe.accept_unrelated_revision().await;
    settle_mounted_component_inspector(&probe).await;
    assert_eq!(
        probe.projection.borrow().as_ref().unwrap().owner.revision,
        relations_revision
    );
    assert_eq!(
        component_inspector_control(probe.root_id, "tab", "Relations")
            .get_attribute("aria-selected")
            .as_deref(),
        Some("true"),
        "Relations tab survives an unrelated accepted revision"
    );
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_finding_return_focus_reaches_the_selected_copied_inspector_tab() {
    let (probe, root) = mounted_probe("layout-finding-return-tab-focus-root").await;
    settle_mounted_component_inspector(&probe).await;
    click_component_inspector(probe.root_id, "tab", "Relations");
    settle_mounted_component_inspector(&probe).await;

    let document = web_sys::window().unwrap().document().unwrap();
    let inspector = document.create_element("section").unwrap();
    inspector.set_id("m1-inspector-panel-content");
    // Other concurrently mounted page fixtures may contain the production ID.
    // Put this short-lived fixture first so the production document query selects it.
    let body = document.body().unwrap();
    body.insert_before(&inspector, body.first_child().as_ref())
        .unwrap();
    inspector.append_child(&root).unwrap();
    let properties = component_inspector_control(probe.root_id, "tab", "Properties");
    properties
        .dyn_ref::<web_sys::HtmlElement>()
        .unwrap()
        .focus()
        .unwrap();
    let selected = component_inspector_control(probe.root_id, "tab", "Relations");

    assert_eq!(
        document
            .query_selector("#m1-inspector-panel-content .layout-inspector-tabs [role='tab'][aria-selected='true']")
            .unwrap(),
        Some(selected.clone().into()),
        "finding return selector must resolve to this fixture's selected tab"
    );
    assert!(
        boardstudio_web_layout::layout_findings_state::focus_layout_finding_return_destination()
    );
    assert_eq!(
        document.active_element(),
        Some(selected.into()),
        "finding return restores the selected tab on the actual mounted Inspector"
    );
    inspector.remove();
}
