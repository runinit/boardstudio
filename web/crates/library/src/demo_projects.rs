use boardstudio_core::model::{
    Board, Material, Operation, OutlineFeature, OutlineSettings, Part, PartDefinition, PartKind,
    Pose2, ProjectDoc, Side, Vec2,
};

#[derive(Clone, Copy)]
pub(crate) struct DemoProject {
    pub(crate) name: &'static str,
    pub(crate) summary: &'static str,
    layout: DemoLayout,
}

#[derive(Clone, Copy)]
enum DemoLayout {
    Split42,
    Studio60,
    Macro12,
}

pub(crate) const DEMO_PROJECTS: [DemoProject; 3] = [
    DemoProject {
        name: "Split 42",
        summary: "A staggered split layout with a compact thumb cluster.",
        layout: DemoLayout::Split42,
    },
    DemoProject {
        name: "Studio 60",
        summary: "A compact staggered layout for everyday typing.",
        layout: DemoLayout::Studio60,
    },
    DemoProject {
        name: "Macro 12",
        summary: "A small 3 × 4 pad for shortcuts and macros.",
        layout: DemoLayout::Macro12,
    },
];

impl DemoProject {
    pub(crate) fn document(self) -> ProjectDoc {
        let mut document = ProjectDoc::empty("demo-template", self.name);
        let positions = match self.layout {
            DemoLayout::Split42 => split42_positions(),
            DemoLayout::Studio60 => grid_positions(5, 12, 19.05),
            DemoLayout::Macro12 => grid_positions(3, 4, 19.05),
        };
        let part_ids: Vec<_> = (0..positions.len())
            .map(|index| format!("key-{:02}", index + 1))
            .collect();
        document.definitions.push(PartDefinition {
            hardware_profile: None,
            input_profile: None,
            id: "demo-switch".into(),
            name: "Demo switch".into(),
            kind: PartKind::Switch,
            keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
            envelope_source: None,
            kicad_source: None,
            terminals: Default::default(),
            matrix_terminals: None,
            envelope_notice: None,
            courtyard: Vec::new(),
            pads: Vec::new(),
            models: None,
            generator: None,
            mechanical_profile: None,
        });
        document.parts = positions
            .into_iter()
            .zip(&part_ids)
            .enumerate()
            .map(|(index, (at, id))| Part {
                keycap: Some(Vec2 { x: 18.0, y: 18.0 }),
                outline: None,
                id: id.clone(),
                definition_id: "demo-switch".into(),
                reference: format!("SW{}", index + 1),
                pose: Pose2 { at, rotation: 0.0 },
                side: Side::Front,
                locked: None,
                properties: None,
                generator_parameters: None,
            })
            .collect();
        document.outline.push(OutlineFeature::PartEnvelope {
            connections: Vec::new(),
            settings: OutlineSettings::default(),
            id: "demo-outline".into(),
            part_ids: part_ids.clone(),
            margin: 4.0,
            operation: Operation::Add,
        });
        document.boards.push(Board {
            id: "demo-board".into(),
            name: "Main board".into(),
            outline_ids: vec!["demo-outline".into()],
            part_ids,
            net_ids: Vec::new(),
            thickness: 1.6,
            traces: Vec::new(),
            vias: Vec::new(),
        });
        document.materials.push(Material {
            id: "pla".into(),
            name: "PLA".into(),
            thickness: 3.0,
        });
        document
    }
}

fn grid_positions(rows: usize, columns: usize, pitch: f64) -> Vec<Vec2> {
    (0..rows)
        .flat_map(|row| {
            (0..columns).map(move |column| Vec2 {
                x: (column as f64 - (columns - 1) as f64 / 2.0) * pitch
                    + if row > 0 { pitch * 0.25 } else { 0.0 },
                y: (row as f64 - (rows - 1) as f64 / 2.0) * pitch,
            })
        })
        .collect()
}

#[cfg(test)]
mod demo_rebuild_tests {
    use super::*;
    use boardstudio_core::migrate::CURRENT_VERSION;

    fn studio60() -> ProjectDoc {
        DEMO_PROJECTS
            .iter()
            .find(|demo| demo.name == "Studio 60")
            .expect("Studio 60 entry")
            .document()
    }

