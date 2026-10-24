use boardstudio_core::CoreEngine;
use boardstudio_core::model::{
    Board, ColumnLayout, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Follow,
    Layout, LayoutMirrorLink, Material, Matrix, MatrixAnchor, Mirror, MirroredCorrespondence,
    MirroredItemKind, Operation, OutlineFeature, OutlineSettings, PartDefinition, PartKind,
    ProjectDoc, RowLayout, Vec2,
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
            CoreReply::Scene {
                scene, document, ..
            } => {
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

    fn snapshot(&mut self) -> Result<ProjectDoc, String> {
        match self.engine.handle(CoreRequest::Snapshot {
            id: "demo-snapshot".into(),
        }) {
            CoreReply::Scene { document, .. } => Ok(*document),
            CoreReply::Error { message, .. } => Err(message),
            reply => Err(format!("demo snapshot failed: {reply:?}")),
        }
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
    let mut document = builder.snapshot()?;
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

fn macro12_document(name: &str) -> ProjectDoc {
    single_matrix_document(
        name,
        3,
        4,
        Vec2 {
            x: -28.575,
            y: -19.05,
        },
    )
    .expect("Macro 12 demo builds through Core")
}

fn demo_matrix(id: &str, rows: u32, columns: u32) -> Matrix {
    Matrix {
        id: id.into(),
        name: None,
        rows,
        columns,
        pitch: Vec2 {
            x: DEMO_PITCH_MM,
            y: DEMO_PITCH_MM,
        },
        origin: Vec2 { x: 0.0, y: 0.0 },
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
        row_layout: Vec::new(),
        anchor: None,
        cells: Vec::new(),
    }
}

fn split42_document(name: &str) -> ProjectDoc {
    split42_build(name).expect("Split 42 demo builds through Core")
}

fn split42_build(name: &str) -> Result<ProjectDoc, String> {
    let mut builder = DemoBuilder::open(demo_shell(name))?;
    // Left main group: the authored source. Its local X mirror turns the
    // negative row Shifts into the legacy positive world-X progression.
    let mut left_main = demo_matrix("left-main", 3, 6);
    left_main.name = Some("Left main".into());
    left_main.origin = Vec2 {
        x: -31.0,
        y: -19.05,
    };
    left_main.mirror = Some(Mirror::X);
    left_main.row_layout = vec![
        RowLayout::default(),
        RowLayout {
            shift: Vec2 { x: -2.5, y: 0.0 },
            ..Default::default()
        },
        RowLayout {
            shift: Vec2 { x: -2.5, y: 0.0 },
            ..Default::default()
        },
    ];
    builder.commit(EditOperation::SetMatrix {
        matrix: left_main,
        definitions: None,
    })?;
    let mut document = builder.snapshot()?;
    document.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec!["left-main".into()],
        part_ids: Vec::new(),
        mirror_link: None,
    });
    builder.replace(document)?;

    // Left thumb cluster: anchored to Main Row 3, Column 3 with a relative
    // authored offset, staggered outward like the legacy cluster.
    let thumb_anchor = MatrixAnchor {
        matrix_id: "left-main".into(),
        row: 2,
        column: 2,
        follow: Follow {
            x: true,
            y: true,
            rotation: true,
        },
    };
    builder.commit(EditOperation::AddAnchoredMatrix {
        id: "left-thumb".into(),
        anchor: thumb_anchor.clone(),
    })?;
    let mut left_thumb = demo_matrix("left-thumb", 1, 3);
    left_thumb.name = Some("Left thumb".into());
    left_thumb.board_id = Some("demo-board".into());
    left_thumb.origin = Vec2 { x: 0.1, y: 20.95 };
    left_thumb.mirror = Some(Mirror::X);
    left_thumb.anchor = Some(thumb_anchor);
    left_thumb.column_layout = vec![
        ColumnLayout::default(),
        ColumnLayout {
            stagger: 3.0,
            ..Default::default()
        },
        ColumnLayout {
            stagger: 3.0,
            ..Default::default()
        },
    ];
    builder.commit(EditOperation::SetMatrix {
        matrix: left_thumb,
        definitions: None,
    })?;

    // Right-side records carry identities and dimensions only. Core's
    // linked-layout sync derives every right-side pose from the left source
    // when the link lands, so no second right-side formula is authored here.
    let mut right_main = demo_matrix("right-main", 3, 6);
    right_main.name = Some("Right main".into());
    builder.commit(EditOperation::SetMatrix {
        matrix: right_main,
        definitions: None,
    })?;
    let mut right_thumb = demo_matrix("right-thumb", 1, 3);
    right_thumb.name = Some("Right thumb".into());
    builder.commit(EditOperation::SetMatrix {
        matrix: right_thumb,
        definitions: None,
    })?;

    let mut correspondences = vec![
        MirroredCorrespondence {
            kind: MirroredItemKind::Matrix,
            source_id: "left-main".into(),
            mirror_id: "right-main".into(),
            companion_mode: None,
            companion_tweak: None,
        },
        MirroredCorrespondence {
            kind: MirroredItemKind::Matrix,
            source_id: "left-thumb".into(),
            mirror_id: "right-thumb".into(),
            companion_mode: None,
            companion_tweak: None,
        },
    ];
    for (source_id, mirror_id, rows, columns) in [
        ("left-main", "right-main", 3u32, 6u32),
        ("left-thumb", "right-thumb", 1u32, 3u32),
    ] {
        for row in 0..rows {
            for column in 0..columns {
                correspondences.push(MirroredCorrespondence {
                    kind: MirroredItemKind::Part,
                    source_id: format!("matrix/{source_id}/r{row}c{column}"),
                    mirror_id: format!("matrix/{mirror_id}/r{row}c{column}"),
                    companion_mode: None,
                    companion_tweak: None,
                });
            }
        }
    }
    let mut document = builder.snapshot()?;
    document.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec!["right-main".into(), "right-thumb".into()],
        part_ids: Vec::new(),
        mirror_link: Some(LayoutMirrorLink {
            source_id: "left".into(),
            axis_x: 0.0,
            correspondences,
            exceptions: Vec::new(),
        }),
    });
    let mut document = builder.replace(document)?;
    document.revision = 0;
    Ok(document)
}

