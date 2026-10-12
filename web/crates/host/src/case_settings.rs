//! Defaults for supported mechanical presets.
use boardstudio_core::mechanical::{MechanicalTarget, effective_inputs};
use boardstudio_core::model::{MechanicalConfiguration, ProjectDoc};

/// The board's mechanical configuration: its stored canonical settings, or Core's
/// reference defaults when the board has none.
///
/// Core owns the defaults, the switch-family inference and the standard layer
/// processes; this adapter reads them through the approved effective-input path, so
/// there is exactly one implementation of each default. A stored configuration is
/// returned exactly as accepted, as before.
pub fn initial_settings(
    document: &ProjectDoc,
    board_id: &str,
) -> Result<MechanicalConfiguration, String> {
    if let Some(config) = document
        .mechanical
        .as_ref()
        .filter(|c| c.board_id == board_id)
    {
        return Ok(config.clone());
    }
    // The accepted contour projection is not an input to a defaulted board
    // configuration, so this read passes none.
    effective_inputs(
        document,
        &[],
        &MechanicalTarget::Board {
            board_id: board_id.to_owned(),
        },
    )
    .map(|inputs| inputs.configuration)
}

/// Keep the standard process overrides in sync, matching MechanicalAssemblyPanel.update.
pub fn update_bottom_thickness(
    config: &mut MechanicalConfiguration,
    thickness: f64,
) -> Result<(), String> {
    if !thickness.is_finite() || thickness <= 0.0 {
        return Err("Bottom thickness must be positive.".into());
    }
    config.bottom_thickness = thickness;
    if let Some(processes) = &mut config.part_processes {
        for process in processes.iter_mut().filter(|p| p.part_id == "bottom") {
            process.thickness = thickness;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::model::{Board, PlateMethod};

    fn board(id: &str, thickness: f64) -> Board {
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

    fn document(board: Board) -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Fixture");
        document.boards.push(board);
        document
    }

    #[test]
    fn a_fresh_board_takes_the_core_owned_reference_defaults() {
        let settings = initial_settings(&document(board("board", 1.6)), "board")
            .expect("a fresh board resolves its defaults");
        assert_eq!(settings.board_id, "board");
        assert_eq!(settings.method, PlateMethod::Printed);
        assert_eq!(
            settings.mount,
            boardstudio_core::model::MechanicalMount::Tray
        );
        assert!(!settings.integrated_plate_frame);
        assert_eq!(settings.plate_thickness, 1.5);
        assert_eq!(settings.plate_to_pcb, 3.5);
        assert_eq!(settings.plate_foam_thickness, 3.0);
        assert_eq!(settings.pcb_thickness, 1.6);
        assert_eq!(settings.bottom_foam_thickness, 2.0);
        assert_eq!(settings.bottom_thickness, 3.0);
        assert_eq!(settings.wall_thickness, 2.0);
        assert_eq!(settings.clearance, 0.3);
        assert_eq!(
            settings.bottom_style,
            Some(boardstudio_core::model::MechanicalBottomStyle::Shell)
        );
        assert_eq!(settings.middle_frame, Some(false));
        assert!(settings.mounts.is_empty());
        assert!(settings.profiles.is_empty());
        let processes = settings
            .part_processes
            .expect("the layer processes are set");
        assert_eq!(
            processes
                .iter()
                .map(|process| process.part_id.as_str())
                .collect::<Vec<_>>(),
            vec!["plate", "plate-foam", "bottom-foam", "bottom"]
        );
        assert_eq!(processes[0].method, PlateMethod::Printed);
        assert_eq!(
            processes[1].method,
            PlateMethod::CutSheet,
            "the foam layers use the Core-owned cut-sheet default"
        );
    }

    #[test]
    fn the_defaulted_pcb_thickness_follows_the_board_with_a_fallback() {
        let thick = initial_settings(&document(board("board", 2.4)), "board").unwrap();
        assert_eq!(thick.pcb_thickness, 2.4);
        let missing = initial_settings(&document(board("board", 0.0)), "board").unwrap();
        assert_eq!(missing.pcb_thickness, 1.6);
    }

    #[test]
    fn a_stored_configuration_is_returned_exactly_as_accepted() {
        let mut doc = document(board("board", 1.6));
        let mut stored = initial_settings(&doc, "board").unwrap();
        stored.wall_thickness = 2.9;
        stored.plate_thickness = 1.9;
        stored.mounts = vec![boardstudio_core::model::Mount {
            id: "mount-1".into(),
            at: boardstudio_core::model::Vec2 { x: 1.0, y: 2.0 },
            kind: boardstudio_core::model::MountKind::Hole,
            hole_diameter: 2.2,
            boss_diameter: None,
            height: None,
        }];
        doc.mechanical = Some(stored.clone());
        assert_eq!(initial_settings(&doc, "board").unwrap(), stored);
    }

    #[test]
    fn an_unknown_board_has_no_defaults() {
        assert!(initial_settings(&document(board("board", 1.6)), "missing").is_err());
    }
}
