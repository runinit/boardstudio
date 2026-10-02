use boardstudio_core::model::{PartDefinition, PartKind};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::HashMap, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

const IMPORTED_PARTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../app/src/parts/imported-parts.json"
));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CatalogueSource {
    Ergogen,
    Imported,
    Project,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct CatalogEntry {
    pub definition: PartDefinition,
    pub source: CatalogueSource,
}

#[derive(Debug)]
pub(super) struct CatalogGroup<'a> {
    pub label: &'static str,
    pub entries: Vec<&'a CatalogEntry>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CatalogKind {
    Switch,
    Controller,
    Connector,
    Encoder,
    Passive,
    Utility,
    Custom,
}

impl CatalogEntry {
    pub fn matches_library_search(&self, query: &str) -> bool {
        query.is_empty() || self.library_search_text().to_lowercase().contains(query)
    }

    pub fn matches(&self, query: &str, category_label: &'static str) -> bool {
        query.is_empty()
            || format!("{} {category_label}", self.library_search_text())
                .to_lowercase()
                .contains(query)
    }

    fn library_search_text(&self) -> String {
        format!(
            "{} {} {} {} {}",
            preferred_label(&self.definition),
            self.definition.name,
            kind_search_label(&self.definition.kind),
            self.definition
                .generator
                .as_ref()
                .map(|generator| generator.source.as_str())
                .unwrap_or_default(),
            search_aliases(&self.definition),
        )
    }
}

#[derive(Deserialize)]
struct ImportedCatalogue {
    parts: Vec<ImportedPart>,
}

#[derive(Deserialize)]
struct ImportedPart {
    definition: PartDefinition,
}

struct CachedCatalogue {
    module: JsValue,
    imported_source_hash: String,
    reversible: bool,
    entries: Rc<Vec<CatalogEntry>>,
}

thread_local! {
    static BUNDLED_CACHE: RefCell<Vec<CachedCatalogue>> = const { RefCell::new(Vec::new()) };
}

pub(super) async fn load_bundled(reversible: bool) -> Result<Rc<Vec<CatalogEntry>>, String> {
    let module = load_ergogen_module().await?;
    let imported_source_hash = format!("{:x}", Sha256::digest(IMPORTED_PARTS_JSON.as_bytes()));
    if let Some(entries) = BUNDLED_CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|cached| {
                cached.reversible == reversible
                    && cached.imported_source_hash == imported_source_hash
                    && js_sys::Object::is(&cached.module, &module)
            })
            .map(|cached| cached.entries.clone())
    }) {
        return Ok(entries);
    }

    let imported: ImportedCatalogue = serde_json::from_str(IMPORTED_PARTS_JSON)
        .map_err(|error| format!("Imported component definitions are invalid: {error}"))?;
    let imported = imported
        .parts
        .into_iter()
        .map(|part| part.definition)
        .collect::<Vec<_>>();
    let ergogen = call_catalogue(&module)?
        .into_iter()
        .map(|definition| construction_definition(&module, definition, reversible))
        .collect::<Result<Vec<_>, _>>()?;
    let imported = imported
        .into_iter()
        .map(|definition| construction_definition(&module, definition, reversible))
        .collect::<Result<Vec<_>, _>>()?;
    let entries = Rc::new(merge_bundled_sources(&ergogen, &imported));
    BUNDLED_CACHE.with(|cache| {
        cache.borrow_mut().push(CachedCatalogue {
            module,
            imported_source_hash,
            reversible,
            entries: entries.clone(),
        });
    });
    Ok(entries)
}

fn merge_bundled_sources(
    ergogen: &[PartDefinition],
    imported: &[PartDefinition],
) -> Vec<CatalogEntry> {
    let mut entries = Vec::with_capacity(ergogen.len() + imported.len());
    let mut positions = HashMap::<String, usize>::with_capacity(entries.capacity());
    for (definitions, source) in [
        (ergogen, CatalogueSource::Ergogen),
        (imported, CatalogueSource::Imported),
    ] {
        for definition in definitions {
            if let Some(index) = positions.get(&definition.id).copied() {
                entries[index] = CatalogEntry {
                    definition: definition.clone(),
                    source,
                };
            } else {
                positions.insert(definition.id.clone(), entries.len());
                entries.push(CatalogEntry {
                    definition: definition.clone(),
                    source,
                });
            }
        }
    }
    entries
}

