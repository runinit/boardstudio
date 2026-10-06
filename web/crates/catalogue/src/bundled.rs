//! Entry points into the bundled catalogue for Parts, Layout, PCB and Case.
use crate::catalogue;
use boardstudio_core::model::ProjectDoc;
use std::rc::Rc;

pub async fn load_mounting_hole_definition()
-> Result<Rc<boardstudio_core::model::PartDefinition>, String> {
    let entries = catalogue::load_bundled(false).await?;
    let mut matches = entries.iter().filter(|entry| {
        entry.source == catalogue::CatalogueSource::Generator
            && entry
                .definition
                .generator
                .as_ref()
                .is_some_and(|generator| generator.source == "ceoloide/mounting_hole_npth")
    });
    let definition = matches
        .next()
        .ok_or_else(|| "Bundled mounting-hole template is unavailable.".to_string())?
        .definition
        .clone();
    if matches.next().is_some() {
        return Err("Bundled mounting-hole template is ambiguous.".into());
    }
    Ok(definition)
}

/// Narrow Parts-owned bridge for the Editor's physical-setup intent owner. Package loading and
/// metadata normalization stay private to the catalogue implementation.
pub async fn prepare_physical_setup_proposal(
    accepted: ProjectDoc,
    intent: crate::physical_setup::SetupIntent,
) -> Result<ProjectDoc, String> {
    catalogue::prepare_physical_setup_proposal_from_package(&accepted, intent).await
}

/// Normalize a matrix-owned clone through the same built-in generator path used by Parts.
pub async fn normalize_matrix_definition(
    definition: boardstudio_core::model::PartDefinition,
) -> Result<boardstudio_core::model::PartDefinition, String> {
    catalogue::normalize_matrix_definition(definition).await
}

pub async fn is_generator_source(source: String) -> Result<bool, String> {
    catalogue::is_generator_source(source).await
}

/// Read the parameter descriptors from the built-in generators. The Parts catalogue remains
/// the sole owner of module loading; PCB receives only the accepted package schema values.
pub async fn generator_parameter_schema(
    source: String,
) -> Result<std::collections::BTreeMap<String, serde_json::Value>, String> {
    catalogue::generator_parameter_schema(&source).await
}

pub async fn load_matrix_templates(
    reversible: bool,
) -> Result<Vec<boardstudio_core::model::PartDefinition>, String> {
    let entries = catalogue::load_bundled(reversible).await?;
    Ok(entries
        .iter()
        .filter(|entry| entry.source == catalogue::CatalogueSource::Generator)
        .map(|entry| (*entry.definition).clone())
        .collect())
}

/// Every bundled catalogue definition, for key-level assembly and attached-component choices.
pub async fn load_all_catalogue_definitions(
    reversible: bool,
) -> Result<Vec<boardstudio_core::model::PartDefinition>, String> {
    let entries = catalogue::load_bundled(reversible).await?;
    Ok(entries
        .iter()
        .map(|entry| (*entry.definition).clone())
        .collect())
}

/// Resolve any selectable item from the construction-normalized bundled catalogue, then apply
/// the accepted project definition with the same precedence as the Parts browser.
pub async fn load_component_definition(
    document: &ProjectDoc,
    definition_id: &str,
) -> Result<boardstudio_core::model::PartDefinition, String> {
    let reversible = reversible_layout(document);
    let bundled = catalogue::load_bundled(reversible).await?;
    let entries = catalogue::merge_project_overrides(&bundled, &document.definitions);
    let definition = entries
        .iter()
        .find(|entry| entry.definition.id == definition_id)
        .ok_or_else(|| "The selected component is no longer in the catalogue.".to_string())?;
    Ok((*definition.definition).clone())
}

pub fn reversible_layout(document: &ProjectDoc) -> bool {
    match document.parameters.get("reversibleLayout") {
        Some(serde_json::Value::Bool(reversible)) => *reversible,
        _ => document
            .hardware
            .as_ref()
            .is_some_and(|hardware| hardware.instances.iter().any(|instance| instance.flipped)),
    }
}
