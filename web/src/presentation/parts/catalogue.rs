use boardstudio_core::generators;
use boardstudio_core::model::ProjectDoc;
use boardstudio_core::model::{PartDefinition, PartKind};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

const IMPORTED_PARTS_JSON: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../catalogue/parts/imported-parts.json"
));

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CatalogueSource {
    Generator,
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
    let imported_source_hash = imported_parts_hash();
    if let Some(entries) = BUNDLED_CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|cached| {
                cached.reversible == reversible
                    && cached.imported_source_hash == imported_source_hash
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
    let generated = generators::catalogue()?
        .into_iter()
        .map(|definition| construction_definition(definition, reversible))
        .collect::<Result<Vec<_>, _>>()?;
    let imported = imported
        .into_iter()
        .map(|definition| construction_definition(definition, reversible))
        .collect::<Result<Vec<_>, _>>()?;
    let entries = Rc::new(merge_bundled_sources(&generated, &imported));
    BUNDLED_CACHE.with(|cache| {
        cache.borrow_mut().push(CachedCatalogue {
            imported_source_hash,
            reversible,
            entries: entries.clone(),
        });
    });
    Ok(entries)
}

fn merge_bundled_sources(
    generated: &[PartDefinition],
    imported: &[PartDefinition],
) -> Vec<CatalogEntry> {
    let mut entries = Vec::with_capacity(generated.len() + imported.len());
    let mut positions = HashMap::<String, usize>::with_capacity(entries.capacity());
    for (definitions, source) in [
        (generated, CatalogueSource::Generator),
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
        "generator:ceoloide/switch_mx" => "MX switch",
        "generator:ceoloide/switch_choc_v1_v2" => "Choc V1 / V2 switch",
        "generator:ceoloide/switch_gateron_ks27_ks33" => "Gateron KS27 / KS33 switch",
        "generator:ceoloide/diode_tht_sod123" => "Matrix diode (SOD-123 / THT)",
        "generator:ceoloide/led_sk6812mini-e" => "SK6812 MINI-E",
        _ => &definition.name,
    }
}

/// Whether a source is a built-in generator. Definition IDs are project-owned identities and
/// are not a reliable proxy for generator membership.
pub(super) async fn is_generator_source(source: String) -> Result<bool, String> {
    Ok(generators::is_generator(&source))
}

pub(super) async fn generator_parameter_schema(
    source: &str,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, String> {
    if !generators::is_generator(source) {
        return Ok(Default::default());
    }
    generators::parameter_schema(source)
}

/// Normalize a matrix-owned generator clone with the built-in generators.
pub(super) async fn normalize_matrix_definition(
    definition: PartDefinition,
) -> Result<PartDefinition, String> {
    normalize_generator_definition(definition).await
}

/// Normalize a transient Parts generator candidate through the same service used for
/// construction. This does not edit the accepted document.
pub(super) async fn normalize_generator_definition(
    definition: PartDefinition,
) -> Result<PartDefinition, String> {
    let Some(generator) = definition.generator.as_ref() else {
        return Err(format!(
            "Matrix component {} has no generator definition.",
            definition.id
        ));
    };
    let source = generator.source.clone();
    if !generators::is_generator(&source) {
        return Err(format!(
            "Matrix component {} does not use a supported footprint generator.",
            definition.id
        ));
    }
    let id = definition.id.clone();
    let normalized = generators::normalize_definition(definition)
        .map_err(|error| format!("Could not normalize {source}: {error}"))?;
    if normalized.id != id
        || normalized
            .generator
            .as_ref()
            .map(|item| item.source.as_str())
            != Some(source.as_str())
    {
        return Err(format!(
            "Normalizing matrix component {id} changed its identity or generator source."
        ));
    }
    Ok(normalized)
}

fn construction_definition(
    definition: PartDefinition,
    reversible: bool,
) -> Result<PartDefinition, String> {
    construction_definition_with_support(definition, reversible).map(|(definition, _)| definition)
}

fn construction_definition_with_support(
    mut definition: PartDefinition,
    reversible: bool,
) -> Result<(PartDefinition, bool), String> {
    let Some(generator) = definition.generator.as_mut() else {
        return Ok((definition, false));
    };
    let source = generator.source.clone();
    if !generators::is_generator(&source)
        || !generators::parameter_schema(&source)?.contains_key("reversible")
    {
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

    let normalized = generators::normalize_definition(definition)
        .map_err(|error| format!("Could not normalize {source}: {error}"))?;
    Ok((normalized, true))
}

/// Prepare a project setup proposal through catalogue-owned helpers; callers receive
/// proposals, not access to the generators.
pub(super) fn prepare_physical_setup_proposal(
    accepted: &ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
) -> Result<ProjectDoc, String> {
    use crate::physical_setup::SetupIntent;

    match intent {
        SetupIntent::ReversibleLayout(enabled) => crate::physical_setup::propose(
            accepted,
            SetupIntent::ReversibleLayout(enabled),
            |definition, enabled| construction_definition_with_support(definition.clone(), enabled),
        ),
        intent => crate::physical_setup::propose(accepted, intent, |definition, _| {
            Ok((definition.clone(), false))
        }),
    }
}

pub(super) async fn prepare_physical_setup_proposal_from_package(
    accepted: &ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
) -> Result<ProjectDoc, String> {
    prepare_physical_setup_proposal(accepted, intent)
}

#[cfg(test)]
mod generator_tests {
    use super::*;
    use crate::physical_setup::SetupIntent;
    use boardstudio_core::model::HardwareConfiguration;
    use serde_json::json;

    fn bundled_definition(source: &str) -> PartDefinition {
        generators::catalogue()
            .unwrap()
            .into_iter()
            .find(|definition| {
                definition
                    .generator
                    .as_ref()
                    .is_some_and(|generator| generator.source == source)
            })
            .unwrap_or_else(|| panic!("catalogue contains {source}"))
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn generator_recognition_uses_source_membership() {
        assert!(generators::is_generator("ceoloide/trrs_pj320a"));
        assert!(!generators::is_generator("generator:not-a-generator"));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn reversible_proposal_uses_gateron_normalizer() {
        let gateron = bundled_definition("ceoloide/switch_gateron_ks27_ks33");
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

        let proposal =
            prepare_physical_setup_proposal(&document, SetupIntent::ReversibleLayout(true))
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

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn matrix_definition_clone_normalizes_through_the_generator() {
        let mut matrix_switch = bundled_definition("ceoloide/switch_mx");
        matrix_switch.id = "assembly-preset-mx-solder-south-matrix-test-0/definition/switch".into();
        let generator = matrix_switch.generator.as_mut().unwrap();
        generator.parameters.insert("hotswap".into(), json!(true));
        generator.parameters.insert("solder".into(), json!(false));
        generator
            .parameters
            .insert("include_keycap".into(), json!(true));
        generator.parameters.insert("side".into(), json!("B"));
        let normalized = futures_lite_block_on(normalize_matrix_definition(matrix_switch))
            .expect("matrix definition clone normalizes");
        assert_eq!(
            normalized.id,
            "assembly-preset-mx-solder-south-matrix-test-0/definition/switch"
        );
        let generator = normalized.generator.as_ref().unwrap();
        assert_eq!(generator.parameters.get("hotswap"), Some(&json!(true)));
        assert_eq!(generator.parameters.get("solder"), Some(&json!(false)));
        assert_eq!(
            generator.parameters.get("include_keycap"),
            Some(&json!(true))
        );
        assert!(!normalized.pads.is_empty());
        assert!(!normalized.courtyard.is_empty());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn binding_schema_uses_generator_parameters() {
        let schema = generators::parameter_schema("infused-kim/smd_0805")
            .expect("read generic-part parameter schema");
        assert_eq!(schema["net_1_from"]["type"], "net");
        assert_eq!(schema["net_1_from"]["value"], "SMD_1_F");
        assert_eq!(schema["net_1_to"]["value"], "SMD_1_T");
        assert_eq!(schema["net_6_to"]["type"], "net");
        assert!(schema.values().all(|parameter| {
            matches!(
                parameter["type"].as_str(),
                Some("string" | "number" | "boolean" | "array" | "object" | "net" | "anchor")
            )
        }));
    }

    /// The async wrappers do no asynchronous work, so one poll completes them.
    fn futures_lite_block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(value) => value,
            std::task::Poll::Pending => panic!("generator wrapper suspended"),
        }
    }
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
        "generator:ceoloide/led_sk6812mini-e" => "RGB LED reverse mount",
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
        && !is_vik_part(&entry.definition)
}

fn is_vik_part(definition: &PartDefinition) -> bool {
    // Retain imported definitions for saved-project resolution, but do not offer
    // their unfinished VIK connector/embedded-circuit parts as new v1 choices.
    let id = definition.id.to_ascii_lowercase();
    id.starts_with("vik:")
        || id.contains("/definition/vik:")
        || definition
            .hardware_profile
            .as_ref()
            .is_some_and(|profile| profile.vik_role.is_some())
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
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn saved_vik_parts_are_not_new_catalogue_choices() {
        let ordinary = imported_definitions().remove(0);
        let mut source_connector = ordinary.clone();
        source_connector.id = "vik:source:horizontal-host-connector".into();
        let mut embedded = ordinary.clone();
        embedded.id = "embedded/review/definition/vik:haptic/component/5".into();
        let entries = [ordinary, source_connector, embedded]
            .into_iter()
            .map(|definition| CatalogEntry {
                definition: Rc::new(definition),
                source: CatalogueSource::Project,
            })
            .collect::<Vec<_>>();

        let choices = catalogue_choices(&entries);
        assert_eq!(choices.len(), 1);
        assert_eq!(choices[0].definition.id, entries[0].definition.id);
    }

    fn imported_definitions() -> Vec<PartDefinition> {
        let catalogue: ImportedCatalogue =
            serde_json::from_str(IMPORTED_PARTS_JSON).expect("bundled imported definitions decode");
        catalogue
            .parts
            .into_iter()
            .map(|part| part.definition)
            .collect()
    }

    #[wasm_bindgen_test]
    fn imported_source_hash_matches_the_bundled_source_digest() {
        assert_eq!(
            imported_parts_hash(),
            "179742117905b66a465c54d454ee5d49b0622e62edcc532c5502ae18aeb459e1"
        );
    }

    #[wasm_bindgen_test]
    fn every_imported_static_definition_decodes_as_the_existing_core_type() {
        let definitions = imported_definitions();
        assert_eq!(definitions.len(), 9);
        assert!(
            definitions
                .iter()
                .all(|definition| definition.kicad_source.is_some())
        );
    }

    #[wasm_bindgen_test]
    fn imported_and_project_overrides_keep_the_first_id_position() {
        let actual = imported_definitions().remove(0);
        let id = actual.id.clone();
        let mut generated = actual.clone();
        generated.name = "Generator value".into();
        let mut imported = actual.clone();
        imported.name = "Imported value".into();
        let bundled = merge_bundled_sources(&[generated], &[imported]);
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

    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
    fn category_remains_searchable_alongside_special_search_aliases() {
        let mut led = imported_definitions().remove(0);
        led.id = "generator:ceoloide/led_sk6812mini-e".into();
        led.kind = PartKind::Passive;
        let entry = CatalogEntry {
            definition: Rc::new(led),
            source: CatalogueSource::Generator,
        };
        assert!(entry.matches("passives & leds", "Passives & LEDs"));
        assert!(!entry.matches_library_search("passives & leds"));
        assert!(entry.matches("rgb led reverse mount", "Passives & LEDs"));
        assert!(entry.matches_library_search("rgb led reverse mount"));

        let mut switch = (*entry.definition).clone();
        switch.id = "generator:ceoloide/switch_mx".into();
        switch.kind = PartKind::Switch;
        let entry = CatalogEntry {
            definition: Rc::new(switch),
            source: CatalogueSource::Generator,
        };
        assert!(entry.matches("switches", "Switches"));
        assert!(!entry.matches_library_search("switches"));
        assert!(entry.matches("solder hotswap", "Switches"));
        assert!(entry.matches_library_search("solder hotswap"));
    }

    #[wasm_bindgen_test]
    fn assembly_dot_matches_javascript_line_terminator_rules() {
        for terminator in ["\n", "\r", "\u{2028}", "\u{2029}"] {
            assert!(!js_regex_dot_matches(&format!("before{terminator}after")));
        }
        assert!(js_regex_dot_matches("parent/child"));
    }

    #[wasm_bindgen_test]
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