impl DemoProject {
    pub(crate) fn document(self) -> ProjectDoc {
        match self.layout {
            DemoLayout::Studio60 => studio60_document(self.name),
            DemoLayout::Macro12 => macro12_document(self.name),
            DemoLayout::Split42 => split42_document(self.name),
        }
    }
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
            !document
                .parts
                .iter()
                .any(|part| part.id.starts_with("key-")),
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
                (
                    "Studio 60",
                    "A compact staggered layout for everyday typing."
                ),
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
        assert_eq!(
            matrix.origin,
            Vec2 {
                x: -28.575,
                y: -19.05
            }
        );
        assert_eq!(document.layouts.len(), 1, "one Layout");
        assert_eq!(document.layouts[0].matrix_ids, vec![matrix.id.clone()]);
        let shifts: Vec<_> = matrix
            .row_layout
            .iter()
            .map(|record| record.shift)
            .collect();
        assert_eq!(
            shifts,
            vec![Vec2 { x: 0.0, y: 0.0 }, Vec2 { x: 4.7625, y: 0.0 },],
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
        for name in ["Studio 60", "Macro 12", "Split 42"] {
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
            // The part-envelope outline keeps its intent and references the
            // generated members, not the retired loose-part ids.
            let envelopes: Vec<_> = document
                .outline
                .iter()
                .filter_map(|feature| match feature {
                    boardstudio_core::model::OutlineFeature::PartEnvelope {
                        id,
                        part_ids,
                        margin,
                        ..
                    } => Some((id.clone(), part_ids.clone(), *margin)),
                    _ => None,
                })
                .collect();
            assert_eq!(envelopes.len(), 1, "{name} one outline envelope");
            assert_eq!(envelopes[0].0, "demo-outline", "{name} outline identity");
            assert_eq!(envelopes[0].2, 4.0, "{name} outline margin");
            assert_eq!(
                envelopes[0]
                    .1
                    .iter()
                    .collect::<std::collections::BTreeSet<_>>(),
                document.parts.iter().map(|part| &part.id).collect(),
                "{name} outline references every generated member"
            );
        }
    }

    fn split42() -> ProjectDoc {
        DEMO_PROJECTS
            .iter()
            .find(|demo| demo.name == "Split 42")
            .expect("Split 42 entry")
            .document()
    }

