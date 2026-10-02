use super::catalogue;
use crate::physical_setup::SetupIntent;
use boardstudio_core::model::ProjectDoc;

/// Build a typed setup proposal through the retained, packaged Ergogen implementation.
/// This is private page-binary preparation only; it neither submits a Session edit nor
/// changes selection. The Editor owner is added in the dependent setup intent slice.
pub(super) fn prepare_proposal_with_module(
    accepted: &ProjectDoc,
    intent: SetupIntent,
    module: &wasm_bindgen::JsValue,
) -> Result<ProjectDoc, String> {
    catalogue::prepare_physical_setup_proposal(accepted, intent, module)
}
