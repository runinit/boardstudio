//! Lazy, read-only projection of the packaged VIK module catalogue.
use super::module_profile_editor::ModuleProfileEditor;
use super::modules_catalogue_logic::{
    ModuleCatalogueRequest, ModuleCatalogueResult, catalogue_view, merge_project_overrides,
    normalize_host_connector_definition,
};
use boardstudio_web_ui_shared::inspector_fields::TOUCH_CLS;
// Host-compatible catalogue projection lives in `modules_catalogue_logic` so the same
// merge/selection predicates run under `cargo test` without a browser.
pub use super::modules_catalogue_logic::{
    EntrySource, ModuleCatalogueView, ModuleEntry, selected_id, variants,
};
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_model::model::{HardwareOutput, ModuleDefinition, PartDefinition, Side, VikRole};
use boardstudio_web_ui_shared::ui::{
    button::{Button, ButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    table::{Table, TableBody, TableCell, TableRow},
};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use serde::Deserialize;
use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub mod module_attachment;

const MODULES_ASSET: &str = "assets/imported-modules.json";

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

pub fn use_catalogue(
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
    let bundled = BUNDLED_MODULES.with(|cache| cache.borrow().clone());
    catalogue_view(&request, result, bundled)
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

pub async fn load_horizontal_host_connector_definition() -> Result<PartDefinition, String> {
    let entries = load_bundled().await?;
    let source = entries
        .iter()
        .flat_map(|entry| {
            entry
                .definition
                .circuit
                .iter()
                .flat_map(|circuit| circuit.definitions.iter())
        })
        .find(|definition| {
            definition
                .hardware_profile
                .as_ref()
                .is_some_and(|profile| profile.vik_role == Some(VikRole::Host))
                && definition.name.to_ascii_lowercase().contains("horizontal")
        })
        .cloned()
        .ok_or_else(|| {
            "The source catalogue has no horizontal VIK host connector footprint.".to_string()
        })?;

    normalize_host_connector_definition(source)
}

struct CircuitPartPreview {
    id: String,
    reference: String,
    source_name: String,
    transform: String,
    courtyard: String,
    drills: Vec<(String, f64, f64, f64)>,
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
        .map(|contour| {
            format!(
                "M{}Z",
                contour
                    .points
                    .iter()
                    .map(|point| format!("{},{}", point.x, point.y))
                    .collect::<Vec<_>>()
                    .join("L")
            )
        })
        .collect::<Vec<_>>()
        .join(" ");
    let view_box = format!(
        "{} {} {} {}",
        min_x - 5.0,
        -max_y - 5.0,
        (max_x - min_x + 10.0).max(10.0),
        (max_y - min_y + 10.0).max(10.0)
    );
    let holes = definition
        .board
        .holes
        .iter()
        .enumerate()
        .map(|(index, points)| {
            (
                index,
                points
                    .iter()
                    .map(|point| format!("{},{}", point.x, point.y))
                    .collect::<Vec<_>>()
                    .join(" "),
            )
        })
        .collect::<Vec<_>>();
    let circuit_parts = definition
        .circuit
        .as_ref()
        .into_iter()
        .flat_map(|circuit| circuit.parts.iter().map(move |part| (circuit, part)))
        .filter_map(|(circuit, part)| {
            let source = circuit
                .definitions
                .iter()
                .find(|source| source.id == part.definition_id)?;
            let courtyard = source
                .courtyard
                .iter()
                .map(|point| format!("{},{}", point.x, point.y))
                .collect::<Vec<_>>()
                .join(" ");
            let drills = source
                .pads
                .iter()
                .filter_map(|pad| {
                    pad.drill
                        .map(|drill| (pad.id.clone(), pad.at.x, pad.at.y, drill / 2.0))
                })
                .collect();
            let mirror = if matches!(&part.side, Side::Back) {
                -1
            } else {
                1
            };
            Some(CircuitPartPreview {
                id: part.id.clone(),
                reference: part.reference.clone(),
                source_name: source.name.clone(),
                transform: format!(
                    "translate({} {}) rotate({}) scale({mirror},1)",
                    part.pose.at.x, part.pose.at.y, part.pose.rotation
                ),
                courtyard,
                drills,
            })
        })
        .collect::<Vec<_>>();
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
                        for (index, points) in &holes {
                            polygon { key: "source-hole-{index}", points: "{points}", fill: "var(--canvas)", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                        }
                        for part in &circuit_parts {
                            g { key: "{part.id}", transform: "{part.transform}",
                                title { "{part.reference} · {part.source_name}" }
                                if !part.courtyard.is_empty() {
                                    polygon { points: "{part.courtyard}", fill: "none", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                                }
                                for (pad_id, x, y, radius) in &part.drills {
                                    circle { key: "{pad_id}", cx: "{x}", cy: "{y}", r: "{radius}", fill: "var(--canvas)", stroke: "var(--muted-foreground)", stroke_width: "0.2" }
                                }
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
    variants: Vec<ModuleEntry>,
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
                options: variants.iter().map(|variant| (variant.definition.id.clone(), variant.definition.variant.clone())).collect(),
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

// Catalogue merge/selection projection runs natively via `modules_catalogue_logic` tests.
