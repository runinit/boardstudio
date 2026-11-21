use super::*;
use boardstudio_application::SnapshotToken;
use boardstudio_web_ui_shared::test_mount::MountedFixture;
use boardstudio_web_ui_shared::test_role::by_role_name;
use boardstudio_web_ui_shared::test_wait::wait_until;
use std::{cell::RefCell, rc::Rc, sync::Arc};
use wasm_bindgen::JsCast;
use wasm_bindgen_test::wasm_bindgen_test;

wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

fn entry(id: &str, name: &str, kind: &str) -> CatalogEntry {
    CatalogEntry {
        definition: Rc::new(
            serde_json::from_value(serde_json::json!({
                "id": id,
                "name": name,
                "kind": kind,
                "courtyard": [{"x": -2.0, "y": -2.0}, {"x": 2.0, "y": -2.0}, {"x": 2.0, "y": 2.0}],
                "pads": []
            }))
            .unwrap(),
        ),
        source: catalogue::CatalogueSource::Imported,
    }
}

fn retired_entry() -> CatalogEntry {
    let mut definition =
        (*entry("retired", "Battery legacy controller", "controller").definition).clone();
    definition.generator = Some(boardstudio_model::model::PartGenerator {
        source: "infused-kim/nice_nano_pretty".into(),
        version: "legacy".into(),
        parameters: Default::default(),
    });
    CatalogEntry {
        definition: Rc::new(definition),
        source: catalogue::CatalogueSource::Project,
    }
}

fn assembly_snapshot_entry() -> CatalogEntry {
    entry(
        "assembly-keyboard/definition/battery",
        "Battery assembly snapshot",
        "custom",
    )
}

#[derive(Clone, Copy)]
struct BrowseSignals {
    query: PartsQuery,
    workspace: Signal<&'static str>,
    compact_open: Signal<bool>,
    panel_settings: Signal<super::super::PanelSettings>,
    generation: Signal<u64>,
}

#[derive(Clone)]
struct CatalogueMountedFixture {
    snapshot: AcceptedSnapshot,
    runtime: Rc<crate::runtime::Runtime>,
}

#[component]
fn PartsLibraryMountedHost() -> Element {
    let fixture = use_context::<Rc<CatalogueMountedFixture>>();
    let query = use_signal(String::new);
    let selected = use_signal(|| None::<(Option<Scope>, String)>);
    let scope_generation = use_signal(|| 1_u64);
    let workspace = use_signal(|| "Parts");
    let generation = use_signal(|| 1_u64);
    let activation = use_signal(|| 0_u64);
    let assembly = use_signal(|| None);
    let orientation = use_signal(|| assembly_presets::SwitchOrientation::South);
    use_context_provider(|| Signal::new(0_u64));
    use_context_provider(|| PartsSelectionGeneration(generation));
    use_context_provider(|| PartsPreviewActivation(activation));
    use_context_provider(|| PartsAssemblySelection(assembly));
    use_context_provider(|| PartsAssemblyOrientation(orientation));
    rsx! {
        PartsLibraryPanel {
            snapshot: fixture.snapshot.clone(),
            scope: None,
            scope_generation,
            workspace,
            query,
            selected,
            on_select: EventHandler::default(),
        }
    }
}

/// The catalogue panel is given the accepted snapshot of a project the real Session opened.
async fn catalogue_fixture() -> CatalogueMountedFixture {
    let runtime = boardstudio_web_test_runtime::project_name_test_support::new_runtime();
    boardstudio_web_test_runtime::project_name_test_support::open_document(
        &runtime,
        ProjectDoc::empty("catalogue-loading-error", "Catalogue states"),
    )
    .await;
    let snapshot = runtime
        .model()
        .accepted
        .expect("the catalogue project is accepted");
    CatalogueMountedFixture { snapshot, runtime }
}

