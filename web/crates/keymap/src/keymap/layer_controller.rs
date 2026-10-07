//! Editor-owned layer intents and ticket settlement.
use super::observed_edits::ObservedEdits;
use crate::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation, layer_resolver};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, KeymapChange};
use boardstudio_web_runtime::pending_edits::PendingEditResult;
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct LayerSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

/// A layer operation's logical identity: the latest edit for it replaces the earlier one.
#[derive(Clone, Debug, PartialEq)]
enum LayerKey {
    Add,
    Rename(String),
    Remove(String),
}

impl From<&KeymapLayerOperation> for LayerKey {
    fn from(request: &KeymapLayerOperation) -> Self {
        match request {
            KeymapLayerOperation::Add => Self::Add,
            KeymapLayerOperation::Rename { layer_id, .. } => Self::Rename(layer_id.clone()),
            KeymapLayerOperation::Remove { layer_id } => Self::Remove(layer_id.clone()),
        }
    }
}

struct LayerOwner {
    scope: Scope,
    generation: u64,
}

type LayerPending = ObservedEdits<LayerKey, LayerOwner>;

#[derive(Clone)]
struct LayerFeedbackState {
    scope: Scope,
    generation: u64,
    operation: boardstudio_application::OperationId,
    feedback: KeymapLayerFeedback,
}

#[derive(Clone, Copy)]
struct LayerTickets(Signal<LayerPending>);

pub(super) fn action_pending(request: &KeymapLayerOperation) -> bool {
    try_consume_context::<LayerTickets>()
        .is_some_and(|tickets| tickets.0.read().is_pending(&LayerKey::from(request)))
}

pub struct LayerActions {
    pub enabled: bool,
    pub feedback: Option<KeymapLayerFeedback>,
    pub on_operation: EventHandler<KeymapLayerOperation>,
}

pub fn use_layer_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> LayerActions {
    let version = use_context::<Signal<u64>>()();
    let captured_generation = scope_generation();
    let pending = use_signal(LayerPending::default);
    use_context_provider(|| LayerTickets(pending));
    let feedback = use_signal(|| None::<LayerFeedbackState>);
    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            if !pending.peek().has_terminal() {
                return;
            }
            for (observation, result) in pending.write().settle() {
                let owner = observation.meta;
                let live = runtime.scope().as_ref() == Some(&owner.scope)
                    && scope_generation() == owner.generation;
                let outcome = match result {
                    PendingEditResult::Failed { message, .. } if live => {
                        Some(KeymapLayerFeedback::Failed(message))
                    }
                    _ => None,
                };
                if feedback
                    .peek()
                    .as_ref()
                    .is_some_and(|state| state.operation == observation.operation)
                {
                    feedback.set(outcome.map(|feedback| LayerFeedbackState {
                        scope: owner.scope,
                        generation: owner.generation,
                        operation: observation.operation,
                        feedback,
                    }));
                }
            }
        }
    }));
    let enabled = current_source(
        &runtime,
        source.as_ref(),
        captured_generation,
        scope_generation,
        workspace(),
        admission_current.as_ref(),
    )
    .is_some();
    let visible_feedback = feedback
        .read()
        .as_ref()
        .filter(|entry| {
            runtime.scope().as_ref() == Some(&entry.scope) && entry.generation == scope_generation()
        })
        .map(|entry| entry.feedback.clone());
    let on_operation = EventHandler::new({
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        let mut active_layer = active_layer;
        move |request: KeymapLayerOperation| {
            let Some(snapshot) = current_source(
                &runtime,
                source.as_ref(),
                captured_generation,
                scope_generation,
                workspace(),
                admission_current.as_ref(),
            ) else {
                return;
            };
            let Some(source) = source.as_ref() else {
                return;
            };
            if !matches!(request, KeymapLayerOperation::Rename { .. })
                && pending.peek().is_pending(&LayerKey::from(&request))
            {
                return;
            }
            let map = snapshot.document.keymap.as_ref();
            if let KeymapLayerOperation::Rename { layer_id, .. }
            | KeymapLayerOperation::Remove { layer_id } = &request
            {
                if resolve_display_layer_id(map, &active_layer()) != Some(layer_id.as_str()) {
                    return;
                }
            }
            let seed = runtime.operation().0;
            let resolver = layer_resolver(source.scope.clone(), request.clone(), seed);
            if matches!(request, KeymapLayerOperation::Add) {
                if let Resolution::Submit(EditCommand {
                    operation:
                        EditOperation::EditKeymap {
                            change: KeymapChange::AddLayer { id, .. },
                        },
                    ..
                }) = resolver.resolve(&snapshot)
                {
                    active_layer.set(id);
                }
            }
            let operation = pending.write().begin(
                &runtime,
                LayerKey::from(&request),
                "keymap-layer",
                "layer",
                LayerOwner {
                    scope: source.scope.clone(),
                    generation: captured_generation,
                },
                resolver,
            );
            feedback.set(Some(LayerFeedbackState {
                scope: source.scope.clone(),
                generation: captured_generation,
                operation,
                feedback: KeymapLayerFeedback::Pending,
            }));
        }
    });
    LayerActions {
        enabled,
        feedback: visible_feedback,
        on_operation,
    }
}

fn current_source(
    runtime: &Runtime,
    source: Option<&LayerSource>,
    expected_generation: u64,
    generation: Signal<u64>,
    workspace: &'static str,
    admission: &dyn Fn() -> bool,
) -> Option<AcceptedSnapshot> {
    let source = source?;
    let model = runtime.model();
    let accepted = model.accepted?;
    (workspace == "Keymap"
        && admission()
        && generation() == expected_generation
        && runtime.scope().as_ref() == Some(&source.scope)
        && accepted.token == source.token
        && accepted.document.revision == source.revision
        && matches!(
            model.lifecycle,
            Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
        )
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        )
        && model.display_preview.is_none()
        && model.gesture.is_none())
    .then_some(accepted)
}

fn resolve_display_layer_id<'a>(
    map: Option<&'a boardstudio_core::model::KeymapConfiguration>,
    requested: &str,
) -> Option<&'a str> {
    let Some(map) = map.filter(|map| !map.layers.is_empty()) else {
        return Some("base");
    };
    map.layers
        .iter()
        .find(|layer| layer.id == requested)
        .or_else(|| map.layers.first())
        .map(|layer| layer.id.as_str())
}