    fn legacy_grid(rows: usize, columns: usize, pitch: f64) -> Vec<Vec2> {
        (0..rows)
            .flat_map(|row| {
                (0..columns).map(move |column| Vec2 {
                    x: (column as f64 - (columns - 1) as f64 / 2.0) * pitch
                        + if row > 0 { pitch * 0.25 } else { 0.0 },
                    y: (row as f64 - (rows - 1) as f64 / 2.0) * pitch,
                })
            })
            .collect()
    }

    #[test]
    fn studio60_is_a_core_generated_v7_matrix_document() {
        let document = studio60();
        assert_eq!(document.format_version, CURRENT_VERSION);
        assert_eq!(document.format_version, 7);
        assert_eq!(document.matrices.len(), 1, "one matrix");
        let matrix = &document.matrices[0];
        assert_eq!((matrix.rows, matrix.columns), (5, 12));
        assert_eq!(matrix.pitch, Vec2 { x: 19.05, y: 19.05 });
        assert_eq!(
            matrix.edge_gap,
            Some(Vec2 { x: 1.05, y: 1.05 }),
            "explicit 1.05 mm edge gap"
        );
        assert_eq!(document.layouts.len(), 1, "one Layout");
        assert_eq!(document.layouts[0].matrix_ids, vec![matrix.id.clone()]);
        // Only Row 2 (zero-based row 1) carries Shift X = 4.7625; every other
        // row/column record stays default.
        for (index, record) in matrix.row_layout.iter().enumerate() {
            if index == 1 {
                assert_eq!(record.shift.x, 4.7625, "Row 2 Shift X");
                assert_eq!(record.shift.y, 0.0);
            } else {
                assert_eq!(record.shift, Vec2 { x: 0.0, y: 0.0 }, "row {index} shift");
            }
            assert_eq!(record.bend, 0.0, "row {index} bend");
            assert_eq!(record.offset, Vec2 { x: 0.0, y: 0.0 }, "row {index} offset");
            assert_eq!(record.padding, None, "row {index} padding");
        }
        for (index, record) in matrix.column_layout.iter().enumerate() {
            assert_eq!(record.stagger, 0.0, "column {index} stagger");
            assert_eq!(record.splay, 0.0, "column {index} splay");
            assert_eq!(record.rotation, 0.0, "column {index} rotation");
        }
        assert!(matrix.mirror.is_none(), "no local mirror");
        assert!(matrix.anchor.is_none(), "unanchored");
    }

    #[test]
    fn studio60_keeps_existing_poses_with_matrix_identities() {
        let document = studio60();
        assert_eq!(document.parts.len(), 60);
        assert!(
            document.parts.iter().all(|part| part.pose.rotation == 0.0),
            "zero headings"
        );
        let expected = legacy_grid(5, 12, 19.05);
        for (part, at) in document.parts.iter().zip(&expected) {
            let member = document
                .matrices
                .iter()
                .find(|matrix| matrix.part_ids.contains(&part.id))
                .expect("every part is a matrix member");
            let _ = member;
            assert!(
                (part.pose.at.x - at.x).abs() < 1e-9 && (part.pose.at.y - at.y).abs() < 1e-9,
                "pose of {} drifts: got {:?}, want {at:?}",
                part.id,
                part.pose.at
            );
        }
        assert!(
            document
                .parts
                .iter()
                .all(|part| part.id.starts_with("matrix/")),
            "matrix-coordinate identities, not legacy key-NN ids"
        );
        assert!(
            !document.parts.iter().any(|part| part.id.starts_with("key-")),
            "no legacy key-NN ids survive"
        );
    }

    #[test]
    fn studio60_copies_do_not_share_mutable_state() {
        let mut first = studio60();
        let second = studio60();
        first.parts[0].pose.at.x += 10.0;
        assert_ne!(first.parts[0].pose.at.x, second.parts[0].pose.at.x);
        assert_eq!(first.parts.len(), second.parts.len());
    }
}

fn split42_positions() -> Vec<Vec2> {
    let mut positions = Vec::with_capacity(42);
    for side in [-1.0, 1.0] {
        for row in 0..3 {
            for column in 0..6 {
                positions.push(Vec2 {
                    x: side * (31.0 + column as f64 * 19.05) + row as f64 * 2.5,
                    y: (row as f64 - 1.0) * 19.05,
                });
            }
        }
        for thumb in 0..3 {
            positions.push(Vec2 {
                x: side * (64.0 + thumb as f64 * 19.05),
                y: 40.0 + thumb as f64 * 3.0,
            });
        }
    }
    positions
}
