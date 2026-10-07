//! Layer edit requests and their resolver. The module compiles natively so its tests
//! drive the real Session and Core through the native Runtime; the Keymap panel binds
//! the resolver to its layer controls.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_core::model::{EditOperation, KeymapChange};

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerOperation {
    Add,
    Rename { layer_id: String, name: String },
    Remove { layer_id: String },
}

#[derive(Clone, Debug, PartialEq)]
pub enum KeymapLayerFeedback {
    Pending,
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

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_core::model::ProjectDoc;
    use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
    use boardstudio_web_runtime::runtime::Runtime;
    use std::rc::Rc;

    fn open() -> (Rc<Runtime>, Scope) {
        let mut document = ProjectDoc::empty("layer-edits", "Layers");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        let runtime = Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let mut scope = runtime.scope().expect("open document has a scope");
        scope.board_id = "board".into();
        (runtime, scope)
    }

    fn settle(
        runtime: &Rc<Runtime>,
        scope: &Scope,
        request: KeymapLayerOperation,
        seed: u64,
    ) -> PendingEditResult<u8> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            0u8,
            "keymap-layer",
            Some("layer".into()),
            layer_resolver(scope.clone(), request, seed),
        );
        let mut results = edits.settle(true);
        assert_eq!(results.len(), 1, "{results:?}");
        results.remove(0)
    }

    fn layer_names(runtime: &Runtime) -> Vec<String> {
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .map(|map| map.layers.iter().map(|layer| layer.name.clone()).collect())
            .unwrap_or_default()
    }

    #[test]
    fn add_rename_and_remove_land_against_the_accepted_keymap() {
        let (runtime, scope) = open();
        let added = settle(&runtime, &scope, KeymapLayerOperation::Add, 7);
        assert!(
            matches!(added, PendingEditResult::Landed { .. }),
            "{added:?}"
        );
        let names = layer_names(&runtime);
        assert_eq!(names.len(), 2, "{names:?}");

        let map = runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .clone()
            .unwrap();
        let added_id = map.layers[1].id.clone();
        let renamed = settle(
            &runtime,
            &scope,
            KeymapLayerOperation::Rename {
                layer_id: added_id.clone(),
                name: "Symbols".into(),
            },
            8,
        );
        assert!(
            matches!(renamed, PendingEditResult::Landed { .. }),
            "{renamed:?}"
        );
        assert_eq!(layer_names(&runtime)[1], "Symbols");

        let removed = settle(
            &runtime,
            &scope,
            KeymapLayerOperation::Remove { layer_id: added_id },
            9,
        );
        assert!(
            matches!(removed, PendingEditResult::Landed { .. }),
            "{removed:?}"
        );
        assert_eq!(layer_names(&runtime).len(), 1);
    }

    #[test]
    fn unchanged_rename_lands_without_a_new_revision() {
        let (runtime, scope) = open();
        let before = runtime.model().accepted.unwrap().document.revision;
        let result = settle(
            &runtime,
            &scope,
            KeymapLayerOperation::Rename {
                layer_id: "base".into(),
                name: "Base".into(),
            },
            1,
        );
        assert_eq!(
            result,
            PendingEditResult::Landed {
                key: 0,
                revision: before,
            }
        );
    }

    #[test]
    fn stale_targets_fail_with_the_resolver_message() {
        let (runtime, scope) = open();
        let base = settle(
            &runtime,
            &scope,
            KeymapLayerOperation::Remove {
                layer_id: "base".into(),
            },
            1,
        );
        assert!(
            matches!(&base, PendingEditResult::Failed { message, .. } if message.contains("no longer exists")),
            "{base:?}"
        );
        let missing = settle(
            &runtime,
            &scope,
            KeymapLayerOperation::Rename {
                layer_id: "ghost".into(),
                name: "Ghost".into(),
            },
            2,
        );
        assert!(
            matches!(&missing, PendingEditResult::Failed { message, .. } if message.contains("no longer exists")),
            "{missing:?}"
        );
        let mut elsewhere = scope.clone();
        elsewhere.board_id = "gone".into();
        let board = settle(&runtime, &elsewhere, KeymapLayerOperation::Add, 3);
        assert!(
            matches!(&board, PendingEditResult::Failed { message, .. } if message.contains("board no longer exists")),
            "{board:?}"
        );
        assert_eq!(layer_names(&runtime).len(), 0);
    }
}
