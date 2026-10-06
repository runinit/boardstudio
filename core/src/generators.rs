//! Built-in footprint generators as Core sees them: the catalogue, parameter
//! schemas, normalization and drawings for the page, and the model references
//! of generated parts. The generators themselves live in `boardstudio-footprints`.
use std::collections::BTreeMap;

use boardstudio_footprints::definition::DefinitionInput;
use boardstudio_footprints::geometry::{Geometry, geometry};
use boardstudio_footprints::models;
use boardstudio_footprints::nets::NoNets;
use boardstudio_footprints::sexpr::Expr;
use boardstudio_footprints::types::{GeneratorRef, PartRef};
use boardstudio_footprints::{GeneratorError, bundled};
use serde::{Serialize, de::DeserializeOwned};

use crate::model::{Part, PartDefinition, PartModel};

/// Move a value between this crate's document types and the footprints crate's
/// wire types, which share their serialized form.
pub(crate) fn convert<T: Serialize, U: DeserializeOwned>(value: &T) -> Result<U, String> {
    serde_json::from_value(serde_json::to_value(value).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn message(error: GeneratorError) -> String {
    error.message()
}

/// Whether `source` is a built-in generator.
pub fn is_generator(source: &str) -> bool {
    bundled().contains(source)
}

/// The generator's parameters keyed by name, each `{ "type", "value"? }`.
pub fn parameter_schema(source: &str) -> Result<BTreeMap<String, serde_json::Value>, String> {
    let schema = bundled().parameters(source).map_err(message)?;
    schema
        .into_iter()
        .map(|parameter| {
            let name = parameter.name.clone();
            let mut value = serde_json::to_value(&parameter).map_err(|error| error.to_string())?;
            value.as_object_mut().map(|object| object.remove("name"));
            Ok((name, value))
        })
        .collect()
}

/// Every built-in generator as a part definition.
pub fn catalogue() -> Result<Vec<PartDefinition>, String> {
    bundled()
        .catalogue()
        .map_err(message)?
        .iter()
        .map(convert)
        .collect()
}

/// Regenerate a generator-backed definition's pads, envelope and terminals.
/// Definitions without a generator are returned unchanged.
pub fn normalize_definition(mut definition: PartDefinition) -> Result<PartDefinition, String> {
    let input: DefinitionInput = convert(&definition)?;
    let Some(normalized) = bundled().normalize(&input).map_err(message)? else {
        return Ok(definition);
    };
    definition.pads = convert(&normalized.pads)?;
    definition.courtyard = convert(&normalized.courtyard)?;
    definition.keycap = convert(&normalized.keycap)?;
    definition.envelope_source = Some(convert(&normalized.envelope_source)?);
    definition.terminals = normalized.terminals;
    definition.envelope_notice = normalized.envelope_notice;
    Ok(definition)
}

fn generator_of(definition: &PartDefinition) -> Result<GeneratorRef, String> {
    convert(definition.generator.as_ref().ok_or_else(|| message(GeneratorError::MissingGenerator))?)
}

/// Render a definition (and optionally a placed part) with every net at index 0.
pub fn render_forms(definition: &PartDefinition, part: Option<&Part>) -> Result<Vec<Expr>, String> {
    let generator = generator_of(definition)?;
    let part: Option<PartRef> = part.map(convert).transpose()?;
    bundled()
        .render(&definition.id, &generator, part.as_ref(), &mut NoNets)
        .map_err(message)
}

/// The rendered footprint's pads and outline, for drawing it.
pub fn drawing(definition: &PartDefinition) -> Result<Geometry, String> {
    geometry(&render_forms(definition, None)?).map_err(message)
}

/// Model assets of a rendered part, in order.
pub fn model_asset_ids(definition: &PartDefinition, part: Option<&Part>) -> Result<Vec<String>, String> {
    models::model_asset_ids(&render_forms(definition, part)?).map_err(message)
}

/// Models of a rendered part with their placement.
pub fn model_bindings(definition: &PartDefinition, part: Option<&Part>) -> Result<Vec<PartModel>, String> {
    models::model_bindings(&render_forms(definition, part)?)
        .map_err(message)?
        .iter()
        .map(convert)
        .collect()
}

/// The asset an attached or bundled model path refers to, for each path.
pub fn model_asset_ids_for_paths(paths: &[String]) -> Vec<Option<String>> {
    let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
    models::model_asset_ids_for_paths(&paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_catalogue_converts_to_part_definitions() {
        let catalogue = catalogue().unwrap();
        assert_eq!(catalogue.len(), 36);
        assert!(catalogue.iter().all(|definition| !definition.pads.is_empty() || definition.kind == crate::model::PartKind::Utility));
        assert!(parameter_schema("ceoloide/switch_mx").unwrap().contains_key("include_keycap"));
        assert!(parameter_schema("ceoloide/missing").is_err());
    }

    #[test]
    fn normalization_regenerates_pads_and_keeps_saved_ids() {
        let mut definition = catalogue().unwrap().into_iter().find(|d| d.id == "ergogen:ceoloide/switch_mx").unwrap();
        definition.pads[0].id = "saved".into();
        definition.generator.as_mut().unwrap().parameters.insert("hotswap".into(), serde_json::json!(false));
        let normalized = normalize_definition(definition).unwrap();
        assert_eq!(normalized.pads[0].id, "saved");
        assert!(normalized.terminals.contains_key("from"));
    }
}
