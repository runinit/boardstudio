//! Parts-owned file import and acceptance reconciliation for source-owned KiCad footprints.

use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, SnapshotToken};
use boardstudio_core::model::{CompiledFootprint, EditCommand, EditOperation, EditPhase};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ImportCapture {
    scope: Scope,
    session_epoch: boardstudio_application::SessionEpoch,
    document_id: String,
    snapshot_token: SnapshotToken,
    revision: u64,
    view_generation: u64,
    scope_generation: u64,
    selection: Option<(Option<Scope>, String)>,
    definition_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ImportOwner {
    scope: Option<Scope>,
    selection: Option<(Option<Scope>, String)>,
    view_generation: u64,
    scope_generation: u64,
    workspace: &'static str,
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
            snapshot_token: snapshot.token,
            revision: snapshot.document.revision,
            view_generation,
            scope_generation,
            selection,
            definition_id,
        }
    }
}

fn capture_matches(
    capture: &ImportCapture,
    current: &AcceptedSnapshot,
    owner: &ImportOwner,
) -> bool {
    owner.scope.as_ref() == Some(&capture.scope)
        && owner.selection == capture.selection
        && current.session_epoch == capture.session_epoch
        && current.document.id == capture.document_id
        && current.token == capture.snapshot_token
        && current.document.revision == capture.revision
        && capture.scope.document_id == capture.document_id
        && capture.scope.session_epoch == capture.session_epoch
        && owner.view_generation == capture.view_generation
        && owner.scope_generation == capture.scope_generation
        && owner.workspace == "Parts"
}

fn prepare_import_edit(
    current: &AcceptedSnapshot,
    owner: &ImportOwner,
    capture: &ImportCapture,
    mut imported: CompiledFootprint,
    operation_id: OperationId,
) -> Result<Event, String> {
    if !capture_matches(capture, current, owner) {
        return Err(
            "The Parts project or selection changed during the KiCad footprint import.".into(),
        );
    }

    if imported.definition.id != capture.definition_id {
        return Err(
            "The KiCad footprint importer returned a different definition identity.".into(),
        );
    }
    if current
        .document
        .definitions
        .iter()
        .any(|definition| definition.id == capture.definition_id)
    {
        return Err("The imported footprint identity is already in use.".into());
    }

    // The source remains the editable artifact. These projected values let the existing
    // Parts preview/read model show precisely what the parser accepted; they are not
    // reconstructed back into KiCad source and authored pad-number rules do not apply.
    imported.definition.pads = imported.geometry.pads;
    imported.definition.courtyard = imported.geometry.courtyard;

    let mut document = current.document.as_ref().clone();
    document.definitions.push(imported.definition);
    Ok(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: capture.revision,
            transaction_id: format!("parts-import-footprint-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![capture.definition_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(document),
            },
        },
    })
}

fn accepted_import_is_current(
    capture: &ImportCapture,
    current: &AcceptedSnapshot,
    owner: &ImportOwner,
) -> bool {
    owner.scope.as_ref() == Some(&capture.scope)
        && owner.selection == capture.selection
        && current.session_epoch == capture.session_epoch
        && current.document.id == capture.document_id
        && current.document.revision > capture.revision
        && owner.view_generation == capture.view_generation
        && owner.scope_generation == capture.scope_generation
        && owner.workspace == "Parts"
        && current
            .document
            .definitions
            .iter()
            .any(|definition| definition.id == capture.definition_id)
}

#[cfg(target_arch = "wasm32")]
mod ui {
    use super::{ImportCapture, ImportOwner, accepted_import_is_current, prepare_import_edit};
    use crate::{operation_outcomes::OutcomeSlot, runtime::Runtime};
    use boardstudio_application::{AcceptedSnapshot, Scope, TerminalOutcome};
    use dioxus::prelude::*;
    use dioxus_web::WebEventExt;
    use js_sys::{Date, Function, Reflect};
    use std::{cell::Cell, rc::Rc};
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_futures::{JsFuture, spawn_local};
    use web_sys::HtmlInputElement;

