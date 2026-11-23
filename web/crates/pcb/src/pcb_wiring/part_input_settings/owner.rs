use super::super::part_input_projection::{binding_schema, project, validate_parameter_value};
use crate::pcb_wiring::mode::pcb_source;
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_model::model::{EditOperation, PressScanMode};
use boardstudio_web_ui_model::editor::{EditGuard, use_guarded_edit};
use dioxus::prelude::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartInputIdentity {
    pub ui_scope: Scope,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub generator_source: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
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
    pub(crate) schema: Rc<BTreeMap<String, Value>>,
}

/// The panel's latest failure, attributed to the part it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PartInputFailure {
    pub(crate) ui_scope: Scope,
    pub(crate) scope_generation: u64,
    pub(crate) part_id: String,
    pub(crate) message: String,
}

impl PartInputFailure {
    fn of(key: &PartInputIdentity, message: String) -> Self {
        Self {
            ui_scope: key.ui_scope.clone(),
            scope_generation: key.scope_generation,
            part_id: key.part_id.clone(),
            message,
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct PartInputActions {
    /// The latest failure for the rendered part; landing and retirement are silent.
    pub(crate) failure: Option<PartInputFailure>,
    pub editable: bool,
    pub on_edit: EventHandler<PartInputEditRequest>,
}

pub fn use_part_input_edits(guard: EditGuard) -> PartInputActions {
    let failure = use_signal(|| None::<PartInputFailure>);
    let generation = guard.editor.scope_generation;
    let on_edit = use_guarded_edit(guard, move |runtime, request: PartInputEditRequest| {
        if current_snapshot(runtime, &request.identity, generation()).is_none() {
            return;
        }
        let identity = request.identity.clone();
        let result = runtime.commit("pcb-part-settings", input_resolver(request));
        let mut failure = failure;
        failure.set(
            result
                .err()
                .map(|message| PartInputFailure::of(&identity, message)),
        );
    });
    let runtime = guard.editor.runtime();
    let editable = guard.workspace_is_current()
        && guard.instance_is_current(&runtime)
        && pcb_source(&runtime, generation()).is_some();
    let live_target = |ui_scope: &Scope, generation: u64, part_id: &str| {
        guard.route_is_current(generation)
            && runtime.scope().as_ref() == Some(ui_scope)
            && runtime.model().selected_part_ids.as_slice() == [part_id]
    };
    let failure = failure().filter(|failure| {
        live_target(
            &failure.ui_scope,
            failure.scope_generation,
            &failure.part_id,
        )
    });
    PartInputActions {
        failure,
        editable,
        on_edit,
    }
}

fn current_snapshot(
    runtime: &Runtime,
    identity: &PartInputIdentity,
    generation: u64,
) -> Option<AcceptedSnapshot> {
    if generation != identity.scope_generation {
        return None;
    }
    let (snapshot, source) = pcb_source(runtime, generation)?;
    if runtime.model().selected_part_ids.as_slice() != [identity.part_id.as_str()]
        || source.ui_scope != identity.ui_scope
        || source.identity.token != identity.token
        || source.identity.revision != identity.revision
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
        boardstudio_model::model::PartKind::Switch | boardstudio_model::model::PartKind::Controller
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

fn input_resolver(request: PartInputEditRequest) -> EditResolver {
    EditResolver::new("pcb-part-settings", move |accepted: &AcceptedSnapshot| {
        let identity = &request.identity;
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
        let schema = &request.schema;
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
        Resolution::submit(vec![identity.part_id.clone()], operation)
    })
}

// Parameter validation lives in the host-compatible projection module so the same
// predicate runs under `cargo test` without a browser (`part_input_projection` tests).