    #[test]
    fn split42_has_two_linked_layouts_with_four_matrices() {
        use boardstudio_core::model::MirroredItemKind;
        let document = split42();
        assert_eq!(document.format_version, CURRENT_VERSION);
        assert_eq!(document.layouts.len(), 2, "two linked Layouts");
        let left = document
            .layouts
            .iter()
            .find(|layout| layout.mirror_link.is_none())
            .expect("left source Layout");
        let right = document
            .layouts
            .iter()
            .find(|layout| layout.mirror_link.is_some())
            .expect("right derived Layout");
        assert_eq!(left.board_id, "demo-board");
        assert_eq!(right.board_id, "demo-board");
        assert_eq!(left.matrix_ids.len(), 2, "left main + thumb");
        assert_eq!(right.matrix_ids.len(), 2, "right main + thumb");
        assert_eq!(document.matrices.len(), 4, "four matrices");
        let link = right.mirror_link.as_ref().expect("right link");
        assert_eq!(link.source_id, left.id);
        assert_eq!(link.axis_x, 0.0, "reflection about X = 0");
        assert!(link.exceptions.is_empty(), "no default exceptions");
        let matrix_links: Vec<_> = link
            .correspondences
            .iter()
            .filter(|record| record.kind == MirroredItemKind::Matrix)
            .collect();
        assert_eq!(matrix_links.len(), 2, "both matrix groups correspond");
        for matrix in &document.matrices {
            let in_left = left.matrix_ids.contains(&matrix.id);
            let in_right = right.matrix_ids.contains(&matrix.id);
            assert!(
                in_left ^ in_right,
                "{} owned by exactly one Layout",
                matrix.id
            );
        }
        let mains: Vec<_> = document
            .matrices
            .iter()
            .filter(|matrix| matrix.rows == 3 && matrix.columns == 6)
            .collect();
        let thumbs: Vec<_> = document
            .matrices
            .iter()
            .filter(|matrix| matrix.rows == 1 && matrix.columns == 3)
            .collect();
        assert_eq!(mains.len(), 2, "two 3x6 main matrices");
        assert_eq!(thumbs.len(), 2, "two 1x3 thumb matrices");
        assert_eq!(document.parts.len(), 42, "42 keys total");
    }

