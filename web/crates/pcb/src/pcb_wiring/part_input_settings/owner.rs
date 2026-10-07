use super::{binding_schema, project};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, PressScanMode};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use serde_json::Value;
use std::{
    cell::{Cell, RefCell},
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};
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
    GeneratorParameter {
        name: String,
        value: Option<Value>,
    },
    GeneratorAnchor {
        name: String,
        axis: String,
        value: Option<f64>,
    },
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

#[derive(Clone, Copy)]
struct InputDrafts(Signal<Vec<(u64, PartInputEditRequest)>>);

pub(super) fn apply_drafts(
    identity: &PartInputIdentity,
    projection: &mut super::PartInputProjection,
) {
    let Some(drafts) = try_consume_context::<InputDrafts>() else {
        return;
    };
    for (_, request) in drafts.0.read().iter().filter(|(_, request)| {
        request.identity.ui_scope == identity.ui_scope
            && request.identity.scope_generation == identity.scope_generation
            && request.identity.part_id == identity.part_id
    }) {
        match &request.intent {
            PartInputIntent::ScanMode(mode) => projection.press_mode = Some(*mode),
            PartInputIntent::GeneratorParameter { name, value } => {
                if let Some(value) = value {
                    projection
                        .generator_parameters
                        .insert(name.clone(), value.clone());
                } else {
                    projection.generator_parameters.remove(name);
                }
            }
            PartInputIntent::GeneratorAnchor { name, axis, value } => {
                let mut anchor = projection
                    .generator_parameters
                    .get(name)
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                if let Some(value) = value {
                    anchor.insert(axis.clone(), Value::from(*value));
                } else {
                    anchor.remove(axis);
                }
                projection
                    .generator_parameters
                    .insert(name.clone(), Value::Object(anchor));
            }
        }
    }
}

pub fn use_part_input_edits(
    runtime: Rc<Runtime>,
    version: Signal<u64>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    instance_is_current: Rc<dyn Fn() -> bool>,
) -> PartInputActions {
    let tickets = use_signal(Vec::<(u64, PartInputIdentity, EditTicket)>::new);
    let drafts = use_signal(Vec::<(u64, PartInputEditRequest)>::new);
    use_context_provider(|| InputDrafts(drafts));
    let sequence = use_hook(|| Rc::new(Cell::new(0u64)));
    let latest = use_signal(|| None::<boardstudio_application::OperationId>);
    let feedback = use_signal(|| None::<PartInputFeedback>);
    // Schema preparation is serial so slower module loading cannot reorder committed values.
    let queue = use_hook(|| Rc::new(RefCell::new(VecDeque::<(u64, PartInputEditRequest)>::new())));
    let preparing = use_hook(|| Rc::new(Cell::new(false)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let runtime = runtime.clone();
        let mut tickets = tickets;
        let mut feedback = feedback;
        let mut drafts = drafts;
        move |_| {
            let mut entries = tickets.peek().clone();
            let before = entries.len();
            entries.retain(|(sequence, identity, ticket)| {
                let live = owner_is_live(&runtime, identity, workspace(), scope_generation());
                let state = match ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(PartInputFeedbackState::Saved),
                    Settlement::Failed { message } => Some(PartInputFeedbackState::Failed(message)),
                    Settlement::Retired => None,
                };
                drafts.write().retain(|(id, _)| id != sequence);
                if *latest.peek() == Some(ticket.operation()) {
                    feedback.set(state.map(|state| PartInputFeedback {
                        identity: identity.clone(),
                        state,
                    }));
                }
                false
            });
            if before != entries.len() {
                tickets.set(entries);
            }
        }
    }));
    let on_edit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        move |request: PartInputEditRequest| {
            if current_snapshot(
                &runtime,
                &request.identity,
                workspace(),
                scope_generation(),
                instance_is_current(),
            )
            .is_none()
            {
                return;
            }
            let id = sequence
                .get()
                .checked_add(1)
                .expect("input draft identity exhausted");
            sequence.set(id);
            let mut drafts = drafts;
            drafts.write().push((id, request.clone()));
            let mut latest = latest;
            latest.set(None);
            let mut feedback = feedback;
            feedback.set(None);
            queue.borrow_mut().push_back((id, request));
            if preparing.replace(true) {
                return;
            }
            let queue = queue.clone();
            let preparing = preparing.clone();
            let alive = alive.clone();
            let runtime = runtime.clone();
            let mut tickets = tickets;
            let mut latest = latest;
            let mut feedback = feedback;
            spawn_local(async move {
                loop {
                    let request = queue.borrow_mut().pop_front();
                    let Some((id, request)) = request else {
                        preparing.set(false);
                        break;
                    };
                    let schema = match &request.intent {
                        PartInputIntent::ScanMode(_) => Ok(BTreeMap::new()),
                        PartInputIntent::GeneratorParameter { .. }
                        | PartInputIntent::GeneratorAnchor { .. } => {
                            match &request.identity.generator_source {
                                Some(source) => {
                                    crate::presentation::parts::generator_parameter_schema(
                                        source.clone(),
                                    )
                                    .await
                                }
                                None => Err("The part has no supported generator.".into()),
                            }
                        }
                    };
                    if !alive.get() {
                        return;
                    }
                    if !owner_is_live(&runtime, &request.identity, workspace(), scope_generation())
                    {
                        drafts.write().retain(|(sequence, _)| *sequence != id);
                        continue;
                    }
                    let identity = request.identity.clone();
                    let ticket = EditTicket::begin(
                        &runtime,
                        "pcb-part-settings",
                        Some("setting".into()),
                        input_resolver(request, schema),
                    );
                    latest.set(Some(ticket.operation()));
                    feedback.set(Some(PartInputFeedback {
                        identity: identity.clone(),
                        state: PartInputFeedbackState::Pending,
                    }));
                    tickets.write().push((id, identity, ticket));
                }
            });
        }
    });
    let editable = current_pcb_source(
        &runtime,
        workspace(),
        scope_generation(),
        instance_is_current(),
    );
    let feedback = feedback()
        .filter(|entry| owner_is_live(&runtime, &entry.identity, workspace(), scope_generation()));
    PartInputActions {
        feedback,
        editable,
        on_edit,
    }
}

