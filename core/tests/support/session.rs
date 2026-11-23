//! Typed Core requests and reply assertions used by integration tests.

use super::fixture;
use boardstudio_core::mechanical::change::{MechanicalChange, MechanicalEdit, MechanicalTarget};
use boardstudio_core::{CoreEngine, model::*};
use serde_json::Value;

#[path = "fixture.rs"]
pub(crate) mod builders;

pub(crate) fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

pub(crate) fn instance_target(id: &str) -> MechanicalTarget {
    MechanicalTarget::Instance {
        instance_id: id.into(),
    }
}

pub(crate) fn wire_commit_to(
    engine: &mut CoreEngine,
    id: &str,
    revision: u64,
    target_ids: &[&str],
    operation: serde_json::Value,
) -> serde_json::Value {
    fixture::request(
        engine,
        serde_json::json!({
            "kind": "edit", "id": id, "command": {
                "baseRevision": revision, "transactionId": id, "phase": "commit",
                "targetIds": target_ids, "operation": operation
            }
        }),
    )
}

pub(crate) fn mechanical_edit(
    target: MechanicalTarget,
    change: MechanicalChange,
    with_template: bool,
) -> MechanicalEdit {
    MechanicalEdit {
        target,
        change,
        mounting_hole: with_template.then(|| Box::new(fixture::mounting_hole())),
    }
}

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

    /// Open a document on a fresh session and return only its scene.
    /// Covers the one-shot `scene(Session::default().open(doc))` projections.
    #[track_caller]
    pub fn projected(document: ProjectDoc) -> SceneDelta {
        fixture::scene(Self::default().open(document))
    }

    /// Commit a mechanical change with the mounting-hole template at the
    /// session revision, returning the raw reply.
    pub fn commit_mechanical(
        &mut self,
        target: MechanicalTarget,
        change: MechanicalChange,
    ) -> CoreReply {
        let base_revision = self.rev();
        self.commit(
            base_revision,
            EditOperation::ApplyMechanicalEdit {
                edit: Box::new(mechanical_edit(target, change, true)),
            },
        )
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

/// A unibody document with one board and its mechanical configuration.
/// Covers the mechanical and identity documents previously built inline.
pub(crate) fn mechanical_doc(id: &str, name: &str) -> ProjectDoc {
    let mut document = ProjectDoc::empty(id, name);
    document.boards.push(fixture::board("board", "Board"));
    document.mechanical = Some(fixture::mechanical("board"));
    document
}

/// A mechanical configuration with an explicitly empty opening list so sparse
/// imported identities seed the high-water mark instead of initializing it.
pub(crate) fn sparse_configuration(board_id: &str) -> MechanicalConfiguration {
    let mut configuration = fixture::mechanical(board_id);
    configuration.openings = Some(vec![]);
    configuration
}

/// [`mechanical_doc`] with an explicitly empty opening list so sparse
/// imported identities seed the high-water mark instead of initializing it.
pub(crate) fn sparse_mechanical_doc(id: &str, name: &str) -> ProjectDoc {
    let mut document = mechanical_doc(id, name);
    document.mechanical.as_mut().unwrap().openings = Some(vec![]);
    document
}

/// One physical board instance with its own mechanical configuration.
/// Unifies the paired-owner constructors previously duplicated per suite.
pub(crate) fn paired_instance(
    id: &str,
    board_id: &str,
    role: &str,
    mechanical: Option<MechanicalConfiguration>,
    construction_ownership: MechanicalConstructionOwnership,
) -> PhysicalBoardInstance {
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: board_id.into(),
        half: id.into(),
        role: role.into(),
        flipped: false,
        controller_part_id: None,
        mechanical,
        construction_ownership,
        mechanical_identity_reservation: None,
    }
}

