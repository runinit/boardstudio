//! Lazy, read-only projection of the packaged VIK module catalogue.
use super::module_profile_editor::ModuleProfileEditor;
pub use super::modules_catalogue_logic::{
    EntrySource, ModuleCatalogueView, ModuleEntry, selected_id,
};
use super::modules_catalogue_logic::{
    ModuleIndex, ModuleVariantChoice, catalogue_view, normalize_host_connector_definition,
};
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::{HardwareOutput, ModuleDefinition, PartDefinition, Side};
use boardstudio_web_ui_shared::inspector_fields::TOUCH_CLS;
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    table::{Table, TableBody, TableCell, TableRow},
};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::{JsFuture, future_to_promise};

pub mod module_attachment;

#[derive(Deserialize)]
struct ModuleRecord {
    row: String,
    definition: ModuleDefinition,
}

thread_local! {
    static BUNDLED_INDEX: std::cell::OnceCell<Rc<ModuleIndex>> = const { std::cell::OnceCell::new() };
    static BUNDLED_MODULES: RefCell<HashMap<String, ModuleEntry>> = RefCell::default();
    static PENDING_ASSETS: RefCell<HashMap<String, js_sys::Promise>> = RefCell::default();
    static HOST_CONNECTOR: std::cell::OnceCell<PartDefinition> = const { std::cell::OnceCell::new() };
}

pub fn use_catalogue(
    selected_id: Option<String>,
    token: SnapshotToken,
    project: Vec<ModuleDefinition>,
) -> ModuleCatalogueView {
    let overridden = selected_id
        .as_ref()
        .is_some_and(|id| project.iter().any(|entry| &entry.id == id));
    let request = (selected_id, token, overridden);
    let load = use_resource(use_reactive(&request, |request| async move {
        let error = match &request {
            (Some(id), _, overridden) => load_selection(id, *overridden).await.err(),
            _ => None,
        };
        (request, error)
    }));
    let error = load
        .read()
        .as_ref()
        .filter(|(owner, _)| owner == &request)
        .and_then(|(_, error)| error.clone());
    let Some(id) = request.0 else {
        return ModuleCatalogueView::default();
    };
    let index = BUNDLED_INDEX.with(|cache| cache.get().cloned());
    let bundled = cached_module(&id);
    catalogue_view(
        &id,
        &project,
        index.as_ref().map_or(&[], |index| index.modules.as_slice()),
        bundled,
        error,
    )
}

fn cached_module(id: &str) -> Option<ModuleEntry> {
    BUNDLED_MODULES.with(|cache| cache.borrow().get(id).cloned())
}

async fn load_selection(id: &str, overridden: bool) -> Result<(), String> {
    let index = load_index().await?;
    if !overridden && cached_module(id).is_none() {
        let entry = index
            .modules
            .iter()
            .find(|entry| entry.id == id)
            .ok_or_else(|| format!("Module catalogue has no source for {id}."))?;
        let record: ModuleRecord = fetch_json(&entry.asset).await?;
        if record.definition.id != id || record.row != entry.row {
            return Err(format!(
                "Module source does not match its catalogue entry: {id}"
            ));
        }
        BUNDLED_MODULES.with_borrow_mut(|modules| {
            modules.insert(
                id.to_owned(),
                ModuleEntry {
                    row: record.row,
                    definition: Rc::new(record.definition),
                    source: EntrySource::Bundled,
                },
            )
        });
    }
    Ok(())
}

async fn load_index() -> Result<Rc<ModuleIndex>, String> {
    if let Some(index) = BUNDLED_INDEX.with(|cache| cache.get().cloned()) {
        return Ok(index);
    }
    let index: ModuleIndex = fetch_json("assets/modules-index.json").await?;
    if index.format_version != 1 {
        return Err(format!(
            "Unsupported VIK module catalogue format version: {}",
            index.format_version
        ));
    }
    Ok(BUNDLED_INDEX.with(|cache| cache.get_or_init(|| Rc::new(index)).clone()))
}

async fn fetch_json<T: serde::de::DeserializeOwned>(asset: &str) -> Result<T, String> {
    let url = crate::runtime::resource_url(asset)?;
    let promise = PENDING_ASSETS.with_borrow_mut(|pending| {
        pending
            .entry(url.clone())
            .or_insert_with(|| {
                future_to_promise(async move {
                    let result = fetch_buffer(&url).await;
                    PENDING_ASSETS.with_borrow_mut(|pending| pending.remove(&url));
                    result
                })
            })
            .clone()
    });
    let buffer = JsFuture::from(promise).await.map_err(js_error)?;
    serde_json::from_slice(&Uint8Array::new(&buffer).to_vec())
        .map_err(|error| format!("Module asset {asset} is invalid: {error}"))
}

