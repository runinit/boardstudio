//! Read-only Parts catalogue and selected-definition presentation slots.
mod catalogue;
mod details;
#[cfg(any(target_arch = "wasm32", test))]
mod mechanical_profile_editor;
#[cfg(target_arch = "wasm32")]
mod mechanical_profile_ui;
#[cfg(all(test, target_arch = "wasm32"))]
mod physical_setup;
mod preview;
mod standard_profile_lifetime;

#[cfg(any(target_arch = "wasm32", test))]
use mechanical_profile_editor::{
    AcceptedProfileOwner, CurrentProfileScope, DetachedProfileSpawner, ManualProfileEditor,
    ManualProfileEditorPorts, StandardProfileFuture, StandardProfileRequester,
};
pub(in crate::presentation) use preview::PartsPreviewPanel;
pub(super) use standard_profile_lifetime::PartsStandardProfileLifetime;

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
pub(super) type PartsQuery = Signal<String>;
pub(super) type PartsSelection = Signal<Option<(Option<Scope>, String)>>;

#[derive(Clone, Copy)]
pub(super) struct PartsSelectionGeneration(pub(super) Signal<u64>);
/// React's library predicate is advisory; Core remains authoritative when SetMatrix is accepted.
pub(super) fn matrix_input_available(definition: &boardstudio_core::model::PartDefinition) -> bool {
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
pub(super) async fn load_mounting_hole_definition()
-> Result<Rc<boardstudio_core::model::PartDefinition>, String> {
    let entries = catalogue::load_bundled(false).await?;
    let mut matches = entries.iter().filter(|entry| {
        entry.source == catalogue::CatalogueSource::Ergogen
            && entry
                .definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
    });
    let definition = matches
        .next()
        .ok_or_else(|| "Bundled mounting-hole template is unavailable.".to_string())?
        .definition
        .clone();
    if matches.next().is_some() {
        return Err("Bundled mounting-hole template is ambiguous.".into());
    }
    Ok(definition)
}

/// Narrow Parts-owned bridge for the Editor's physical-setup intent owner. Package loading and
/// metadata normalization stay private to the catalogue implementation.
pub(super) async fn prepare_physical_setup_proposal(
    accepted: ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
) -> Result<ProjectDoc, String> {
    catalogue::prepare_physical_setup_proposal_from_package(&accepted, intent).await
}

/// Normalize a matrix-owned clone through the same packaged Ergogen path used by Parts.
pub(super) async fn normalize_matrix_definition(
    definition: boardstudio_core::model::PartDefinition,
) -> Result<boardstudio_core::model::PartDefinition, String> {
    catalogue::normalize_matrix_definition(definition).await
}

pub(super) async fn is_ergogen_source(source: String) -> Result<bool, String> {
    catalogue::is_ergogen_source(source).await
}

/// Read the parameter descriptors from the packaged Ergogen module. The Parts catalogue remains
/// the sole owner of module loading; PCB receives only the accepted package schema values.
pub(super) async fn ergogen_parameter_schema(
    source: String,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, String> {
    catalogue::ergogen_parameter_schema(&source).await
}

pub(super) async fn load_matrix_templates(
    reversible: bool,
) -> Result<Vec<boardstudio_core::model::PartDefinition>, String> {
    let entries = catalogue::load_bundled(reversible).await?;
    Ok(entries
        .iter()
        .filter(|entry| entry.source == catalogue::CatalogueSource::Ergogen)
        .map(|entry| (*entry.definition).clone())
        .collect())
}

/// Resolve any selectable item from the construction-normalized bundled catalogue, then apply
/// the accepted project definition with the same precedence as the Parts browser.
pub(super) async fn load_component_definition(
    document: &ProjectDoc,
    definition_id: &str,
) -> Result<boardstudio_core::model::PartDefinition, String> {
    let reversible = reversible_layout(document);
    let bundled = catalogue::load_bundled(reversible).await?;
    let entries = catalogue::merge_project_overrides(&bundled, &document.definitions);
    let definition = entries
        .iter()
        .find(|entry| entry.definition.id == definition_id)
        .ok_or_else(|| "The selected component is no longer in the catalogue.".to_string())?;
    Ok((*definition.definition).clone())
}

#[component]
pub(super) fn AddObjectComponentChooser(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    on_place: EventHandler<super::part_placement::ComponentPlacementAction>,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let mut query = use_signal(String::new);
    let content = if let Some(entries) = catalogue.entries {
        let search = query().trim().to_lowercase();
        let groups = group_choices(&entries)
            .iter()
            .map(|group| {
                let items = group
                    .entries
                    .iter()
                    .copied()
                    .filter(|entry| entry.matches(&search, group.label))
                    .collect::<Vec<_>>();
                (group.label, items)
            })
            .filter(|(_, items)| !items.is_empty())
            .collect::<Vec<_>>();
        if entries.is_empty() {
            rsx! { p { class: "m1-parts-empty", role: "status", "No component definitions are available." } }
        } else if groups.is_empty() {
            rsx! { p { class: "m1-parts-empty", role: "status", "No parts match this search." } }
        } else {
            rsx! {
                label { class: "m1-parts-search-label", "Search parts"
                    input {
                        class: "m1-parts-search",
                        type: "search",
                        "aria-label": "Search parts to place",
                        placeholder: "Name or category",
                        value: "{query()}",
                        oninput: move |event| query.set(event.value()),
                    }
                }
                for (group, items) in groups {
                    section { key: "{group}", class: "m1-parts-category", "aria-label": "{group}",
                        h3 { "{group}" }
                        for entry in items {
                            { let definition_id = entry.definition.id.clone();
                              let kind = entry.definition.kind.clone();
                              let label = preferred_label(&entry.definition);
                              rsx! {
                                button {
                                    key: "{definition_id}",
                                    class: "m1-parts-catalogue-choice",
                                    r#type: "button",
                                    onclick: move |_| on_place.call(
                                        super::part_placement::ComponentPlacementAction::AddObject {
                                            definition_id: definition_id.clone(),
                                            kind: kind.clone(),
                                        }
                                    ),
                                    "{label}"
                                }
                              }
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
    rsx! { div { class: "m1-add-component-chooser", {content} } }
}

/// Place inside the existing Objects panel when Parts is the active workspace.
#[component]
pub(super) fn PartsLibraryPanel(
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
            if entries.is_empty() {
                p { class: "m1-parts-empty", role: "status", "No component definitions are available." }
            } else if groups.is_empty() {
                p { class: "m1-parts-empty", role: "status", "No parts match this search." }
            } else {
                details { class: "m1-parts-catalogue-scroll", open: true,
                    summary { "Components" small { "{result_count}" } }
                    div { role: "listbox", "aria-label": "Footprint library",
                        for (group, items) in groups {
                            section { key: "{group.label}", class: "m1-parts-category", "aria-label": "{group.label}",
                                h3 { "{group.label}" }
                                for entry in items {
                                    { let is_selected = selected_id.as_deref() == Some(entry.definition.id.as_str());
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
                                                selected.set(Some((scope.clone(), id.clone())));
                                                view_generation.set(view_generation() + 1);
                                                generation.with_mut(|value| *value = value.wrapping_add(1));
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

/// Place inside the existing Inspector panel when Parts is active.
#[component]
pub(super) fn PartsInspectorPanel(
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
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
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
            if let Some(error) = catalogue.error.as_ref() {
                p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
            } else if catalogue.entries.is_none() {
                p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" }
            }
            SelectedDefinition { entry: entry.clone() }
            if let Some(definition) = editable_definition {
                crate::parts_definition_name::DefinitionNameEditor {
                    snapshot: snapshot.clone(),
                    scope: scope.clone(),
                    selection: selected,
                    definition,
                }
            }
            if !controller_placement_enabled && let Some(entry) = entry.as_ref() {
                { let definition_id = entry.definition.id.clone();
                  let kind = entry.definition.kind.clone();
                  let can_apply = matches!(kind, boardstudio_core::model::PartKind::Switch)
                      || matrix_input_available(&entry.definition);
                  rsx! {
                    button {
                        class: "m1-parts-place-component",
                        r#type: "button",
                        disabled: placement_busy || (apply_to_key && !can_apply),
                        onclick: move |_| on_place_component.call(
                            super::part_placement::ComponentPlacementAction::PartsInspector {
                                definition_id: definition_id.clone(),
                                kind: kind.clone(),
                            }
                        ),
                        if apply_to_key { "Apply to selected key" } else { "Place component" }
                    }
                  }
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
        if let Some(message) = placement_error {
            p { class: "m1-parts-placement-error", role: "alert", "{message}" }
        }
    }
}

/// Mount the selected accepted catalogue definition in the workspace canvas.
/// Selection and catalogue lookup stay with the Parts owner; the preview only
/// receives the immutable definition and its accepted source identity.
#[component]
pub(super) fn PartsPreviewWorkspace(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    query: PartsQuery,
    selected: PartsSelection,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
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
    let search = query().trim().to_lowercase();
    let selected_id = selected_definition_id(&listed_entries, &search, selected(), &scope);
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
                scope,
                snapshot_token: snapshot.token,
            }
        };
    };
    let source = match entry.source {
        catalogue::CatalogueSource::Project => ProfileDefinitionSource::Project,
        catalogue::CatalogueSource::Ergogen => ProfileDefinitionSource::Ergogen,
        catalogue::CatalogueSource::Imported => ProfileDefinitionSource::Imported,
    };

    rsx! {
        PartsMechanicalProfileWorkspace {
            snapshot,
            scope,
            selection: selected,
            definition: (*entry.definition).clone(),
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
                match catalogue::load_bundled(reversible_layout).await {
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
                    .find(|entry| entry.definition.id == "ergogen:ceoloide/switch_mx")
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

fn reversible_layout(document: &ProjectDoc) -> bool {
    match document.parameters.get("reversibleLayout") {
        Some(serde_json::Value::Bool(reversible)) => *reversible,
        _ => document
            .hardware
            .as_ref()
            .is_some_and(|hardware| hardware.instances.iter().any(|instance| instance.flipped)),
    }
}