/// The stored identities of one mechanical namespace.
/// Covers the per-kind id-list helpers previously repeated per suite.
pub(crate) fn mechanical_ids(document: &ProjectDoc, kind: &str) -> Vec<String> {
    let configuration = document.mechanical.as_ref().unwrap();
    match kind {
        "opening" => collect_ids(configuration.openings.iter().flatten(), |item| &item.id),
        "vertex" => configuration
            .openings
            .iter()
            .flatten()
            .flat_map(|opening| opening.vertices.iter())
            .map(|vertex| vertex.id.clone())
            .collect(),
        "fit" => collect_ids(configuration.critical_fits.iter().flatten(), |item| {
            &item.id
        }),
        "hardware" => collect_ids(configuration.hardware.iter().flatten(), |item| &item.id),
        "mount" => collect_ids(&configuration.mounts, |item| &item.id),
        other => panic!("unknown mechanical identity kind {other}"),
    }
}

/// The effective inputs of both `left` and `right` halves.
/// Covers the repeated paired `instance_inputs` sequences.
pub(crate) fn both_instance_inputs(
    document: &ProjectDoc,
) -> (
    boardstudio_core::mechanical::MechanicalInputs,
    boardstudio_core::mechanical::MechanicalInputs,
) {
    use boardstudio_core::mechanical::effective_inputs;
    let inputs = |id: &str| {
        effective_inputs(document, &[], &instance_target(id))
            .unwrap_or_else(|error| panic!("effective mechanical inputs for {id}: {error:?}"))
    };
    (inputs("left"), inputs("right"))
}

/// Open the shared mounted-module host document on a fresh engine.
/// Covers the repeated open-the-host-document test setups.
#[track_caller]
pub(crate) fn module_engine() -> CoreEngine {
    let mut engine = CoreEngine::new();
    fixture::wire_open(
        &mut engine,
        "open",
        &crate::support::query::module_host_document(),
    );
    engine
}

/// The wire snapshot's document without the reply envelope.
#[track_caller]
pub(crate) fn wire_snapshot_doc(engine: &mut CoreEngine) -> Value {
    fixture::wire_snapshot(engine, "snapshot")["document"].clone()
}

/// Send a mechanical resolution request, returning its assembly.
/// Shares the `resolve-mechanical` invocation previously repeated per suite.
#[track_caller]
pub(crate) fn resolve_request(request: Value) -> Value {
    let reply: Value =
        serde_json::from_str(&CoreEngine::new().request(&request.to_string())).unwrap();
    reply["assembly"].clone()
}

/// Resolve a mechanical document against a `width` x `height` contour.
/// Covers the repeated `resolve-mechanical` request sequences.
#[track_caller]
pub(crate) fn resolve_mechanical(document: Value, width: f64, height: f64) -> Value {
    resolve_request(
        serde_json::json!({"kind": "resolve-mechanical", "id": "resolve",
        "document": document, "contours": [{"hole": false, "points": [
            {"x": 0, "y": 0}, {"x": width, "y": 0},
            {"x": width, "y": height}, {"x": 0, "y": height}]}]}),
    )
}

/// Run a KiCad CLI command, asserting success.
/// Shares the `kicad-cli` invocation previously duplicated per suite.
#[track_caller]
pub(crate) fn run_kicad_cli(arguments: &[&str]) {
    let output = crate::support::kicad_command("kicad-cli")
        .args(arguments)
        .output()
        .expect("kicad-cli (KiCad 10) is required for the KiCad integration checks");
    assert!(
        output.status.success(),
        "kicad-cli {arguments:?}: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Whether KiCad 10 parses generated boards.
pub(crate) fn kicad10_available() -> bool {
    crate::support::kicad_command("kicad-cli")
        .arg("version")
        .output()
        .map(|output| {
            String::from_utf8_lossy(&output.stdout)
                .trim()
                .starts_with("10.")
        })
        .unwrap_or(false)
}

/// Whether KiCad 10 parses an exported board, via `pcb export stats`.
#[track_caller]
pub(crate) fn kicad_board_parses(content: &str) -> bool {
    let directory = temp_dir("generator-kicad");
    let board = directory.join("main.kicad_pcb");
    std::fs::write(&board, content).unwrap();
    let report = directory.join("stats.json");
    let status = crate::support::kicad_command("kicad-cli")
        .args(["pcb", "export", "stats", "--format", "json", "--output"])
        .arg(&report)
        .arg(&board)
        .output()
        .unwrap();
    let _ = std::fs::remove_dir_all(&directory);
    status.status.success()
}
