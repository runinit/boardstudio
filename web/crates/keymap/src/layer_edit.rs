//! Layer edit requests and their resolver.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_model::model::{EditOperation, KeymapChange};

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerOperation {
    Add,
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerFeedback {
    Failed(String),
}

pub(crate) fn layer_resolver(
    scope: Scope,
    request: KeymapLayerOperation,
    seed: u64,
) -> EditResolver {
    EditResolver::new("keymap-layer", move |accepted: &AcceptedSnapshot| {
        if !accepted
            .document
            .boards
            .iter()
            .any(|b| b.id == scope.board_id)
        {
            return Resolution::Retire("This board no longer exists.".into());
        }
        let map = accepted.document.keymap.as_ref();
        if map.is_some_and(|m| m.layers.is_empty()) {
            return Resolution::Retire("The keymap has no base layer.".into());
        }
        let change = match &request {
            KeymapLayerOperation::Add => {
                let count = map.map_or(1, |m| m.layers.len());
                if count >= 32 {
                    return Resolution::Retire("The keymap already has 32 layers.".into());
                }
                let mut id = format!("keymap-layer-{seed}");
                let mut suffix = 0u64;
                while map.is_some_and(|m| m.layers.iter().any(|l| l.id == id)) {
                    suffix += 1;
                    id = format!("keymap-layer-{seed}-{suffix}");
                }
                KeymapChange::AddLayer {
                    id,
                    name: format!("Layer {count}"),
                }
            }
            KeymapLayerOperation::Rename { layer_id, name } => {
                let cur = map
                    .and_then(|m| m.layers.iter().find(|l| l.id == *layer_id))
                    .map(|l| l.name.as_str())
                    .or_else(|| (map.is_none() && layer_id == "base").then_some("Base"));
                let Some(cur) = cur else {
                    return Resolution::Retire("This layer no longer exists.".into());
                };
                if cur == name {
                    return Resolution::Unchanged;
                }
                KeymapChange::RenameLayer {
                    id: layer_id.clone(),
                    name: name.clone(),
                }
            }
            KeymapLayerOperation::Remove { layer_id } => {
                let Some(idx) = map.and_then(|m| m.layers.iter().position(|l| l.id == *layer_id))
                else {
                    return Resolution::Retire("This layer no longer exists.".into());
                };
                if idx == 0 {
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_model::model::ProjectDoc;
    use boardstudio_web_runtime::runtime::Runtime;
    use std::rc::Rc;

    #[test]
    fn add_rename_remove_layer_round_trip() {
        let mut doc = ProjectDoc::empty("layer-edits", "Layers");
        doc.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        let rt = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
        rt.submit(Event::Open {
            operation_id: rt.operation(),
            document: doc,
        });
        let mut scope = rt.scope().unwrap();
        scope.board_id = "board".into();
        assert!(
            rt.commit(
                "keymap-layer",
                layer_resolver(scope.clone(), KeymapLayerOperation::Add, 7)
            )
            .is_ok()
        );
        let id = rt
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .layers[1]
            .id
            .clone();
        assert!(
            rt.commit(
                "keymap-layer",
                layer_resolver(
                    scope.clone(),
                    KeymapLayerOperation::Rename {
                        layer_id: id.clone(),
                        name: "Symbols".into()
                    },
                    8
                )
            )
            .is_ok()
        );
        assert_eq!(
            rt.model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .layers[1]
                .name,
            "Symbols"
        );
        assert!(
            rt.commit(
                "keymap-layer",
                layer_resolver(scope, KeymapLayerOperation::Remove { layer_id: id }, 9)
            )
            .is_ok()
        );
        assert_eq!(
            rt.model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .layers
                .len(),
            1
        );
    }
}
