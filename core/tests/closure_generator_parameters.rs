//! Closure clearance holes must be renderable generator definitions.
//!
//! The projection of kept Case closure mounts writes `ceoloide/mounting_hole_npth`
//! parameters. `hole_size` and `hole_drill` are declared as numbers, and the registry
//! rejects anything else only when the footprint is rendered, so a wrongly typed
//! value is invisible until preview or export. These cases keep closure mounts and
//! then render every generator-backed definition and part, preview the board and
//! export it through the same entry points the engine uses.

use std::collections::BTreeMap;

use boardstudio_core::artifact::kicad;
use boardstudio_core::generators;
use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalDimension, MechanicalEdit, MechanicalFeatureCorrespondence,
    MechanicalTarget,
};
use boardstudio_core::model::{
    Board, Contour, ExportTarget, HardwareConfiguration, HardwareTopology, HardwareTransport,
    MechanicalConfiguration, MechanicalConstructionOwnership, MechanicalFeatureKind, Mount,
    MountKind, Operation, OutlineFeature, PartDefinition, PhysicalBoardInstance, Pose2,
    PrepareExportRequest, ProjectDoc, Vec2,
};
use boardstudio_core::model::{CoreReply, CoreRequest};
use boardstudio_core::{CoreEngine, migrate};
use serde_json::{Value, json};

const SOURCE: &str = "ceoloide/mounting_hole_npth";

/// The normalized bundled catalogue entry, exactly as the page supplies it.
fn template() -> PartDefinition {
    generators::catalogue()
        .expect("catalogue")
        .into_iter()
        .find(|entry| entry.generator.as_ref().is_some_and(|g| g.source == SOURCE))
        .expect("bundled mounting hole")
}

fn board(id: &str) -> Board {
    serde_json::from_value(json!({
        "id": id, "name": id, "outlineIds": ["outline"], "partIds": [], "netIds": [],
        "thickness": 1.6,
    }))
    .unwrap()
}

fn configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": "board", "method": "printed", "mount": "rigid",
        "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4, "batteryHeight": 4.0, "bottomThickness": 3.0,
        "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
        "bottomStyle": "shell", "middleFrame": false, "openingAllowance": 0.25,
        "mounts": [], "closureMounts": [], "profiles": []
    }))
    .unwrap()
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("closure", "Closure");
    document.boards.push(board("board"));
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0.0, y: 0.0 },
        size: Vec2 { x: 80.0, y: 60.0 },
        radius: 0.0,
        rotation: None,
        operation: Operation::Add,
    });
    document.mechanical = Some(configuration());
    document
}

fn target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn apply(document: &mut ProjectDoc, change: MechanicalChange) {
    boardstudio_core::mechanical::edit::apply(
        document,
        &[],
        &MechanicalEdit {
            target: target(),
            change,
            mounting_hole: Some(Box::new(template())),
        },
    )
    .expect("mechanical edit");
}

fn instance(id: &str, flipped: bool, mounts: Vec<Mount>) -> PhysicalBoardInstance {
    let mut mechanical = configuration();
    mechanical.closure_mounts = Some(mounts);
    PhysicalBoardInstance {
        id: id.into(),
        name: id.into(),
        board_id: "board".into(),
        half: id.into(),
        role: "controller".into(),
        flipped,
        controller_part_id: None,
        mechanical: Some(mechanical),
        construction_ownership: MechanicalConstructionOwnership::Local,
        mechanical_identity_reservation: None,
    }
}

fn boss(id: &str, x: f64, y: f64, boss_diameter: f64) -> Mount {
    Mount {
        id: id.into(),
        at: Vec2 { x, y },
        kind: MountKind::Boss,
        hole_diameter: 2.2,
        boss_diameter: Some(boss_diameter),
        height: None,
    }
}

fn generated_parts(document: &ProjectDoc) -> Vec<&boardstudio_core::model::Part> {
    document
        .parts
        .iter()
        .filter(|part| part.id.starts_with("case-closure/"))
        .collect()
}

/// Renders every generator-backed definition and part, then previews and exports the
/// board, the way a kept closure mount is eventually consumed.
fn assert_renders_previews_and_exports(document: &ProjectDoc) {
    let generated = generated_parts(document);
    assert!(!generated.is_empty(), "closure mounts were projected");

    for definition in document
        .definitions
        .iter()
        .filter(|definition| definition.generator.is_some())
    {
        generators::render_forms(definition, None)
            .unwrap_or_else(|error| panic!("definition {}: {error}", definition.id));
    }
    for part in &document.parts {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == part.definition_id)
            .unwrap_or_else(|| panic!("definition of {}", part.id));
        if definition.generator.is_some() {
            generators::render_forms(definition, Some(part))
                .unwrap_or_else(|error| panic!("part {}: {error}", part.id));
        }
    }

    let request = || PrepareExportRequest {
        snapshot_token: "closure".into(),
        expected_revision: document.revision,
        document: document.clone(),
        target: ExportTarget::Board {
            board_id: "board".into(),
        },
        contours: vec![Contour {
            hole: false,
            points: vec![
                Vec2 { x: -60.0, y: -40.0 },
                Vec2 { x: 60.0, y: -40.0 },
                Vec2 { x: 60.0, y: 40.0 },
                Vec2 { x: -60.0, y: 40.0 },
            ],
        }],
        model_paths: BTreeMap::new(),
    };
    kicad::preview(request()).unwrap_or_else(|error| panic!("preview: {}", error.message));
    let artifact =
        kicad::export(request()).unwrap_or_else(|error| panic!("export: {}", error.message));
    let board = artifact
        .files
        .iter()
        .find(|file| file.filename.ends_with(".kicad_pcb"))
        .expect("board file");
    assert_eq!(
        board.content.matches("ceoloide:mounting_hole_npth").count(),
        generated.len(),
        "every generated clearance hole is exported"
    );
}

