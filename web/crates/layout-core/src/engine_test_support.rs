//! Shared Core open/edit harness for layout tests.
use boardstudio_core::{CommittedScene, CoreEngine};
use boardstudio_model::model::{EditCommand, EditOperation, EditPhase, ProjectDoc, SceneDelta};

pub fn open_engine(document: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let CommittedScene { scene, document } = engine.open(document).expect("Core opens the fixture");
    (engine, scene, document)
}

pub fn edit_engine(
    engine: &mut CoreEngine,
    document: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    let committed = engine
        .commit_edit(EditCommand {
            base_revision: document.revision,
            transaction_id: "edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        })
        .expect("Core accepts the fixture edit");
    (committed.scene, committed.document)
}
