//! Built-in footprint generators as Core sees them: the catalogue, parameter
//! schemas, normalization and drawings for the page, and the model references
//! of generated parts. The generators themselves live in `boardstudio-footprints`.
use std::collections::BTreeMap;

use boardstudio_footprints::params::{ParamKind, ParameterProblem};
use serde_json::Value;

pub use boardstudio_footprints::definition::DEFINITION_ID_PREFIX;
use boardstudio_footprints::definition::DefinitionInput;
use boardstudio_footprints::geometry::{Geometry, geometry};
use boardstudio_footprints::models;
pub use boardstudio_footprints::models::BUNDLED_MODEL_PREFIX;
use boardstudio_footprints::nets::NoNets;
pub use boardstudio_footprints::sexpr::{Expr, child, children, parse_forms};
use boardstudio_footprints::types::{GeneratorRef, PartRef};
use boardstudio_footprints::{GeneratorError, bundled};
use serde::{Serialize, de::DeserializeOwned};

use crate::model::{Finding, Part, PartDefinition, PartModel, ProjectDoc, Scope, Severity};

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
    convert(
        definition
            .generator
            .as_ref()
            .ok_or_else(|| message(GeneratorError::MissingGenerator))?,
    )
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

/// Where a generator's saved parameters are held.
#[derive(Clone, Debug, PartialEq)]
pub enum ParameterOwner {
    /// A generator-backed definition's own parameters.
    Definition(String),
    /// A placed part's own parameters, applied over its definition's at render.
    Part(String),
    /// An assembly member's parameters.
    Member { assembly: String, member: String },
}

/// A saved generator parameter that the shared check rejects, with where it is held.
#[derive(Clone, Debug, PartialEq)]
pub struct GeneratorParameterProblem {
    pub owner: ParameterOwner,
    pub generator: String,
    pub problem: ParameterProblem,
}

/// Every saved generator parameter of a document that its declared schema rejects.
///
/// Each map is checked where it is saved: a definition's own parameters at the definition, a
/// part's at the part, a member's at the member. A value is never checked twice, and a
/// definition's value that a part overrides is still checked, because it is stored.
pub fn parameter_problems(doc: &ProjectDoc) -> Vec<GeneratorParameterProblem> {
    let sources: BTreeMap<&str, &str> = doc
        .definitions
        .iter()
        .filter_map(|definition| {
            definition
                .generator
                .as_ref()
                .map(|generator| (definition.id.as_str(), generator.source.as_str()))
        })
        .collect();
    let mut problems = Vec::new();
    let mut check = |owner: ParameterOwner, source: &str, parameters: &BTreeMap<String, Value>| {
        // Unknown generators are reported where they fail to render, not here.
        let Ok(found) = bundled().check_parameters(source, parameters) else {
            return;
        };
        problems.extend(found.into_iter().map(|problem| GeneratorParameterProblem {
            owner: owner.clone(),
            generator: source.to_owned(),
            problem,
        }));
    };
    for definition in &doc.definitions {
        if let Some(generator) = &definition.generator {
            check(
                ParameterOwner::Definition(definition.id.clone()),
                &generator.source,
                &generator.parameters,
            );
        }
    }
    for part in &doc.parts {
        let (Some(source), Some(parameters)) = (
            sources.get(part.definition_id.as_str()),
            part.generator_parameters.as_ref(),
        ) else {
            continue;
        };
        check(ParameterOwner::Part(part.id.clone()), source, parameters);
    }
    for assembly in &doc.assemblies {
        for member in &assembly.members {
            let (Some(source), Some(parameters)) = (
                member
                    .definition_id
                    .as_deref()
                    .and_then(|id| sources.get(id)),
                member.parameters.as_ref(),
            ) else {
                continue;
            };
            check(
                ParameterOwner::Member {
                    assembly: assembly.id.clone(),
                    member: member.id.clone(),
                },
                source,
                parameters,
            );
        }
    }
    problems
}

/// A non-empty text value for a list parameter. Saved data can hold it (version 4 keeps it as
/// text), so Open and ReplaceDocument warn instead of refusing.
fn is_list_text(problem: &ParameterProblem) -> bool {
    matches!(
        problem,
        ParameterProblem::WrongType {
            expected: ParamKind::Array,
            found: Value::String(text),
            ..
        } if !text.trim().is_empty()
    )
}

fn owner_text(owner: &ParameterOwner) -> String {
    match owner {
        ParameterOwner::Definition(id) => format!("definition {id}"),
        ParameterOwner::Part(id) => format!("part {id}"),
        ParameterOwner::Member { assembly, member } => {
            format!("assembly {assembly} member {member}")
        }
    }
}

fn owner_key(owner: &ParameterOwner) -> String {
    match owner {
        ParameterOwner::Definition(id) => format!("definition:{id}"),
        ParameterOwner::Part(id) => format!("part:{id}"),
        ParameterOwner::Member { assembly, member } => format!("member:{assembly}/{member}"),
    }
}

fn owner_targets(owner: &ParameterOwner) -> Vec<String> {
    match owner {
        ParameterOwner::Definition(id) | ParameterOwner::Part(id) => vec![id.clone()],
        ParameterOwner::Member { assembly, member } => vec![assembly.clone(), member.clone()],
    }
}