pub(super) fn merge_project_overrides(
    bundled: &[CatalogEntry],
    project: &[PartDefinition],
) -> Vec<CatalogEntry> {
    let mut entries = bundled.to_vec();
    let mut positions = HashMap::<String, usize>::with_capacity(entries.len() + project.len());
    for (index, entry) in entries.iter().enumerate() {
        positions.insert(entry.definition.id.clone(), index);
    }
    for definition in project {
        if let Some(index) = positions.get(&definition.id).copied() {
            entries[index] = CatalogEntry {
                definition: definition.clone(),
                source: CatalogueSource::Project,
            };
        } else {
            positions.insert(definition.id.clone(), entries.len());
            entries.push(CatalogEntry {
                definition: definition.clone(),
                source: CatalogueSource::Project,
            });
        }
    }
    entries
}

pub(super) fn catalogue_choices(entries: &[CatalogEntry]) -> Vec<&CatalogEntry> {
    entries
        .iter()
        .filter(|entry| is_catalogue_choice(entry))
        .collect()
}

pub(super) fn group_choices(entries: &[CatalogEntry]) -> Vec<CatalogGroup<'_>> {
    let groups = [
        ("Switches", CatalogKind::Switch),
        ("Controllers", CatalogKind::Controller),
        ("Connectors & sockets", CatalogKind::Connector),
        ("Encoders", CatalogKind::Encoder),
        ("Passives & LEDs", CatalogKind::Passive),
        ("Utilities", CatalogKind::Utility),
        ("Custom", CatalogKind::Custom),
    ];
    groups
        .into_iter()
        .filter_map(|(label, kind)| {
            let entries = catalogue_choices(entries)
                .into_iter()
                .filter(|entry| catalog_kind(&entry.definition.kind) == kind)
                .collect::<Vec<_>>();
            (!entries.is_empty()).then_some(CatalogGroup { label, entries })
        })
        .collect()
}

pub(super) fn preferred_label(definition: &PartDefinition) -> &str {
    match definition.id.as_str() {
        "ergogen:ceoloide/switch_mx" => "MX switch",
        "ergogen:ceoloide/switch_choc_v1_v2" => "Choc V1 / V2 switch",
        "ergogen:ceoloide/switch_gateron_ks27_ks33" => "Gateron KS27 / KS33 switch",
        "ergogen:ceoloide/diode_tht_sod123" => "Matrix diode (SOD-123 / THT)",
        "ergogen:ceoloide/led_sk6812mini-e" => "SK6812 MINI-E",
        _ => &definition.name,
    }
}

async fn load_ergogen_module() -> Result<JsValue, String> {
    let url = crate::runtime::resource_url("assets/layout-generators/src/index.js")?;
    let import = js_sys::Function::new_with_args("url", "return import(url)");
    let promise = import
        .call1(&JsValue::NULL, &url.into())
        .map_err(js_error)?
        .dyn_into::<js_sys::Promise>()
        .map_err(js_error)?;
    JsFuture::from(promise).await.map_err(js_error)
}

fn call_catalogue(module: &JsValue) -> Result<Vec<PartDefinition>, String> {
    let catalogue = function(module, "catalogue")?;
    let value = catalogue.call0(module).map_err(js_error)?;
    serde_wasm_bindgen::from_value(value)
        .map_err(|error| format!("Ergogen catalogue could not be decoded: {error}"))
}

fn construction_definition(
    module: &JsValue,
    mut definition: PartDefinition,
    reversible: bool,
) -> Result<PartDefinition, String> {
    let Some(generator) = definition.generator.as_mut() else {
        return Ok(definition);
    };
    let source = generator.source.clone();
    let is_ergogen = function(module, "isErgogen")?
        .call1(module, &source.clone().into())
        .map_err(js_error)?
        .as_bool()
        .unwrap_or(false);
    if !is_ergogen {
        return Ok(definition);
    }
    let parameter_schema = function(module, "parameters")?
        .call1(module, &source.clone().into())
        .map_err(js_error)?;
    if !js_sys::Reflect::has(&parameter_schema, &"reversible".into()).map_err(js_error)? {
        return Ok(definition);
    }

    generator
        .parameters
        .insert("reversible".into(), serde_json::Value::Bool(reversible));
    if reversible && source == "ceoloide/switch_gateron_ks27_ks33" {
        generator
            .parameters
            .insert("hotswap".into(), serde_json::Value::Bool(false));
        generator
            .parameters
            .insert("solder".into(), serde_json::Value::Bool(true));
    }

    // Generator code spreads `generator.parameters` as a plain JS object. The
    // default serde-wasm-bindgen Map representation is not compatible with it.
    let definition = json_plain_value(&definition)
        .map_err(|error| format!("Could not prepare {source} for construction: {error}"))?;
    let normalized = function(module, "normalizeDefinition")?
        .call1(module, &definition)
        .map_err(js_error)?;
    serde_wasm_bindgen::from_value(normalized)
        .map_err(|error| format!("Could not decode normalized {source} definition: {error}"))
}

