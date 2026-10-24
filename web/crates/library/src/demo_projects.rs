use boardstudio_core::model::{
    Board, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Layout, Material,
    Matrix, Operation, OutlineFeature, OutlineSettings, Part, PartDefinition, PartKind, Pose2,
    ProjectDoc, RowLayout, Side, Vec2,
};
use boardstudio_core::CoreEngine;

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

/// Builds one shipped example by driving Core through its public requests, so
/// every switch pose is Core-generated matrix materialization (decision 14),
/// never a baked part pose.
struct DemoBuilder {
    engine: CoreEngine,
    revision: u64,
}

impl DemoBuilder {
    fn open(document: ProjectDoc) -> Result<Self, String> {
        let mut engine = CoreEngine::new();
        let revision = match engine.handle(CoreRequest::Open {
            id: "demo-open".into(),
            document,
        }) {
            CoreReply::Scene { scene, .. } => scene.revision,
            CoreReply::Error { message, .. } => return Err(message),
            reply => return Err(format!("demo Open failed: {reply:?}")),
        };
        Ok(Self { engine, revision })
    }

    fn commit(&mut self, operation: EditOperation) -> Result<ProjectDoc, String> {
        let revision = self.revision;
        let reply = self.engine.handle(CoreRequest::Edit {
            id: "demo-edit".into(),
            command: EditCommand {
                base_revision: revision,
                transaction_id: format!("demo-{revision}"),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        });
        match reply {
            CoreReply::Scene { scene, document, .. } => {
                self.revision = scene.revision;
                Ok(*document)
            }
            CoreReply::Error { message, .. } => Err(message),
            reply => Err(format!("demo edit failed: {reply:?}")),
        }
    }

    fn replace(&mut self, document: ProjectDoc) -> Result<ProjectDoc, String> {
        self.commit(EditOperation::ReplaceDocument {
            document: Box::new(document),
        })
    }
}

const DEMO_PITCH_MM: f64 = 19.05;
const DEMO_EDGE_GAP_MM: f64 = 1.05;
const DEMO_ROW_SHIFT_MM: f64 = 4.7625;

fn demo_shell(name: &str) -> ProjectDoc {
    let mut document = ProjectDoc::empty("demo-template", name);
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
    document.outline.push(OutlineFeature::PartEnvelope {
        connections: Vec::new(),
        settings: OutlineSettings::default(),
        id: "demo-outline".into(),
        part_ids: Vec::new(),
        margin: 4.0,
        operation: Operation::Add,
    });
    document.boards.push(Board {
        id: "demo-board".into(),
        name: "Main board".into(),
        outline_ids: vec!["demo-outline".into()],
        part_ids: Vec::new(),
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

fn single_matrix_document(
    name: &str,
    rows: u32,
    columns: u32,
    origin: Vec2,
) -> Result<ProjectDoc, String> {
    let mut builder = DemoBuilder::open(demo_shell(name))?;
    builder.commit(EditOperation::SetMatrix {
        matrix: Matrix {
            id: "main".into(),
            name: Some("Main".into()),
            rows,
            columns,
            pitch: Vec2 {
                x: DEMO_PITCH_MM,
                y: DEMO_PITCH_MM,
            },
            origin,
            definition_id: "demo-switch".into(),
            part_ids: Vec::new(),
            board_id: None,
            mirror: None,
            rotation: None,
            edge_gap: Some(Vec2 {
                x: DEMO_EDGE_GAP_MM,
                y: DEMO_EDGE_GAP_MM,
            }),
            diode_direction: None,
            column_layout: Vec::new(),
            row_layout: vec![
                RowLayout::default(),
                RowLayout {
                    shift: Vec2 {
                        x: DEMO_ROW_SHIFT_MM,
                        y: 0.0,
                    },
                    ..Default::default()
                },
            ],
            anchor: None,
            cells: Vec::new(),
        },
        definitions: None,
    })?;
    let mut document = match builder.engine.handle(CoreRequest::Snapshot {
        id: "demo-snapshot".into(),
    }) {
        CoreReply::Scene { document, .. } => *document,
        CoreReply::Error { message, .. } => return Err(message),
        reply => return Err(format!("demo snapshot failed: {reply:?}")),
    };
    document.layouts.push(Layout {
        id: "layout".into(),
        name: "Main".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: Vec::new(),
        mirror_link: None,
    });
    let mut document = builder.replace(document)?;
    // The builder session stays inside `document()`; the template itself
    // carries no session revision. `Runtime::open_demo_copy` assigns a fresh
    // id and revision when a user opens it.
    document.revision = 0;
    Ok(document)
}

fn studio60_document(name: &str) -> ProjectDoc {
    single_matrix_document(
        name,
        5,
        12,
        Vec2 {
            x: -104.775,
            y: -38.1,
        },
    )
    .expect("Studio 60 demo builds through Core")
}

impl DemoProject {
    pub(crate) fn document(self) -> ProjectDoc {
        if matches!(self.layout, DemoLayout::Studio60) {
            return studio60_document(self.name);
        }
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
    fn demo_entries_keep_their_identities() {
        let entries: Vec<_> = DEMO_PROJECTS
            .iter()
            .map(|demo| (demo.name, demo.summary))
            .collect();
        assert_eq!(
            entries,
            vec![
                (
                    "Split 42",
                    "A staggered split layout with a compact thumb cluster."
                ),
                ("Studio 60", "A compact staggered layout for everyday typing."),
                ("Macro 12", "A small 3 × 4 pad for shortcuts and macros."),
            ]
        );
        let counts: Vec<_> = DEMO_PROJECTS
            .iter()
            .map(|demo| (demo.name, demo.document().parts.len()))
            .collect();
        assert_eq!(
            counts,
            vec![("Split 42", 42), ("Studio 60", 60), ("Macro 12", 12)]
        );
    }

    fn macro12() -> ProjectDoc {
        DEMO_PROJECTS
            .iter()
            .find(|demo| demo.name == "Macro 12")
            .expect("Macro 12 entry")
            .document()
    }

    #[test]
    fn macro12_uses_the_single_matrix_recipe_at_3x4() {
        let document = macro12();
        assert_eq!(document.format_version, CURRENT_VERSION);
        assert_eq!(document.matrices.len(), 1, "one matrix");
        let matrix = &document.matrices[0];
        assert_eq!((matrix.rows, matrix.columns), (3, 4));
        assert_eq!(matrix.pitch, Vec2 { x: 19.05, y: 19.05 });
        assert_eq!(matrix.origin, Vec2 { x: -28.575, y: -19.05 });
        assert_eq!(document.layouts.len(), 1, "one Layout");
        assert_eq!(document.layouts[0].matrix_ids, vec![matrix.id.clone()]);
        let shifts: Vec<_> = matrix
            .row_layout
            .iter()
            .map(|record| record.shift)
            .collect();
        assert_eq!(
            shifts,
            vec![
                Vec2 { x: 0.0, y: 0.0 },
                Vec2 { x: 4.7625, y: 0.0 },
            ],
            "only Row 2 carries Shift X"
        );
        assert_eq!(document.parts.len(), 12);
        let expected = legacy_grid(3, 4, 19.05);
        for (part, at) in document.parts.iter().zip(&expected) {
            assert!(
                document.matrices[0].part_ids.contains(&part.id),
                "{} is a matrix member",
                part.id
            );
            assert!(
                (part.pose.at.x - at.x).abs() < 1e-9 && (part.pose.at.y - at.y).abs() < 1e-9,
                "pose of {} drifts: got {:?}, want {at:?}",
                part.id,
                part.pose.at
            );
            assert_eq!(part.pose.rotation, 0.0, "zero heading");
        }
    }

    #[test]
    fn rebuilt_examples_share_pitch_gap_caps_and_board_intent() {
        for name in ["Studio 60", "Macro 12"] {
            let document = DEMO_PROJECTS
                .iter()
                .find(|demo| demo.name == name)
                .expect("demo entry")
                .document();
            for matrix in &document.matrices {
                assert_eq!(matrix.pitch, Vec2 { x: 19.05, y: 19.05 }, "{name} pitch");
                assert_eq!(
                    matrix.edge_gap,
                    Some(Vec2 { x: 1.05, y: 1.05 }),
                    "{name} edge gap"
                );
                assert_eq!(matrix.definition_id, "demo-switch", "{name} switch");
            }
            let definition = document
                .definitions
                .iter()
                .find(|definition| definition.id == "demo-switch")
                .expect("synthetic Demo switch");
            assert_eq!(
                definition.keycap,
                Some(Vec2 { x: 18.0, y: 18.0 }),
                "{name} 18x18 caps"
            );
            for part in &document.parts {
                let cap = part.keycap.or(definition.keycap);
                assert_eq!(cap, Some(Vec2 { x: 18.0, y: 18.0 }), "{name} cap resolves");
            }
            assert_eq!(document.boards.len(), 1, "{name} one board");
            assert_eq!(document.boards[0].id, "demo-board", "{name} board identity");
            assert!(
                document.boards[0]
                    .part_ids
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>()
                    == document.parts.iter().map(|part| &part.id).collect(),
                "{name} board owns every member"
            );
            assert_eq!(document.materials.len(), 1, "{name} one material");
            assert_eq!(document.materials[0].id, "pla", "{name} material identity");
            assert!(document.hardware.is_none(), "{name} no hardware claim");
            assert!(document.nets.is_empty(), "{name} no wiring");
            assert!(document.modules.is_empty(), "{name} no controllers");
        }
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
