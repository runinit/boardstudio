//! Editor-owned layer intents and ticket settlement.
use super::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_core::model::{EditCommand, EditOperation, EditPhase, KeymapChange};
use boardstudio_web_runtime::edit_ticket::{EditTicket, Settlement};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct LayerSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone)]
struct LayerTicket {
    scope: Scope,
    generation: u64,
    request: KeymapLayerOperation,
    ticket: EditTicket,
}

#[derive(Clone)]
struct LayerFeedbackState {
    scope: Scope,
    generation: u64,
    operation: boardstudio_application::OperationId,
    feedback: KeymapLayerFeedback,
}

#[derive(Clone, Copy)]
struct LayerTickets(Signal<Vec<LayerTicket>>);

pub(super) fn action_pending(request: &KeymapLayerOperation) -> bool {
    try_consume_context::<LayerTickets>().is_some_and(|tickets| {
        tickets
            .0
            .read()
            .iter()
            .any(|entry| &entry.request == request && entry.ticket.is_pending())
    })
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
    let pending = use_signal(Vec::<LayerTicket>::new);
    use_context_provider(|| LayerTickets(pending));
    let feedback = use_signal(|| None::<LayerFeedbackState>);
    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            let mut tickets = pending.peek().clone();
            let before = tickets.len();
            tickets.retain(|entry| {
                let live = runtime.scope().as_ref() == Some(&entry.scope)
                    && scope_generation() == entry.generation;
                let result = match entry.ticket.settlement(live) {
                    Settlement::Pending => return true,
                    Settlement::Landed { .. } => Some(KeymapLayerFeedback::Saved),
                    Settlement::Failed { message } => Some(KeymapLayerFeedback::Failed(message)),
                    Settlement::Retired => None,
                };
                if feedback
                    .peek()
                    .as_ref()
                    .is_some_and(|state| state.operation == entry.ticket.operation())
                {
                    feedback.set(result.map(|result| LayerFeedbackState {
                        scope: entry.scope.clone(),
                        generation: entry.generation,
                        operation: entry.ticket.operation(),
                        feedback: result,
                    }));
                }
                false
            });
            if before != tickets.len() {
                pending.set(tickets);
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
                && pending
                    .peek()
                    .iter()
                    .any(|entry| entry.request == request && entry.ticket.is_pending())
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
            let ticket =
                EditTicket::begin(&runtime, "keymap-layer", Some("layer".into()), resolver);
            feedback.set(Some(LayerFeedbackState {
                scope: source.scope.clone(),
                generation: captured_generation,
                operation: ticket.operation(),
                feedback: KeymapLayerFeedback::Pending,
            }));
            pending.write().push(LayerTicket {
                scope: source.scope.clone(),
                generation: captured_generation,
                request,
                ticket,
            });
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

fn layer_resolver(scope: Scope, request: KeymapLayerOperation, seed: u64) -> EditResolver {
    EditResolver::new("keymap-layer", move |accepted: &AcceptedSnapshot| {
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == scope.board_id)
        {
            return Resolution::Retire("This board no longer exists.".into());
        }
        let map = accepted.document.keymap.as_ref();
        if map.is_some_and(|map| map.layers.is_empty()) {
            return Resolution::Retire("The keymap has no base layer.".into());
        }
        let change = match &request {
            KeymapLayerOperation::Add => {
                let count = map.map_or(1, |map| map.layers.len());
                if count >= 32 {
                    return Resolution::Retire("The keymap already has 32 layers.".into());
                }
                let mut id = format!("keymap-layer-{seed}");
                let mut suffix = 0u64;
                while map.is_some_and(|map| map.layers.iter().any(|layer| layer.id == id)) {
                    suffix += 1;
                    id = format!("keymap-layer-{seed}-{suffix}");
                }
                KeymapChange::AddLayer {
                    id,
                    name: format!("Layer {count}"),
                }
            }
            KeymapLayerOperation::Rename { layer_id, name } => {
                let current = map
                    .and_then(|map| map.layers.iter().find(|layer| layer.id == *layer_id))
                    .map(|layer| layer.name.as_str())
                    .or_else(|| (map.is_none() && layer_id == "base").then_some("Base"));
                let Some(current) = current else {
                    return Resolution::Retire("This layer no longer exists.".into());
                };
                if current == name {
                    return Resolution::Unchanged;
                }
                KeymapChange::RenameLayer {
                    id: layer_id.clone(),
                    name: name.clone(),
                }
            }
            KeymapLayerOperation::Remove { layer_id } => {
                let Some(index) =
                    map.and_then(|map| map.layers.iter().position(|layer| layer.id == *layer_id))
                else {
                    return Resolution::Retire("This layer no longer exists.".into());
                };
                if index == 0 {
                    return Resolution::Retire("The base layer cannot be removed.".into());
                }
                KeymapChange::RemoveLayer {
                    id: layer_id.clone(),
                }
            }
        };
        Resolution::submit(
            vec![scope.board_id.clone()],
            EditOperation::EditKeymap { change },
        )
    })
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
