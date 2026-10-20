use crate::model::{
    ArtifactError, ArtifactErrorCode, ArtifactFile, Contour, ExportArtifact, ProjectDoc, Severity,
};

pub(super) fn export(
    document: &ProjectDoc,
    contours: &[Contour],
) -> Result<ExportArtifact, ArtifactError> {
    let config = document.mechanical.as_ref().ok_or_else(|| {
        ArtifactError::new(
            ArtifactErrorCode::Validation,
            "Configure a mechanical plate first",
        )
    })?;
    let assembly = crate::mechanical::resolve(document, contours);
    if let Some(finding) = assembly
        .diagnostics
        .iter()
        .find(|finding| finding.severity == Severity::Error)
    {
        return Err(ArtifactError::new(
            ArtifactErrorCode::Validation,
            &finding.message,
        ));
    }
    let mut geometry = String::new();
    for contour in &assembly.plate_contours {
        for index in 0..contour.points.len() {
            let a = contour.points[index];
            let b = contour.points[(index + 1) % contour.points.len()];
            if a == b {
                continue;
            }
            geometry.push_str(&format!("  (gr_line (start {:.6} {:.6}) (end {:.6} {:.6}) (stroke (width 0.05) (type solid)) (layer \"Edge.Cuts\"))\n", a.x, -a.y, b.x, -b.y));
        }
    }
    for mount in assembly
        .case
        .bodies
        .iter()
        .filter(|entry| entry.body.id == "plate")
        .flat_map(|entry| entry.body.mounts.iter().flatten())
    {
        geometry.push_str(&format!("  (footprint \"Mechanical:MountingHole\" (layer \"F.Cu\") (at {:.6} {:.6}) (attr board_only exclude_from_pos_files exclude_from_bom) (pad \"\" np_thru_hole circle (at 0 0) (size {:.6} {:.6}) (drill {:.6}) (layers \"*.Cu\" \"*.Mask\")))\n", mount.at.x, -mount.at.y, mount.hole_diameter, mount.hole_diameter, mount.hole_diameter));
    }
    let board = format!(
        "(kicad_pcb (version 20241229) (generator \"BoardStudio\")\n  (general (thickness {:.6}))\n  (paper \"A4\")\n  (layers (0 \"F.Cu\" signal) (31 \"B.Cu\" signal) (38 \"B.Mask\" user) (39 \"F.Mask\" user) (44 \"Edge.Cuts\" user))\n{geometry})\n",
        config.plate_thickness
    );
    let settings = serde_json::json!({"meta":{"filename":"mechanical-plate.kicad_pro","version":1},"board":{"design_settings":{"defaults":{"board_outline_line_width":0.05},"rules":{"min_hole_clearance":0.25,"min_hole_to_hole":0.25}}},"text_variables":{"PLATE_THICKNESS_MM":config.plate_thickness.to_string(),"FABRICATION_TYPE":"MECHANICAL PLATE — NO CIRCUITRY"}});
    Ok(ExportArtifact {
        snapshot_token: format!("mechanical:{}:{}", document.id, document.revision),
        revision: document.revision,
        skipped_utilities: vec![],
        files: vec![
            ArtifactFile {
                filename: "mechanical-plate.kicad_pcb".into(),
                content: board,
            },
            ArtifactFile {
                filename: "mechanical-plate.kicad_pro".into(),
                content: serde_json::to_string_pretty(&settings).unwrap(),
            },
            ArtifactFile {
                filename: "FABRICATION.md".into(),
                content: format!(
                    "# Mechanical plate\n\nRevision: {}\nThickness: {} mm\n\nThis is a separate mechanical plate, with no circuitry, copper tracks, vias or electrical nets. Edge.Cuts contains closed perimeter and switch cutouts. Mounting holes are non-plated through holes. Request no copper and no plated holes from the fabricator. Confirm material, finished thickness, machining tolerances and minimum cutout radii before ordering. Inspect generated Gerber and drill files in a CAM viewer and verify physical switch, stabilizer, PCB and enclosure fit. This project is not an electrical PCB export.\n",
                    document.revision, config.plate_thickness
                ),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;
    /// `kicad-cli` without an AppImage launcher's library paths, which break KiCad's binaries.
    fn kicad_cli() -> Command {
        let mut command = Command::new("kicad-cli");
        if std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some() {
            for name in [
                "APPDIR",
                "APPIMAGE",
                "LD_LIBRARY_PATH",
                "GSETTINGS_SCHEMA_DIR",
            ] {
                command.env_remove(name);
            }
        }
        command
    }
    fn fixture() -> (ProjectDoc, Vec<Contour>) {
        let mut doc = ProjectDoc::empty("plate-test", "Plate test");
        doc.boards.push(serde_json::from_value(serde_json::json!({"id":"board","name":"Board","outlineIds":[],"partIds":[],"netIds":[],"thickness":1.6,"traces":[],"vias":[]})).unwrap());
        doc.mechanical = Some(serde_json::from_value(serde_json::json!({
            "boardId":"board","method":"pcb-fr4","mount":"rigid","plateThickness":1.5,
            "plateFoamThickness":0,"pcbThickness":1.6,"bottomFoamThickness":0,
            "batteryHeight":0,"bottomThickness":2,"plateToPcb":3.5,"wallThickness":2,
            "clearance":0.2,"profiles":[],"mounts":[{"id":"mount","at":{"x":4,"y":4},"kind":"hole","holeDiameter":2.2,"bossDiameter":5}]
        })).unwrap());
        let mut contours: Vec<Contour> = serde_json::from_value(serde_json::json!([
            {"hole":false,"points":[{"x":0,"y":0},{"x":40,"y":0},{"x":40,"y":30},{"x":0,"y":30}]},
            {"hole":true,"points":[{"x":10,"y":10},{"x":24,"y":10},{"x":24,"y":24},{"x":10,"y":24}]}
        ]))
        .unwrap();
        contours[1].points = (0..64)
            .map(|index| {
                let angle = index as f64 * std::f64::consts::TAU / 64.0;
                crate::model::Vec2 {
                    x: 20.0 + 7.0 * angle.cos(),
                    y: 15.0 + 7.0 * angle.sin(),
                }
            })
            .collect();
        (doc, contours)
    }
    #[test]
    fn plate_has_closed_cuts_npth_and_no_circuitry() {
        let (doc, contours) = fixture();
        let result = export(&doc, &contours).unwrap();
        let board = &result.files[0].content;
        assert_eq!(board.matches("(gr_line ").count(), 68);
        assert!(board.contains("(thickness 1.500000)"));
        assert!(board.contains("np_thru_hole"));
        assert!(!board.contains("(net "));
        assert!(!board.contains("(segment "));
        assert!(!board.contains("(via "));
        assert!(
            result
                .files
                .iter()
                .any(|file| file.filename.ends_with(".kicad_pro"))
        );
    }
    #[test]
    fn rejects_sharp_fr4_routed_cutouts() {
        let (doc, mut contours) = fixture();
        contours[1].points = vec![
            crate::model::Vec2 { x: 10.0, y: 10.0 },
            crate::model::Vec2 { x: 24.0, y: 10.0 },
            crate::model::Vec2 { x: 24.0, y: 24.0 },
            crate::model::Vec2 { x: 10.0, y: 24.0 },
        ];
        assert!(
            export(&doc, &contours)
                .unwrap_err()
                .message
                .contains("sharp")
        );
    }
    #[test]
    #[ignore = "requires KiCad CLI 10; run explicitly for manufacturing export validation"]
    fn kicad_10_exports_actual_gerber_and_npth_drill() {
        let version = kicad_cli().arg("--version").output().unwrap();
        assert!(String::from_utf8_lossy(&version.stdout).starts_with("10."));
        let (doc, contours) = fixture();
        let result = export(&doc, &contours).unwrap();
        let dir =
            std::env::temp_dir().join(format!("boardstudio-mechanical-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for file in &result.files {
            std::fs::write(dir.join(&file.filename), &file.content).unwrap();
        }
        let board = dir.join("mechanical-plate.kicad_pcb");
        let output = dir.join("cam");
        for args in [
            vec!["pcb", "export", "gerbers", "--layers", "Edge.Cuts"],
            vec![
                "pcb",
                "export",
                "drill",
                "--format",
                "excellon",
                "--excellon-separate-th",
            ],
        ] {
            let result = kicad_cli()
                .args(args)
                .arg("--output")
                .arg(&output)
                .arg(&board)
                .output()
                .unwrap();
            assert!(
                result.status.success(),
                "{}",
                String::from_utf8_lossy(&result.stderr)
            );
        }
        let paths: Vec<_> = std::fs::read_dir(&output)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        let gerber = paths
            .iter()
            .find(|p| p.extension().is_some_and(|e| e == "gm1"))
            .unwrap();
        assert!(std::fs::read_to_string(gerber).unwrap().contains("M02*"));
        let drill = paths
            .iter()
            .find(|p| {
                p.to_string_lossy().contains("NPTH") && p.extension().is_some_and(|e| e == "drl")
            })
            .unwrap();
        assert!(std::fs::read_to_string(drill).unwrap().contains("C2.200"));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
