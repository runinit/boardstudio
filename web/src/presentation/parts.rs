//! Read-only Parts catalogue and selected-definition presentation slots.
mod catalogue;
mod details;

use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::ProjectDoc;
use catalogue::{CatalogEntry, catalogue_choices, group_choices, preferred_label};
use details::SelectedDefinition;
use dioxus::prelude::*;
use std::{cell::RefCell, rc::Rc, sync::Arc};

/// Parent-owned interaction state keeps the reference library query and choice
/// alive while the editor switches between workspaces.
pub(super) type PartsQuery = Signal<String>;
pub(super) type PartsSelection = Signal<Option<(Option<Scope>, String)>>;

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

/// Place inside the existing Objects panel when Parts is the active workspace.
#[component]
pub(super) fn PartsLibraryPanel(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    mut query: PartsQuery,
    mut selected: PartsSelection,
    on_select: EventHandler<()>,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let Some(entries) = catalogue.entries else {
        return if let Some(error) = catalogue.error {
            rsx! {
                section { class: "m1-parts-library", "aria-label": "Parts library",
                    h2 { "Parts library" }
                    p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
                }
            }
        } else {
            rsx! {
                section { class: "m1-parts-library", "aria-label": "Parts library",
                    h2 { "Parts library" }
                    p { class: "m1-parts-loading", role: "status", "Loading component catalogue…" }
                }
            }
        };
    };

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
        section { class: "m1-parts-library", "aria-label": "Parts library",
            h2 { "Parts library" }
            label { class: "m1-parts-search-label", "Search parts"
                input {
                    class: "m1-parts-search",
                    type: "search",
                    "aria-label": "Search footprints",
                    placeholder: "Name or category",
                    value: "{query()}",
                    oninput: move |event: FormEvent| query.set(event.value()),
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
    }
}

/// Place inside the existing Inspector panel when Parts is active.
#[component]
pub(super) fn PartsInspectorPanel(
    snapshot: AcceptedSnapshot,
    scope: Option<Scope>,
    query: PartsQuery,
    selected: PartsSelection,
) -> Element {
    let catalogue = use_catalogue(&snapshot, &scope);
    let Some(entries) = catalogue.entries else {
        return if let Some(error) = catalogue.error {
            rsx! {
                section { class: "m1-parts-inspector", "aria-label": "Selected component details",
                    p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
                }
            }
        } else {
            rsx! {
                section { class: "m1-parts-inspector", "aria-label": "Selected component details",
                    p { class: "m1-parts-loading", role: "status", "Loading selected component details…" }
                }
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

    rsx! {
        section { class: "m1-parts-inspector", "aria-label": "Selected component details",
            SelectedDefinition { entry }
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
