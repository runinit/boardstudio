//! Read-only Parts catalogue presentation.
mod catalogue;
mod details;

use boardstudio_application::{AcceptedSnapshot, Scope};
use boardstudio_core::model::ProjectDoc;
use catalogue::{CatalogEntry, catalogue_choices, group_choices, preferred_label};
use details::SelectedDefinition;
use dioxus::prelude::*;
use std::rc::Rc;

/// Render the project-aware catalogue while keeping search and selection local
/// to the Parts workspace. All inputs are accepted read models or bundled data.
#[component]
pub(super) fn PartsWorkspace(snapshot: AcceptedSnapshot, scope: Option<Scope>) -> Element {
    let document = snapshot.document.clone();
    let reversible_layout = reversible_layout(&document);
    let mut query = use_signal(String::new);
    let mut selected = use_signal(|| None::<(Option<Scope>, String)>);
    let catalogue = use_resource(use_reactive((&scope, &reversible_layout), {
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
    let (source_entries, load_state) = match catalogue_state {
        Some(Ok((loaded_scope, loaded_reversible, entries)))
            if loaded_scope == scope && loaded_reversible == reversible_layout =>
        {
            (Some(entries), None)
        }
        Some(Err((loaded_scope, loaded_reversible, error)))
            if loaded_scope == scope && loaded_reversible == reversible_layout =>
        {
            (None, Some(error))
        }
        _ => (None, None),
    };
    let bundle_identity = source_entries
        .as_ref()
        .map_or(0, |entries| Rc::as_ptr(entries) as usize);
    let accepted_token = snapshot.token;
    let merged = use_memo(use_reactive((&accepted_token, &bundle_identity), {
        let document = document.clone();
        let source_entries = source_entries.clone();
        move |_| {
            source_entries.as_ref().map(|source_entries| {
                Rc::new(catalogue::merge_project_overrides(
                    source_entries,
                    &document.definitions,
                ))
            })
        }
    }));
    let merged_entries = merged.read().clone();
    let Some(entries) = merged_entries else {
        return if let Some(error) = load_state {
            rsx! {
                section { class: "m1-parts-workspace", "aria-label": "Parts library",
                    h1 { "Parts library" }
                    p { class: "m1-parts-load-error", role: "alert", "Component catalogue could not be loaded: {error}" }
                }
            }
        } else {
            rsx! {
                section { class: "m1-parts-workspace", "aria-label": "Parts library",
                    h1 { "Parts library" }
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
    let selected_id = selected()
        .filter(|(selected_scope, id)| {
            selected_scope == &scope
                && listed_entries
                    .iter()
                    .any(|entry| entry.definition.id == *id)
        })
        .map(|(_, id)| id)
        .or_else(|| {
            if search.is_empty() {
                listed_entries
                    .iter()
                    .find(|entry| entry.definition.id == "ergogen:ceoloide/switch_mx")
            } else {
                None
            }
            .or_else(|| {
                listed_entries
                    .iter()
                    .find(|entry| entry.matches_library_search(&search))
            })
            .or_else(|| listed_entries.first())
            .map(|entry| entry.definition.id.clone())
        });
    let selected_entry = selected_id
        .as_deref()
        .and_then(|id| {
            listed_entries
                .iter()
                .copied()
                .find(|entry| entry.definition.id == id)
        })
        .map(|entry| entry.clone());

    rsx! {
        section { class: "m1-parts-workspace", "aria-label": "Parts library",
            h1 { "Parts library" }
            div { class: "m1-parts-catalogue-layout",
                section { class: "m1-parts-catalogue", "aria-label": "Component catalogue",
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
                                                    title: "{entry.definition.generator.as_ref().map(|generator| generator.source.as_str()).unwrap_or("")}",
                                                    onclick: move |_| selected.set(Some((scope.clone(), id.clone()))),
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
                SelectedDefinition { entry: selected_entry }
            }
        }
    }
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