#[component]
fn BrowseChooserHost(compact: bool) -> Element {
    let signals = use_context::<Rc<RefCell<Option<BrowseSignals>>>>();
    let query = use_signal(String::new);
    let workspace = use_signal(|| "Layout");
    let compact_open = use_signal(|| false);
    let panel_settings = use_signal(|| super::super::PanelSettings {
        mode: super::super::PanelMode::Collapsed,
        width: None,
    });
    let generation = use_signal(|| 0u64);
    use_context_provider(|| PartsSelectionGeneration(generation));
    *signals.borrow_mut() = Some(BrowseSignals {
        query,
        workspace,
        compact_open,
        panel_settings,
        generation,
    });
    let layout_target = use_signal(|| None);
    let on_browse = EventHandler::new(move |_| {
        super::super::browse_parts_workspace(workspace, compact_open, panel_settings, compact);
    });
    rsx! {
        AddObjectComponentChooser {
            snapshot: AcceptedSnapshot {
                token: SnapshotToken(4),
                session_epoch: boardstudio_application::SessionEpoch(3),
                document: Arc::new(ProjectDoc::empty("browse-project", "Browse test")),
                scene: Arc::new(
                    serde_json::from_value(serde_json::json!({
                        "revision": 0,
                        "transactionId": "browse-test",
                        "changedIds": [],
                        "transforms": [],
                        "matrixScenes": [],
                        "contours": [],
                        "boardContours": [],
                        "boardReadiness": [],
                        "findings": [],
                        "readiness": {
                            "layout": false,
                            "outline": false,
                            "pcb": false,
                            "case": false
                        }
                    }))
                    .unwrap(),
                ),
            },
            scope: None,
            layout_target,
            query,
            on_browse,
            on_place: EventHandler::default(),
        }
    }
}

async fn browse_from_mounted_chooser(compact: bool) {
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id(if compact {
        "parts-browse-compact-test-root"
    } else {
        "parts-browse-desktop-test-root"
    });
    document.body().unwrap().append_child(&root).unwrap();
    let signals: Rc<RefCell<Option<BrowseSignals>>> = Rc::new(RefCell::new(None));
    let dom = VirtualDom::new_with_props(BrowseChooserHost, BrowseChooserHostProps { compact });
    dom.provide_root_context(signals.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);
    wait_until("Parts browse chooser mounts", || {
        by_role_name(&root, "searchbox", "Search parts").is_some()
    })
    .await;

    let input = by_role_name(&root, "searchbox", "Search parts")
        .unwrap()
        .dyn_into::<web_sys::HtmlInputElement>()
        .unwrap();
    input.set_value("battery");
    let input_event = web_sys::EventInit::new();
    input_event.set_bubbles(true);
    input
        .dispatch_event(&web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap())
        .unwrap();
    let signals = signals.borrow().as_ref().copied().unwrap();
    wait_until("Parts search draft updates", || {
        (signals.query)() == "battery"
    })
    .await;
    assert_eq!(
        boardstudio_web_ui_shared::test_role::accessible_name(input.as_ref()),
        "Search parts"
    );
    by_role_name(&root, "button", "Browse all parts")
        .unwrap()
        .click();
    wait_until("Parts browse action selects the workspace", || {
        (signals.generation)() == 1 && (signals.workspace)() == "Parts"
    })
    .await;

    assert_eq!((signals.query)(), "battery");
    assert_eq!((signals.generation)(), 1);
    assert_eq!((signals.workspace)(), "Parts");
    assert_eq!((signals.compact_open)(), compact);
    assert_eq!(
        (signals.panel_settings)().mode,
        if compact {
            super::super::PanelMode::Collapsed
        } else {
            super::super::PanelMode::Pinned
        }
    );
}

