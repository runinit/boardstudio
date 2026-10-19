//! Parts catalogue, selected-definition editing and placement presentation.
mod assembly_editor;
mod assembly_presets;
pub use boardstudio_web_catalogue::bundled::{
    generator_parameter_schema, is_generator_source, load_all_catalogue_definitions,
    load_component_definition, load_matrix_templates, normalize_matrix_definition,
    prepare_physical_setup_proposal, prepare_physical_setup_resources, reversible_layout,
};
use boardstudio_web_catalogue::catalogue;
use boardstudio_web_ui_shared::controls::BoardButton;
use boardstudio_web_ui_shared::controls::BoardDisclosure;
mod component_model_editor;
mod details;
mod generator_settings;
#[cfg(any(target_arch = "wasm32", test))]
mod mechanical_profile_editor;
#[cfg(target_arch = "wasm32")]
mod mechanical_profile_ui;
mod module_profile_editor;
mod modules_catalogue;
pub use modules_catalogue::module_attachment::AttachedModuleNavigation;
mod preview;
mod standard_profile_lifetime;
pub use assembly_presets::MatrixPresetId;

/// Data source for the existing Layout matrix-placement lifecycle. Saved
/// assembly recipes carry only their source definitions; Core still validates
/// and commits the matrix plus definition snapshots through SetMatrix.
#[derive(Clone, Copy)]
struct AssemblyCreateAction(Signal<Option<(ScopeId, EventHandler<MouseEvent>)>>);

pub fn use_assembly_action_provider() {
    let action = use_signal(|| None);
    use_context_provider(|| AssemblyCreateAction(action));
}

