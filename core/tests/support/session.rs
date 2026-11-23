//! Typed Core requests and reply assertions used by integration tests.

use super::fixture;
use boardstudio_core::{CoreEngine, model::*};

pub(crate) fn temp_dir(label: &str) -> std::path::PathBuf {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let directory = std::env::temp_dir().join(format!(
        "boardstudio-{label}-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[derive(Default)]
pub(crate) struct Session(CoreEngine);

impl Session {
    #[track_caller]
    pub fn opened(document: ProjectDoc) -> (Self, SceneDelta, ProjectDoc) {
        let mut session = Self::default();
        let (scene, document) = fixture::scene_and_document(session.open(document));
        (session, scene, document)
    }

    pub fn send(&mut self, request: CoreRequest) -> CoreReply {
        self.0.handle(request)
    }

    pub fn open(&mut self, document: ProjectDoc) -> CoreReply {
        self.send(fixture::open(document))
    }

    pub fn commit(&mut self, revision: u64, operation: EditOperation) -> CoreReply {
        self.edit(revision, EditPhase::Commit, operation)
    }

    pub fn preview(&mut self, revision: u64, operation: EditOperation) -> CoreReply {
        self.edit(revision, EditPhase::Preview, operation)
    }

    fn edit(&mut self, revision: u64, phase: EditPhase, operation: EditOperation) -> CoreReply {
        self.send(fixture::edit("edit", "edit", revision, phase, operation))
    }

    pub fn undo(&mut self) -> CoreReply {
        self.send(CoreRequest::Undo { id: "undo".into() })
    }

    pub fn redo(&mut self) -> CoreReply {
        self.send(CoreRequest::Redo { id: "redo".into() })
    }

    #[track_caller]
    pub fn snapshot(&mut self) -> ProjectDoc {
        fixture::document(self.send(CoreRequest::Snapshot {
            id: "snapshot".into(),
        }))
    }

    pub fn rev(&mut self) -> u64 {
        self.snapshot().revision
    }
}

/// Open a document on a fresh engine, returning the engine alongside the
/// projected scene and document. Used by tests that drive `CoreEngine`
/// directly instead of through [`Session`].
pub(crate) fn open_engine(document: ProjectDoc) -> (CoreEngine, SceneDelta, ProjectDoc) {
    let mut engine = CoreEngine::new();
    let (scene, document) = fixture::scene_and_document(engine.handle(fixture::open(document)));
    (engine, scene, document)
}

/// Commit an operation at the document revision, returning the new scene and
/// document. Used by tests that drive `CoreEngine` directly.
pub(crate) fn commit_edit(
    engine: &mut CoreEngine,
    doc: &ProjectDoc,
    operation: EditOperation,
) -> (SceneDelta, ProjectDoc) {
    fixture::scene_and_document(engine.handle(fixture::commit(
        "edit",
        "edit",
        doc.revision,
        operation,
    )))
}

#[track_caller]
pub(crate) fn changed(reply: &CoreReply) -> &[String] {
    match reply {
        CoreReply::Scene { scene, .. } => &scene.changed_ids,
        other => panic!("expected committed edit, got {other:?}"),
    }
}

#[track_caller]
pub(crate) fn error(reply: &CoreReply) -> &str {
    match reply {
        CoreReply::Error { message, .. } => message,
        other => panic!("expected an error, got {other:?}"),
    }
}

#[track_caller]
pub(crate) fn one_id(reply: &CoreReply, prefix: &str) -> String {
    let ids: Vec<_> = changed(reply)
        .iter()
        .filter(|id| id.starts_with(prefix))
        .collect();
    assert_eq!(ids.len(), 1, "expected one {prefix} id in {ids:?}");
    ids[0].clone()
}

pub(crate) fn collect_ids<'a, T: 'a>(
    items: impl IntoIterator<Item = &'a T>,
    id: impl Fn(&T) -> &String,
) -> Vec<String> {
    items.into_iter().map(|item| id(item).clone()).collect()
}