fn owner_is_live(
    runtime: &Runtime,
    identity: &PartInputIdentity,
    workspace: &str,
    generation: u64,
) -> bool {
    workspace == "PCB"
        && generation == identity.scope_generation
        && runtime.scope().as_ref() == Some(&identity.ui_scope)
        && runtime.model().selected_part_ids.as_slice() == [identity.part_id.as_str()]
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
    if !matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) || model.display_preview.is_some()
        || model.gesture.is_some()
        || !matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
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
    matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && model.display_preview.is_none()
        && model.gesture.is_none()
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        && model
            .accepted
            .as_ref()
            .is_some_and(|snapshot| snapshot.scene.revision == snapshot.document.revision)
        && runtime.scope().is_some_and(|scope| {
            scope.board_id == model.active_board_id && scope.instance_id == model.active_instance_id
        })
}

fn input_resolver(
    request: PartInputEditRequest,
    schema: Result<BTreeMap<String, Value>, String>,
) -> EditResolver {
    EditResolver::new("pcb-part-settings", move |accepted: &AcceptedSnapshot| {
        let identity = &request.identity;
        if accepted.session_epoch != identity.ui_scope.session_epoch
            || accepted.document.id != identity.ui_scope.document_id
        {
            return Resolution::Retire(boardstudio_application::DOCUMENT_SESSION_CHANGED.into());
        }
        let Some(projection) = project(&accepted.document, &identity.board_id, &identity.part_id)
        else {
            return Resolution::Retire(
                "The part was deleted or no longer supports these controls.".into(),
            );
        };
        if projection.part.locked == Some(true)
            || projection.definition.id != identity.definition_id
            || projection
                .definition
                .generator
                .as_ref()
                .map(|generator| &generator.source)
                != identity.generator_source.as_ref()
        {
            return Resolution::Retire("The part is locked or its definition changed.".into());
        }
        let schema = match &schema {
            Ok(schema) => schema,
            Err(reason) => return Resolution::Retire(reason.clone()),
        };
        let operation = match &request.intent {
            PartInputIntent::ScanMode(mode) => {
                if projection.press_mode.is_none()
                    || (*mode == PressScanMode::Matrix && !projection.matrix_mode_enabled)
                {
                    return Resolution::Retire(
                        "This part cannot use the selected press scan mode.".into(),
                    );
                }
                if projection.press_mode == Some(*mode) {
                    return Resolution::Unchanged;
                }
                EditOperation::SetInputScanMode {
                    part_id: identity.part_id.clone(),
                    mode: *mode,
                }
            }
            PartInputIntent::GeneratorParameter { name, value } => {
                let terminals = projection
                    .definition
                    .terminals
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let fields = binding_schema(schema, &terminals);
                let Some((_, kind)) = fields.iter().find(|(field, _)| field == name) else {
                    return Resolution::Retire(
                        "This generator parameter is no longer an editable PCB binding.".into(),
                    );
                };
                if let Err(reason) = validate_parameter_value(kind, value.as_ref(), &projection) {
                    return Resolution::Retire(reason);
                }
                let mut document = (*accepted.document).clone();
                let part = document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == identity.part_id)
                    .unwrap();
                let mut parameters = part.generator_parameters.clone().unwrap_or_default();
                if parameters.get(name) == value.as_ref() {
                    return Resolution::Unchanged;
                }
                if let Some(value) = value {
                    parameters.insert(name.clone(), value.clone());
                } else {
                    parameters.remove(name);
                }
                part.generator_parameters = Some(parameters);
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                }
            }
            PartInputIntent::GeneratorAnchor { name, axis, value } => {
                if !matches!(axis.as_str(), "x" | "y")
                    || value.is_some_and(|value| !value.is_finite())
                {
                    return Resolution::Retire("Anchor coordinates must be finite numbers.".into());
                }
                let terminals = projection
                    .definition
                    .terminals
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if !binding_schema(schema, &terminals)
                    .iter()
                    .any(|(field, kind)| field == name && kind == "anchor")
                {
                    return Resolution::Retire(
                        "This generator anchor is no longer editable.".into(),
                    );
                }
                let mut document = (*accepted.document).clone();
                let part = document
                    .parts
                    .iter_mut()
                    .find(|part| part.id == identity.part_id)
                    .unwrap();
                let mut parameters = part.generator_parameters.clone().unwrap_or_default();
                let mut anchor = parameters
                    .get(name)
                    .and_then(Value::as_object)
                    .cloned()
                    .unwrap_or_default();
                if anchor.get(axis).and_then(Value::as_f64) == *value {
                    return Resolution::Unchanged;
                }
                if let Some(value) = value {
                    anchor.insert(axis.clone(), Value::from(*value));
                } else {
                    anchor.remove(axis);
                }
                if anchor.is_empty() {
                    parameters.remove(name);
                } else {
                    parameters.insert(name.clone(), Value::Object(anchor));
                }
                part.generator_parameters = Some(parameters);
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                }
            }
        };
        Resolution::Submit(EditCommand {
            base_revision: accepted.document.revision,
            transaction_id: String::new(),
            phase: EditPhase::Commit,
            target_ids: vec![identity.part_id.clone()],
            operation,
        })
    })
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
