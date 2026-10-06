//! Lazy, read-only projection of the packaged VIK module catalogue.
use super::module_profile_editor::ModuleProfileEditor;
use boardstudio_application::{AcceptedSnapshot, Scope, SnapshotToken};
use boardstudio_core::model::{
    HardwareOutput, ModuleDefinition, PartDefinition, PartModel, Side, Vec3, VikRole,
};
use dioxus::prelude::*;
use js_sys::Uint8Array;
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub(in crate::presentation) mod module_attachment;

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
    let bundled = BUNDLED_MODULES.with(|cache| cache.borrow().clone());
    catalogue_view(&request, result, bundled)
}

fn catalogue_view(
    request: &ModuleCatalogueRequest,
    result: Option<ModuleCatalogueResult>,
    bundled: Option<Rc<Vec<ModuleEntry>>>,
) -> ModuleCatalogueView {
    if !request.active {
        return ModuleCatalogueView::default();
    }
    match result.filter(|result| result.request == *request) {
        Some(result) => ModuleCatalogueView {
            entries: result.entries,
            error: result.error,
            pending: false,
        },
        None => match bundled {
            // The package catalogue is immutable. Re-project only the current accepted
            // document's overrides while the request-owned resource catches up, so an
            // accepted edit cannot unmount the editor that is settling that edit.
            Some(bundled) => ModuleCatalogueView {
                entries: Some(Rc::new(merge_project_overrides(&bundled, &request.project))),
                error: None,
                pending: false,
            },
            None => ModuleCatalogueView {
                pending: true,
                ..ModuleCatalogueView::default()
            },
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

pub(super) async fn load_horizontal_host_connector_definition() -> Result<PartDefinition, String> {
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

fn normalize_host_connector_definition(
    mut definition: PartDefinition,
) -> Result<PartDefinition, String> {
    const MODEL_ASSET: &str =
        "bundled-model:vik/sadekbaroudi-vik/kicad/3dmodels/vik-connector-horizontal.stp";
    definition.id = "vik:source:horizontal-host-connector".into();
    definition.name = "VIK horizontal host connector".into();
    definition.models = Some(vec![PartModel {
        asset_id: MODEL_ASSET.into(),
        offset: Vec3 {
            x: -2.75,
            y: 2.3,
            z: 0.0,
        },
        rotation: Vec3::default(),
        scale: Vec3 {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
    }]);
    if let Some(source) = &mut definition.kicad_source {
        source.source = without_embedded_connector_model(&source.source)?;
    }
    Ok(definition)
}

fn without_embedded_connector_model(source: &str) -> Result<String, String> {
    const MARKER: &str = "(model \"../../kicad/3dmodels/vik-connector-horizontal.stp\"";
    let Some(start) = source.find(MARKER) else {
        return Ok(source.to_owned());
    };
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escaped = false;
    for (offset, character) in source[start..].char_indices() {
        if quoted {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                quoted = false;
            }
            continue;
        }
        match character {
            '"' => quoted = true,
            '(' => depth += 1,
            ')' => {
                depth = depth.checked_sub(1).ok_or_else(|| {
                    "The source VIK host connector has a malformed embedded model form.".to_string()
                })?;
                if depth == 0 {
                    let end = start + offset + character.len_utf8();
                    return Ok(format!("{}{}", &source[..start], &source[end..]));
                }
            }
            _ => {}
        }
    }
    Err("The source VIK host connector has a malformed embedded model form.".into())
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
pub(super) fn ModuleSourcePreview(module: ModuleEntry) -> Element {
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
        section { class: "m1-workspace-content m1-parts-preview", "aria-label": "Module source preview",
            header { class: "m1-parts-module-preview-heading", style: "display: grid; gap: 4px; padding: 12px;",
                h2 { style: "margin: 0; color: var(--wb-ink); font-size: 16px;", "{definition.name}" }
                small { style: "color: var(--wb-muted);", "VIK module · {definition.variant}" }
            }
            if board_path.is_empty() {
                p { class: "m1-parts-preview-status", role: "status", "This module snapshot has no recorded board contours." }
            } else {
                svg { role: "img", "aria-label": "{definition.name} source board and components", view_box: "{view_box}", style: "flex: 1; width: 100%; min-height: 0;",
                    g { transform: "scale(1,-1)",
                        path { d: "{board_path}", fill: "rgba(52,124,99,.2)", fill_rule: "evenodd", stroke: "var(--wb-accent)", stroke_width: "0.35" }
                        for (index, points) in &holes {
                            polygon { key: "source-hole-{index}", points: "{points}", fill: "var(--wb-canvas)", stroke: "var(--wb-muted)", stroke_width: "0.2" }
                        }
                        for part in &circuit_parts {
                            g { key: "{part.id}", transform: "{part.transform}",
                                title { "{part.reference} · {part.source_name}" }
                                if !part.courtyard.is_empty() {
                                    polygon { points: "{part.courtyard}", fill: "none", stroke: "var(--wb-muted)", stroke_width: "0.2" }
                                }
                                for (pad_id, x, y, radius) in &part.drills {
                                    circle { key: "{pad_id}", cx: "{x}", cy: "{y}", r: "{radius}", fill: "var(--wb-canvas)", stroke: "var(--wb-muted)", stroke_width: "0.2" }
                                }
                            }
                        }
                    }
                }
            }
            p { class: "m1-parts-preview-status", style: "margin: 8px 12px;", "{source_summary}" }
        }
    }
}

#[component]
pub(super) fn ModuleInspector(
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
            label { class: "m1-parts-search-label",
                "Variant"
                select {
                    "aria-label": "Module variant",
                    value: "{definition.id}",
                    onchange: move |event| {
                        selected.set(Some((variant_scope.clone(), format!("module:{}", event.value()))));
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
        if let Some(placement_id) = placement_id {
            button {
                class: "m1-parts-place-assembly",
                r#type: "button",
                onclick: move |_| on_open_placement.call(placement_id.clone()),
                "Edit selected mounted placement"
            }
        }
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

#[cfg(all(test, target_arch = "wasm32"))]
mod catalogue_refresh_tests {
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn definition(name: &str) -> ModuleDefinition {
        serde_json::from_value(serde_json::json!({
            "id": "module/test", "name": name, "family": "test", "variant": "test",
            "source": {"repository": "test", "revision": "test", "path": "test", "license": "test"},
            "board": {"contours": []}, "electrical": {"protocol": "i2c"}
        }))
        .unwrap()
    }

    #[wasm_bindgen_test]
    fn module_catalogue_refresh_keeps_current_projection_without_stale_project_entries() {
        let bundled = Rc::new(vec![ModuleEntry {
            row: "test".into(),
            definition: Rc::new(definition("bundled")),
            source: EntrySource::Bundled,
        }]);
        let request = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(2),
            project: vec![definition("accepted")],
        };
        let stale_request = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(1),
            project: vec![definition("previous")],
        };
        let stale_result = ModuleCatalogueResult {
            entries: Some(Rc::new(merge_project_overrides(
                &bundled,
                &stale_request.project,
            ))),
            request: stale_request,
            error: None,
        };
        let view = catalogue_view(&request, Some(stale_result), Some(bundled.clone()));
        assert!(
            !view.pending,
            "accepted token refresh must not unmount the selected module owner"
        );
        let entries = view.entries.unwrap();
        assert_eq!(entries[0].definition.name, "accepted");
        assert_eq!(entries[0].source, EntrySource::Project);

        let switched = ModuleCatalogueRequest {
            active: true,
            token: SnapshotToken(3),
            project: Vec::new(),
        };
        let previous = ModuleCatalogueResult {
            request: request.clone(),
            entries: Some(entries),
            error: None,
        };
        let view = catalogue_view(&switched, Some(previous), Some(bundled.clone()));
        assert_eq!(view.entries.unwrap()[0].definition.name, "bundled");
        assert!(catalogue_view(&switched, None, None).pending);
        let inactive = ModuleCatalogueRequest {
            active: false,
            ..switched
        };
        assert!(
            catalogue_view(&inactive, None, Some(bundled))
                .entries
                .is_none()
        );
    }
}
