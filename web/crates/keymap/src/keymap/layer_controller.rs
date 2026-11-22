//! Editor-owned layer commands.
use crate::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation, layer_resolver};
use crate::runtime::Runtime;
use boardstudio_application::{
    AcceptedSnapshot, Durability, Lifecycle, Resolution, Scope, SnapshotToken,
};
use boardstudio_model::model::{EditCommand, EditOperation, KeymapChange};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct LayerSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone)]
struct LayerFeedbackState {
    scope: Scope,
    generation: u64,
    feedback: KeymapLayerFeedback,
}

#[derive(Clone, Copy)]
pub(super) struct LayerEditsContext {
    pub(super) name_draft: Signal<String>,
    pub(super) name_failure: Signal<Option<String>>,
}

#[cfg(test)]
pub(super) fn provide_idle_context() {
    let name_draft = use_signal(String::new);
    let name_failure = use_signal(|| None::<String>);
    use_context_provider(|| LayerEditsContext {
        name_draft,
        name_failure,
    });
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
    let captured_generation = scope_generation();
    let mut name_draft = use_signal(String::new);
    let mut name_failure = use_signal(|| None::<String>);
    use_context_provider(|| LayerEditsContext {
        name_draft,
        name_failure,
    });
    let feedback = use_signal(|| None::<LayerFeedbackState>);
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
            let map = snapshot.document.keymap.as_ref();
            if let KeymapLayerOperation::Rename { layer_id, .. }
            | KeymapLayerOperation::Remove { layer_id } = &request
                && resolve_display_layer_id(map, &active_layer()) != Some(layer_id.as_str())
            {
                return;
            }
            let seed = runtime.operation().0;
            let resolver = layer_resolver(source.scope.clone(), request.clone(), seed);
            let added = match resolver.resolve(&snapshot) {
                Resolution::Submit(EditCommand {
                    operation:
                        EditOperation::EditKeymap {
                            change: KeymapChange::AddLayer { id, .. },
                        },
                    ..
                }) => Some(id),
                _ => None,
            };
            let result = runtime.commit("keymap-layer", resolver);
            match request {
                KeymapLayerOperation::Rename { layer_id, .. } => {
                    if let Some(layer) = runtime
                        .model()
                        .accepted
                        .as_ref()
                        .and_then(|snapshot| snapshot.document.keymap.as_ref())
                        .and_then(|map| map.layers.iter().find(|layer| layer.id == layer_id))
                    {
                        name_draft.set(layer.name.clone());
                    } else if layer_id == "base" {
                        name_draft.set("Base".into());
                    }
                    name_failure.set(result.err());
                    feedback.set(None);
                }
                _ => match result {
                    Ok(_) => {
                        if let Some(id) = added {
                            active_layer.set(id);
                        }
                        feedback.set(None);
                    }
                    Err(message) => feedback.set(Some(LayerFeedbackState {
                        scope: source.scope.clone(),
                        generation: captured_generation,
                        feedback: KeymapLayerFeedback::Failed(message),
                    })),
                },
            }
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
    map: Option<&'a boardstudio_model::model::KeymapConfiguration>,
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
