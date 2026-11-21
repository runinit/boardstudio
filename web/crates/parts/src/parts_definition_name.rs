//! Parts-private admission and resolver construction for one definition-name commit.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution};
use boardstudio_model::model::EditOperation;

use crate::parts_custom_definition::{DEFINITION_GONE, GENERATOR_LOCKED, replacement_commit};

#[cfg(target_arch = "wasm32")]
#[path = "parts_definition_name/ui.rs"]
mod ui;

#[cfg(target_arch = "wasm32")]
pub use ui::DefinitionNameEditor;

/// Resolve a definition-name commit against the accepted document at execution: retire
/// when the definition has gone or is generated, resolve `Unchanged` when the accepted
/// name already equals the committed one, and otherwise submit a replacement built from
/// the accepted document, so a rename queued behind another edit never reverts it.
pub fn definition_name_resolver(definition_id: String, name: String) -> EditResolver {
    EditResolver::new(
        "parts-definition-name",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let Some(existing) = document
                .definitions
                .iter()
                .find(|definition| definition.id == definition_id)
            else {
                return Resolution::Retire(DEFINITION_GONE.into());
            };
            if existing.generator.is_some() {
                return Resolution::Retire(GENERATOR_LOCKED.into());
            }
            if existing.name == name {
                return Resolution::Unchanged;
            }
            let mut replacement = document.as_ref().clone();
            let Some(definition) = replacement
                .definitions
                .iter_mut()
                .find(|definition| definition.id == definition_id)
            else {
                return Resolution::Retire(DEFINITION_GONE.into());
            };
            definition.name = name.clone();
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![definition_id.clone()],
            )
        },
    )
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "parts_definition_name/tests.rs"]
mod tests;

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "parts_definition_name/mounted_tests.rs"]
mod mounted_tests;