fn json_text<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| error.to_string())
}

fn json_plain_value<T: serde::Serialize>(value: &T) -> Result<JsValue, String> {
    js_sys::JSON::parse(&json_text(value)?).map_err(js_error)
}

fn function(module: &JsValue, name: &str) -> Result<js_sys::Function, String> {
    js_sys::Reflect::get(module, &name.into())
        .map_err(js_error)?
        .dyn_into::<js_sys::Function>()
        .map_err(|_| format!("Packaged Ergogen catalogue is missing {name}()"))
}

fn js_error(error: JsValue) -> String {
    format!("{error:?}")
}

fn catalog_kind(kind: &PartKind) -> CatalogKind {
    match kind {
        PartKind::Switch => CatalogKind::Switch,
        PartKind::Controller => CatalogKind::Controller,
        PartKind::Connector => CatalogKind::Connector,
        PartKind::Encoder => CatalogKind::Encoder,
        PartKind::Passive => CatalogKind::Passive,
        PartKind::Utility => CatalogKind::Utility,
        PartKind::Custom => CatalogKind::Custom,
    }
}

fn kind_search_label(kind: &PartKind) -> &'static str {
    match kind {
        PartKind::Switch => "switch",
        PartKind::Controller => "controller",
        PartKind::Connector => "connector",
        PartKind::Encoder => "encoder",
        PartKind::Passive => "passive",
        PartKind::Utility => "utility",
        PartKind::Custom => "custom",
    }
}

fn category_label(kind: &PartKind) -> &'static str {
    match kind {
        PartKind::Switch => "Switches",
        PartKind::Controller => "Controllers",
        PartKind::Connector => "Connectors & sockets",
        PartKind::Encoder => "Encoders",
        PartKind::Passive => "Passives & LEDs",
        PartKind::Utility => "Utilities",
        PartKind::Custom => "Custom",
    }
}

fn search_aliases(definition: &PartDefinition) -> &'static str {
    match definition.id.as_str() {
        "ergogen:ceoloide/led_sk6812mini-e" => "RGB LED reverse mount",
        _ if matches!(&definition.kind, PartKind::Switch) => "solder hotswap",
        _ => "",
    }
}

fn is_catalogue_choice(entry: &CatalogEntry) -> bool {
    entry
        .definition
        .generator
        .as_ref()
        .is_none_or(|generator| generator.source != "infused-kim/nice_nano_pretty")
        && !is_assembly_snapshot(&entry.definition)
}

fn is_assembly_snapshot(definition: &PartDefinition) -> bool {
    if definition.kicad_source.is_some() {
        return false;
    }
    let Some((prefix, tail)) = definition.id.split_once('/') else {
        return false;
    };
    let Some((marker, _)) = tail.split_once('/') else {
        return false;
    };
    if !marker.eq_ignore_ascii_case("definition") {
        return false;
    }
    (prefix
        .get(.."assembly-".len())
        .is_some_and(|start| start.eq_ignore_ascii_case("assembly-"))
        && prefix.len() > "assembly-".len())
        || is_uuid(prefix)
}

