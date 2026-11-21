//! Typed Core outline edits and atomic version lifecycle (typed Core edits 13).
//!
//! One `apply-outline-edit` operation carries a typed target and named change; Core owns
//! validation, identity allocation from the captured seed, mutation and the single Undo
//! step. Expectations here are hand-derived from the fixture's millimetre geometry
//! (a switch courtyard of 14x14 with margin 1 gives a 16x16 envelope centred on its pose).
use boardstudio_core::outline_edit::{
    OutlineChange, OutlineEdit, OutlineEditDisposition, OutlineEditTarget, evaluate,
};
use boardstudio_core::{CoreEngine, model::*};

const ENVELOPE: &str = "board:board:version:refined:contour:0";

fn settings() -> serde_json::Value {
    serde_json::json!({
        "corners": "sharp", "size": 2.0, "bridgeWidth": 10.0,
        "repair": {
            "enabled": true, "maximumGapSpan": 20.0,
            "minimumConnectionWidth": 2.0, "edgeClearance": 0.0, "keepGaps": []
        }
    })
}

fn definition() -> serde_json::Value {
    serde_json::json!({
        "id": "switch", "name": "Switch", "kind": "switch",
        "courtyard": [
            { "x": -7.0, "y": -7.0 }, { "x": 7.0, "y": -7.0 },
            { "x": 7.0, "y": 7.0 }, { "x": -7.0, "y": 7.0 }
        ],
        "pads": [{ "id": "p0", "number": "1", "at": { "x": 0.0, "y": 0.0 },
                   "size": { "x": 4.0, "y": 4.0 }, "shape": "rect" }],
    })
}

/// Board "board" owns parts a (origin) and b (14,0); board "spare" owns part c far away.
fn fixture(b_at: (f64, f64)) -> serde_json::Value {
    let mut document = serde_json::to_value(ProjectDoc::empty("edits", "Edits")).unwrap();
    document["definitions"] = serde_json::json!([definition()]);
    document["parts"] = serde_json::json!([
        { "id": "a", "definitionId": "switch", "reference": "SW1", "side": "front",
          "pose": { "at": { "x": 0.0, "y": 0.0 }, "rotation": 0.0 } },
        { "id": "b", "definitionId": "switch", "reference": "SW2", "side": "front",
          "pose": { "at": { "x": b_at.0, "y": b_at.1 }, "rotation": 0.0 } },
        { "id": "c", "definitionId": "switch", "reference": "SW3", "side": "front",
          "pose": { "at": { "x": 100.0, "y": 0.0 }, "rotation": 0.0 } }
    ]);
    document["boards"] = serde_json::json!([
        { "id": "board", "name": "Board", "outlineIds": ["gen"], "partIds": ["a", "b"],
          "netIds": [], "thickness": 1.6 },
        { "id": "spare", "name": "Spare", "outlineIds": ["spare-gen"], "partIds": ["c"],
          "netIds": [], "thickness": 1.6 }
    ]);
    document["outline"] = serde_json::json!([
        { "kind": "part-envelope", "id": "gen", "partIds": ["a", "b"], "margin": 1.0,
          "operation": "add", "settings": settings() },
        { "kind": "part-envelope", "id": "spare-gen", "partIds": ["c"], "margin": 1.0,
          "operation": "add", "settings": settings() }
    ]);
    document
}

/// Board "board" with no Generated envelope at all (the CreateGenerated starting state).
fn fixture_without_generated() -> serde_json::Value {
    let mut document = fixture((14.0, 0.0));
    document["outline"] = serde_json::json!([{
        "kind": "part-envelope", "id": "spare-gen", "partIds": ["c"], "margin": 1.0,
        "operation": "add", "settings": settings()
    }]);
    document["boards"][0]["outlineIds"] = serde_json::json!([]);
    document
}

fn scene(reply: CoreReply) -> SceneDelta {
    match reply {
        CoreReply::Scene { scene, .. } => scene,
        reply => panic!("{reply:?}"),
    }
}

fn open(b_at: (f64, f64)) -> (CoreEngine, SceneDelta) {
    let mut engine = CoreEngine::new();
    let opened = scene(engine.handle(CoreRequest::Open {
        id: "open".into(),
        document: serde_json::from_value(fixture(b_at)).unwrap(),
    }));
    (engine, opened)
}

fn reply(engine: &mut CoreEngine, revision: u64, edit: serde_json::Value) -> CoreReply {
    let operation = serde_json::from_value(serde_json::json!({
        "kind": "apply-outline-edit", "edit": edit
    }))
    .expect("the typed outline operation parses");
    engine.handle(CoreRequest::Edit {
        id: "outline-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "outline-edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    })
}