#[wasm_bindgen_test]
async fn mounted_parts_library_announces_catalogue_loading_then_source_error() {
    let (complete, result) =
        futures_channel::oneshot::channel::<Result<Rc<Vec<CatalogEntry>>, String>>();
    let pending = Rc::new(RefCell::new(Some(result)));
    let loader = CatalogueLoadTestPort(Rc::new(move |_reversible| {
        let result = pending.borrow_mut().take().expect("one catalogue request");
        Box::pin(async move {
            result
                .await
                .unwrap_or_else(|_| Err("catalogue test request dropped".into()))
        })
    }));
    let fixture = Rc::new(catalogue_fixture().await);
    let document = web_sys::window().unwrap().document().unwrap();
    let root = document.create_element("div").unwrap();
    root.set_id("parts-catalogue-source-state-test");
    document.body().unwrap().append_child(&root).unwrap();
    let dom = VirtualDom::new(PartsLibraryMountedHost);
    dom.provide_root_context(fixture.clone());
    dom.provide_root_context(loader);
    dom.provide_root_context(fixture.runtime.clone());
    let _mounted = MountedFixture::new(root.clone(), dom);

    wait_until("catalogue loading state renders", || {
        root.text_content()
            .unwrap_or_default()
            .contains("Loading component catalogue…")
    })
    .await;
    complete
        .send(Err("controlled catalogue source failure".into()))
        .unwrap();
    wait_until("catalogue source failure renders", || {
        by_role_name(
            &root,
            "alert",
            "Component catalogue could not be loaded: controlled catalogue source failure",
        )
        .is_some()
    })
    .await;
    let alert = by_role_name(
        &root,
        "alert",
        "Component catalogue could not be loaded: controlled catalogue source failure",
    )
    .unwrap();
    assert!(
        alert.text_content().unwrap().contains(
            "Component catalogue could not be loaded: controlled catalogue source failure"
        )
    );
    root.remove();
}

#[wasm_bindgen_test::wasm_bindgen_test]
async fn mounted_browse_button_preserves_search_and_pins_desktop_objects() {
    browse_from_mounted_chooser(false).await;
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn add_menu_uses_the_four_reference_groups_for_available_definitions() {
    let entries = vec![
        entry("battery", "Battery connector", "connector"),
        entry("controller", "nice!nano", "controller"),
        entry("display", "OLED display", "custom"),
        entry("encoder", "Rotary encoder EC11", "encoder"),
        entry("stabilizer", "MX stabilizer", "custom"),
        retired_entry(),
        assembly_snapshot_entry(),
    ];

    let choices = add_object_choices(&entries);
    assert_eq!(choices.len(), 5);
    assert!(choices.iter().all(|entry| {
        !entry.definition.id.starts_with("assembly-")
            && entry
                .definition
                .generator
                .as_ref()
                .is_none_or(|generator| generator.source != "infused-kim/nice_nano_pretty")
    }));
    let groups = add_object_groups(&choices);

    assert_eq!(
        groups.iter().map(|(label, _)| *label).collect::<Vec<_>>(),
        ["Components", "Controllers", "Displays", "Encoders"]
    );
    let group_ids = |label: &str| {
        groups
            .iter()
            .find(|(group, _)| *group == label)
            .unwrap()
            .1
            .iter()
            .map(|entry| entry.definition.id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(group_ids("Components"), ["battery"]);
    assert_eq!(group_ids("Controllers"), ["controller"]);
    assert_eq!(group_ids("Displays"), ["display"]);
    assert_eq!(group_ids("Encoders"), ["encoder"]);
    let battery_matches = add_object_search_results(&entries, "battery");
    assert_eq!(
        battery_matches
            .iter()
            .map(|entry| entry.definition.id.as_str())
            .collect::<Vec<_>>(),
        ["battery"]
    );
}

#[wasm_bindgen_test::wasm_bindgen_test]
fn key_assembly_search_hides_nonmatching_presets_without_losing_categories() {
    let all = matching_assembly_presets("");
    assert_eq!(all.len(), 8);
    assert_eq!(
        all.iter().map(|preset| preset.name).collect::<Vec<_>>(),
        [
            "MX Solder",
            "MX Hotswap",
            "Choc V1 Solder",
            "Choc V1 Hotswap",
            "MX RGB",
            "Choc V1 RGB",
            "MX Hotswap RGB",
            "Choc V1 Hotswap RGB",
        ]
    );

    let none = matching_assembly_presets("no-such-assembly");
    assert!(none.is_empty());

    let mx = matching_assembly_presets("mx");
    assert_eq!(
        mx.iter().map(|preset| preset.name).collect::<Vec<_>>(),
        ["MX Solder", "MX Hotswap", "MX RGB", "MX Hotswap RGB"]
    );
    assert_eq!(matching_assembly_presets("key assembly").len(), 8);
}