fn problem_message(problem: &GeneratorParameterProblem) -> String {
    match &problem.problem {
        ParameterProblem::WrongType {
            parameter,
            expected,
            ..
        } => format!(
            "Generator {} parameter {parameter} on {} must be {}.",
            problem.generator,
            owner_text(&problem.owner),
            expected.describe()
        ),
        ParameterProblem::Undeclared { parameter, .. } => format!(
            "Generator {} has no parameter named {parameter} on {}; render ignores it.",
            problem.generator,
            owner_text(&problem.owner)
        ),
    }
}

/// Refuses a document whose saved generator parameters have a wrong type. List text and
/// undeclared names do not refuse; they are warnings from [`parameter_findings`].
pub fn document_error(doc: &ProjectDoc) -> Result<(), String> {
    parameter_problems(doc)
        .iter()
        .find(|problem| {
            matches!(problem.problem, ParameterProblem::WrongType { .. })
                && !is_list_text(&problem.problem)
        })
        .map_or(Ok(()), |problem| Err(problem_message(problem)))
}

/// Refuses an edit that writes a wrong type or an undeclared name. `before` is the problem list
/// of the document the edit starts from, so only a value the edit introduces or changes is
/// judged: a legacy warning does not block an unrelated edit.
pub fn introduced_error(
    before: &[GeneratorParameterProblem],
    after: &ProjectDoc,
) -> Result<(), String> {
    parameter_problems(after)
        .iter()
        .find(|problem| !before.contains(problem))
        .map_or(Ok(()), |problem| Err(problem_message(problem)))
}

/// Refuses a definition whose own generator parameters the schema rejects at all, which is the
/// strictest form: an edit submitted from the Parts editor must pass it.
pub fn definition_error(definition: &PartDefinition) -> Result<(), String> {
    let Some(generator) = &definition.generator else {
        return Ok(());
    };
    let found = bundled()
        .check_parameters(&generator.source, &generator.parameters)
        .map_err(message)?;
    match found.into_iter().next() {
        None => Ok(()),
        Some(problem) => Err(problem_message(&GeneratorParameterProblem {
            owner: ParameterOwner::Definition(definition.id.clone()),
            generator: generator.source.clone(),
            problem,
        })),
    }
}

/// Findings for every saved generator parameter the schema rejects: wrong types are errors,
/// undeclared names and list text are warnings, each located at its definition, part or member.
pub fn parameter_findings(doc: &ProjectDoc) -> Vec<Finding> {
    parameter_problems(doc)
        .iter()
        .map(|problem| {
            let (kind, severity, message) = match &problem.problem {
                ParameterProblem::Undeclared { .. } => {
                    ("undeclared", Severity::Warning, problem_message(problem))
                }
                ParameterProblem::WrongType { .. } if is_list_text(&problem.problem) => (
                    "list-text",
                    Severity::Warning,
                    format!(
                        "{} The saved value is text, so the part does not generate until it is a list.",
                        problem_message(problem)
                    ),
                ),
                ParameterProblem::WrongType { .. } => {
                    ("wrong-type", Severity::Error, problem_message(problem))
                }
            };
            let parameter = match &problem.problem {
                ParameterProblem::WrongType { parameter, .. }
                | ParameterProblem::Undeclared { parameter, .. } => parameter,
            };
            Finding {
                id: format!(
                    "generator-parameter:{}:{parameter}:{kind}",
                    owner_key(&problem.owner)
                ),
                severity,
                scope: Scope::Pcb,
                message,
                target_ids: owner_targets(&problem.owner),
            }
        })
        .collect()
}

/// Model assets of a rendered part, in order.
pub fn model_asset_ids(
    definition: &PartDefinition,
    part: Option<&Part>,
) -> Result<Vec<String>, String> {
    models::model_asset_ids(&render_forms(definition, part)?).map_err(message)
}

/// Models of a rendered part with their placement.
pub fn model_bindings(
    definition: &PartDefinition,
    part: Option<&Part>,
) -> Result<Vec<PartModel>, String> {
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
        assert!(
            catalogue
                .iter()
                .all(|definition| !definition.pads.is_empty()
                    || definition.kind == crate::model::PartKind::Utility)
        );
        assert!(
            parameter_schema("ceoloide/switch_mx")
                .unwrap()
                .contains_key("include_keycap")
        );
        assert!(parameter_schema("ceoloide/missing").is_err());
    }

    #[test]
    fn normalization_regenerates_pads_and_keeps_saved_ids() {
        let mut definition = catalogue()
            .unwrap()
            .into_iter()
            .find(|d| d.id == "generator:ceoloide/switch_mx")
            .unwrap();
        definition.pads[0].id = "saved".into();
        definition
            .generator
            .as_mut()
            .unwrap()
            .parameters
            .insert("hotswap".into(), serde_json::json!(false));
        let normalized = normalize_definition(definition).unwrap();
        assert_eq!(normalized.pads[0].id, "saved");
        assert!(normalized.terminals.contains_key("from"));
    }
}