#[component]
fn AssemblyCreateButton() -> Element {
    let action = try_consume_context::<AssemblyCreateAction>();
    let current = action.and_then(|action| action.0.read().as_ref().map(|(_, action)| *action));
    rsx! { BoardButton { aria_label: "New assembly", title: "New key assembly", disabled: current.is_none(),
        onclick: move |event: MouseEvent| { event.prevent_default(); event.stop_propagation(); if let Some(action) = current { action.call(event); } },
        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 4v12M4 10h12" } }
    } }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixPlacementSource {
    Preset(MatrixPresetId),
    Assembly {
        assembly: boardstudio_core::model::AssemblyDefinition,
        definitions: Vec<boardstudio_core::model::PartDefinition>,
    },
}

pub use assembly_presets::SwitchOrientation;
pub use assembly_presets::matrix_with_assembly;

pub fn matrix_setup_preset(
    preset: assembly_presets::MatrixPresetId,
) -> crate::matrix_setup_operation::MatrixSetupPreset {
    use crate::matrix_setup_operation::MatrixSetupPreset as SetupPreset;
    use assembly_presets::MatrixPresetId as PartsPreset;
    match preset {
        PartsPreset::MxSolder => SetupPreset::MxSolder,
        PartsPreset::MxHotswap => SetupPreset::MxHotswap,
        PartsPreset::ChocSolder => SetupPreset::ChocSolder,
        PartsPreset::ChocHotswap => SetupPreset::ChocHotswap,
        PartsPreset::MxRgb => SetupPreset::MxRgb,
        PartsPreset::ChocRgb => SetupPreset::ChocRgb,
        PartsPreset::MxHotswapRgb => SetupPreset::MxHotswapRgb,
        PartsPreset::ChocHotswapRgb => SetupPreset::ChocHotswapRgb,
    }
}
pub use generator_settings::GeneratorPreviewStatus;

#[cfg(any(target_arch = "wasm32", test))]
use mechanical_profile_editor::{
    AcceptedProfileOwner, CurrentProfileScope, DetachedProfileSpawner, ManualProfileEditor,
    ManualProfileEditorPorts, MechanicalExtractionFuture, MechanicalExtractionRequester,
    StandardProfileFuture, StandardProfileRequester,
};
pub use preview::PartsPreviewPanel;
pub use standard_profile_lifetime::PartsStandardProfileLifetime;
#[cfg(all(test, target_arch = "wasm32"))]
mod placement_action_tests;

use crate::parts_mechanical_profile::ProfileDefinitionSource;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::ProjectDoc;
use catalogue::{CatalogEntry, catalogue_choices, group_choices, preferred_label};
use details::SelectedDefinition;
use dioxus::prelude::*;
#[cfg(target_arch = "wasm32")]
use mechanical_profile_ui::PartsMechanicalProfileWorkspace;
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// Parent-owned interaction state keeps the reference library query and choice
/// alive while the editor switches between workspaces.
pub type PartsQuery = Signal<String>;
pub type PartsSelection = Signal<Option<(Option<Scope>, String)>>;

#[derive(Clone, Copy)]
pub struct PartsAssemblySelection(pub Signal<Option<assembly_presets::MatrixPresetId>>);

#[derive(Clone, Copy)]
pub struct PartsAssemblyOrientation(pub Signal<assembly_presets::SwitchOrientation>);

#[derive(Clone, Copy)]
pub struct PartsSelectionGeneration(pub Signal<u64>);

/// Explicit catalogue activation resets the local preview mode. Search/filter
/// changes use `PartsSelectionGeneration` for stale-work admission, but do not
/// count as a user selecting a catalogue item.
#[derive(Clone, Copy)]
pub struct PartsPreviewActivation(pub Signal<u64>);

#[derive(Clone, PartialEq)]
struct AssemblyRecipeRequest {
    scope: Option<Scope>,
    snapshot_token: SnapshotToken,
    selection: Option<(Option<Scope>, String)>,
    selection_generation: u64,
    scope_generation: u64,
    preset: Option<assembly_presets::MatrixPresetId>,
    orientation: assembly_presets::SwitchOrientation,
    reversible: bool,
    entries: Option<Rc<Vec<CatalogEntry>>>,
    preview_definition: Option<boardstudio_core::model::PartDefinition>,
}

impl AssemblyRecipeRequest {
    fn identity(&self) -> String {
        format!(
            "{:?}|{:?}|{:?}|{}|{}|{:?}|{:?}|{}|{:?}|{}",
            self.scope,
            self.snapshot_token,
            self.selection,
            self.selection_generation,
            self.scope_generation,
            self.preset,
            self.orientation,
            self.reversible,
            self.entries
                .as_ref()
                .map(|entries| Rc::as_ptr(entries) as usize),
            self.preview_definition
                .as_ref()
                .and_then(|definition| serde_json::to_string(definition).ok())
                .unwrap_or_default(),
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorPreviewDraft {
    owner: generator_settings::GeneratorOwner,
    definition: Option<boardstudio_core::model::PartDefinition>,
    status: generator_settings::GeneratorPreviewStatus,
}

#[derive(Clone, Copy)]
pub struct GeneratorDraftStore(pub Signal<Option<GeneratorPreviewDraft>>);
/// React's library predicate is advisory; Core remains authoritative when SetMatrix is accepted.
pub fn matrix_input_available(definition: &boardstudio_core::model::PartDefinition) -> bool {
    let press = if let Some(profile) = definition.input_profile.as_ref() {
        profile
            .press
            .as_ref()
            .map(|press| (press.row.clone(), press.column.clone(), press.independent))
    } else {
        definition
            .matrix_terminals
            .as_ref()
            .map(|press| (press.row.clone(), press.column.clone(), true))
            .or_else(|| {
                definition
                    .generator
                    .as_ref()
                    .filter(|generator| generator.source == "ceoloide/rotary_encoder_ec11_ec12")
                    .map(|_| ("S1".into(), "S2".into(), true))
            })
    };
    let Some((row_terminal, column_terminal, independent)) = press else {
        return definition.input_profile.is_none()
            && definition.pads.iter().any(|pad| pad.id == "one")
            && definition.pads.iter().any(|pad| pad.id == "two");
    };
    let pads_for = |terminal: &str| {
        definition
            .terminals
            .get(terminal)
            .cloned()
            .unwrap_or_else(|| {
                definition
                    .pads
                    .iter()
                    .filter(|pad| pad.id == terminal || pad.number == terminal)
                    .map(|pad| pad.id.clone())
                    .collect()
            })
    };
    let row = pads_for(&row_terminal);
    let column = pads_for(&column_terminal);
    let rotation = definition
        .input_profile
        .as_ref()
        .and_then(|profile| profile.rotary.as_ref())
        .into_iter()
        .flat_map(|rotation| [&rotation.a, &rotation.b, &rotation.common])
        .flat_map(|terminal| pads_for(terminal))
        .collect::<std::collections::BTreeSet<_>>();
    independent
        && !row.is_empty()
        && !column.is_empty()
        && row.iter().all(|id| !column.contains(id))
        && row
            .iter()
            .chain(&column)
            .all(|id| definition.pads.iter().any(|pad| pad.id == *id) && !rotation.contains(id))
}

#[derive(Clone)]
struct CatalogueView {
    entries: Option<Rc<Vec<CatalogEntry>>>,
    error: Option<String>,
}

#[cfg(test)]
type CatalogueLoadFuture =
    std::pin::Pin<Box<dyn std::future::Future<Output = Result<Rc<Vec<CatalogEntry>>, String>>>>;

/// Per-host source boundary for mounted loading/error-state tests only.
#[cfg(test)]
#[derive(Clone)]
struct CatalogueLoadTestPort(Rc<dyn Fn(bool) -> CatalogueLoadFuture>);

struct CachedProjectCatalogue {
    scope: Option<Scope>,
    token: SnapshotToken,
    document: Arc<ProjectDoc>,
    bundled: Rc<Vec<CatalogEntry>>,
    entries: Rc<Vec<CatalogEntry>>,
}

thread_local! {
    static PROJECT_CATALOGUE_CACHE: RefCell<Vec<CachedProjectCatalogue>> = const { RefCell::new(Vec::new()) };
}

/// Reuse the normalized bundled catalogue for project-owned closure projection.
/// Project overrides never replace this generator template.
pub async fn load_horizontal_host_connector_definition()
-> Result<boardstudio_core::model::PartDefinition, String> {
    modules_catalogue::load_horizontal_host_connector_definition().await
}

pub fn placement_label(definition: &boardstudio_core::model::PartDefinition) -> &str {
    preferred_label(definition)
}

fn has_unique_mirror_board_pair(document: &ProjectDoc, board_id: &str) -> bool {
    let Some(hardware) = document.hardware.as_ref() else {
        return false;
    };
    let mut matches = hardware.pairs.iter().filter(|pair| {
        pair.halves
            .as_ref()
            .is_some_and(|halves| halves.mirror_board_id == board_id)
    });
    let Some(pair) = matches.next() else {
        return false;
    };
    if matches.next().is_some() {
        return false;
    }
    let Some(halves) = pair.halves.as_ref() else {
        return false;
    };
    halves.source_board_id != halves.mirror_board_id
        && [
            halves.source_board_id.as_str(),
            halves.mirror_board_id.as_str(),
        ]
        .into_iter()
        .all(|id| {
            document
                .boards
                .iter()
                .filter(|board| board.id == id)
                .count()
                == 1
        })
}

#[component]
fn AddObjectChoice(
    definition_id: String,
    kind: boardstudio_core::model::PartKind,
    label: String,
    mirror_only_available: bool,
    on_place: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let linked_definition_id = definition_id.clone();
    let linked_kind = kind.clone();
    let mirror_definition_id = definition_id.clone();
    let mirror_kind = kind.clone();
    rsx! {
        div { role: "group", "aria-label": "Add {label}",
            button {
                key: "{definition_id}",
                class: "m1-parts-catalogue-choice",
                r#type: "button",
                "aria-label": "Add {label}",
                onclick: move |_| on_place.call(
                    super::part_placement::ComponentPlacementAction::AddObject {
                        definition_id: linked_definition_id.clone(),
                        kind: linked_kind.clone(),
                    }
                ),
                "{label}"
            }
            if mirror_only_available {
                button {
                    class: "m1-parts-catalogue-choice m1-mirror-only-choice",
                    r#type: "button",
                    "aria-label": "Add {label} to this half only",
                    onclick: move |_| on_place.call(
                        super::part_placement::ComponentPlacementAction::AddMirrorOnlyObject {
                            definition_id: mirror_definition_id.clone(),
                            kind: mirror_kind.clone(),
                        }
                    ),
                    "Only this half"
                }
            }
        }
    }
}

#[component]
pub fn AddObjectComponentChooser(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    layout_target: Signal<Option<String>>,
    mut query: PartsQuery,
    on_browse: EventHandler<()>,
    on_place: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let mut selection_generation = use_context::<PartsSelectionGeneration>().0;
    let board_id = scope.as_ref().map(|scope| scope.board_id.as_str());
    let mirror_only_available =
        board_id.is_some_and(|board_id| has_unique_mirror_board_pair(&snapshot.document, board_id));
    let layouts = snapshot
        .document
        .layouts
        .iter()
        .filter(|layout| Some(layout.board_id.as_str()) == board_id)
        .collect::<Vec<_>>();
    let selected_layout = layout_target()
        .filter(|id| layouts.iter().any(|layout| layout.id == *id))
        .unwrap_or_default();
    let search = query().trim().to_lowercase();
    let content = if let Some(entries) = catalogue.entries {
        let choices = add_object_choices(&entries);
        if choices.is_empty() {
            rsx! { p { class: "m1-parts-empty", role: "status", "No component definitions are available." } }
        } else if search.is_empty() {
            let groups = add_object_groups(&choices);
            rsx! {
                for (label, items) in groups {
                    BoardDisclosure { key: "{label}", class: "m1-parts-category", open: label == "Components",
                        heading: rsx! { summary { "{label}" } },
                        div { class: "m1-add-part-results",
                            for entry in items {
                                { let definition_id = entry.definition.id.clone();
                                  let kind = entry.definition.kind.clone();
                                  let label = preferred_label(&entry.definition);
                                  rsx! { AddObjectChoice {
                                      definition_id,
                                      kind,
                                      label: label.to_string(),
                                      mirror_only_available,
                                      on_place,
                                  } }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            let results = add_object_search_results(&entries, &search);
            if results.is_empty() {
                rsx! { p { class: "m1-parts-empty", role: "status", "No parts match this search." } }
            } else {
                rsx! {
                    div { class: "m1-add-part-results",
                        for entry in results {
                            { let definition_id = entry.definition.id.clone();
                              let kind = entry.definition.kind.clone();
                              let label = preferred_label(&entry.definition);
                              rsx! { AddObjectChoice {
                                  definition_id,
                                  kind,
                                  label: label.to_string(),
                                  mirror_only_available,
                                  on_place,
                              } }
                            }
                        }
                    }
                }
            }
        }
    } else if let Some(error) = catalogue.error {
        rsx! { p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" } }
    } else {
        rsx! { p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" } }
    };
    rsx! {
        div { class: "m1-add-component-chooser",
            if !layouts.is_empty() {
                label { class: "m1-parts-placement-layout",
                    "Place in"
                    select {
                        "aria-label": "Part placement layout",
                        value: "{selected_layout}",
                        onchange: move |event| {
                            layout_target.set((!event.value().is_empty()).then(|| event.value()));
                        },
                        option { value: "", "Board / ungrouped" }
                        for layout in layouts {
                            option { key: "{layout.id}", value: "{layout.id}", "{layout.name}" }
                        }
                    }
                }
            }
            label { class: "m1-parts-search-label", "Parts"
                input {
                    class: "m1-parts-search",
                    type: "search",
                    "aria-label": "Search parts",
                    placeholder: "Find a part…",
                    value: "{query()}",
                    oninput: move |event| {
                        query.set(event.value());
                        selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                    },
                }
            }
            {content}
            BoardButton {
                class: "m1-add-browse",
                r#type: "button",
                onclick: move |_| on_browse.call(()),
                "Browse all parts"
            }
        }
    }
}

fn add_object_search_results<'a>(
    entries: &'a [CatalogEntry],
    search: &str,
) -> Vec<&'a CatalogEntry> {
    add_object_choices(entries)
        .into_iter()
        .filter(|entry| entry.matches_library_search(search))
        .collect()
}

fn matching_assembly_presets(search: &str) -> Vec<assembly_presets::Preset> {
    let search = search.trim().to_lowercase();
    assembly_presets::PRESETS
        .iter()
        .copied()
        .filter(|preset| {
            search.is_empty()
                || format!("{} key assembly", preset.name)
                    .to_lowercase()
                    .contains(&search)
        })
        .collect()
}

fn add_object_choices(entries: &[CatalogEntry]) -> Vec<&CatalogEntry> {
    catalogue_choices(entries)
}

fn add_object_groups<'a>(
    entries: &[&'a CatalogEntry],
) -> Vec<(&'static str, Vec<&'a CatalogEntry>)> {
    type GroupMatcher = fn(&CatalogEntry) -> bool;
    let groups: [(&str, GroupMatcher); 4] = [
        ("Components", |entry| {
            let name = entry.definition.name.to_lowercase();
            ["power", "reset", "battery"]
                .iter()
                .any(|needle| name.contains(needle))
        }),
        ("Controllers", |entry| {
            matches!(
                entry.definition.kind,
                boardstudio_core::model::PartKind::Controller
            )
        }),
        ("Displays", |entry| {
            let name = entry.definition.name.to_lowercase();
            ["display", "oled", "nice.view"]
                .iter()
                .any(|needle| name.contains(needle))
        }),
        ("Encoders", |entry| {
            matches!(
                entry.definition.kind,
                boardstudio_core::model::PartKind::Encoder
            ) || entry.definition.name.to_lowercase().contains("encoder")
        }),
    ];
    groups
        .into_iter()
        .map(|(label, matches)| {
            (
                label,
                entries
                    .iter()
                    .copied()
                    .filter(|entry| matches(entry))
                    .collect(),
            )
        })
        .collect()
}

#[cfg(test)]
mod add_object_menu_tests {
    use super::*;
    use boardstudio_application::SnapshotToken;
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
        definition.generator = Some(boardstudio_core::model::PartGenerator {
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
        let runtime = crate::runtime::project_name_test_support::new_runtime();
        crate::runtime::project_name_test_support::open_document(
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        wait_until("Parts browse chooser mounts", || {
            root.query_selector("input[aria-label='Search parts']")
                .unwrap()
                .is_some()
        })
        .await;

        let input = root
            .query_selector("input[aria-label='Search parts']")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlInputElement>()
            .unwrap();
        input.set_value("battery");
        let input_event = web_sys::EventInit::new();
        input_event.set_bubbles(true);
        input
            .dispatch_event(
                &web_sys::Event::new_with_event_init_dict("input", &input_event).unwrap(),
            )
            .unwrap();
        let signals = signals.borrow().as_ref().copied().unwrap();
        wait_until("Parts search draft updates", || {
            (signals.query)() == "battery"
        })
        .await;
        root.query_selector("button.m1-add-browse")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::HtmlElement>()
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
        dioxus_web::launch::launch_virtual_dom(
            dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );

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
            root.query_selector("p[role='alert']")
                .unwrap()
                .and_then(|alert| alert.text_content())
                .is_some_and(|text| text.contains("controlled catalogue source failure"))
        })
        .await;
        let alert = root.query_selector("p[role='alert']").unwrap().unwrap();
        assert!(alert.text_content().unwrap().contains(
            "Component catalogue could not be loaded: controlled catalogue source failure"
        ));
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
}

/// Place inside the existing Objects panel when Parts is the active workspace.
#[component]
pub fn PartsLibraryPanel(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    scope_generation: Signal<u64>,
    workspace: Signal<&'static str>,
    mut query: PartsQuery,
    mut selected: PartsSelection,
    on_select: EventHandler<()>,
) -> Element {
    let mut view_generation = use_signal(|| 0_u64);
    let mut generation = use_context::<PartsSelectionGeneration>().0;
    let mut preview_activation = use_context::<PartsPreviewActivation>().0;
    let mut assembly_selection = use_context::<PartsAssemblySelection>().0;
    let mut assembly_orientation = use_context::<PartsAssemblyOrientation>().0;
    let mut selection_generation = use_context::<PartsSelectionGeneration>().0;
    let catalogue = use_catalogue(&snapshot, &scope);
    let content = if let Some(entries) = catalogue.entries {
        let choices = group_choices(&entries);
        let listed_entries = catalogue_choices(&entries);
        let search = query().trim().to_lowercase();
        let groups = choices
            .iter()
            .map(|group| {
                let items = group
                    .entries
                    .iter()
                    .copied()
                    .filter(|entry| entry.matches(&search, group.label))
                    .collect::<Vec<_>>();
                (group, items)
            })
            .filter(|(_, items)| !items.is_empty())
            .collect::<Vec<_>>();
        let result_count = groups.iter().map(|(_, items)| items.len()).sum::<usize>();
        let selected_id = selected_definition_id(&listed_entries, &search, selected(), &scope);
        let visible_assemblies = matching_assembly_presets(&search);
        let matching_assembly = !visible_assemblies.is_empty();
        let no_matches = groups.is_empty() && !matching_assembly;

        rsx! {
            label { class: "m1-parts-search-label", "Search parts"
                input {
                    class: "m1-parts-search",
                    type: "search",
                    "aria-label": "Search footprints",
                    placeholder: "Name or category",
                    value: "{query()}",
                    oninput: move |event: FormEvent| {
                        query.set(event.value());
                        view_generation.set(view_generation() + 1);
                        generation.with_mut(|value| *value = value.wrapping_add(1));
                    },
                }
            }
            BoardDisclosure { class: "m1-parts-catalogue-scroll m1-parts-assembly-list", open: true,
                heading: rsx! { summary { span { "Key assemblies" small { "{visible_assemblies.len()}" } } AssemblyCreateButton {} } },
                if assembly_selection().is_some() {
                    label { class: "m1-parts-search-label", "Switch orientation"
                        select {
                            "aria-label": "Switch orientation",
                            value: if assembly_orientation() == assembly_presets::SwitchOrientation::North { "north" } else { "south" },
                            onchange: move |event| {
                                assembly_orientation.set(if event.value() == "north" { assembly_presets::SwitchOrientation::North } else { assembly_presets::SwitchOrientation::South });
                                selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                            },
                            option { value: "south", "South-facing LED" }
                            option { value: "north", "North-facing LED" }
                        }
                        small { "Viewed from the keycap side; rotates the switch and attached parts together." }
                    }
                }
                div { role: "listbox", "aria-label": "Key assemblies",
                    for preset in visible_assemblies {
                        { let is_selected = assembly_selection() == Some(preset.id);
                          let preset_id = preset.id;
                          let definition_id = preset.definition_id.to_owned();
                          let scope = scope.clone();
                          rsx! {
                            button {
                                key: "{preset.name}",
                                class: "m1-parts-catalogue-choice",
                                type: "button",
                                role: "option",
                                "aria-selected": "{is_selected}",
                                onclick: move |_| {
                                    assembly_selection.set(Some(preset_id));
                                    selected.set(Some((scope.clone(), definition_id.clone())));
                                    view_generation.set(view_generation() + 1);
                                    generation.with_mut(|value| *value = value.wrapping_add(1));
                                    preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                                    on_select.call(());
                                },
                                "{preset.name}"
                            }
                          }
                        }
                    }
                }
            }
            if entries.is_empty() {
                p { class: "m1-parts-empty", role: "status", "No component definitions are available." }
            } else if !groups.is_empty() {
                BoardDisclosure { class: "m1-parts-catalogue-scroll", open: true,
                    heading: rsx! { summary { "Components" small { "{result_count}" } } },
                    div { role: "listbox", "aria-label": "Footprint library",
                        for (group, items) in groups {
                            BoardDisclosure { key: "{group.label}", class: "m1-parts-category", "aria-label": "{group.label}",
                                open: !query().trim().is_empty() || items.iter().any(|entry| selected_id.as_deref() == Some(entry.definition.id.as_str())),
                                heading: rsx! { summary { "{group.label}" } },
                                for entry in items {
                                    { let is_selected = assembly_selection().is_none() && selected_id.as_deref() == Some(entry.definition.id.as_str());
                                      let id = entry.definition.id.clone();
                                      let scope = scope.clone();
                                      rsx! {
                                        button {
                                            key: "{entry.definition.id}",
                                            class: "m1-parts-catalogue-choice",
                                            type: "button",
                                            role: "option",
                                            "aria-selected": "{is_selected}",
                                            title: entry.definition.generator.as_ref().map(|generator| generator.source.as_str()).unwrap_or(""),
                                            onclick: move |_| {
                                                assembly_selection.set(None);
                                                selected.set(Some((scope.clone(), id.clone())));
                                                view_generation.set(view_generation() + 1);
                                                generation.with_mut(|value| *value = value.wrapping_add(1));
                                                preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                                                on_select.call(());
                                            },
                                            "{preferred_label(&entry.definition)}"
                                        }
                                      }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            if no_matches && !search.is_empty() {
                p { class: "m1-parts-empty", role: "status", "No parts match this search." }
            }
        }
    } else if let Some(error) = catalogue.error {
        rsx! {
            p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
        }
    } else {
        rsx! {
            p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" }
        }
    };

    rsx! {
        section { class: "m1-parts-library", "aria-label": "Parts library",
            crate::parts_view_generation::PartsViewGenerationOwner {
                workspace,
                view_generation,
            }
            h2 { "Parts library" }
            {content}
            div { class: "m1-parts-actions",
                crate::presentation::parts_import_footprint::ImportKiCadFootprintAction {
                    scope: scope.clone(),
                    view_generation,
                    scope_generation,
                    workspace,
                    selected,
                    query,
                    on_select,
                }
                crate::parts_new_component::NewCustomComponentAction {
                    scope: scope.clone(),
                    view_generation,
                    scope_generation,
                    workspace,
                    query,
                    selected,
                    on_select,
                }
            }
        }
    }
}

/// Focused setup picker; uses the same catalogue and source-backed preview as Parts.
#[component]
pub fn ControllerSetupPicker(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut selected: PartsSelection,
    on_place: EventHandler<String>,
    busy: bool,
    enabled: bool,
    error: Option<String>,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let mut generation = use_context::<PartsSelectionGeneration>().0;
    let mut activation = use_context::<PartsPreviewActivation>().0;
    let mut assembly = use_context::<PartsAssemblySelection>().0;
    let mut search = use_signal(String::new);
    let loaded = catalogue.entries.is_some();
    let entries = catalogue.entries.unwrap_or_default();
    let controllers = catalogue_choices(&entries)
        .into_iter()
        .filter(|entry| entry.definition.kind == boardstudio_core::model::PartKind::Controller)
        .collect::<Vec<_>>();
    let selection = selected().filter(|(owner, _)| owner == &scope);
    let needle = search().trim().to_lowercase();
    let matches = controllers
        .iter()
        .copied()
        .filter(|entry| {
            preferred_label(&entry.definition)
                .to_lowercase()
                .contains(&needle)
        })
        .collect::<Vec<_>>();
    let chosen = matches
        .iter()
        .copied()
        .find(|entry| {
            selection
                .as_ref()
                .is_some_and(|(_, id)| id == &entry.definition.id)
        })
        .or_else(|| matches.first().copied());
    let chosen_id = chosen.map(|entry| entry.definition.id.clone());
    let position = matches
        .iter()
        .position(|entry| Some(&entry.definition.id) == chosen_id.as_ref())
        .unwrap_or(0);
    let previous = (!matches.is_empty()).then(|| {
        matches[(position + matches.len() - 1) % matches.len()]
            .definition
            .id
            .clone()
    });
    let next = (!matches.is_empty()).then(|| {
        matches[(position + 1) % matches.len()]
            .definition
            .id
            .clone()
    });
    let owner = scope.clone();
    let choose = EventHandler::new(move |id: String| {
        selected.set(Some((owner.clone(), id)));
        assembly.set(None);
        generation.with_mut(|value| *value = value.wrapping_add(1));
        activation.with_mut(|value| *value = value.wrapping_add(1));
    });
    use_effect(use_reactive((&loaded, &needle, &chosen_id), move |_| {
        let _ = document::eval(
            r#"
            const rail = document.querySelector('.m1-controller-choices');
            const card = rail?.querySelector('[aria-pressed="true"]');
            if (rail && card) rail.scrollTo({left: card.offsetLeft - rail.offsetLeft - (rail.clientWidth - card.offsetWidth) / 2,
                behavior: matchMedia('(prefers-reduced-motion: reduce)').matches ? 'instant' : 'smooth'});
        "#,
        );
    }));
    rsx! {
        section { class: "m1-controller-picker", aria_label: "Controller selection", aria_roledescription: "carousel",
            input { r#type: "search", aria_label: "Search controllers", placeholder: "Find a controller…", value: "{search()}", oninput: move |event| search.set(event.value()) }
            div { class: "m1-controller-carousel-navigation",
                span { role: "status", aria_live: "polite", if matches.is_empty() { "0 controllers" } else { "{position + 1} of {matches.len()}" } }
                button { r#type: "button", aria_label: "Previous controller", title: "Previous controller", disabled: matches.len() < 2,
                    onclick: move |_| { if let Some(id) = previous.clone() { choose.call(id); } },
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m12 5-5 5 5 5" } }
                }
                button { r#type: "button", aria_label: "Next controller", title: "Next controller", disabled: matches.len() < 2,
                    onclick: move |_| { if let Some(id) = next.clone() { choose.call(id); } },
                    svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: "m8 5 5 5-5 5" } }
                }
            }
            div { class: "m1-controller-choices", role: "group", aria_label: "Controllers",
                for entry in &controllers {
                    {
                        let id = entry.definition.id.clone();
                        let active = chosen_id.as_ref() == Some(&id);
                        let matching = preferred_label(&entry.definition).to_lowercase().contains(&needle);
                        rsx! { button {
                            key: "{id}", class: "m1-controller-card", r#type: "button", aria_pressed: active,
                            "data-match": matching, aria_hidden: (!matching).then_some("true"), disabled: !matching,
                            onclick: move |_| choose.call(id.clone()),
                            preview::ControllerFootprintThumbnail { definition: entry.definition.clone() }
                            strong { "{preferred_label(&entry.definition)}" }
                        } }
                    }
                }
            }
            if let Some(entry) = chosen {
                {
                    let id = entry.definition.id.clone();
                    rsx! { BoardButton {
                        variant: boardstudio_web_ui_shared::controls::BoardButtonVariant::Primary,
                        disabled: busy || !enabled, onclick: move |_| { choose.call(id.clone()); on_place.call(id.clone()); },
                        if busy { "Preparing controller…" } else { "Place controller" }
                    } }
                }
            } else {
                p { class: "m1-controller-hint", role: "status",
                    if !loaded { "{catalogue.error.as_deref().unwrap_or(\"Loading controllers…\")}" }
                    else { "No controllers match this search." }
                }
            }
            if let Some(error) = error { p { role: "alert", "{error}" } }
        }
    }
}

/// Place inside the existing Inspector panel when Parts is active.
#[component]
pub fn PartsInspectorPanel(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    query: PartsQuery,
    selected: PartsSelection,
    selected_context: Signal<Option<super::objects::ScopedTreeContext>>,
    on_place_controller: EventHandler<String>,
    on_place_component: EventHandler<super::part_placement::ComponentPlacementAction>,
    controller_placement_enabled: bool,
    placement_busy: bool,
    placement_error: Option<String>,
    mut layout_target: Signal<Option<String>>,
    on_place_assembly: EventHandler<super::objects::MatrixPlacementSource>,
    on_board_placed: EventHandler<()>,
    on_open_module_placement: EventHandler<String>,
    on_module_attached: EventHandler<AttachedModuleNavigation>,
) -> Element {
    let active_assembly = use_context::<PartsAssemblySelection>().0();
    let catalogue = use_catalogue(&snapshot, &scope);
    let selected_module_id = modules_catalogue::selected_id(selected(), &scope);
    let selected_module_selection = selected().map(|(_, id)| id);
    let selected_module_placement = selected_context()
        .as_ref()
        .filter(|tree| scope.as_ref() == Some(&tree.scope))
        .and_then(|tree| match &tree.context {
            super::objects::TreeContext::MountedModule {
                board_id,
                module_id,
            } if scope
                .as_ref()
                .is_some_and(|scope| scope.board_id == *board_id) =>
            {
                snapshot
                    .document
                    .modules
                    .iter()
                    .find(|module| module.id == *module_id)
                    .filter(|module| {
                        Some(format!("module:{}", module.definition_id))
                            == selected_module_selection
                    })
                    .map(|module| module.id.clone())
            }
            _ => None,
        });
    let module_catalogue = modules_catalogue::use_catalogue(
        selected_module_id.is_some(),
        snapshot.token,
        snapshot.document.module_definitions.clone(),
    );
    let project_entry =
        crate::parts_new_component::accepted_project_definition(&snapshot, &scope, &selected())
            .map(|definition| CatalogEntry {
                definition: Rc::new(definition),
                source: catalogue::CatalogueSource::Project,
            });
    let entry = catalogue
        .entries
        .as_ref()
        .and_then(|entries| {
            let listed_entries = catalogue_choices(entries);
            let search = query().trim().to_lowercase();
            let selected_id = selected_definition_id(&listed_entries, &search, selected(), &scope);
            selected_id.as_deref().and_then(|id| {
                listed_entries
                    .iter()
                    .copied()
                    .find(|entry| entry.definition.id == id)
                    .cloned()
            })
        })
        .or_else(|| {
            catalogue
                .entries
                .is_none()
                .then_some(project_entry)
                .flatten()
        });
    let editable_definition = entry
        .as_ref()
        .filter(|entry| {
            entry.source == catalogue::CatalogueSource::Project
                && entry.definition.generator.is_none()
        })
        .map(|entry| (*entry.definition).clone());
    let controller_id = entry
        .as_ref()
        .filter(|entry| {
            matches!(
                entry.definition.kind,
                boardstudio_core::model::PartKind::Controller
            )
        })
        .map(|entry| entry.definition.id.clone());
    let board_id = scope.as_ref().map(|scope| scope.board_id.as_str());
    let layouts = snapshot
        .document
        .layouts
        .iter()
        .filter(|layout| Some(layout.board_id.as_str()) == board_id)
        .collect::<Vec<_>>();
    let selected_layout = layout_target()
        .filter(|id| layouts.iter().any(|layout| layout.id == *id))
        .unwrap_or_default();
    let apply_to_key = selected_context().as_ref().is_some_and(|selected| {
        scope.as_ref() == Some(&selected.scope)
            && matches!(selected.context, super::objects::TreeContext::Key { .. })
    });

    rsx! {
        section { class: "m1-parts-inspector", "aria-label": "Selected component details",
            assembly_editor::SavedAssembliesEditor {
                snapshot: snapshot.clone(),
                scope: scope.clone(),
                definitions: snapshot.document.definitions.iter().cloned().chain(catalogue.entries.as_ref().into_iter().flat_map(|entries| entries.iter()).map(|entry| (*entry.definition).clone())).collect(),
                preset_definitions: catalogue.entries.as_ref().into_iter().flat_map(|entries| entries.iter()).map(|entry| (*entry.definition).clone()).collect(),
                selected_context,
                on_place: on_place_assembly,
                on_board_placed,
            }
            if let Some(module_id) = selected_module_id.as_deref() {
                if let Some(module) = module_catalogue.entries.as_deref().and_then(|entries| entries.iter().find(|entry| entry.definition.id == module_id)) {
                    { let variants = module_catalogue.entries.as_deref().map(|entries| modules_catalogue::variants(entries, &module.row)).unwrap_or_default();
                      rsx! { modules_catalogue::ModuleInspector { snapshot: snapshot.clone(), module: module.clone(), variants, scope: scope.clone(), selected, placement_id: selected_module_placement.clone(), on_open_placement: on_open_module_placement, on_attached: on_module_attached } }
                    }
                } else if let Some(error) = module_catalogue.error.as_ref() {
                    p { class: "m1-parts-load-error", role: "alert", "Module sources could not be loaded: {error}" }
                } else {
                    p { class: "m1-parts-loading", role: "status", "Loading module sources…" }
                }
            } else {
            if let Some(error) = catalogue.error.as_ref() {
                p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
            } else if catalogue.entries.is_none() {
                p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" }
            }
            SelectedDefinition { entry: entry.clone() }
            if let Some(preset) = active_assembly {
                button {
                    class: "m1-parts-place-assembly",
                    r#type: "button",
                    disabled: placement_busy,
                    onclick: move |_| on_place_assembly.call(super::objects::MatrixPlacementSource::Preset(preset)),
                    "Place key assembly"
                }
            }
            if let Some(definition) = editable_definition {
                crate::parts_definition_name::DefinitionNameEditor {
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                    selection: selected,
                    definition: definition.clone(),
                    crate::parts_custom_definition::CustomDefinitionFields {
                        snapshot: snapshot.clone(),
                        scope: scope.clone(),
                        selection: selected,
                        definition,
                    }
                }
            }
            if let Some(entry) = entry.as_ref().filter(|entry| entry.definition.generator.is_some()) {
                generator_settings::GeneratorSettingsEditor {
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                    selected,
                    definition: (*entry.definition).clone(),
                }
            }
            if active_assembly.is_none()
                && let Some(entry) = entry.as_ref().filter(|entry| entry.definition.generator.is_none())
            {
                component_model_editor::ComponentModelEditor {
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                    selected,
                    definition: (*entry.definition).clone(),
                    project_owned: entry.source == catalogue::CatalogueSource::Project,
                }
            }
            if !controller_placement_enabled && let Some(entry) = entry.as_ref() {
                PartsInspectorPlacementAction {
                    entry: entry.clone(),
                    apply_to_key,
                    busy: placement_busy,
                    on_place: on_place_component,
                }
            }
            if controller_placement_enabled {
                if !layouts.is_empty() {
                    label { class: "m1-parts-placement-layout",
                        "Place in"
                        select {
                            aria_label: "Part placement layout",
                            value: "{selected_layout}",
                            onchange: move |event| {
                                layout_target.set((!event.value().is_empty()).then(|| event.value()));
                            },
                            option { value: "", "Board / ungrouped" }
                            for layout in layouts {
                                option { value: "{layout.id}", "{layout.name}" }
                            }
                        }
                    }
                }
                if let Some(definition_id) = controller_id {
                    button {
                        class: "m1-parts-place-controller",
                        r#type: "button",
                        disabled: placement_busy,
                        onclick: move |_| on_place_controller.call(definition_id.clone()),
                        if placement_busy { "Preparing controller…" } else { "Place component" }
                    }
                }
            }
            }
        }
        if let Some(message) = placement_error {
            p { class: "m1-parts-placement-error", role: "alert", "{message}" }
        }
    }
}

#[component]
fn PartsInspectorPlacementAction(
    entry: CatalogEntry,
    apply_to_key: bool,
    busy: bool,
    on_place: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let definition_id = entry.definition.id.clone();
    let kind = entry.definition.kind.clone();
    let can_apply_to_key = matrix_input_available(&entry.definition);
    rsx! {
        button {
            class: "m1-parts-place-component",
            r#type: "button",
            disabled: busy || (apply_to_key && !can_apply_to_key),
            onclick: move |_| on_place.call(
                super::part_placement::ComponentPlacementAction::PartsInspector {
                    definition_id: definition_id.clone(),
                    kind: kind.clone(),
                }
            ),
            if apply_to_key { "Apply to selected key" } else { "Place component" }
        }
        if apply_to_key && !can_apply_to_key {
            p { class: "m1-parts-empty", "This footprint needs an independent press contact pair to replace a matrix key. Clear the key selection to place it as a standalone component." }
        }
    }
}

/// Mount the selected accepted catalogue definition in the workspace canvas.
/// Selection and catalogue lookup stay with the Parts owner; the preview only
/// receives the immutable definition and its accepted source identity.
#[component]
pub fn PartsPreviewWorkspace(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    query: PartsQuery,
    selected: PartsSelection,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let store = use_context::<GeneratorDraftStore>().0;
    let selection_generation = use_context::<PartsSelectionGeneration>().0;
    let scope_generation = use_context::<super::SelectionAdapter>().generation;
    let workspace = use_context::<super::WorkspaceState>().0;
    let active_assembly = use_context::<PartsAssemblySelection>().0();
    let assembly_orientation = use_context::<PartsAssemblyOrientation>().0();
    let catalogue = use_catalogue(&snapshot, &scope);
    let selected_module_id = modules_catalogue::selected_id(selected(), &scope);
    let module_catalogue = modules_catalogue::use_catalogue(
        selected_module_id.is_some(),
        snapshot.token,
        snapshot.document.module_definitions.clone(),
    );
    let search = query().trim().to_lowercase();
    let listed_entries = catalogue
        .entries
        .as_deref()
        .map(|entries| catalogue_choices(entries))
        .unwrap_or_default();
    let selected_id = selected_definition_id(&listed_entries, &search, selected(), &scope);
    let generator_draft = store().filter(|draft| {
        generator_settings::owner_is_current(
            &draft.owner,
            &runtime,
            selected,
            scope_generation(),
            selection_generation(),
            workspace(),
        )
    });
    let preview_definition = generator_draft
        .as_ref()
        .filter(|draft| draft.status == generator_settings::GeneratorPreviewStatus::Ready)
        .and_then(|draft| draft.definition.as_ref())
        .cloned();
    let request = AssemblyRecipeRequest {
        scope: scope.clone(),
        snapshot_token: snapshot.token,
        selection: selected(),
        selection_generation: selection_generation(),
        scope_generation: scope_generation(),
        preset: active_assembly,
        orientation: assembly_orientation,
        reversible: reversible_layout(&snapshot.document),
        entries: catalogue.entries.clone(),
        preview_definition: preview_definition.clone(),
    };
    let recipe_resource = use_resource(use_reactive(&request, |request| async move {
        let result = match (request.preset, request.entries.as_deref()) {
            (Some(preset), Some(entries)) => assembly_presets::resolve(
                preset,
                entries,
                request.reversible,
                request.orientation,
                request.preview_definition.clone(),
            )
            .await
            .map(Some),
            (Some(_), None) => Ok(None),
            (None, _) => Ok(Some(Vec::new())),
        };
        (request, result)
    }));
    let recipe_state = recipe_resource.read().clone();
    let matching_recipe = recipe_state.filter(|(owner, _)| owner == &request);
    let recipe_error = matching_recipe
        .as_ref()
        .and_then(|(_, result)| result.as_ref().err().cloned());
    let recipe = matching_recipe
        .as_ref()
        .and_then(|(_, result)| result.as_ref().ok().cloned().flatten())
        .unwrap_or_default();
    let recipe_pending = active_assembly.is_some() && matching_recipe.is_none();
    let recipe_identity = request.identity();
    let preview_title = active_assembly.map(|preset| assembly_presets::name(preset).to_owned());
    if let Some(module_id) = selected_module_id.as_deref() {
        if let Some(module) = module_catalogue.entries.as_deref().and_then(|entries| {
            entries
                .iter()
                .find(|entry| entry.definition.id == module_id)
        }) {
            return rsx! { modules_catalogue::ModuleSourcePreview { module: module.clone() } };
        }
        if let Some(error) = module_catalogue.error {
            return rsx! {
                section { class: "m1-workspace-content m1-parts-preview", "aria-label": "Module source preview",
                    p { class: "m1-parts-preview-error", role: "alert", "Module source preview could not be loaded: {error}" }
                }
            };
        }
        return rsx! {
            section { class: "m1-workspace-content m1-parts-preview", "aria-label": "Module source preview",
                p { class: "m1-parts-preview-status", role: "status", "Loading module source preview…" }
            }
        };
    }
    let Some(entries) = catalogue.entries else {
        return if let Some(error) = catalogue.error {
            rsx! {
                p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
            }
        } else {
            rsx! {
                p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" }
            }
        };
    };
    let listed_entries = catalogue_choices(&entries);
    let entry = selected_id.as_deref().and_then(|id| {
        listed_entries
            .iter()
            .copied()
            .find(|entry| entry.definition.id == id)
            .cloned()
    });
    let Some(entry) = entry else {
        return rsx! {
            PartsPreviewPanel {
                definition: None,
                recipe,
                recipe_error,
                recipe_pending,
                recipe_identity,
                preview_title,
                scope,
                snapshot_token: snapshot.token,
                generator_draft: None,
            }
        };
    };
    let source = match entry.source {
        catalogue::CatalogueSource::Project => ProfileDefinitionSource::Project,
        catalogue::CatalogueSource::Generator => ProfileDefinitionSource::Generator,
        catalogue::CatalogueSource::Imported => ProfileDefinitionSource::Imported,
    };

    rsx! {
        PartsMechanicalProfileWorkspace {
            snapshot,
            scope,
            selection: selected,
            definition: (*entry.definition).clone(),
            preview_definition,
            generator_draft,
            recipe,
            recipe_error,
            recipe_pending,
            recipe_identity,
            preview_title,
            source,
        }
    }
}

fn use_catalogue(snapshot: &AcceptedSnapshot, scope: &Option<Scope>) -> CatalogueView {
    let document = snapshot.document.clone();
    let reversible_layout = reversible_layout(&document);
    #[cfg(test)]
    let test_loader = try_consume_context::<CatalogueLoadTestPort>();
    let catalogue = use_resource(use_reactive((scope, &reversible_layout), {
        move |(scope, reversible_layout)| {
            let scope = scope.clone();
            #[cfg(test)]
            let test_loader = test_loader.clone();
            async move {
                #[cfg(test)]
                let loaded = if let Some(test_loader) = test_loader {
                    (test_loader.0)(reversible_layout).await
                } else {
                    catalogue::load_bundled(reversible_layout).await
                };
                #[cfg(not(test))]
                let loaded = catalogue::load_bundled(reversible_layout).await;
                match loaded {
                    Ok(entries) => Ok((scope, reversible_layout, entries)),
                    Err(error) => Err((scope, reversible_layout, error)),
                }
            }
        }
    }));
    let catalogue_state = catalogue.read().clone();
    let (source_entries, error) = match catalogue_state {
        Some(Ok((loaded_scope, loaded_reversible, entries)))
            if loaded_scope == *scope && loaded_reversible == reversible_layout =>
        {
            (Some(entries), None)
        }
        Some(Err((loaded_scope, loaded_reversible, error)))
            if loaded_scope == *scope && loaded_reversible == reversible_layout =>
        {
            (None, Some(error))
        }
        _ => (None, None),
    };
    let bundle_identity = source_entries
        .as_ref()
        .map_or(0, |entries| Rc::as_ptr(entries) as usize);
    let accepted_token = snapshot.token;
    let accepted_scope = scope.clone();
    let merged = use_memo(use_reactive((&accepted_token, &bundle_identity), {
        let document = document.clone();
        let source_entries = source_entries.clone();
        move |_| {
            source_entries.as_ref().map(|source_entries| {
                cached_project_catalogue(
                    &accepted_scope,
                    accepted_token,
                    document.clone(),
                    source_entries.clone(),
                )
            })
        }
    }));
    CatalogueView {
        entries: merged.read().clone(),
        error,
    }
}

fn cached_project_catalogue(
    scope: &Option<Scope>,
    token: SnapshotToken,
    document: Arc<ProjectDoc>,
    bundled: Rc<Vec<CatalogEntry>>,
) -> Rc<Vec<CatalogEntry>> {
    if let Some(entries) = PROJECT_CATALOGUE_CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|cached| {
                cached.scope == *scope
                    && cached.token == token
                    && Arc::ptr_eq(&cached.document, &document)
                    && Rc::ptr_eq(&cached.bundled, &bundled)
            })
            .map(|cached| cached.entries.clone())
    }) {
        return entries;
    }

    let entries = Rc::new(catalogue::merge_project_overrides(
        &bundled,
        &document.definitions,
    ));
    PROJECT_CATALOGUE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() == 4 {
            cache.remove(0);
        }
        cache.push(CachedProjectCatalogue {
            scope: scope.clone(),
            token,
            document,
            bundled,
            entries: entries.clone(),
        });
    });
    entries
}

fn selected_definition_id(
    entries: &[&CatalogEntry],
    search: &str,
    selected: Option<(Option<Scope>, String)>,
    scope: &Option<Scope>,
) -> Option<String> {
    selected
        .filter(|(selected_scope, id)| {
            selected_scope == scope && entries.iter().any(|entry| entry.definition.id == *id)
        })
        .map(|(_, id)| id)
        .or_else(|| {
            if search.is_empty() {
                entries
                    .iter()
                    .find(|entry| entry.definition.id == "generator:ceoloide/switch_mx")
            } else {
                None
            }
            .or_else(|| {
                entries
                    .iter()
                    .find(|entry| entry.matches_library_search(search))
            })
            .or_else(|| entries.first())
            .map(|entry| entry.definition.id.clone())
        })
}