fn is_uuid(value: &str) -> bool {
    if value.len() != 36 {
        return false;
    }
    value.bytes().enumerate().all(|(index, byte)| {
        if matches!(index, 8 | 13 | 18 | 23) {
            byte == b'-'
        } else {
            byte.is_ascii_hexdigit()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn imported_definitions() -> Vec<PartDefinition> {
        let catalogue: ImportedCatalogue =
            serde_json::from_str(IMPORTED_PARTS_JSON).expect("bundled imported definitions decode");
        catalogue
            .parts
            .into_iter()
            .map(|part| part.definition)
            .collect()
    }

    #[test]
    fn every_imported_static_definition_decodes_as_the_existing_core_type() {
        let definitions = imported_definitions();
        assert_eq!(definitions.len(), 9);
        assert!(
            definitions
                .iter()
                .all(|definition| definition.kicad_source.is_some())
        );
    }

    #[test]
    fn imported_and_project_overrides_keep_the_first_id_position() {
        let actual = imported_definitions().remove(0);
        let id = actual.id.clone();
        let mut ergogen = actual.clone();
        ergogen.name = "Ergogen value".into();
        let mut imported = actual.clone();
        imported.name = "Imported value".into();
        let bundled = merge_bundled_sources(&[ergogen], &[imported]);
        assert_eq!(bundled.len(), 1);
        assert_eq!(bundled[0].definition.name, "Imported value");
        assert_eq!(bundled[0].source, CatalogueSource::Imported);

        let mut project = actual;
        project.name = "Project override".into();
        let entries = merge_project_overrides(&bundled, &[project]);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].definition.id, id);
        assert_eq!(entries[0].definition.name, "Project override");
        assert_eq!(entries[0].source, CatalogueSource::Project);
    }

    #[test]
    fn category_remains_searchable_alongside_special_search_aliases() {
        let mut led = imported_definitions().remove(0);
        led.id = "ergogen:ceoloide/led_sk6812mini-e".into();
        led.kind = PartKind::Passive;
        let entry = CatalogEntry {
            definition: led,
            source: CatalogueSource::Ergogen,
        };
        assert!(entry.matches("passives & leds", "Passives & LEDs"));
        assert!(!entry.matches_library_search("passives & leds"));
        assert!(entry.matches("rgb led reverse mount", "Passives & LEDs"));
        assert!(entry.matches_library_search("rgb led reverse mount"));

        let mut switch = entry.definition.clone();
        switch.id = "ergogen:ceoloide/switch_mx".into();
        switch.kind = PartKind::Switch;
        let entry = CatalogEntry {
            definition: switch,
            source: CatalogueSource::Ergogen,
        };
        assert!(entry.matches("switches", "Switches"));
        assert!(!entry.matches_library_search("switches"));
        assert!(entry.matches("solder hotswap", "Switches"));
        assert!(entry.matches_library_search("solder hotswap"));
    }

    #[test]
    fn typed_generator_parameters_serialize_as_plain_json_objects() {
        let mut definition = imported_definitions().remove(0);
        let mut parameters = std::collections::BTreeMap::new();
        parameters.insert("reversible".into(), serde_json::Value::Bool(true));
        parameters.insert("hotswap".into(), serde_json::Value::Bool(false));
        definition.generator = Some(boardstudio_core::model::PartGenerator {
            source: "ceoloide/switch_gateron_ks27_ks33".into(),
            version: "test".into(),
            parameters,
        });
        let json = json_text(&definition).expect("serialize typed definition");
        let json: serde_json::Value = serde_json::from_str(&json).expect("definition JSON");
        assert!(json["generator"]["parameters"].is_object());
        assert_eq!(json["generator"]["parameters"]["reversible"], true);
        assert_eq!(json["generator"]["parameters"]["hotswap"], false);
    }

    #[test]
    fn catalog_hides_retired_generator_and_unassigned_assembly_snapshots() {
        let mut definitions = imported_definitions();
        let mut snapshot = definitions[0].clone();
        snapshot.id = "assembly-preset-mx-rgb-south-matrix-0/definition/switch".into();
        snapshot.kicad_source = None;
        snapshot.generator = None;
        definitions.push(snapshot);
        let mut uppercase_snapshot = definitions[0].clone();
        uppercase_snapshot.id = "ASSEMBLY-x/DEFINITION/switch".into();
        uppercase_snapshot.kicad_source = None;
        uppercase_snapshot.generator = None;
        definitions.push(uppercase_snapshot);
        let mut uppercase_uuid_snapshot = definitions[0].clone();
        uppercase_uuid_snapshot.id =
            "A1B2C3D4-E5F6-A1B2-C3D4-E5F6A1B2C3D4/DEFINITION/switch".into();
        uppercase_uuid_snapshot.kicad_source = None;
        uppercase_uuid_snapshot.generator = None;
        definitions.push(uppercase_uuid_snapshot);
        let mut retired = definitions[0].clone();
        retired.id = "legacy-nice-nano".into();
        retired.generator = Some(boardstudio_core::model::PartGenerator {
            source: "infused-kim/nice_nano_pretty".into(),
            version: "legacy".into(),
            parameters: Default::default(),
        });
        definitions.push(retired);

        let entries = definitions
            .into_iter()
            .map(|definition| CatalogEntry {
                definition,
                source: CatalogueSource::Project,
            })
            .collect::<Vec<_>>();
        let choices = group_choices(&entries);
        let visible = choices
            .iter()
            .flat_map(|group| &group.entries)
            .map(|entry| entry.definition.id.as_str())
            .collect::<Vec<_>>();
        assert!(!visible.iter().any(|id| {
            matches!(
                *id,
                "assembly-preset-mx-rgb-south-matrix-0/definition/switch"
                    | "ASSEMBLY-x/DEFINITION/switch"
                    | "A1B2C3D4-E5F6-A1B2-C3D4-E5F6A1B2C3D4/DEFINITION/switch"
            )
        }));
        assert!(
            choices
                .iter()
                .flat_map(|group| &group.entries)
                .all(|entry| {
                    entry
                        .definition
                        .generator
                        .as_ref()
                        .is_none_or(|generator| generator.source != "infused-kim/nice_nano_pretty")
                })
        );
    }
}
