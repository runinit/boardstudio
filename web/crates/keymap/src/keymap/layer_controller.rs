//! Editor-owned layer intents and ticket settlement.
use super::owned_edits::OwnedEdits;
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

type LayerPending = OwnedEdits<LayerKey>;

#[derive(Clone)]
struct LayerFeedbackState {
    scope: Scope,
    generation: u64,
    key: LayerKey,
    feedback: KeymapLayerFeedback,
}

/// Controller-lifetime state: the observed edits and the layer-name field's Signals, which
/// outlive the panel so a settlement never writes a dropped Signal.
#[derive(Clone, Copy)]
pub(super) struct LayerEditsContext {
    edits: Signal<LayerPending>,
    pub(super) name_draft: Signal<String>,
    pub(super) name_failure: Signal<Option<String>>,
}

pub(super) fn action_pending(request: &KeymapLayerOperation) -> bool {
    try_consume_context::<LayerEditsContext>()
        .is_some_and(|context| context.edits.read().is_pending(&LayerKey::from(request)))
}

/// The accepted name of the layer a rename key addresses.
fn accepted_layer_name(runtime: &Runtime, key: &LayerKey) -> String {
    let LayerKey::Rename(layer_id) = key else {
        return String::new();
    };
    let accepted = runtime.model().accepted;
    let map = accepted
        .as_ref()
        .and_then(|snapshot| snapshot.document.keymap.as_ref());
    map.and_then(|map| map.layers.iter().find(|layer| layer.id == *layer_id))
        .map(|layer| layer.name.clone())
        .or_else(|| (map.is_none() && layer_id == "base").then(|| "Base".to_owned()))
        .unwrap_or_default()
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
    let name_draft = use_signal(String::new);
    let name_failure = use_signal(|| None::<String>);
    use_context_provider(|| LayerEditsContext {
        edits: pending,
        name_draft,
        name_failure,
    });
    let feedback = use_signal(|| None::<LayerFeedbackState>);
    let bound_layer = use_hook(|| Rc::new(std::cell::RefCell::new(None::<String>)));
    {
        // Only the displayed layer's name field is bound, so an older layer's outcome can
        // never reach the Signals now shown for another layer.
        let displayed = runtime.model().accepted.and_then(|snapshot| {
            resolve_display_layer_id(snapshot.document.keymap.as_ref(), &active_layer())
                .map(str::to_owned)
        });
        let mut bound = bound_layer.borrow_mut();
        if *bound != displayed {
            if let Some(old) = bound.take() {
                pending.peek().unbind_field(&LayerKey::Rename(old));
            }
            if let Some(layer_id) = displayed.clone() {
                pending
                    .peek()
                    .bind_field(LayerKey::Rename(layer_id), name_draft, name_failure);
            }
            *bound = displayed;
        }
    }
    use_effect(use_reactive((&version,), {
        let runtime = runtime.clone();
        let mut pending = pending;
        let mut feedback = feedback;
        move |_| {
            // A changed owner retires the observations of the previous one.
            let owner_scope = runtime.scope();
            let owner_generation = scope_generation();
            if pending
                .peek()
                .owner_changed(owner_scope.as_ref(), owner_generation)
            {
                pending
                    .write()
                    .follow_owner(owner_scope.as_ref(), owner_generation);
            }
            if !pending.peek().has_terminal() {
                return;
            }
            let results = pending
                .peek()
                .helper
                .settle(true, |key| accepted_layer_name(&runtime, key));
            for result in results {
                let (PendingEditResult::Landed { key, .. }
                | PendingEditResult::Failed { key, .. }
                | PendingEditResult::Retired { key }) = &result;
                let outcome = match &result {
                    PendingEditResult::Failed { message, .. } => {
                        Some(KeymapLayerFeedback::Failed(message.clone()))
                    }
                    _ => None,
                };
                let matches = feedback
                    .peek()
                    .as_ref()
                    .is_some_and(|state| state.key == *key);
                if matches {
                    let (scope, generation) = feedback
                        .peek()
                        .as_ref()
                        .map(|state| (state.scope.clone(), state.generation))
                        .expect("matched feedback");
                    feedback.set(outcome.map(|feedback| LayerFeedbackState {
                        scope,
                        generation,
                        key: key.clone(),
                        feedback,
                    }));
                }
            }
            pending.write().prune();
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
            if pending
                .peek()
                .owner_changed(Some(&source.scope), captured_generation)
            {
                pending
                    .write()
                    .follow_owner(Some(&source.scope), captured_generation);
            }
            let key = LayerKey::from(&request);
            let submitted = match &request {
                KeymapLayerOperation::Rename { name, .. } => name.clone(),
                _ => String::new(),
            };
            pending.peek().helper.begin_field(
                &runtime,
                key.clone(),
                "keymap-layer",
                Some("layer".into()),
                resolver,
                &submitted,
            );
            pending.write().remember(key.clone());
            feedback.set(Some(LayerFeedbackState {
                scope: source.scope.clone(),
                generation: captured_generation,
                key,
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
