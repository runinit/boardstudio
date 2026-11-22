use super::*;

#[wasm_bindgen_test]
async fn mounted_mirrored_half_actions_work_from_either_board_and_undo() {
    use boardstudio_core::model::{HalfExceptionState, MirroredExceptionState};
    use boardstudio_web_ui_shared::test_wait::wait_until_step;

    let (probe, root) = mounted_runtime_probe(
        "layout-mirrored-half-round-trip-test-root",
        configure_mirrored_part_pair,
        &["selected-part"],
    )
    .await;
    async fn drive_to_state(
        probe: &MountedProbe,
        expected: HalfExceptionState,
        previous_revision: u64,
    ) {
        wait_until_step("the mirrored-half state is accepted", async || {
            accept_pending(probe).await;
            let state = probe
                .runtime
                .model()
                .accepted
                .as_ref()
                .unwrap()
                .document
                .hardware
                .as_ref()
                .unwrap()
                .pairs[0]
                .halves
                .as_ref()
                .unwrap()
                .exceptions
                .iter()
                .find(|exception| exception.mirror_id == "source-part")
                .map(|exception| match exception.state {
                    MirroredExceptionState::Local => HalfExceptionState::Local,
                    MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
                })
                .unwrap_or(HalfExceptionState::Linked);
            let model = probe.runtime.model();
            state == expected
                && model
                    .accepted
                    .as_ref()
                    .is_some_and(|accepted| accepted.document.revision > previous_revision)
        })
        .await;
        let accepted_state = probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .hardware
            .as_ref()
            .unwrap()
            .pairs[0]
            .halves
            .as_ref()
            .unwrap()
            .exceptions
            .iter()
            .find(|exception| exception.mirror_id == "source-part")
            .map(|exception| match exception.state {
                MirroredExceptionState::Local => HalfExceptionState::Local,
                MirroredExceptionState::Omitted => HalfExceptionState::Omitted,
            })
            .unwrap_or(HalfExceptionState::Linked);
        assert_eq!(
            accepted_state,
            expected,
            "runtime error: {:?}; revision: {} (expected after {})",
            probe.runtime.model().last_error,
            probe.runtime.model().accepted.unwrap().document.revision,
            previous_revision,
        );
        use boardstudio_web_ui_shared::test_wait::wait_until;
        wait_until(
            "the Inspector projects the accepted mirrored-half state",
            || {
                probe.refresh();
                probe
                    .projection
                    .borrow()
                    .as_ref()
                    .and_then(|projection| projection.mirrored_half.as_ref())
                    .is_some_and(|half| half.state == expected)
            },
        )
        .await;
    }

    use boardstudio_web_ui_shared::test_wait::wait_until;
    wait_until("the source part Inspector mounts", || {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.mirrored_half.as_ref())
            .is_some_and(|half| half.state == HalfExceptionState::Linked)
    })
    .await;
    let revision = accepted_revision(&probe);
    click_component_inspector(probe.root_id, "button", "Mirrored half");
    click_component_inspector(probe.root_id, "button", "Omit");
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetMirroredHalfState {
            state: HalfExceptionState::Omitted,
            ..
        })
    ));
    drive_to_state(&probe, HalfExceptionState::Omitted, revision).await;

    let revision = accepted_revision(&probe);
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    drive_to_state(&probe, HalfExceptionState::Linked, revision).await;

    probe
        .runtime
        .submit(boardstudio_application::Event::Navigate {
            operation_id: probe.runtime.operation(),
            board_id: "mirror-board".into(),
            instance_id: None,
        });
    boardstudio_web_test_runtime::project_name_test_support::run_pending(&probe.runtime).await;
    probe.select(Some("source-part"));
    probe.refresh();
    wait_until("the counterpart Inspector mounts", || {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.mirrored_half.as_ref())
            .is_some_and(|half| half.state == HalfExceptionState::Linked)
    })
    .await;
    wait_until("the mirror-board selection owns the Inspector", || {
        probe
            .adapter_context
            .borrow()
            .as_ref()
            .is_some_and(|selected| {
                selected.scope.board_id == "mirror-board"
                    && matches!(
                        &selected.context,
                        objects::TreeContext::Component { part_id: Some(part_id), .. }
                            if part_id == "source-part"
                    )
            })
    })
    .await;
    let revision = accepted_revision(&probe);
    click_component_inspector(probe.root_id, "button", "Mirrored half");
    click_component_inspector(probe.root_id, "button", "Make local");
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetMirroredHalfState {
            state: HalfExceptionState::Local,
            ..
        })
    ));
    drive_to_state(&probe, HalfExceptionState::Local, revision).await;

    let revision = accepted_revision(&probe);
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    drive_to_state(&probe, HalfExceptionState::Linked, revision).await;
    root.remove();
}