/// The closure definitions carry their numeric generator parameters as numbers.
fn assert_numeric_parameters(document: &ProjectDoc) {
    let closure: Vec<&PartDefinition> = document
        .definitions
        .iter()
        .filter(|definition| definition.id.starts_with("assembly-closure/definition/"))
        .collect();
    assert!(!closure.is_empty());
    for definition in closure {
        let parameters = &definition.generator.as_ref().unwrap().parameters;
        for name in ["hole_drill", "hole_size"] {
            let value = &parameters[name];
            assert!(value.is_number(), "{} {name}: {value}", definition.id);
            let drill = definition.pads[0].drill.expect("drilled pad");
            assert_eq!(value.as_f64(), Some(drill), "{} {name}", definition.id);
        }
    }
}

#[test]
fn proposed_closure_mounts_render_preview_and_export() {
    let mut document = document();
    document.mechanical.as_mut().unwrap().closure_mounts = None;
    // An edit on a configuration without closure mounts proposes and keeps them.
    apply(
        &mut document,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    );
    assert!(
        !document
            .mechanical
            .as_ref()
            .unwrap()
            .closure_mounts
            .as_ref()
            .unwrap()
            .is_empty(),
        "closure mounts were proposed and kept"
    );
    assert_numeric_parameters(&document);
    assert_renders_previews_and_exports(&document);
}

#[test]
fn a_boss_diameter_that_is_not_a_clean_decimal_renders_on_a_flipped_linked_pair() {
    let mut document = document();
    // 2.2 + 2 * 0.3 clearance = 2.8000000000000003.
    document.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: vec![
            instance("left", false, vec![boss("closure-1", -30.0, 25.0, 2.2)]),
            // The flipped half also keeps a mount of its own (3.1 + 0.6 is not clean).
            instance(
                "right",
                true,
                vec![
                    boss("closure-1", -30.0, 25.0, 2.2),
                    boss("local-1", 12.5, -7.25, 3.1),
                ],
            ),
        ],
        boards: vec![],
        shared_construction: None,
        pairs: vec![],
    });
    apply(
        &mut document,
        MechanicalChange::LinkMirrorPair {
            pair_id: "pair-left-right".into(),
            members: ["left".into(), "right".into()],
            mapping: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            construction_source: "left".into(),
            correspondences: vec![MechanicalFeatureCorrespondence {
                kind: MechanicalFeatureKind::ClosureMount,
                first_feature_id: "closure-1".into(),
                second_feature_id: "closure-1".into(),
                source_instance_id: None,
            }],
        },
    );
    assert!(
        generated_parts(&document).len() >= 2,
        "the linked mount and the flipped half's own mount are projected"
    );
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id.starts_with("assembly-closure/definition/"))
        .expect("closure definition");
    assert_eq!(
        definition.generator.as_ref().unwrap().parameters["hole_size"],
        Value::from(2.8000000000000003)
    );
    assert_numeric_parameters(&document);
    assert_renders_previews_and_exports(&document);
}

/// A project saved by the string writer: format version 3 with its closure holes as text.
/// Browser storage and archive import migrate it, Core opens it, and it exports.
#[test]
fn a_version_3_closure_project_is_refused_and_current_numeric_parameters_export() {
    let mut document = document();
    document.mechanical.as_mut().unwrap().closure_mounts = None;
    apply(
        &mut document,
        MechanicalChange::SetDimension {
            field: MechanicalDimension::WallThickness,
            value: 3.0,
        },
    );
    let mut saved = serde_json::to_value(&document).unwrap();
    saved["formatVersion"] = json!(3);
    for definition in saved["definitions"].as_array_mut().unwrap() {
        if !definition["id"]
            .as_str()
            .is_some_and(|id| id.starts_with("assembly-closure/definition/"))
        {
            continue;
        }
        let parameters = definition["generator"]["parameters"]
            .as_object_mut()
            .unwrap();
        for name in ["hole_drill", "hole_size"] {
            let number = parameters[name]
                .as_f64()
                .expect("a numeric closure parameter");
            parameters.insert(name.into(), Value::String(number.to_string()));
        }
    }
    assert!(saved.to_string().contains("\"hole_drill\":\""));

    let original = saved.to_string();
    let error = migrate::migrate_json(&original).unwrap_err();
    assert!(
        error.contains("format 3") && error.contains("No conversion"),
        "{error}"
    );
    assert_eq!(saved.to_string(), original);
    let mut engine = CoreEngine::new();
    let reply = engine.handle(CoreRequest::Open {
        id: "old".into(),
        document: serde_json::from_value(saved).unwrap(),
    });
    assert!(matches!(reply, CoreReply::Error { .. }));
    // The current authored numeric document still opens and exports without conversion.
    assert!(matches!(
        engine.handle(CoreRequest::Open {
            id: "current".into(),
            document: document.clone()
        }),
        CoreReply::Scene { .. }
    ));
    let migrated = document;
    assert_renders_previews_and_exports(&migrated);
}
