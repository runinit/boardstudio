//! Parts-owned file import and acceptance reconciliation for source-owned KiCad footprints.

use boardstudio_application::{AcceptedSnapshot, EditResolver, Scope};
use boardstudio_model::model::CompiledFootprint;
use boardstudio_web_ui_model::Workspace;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportCapture {
    guard: crate::parts_custom_definition::PartsOwnerGuard,
    scope: Scope,
    view_generation: u64,
    definition_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportOwner {
    guard: crate::parts_custom_definition::PartsOwnerGuard,
    view_generation: u64,
    workspace: Workspace,
}

fn import_guard(
    snapshot: &AcceptedSnapshot,
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    scope_generation: u64,
) -> crate::parts_custom_definition::PartsOwnerGuard {
    crate::parts_custom_definition::PartsOwnerGuard::document(
        scope,
        snapshot.session_epoch,
        snapshot.document.id.clone(),
    )
    .with_selection(selection)
    .generations(None, Some(scope_generation))
}

impl ImportOwner {
    fn new(
        current: &AcceptedSnapshot,
        scope: Option<Scope>,
        selection: Option<(Option<Scope>, String)>,
        view_generation: u64,
        scope_generation: u64,
        workspace: Workspace,
    ) -> Self {
        Self {
            guard: import_guard(current, scope, selection, scope_generation),
            view_generation,
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
            guard: import_guard(snapshot, Some(scope.clone()), selection, scope_generation),
            scope,
            view_generation,
            definition_id,
        }
    }
}

fn owner_matches(capture: &ImportCapture, owner: &ImportOwner) -> bool {
    owner.guard == capture.guard
        && owner.view_generation == capture.view_generation
        && capture.scope.document_id == capture.guard.document_id
        && capture.scope.session_epoch == capture.guard.session_epoch
        && owner.workspace == Workspace::Parts
}

fn import_resolver(capture: ImportCapture, imported: CompiledFootprint) -> EditResolver {
    crate::parts_custom_definition::replace_document_resolver(
        "parts-import-footprint",
        move |accepted, replacement| {
            if accepted.document.id != capture.guard.document_id
                || accepted.session_epoch != capture.guard.session_epoch
            {
                return Err("The Parts project changed during import.".into());
            }
            if imported.definition.id != capture.definition_id {
                return Err("The importer returned a different definition identity.".into());
            }
            let mut definition = imported.definition.clone();
            let mut suffix = 0_u64;
            while replacement
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
            replacement.definitions.push(definition);
            Ok(vec![id])
        },
    )
}

#[cfg(test)]
mod tests {
    use super::{ImportCapture, ImportOwner, owner_matches};
    use boardstudio_application::{Scope, SessionEpoch};
    use boardstudio_web_ui_model::Workspace;

    #[test]
    fn import_feedback_is_limited_to_the_captured_owner_but_survives_revision_advance() {
        let scope = Scope {
            session_epoch: SessionEpoch(4),
            document_id: "project-1".into(),
            board_id: "board-1".into(),
            instance_id: None,
        };
        let guard_for = |id: &str| {
            crate::parts_custom_definition::PartsOwnerGuard::document(
                Some(scope.clone()),
                SessionEpoch(4),
                "project-1".into(),
            )
            .with_selection(Some((Some(scope.clone()), id.into())))
            .generations(None, Some(5))
        };
        let capture = ImportCapture {
            guard: guard_for("existing-footprint"),
            scope: scope.clone(),
            view_generation: 3,
            definition_id: "imported-1".into(),
        };
        let owner = ImportOwner {
            guard: guard_for("existing-footprint"),
            view_generation: 3,
            workspace: Workspace::Parts,
        };
        // Persistence may advance the accepted revision before its failure is reported.
        // Feedback still belongs to this selection as long as the project/view owner holds.
        assert!(owner_matches(&capture, &owner));
        let replacement = ImportOwner {
            guard: guard_for("replacement-footprint"),
            ..owner
        };
        assert!(!owner_matches(&capture, &replacement));
    }
}

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::*;
    use crate::{parts_new_component::new_definition_identity, runtime::Runtime};
    use boardstudio_web_ui_shared::inspector_fields::FilePickerButton;
    use boardstudio_web_ui_shared::ui::{
        alert::{Alert, AlertVariant},
        button::{Button, ButtonVariant},
        spinner::Spinner,
    };
    use dioxus::{dioxus_core::Task, prelude::*};
    use std::rc::Rc;
    use wasm_bindgen_futures::JsFuture;

    #[component]
    pub fn ImportKiCadFootprintAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<Workspace>,
        mut selected: Signal<Option<(Option<Scope>, String)>>,
        mut query: Signal<String>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
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
        let mut on_file = {
            let runtime = runtime.clone();
            let scope = scope.clone();
            let current_owner = current_owner.clone();
            move |file: web_sys::File| {
                let Some(owner) = current_owner() else { return };
                let Some(scope) = scope
                    .clone()
                    .filter(|scope| owner.guard.scope.as_ref() == Some(scope))
                else {
                    return;
                };
                if pending.peek().is_some() || owner.workspace != Workspace::Parts {
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
            FilePickerButton { label: "Import KiCad footprint", accept: ".kicad_mod",
                variant: ButtonVariant::Default, disabled: busy || scope.is_none(),
                on_files: move |files: Vec<web_sys::File>| if let Some(file) = files.into_iter().next() { on_file(file); },
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
