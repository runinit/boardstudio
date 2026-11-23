//! Parts catalogue, selected-definition editing and placement presentation.
mod add_object_logic;
mod assembly_editor;
use add_object_logic::{add_object_choices, add_object_groups, add_object_search_results};
pub use boardstudio_web_catalogue::bundled::{
    generator_parameter_schema, is_generator_source, load_all_catalogue_definitions,
    load_component_definition, load_matrix_templates, normalize_matrix_definition,
    prepare_physical_setup_proposal, prepare_physical_setup_resources, reversible_layout,
};
use boardstudio_web_catalogue::catalogue;
use boardstudio_web_ui_shared::inspector_fields::{SelectField, TOUCH_CLS};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::Callout,
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::Field,
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    item::{Item, ItemGroup, ItemSize},
    label::Label,
    spinner::Spinner,
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
mod component_model_editor;
mod details;
mod details_logic;
mod generator_settings;
mod model_transform;
mod module_profile_editor;
mod modules_catalogue;
mod modules_catalogue_logic;
pub use modules_catalogue::module_attachment::AttachedModuleNavigation;
mod preview;
pub use assembly_editor::MatrixPresetId;

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
    rsx! { Tooltip {
        TooltipTrigger { as: move |attributes| rsx! {
            Button { attributes, variant: ButtonVariant::Ghost, size: ButtonSize::IconSm,
                class: "{TOUCH_CLS} max-[960px]:min-w-control-touch [@media(pointer:coarse)]:min-w-control-touch",
                aria_label: "New assembly", disabled: current.is_none(),
                onclick: move |event: MouseEvent| { event.prevent_default(); event.stop_propagation(); if let Some(action) = current { action.call(event); } },
                svg { class: "fill-none stroke-current stroke-[1.5]", view_box: "0 0 20 20", "aria-hidden": "true", path { d: "M10 4v12M4 10h12" } }
            }
        } }
        TooltipContent { "New key assembly" }
    } }
}

