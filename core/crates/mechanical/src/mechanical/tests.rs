#[path = "tests/stack.rs"]
mod stack;
#[path = "tests/switch_fits.rs"]
mod switch_fits;
#[path = "tests/cutouts.rs"]
mod cutouts;

use super::*;
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
