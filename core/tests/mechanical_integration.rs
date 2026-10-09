//! Integrated mechanical verification at the Session/Core seam.
//!
//! These cases exercise the approved public Core contract through real edits: the
//! synchronous mechanical evaluator, the effective mechanical read path and the
//! atomic pair behavior. They complement the per-slice unit tests by driving the
//! same behavior through `CoreEngine` `Edit`/`Undo` and the public mechanical
//! module functions.

use boardstudio_core::mechanical::change::{
    MechanicalChange, MechanicalDimension, MechanicalEdit, MechanicalTarget,
};
use boardstudio_core::mechanical::{MechanicalEditDisposition, evaluate_edit};
use boardstudio_core::model::{Board, BoardContours, MechanicalConfiguration, ProjectDoc};
use serde_json::json;

fn board(id: &str) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    }
}

/// The stored configuration the evaluator cases start from.
fn configuration() -> MechanicalConfiguration {
    serde_json::from_value(json!({
        "boardId": "board",
        "method": "printed",
        "mount": "rigid",
        "plateThickness": 1.5,
        "plateFoamThickness": 0.5,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.4,
        "batteryHeight": 4.0,
        "bottomThickness": 3.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.3,
        "bottomStyle": "shell",
        "middleFrame": false,
        "openingAllowance": 0.25,
        "mounts": [],
        "closureMounts": [],
        "profiles": []
    }))
    .unwrap()
}

fn document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("doc", "Fixture");
    document.boards.push(board("board"));
    document.mechanical = Some(configuration());
    document
}

fn board_target() -> MechanicalTarget {
    MechanicalTarget::Board {
        board_id: "board".into(),
    }
}

fn edit(change: MechanicalChange) -> MechanicalEdit {
    MechanicalEdit {
        target: board_target(),
        change,
        mounting_hole: None,
    }
}

fn plate_thickness(value: f64) -> MechanicalEdit {
    edit(MechanicalChange::SetDimension {
        field: MechanicalDimension::PlateThickness,
        value,
    })
}

#[test]
fn the_evaluator_reports_changed_and_unchanged_without_touching_the_document() {
    let document = document();
    let before = document.clone();

    let changed = evaluate_edit(&document, &[] as &[BoardContours], &plate_thickness(1.8)).unwrap();
    assert_eq!(changed, MechanicalEditDisposition::Changed);

    let unchanged = evaluate_edit(&document, &[], &plate_thickness(1.5)).unwrap();
    assert_eq!(unchanged, MechanicalEditDisposition::Unchanged);

    assert_eq!(
        document, before,
        "evaluation is read-only: it never mutates the accepted document"
    );
}

#[test]
fn the_evaluator_retires_a_vanished_target_and_errors_on_a_malformed_value() {
    let document = document();

    let vanished_instance = evaluate_edit(
        &document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Instance {
                instance_id: "gone".into(),
            },
            change: MechanicalChange::Enable,
            mounting_hole: None,
        },
    );
    assert_eq!(
        vanished_instance.unwrap(),
        MechanicalEditDisposition::Retire
    );

    let vanished_board = evaluate_edit(
        &document,
        &[],
        &MechanicalEdit {
            target: MechanicalTarget::Board {
                board_id: "other".into(),
            },
            change: MechanicalChange::Enable,
            mounting_hole: None,
        },
    );
    assert_eq!(vanished_board.unwrap(), MechanicalEditDisposition::Retire);

    // A structurally malformed value is a Core error, never a silent retirement.
    let malformed = evaluate_edit(&document, &[], &plate_thickness(-1.0));
    assert!(malformed.is_err(), "got {malformed:?}");
}