#[component]
fn PartsCatalogueCategory(label: String, reveal: bool, children: Element) -> Element {
    let mut open = use_signal(|| reveal);
    use_effect(use_reactive((&reveal,), move |(reveal,)| open.set(reveal)));
    rsx! {
        Collapsible { class: "py-1", aria_label: label.clone(), keep_mounted: true,
            open: Some(open()), on_open_change: move |next| open.set(next),
            CollapsibleTrigger { class: TOUCH_CLS, "{label}" }
            CollapsibleContent { class: "data-[open=false]:hidden", ItemGroup { {children} } }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixPlacementSource {
    Preset(MatrixPresetId),
    Assembly {
        assembly: boardstudio_model::model::AssemblyDefinition,
        definitions: Vec<boardstudio_model::model::PartDefinition>,
    },
}

pub use assembly_editor::SwitchOrientation;
pub use assembly_editor::matrix_with_assembly;

pub fn matrix_setup_preset(
    preset: assembly_editor::MatrixPresetId,
) -> crate::matrix_setup_operation::MatrixSetupPreset {
    use crate::matrix_setup_operation::MatrixSetupPreset as SetupPreset;
    use assembly_editor::MatrixPresetId as PartsPreset;
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

pub use preview::PartsPreviewPanel;

use crate::parts_mechanical_profile::ProfileDefinitionSource;
#[cfg(target_arch = "wasm32")]
use crate::parts_mechanical_profile::ui::PartsMechanicalProfileWorkspace;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::ProjectDoc;
use catalogue::{CatalogEntry, catalogue_choices, group_choices, preferred_label};
use details::SelectedDefinition;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// Parent-owned interaction state keeps the reference library query and choice
/// alive while the editor switches between workspaces.
pub type PartsQuery = Signal<String>;
pub type PartsSelection = Signal<Option<(Option<Scope>, String)>>;

#[derive(Clone, Copy)]
pub struct PartsAssemblySelection(pub Signal<Option<assembly_editor::MatrixPresetId>>);

#[derive(Clone, Copy)]
pub struct PartsAssemblyOrientation(pub Signal<assembly_editor::SwitchOrientation>);

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
    preset: Option<assembly_editor::MatrixPresetId>,
    orientation: assembly_editor::SwitchOrientation,
    reversible: bool,
    entries: Option<Rc<Vec<CatalogEntry>>>,
    preview_definition: Option<boardstudio_model::model::PartDefinition>,
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
    definition: Option<boardstudio_model::model::PartDefinition>,
    status: generator_settings::GeneratorPreviewStatus,
}

#[derive(Clone, Copy)]
pub struct GeneratorDraftStore(pub Signal<Option<GeneratorPreviewDraft>>);
/// React's library predicate is advisory; Core remains authoritative when SetMatrix is accepted.
/// Single host-compatible implementation lives in the catalogue; this re-export keeps
/// the Parts presentation call sites stable on both targets and in tests.
pub use boardstudio_web_catalogue::matrix_input::matrix_input_available;

#[derive(Clone)]
struct CatalogueView {
    entries: Option<Rc<Vec<CatalogEntry>>>,
    error: Option<String>,
}

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
-> Result<boardstudio_model::model::PartDefinition, String> {
    modules_catalogue::load_horizontal_host_connector_definition().await
}

pub fn placement_label(definition: &boardstudio_model::model::PartDefinition) -> &str {
    preferred_label(definition)
}

pub use boardstudio_web_catalogue::mirror_pair::has_unique_mirror_board_pair;

#[component]
fn AddObjectChoice(
    definition_id: String,
    kind: boardstudio_model::model::PartKind,
    label: String,
    mirror_only_available: bool,
    on_place: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let linked_definition_id = definition_id.clone();
    let linked_kind = kind.clone();
    let mirror_definition_id = definition_id.clone();
    let mirror_kind = kind.clone();
    rsx! {
        Item { size: ItemSize::Xs, class: "p-0",
        div { role: "group", "aria-label": "Add {label}", class: "w-full",
            Button {
                key: "{definition_id}", variant: ButtonVariant::Ghost,
                class: "w-full justify-start whitespace-normal text-left {TOUCH_CLS}",
                aria_label: "Add {label}",
                onclick: move |_| on_place.call(
                    super::part_placement::ComponentPlacementAction::AddObject {
                        definition_id: linked_definition_id.clone(),
                        kind: linked_kind.clone(),
                    }
                ),
                "{label}"
            }
            if mirror_only_available {
                Button { variant: ButtonVariant::Ghost,
                    class: "w-full justify-start whitespace-normal text-left {TOUCH_CLS}",
                    aria_label: "Add {label} to this half only",
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
    let search = query().trim().to_lowercase();
    let content = if let Some(entries) = catalogue.entries {
        let choices = add_object_choices(&entries);
        if choices.is_empty() {
            rsx! { div { role: "status", Callout { "No component definitions are available." } } }
        } else if search.is_empty() {
            let groups = add_object_groups(&choices);
            rsx! {
                for (label, items) in groups {
                    Collapsible { key: "{label}", class: "py-1", default_open: label == "Components", keep_mounted: true,
                        CollapsibleTrigger { class: TOUCH_CLS, "{label}" }
                        CollapsibleContent { class: "data-[open=false]:hidden", ItemGroup { class: "gap-px",
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
                        } }
                    }
                }
            }
        } else {
            let results = add_object_search_results(&entries, &search);
            if results.is_empty() {
                rsx! { div { role: "status", Callout { "No parts match this search." } } }
            } else {
                rsx! {
                    ItemGroup { class: "gap-px",
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
    } else {
        catalogue_status(
            catalogue.error,
            "Component catalogue",
            "Loading component catalogue…",
        )
    };
    rsx! {
        div { class: "grid gap-2 min-w-0",
            {placement_layout_select(&snapshot, &scope, layout_target)}
            Field {
                Label { html_for: format!("parts-add-search-{}", dioxus::core::current_scope_id().0), "Parts" }
                InputGroup { class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch", InputGroupInput { class: "max-[960px]:h-control-touch [@media(pointer:coarse)]:h-control-touch",
                    id: format!("parts-add-search-{}", dioxus::core::current_scope_id().0),
                    r#type: InputType::Search, aria_label: "Search parts", placeholder: "Find a part…", value: query(),
                    oninput: move |event: FormEvent| {
                        query.set(event.value());
                        selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                    },
                } }
            }
            {content}
            Button { variant: ButtonVariant::Secondary,
                class: "mt-1 {TOUCH_CLS}",
                onclick: move |_| on_browse.call(()),
                "Browse all parts"
            }
        }
    }
}

/// Shared "Place in" layout picker; renders nothing without board layouts.
fn placement_layout_select(
    snapshot: &AcceptedSnapshot,
    scope: &Option<Scope>,
    mut layout_target: Signal<Option<String>>,
) -> Element {
    let board_id = scope.as_ref().map(|scope| scope.board_id.as_str());
    let layouts = snapshot
        .document
        .layouts
        .iter()
        .filter(|layout| Some(layout.board_id.as_str()) == board_id)
        .collect::<Vec<_>>();
    if layouts.is_empty() {
        return rsx! {};
    }
    let selected = layout_target()
        .filter(|id| layouts.iter().any(|layout| layout.id == *id))
        .unwrap_or_default();
    rsx! {
        SelectField::<String> {
            label: "Place in", aria_label: "Part placement layout", value: selected,
            options: std::iter::once((String::new(), "Board / ungrouped".into()))
                .chain(layouts.iter().map(|layout| (layout.id.clone(), layout.name.clone()))).collect(),
            on_commit: move |value: String| layout_target.set((!value.is_empty()).then_some(value)),
        }
    }
}

/// Shared catalogue status: the load error alert, otherwise the spinner.
fn catalogue_status(error: Option<String>, context: &str, loading: &str) -> Element {
    if let Some(error) = error {
        rsx! { Alert { variant: AlertVariant::Destructive, "{context} could not be loaded: {error}" } }
    } else {
        rsx! { div { role: "status", Spinner {} "{loading}" } }
    }
}

fn matching_assembly_presets(search: &str) -> Vec<assembly_editor::Preset> {
    let search = search.trim().to_lowercase();
    assembly_editor::PRESETS
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

#[path = "parts/library_panels.rs"]
mod library_panels;
pub use library_panels::{ControllerSetupPicker, PartsLibraryPanel};

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
        selected_module_id.clone(),
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
                boardstudio_model::model::PartKind::Controller
            )
        })
        .map(|entry| entry.definition.id.clone());
    let apply_to_key = selected_context().as_ref().is_some_and(|selected| {
        scope.as_ref() == Some(&selected.scope)
            && matches!(selected.context, super::objects::TreeContext::Key { .. })
    });

    rsx! {
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto p-4 text-foreground", "aria-label": "Selected component details",
            assembly_editor::SavedAssembliesEditor {
                snapshot: snapshot.clone(),
                scope: scope.clone(),
                definitions: snapshot.document.definitions.iter().cloned().chain(catalogue.entries.as_ref().into_iter().flat_map(|entries| entries.iter()).map(|entry| (*entry.definition).clone())).collect(),
                preset_definitions: catalogue.entries.as_ref().into_iter().flat_map(|entries| entries.iter()).map(|entry| (*entry.definition).clone()).collect(),
                selected_context,
                on_place: on_place_assembly,
                on_board_placed,
            }
            Collapsible { class: "mb-3", keep_mounted: false,
                CollapsibleTrigger { class: "w-full justify-start {TOUCH_CLS}", "Modules" }
                CollapsibleContent { modules_catalogue::ModuleBrowser { scope: scope.clone(), selected } }
            }
            if selected_module_id.is_some() {
                if let Some(module) = module_catalogue.selected {
                    modules_catalogue::ModuleInspector { snapshot: snapshot.clone(), module, variants: module_catalogue.variants, scope: scope.clone(), selected, placement_id: selected_module_placement.clone(), on_open_placement: on_open_module_placement, on_attached: on_module_attached }
                } else {
                    {catalogue_status(module_catalogue.error.clone(), "Module sources", "Loading module sources…")}
                }
            } else {
            if catalogue.entries.is_none() {
                {catalogue_status(catalogue.error.clone(), "Component catalogue", "Loading component catalogue…")}
            }
            SelectedDefinition { entry: entry.clone() }
            if let Some(preset) = active_assembly {
                Button {
                    class: "w-full {TOUCH_CLS}",
                    disabled: placement_busy,
                    onclick: move |_| on_place_assembly.call(super::objects::MatrixPlacementSource::Preset(preset)),
                    "Place key assembly"
                }
            }
            if let Some(definition) = editable_definition {
                crate::parts_custom_definition::CustomDefinitionFields {
                    key: "{definition.id}",
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                    selection: selected,
                    definition,
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
                {placement_layout_select(&snapshot, &scope, layout_target)}
                if let Some(definition_id) = controller_id {
                    Button {
                        class: "w-full {TOUCH_CLS}",
                        disabled: placement_busy,
                        onclick: move |_| on_place_controller.call(definition_id.clone()),
                        if placement_busy { "Preparing controller…" } else { "Place component" }
                    }
                }
            }
            }
        }
        if let Some(message) = placement_error {
            Alert { variant: AlertVariant::Destructive, "{message}" }
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
        Button {
            class: "w-full {TOUCH_CLS}",
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
            Callout { "This footprint needs an independent press contact pair to replace a matrix key. Clear the key selection to place it as a standalone component." }
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
        selected_module_id.clone(),
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
            (Some(preset), Some(entries)) => assembly_editor::resolve(
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
    let preview_title = active_assembly.map(|preset| assembly_editor::name(preset).to_owned());
    if selected_module_id.is_some() {
        if let Some(module) = module_catalogue.selected {
            return rsx! { modules_catalogue::ModuleSourcePreview { module } };
        }
        return rsx! {
            section { class: "relative order-2 flex flex-1 flex-col min-w-0 min-h-0 overflow-hidden rounded-xl bg-card shadow-sm", "aria-label": "Module source preview",
                {catalogue_status(module_catalogue.error, "Module source preview", "Loading module source preview…")}
            }
        };
    }
    let Some(entries) = catalogue.entries else {
        return catalogue_status(
            catalogue.error,
            "Component catalogue",
            "Loading component catalogue…",
        );
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
    let catalogue = use_resource(use_reactive((scope, &reversible_layout), {
        move |(scope, reversible_layout)| {
            let scope = scope.clone();
            async move {
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
