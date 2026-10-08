use super::{binding_schema, project};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditOperation, PressScanMode};
use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
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

/// One part-input control's bounded logical key: the part target plus the field it edits.
#[derive(Clone, Debug, PartialEq)]
pub struct PartInputKey {
    pub ui_scope: Scope,
    pub scope_generation: u64,
    pub part_id: String,
    pub field: PartInputField,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PartInputField {
    ScanMode,
    GeneratorParameter(String),
    GeneratorAnchor { name: String, axis: String },
}

impl PartInputKey {
    fn of(request: &PartInputEditRequest) -> Self {
        let field = match &request.intent {
            PartInputIntent::ScanMode(_) => PartInputField::ScanMode,
            PartInputIntent::GeneratorParameter { name, .. } => {
                PartInputField::GeneratorParameter(name.clone())
            }
            PartInputIntent::GeneratorAnchor { name, axis, .. } => {
                PartInputField::GeneratorAnchor {
                    name: name.clone(),
                    axis: axis.clone(),
                }
            }
        };
        Self {
            ui_scope: request.identity.ui_scope.clone(),
            scope_generation: request.identity.scope_generation,
            part_id: request.identity.part_id.clone(),
            field,
        }
    }

    fn same_part(&self, identity: &PartInputIdentity) -> bool {
        self.ui_scope == identity.ui_scope
            && self.scope_generation == identity.scope_generation
            && self.part_id == identity.part_id
    }
}

/// The panel's latest failure, attributed to the part it belongs to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartInputFailure {
    pub ui_scope: Scope,
    pub scope_generation: u64,
    pub part_id: String,
    pub message: String,
}

impl PartInputFailure {
    fn of(key: &PartInputKey, message: String) -> Self {
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
    /// A committed setting for the rendered part has not settled yet.
    pub pending: bool,
    /// The latest failure for the rendered part; landing and retirement are silent.
    pub failure: Option<PartInputFailure>,
    pub editable: bool,
    pub on_edit: EventHandler<PartInputEditRequest>,
}

/// The shared projection the rendered controls read: the latest committed intent per
/// field. The collection owns the ticket lifetime and settlement.
#[derive(Clone, Copy)]
struct InputDrafts(Signal<Vec<(PartInputKey, PartInputEditRequest)>>);

/// Reapply the committed pending intents for this part over the accepted projection.
pub(super) fn apply_drafts(
    identity: &PartInputIdentity,
    projection: &mut super::PartInputProjection,
) {
    let Some(drafts) = try_consume_context::<InputDrafts>() else {
        return;
    };
    for (_, request) in drafts
        .0
        .read()
        .iter()
        .filter(|(key, _)| key.same_part(identity))
    {
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
    let edits = use_hook(|| Rc::new(RefCell::new(PendingEdits::<PartInputKey>::default())));
    let drafts = use_signal(Vec::<(PartInputKey, PartInputEditRequest)>::new);
    let latest = use_signal(|| None::<PartInputKey>);
    let failure = use_signal(|| None::<PartInputFailure>);
    let settlement_tick = use_signal(|| 0u64);
    let _ = settlement_tick();
    use_context_provider(|| InputDrafts(drafts));
    // Schema preparation is serial so slower module loading cannot reorder committed values.
    let queue = use_hook(|| Rc::new(RefCell::new(VecDeque::<PartInputEditRequest>::new())));
    let preparing = use_hook(|| Rc::new(Cell::new(false)));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });
    let observed_version = version();
    use_effect(use_reactive((&observed_version,), {
        let edits = edits.clone();
        let mut drafts = drafts;
        let latest = latest;
        let mut failure = failure;
        let mut settlement_tick = settlement_tick;
        move |_| {
            // The collection owns the ticket lifetime and retires each ticket whose
            // captured Scope departed; the keyed drafts are this panel's projection.
            let results = edits.borrow_mut().settle(workspace() == "PCB");
            if results.is_empty() {
                return;
            }
            for result in results {
                let (key, message) = match result {
                    PendingEditResult::Failed { key, message } => (key, Some(message)),
                    PendingEditResult::Landed { key, .. } | PendingEditResult::Retired { key } => {
                        (key, None)
                    }
                };
                drafts.write().retain(|(existing, _)| existing != &key);
                if latest.peek().as_ref() == Some(&key) {
                    failure.set(message.map(|message| PartInputFailure::of(&key, message)));
                }
            }
            settlement_tick.set(settlement_tick().wrapping_add(1));
        }
    }));
    let on_edit = use_callback({
        let runtime = runtime.clone();
        let instance_is_current = instance_is_current.clone();
        let edits = edits.clone();
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
            let key = PartInputKey::of(&request);
            let mut latest = latest;
            latest.set(Some(key.clone()));
            let mut failure = failure;
            failure.set(None);
            let mut drafts = drafts;
            let mut entries = drafts.write();
            match entries.iter_mut().find(|(existing, _)| existing == &key) {
                Some(entry) => entry.1 = request.clone(),
                None => entries.push((key.clone(), request.clone())),
            }
            drop(entries);
            queue.borrow_mut().push_back(request);
            if preparing.replace(true) {
                return;
            }
            let queue = queue.clone();
            let preparing = preparing.clone();
            let alive = alive.clone();
            let runtime = runtime.clone();
            let edits = edits.clone();
            spawn_local(async move {
                loop {
                    let request = queue.borrow_mut().pop_front();
                    let Some(request) = request else {
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
                    let key = PartInputKey::of(&request);
                    let live = workspace() == "PCB"
                        && scope_generation() == key.scope_generation
                        && runtime.scope().as_ref() == Some(&key.ui_scope)
                        && runtime.model().selected_part_ids.as_slice() == [key.part_id.as_str()];
                    if !live {
                        continue;
                    }
                    edits.borrow_mut().begin(
                        &runtime,
                        key,
                        "pcb-part-settings",
                        Some("setting".into()),
                        input_resolver(request, schema),
                    );
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
    let live_target = |ui_scope: &Scope, generation: u64, part_id: &str| {
        workspace() == "PCB"
            && generation == scope_generation()
            && runtime.scope().as_ref() == Some(ui_scope)
            && runtime.model().selected_part_ids.as_slice() == [part_id]
    };
    let pending = drafts.read().iter().any(|(key, _)| {
        live_target(&key.ui_scope, key.scope_generation, &key.part_id)
            && edits.borrow().is_pending(key)
    });
    let failure = failure().filter(|failure| {
        live_target(
            &failure.ui_scope,
            failure.scope_generation,
            &failure.part_id,
        )
    });
    PartInputActions {
        pending,
        failure,
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
        Resolution::submit(vec![identity.part_id.clone()], operation)
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
