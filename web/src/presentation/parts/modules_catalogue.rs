//! Lazy, read-only projection of the packaged VIK module catalogue.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{HardwareOutput, ModuleDefinition};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

const MODULES_ASSET: &str = "assets/imported-modules.json";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ModuleEntry {
    pub row: String,
    pub definition: Rc<ModuleDefinition>,
    source: EntrySource,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntrySource {
    Bundled,
    Project,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ModuleGroup {
    pub row: String,
    pub name: String,
    pub entries: Vec<ModuleEntry>,
}

impl ModuleGroup {
    pub fn matches(&self, query: &str) -> bool {
        query.is_empty()
            || format!(
                "{} {} VIK {}",
                self.name,
                self.row,
                self.entries
                    .iter()
                    .map(|entry| format!(
                        "{} {}",
                        entry.definition.family, entry.definition.variant
                    ))
                    .collect::<Vec<_>>()
                    .join(" ")
            )
            .to_lowercase()
            .contains(query)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ModulePackage {
    format_version: u32,
    modules: Vec<ModuleRecord>,
}

#[derive(Deserialize)]
struct ModuleRecord {
    row: String,
    definition: ModuleDefinition,
}

thread_local! {
    static BUNDLED_MODULES: RefCell<Option<Rc<Vec<ModuleEntry>>>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug, PartialEq)]
struct ModuleCatalogueRequest {
    active: bool,
    token: SnapshotToken,
    project: Vec<ModuleDefinition>,
}

#[derive(Clone, Debug, PartialEq)]
struct ModuleCatalogueResult {
    request: ModuleCatalogueRequest,
    entries: Option<Rc<Vec<ModuleEntry>>>,
    error: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct ModuleCatalogueView {
    pub entries: Option<Rc<Vec<ModuleEntry>>>,
    pub error: Option<String>,
    pub pending: bool,
}

pub(super) fn use_catalogue(
    active: bool,
    token: SnapshotToken,
    project: Vec<ModuleDefinition>,
) -> ModuleCatalogueView {
    let request = ModuleCatalogueRequest {
        active,
        token,
        project,
    };
    let resource = use_resource(use_reactive(&request, |request| async move {
        if !request.active {
            return ModuleCatalogueResult {
                request,
                entries: None,
                error: None,
            };
        }
        match load_bundled().await {
            Ok(bundled) => ModuleCatalogueResult {
                request: request.clone(),
                entries: Some(Rc::new(merge_project_overrides(&bundled, &request.project))),
                error: None,
            },
            Err(error) => ModuleCatalogueResult {
                request,
                entries: None,
                error: Some(error),
            },
        }
    }));
    let result = resource.read().clone();
    if !active {
        return ModuleCatalogueView::default();
    }
    match result.filter(|result| result.request == request) {
        Some(result) => ModuleCatalogueView {
            entries: result.entries,
            error: result.error,
            pending: false,
        },
        None => ModuleCatalogueView {
            pending: true,
            ..ModuleCatalogueView::default()
        },
    }
}

async fn load_bundled() -> Result<Rc<Vec<ModuleEntry>>, String> {
    if let Some(entries) = BUNDLED_MODULES.with(|cache| cache.borrow().clone()) {
        return Ok(entries);
    }
    let window = web_sys::window().ok_or_else(|| "Browser window is unavailable.".to_string())?;
    let url = crate::runtime::resource_url(MODULES_ASSET)?;
    let response = JsFuture::from(window.fetch_with_str(&url))
        .await
        .map_err(js_error)?
        .dyn_into::<web_sys::Response>()
        .map_err(js_error)?;
    if !response.ok() {
        return Err(format!(
            "Required module catalogue unavailable ({})",
            response.status()
        ));
    }
    let buffer = JsFuture::from(response.array_buffer().map_err(js_error)?)
        .await
        .map_err(js_error)?;
    let bytes = Uint8Array::new(&buffer).to_vec();
    let package: ModulePackage = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Bundled VIK module catalogue is invalid: {error}"))?;
    if package.format_version != 1 {
        return Err(format!(
            "Unsupported VIK module catalogue format version: {}",
            package.format_version
        ));
    }
    let entries: Rc<Vec<ModuleEntry>> = Rc::new(
        package
            .modules
            .into_iter()
            .map(|record| ModuleEntry {
                row: record.row,
                definition: Rc::new(record.definition),
                source: EntrySource::Bundled,
            })
            .collect(),
    );
    BUNDLED_MODULES.with(|cache| *cache.borrow_mut() = Some(entries.clone()));
    Ok(entries)
}

fn merge_project_overrides(
    bundled: &[ModuleEntry],
    project: &[ModuleDefinition],
) -> Vec<ModuleEntry> {
    let mut entries = bundled.to_vec();
    let mut positions = entries
        .iter()
        .enumerate()
        .map(|(index, entry)| (entry.definition.id.clone(), index))
        .collect::<HashMap<_, _>>();
    for definition in project {
        if let Some(index) = positions.get(&definition.id).copied() {
            entries[index].definition = Rc::new(definition.clone());
            entries[index].source = EntrySource::Project;
        } else {
            let row = definition
                .catalogue_row
                .clone()
                .unwrap_or_else(|| definition.id.clone());
            positions.insert(definition.id.clone(), entries.len());
            entries.push(ModuleEntry {
                row,
                definition: Rc::new(definition.clone()),
                source: EntrySource::Project,
            });
        }
    }
    entries
}

pub(super) fn group_choices(entries: &[ModuleEntry]) -> Vec<ModuleGroup> {
    let mut groups = Vec::<ModuleGroup>::new();
    let mut positions = HashMap::<String, usize>::new();
    for entry in entries {
        if let Some(index) = positions.get(&entry.row).copied() {
            groups[index].entries.push(entry.clone());
        } else {
            positions.insert(entry.row.clone(), groups.len());
            groups.push(ModuleGroup {
                row: entry.row.clone(),
                name: entry.definition.name.clone(),
                entries: vec![entry.clone()],
            });
        }
    }
    groups
}

pub(super) fn variants(entries: &[ModuleEntry], row: &str) -> Vec<ModuleEntry> {
    entries
        .iter()
        .filter(|entry| entry.row == row)
        .cloned()
        .collect()
}

pub(super) fn selected_id(
    selection: Option<(Option<Scope>, String)>,
    scope: &Option<Scope>,
) -> Option<String> {
    selection.and_then(|(selected_scope, id)| {
        (selected_scope == *scope)
            .then(|| id.strip_prefix("module:").map(str::to_owned))
            .flatten()
    })
}

#[component]
pub(super) fn ModuleInspector(
    module: ModuleEntry,
    variants: Vec<ModuleEntry>,
    scope: Option<Scope>,
    mut selected: super::PartsSelection,
) -> Element {
    let definition = &module.definition;
    let mut selection_generation = use_context::<super::PartsSelectionGeneration>().0;
    let mut preview_activation = use_context::<super::PartsPreviewActivation>().0;
    let readiness = [
        (HardwareOutput::Footprint, "Footprint"),
        (HardwareOutput::Electrical, "Electrical"),
        (HardwareOutput::Mechanical, "Case"),
        (HardwareOutput::Model, "3D model"),
        (HardwareOutput::Firmware, "Firmware"),
    ];
    let source = &definition.source;
    let source_url = format!(
        "{}/blob/{}/{}",
        source.repository.trim_end_matches('/'),
        source.revision,
        source
            .path
            .split('/')
            .map(|segment| js_sys::encode_uri_component(segment)
                .as_string()
                .unwrap_or_default())
            .collect::<Vec<_>>()
            .join("/")
    );
    let family = definition.family.replace('-', " ");
    let source_summary = format!(
        "{} · {}{}",
        source.license,
        source.revision.chars().take(12).collect::<String>(),
        source
            .upstream_status
            .as_ref()
            .map(|status| format!(" · Upstream: {status}"))
            .unwrap_or_default()
    );
    let project_owned = module.source == EntrySource::Project;
    let gate_count = definition.gates.len();
    let gate_noun = if gate_count == 1 { "item" } else { "items" };
    rsx! {
        h2 { "{definition.name}" }
        small { "VIK · {family}" }
        if !variants.is_empty() {
            label { class: "m1-parts-search-label",
                "Variant"
                select {
                    "aria-label": "Module variant",
                    value: "{definition.id}",
                    onchange: move |event| {
                        selected.set(Some((scope.clone(), format!("module:{}", event.value()))));
                        selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                        preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                    },
                    for variant in variants.iter() {
                        option { value: "{variant.definition.id}", "{variant.definition.variant}" }
                    }
                }
            }
        }
        if project_owned { small { "Project snapshot" } }
        table { class: "m1-module-readiness",
            tbody {
                for (output, label) in readiness {
                    { let pending = definition.gates.iter().filter(|gate| gate.output == output).count();
                      rsx! { tr { th { scope: "row", "{label}" } td { if pending == 0 { "No recorded blockers" } else { "{pending} to review" } } } }
                    }
                }
            }
        }
        if !definition.gates.is_empty() {
            details {
                summary { "Review {gate_count} remaining {gate_noun}" }
                ul {
                    for gate in &definition.gates {
                        li { "{gate.message}" }
                    }
                }
            }
        }
        a { href: "{source_url}", target: "_blank", rel: "noreferrer", "Pinned source ↗" }
        p { class: "m1-parts-empty", "{source_summary}" }
    }
}

fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
