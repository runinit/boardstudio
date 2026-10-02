use boardstudio_core::model::ProjectDoc;
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
    pub definition: Rc<PartDefinition>,
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

fn imported_parts_hash() -> String {
    Sha256::digest(IMPORTED_PARTS_JSON.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

pub(super) async fn load_bundled(reversible: bool) -> Result<Rc<Vec<CatalogEntry>>, String> {
    let module = load_ergogen_module().await?;
    let imported_source_hash = imported_parts_hash();
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
                    definition: Rc::new(definition.clone()),
                    source,
                };
            } else {
                positions.insert(definition.id.clone(), entries.len());
                entries.push(CatalogEntry {
                    definition: Rc::new(definition.clone()),
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
                definition: Rc::new(definition.clone()),
                source: CatalogueSource::Project,
            };
        } else {
            positions.insert(definition.id.clone(), entries.len());
            entries.push(CatalogEntry {
                definition: Rc::new(definition.clone()),
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
    import_ergogen_module(&url).await
}

async fn import_ergogen_module(url: &str) -> Result<JsValue, String> {
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
    definition: PartDefinition,
    reversible: bool,
) -> Result<PartDefinition, String> {
    construction_definition_with_support(module, definition, reversible)
        .map(|(definition, _)| definition)
}

fn construction_definition_with_support(
    module: &JsValue,
    mut definition: PartDefinition,
    reversible: bool,
) -> Result<(PartDefinition, bool), String> {
    let Some(generator) = definition.generator.as_mut() else {
        return Ok((definition, false));
    };
    let source = generator.source.clone();
    let is_ergogen = function(module, "isErgogen")?
        .call1(module, &source.clone().into())
        .map_err(js_error)?
        .as_bool()
        .unwrap_or(false);
    if !is_ergogen {
        return Ok((definition, false));
    }
    let parameter_schema = function(module, "parameters")?
        .call1(module, &source.clone().into())
        .map_err(js_error)?;
    if !js_sys::Reflect::has(&parameter_schema, &"reversible".into()).map_err(js_error)? {
        return Ok((definition, false));
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
    let normalized = serde_wasm_bindgen::from_value(normalized)
        .map_err(|error| format!("Could not decode normalized {source} definition: {error}"))?;
    Ok((normalized, true))
}

/// Prepare a project setup proposal through catalogue-owned package helpers. The normalizer
/// implementation and its JS module remain private to this module; callers receive proposals,
/// not access to the catalogue module or its member functions.
pub(super) fn prepare_physical_setup_proposal(
    accepted: &ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
    module: Option<&JsValue>,
) -> Result<ProjectDoc, String> {
    use crate::physical_setup::SetupIntent;

    if matches!(intent, SetupIntent::ReversibleLayout(_)) {
        let module = module
            .ok_or_else(|| "The packaged construction normalizer is unavailable.".to_string())?;
        return prepare_physical_setup_proposal_with_module(accepted, intent, module);
    }
    crate::physical_setup::propose(accepted, intent, |definition, _| {
        Ok((definition.clone(), false))
    })
}

pub(super) async fn prepare_physical_setup_proposal_from_package(
    accepted: &ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
) -> Result<ProjectDoc, String> {
    if matches!(
        intent,
        crate::physical_setup::SetupIntent::ReversibleLayout(_)
    ) {
        let module = load_ergogen_module().await?;
        return prepare_physical_setup_proposal(accepted, intent, Some(&module));
    }
    prepare_physical_setup_proposal(accepted, intent, None)
}

fn prepare_physical_setup_proposal_with_module(
    accepted: &ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
    module: &JsValue,
) -> Result<ProjectDoc, String> {
    use crate::physical_setup::SetupIntent;

    match intent {
        SetupIntent::ReversibleLayout(enabled) => crate::physical_setup::propose(
            accepted,
            SetupIntent::ReversibleLayout(enabled),
            |definition, enabled| {
                construction_definition_with_support(module, definition.clone(), enabled)
            },
        ),
        intent => crate::physical_setup::propose(accepted, intent, |definition, _| {
            Ok((definition.clone(), false))
        }),
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod wasm_tests {
    use super::*;
    use crate::physical_setup::SetupIntent;
    use boardstudio_core::model::HardwareConfiguration;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    async fn reversible_proposal_uses_the_packaged_gateron_normalizer() {
        let module_url = match option_env!("BOARDSTUDIO_TEST_LAYOUT_GENERATOR_MODULE_URL") {
            Some(url) => url,
            None => panic!(
                "run scripts/web/test-physical-setup-proposal.mjs to provide packaged module URL"
            ),
        };
        let module = import_ergogen_module(module_url)
            .await
            .expect("import generated layout-generator asset");
        let catalogue = call_catalogue(&module).unwrap();
        let gateron = catalogue
            .into_iter()
            .find(|definition| {
                definition.generator.as_ref().is_some_and(|generator| {
                    generator.source == "ceoloide/switch_gateron_ks27_ks33"
                })
            })
            .expect("packaged catalogue contains Gateron KS27/KS33");
        let original = gateron.clone();
        let mut document = ProjectDoc::empty("packaged-test", "Packaged module test");
        document.definitions = vec![gateron.clone()];
        document.parts.push(
            serde_json::from_value(json!({
                "id":"gateron-1","definitionId":gateron.id,"reference":"SW1",
                "pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front",
                "generatorParameters":{"reversible":false,"custom":"keep"}
            }))
            .unwrap(),
        );
        document.hardware = Some(HardwareConfiguration::default());
        let accepted = document.clone();

        let proposal = super::super::physical_setup::prepare_proposal_with_module(
            &document,
            SetupIntent::ReversibleLayout(true),
            &module,
        )
        .expect("prepare reversible project proposal");

        assert_eq!(
            document, accepted,
            "proposal does not mutate accepted input"
        );
        assert_eq!(
            document.definitions[0], original,
            "source definition remains intact"
        );
        let generator = proposal.definitions[0].generator.as_ref().unwrap();
        assert_eq!(generator.parameters.get("reversible"), Some(&json!(true)));
        assert_eq!(generator.parameters.get("hotswap"), Some(&json!(false)));
        assert_eq!(generator.parameters.get("solder"), Some(&json!(true)));
        let part = &proposal.parts[0];
        assert_eq!(
            part.generator_parameters
                .as_ref()
                .unwrap()
                .get("reversible"),
            Some(&json!(true))
        );
        assert_eq!(
            part.generator_parameters.as_ref().unwrap().get("custom"),
            Some(&json!("keep"))
        );
        assert_eq!(
            proposal.parameters.get("reversibleLayout"),
            Some(&json!(true))
        );
    }
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
    const ASSEMBLY_PREFIX: &str = "assembly-";
    const MARKER: &[u8] = b"/definition/";
    let id = definition.id.as_str();
    let bytes = id.as_bytes();
    let has_assembly_prefix = id
        .get(..ASSEMBLY_PREFIX.len())
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case(ASSEMBLY_PREFIX));
    if has_assembly_prefix
        && bytes
            .windows(MARKER.len())
            .enumerate()
            .any(|(position, candidate)| {
                position > ASSEMBLY_PREFIX.len()
                    && candidate.eq_ignore_ascii_case(MARKER)
                    && id
                        .get(ASSEMBLY_PREFIX.len()..position)
                        .is_some_and(js_regex_dot_matches)
            })
    {
        return true;
    }
    bytes
        .windows(MARKER.len())
        .enumerate()
        .any(|(position, candidate)| {
            position == 36
                && candidate.eq_ignore_ascii_case(MARKER)
                && id.get(..position).is_some_and(is_uuid)
        })
}

fn js_regex_dot_matches(text: &str) -> bool {
    !text
        .chars()
        .any(|character| matches!(character, '\n' | '\r' | '\u{2028}' | '\u{2029}'))
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
    fn imported_source_hash_matches_the_bundled_source_digest() {
        assert_eq!(
            imported_parts_hash(),
            "000f4ba13114305c33e1378806c25840d903fa335da559b88c9d9404731acd7f"
        );
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
    fn overlay_and_selected_handle_share_untouched_bundled_definitions() {
        let definitions = imported_definitions();
        let mut overridden = definitions[0].clone();
        overridden.name = "Bundled value".into();
        let untouched = definitions[1].clone();
        let bundled = merge_bundled_sources(&[overridden.clone(), untouched], &[]);
        let untouched_definition = Rc::clone(&bundled[1].definition);

        let mut project_override = overridden;
        project_override.name = "Project value".into();
        let merged = merge_project_overrides(&bundled, &[project_override]);
        assert!(Rc::ptr_eq(&untouched_definition, &merged[1].definition));
        let selected_handle = merged[1].clone();
        assert!(Rc::ptr_eq(
            &untouched_definition,
            &selected_handle.definition
        ));
    }

    #[test]
    fn category_remains_searchable_alongside_special_search_aliases() {
        let mut led = imported_definitions().remove(0);
        led.id = "ergogen:ceoloide/led_sk6812mini-e".into();
        led.kind = PartKind::Passive;
        let entry = CatalogEntry {
            definition: Rc::new(led),
            source: CatalogueSource::Ergogen,
        };
        assert!(entry.matches("passives & leds", "Passives & LEDs"));
        assert!(!entry.matches_library_search("passives & leds"));
        assert!(entry.matches("rgb led reverse mount", "Passives & LEDs"));
        assert!(entry.matches_library_search("rgb led reverse mount"));

        let mut switch = (*entry.definition).clone();
        switch.id = "ergogen:ceoloide/switch_mx".into();
        switch.kind = PartKind::Switch;
        let entry = CatalogEntry {
            definition: Rc::new(switch),
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
    fn assembly_dot_matches_javascript_line_terminator_rules() {
        for terminator in ["\n", "\r", "\u{2028}", "\u{2029}"] {
            assert!(!js_regex_dot_matches(&format!("before{terminator}after")));
        }
        assert!(js_regex_dot_matches("parent/child"));
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
        let mut nested_snapshot = definitions[0].clone();
        nested_snapshot.id = "ASSEMBLY-parent/child/DEFINITION/switch".into();
        nested_snapshot.kicad_source = None;
        nested_snapshot.generator = None;
        definitions.push(nested_snapshot);
        let mut later_marker_snapshot = definitions[0].clone();
        later_marker_snapshot.id = "assembly-/definition/x/definition/switch".into();
        later_marker_snapshot.kicad_source = None;
        later_marker_snapshot.generator = None;
        definitions.push(later_marker_snapshot);
        let mut line_terminator_snapshot = definitions[0].clone();
        line_terminator_snapshot.id = "assembly-parent\nchild/definition/switch".into();
        line_terminator_snapshot.kicad_source = None;
        line_terminator_snapshot.generator = None;
        definitions.push(line_terminator_snapshot);
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
                definition: Rc::new(definition),
                source: CatalogueSource::Project,
            })
            .collect::<Vec<_>>();
        let choices = group_choices(&entries);
        let visible = choices
            .iter()
            .flat_map(|group| &group.entries)
            .map(|entry| entry.definition.id.as_str())
            .collect::<Vec<_>>();
        assert!(visible.contains(&"assembly-parent\nchild/definition/switch"));
        assert!(!visible.iter().any(|id| {
            matches!(
                *id,
                "assembly-preset-mx-rgb-south-matrix-0/definition/switch"
                    | "ASSEMBLY-x/DEFINITION/switch"
                    | "ASSEMBLY-parent/child/DEFINITION/switch"
                    | "assembly-/definition/x/definition/switch"
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