fn edit(engine: &mut CoreEngine, revision: u64, edit: serde_json::Value) -> SceneDelta {
    scene(reply(engine, revision, edit))
}

/// A non-outline operation (for example a component move) submitted at the same seam.
fn raw_edit(engine: &mut CoreEngine, revision: u64, operation: serde_json::Value) -> SceneDelta {
    let operation = serde_json::from_value(operation).expect("the operation parses");
    scene(engine.handle(CoreRequest::Edit {
        id: "raw-edit".into(),
        command: EditCommand {
            base_revision: revision,
            transaction_id: "raw-edit".into(),
            phase: EditPhase::Commit,
            target_ids: vec![],
            operation,
        },
    }))
}

fn document(engine: &mut CoreEngine) -> ProjectDoc {
    match engine.handle(CoreRequest::Snapshot {
        id: "snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        reply => panic!("{reply:?}"),
    }
}

fn board_state(doc: &ProjectDoc) -> Option<&BoardOutline> {
    doc.board_outlines
        .iter()
        .find(|state| state.board_id == "board")
}

fn version_of<'a>(doc: &'a ProjectDoc, id: &str) -> Option<&'a OutlineVersion> {
    board_state(doc)?
        .versions
        .iter()
        .find(|version| version.id == id)
}

fn create_refined(engine: &mut CoreEngine, revision: u64) -> SceneDelta {
    edit(
        engine,
        revision,
        serde_json::json!({
            "target": { "kind": "board", "boardId": "board" },
            "change": { "create-linked-refinement": {
                "version_id": "refined", "name": "Refined"
            } },
            "seed": 4
        }),
    )
}

fn sorted(mut ids: Vec<String>) -> Vec<String> {
    ids.sort();
    ids
}

fn undo(engine: &mut CoreEngine) -> SceneDelta {
    scene(engine.handle(CoreRequest::Undo { id: "undo".into() }))
}

fn redo(engine: &mut CoreEngine) -> SceneDelta {
    scene(engine.handle(CoreRequest::Redo { id: "redo".into() }))
}

fn board_bounds(scene: &SceneDelta, board_id: &str) -> (f64, f64, f64, f64) {
    let contours = scene
        .board_contours
        .iter()
        .find(|board| board.board_id == board_id)
        .map(|board| board.contours.as_slice())
        .unwrap_or(&[]);
    bounds(contours)
}

fn bounds(contours: &[Contour]) -> (f64, f64, f64, f64) {
    contours.iter().flat_map(|contour| &contour.points).fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, min_y, max_x, max_y), point| {
            (
                min_x.min(point.x),
                min_y.min(point.y),
                max_x.max(point.x),
                max_y.max(point.y),
            )
        },
    )
}

fn reference_generated_settings() -> OutlineSettings {
    OutlineSettings {
        corners: CornerStyle::Fillet,
        size: 2.0,
        bridge_width: 10.0,
        repair: None,
    }
}

trait FixtureAccess {
    fn part_envelope_margin(&self) -> f64;
    fn connections(&self) -> Vec<OutlineConnection>;
    fn keep_gap_ids(&self) -> Vec<String>;
}

impl FixtureAccess for OutlineFeature {
    fn part_envelope_margin(&self) -> f64 {
        match self {
            OutlineFeature::PartEnvelope { margin, .. } => *margin,
            _ => panic!("expected the generated envelope"),
        }
    }
    fn connections(&self) -> Vec<OutlineConnection> {
        match self {
            OutlineFeature::PartEnvelope { connections, .. } => connections.clone(),
            _ => panic!("expected the generated envelope"),
        }
    }
    fn keep_gap_ids(&self) -> Vec<String> {
        match self {
            OutlineFeature::PartEnvelope { settings, .. } => settings
                .repair
                .iter()
                .flat_map(|repair| repair.keep_gaps.iter().map(|gap| gap.id.clone()))
                .collect(),
            _ => panic!("expected the generated envelope"),
        }
    }
}

trait ContourFromFeature {
    fn from_feature(feature: &OutlineFeature) -> Contour;
}

impl ContourFromFeature for Contour {
    fn from_feature(feature: &OutlineFeature) -> Contour {
        match feature {
            OutlineFeature::Polygon { points, .. } => Contour {
                points: points.clone(),
                hole: false,
            },
            other => panic!("expected an authored polygon, got {other:?}"),
        }
    }
}

#[path = "outline_edits/edits_generated.rs"]
mod edits_generated;
#[path = "outline_edits/edits_settings.rs"]
mod edits_settings;
#[path = "outline_edits/edits_versions.rs"]
mod edits_versions;