// The inspector and source preview can request the same asset in one render.
async fn fetch_buffer(url: &str) -> Result<JsValue, JsValue> {
    let window = web_sys::window().ok_or("Browser window is unavailable.")?;
    let response: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
        .await?
        .dyn_into()?;
    if !response.ok() {
        return Err(JsValue::from_str(&format!(
            "Module asset {url} unavailable ({})",
            response.status()
        )));
    }
    JsFuture::from(response.array_buffer()?).await
}

pub async fn load_horizontal_host_connector_definition() -> Result<PartDefinition, String> {
    if let Some(definition) = HOST_CONNECTOR.with(|cache| cache.get().cloned()) {
        return Ok(definition);
    }
    let index = load_index().await?;
    let definition =
        normalize_host_connector_definition(fetch_json(&index.host_connector_asset).await?)?;
    Ok(HOST_CONNECTOR.with(|cache| cache.get_or_init(|| definition).clone()))
}

#[component]
pub fn ModuleBrowser(scope: Option<Scope>, mut selected: super::PartsSelection) -> Element {
    let index = use_resource(load_index);
    let mut generation = use_context::<super::PartsSelectionGeneration>().0;
    let mut activation = use_context::<super::PartsPreviewActivation>().0;
    let mut assembly = use_context::<super::PartsAssemblySelection>().0;
    let selection = selected_id(selected(), &scope).unwrap_or_default();
    match &*index.read() {
        Some(Ok(index)) => rsx! {
            super::SelectField::<String> {
                label: "Module", placeholder: "Choose a module", aria_label: "Browse modules", value: selection,
                options: index.modules.iter().map(|entry| (entry.id.clone(), format!("{} · {}", entry.name, entry.variant))).collect(),
                on_commit: move |id: String| {
                    assembly.set(None);
                    selected.set(Some((scope.clone(), format!("module:{id}"))));
                    generation.with_mut(|value| *value = value.wrapping_add(1));
                    activation.with_mut(|value| *value = value.wrapping_add(1));
                },
            }
        },
        result => super::catalogue_status(
            result
                .as_ref()
                .and_then(|result| result.as_ref().err().cloned()),
            "Module catalogue",
            "Loading module catalogue…",
        ),
    }
}

fn svg_points(points: &[boardstudio_model::model::Vec2], separator: &str) -> String {
    points
        .iter()
        .map(|point| format!("{},{}", point.x, point.y))
        .collect::<Vec<_>>()
        .join(separator)
}

