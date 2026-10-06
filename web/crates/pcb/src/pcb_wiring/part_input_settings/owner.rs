use super::{binding_schema, project};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Event, Lifecycle, Scope, SnapshotToken, TerminalOutcome,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, PressScanMode, ProjectDoc};
use dioxus::prelude::*;
use serde_json::Value;
use std::{cell::Cell, collections::BTreeSet, rc::Rc};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartInputIdentity {
    pub ui_scope: Scope,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub generator_source: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub executor_epoch: u64,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PartInputIntent {
    ScanMode(PressScanMode),
    GeneratorParameter { name: String, value: Option<Value> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartInputEditRequest {
    pub identity: PartInputIdentity,
    pub intent: PartInputIntent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartInputFeedbackState {
    Preparing,
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartInputFeedback {
    pub identity: PartInputIdentity,
    pub state: PartInputFeedbackState,
}

#[derive(Clone, PartialEq)]
pub struct PartInputActions {
    pub feedback: Option<PartInputFeedback>,
    pub editable: bool,
    pub on_edit: EventHandler<PartInputEditRequest>,
}

#[derive(Clone)]
enum ExpectedEdit {
    ScanMode(PressScanMode),
    Document(Box<ProjectDoc>),
}

#[derive(Clone)]
struct PendingEdit {
    identity: PartInputIdentity,
    base_revision: u64,
    outcome: crate::operation_outcomes::OutcomeSlot,
    expected: ExpectedEdit,
}

pub fn use_part_input_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
) -> PartInputActions {
    let pending = use_signal(|| None::<PendingEdit>);
    let feedback = use_signal(|| None::<PartInputFeedback>);
    let preparing = use_signal(|| false);
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let observed_version = version();

    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let Some(waiting) = pending.read().clone() else {
                return;
            };
            let Some(outcome) = waiting.outcome.borrow_mut().take() else {
                return;
            };
            let model = runtime.model();
            let accepted = model.accepted.as_ref().filter(|accepted| {
                accepted.session_epoch == waiting.identity.ui_scope.session_epoch
                    && accepted.document.id == waiting.identity.ui_scope.document_id
            });
            let saved_source = accepted.is_some_and(|accepted| {
                model.lifecycle == Lifecycle::Ready
                    && model.durability
                        == (Durability::Saved {
                            revision: accepted.document.revision,
                        })
                    && accepted.document.revision > waiting.base_revision
            });
            let saved_value = accepted.is_some_and(|accepted| match &waiting.expected {
                ExpectedEdit::ScanMode(mode) => {
                    accepted
                        .document
                        .parts
                        .iter()
                        .find(|part| part.id == waiting.identity.part_id)
                        .and_then(|part| {
                            part.properties
                                .as_ref()
                                .and_then(|properties| properties.get("pressScanMode"))
                        })
                        .and_then(|value| {
                            serde_json::from_value::<PressScanMode>(value.clone()).ok()
                        })
                        == Some(*mode)
                }
                ExpectedEdit::Document(expected) => {
                    let mut expected = (**expected).clone();
                    expected.revision = accepted.document.revision;
                    *accepted.document == expected
                }
            });
            let durability_failure = match &model.durability {
                Durability::Failed { reason, .. } => Some(reason.clone()),
                _ => None,
            };
            let state = match outcome {
                TerminalOutcome::Completed if saved_source && saved_value => {
                    let mut identity = waiting.identity.clone();
                    if let Some(accepted) = accepted {
                        identity.token = accepted.token;
                        identity.revision = accepted.document.revision;
                    }
                    pending.set(None);
                    feedback.set(Some(PartInputFeedback {
                        identity,
                        state: PartInputFeedbackState::Saved,
                    }));
                    return;
                }
                TerminalOutcome::Completed => PartInputFeedbackState::Failed(
                    "The edit completed, but the requested setting is not the saved project state. Retry the edit.".into(),
                ),
                TerminalOutcome::PersistenceFailed(message) => {
                    PartInputFeedbackState::Failed(format!("Could not save the setting: {message}"))
                }
                TerminalOutcome::Rejected(message)
                | TerminalOutcome::ExecutorFailed(message)
                | TerminalOutcome::BlockedByRecovery(message) => {
                    PartInputFeedbackState::Failed(format!("Setting was rejected: {message}"))
                }
                TerminalOutcome::Cancelled
                | TerminalOutcome::Closed
                | TerminalOutcome::Superseded => PartInputFeedbackState::Failed(
                    "The setting was cancelled before it could be saved.".into(),
                ),
            };
            let mut identity = waiting.identity.clone();
            if saved_source && let Some(accepted) = accepted {
                identity.token = accepted.token;
                identity.revision = accepted.document.revision;
            }
            pending.set(None);
            feedback.set(Some(PartInputFeedback {
                identity,
                state: durability_failure.map_or(state.clone(), |reason| {
                    PartInputFeedbackState::Failed(format!("Could not save the setting: {reason}"))
                }),
            }));
        }
    }));

    let on_edit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |request: PartInputEditRequest| {
            if pending.peek().is_some() || *preparing.peek() {
                return;
            }
            let Some(snapshot) = current_snapshot(
                &runtime,
                &request.identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            ) else {
                return;
            };
            let runtime = runtime.clone();
            let mut pending = pending;
            let mut feedback = feedback;
            let mut preparing = preparing;
            let instance_is_current = instance_is_current.clone();
            let alive = alive.clone();
            preparing.set(true);
            feedback.set(Some(PartInputFeedback {
                identity: request.identity.clone(),
                state: PartInputFeedbackState::Preparing,
            }));
            spawn_local(async move {
                let (mut command, expected) = match prepare_edit(&snapshot, &request).await {
                    Ok(prepared) => prepared,
                    Err(message) => {
                        if alive.get() {
                            preparing.set(false);
                            feedback.set(Some(PartInputFeedback {
                                identity: request.identity.clone(),
                                state: PartInputFeedbackState::Failed(message),
                            }));
                        }
                        return;
                    }
                };
                // Generator-schema loading can outlive the Editor. Do not touch captured Signals
                // or selection state after unmount; check the live workspace before admission.
                if !alive.get() {
                    return;
                }
                if current_snapshot(
                    &runtime,
                    &request.identity,
                    workspace(),
                    scope_generation(),
                    instance_is_current(),
                )
                .is_none()
                {
                    preparing.set(false);
                    feedback.set(None);
                    return;
                }
                let operation_id = runtime.operation();
                command.transaction_id = format!("pcb-part-settings-{}", operation_id.0);
                let outcome = runtime.observe_operation(operation_id);
                preparing.set(false);
                pending.set(Some(PendingEdit {
                    identity: request.identity.clone(),
                    base_revision: command.base_revision,
                    outcome,
                    expected,
                }));
                feedback.set(Some(PartInputFeedback {
                    identity: request.identity.clone(),
                    state: PartInputFeedbackState::Pending,
                }));
                runtime.submit(Event::Edit {
                    operation_id,
                    command,
                });
            });
        }
    });

    let editable = pending().is_none()
        && !preparing()
        && current_pcb_source(
            &runtime,
            workspace(),
            scope_generation(),
            instance_is_current(),
        );
    PartInputActions {
        feedback: feedback(),
        editable,
        on_edit,
    }
}

