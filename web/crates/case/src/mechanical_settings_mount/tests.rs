use super::*;
use boardstudio_model::model::{Board, PartDefinition, PartGenerator, Pose2, Side};
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn workspace_props_host() -> Element {
    crate::test_contexts::use_case_viewer_test_contexts();
    boardstudio_web_ui_model::state::use_empty_test_instance_selection();
    let runtime = use_context::<Rc<Runtime>>();
    let crate::WorkspaceState(mut workspace) = use_context::<crate::WorkspaceState>();
    let mut version = use_signal(|| 0_u64);
    use_context_provider(|| version);
    let mount = use_mechanical_settings_mount(
        runtime,
        use_context::<crate::selection::SelectionAdapter>().generation,
        workspace,
        use_context::<InstanceSelection>(),
        use_context::<CaseSelection>(),
        EventHandler::new(|_| {}),
    );
    rsx! {
        Button { id: "show-layout", onclick: move |_| workspace.set("Layout"), "Layout" }
        Button { id: "show-case", onclick: move |_| workspace.set("Case"), "Case" }
        output { id: "workspace", "{workspace()}" }
        output { id: "case-props", "{mount.props.is_some()}" }
        Button { id: "refresh-case-source", onclick: move |_| version += 1, "Refresh" }
        output { id: "source-version", "{version()}" }
    }
}

#[wasm_bindgen_test]
async fn hidden_case_skips_presentation_props_and_rebuilds_them_on_return() {
    use boardstudio_web_test_runtime::in_process_support;
    use boardstudio_web_test_runtime::project_name_test_support as support;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use wasm_bindgen::JsCast;

    let runtime = support::new_runtime();
    support::open_document(&runtime, in_process_support::board_document()).await;
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    document.body().unwrap().append_child(&root).unwrap();
    let mut dom = VirtualDom::new(workspace_props_host);
    dom.insert_any_root_context(Box::new(runtime.clone()));
    let _mounted = MountedFixture::new(root.clone(), dom);
    let props_visible = || {
        root.query_selector("#case-props")
            .ok()
            .flatten()
            .and_then(|element| element.text_content())
            .as_deref()
            == Some("true")
    };
    wait_until("Case prepares presentation props", props_visible).await;
    root.query_selector("#show-layout")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until("workspace switches to Layout", || {
        root.query_selector("#workspace")
            .ok()
            .flatten()
            .and_then(|element| element.text_content())
            .as_deref()
            == Some("Layout")
    })
    .await;
    assert!(
        !props_visible(),
        "hidden Case must skip preparing presentation props"
    );
    SOURCE_PROJECTIONS.set(0);
    let mut changed = runtime.model().accepted.unwrap().document.as_ref().clone();
    changed.name = "Changed in Layout".into();
    support::replace_document(&runtime, "hidden-case-change", &changed.id.clone(), changed).await;
    root.query_selector("#refresh-case-source")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until("hidden Case observes the accepted update", || {
        root.query_selector("#source-version")
            .unwrap()
            .unwrap()
            .text_content()
            .as_deref()
            == Some("1")
    })
    .await;
    assert_eq!(
        SOURCE_PROJECTIONS.get(),
        0,
        "Layout commits must not prepare Case geometry"
    );
    root.query_selector("#show-case")
        .unwrap()
        .unwrap()
        .dyn_into::<web_sys::HtmlElement>()
        .unwrap()
        .click();
    wait_until(
        "Case rebuilds presentation props after return",
        props_visible,
    )
    .await;
    assert!(
        SOURCE_PROJECTIONS.get() > 0,
        "returning to Case prepares the latest source"
    );
    root.remove();
}

fn assembly_with_plate(thickness: f64) -> MechanicalAssembly {
    serde_json::from_value(serde_json::json!({
        "revision": 1,
        "case": { "revision": 1, "bodies": [] },
        "stack": [{ "id": "plate", "z": 3.5, "thickness": thickness }],
        "suggestedMounts": [],
        "nominalPlateContours": [],
        "plateContours": [],
        "diagnostics": []
    }))
    .expect("valid minimal mechanical assembly")
}