    #[derive(Clone)]
    struct PendingImport {
        capture: ImportCapture,
        filename: String,
        diagnostics: Vec<String>,
        outcome: Option<OutcomeSlot>,
    }

    /// The upload action stays mounted independently from catalogue success/loading state.
    #[component]
    pub(crate) fn ImportKiCadFootprintAction(
        scope: Option<Scope>,
        view_generation: Signal<u64>,
        scope_generation: Signal<u64>,
        workspace: Signal<&'static str>,
        selected: Signal<Option<(Option<Scope>, String)>>,
        query: Signal<String>,
        on_select: EventHandler<()>,
    ) -> Element {
        let runtime = use_context::<Rc<Runtime>>();
        let version = use_context::<Signal<u64>>();
        let pending = use_signal(|| None::<PendingImport>);
        let mut error = use_signal(|| None::<String>);
        let mut notice = use_signal(|| None::<String>);
        let owner_is_mounted = use_hook(|| Rc::new(Cell::new(true)));
        let request_epoch = use_hook(|| Rc::new(Cell::new(0_u64)));
        use_drop({
            let owner_is_mounted = owner_is_mounted.clone();
            let request_epoch = request_epoch.clone();
            move || {
                owner_is_mounted.set(false);
                request_epoch.set(request_epoch.get().wrapping_add(1));
            }
        });

        use_effect(use_reactive((&version(),), {
            let runtime = runtime.clone();
            let owner_is_mounted = owner_is_mounted.clone();
            let mut pending = pending;
            let mut selected = selected;
            let mut query = query;
            move |_| {
                let Some(waiting) = pending.read().clone() else {
                    return;
                };
                let Some(outcome) = waiting
                    .outcome
                    .as_ref()
                    .and_then(|slot| slot.borrow().clone())
                else {
                    return;
                };
                pending.set(None);
                if !owner_is_mounted.get() {
                    return;
                }
                match outcome {
                    TerminalOutcome::Completed => {}
                    TerminalOutcome::Rejected(reason)
                    | TerminalOutcome::PersistenceFailed(reason)
                    | TerminalOutcome::BlockedByRecovery(reason)
                    | TerminalOutcome::ExecutorFailed(reason) => {
                        error.set(Some(reason));
                        return;
                    }
                    TerminalOutcome::Cancelled => {
                        error.set(Some("The KiCad footprint edit was cancelled.".into()));
                        return;
                    }
                    TerminalOutcome::Closed | TerminalOutcome::Superseded => return,
                }

                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref() else {
                    return;
                };
                if !accepted_import_is_current(
                    &waiting.capture,
                    snapshot,
                    &ImportOwner {
                        scope: runtime.scope(),
                        selection: selected(),
                        view_generation: view_generation(),
                        scope_generation: scope_generation(),
                        workspace: workspace(),
                    },
                ) {
                    return;
                }
                selected.set(Some((
                    Some(waiting.capture.scope.clone()),
                    waiting.capture.definition_id.clone(),
                )));
                query.set(String::new());
                error.set(None);
                notice
                    .set((!waiting.diagnostics.is_empty()).then(|| {
                        format!("Imported with notes: {}", waiting.diagnostics.join(" "))
                    }));
                on_select.call(());
            }
        }));

        let cancel = {
            let request_epoch = request_epoch.clone();
            let mut pending = pending;
            let mut error = error;
            let mut notice = notice;
            move |_| {
                request_epoch.set(request_epoch.get().wrapping_add(1));
                pending.set(None);
                error.set(None);
                notice.set(None);
            }
        };
        let onchange = {
            let runtime = runtime.clone();
            let owner_is_mounted = owner_is_mounted.clone();
            let request_epoch = request_epoch.clone();
            let mut pending = pending;
            let mut error = error;
            let mut notice = notice;
            let selected = selected;
            let scope = scope.clone();
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
                error.set(None);
                notice.set(None);
                if !file.name().to_lowercase().ends_with(".kicad_mod") {
                    error.set(Some("Select a .kicad_mod footprint.".into()));
                    return;
                }
                if pending.read().is_some() || !owner_is_mounted.get() || workspace() != "Parts" {
                    return;
                }

                let model = runtime.model();
                let Some(snapshot) = model.accepted.as_ref().cloned() else {
                    error.set(Some("Open a keyboard before importing a footprint.".into()));
                    return;
                };
                let Some(current_scope) = runtime.scope() else {
                    error.set(Some("The active keyboard scope is unavailable.".into()));
                    return;
                };
                if scope.as_ref() != Some(&current_scope)
                    || snapshot.document.id != current_scope.document_id
                    || snapshot.session_epoch != current_scope.session_epoch
                {
                    error.set(Some(
                        "The active Parts project changed. Choose the file again.".into(),
                    ));
                    return;
                }
                let definition_id = match unique_definition_id(&snapshot) {
                    Ok(id) => id,
                    Err(message) => {
                        error.set(Some(message));
                        return;
                    }
                };
                let capture = ImportCapture::new(
                    &snapshot,
                    current_scope,
                    view_generation(),
                    scope_generation(),
                    selected(),
                    definition_id,
                );
                let epoch = request_epoch.get().wrapping_add(1);
                request_epoch.set(epoch);
                let filename = file.name();
                pending.set(Some(PendingImport {
                    capture: capture.clone(),
                    filename: filename.clone(),
                    diagnostics: Vec::new(),
                    outcome: None,
                }));
                let runtime = runtime.clone();
                let owner_is_mounted = owner_is_mounted.clone();
                let request_epoch = request_epoch.clone();
                let mut pending = pending;
                let mut error = error;
                let selected = selected;
                let scope = scope.clone();
                spawn_local(async move {
                    let still_owned = || owner_is_mounted.get() && request_epoch.get() == epoch;
                    let source_is_current = || {
                        runtime.model().accepted.as_ref().is_some_and(|current| {
                            super::capture_matches(
                                &capture,
                                current,
                                &ImportOwner {
                                    scope: runtime.scope(),
                                    selection: selected(),
                                    view_generation: view_generation(),
                                    scope_generation: scope_generation(),
                                    workspace: workspace(),
                                },
                            )
                        })
                    };
                    let source = match JsFuture::from(file.text()).await {
                        Ok(value) => match value.as_string() {
                            Some(source) => source,
                            None => {
                                if still_owned() {
                                    pending.set(None);
                                    error.set(Some(
                                        "The selected footprint could not be read as text.".into(),
                                    ));
                                }
                                return;
                            }
                        },
                        Err(cause) => {
                            if still_owned() {
                                pending.set(None);
                                error.set(Some(format!(
                                    "Could not read the selected footprint: {cause:?}"
                                )));
                            }
                            return;
                        }
                    };
                    if !still_owned() {
                        return;
                    }
                    if !source_is_current() {
                        pending.set(None);
                        error.set(Some("The Parts project or selection changed during file reading. Choose the file again.".into()));
                        return;
                    }
                    let request_id = format!("parts-import-footprint-{}", runtime.operation().0);
                    let compiled = match runtime
                        .import_footprint(request_id, capture.definition_id.clone(), source)
                        .await
                    {
                        Ok(compiled) => compiled,
                        Err(message) => {
                            if still_owned() {
                                pending.set(None);
                                error.set(Some(if source_is_current() {
                                    message
                                } else {
                                    "The Parts project or selection changed during import. Choose the file again.".into()
                                }));
                            }
                            return;
                        }
                    };
                    if !still_owned() {
                        return;
                    }
                    if !source_is_current() {
                        pending.set(None);
                        error.set(Some("The Parts project or selection changed during import. Choose the file again.".into()));
                        return;
                    }
                    let current = runtime.model().accepted;
                    let Some(current) = current else {
                        pending.set(None);
                        error.set(Some("The accepted keyboard closed during import.".into()));
                        return;
                    };
                    let diagnostics = compiled
                        .diagnostics
                        .iter()
                        .map(|diagnostic| diagnostic.message.clone())
                        .collect::<Vec<_>>();
                    let event = match prepare_import_edit(
                        &current,
                        &ImportOwner {
                            scope: runtime.scope(),
                            selection: selected(),
                            view_generation: view_generation(),
                            scope_generation: scope_generation(),
                            workspace: workspace(),
                        },
                        &capture,
                        compiled,
                        runtime.operation(),
                    ) {
                        Ok(event) => event,
                        Err(message) => {
                            pending.set(None);
                            error.set(Some(format!("{message} Choose the file again.")));
                            return;
                        }
                    };
                    if scope.as_ref() != runtime.scope().as_ref() || !still_owned() {
                        pending.set(None);
                        error.set(Some(
                            "The Parts project changed during import. Choose the file again."
                                .into(),
                        ));
                        return;
                    }
                    let operation_id = match &event {
                        boardstudio_application::Event::Edit { operation_id, .. } => *operation_id,
                        _ => unreachable!("footprint import prepares an edit"),
                    };
                    let outcome = runtime.observe_operation(operation_id);
                    pending.set(Some(PendingImport {
                        capture,
                        filename,
                        diagnostics,
                        outcome: Some(outcome),
                    }));
                    runtime.submit(event);
                });
            }
        };