fn current_snapshot(
    runtime: &Runtime,
    identity: &PartInputIdentity,
    workspace: &'static str,
    generation: u64,
    instance_is_current: bool,
) -> Option<AcceptedSnapshot> {
    if workspace != "PCB" || !instance_is_current || generation != identity.scope_generation {
        return None;
    }
    let model = runtime.model();
    if model.lifecycle != Lifecycle::Ready
        || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(model.durability, Durability::Saved { .. })
        || model.active_board_id != identity.board_id
        || model.active_instance_id != identity.ui_scope.instance_id
        || model.selected_part_ids.len() != 1
        || model.selected_part_ids.first() != Some(&identity.part_id)
        || runtime.scope().as_ref() != Some(&identity.ui_scope)
        || runtime.electrical_preview_executor_epoch() != identity.executor_epoch
    {
        return None;
    }
    let snapshot = model.accepted?;
    if snapshot.session_epoch != identity.ui_scope.session_epoch
        || snapshot.document.id != identity.ui_scope.document_id
        || snapshot.token != identity.token
        || snapshot.document.revision != identity.revision
        || snapshot.scene.revision != snapshot.document.revision
    {
        return None;
    }
    let part = snapshot
        .document
        .parts
        .iter()
        .find(|part| part.id == identity.part_id)?;
    if part.definition_id != identity.definition_id
        || part.locked == Some(true)
        || !snapshot.document.boards.iter().any(|board| {
            board.id == identity.board_id && board.part_ids.contains(&identity.part_id)
        })
    {
        return None;
    }
    let definition = snapshot
        .document
        .definitions
        .iter()
        .find(|definition| definition.id == identity.definition_id)?;
    if matches!(
        &definition.kind,
        boardstudio_core::model::PartKind::Switch | boardstudio_core::model::PartKind::Controller
    ) || definition
        .generator
        .as_ref()
        .map(|generator| generator.source.as_str())
        != identity.generator_source.as_deref()
    {
        return None;
    }
    Some(snapshot)
}