/// Read-only rendering of the selected module snapshot's own board and circuit source geometry.
#[component]
pub fn ModuleSourcePreview(module: ModuleEntry) -> Element {
    let definition = &module.definition;
    let board_points = definition
        .board
        .contours
        .iter()
        .flat_map(|contour| contour.points.iter());
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (0.0_f64, 0.0_f64, 0.0_f64, 0.0_f64);
    for point in board_points {
        min_x = min_x.min(point.x);
        max_x = max_x.max(point.x);
        min_y = min_y.min(point.y);
        max_y = max_y.max(point.y);
    }
    let board_path = definition
        .board
        .contours
        .iter()
        .filter(|contour| !contour.points.is_empty())
        .map(|contour| format!("M{}Z", svg_points(&contour.points, "L")))
        .collect::<Vec<_>>()
        .join(" ");
    let view_box = format!(
        "{} {} {} {}",
        min_x - 5.0,
        -max_y - 5.0,
        (max_x - min_x + 10.0).max(10.0),
        (max_y - min_y + 10.0).max(10.0)
    );
    let circuit_parts = definition.circuit.iter().flat_map(|circuit| {
        circuit.parts.iter().filter_map(|part| {
            let source = circuit
                .definitions
                .iter()
                .find(|source| source.id == part.definition_id)?;
            Some((part, source))
        })
    });
    let thickness = definition
        .board
        .thickness
        .map(|thickness| format!("{thickness} mm PCB"))
        .unwrap_or_else(|| "PCB thickness needs review".into());
    let source_summary = format!(
        "{thickness} · {} source components · {} mounting holes",
        definition.constituents.len(),
        definition.mounts.len()
    );
    rsx! {
        section { class: "relative order-2 flex flex-1 flex-col min-w-0 min-h-0 overflow-hidden rounded-xl bg-card shadow-sm", "aria-label": "Module source preview",
            header { style: "display: grid; gap: 4px; padding: 12px;",
                h2 { style: "margin: 0; color: var(--foreground); font-size: 16px;", "{definition.name}" }
                small { style: "color: var(--muted-foreground);", "VIK module · {definition.variant}" }
            }
            if board_path.is_empty() {
                p { class: "text-primary", role: "status", "This module snapshot has no recorded board contours." }
            } else {
                svg { role: "img", "aria-label": "{definition.name} source board and components", view_box: "{view_box}", style: "flex: 1; width: 100%; min-height: 0;",
                    g { transform: "scale(1,-1)",
                        path { d: "{board_path}", fill: "rgba(52,124,99,.2)", fill_rule: "evenodd", stroke: "var(--primary)", stroke_width: "0.35" }
                        for (index, points) in definition.board.holes.iter().enumerate() {
                            polygon { key: "source-hole-{index}", points: svg_points(points, " "), fill: "var(--canvas)", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                        }
                        for (part, source) in circuit_parts {
                            { let mirror = if part.side == Side::Back { -1 } else { 1 };
                              let transform = format!("translate({} {}) rotate({}) scale({mirror},1)", part.pose.at.x, part.pose.at.y, part.pose.rotation);
                              rsx! { g { key: "{part.id}", transform,
                                  title { "{part.reference} · {source.name}" }
                                  if !source.courtyard.is_empty() {
                                      polygon { points: svg_points(&source.courtyard, " "), fill: "none", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                                  }
                                  for pad in &source.pads {
                                      if let Some(drill) = pad.drill {
                                          circle { key: "{pad.id}", cx: "{pad.at.x}", cy: "{pad.at.y}", r: "{drill / 2.0}", fill: "var(--canvas)", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                                      }
                                  }
                              } }
                            }
                        }
                    }
                }
            }
            p { class: "text-primary", style: "margin: 8px 12px;", "{source_summary}" }
        }
    }
}

#[component]
pub fn ModuleInspector(
    snapshot: AcceptedSnapshot,
    module: ModuleEntry,
    variants: Vec<ModuleVariantChoice>,
    scope: Option<Scope>,
    mut selected: super::PartsSelection,
    placement_id: Option<String>,
    on_open_placement: EventHandler<String>,
    on_attached: EventHandler<module_attachment::AttachedModuleNavigation>,
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
    let variant_scope = scope.clone();
    let gate_count = definition.gates.len();
    let gate_noun = if gate_count == 1 { "item" } else { "items" };
    rsx! {
        h2 { "{definition.name}" }
        small { "VIK · {family}" }
        if !variants.is_empty() {
            super::SelectField::<String> {
                label: "Variant", aria_label: "Module variant", value: definition.id.clone(),
                options: variants.iter().map(|variant| (variant.id.clone(), variant.variant.clone())).collect(),
                on_commit: move |value: String| {
                    selected.set(Some((variant_scope.clone(), format!("module:{value}"))));
                    selection_generation.with_mut(|value| *value = value.wrapping_add(1));
                    preview_activation.with_mut(|value| *value = value.wrapping_add(1));
                },
            }
        }
        if project_owned { small { "Project snapshot" } }
        if let Some(placement_id) = placement_id {
            Button { variant: ButtonVariant::Secondary,
                class: TOUCH_CLS,
                onclick: move |_| on_open_placement.call(placement_id.clone()),
                "Edit selected mounted placement"
            }
        }
        Table {
            TableBody {
                for (output, label) in readiness {
                    { let pending = definition.gates.iter().filter(|gate| gate.output == output).count();
                      rsx! { TableRow { th { scope: "row", "{label}" } TableCell { if pending == 0 { "No recorded blockers" } else { "{pending} to review" } } } }
                    }
                }
            }
        }
        if !definition.gates.is_empty() {
            Collapsible { keep_mounted: true,
                CollapsibleTrigger { class: TOUCH_CLS, "Review {gate_count} remaining {gate_noun}" }
                CollapsibleContent { class: "data-[open=false]:hidden", ul {
                    for gate in &definition.gates {
                        li { "{gate.message}" }
                    }
                } }
            }
        }
        a { href: "{source_url}", target: "_blank", rel: "noreferrer", "Pinned source ↗" }
        p { class: "my-3 text-muted-foreground text-sm leading-normal", "{source_summary}" }
        ModuleProfileEditor {
            snapshot: snapshot.clone(),
            definition: (*module.definition).clone(),
            project_owned,
            scope: scope.clone(),
            selected,
        }
        for owner_key in [format!("{:?}:{}:{}", scope, definition.id, selection_generation())] {
            module_attachment::ModuleAttachment {
                key: "{owner_key}",
                snapshot: snapshot.clone(),
                module: module.clone(),
                scope: scope.clone(),
                selected,
                on_attached,
            }
        }
    }
}

fn js_error(error: JsValue) -> String {
    error.as_string().unwrap_or_else(|| format!("{error:?}"))
}