        let busy = pending.read().is_some();
        rsx! {
            div { class: "m1-parts-import-control",
                label {
                    "Import KiCad footprint"
                    input {
                        r#type: "file",
                        accept: ".kicad_mod",
                        aria_label: "Import KiCad footprint",
                        disabled: busy || scope.is_none(),
                        onchange,
                    }
                }
            }
            if let Some(waiting) = pending.read().as_ref() {
                p { class: "m1-parts-loading", role: "status",
                    if waiting.outcome.is_some() { "Adding {waiting.filename}…" } else { "Reading and importing {waiting.filename}…" }
                    if waiting.outcome.is_none() { button { type: "button", onclick: cancel, "Cancel import" } }
                }
            }
            if let Some(message) = error() {
                p { class: "m1-parts-load-error", role: "alert", "{message}" }
            }
            if let Some(message) = notice() {
                p { class: "m1-parts-import-notice", role: "status", "{message}" }
            }
        }
    }

    fn unique_definition_id(snapshot: &AcceptedSnapshot) -> Result<String, String> {
        let base = browser_identity().unwrap_or_else(date_identity);
        (0_u64..)
            .map(|suffix| {
                if suffix == 0 {
                    format!("imported-{base}")
                } else {
                    format!("imported-{base}-{suffix}")
                }
            })
            .find(|candidate| {
                snapshot
                    .document
                    .definitions
                    .iter()
                    .all(|definition| definition.id != *candidate)
            })
            .ok_or_else(|| "Could not allocate a unique imported footprint identity.".into())
    }

    fn browser_identity() -> Option<String> {
        let crypto = Reflect::get(&js_sys::global(), &JsValue::from_str("crypto")).ok()?;
        let random_uuid = Reflect::get(&crypto, &JsValue::from_str("randomUUID"))
            .ok()?
            .dyn_into::<Function>()
            .ok()?;
        random_uuid.call0(&crypto).ok()?.as_string()
    }

    fn date_identity() -> String {
        let mut value = Date::now().max(0.0) as u64;
        const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut digits = Vec::new();
        while value > 0 {
            digits.push(DIGITS[(value % 36) as usize]);
            value /= 36;
        }
        if digits.is_empty() {
            digits.push(b'0');
        }
        digits.reverse();
        String::from_utf8(digits).expect("base-36 digits are valid UTF-8")
    }
}

#[cfg(target_arch = "wasm32")]
pub(crate) use ui::ImportKiCadFootprintAction;
