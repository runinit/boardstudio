#[path = "tests/cutouts.rs"]
mod cutouts;
#[path = "tests/stack.rs"]
mod stack;
#[path = "tests/switch_fits.rs"]
mod switch_fits;

use super::*;

pub(super) fn empty_board(id: &str, thickness: f64) -> Board {
    Board {
        id: id.into(),
        name: id.into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness,
        traces: vec![],
        vias: vec![],
    }
}

pub(super) fn configuration(board_id: &str) -> MechanicalConfiguration {
    serde_json::from_value(serde_json::json!({
        "boardId": board_id,
        "method": "printed",
        "mount": "tray",
        "plateThickness": 1.5,
        "plateFoamThickness": 1.0,
        "pcbThickness": 1.6,
        "bottomFoamThickness": 0.5,
        "batteryHeight": 0.0,
        "bottomThickness": 2.0,
        "plateToPcb": 3.5,
        "wallThickness": 2.0,
        "clearance": 0.2,
        "profiles": []
    }))
    .unwrap()
}

fn board(part_ids: Vec<String>, thickness: f64) -> Board {
    Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids,
        net_ids: vec![],
        thickness,
        traces: vec![],
        vias: vec![],
    }
}

fn rectangle(x0: f64, y0: f64, x1: f64, y1: f64) -> Contour {
    Contour {
        hole: false,
        points: vec![
            Vec2 { x: x0, y: y0 },
            Vec2 { x: x1, y: y0 },
            Vec2 { x: x1, y: y1 },
            Vec2 { x: x0, y: y1 },
        ],
    }
}

fn config() -> MechanicalConfiguration {
    MechanicalConfiguration {
        internal_gasket: None,
        gasket_layout: None,
        hardware: None,
        critical_fits: None,
        bottom_style: None,
        middle_frame: None,
        gasket_travel: None,
        openings: None,
        opening_allowance: None,
        stabilizers: None,
        part_processes: None,
        gasket: None,
        closure_mounts: None,
        board_id: "board".into(),
        next_opening_identity: 0,
        next_critical_fit_identity: 0,
        next_hardware_identity: 0,
        next_mount_identity: 0,
        method: PlateMethod::Printed,
        mount: MechanicalMount::Tray,
        integrated_plate_frame: false,
        battery: None,
        mounts: vec![],
        plate_thickness: 1.5,
        plate_foam_thickness: 1.0,
        pcb_thickness: 1.6,
        bottom_foam_thickness: 0.5,
        battery_height: 3.0,
        bottom_thickness: 2.0,
        plate_to_pcb: 3.5,
        wall_thickness: 2.0,
        clearance: 0.2,
        profiles: vec![],
    }
}