fn current_pcb_source(
    runtime: &Runtime,
    workspace: &'static str,
    _generation: u64,
    instance_is_current: bool,
) -> bool {
    if workspace != "PCB" || !instance_is_current {
        return false;
    }
    let model = runtime.model();
    model.lifecycle == Lifecycle::Ready
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && matches!(model.durability, Durability::Saved { .. })
        && model
            .accepted
            .as_ref()
            .is_some_and(|snapshot| snapshot.scene.revision == snapshot.document.revision)
        && runtime.scope().is_some_and(|scope| {
            scope.board_id == model.active_board_id && scope.instance_id == model.active_instance_id
        })
}

async fn prepare_edit(
    snapshot: &AcceptedSnapshot,
    request: &PartInputEditRequest,
) -> Result<(EditCommand, ExpectedEdit), String> {
    let identity = &request.identity;
    let projection = project(&snapshot.document, &identity.board_id, &identity.part_id)
        .ok_or_else(|| "The selected part no longer supports these controls.".to_owned())?;
    let (operation, expected, target_ids) = match &request.intent {
        PartInputIntent::ScanMode(mode) => {
            if projection.press_mode.is_none()
                || (*mode == PressScanMode::Matrix && !projection.matrix_mode_enabled)
            {
                return Err("This part cannot use the selected press scan mode.".into());
            }
            (
                EditOperation::SetInputScanMode {
                    part_id: identity.part_id.clone(),
                    mode: *mode,
                },
                ExpectedEdit::ScanMode(*mode),
                vec![identity.part_id.clone()],
            )
        }
        PartInputIntent::GeneratorParameter { name, value } => {
            let source = identity
                .generator_source
                .as_deref()
                .ok_or_else(|| "The selected part has no supported generator.".to_owned())?;
            let schema =
                crate::presentation::parts::generator_parameter_schema(source.to_owned()).await?;
            let terminals = projection
                .definition
                .terminals
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let fields = binding_schema(&schema, &terminals);
            let kind = fields
                .iter()
                .find(|(field, _)| field == name)
                .map(|(_, kind)| kind.as_str())
                .ok_or_else(|| {
                    "This generator parameter is not an editable PCB binding.".to_owned()
                })?;
            validate_parameter_value(kind, value.as_ref(), &projection)?;
            let mut document = (*snapshot.document).clone();
            let part = document
                .parts
                .iter_mut()
                .find(|part| part.id == identity.part_id)
                .ok_or_else(|| "The selected part is no longer available.".to_owned())?;
            let mut parameters = part.generator_parameters.clone().unwrap_or_default();
            if let Some(value) = value {
                parameters.insert(name.clone(), value.clone());
            } else {
                parameters.remove(name);
            }
            part.generator_parameters = Some(parameters);
            let target = vec![identity.part_id.clone(), name.clone()];
            (
                EditOperation::ReplaceDocument {
                    document: Box::new(document.clone()),
                },
                ExpectedEdit::Document(Box::new(document)),
                target,
            )
        }
    };
    Ok((
        EditCommand {
            base_revision: snapshot.document.revision,
            transaction_id: format!("pcb-part-settings-{}", identity.scope_generation),
            phase: EditPhase::Commit,
            target_ids,
            operation,
        },
        expected,
    ))
}

fn validate_parameter_value(
    kind: &str,
    value: Option<&Value>,
    projection: &super::PartInputProjection,
) -> Result<(), String> {
    match (kind, value) {
        ("net", None) => Ok(()),
        ("net", Some(Value::String(value)))
            if projection.nets.iter().any(|net| net.name == *value) =>
        {
            Ok(())
        }
        ("anchor", None) => Ok(()),
        ("anchor", Some(Value::Object(values)))
            if ["x", "y"].into_iter().all(|axis| {
                values
                    .get(axis)
                    .is_none_or(|value| value.as_f64().is_some_and(f64::is_finite))
            }) =>
        {
            Ok(())
        }
        ("net", _) => Err("Choose a net on the selected board.".into()),
        ("anchor", _) => Err("Anchor coordinates must be finite numbers.".into()),
        _ => Err("Unsupported generator binding type.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[wasm_bindgen_test]
    fn parameter_values_are_limited_to_board_nets_and_finite_anchor_coordinates() {
        let projection = super::super::tests::projection_for_validation();
        assert!(validate_parameter_value("net", Some(&json!("VCC")), &projection).is_ok());
        assert!(
            validate_parameter_value("net", Some(&json!("Other board only")), &projection).is_err()
        );
        assert!(validate_parameter_value("anchor", Some(&json!({"x": 2.5})), &projection).is_ok());
        assert!(
            validate_parameter_value("anchor", Some(&json!({"x": f64::INFINITY})), &projection)
                .is_err()
        );
        assert!(validate_parameter_value("anchor", Some(&json!([])), &projection).is_err());
    }
}
