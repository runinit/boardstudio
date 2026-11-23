//! Parts-owned file import and acceptance reconciliation for source-owned KiCad footprints.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_model::model::{CompiledFootprint, EditOperation};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportCapture {
    scope: Scope,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    view_generation: u64,
    scope_generation: u64,
    selection: Option<(Option<Scope>, String)>,
    definition_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportOwner {
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    view_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
}

impl ImportOwner {
    fn new(
        current: &AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Option<(Option<Scope>, String)>,
        view_generation: u64,
        scope_generation: u64,
        workspace: &'static str,
    ) -> Self {
        Self {
            scope,
            selection,
            session_epoch: current.session_epoch,
            document_id: current.document.id.clone(),
            view_generation,
            scope_generation,
            workspace,
        }
    }
}

impl ImportCapture {
    fn new(
        snapshot: &AcceptedSnapshot,
        scope: Scope,
        view_generation: u64,
        scope_generation: u64,
        selection: Option<(Option<Scope>, String)>,
        definition_id: String,
    ) -> Self {
        Self {
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            view_generation,
            scope_generation,
            selection,
            definition_id,
        }
    }
}

fn owner_matches(capture: &ImportCapture, owner: &ImportOwner) -> bool {
    owner.scope.as_ref() == Some(&capture.scope)
        && owner.selection == capture.selection
        && owner.session_epoch == capture.session_epoch
        && owner.document_id == capture.document_id
        && capture.scope.document_id == capture.document_id
        && capture.scope.session_epoch == capture.session_epoch
        && owner.view_generation == capture.view_generation
        && owner.scope_generation == capture.scope_generation
        && owner.workspace == "Parts"
}