    #[test]
    fn split42_keeps_approved_poses_with_matrix_identities() {
        let document = split42();
        let pose_of = |member: &str| {
            document
                .parts
                .iter()
                .find(|part| part.id == member)
                .unwrap_or_else(|| panic!("missing member {member}"))
                .pose
        };
        // Left keeps every legacy pose; both thumbs keep theirs; the right
        // row 1 keeps its X. Only right-main rows 2/3 move, by -5/-10 mm.
        let left_id = document
            .layouts
            .iter()
            .find(|layout| layout.mirror_link.is_none())
            .expect("left")
            .id
            .clone();
        let left_main = document
            .matrices
            .iter()
            .find(|matrix| left_has(&document, &left_id, &matrix.id) && matrix.rows == 3)
            .expect("left main")
            .id
            .clone();
        let left_thumb = document
            .matrices
            .iter()
            .find(|matrix| left_has(&document, &left_id, &matrix.id) && matrix.rows == 1)
            .expect("left thumb")
            .id
            .clone();
        for row in 0..3 {
            for column in 0..6 {
                let legacy = Vec2 {
                    x: -(31.0 + column as f64 * 19.05) + row as f64 * 2.5,
                    y: (row as f64 - 1.0) * 19.05,
                };
                let pose = pose_of(&format!("matrix/{left_main}/r{row}c{column}"));
                assert!(
                    (pose.at.x - legacy.x).abs() < 1e-9 && (pose.at.y - legacy.y).abs() < 1e-9,
                    "left main r{row}c{column} drifts: got {:?}, want {legacy:?}",
                    pose.at
                );
                assert_eq!(pose.rotation, 0.0);
            }
        }
        for thumb in 0..3 {
            let legacy = Vec2 {
                x: -(64.0 + thumb as f64 * 19.05),
                y: 40.0 + thumb as f64 * 3.0,
            };
            let pose = pose_of(&format!("matrix/{left_thumb}/r0c{thumb}"));
            assert!(
                (pose.at.x - legacy.x).abs() < 1e-9 && (pose.at.y - legacy.y).abs() < 1e-9,
                "left thumb {thumb} drifts: got {:?}, want {legacy:?}",
                pose.at
            );
            assert_eq!(pose.rotation, 0.0);
        }
        let right_id = document
            .layouts
            .iter()
            .find(|layout| layout.mirror_link.is_some())
            .expect("right")
            .id
            .clone();
        let right_main = document
            .matrices
            .iter()
            .find(|matrix| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.id == right_id)
                    .expect("right")
                    .matrix_ids
                    .contains(&matrix.id)
                    && matrix.rows == 3
            })
            .expect("right main")
            .id
            .clone();
        let right_thumb: Vec<String> = document
            .matrices
            .iter()
            .filter(|matrix| {
                document
                    .layouts
                    .iter()
                    .find(|layout| layout.id == right_id)
                    .expect("right")
                    .matrix_ids
                    .contains(&matrix.id)
                    && matrix.rows == 1
            })
            .map(|matrix| matrix.id.clone())
            .collect();
        assert_eq!(right_thumb.len(), 1);
        for row in 0..3 {
            for column in 0..6 {
                let legacy_x = (31.0 + column as f64 * 19.05) + row as f64 * 2.5;
                let approved = match row {
                    0 => legacy_x,
                    1 => legacy_x - 5.0,
                    _ => legacy_x - 10.0,
                };
                let legacy_y = (row as f64 - 1.0) * 19.05;
                let pose = pose_of(&format!("matrix/{right_main}/r{row}c{column}"));
                assert!(
                    (pose.at.x - approved).abs() < 1e-9 && (pose.at.y - legacy_y).abs() < 1e-9,
                    "right main r{row}c{column}: got {:?}, want ({approved}, {legacy_y})",
                    pose.at
                );
                assert_eq!(pose.rotation, 0.0);
            }
        }
        for thumb in 0..3 {
            let pose = pose_of(&format!("matrix/{}/r0c{thumb}", right_thumb[0]));
            let want = Vec2 {
                x: 64.0 + thumb as f64 * 19.05,
                y: 40.0 + thumb as f64 * 3.0,
            };
            assert!(
                (pose.at.x - want.x).abs() < 1e-9 && (pose.at.y - want.y).abs() < 1e-9,
                "right thumb {thumb}: got {:?}, want {want:?}",
                pose.at
            );
            assert_eq!(pose.rotation, 0.0);
        }
        assert!(
            document
                .parts
                .iter()
                .all(|part| part.id.starts_with("matrix/")),
            "matrix-coordinate identities only"
        );
    }

    use boardstudio_core::model::Position;

    struct Session {
        engine: CoreEngine,
        revision: u64,
    }

    impl Session {
        fn open(document: ProjectDoc) -> Self {
            let mut engine = CoreEngine::new();
            let revision = match engine.handle(CoreRequest::Open {
                id: "split-open".into(),
                document,
            }) {
                CoreReply::Scene { scene, .. } => scene.revision,
                reply => panic!("Split 42 Open failed: {reply:?}"),
            };
            Self { engine, revision }
        }

        fn commit(&mut self, operation: EditOperation) -> ProjectDoc {
            let revision = self.revision;
            match self.engine.handle(CoreRequest::Edit {
                id: "split-edit".into(),
                command: EditCommand {
                    base_revision: revision,
                    transaction_id: format!("split-{revision}"),
                    phase: EditPhase::Commit,
                    target_ids: vec![],
                    operation,
                },
            }) {
                CoreReply::Scene {
                    scene, document, ..
                } => {
                    self.revision = scene.revision;
                    *document
                }
                reply => panic!("Split 42 edit failed: {reply:?}"),
            }
        }

        fn snapshot(&mut self) -> ProjectDoc {
            match self.engine.handle(CoreRequest::Snapshot {
                id: "split-snapshot".into(),
            }) {
                CoreReply::Scene { document, .. } => *document,
                reply => panic!("Split 42 snapshot failed: {reply:?}"),
            }
        }
    }

    #[test]
    fn split42_anchor_edit_moves_thumb_and_counterpart_with_one_undo() {
        let template = split42();
        let before = |document: &ProjectDoc, member: &str| {
            document
                .parts
                .iter()
                .find(|part| part.id == member)
                .unwrap_or_else(|| panic!("missing member {member}"))
                .pose
                .at
        };
        let anchor = "matrix/left-main/r2c2";
        let start = before(&template, anchor);
        let mut session = Session::open(template.clone());
        let moved = session.commit(EditOperation::MoveParts {
            positions: vec![Position {
                id: anchor.into(),
                at: Vec2 {
                    x: start.x + 10.0,
                    y: start.y,
                },
            }],
        });
        // The anchored thumb follows its actual anchor key on the same half.
        let thumb_start = before(&template, "matrix/left-thumb/r0c0");
        let thumb = before(&moved, "matrix/left-thumb/r0c0");
        assert!(
            (thumb.x - (thumb_start.x + 10.0)).abs() < 1e-9,
            "thumb follows X"
        );
        assert!((thumb.y - thumb_start.y).abs() < 1e-9, "thumb keeps Y");
        // The linked counterpart moves through the reflection, not a second
        // formula: +10 X on the left is -10 X on the right.
        let mirror_start = before(&template, "matrix/right-main/r2c2");
        let mirror = before(&moved, "matrix/right-main/r2c2");
        assert!(
            (mirror.x - (mirror_start.x - 10.0)).abs() < 1e-9,
            "counterpart reflects X"
        );
        assert!(
            (mirror.y - mirror_start.y).abs() < 1e-9,
            "counterpart keeps Y"
        );
        let mirror_thumb_start = before(&template, "matrix/right-thumb/r0c0");
        let mirror_thumb = before(&moved, "matrix/right-thumb/r0c0");
        assert!((mirror_thumb.x - (mirror_thumb_start.x - 10.0)).abs() < 1e-9);
        assert!((mirror_thumb.y - mirror_thumb_start.y).abs() < 1e-9);
        // One Undo restores every pose; Redo reapplies the same edit.
        let undone = match session.engine.handle(CoreRequest::Undo {
            id: "split-undo".into(),
        }) {
            CoreReply::Scene {
                scene, document, ..
            } => {
                session.revision = scene.revision;
                *document
            }
            reply => panic!("Split 42 Undo failed: {reply:?}"),
        };
        assert_eq!(undone.parts, template.parts, "one Undo restores all poses");
        let redone = match session.engine.handle(CoreRequest::Redo {
            id: "split-redo".into(),
        }) {
            CoreReply::Scene {
                scene, document, ..
            } => {
                session.revision = scene.revision;
                *document
            }
            reply => panic!("Split 42 Redo failed: {reply:?}"),
        };
        assert_eq!(redone.parts, moved.parts, "Redo reapplies the edit");
    }

    #[test]
    fn split42_right_half_edit_routes_to_the_left_counterpart() {
        let template = split42();
        let start = template
            .parts
            .iter()
            .find(|part| part.id == "matrix/right-main/r0c0")
            .expect("right key")
            .pose
            .at;
        let mut session = Session::open(template.clone());
        let moved = session.commit(EditOperation::MoveParts {
            positions: vec![Position {
                id: "matrix/right-main/r0c0".into(),
                at: Vec2 {
                    x: start.x,
                    y: start.y + 5.0,
                },
            }],
        });
        let mirror = moved
            .parts
            .iter()
            .find(|part| part.id == "matrix/left-main/r0c0")
            .expect("left counterpart")
            .pose
            .at;
        let mirror_start = template
            .parts
            .iter()
            .find(|part| part.id == "matrix/left-main/r0c0")
            .expect("left key")
            .pose
            .at;
        assert!((mirror.x - mirror_start.x).abs() < 1e-9, "X unchanged");
        assert!(
            (mirror.y - (mirror_start.y + 5.0)).abs() < 1e-9,
            "Y follows"
        );
    }

    #[test]
    fn split42_preview_cancel_leaves_the_document_unchanged() {
        let template = split42();
        let start = template
            .parts
            .iter()
            .find(|part| part.id == "matrix/left-main/r2c2")
            .expect("anchor key")
            .pose
            .at;
        let mut session = Session::open(template.clone());
        let revision = session.revision;
        let preview = session.engine.handle(CoreRequest::Edit {
            id: "split-preview".into(),
            command: EditCommand {
                base_revision: revision,
                transaction_id: "split-preview".into(),
                phase: EditPhase::Preview,
                target_ids: vec![],
                operation: EditOperation::MoveParts {
                    positions: vec![Position {
                        id: "matrix/left-main/r2c2".into(),
                        at: Vec2 {
                            x: start.x + 10.0,
                            y: start.y,
                        },
                    }],
                },
            },
        });
        assert!(
            matches!(preview, CoreReply::Preview { .. }),
            "preview does not commit"
        );
        assert_eq!(
            session.snapshot().parts,
            template.parts,
            "cancel leaves every pose"
        );
        assert!(
            matches!(
                session.engine.handle(CoreRequest::Undo {
                    id: "split-undo".into()
                }),
                CoreReply::Error { .. }
            ),
            "no Undo entry from a preview"
        );
    }

    fn left_has(document: &ProjectDoc, left_id: &str, matrix_id: &str) -> bool {
        document
            .layouts
            .iter()
            .find(|layout| layout.id == left_id)
            .expect("left")
            .matrix_ids
            .contains(&matrix_id.to_string())
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