#[wasm_bindgen_test]
async fn mounted_part_mirror_toggle_mirrors_and_unmirrors_with_undo() {
    use boardstudio_web_ui_shared::test_wait::wait_until_step;

    async fn settled_part_mirror(probe: &MountedProbe, expected: Option<bool>) -> bool {
        accept_pending(probe).await;
        current_revision_is_saved(probe)
            && probe
                .projection
                .borrow()
                .as_ref()
                .and_then(|projection| projection.part_mirror)
                == expected
    }

    let (probe, root) = mounted_runtime_probe(
        "layout-part-mirror-toggle-test-root",
        configure_gapped_part_mirror,
        &["selected-part"],
    )
    .await;
    settle_mounted_component_inspector(&probe).await;

    fn part_mirror(probe: &MountedProbe) -> Option<bool> {
        probe
            .projection
            .borrow()
            .as_ref()
            .and_then(|projection| projection.part_mirror)
    }

    fn accepted_part_records(probe: &MountedProbe) -> Vec<MirroredCorrespondence> {
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .expect("fixture snapshot")
            .document
            .layouts
            .iter()
            .filter_map(|layout| layout.mirror_link.as_ref())
            .flat_map(|link| link.correspondences.clone())
            .filter(|record| record.kind == MirroredItemKind::Part)
            .collect()
    }

    fn toggle_part_mirror(root_id: &str, mirrored: bool) {
        let root = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id(root_id)
            .expect("inspector root");
        let checkbox =
            by_role_name(&root, "checkbox", "Mirror on both halves").expect("part mirror toggle");
        assert_eq!(
            checkbox.get_attribute("aria-checked").as_deref(),
            Some(if mirrored { "false" } else { "true" }),
        );
        checkbox.click();
    }

    // The independent part shows the ticket-12 toggle instead of the member
    // exception buttons — checked while its Part correspondence is live.
    wait_until("the part mirror toggle mounts checked", || {
        probe.refresh();
        part_mirror(&probe) == Some(true)
    })
    .await;
    click_component_inspector(probe.root_id, "button", "Mirrored half");
    assert!(
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!(
                "#{} #component-inspector-mirrored-half-local",
                probe.root_id
            ))
            .unwrap()
            .is_none(),
        "independent parts use the toggle, not the member exception buttons"
    );

    // Off: the partner and its record go away together, in one Undo entry.
    let revision = accepted_revision(&probe);
    toggle_part_mirror(probe.root_id, false);
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetPartMirrored {
            part_id,
            mirrored: false,
            ..
        }) if part_id == "selected-part"
    ));
    wait_until_step("unmirroring is accepted", async || {
        accept_pending(&probe).await;
        current_revision_is_saved(&probe)
            && accepted_part_records(&probe).is_empty()
            && part_mirror(&probe) == Some(false)
    })
    .await;
    assert!(
        accepted_revision(&probe) > revision,
        "the toggle off commits one Undo entry"
    );
    assert!(
        probe
            .runtime
            .model()
            .accepted
            .as_ref()
            .expect("fixture snapshot")
            .document
            .parts
            .iter()
            .all(|part| part.id != "selected-part-mirror"),
        "the partner part is removed with its record"
    );

    // On: the partner returns at the reflected pose, in one Undo entry.
    let revision = accepted_revision(&probe);
    toggle_part_mirror(probe.root_id, true);
    assert!(matches!(
        probe.last_action.borrow().as_ref(),
        Some(LayoutComponentInspectorAction::SetPartMirrored {
            part_id,
            mirrored: true,
            ..
        }) if part_id == "selected-part"
    ));
    wait_until_step("remirroring is accepted", async || {
        accept_pending(&probe).await;
        current_revision_is_saved(&probe) && part_mirror(&probe) == Some(true)
    })
    .await;
    assert!(
        accepted_revision(&probe) > revision,
        "the toggle on commits one Undo entry"
    );
    let records = accepted_part_records(&probe);
    assert_eq!(records.len(), 1);
    let partner = probe
        .runtime
        .model()
        .accepted
        .as_ref()
        .expect("fixture snapshot")
        .document
        .parts
        .iter()
        .find(|part| part.id == records[0].mirror_id)
        .expect("the partner part returns with its record")
        .clone();
    assert_eq!(partner.pose.at, Vec2 { x: -4.0, y: 3.0 });

    // One Undo entry each way: undo unmirrors, undo again remirrors.
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    wait_until_step("undo unmirrors the part", async || {
        settled_part_mirror(&probe, Some(false)).await
    })
    .await;
    probe.runtime.submit(boardstudio_application::Event::Undo {
        operation_id: probe.runtime.operation(),
    });
    wait_until_step("undo remirrors the part", async || {
        settled_part_mirror(&probe, Some(true)).await
    })
    .await;
    root.remove();
}

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
                matrix_ids: vec!["component-paired-matrix".into()],
                part_ids: vec![],
                mirror_link: Some(LayoutMirrorLink {
                    correspondences: vec![MirroredCorrespondence {
                        companion_mode: None,
                        companion_tweak: None,
                        kind: MirroredItemKind::Matrix,
                        source_id: "component-matrix".into(),
                        mirror_id: "component-paired-matrix".into(),
                    }],
                    exceptions: vec![],
                    source_id: "component-layout".into(),
                    axis_x: 0.0,
                    split_gap: None,
                    spacing: None,
                }),
            });
            document.matrices.clear();
            document.matrices.push(Matrix {
                id: "component-matrix".into(),
                name: Some("keys".into()),
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "component-definition".into(),
                part_ids: vec![],
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
            document.matrices.push(Matrix {
                id: "component-paired-matrix".into(),
                name: Some("component-paired-matrix".into()),
                rows: 1,
                columns: 1,
                pitch: Vec2 { x: 19.0, y: 19.0 },
                origin: Vec2::default(),
                definition_id: "component-definition".into(),
                part_ids: vec![],
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
        &["matrix/component-paired-matrix/r0c0"],
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