#[wasm_bindgen_test]
fn stabilizer_projection_uses_accepted_wide_mx_keys_and_current_override_kind() {
    let mut document = ProjectDoc::empty("doc", "doc");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["wide-key".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.parts.push(Part {
        keycap: Some(boardstudio_model::model::Vec2 { x: 37.0, y: 42.0 }),
        outline: None,
        id: "wide-key".into(),
        definition_id: "mx-switch".into(),
        reference: "K1".into(),
        pose: Pose2 {
            at: boardstudio_model::model::Vec2 { x: 10.0, y: 20.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    });
    document.definitions.push(PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: "mx-switch".into(),
        name: "MX switch".into(),
        kind: PartKind::Switch,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: Some(PartGenerator {
            source: "ceoloide/switch_mx".into(),
            version: "1".into(),
            parameters: Default::default(),
        }),
        mechanical_profile: None,
    });
    let mut configuration: MechanicalConfiguration = serde_json::from_value(serde_json::json!({
        "boardId": "board",
        "method": "printed",
        "mount": "tray",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 2.0,
        "batteryHeight": 0.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "profiles": []
    }))
    .unwrap();
    configuration.stabilizers = Some(vec![MechanicalStabilizerOverride {
        profile: None,
        part_id: "wide-key".into(),
        kind: MechanicalStabilizerKind::None,
        units: 6.25,
        rotation: Some(45.0),
    }]);

    let fits = stabilizer_fits(&document, &configuration);
    assert_eq!(fits.len(), 1);
    assert_eq!(fits[0].key_units, 2.25);
    assert_eq!(fits[0].default.units, 2.0);
    assert_eq!(fits[0].default.rotation, Some(90.0));
    assert_eq!(fits[0].current.kind, MechanicalStabilizerKind::None);
    assert_eq!(fits[0].current.units, 2.0);
    assert_eq!(fits[0].current.rotation, Some(90.0));
}

#[wasm_bindgen_test]
fn current_resolution_layers_precede_previous_display_layers() {
    let previous = assembly_with_plate(1.5);
    let current = assembly_with_plate(2.0);
    let rows = project_layer_rows(Some(&previous), Some(&current), false, true);

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].thickness, 2.0);
    assert!(!rows[0].is_previous);

    let fallback = project_layer_rows(Some(&previous), None, false, true);
    assert_eq!(fallback.len(), 1);
    assert_eq!(fallback[0].thickness, 1.5);
    assert!(fallback[0].is_previous);
}

#[wasm_bindgen_test]
fn mounted_current_resolution_layer_is_selectable_before_scene_completion() {
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};

    let identity = MechanicalSettingsIdentity {
        editor_instance_id: 1,
        scope_generation: 3,
        presentation_generation: 2,
        scope: Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        },
        snapshot_token: SnapshotToken(8),
        revision: 12,
        active_board_id: "board".into(),
        configuration_board_id: "board".into(),
    };
    let effective_configuration = serde_json::from_value(serde_json::json!({
        "boardId": "board",
        "method": "printed",
        "mount": "tray",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.0,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.0,
        "batteryHeight": 0.0,
        "bottomThickness": 2.0,
        "plateToPcb": 3.1,
        "wallThickness": 2.0,
        "clearance": 0.5,
        "profiles": []
    }))
    .expect("minimal effective configuration");
    let projection = MechanicalSettingsResolvedProjection {
        key: SettingsSourceKey {
            scope: identity.scope.clone(),
            token: identity.snapshot_token,
            revision: identity.revision,
        },
        identity: identity.clone(),
        resolution: Rc::new(MechanicalResolution {
            assembly: assembly_with_plate(2.0),
            effective_configuration,
        }),
    };

    assert!(resolved_projection_layer_is_selectable(
        &projection,
        &identity,
        "plate"
    ));

    let mut stale_identity = identity.clone();
    stale_identity.snapshot_token = SnapshotToken(7);
    assert!(!resolved_projection_layer_is_selectable(
        &projection,
        &stale_identity,
        "plate"
    ));
}