fn import_resolver(capture: ImportCapture, imported: CompiledFootprint) -> EditResolver {
    EditResolver::new(
        "parts-import-footprint",
        move |accepted: &AcceptedSnapshot| {
            if accepted.document.id != capture.document_id
                || accepted.session_epoch != capture.session_epoch
            {
                return Resolution::Retire("The Parts project changed during import.".into());
            }
            if imported.definition.id != capture.definition_id {
                return Resolution::Retire(
                    "The importer returned a different definition identity.".into(),
                );
            }
            let mut definition = imported.definition.clone();
            let mut suffix = 0_u64;
            while accepted
                .document
                .definitions
                .iter()
                .any(|existing| existing.id == definition.id)
            {
                suffix += 1;
                definition.id = format!("{}-{suffix}", capture.definition_id);
            }
            definition.pads = imported.geometry.pads.clone();
            definition.courtyard = imported.geometry.courtyard.clone();
            let id = definition.id.clone();
            let mut document = accepted.document.as_ref().clone();
            document.definitions.push(definition);
            crate::parts_custom_definition::replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
                vec![id],
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{ImportCapture, ImportOwner, owner_matches};
    use boardstudio_application::{Scope, SessionEpoch};

    #[test]
    fn import_feedback_is_limited_to_the_captured_owner_but_survives_revision_advance() {
        let scope = Scope {
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            board_id: "board-1".into(),
            instance_id: None,
        };
        let capture = ImportCapture {
            scope: scope.clone(),
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            view_generation: 3,
            scope_generation: 5,
            selection: Some((Some(scope.clone()), "existing-footprint".into())),
            definition_id: "imported-1".into(),
        };
        let owner = ImportOwner {
            scope: Some(scope.clone()),
            selection: capture.selection.clone(),
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            view_generation: 3,
            scope_generation: 5,
            workspace: "Parts",
        };

        // Persistence may advance the accepted revision before its failure is reported.
        // Feedback still belongs to this selection as long as the project/view owner holds.
        assert!(owner_matches(&capture, &owner));

        let replacement_selection = ImportOwner {
            selection: Some((Some(scope), "replacement-footprint".into())),
            ..owner
        };
        assert!(!owner_matches(&capture, &replacement_selection));
    }
}

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::*;
    use crate::{parts_new_component::new_definition_identity, runtime::Runtime};
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::Button,
        spinner::Spinner,
    };
    use dioxus::{dioxus_core::Task, prelude::*};
    use dioxus_web::WebEventExt;
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::HtmlInputElement;

    #[component]
    pub fn ImportKiCadFootprintAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<&'static str>,
        mut selected: Signal<Option<(Option<Scope>, String)>>,
        mut query: Signal<String>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let mut input = use_signal(|| None::<HtmlInputElement>);
        let mut pending = use_signal(|| None::<(Task, String)>);
        let mut feedback = use_signal(|| None::<(ImportOwner, Result<String, String>)>);
        let current_owner = {
            let runtime = runtime.clone();
            move || {
                Some(ImportOwner::new(
                    &runtime.model().accepted?,
                    runtime.scope(),
                    selected(),
                    view_generation(),
                    scope_generation(),
                    workspace(),
                ))
            }
        };
        let on_file = {
            let runtime = runtime.clone();
            let scope = scope.clone();
            let current_owner = current_owner.clone();
            move |event: FormEvent| {
                let Some(input) = event
                    .data()
                    .try_as_web_event()
                    .and_then(|event| event.target())
                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                else {
                    return;
                };
                let Some(file) = input.files().and_then(|files| files.get(0)) else {
                    return;
                };
                input.set_value("");
                let Some(owner) = current_owner() else { return };
                let Some(scope) = scope
                    .clone()
                    .filter(|scope| owner.scope.as_ref() == Some(scope))
                else {
                    return;
                };
                if pending.peek().is_some() || owner.workspace != "Parts" {
                    return;
                }
                let filename = file.name();
                if !filename.to_lowercase().ends_with(".kicad_mod") {
                    feedback.set(Some((
                        owner,
                        Err("Select a .kicad_mod footprint.".to_owned()),
                    )));
                    return;
                }
                let Some(snapshot) = runtime.model().accepted else {
                    return;
                };
                let capture = ImportCapture::new(
                    &snapshot,
                    scope,
                    view_generation(),
                    scope_generation(),
                    selected(),
                    format!("imported-{}", new_definition_identity()),
                );
                if !owner_matches(&capture, &owner) {
                    return;
                }
                let runtime = runtime.clone();
                let current_owner = current_owner.clone();
                feedback.set(None);
                let task = spawn(async move {
                    let still_current =
                        || current_owner().is_some_and(|owner| owner_matches(&capture, &owner));
                    let result = async {
                        let source = JsFuture::from(file.text())
                            .await
                            .map_err(|cause| {
                                format!("Could not read the selected footprint: {cause:?}")
                            })?
                            .as_string()
                            .ok_or("Could not read the footprint as text.")?;
                        if !still_current() {
                            return Ok(None);
                        }
                        let request = format!("parts-import-footprint-{}", runtime.operation().0);
                        let compiled = runtime.import_footprint(
                            request,
                            capture.definition_id.clone(),
                            source,
                        )?;
                        if !still_current() {
                            return Ok(None);
                        }
                        let notes = compiled
                            .diagnostics
                            .iter()
                            .map(|diagnostic| diagnostic.message.as_str())
                            .collect::<Vec<_>>()
                            .join(" ");
                        let accepted = runtime.commit(
                            "parts-import-footprint",
                            import_resolver(capture.clone(), compiled),
                        )?;
                        Ok(accepted
                            .document
                            .definitions
                            .last()
                            .map(|part| (part.id.clone(), notes)))
                    }
                    .await;
                    pending.set(None);
                    if !still_current() {
                        return;
                    }
                    match result {
                        Ok(Some((id, notes))) => {
                            selected.set(Some((Some(capture.scope), id)));
                            query.set(String::new());
                            if !notes.is_empty()
                                && let Some(owner) = current_owner()
                            {
                                feedback.set(Some((
                                    owner,
                                    Ok(format!("Imported with notes: {notes}")),
                                )));
                            }
                            on_select.call(());
                        }
                        Ok(None) => {}
                        Err(message) => feedback.set(Some((owner, Err(message)))),
                    }
                });
                pending.set(Some((task, filename)));
            }
        };
        let busy = pending().is_some();
        rsx! {
            Button { disabled: busy || scope.is_none(), onclick: move |_| if let Some(input) = input() { input.click(); }, "Import KiCad footprint" }
            input {
                r#type: "file", hidden: true, accept: ".kicad_mod", disabled: busy || scope.is_none(),
                aria_label: "Import KiCad footprint", onchange: on_file,
                onmounted: move |event: MountedEvent| {
                    input.set(event.data().downcast::<web_sys::Element>().and_then(|e| e.dyn_ref::<HtmlInputElement>()).cloned());
                },
            }
            if let Some((task, filename)) = pending() {
                div { role: "status", Spinner {} "Importing {filename}…" }
                Button { onclick: move |_| { task.cancel(); pending.set(None); }, "Cancel import" }
            }
            if let Some((_, message)) = feedback().filter(|(owner, _)| current_owner().as_ref() == Some(owner)) {
                match message {
                    Ok(notes) => rsx! { p { role: "status", "{notes}" } },
                    Err(error) => rsx! { Alert { variant: AlertVariant::Destructive, "{error}" } },
                }
            }
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub use ui::ImportKiCadFootprintAction;
