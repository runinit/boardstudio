//! Shared `CoreEngine` open/edit harness for layout-core's native tests.
//!
//! `anchor_pick`, `matrix_relations` and `matrix_transform_operation` tests
//! open documents the same way; one helper keeps the open/edit envelopes in
//! a single place instead of three copies.
use boardstudio_core::CoreEngine;
use boardstudio_model::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, ProjectDoc, SceneDelta,
};

/// Open a document in a scratch engine; panics unless Core returns a scene.
pub fn open_engine(document: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    match engine.handle(CoreRequest::Open {
        id: "open".into(),
        document,
    }) {
        CoreReply::Scene {
            scene, document, ..
        } => (engine, scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

/// Commit one typed operation against the document's revision; panics unless
/// Core returns a scene.
pub fn edit_engine(
    engine: &mut CoreEngine,
    document: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: document.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }) {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}

/// Commit one JSON operation (parsed as a typed edit); panics unless Core
/// returns a scene.
pub fn edit_json(
    engine: &mut CoreEngine,
    revision: u64,
    operation: serde_json::Value,
) -> (SceneDelta, ProjectDoc) {
    match engine.handle(CoreRequest::Edit {
        id: "edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation: serde_json::from_value(operation).expect("the operation parses"),
        },
    }) {
        CoreReply::Scene {
            scene, document, ..
        } => (scene, *document),
        other => panic!("expected scene: {other:?}"),
    }
}
