use boardstudio_model::mirrored_half::{MirrorDestination, MirroredPair};
use boardstudio_model::model::{
    Board, ColumnLayout, CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Follow,
    Layout, Material, Matrix, MatrixAnchor, Mirror, Operation, OutlineFeature, OutlineSettings,
    PartDefinition, PartKind, Pose2, ProjectDoc, RowLayout, Vec2,
};
use boardstudio_web_runtime::core_services::{OpenScratchCore, ScratchCore};

#[derive(Clone, Copy)]
pub(crate) struct DemoProject {
    pub(crate) name: &'static str,
    #[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
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

struct DemoBuilder {
    engine: ScratchCore,
    revision: u64,
}

impl DemoBuilder {
    fn open(document: ProjectDoc, open_scratch: OpenScratchCore) -> Result<Self, String> {
        let mut engine = open_scratch();
        match engine(CoreRequest::Open {
            id: "demo-open".into(),
            document,
        }) {
            CoreReply::Scene { scene, .. } => Ok(Self {
                engine,
                revision: scene.revision,
            }),
            CoreReply::Error { message, .. } => Err(message),
            reply => Err(format!("demo Open failed: {reply:?}")),
        }
    }

    fn commit(&mut self, operation: EditOperation) -> Result<ProjectDoc, String> {
        let rev = self.revision;
        match (self.engine)(CoreRequest::Edit {
            id: "demo-edit".into(),
            command: EditCommand {
                base_revision: rev,
                transaction_id: format!("demo-{rev}"),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        }) {
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

    fn snapshot(&mut self) -> Result<ProjectDoc, String> {
        match (self.engine)(CoreRequest::Snapshot {
            id: "demo-snapshot".into(),
        }) {
            CoreReply::Scene { document, .. } => Ok(*document),
            CoreReply::Error { message, .. } => Err(message),
            reply => Err(format!("demo snapshot failed: {reply:?}")),
        }
    }
}

const DEMO_PITCH: Vec2 = Vec2 { x: 19.05, y: 19.05 };
const DEMO_GAP: Vec2 = Vec2 { x: 1.05, y: 1.05 };

fn demo_shell(name: &str) -> ProjectDoc {
    let mut doc = ProjectDoc::empty("demo-template", name);
    doc.definitions.push(PartDefinition {
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
        courtyard: vec![],
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    });
    doc.outline.push(OutlineFeature::PartEnvelope {
        connections: vec![],
        settings: OutlineSettings::default(),
        id: "demo-outline".into(),
        part_ids: vec![],
        margin: 4.0,
        operation: Operation::Add,
    });
    doc.boards.push(Board {
        id: "demo-board".into(),
        name: "Main board".into(),
        outline_ids: vec!["demo-outline".into()],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.materials.push(Material {
        id: "pla".into(),
        name: "PLA".into(),
        thickness: 3.0,
    });
    doc
}

fn demo_matrix(id: &str, name: Option<&str>, rows: u32, columns: u32, origin: Vec2) -> Matrix {
    Matrix {
        id: id.into(),
        name: name.map(str::to_owned),
        rows,
        columns,
        pitch: DEMO_PITCH,
        origin,
        definition_id: "demo-switch".into(),
        part_ids: vec![],
        board_id: None,
        mirror: None,
        rotation: None,
        edge_gap: Some(DEMO_GAP),
        diode_direction: None,
        column_layout: vec![],
        row_layout: vec![],
        anchor: None,
        cells: vec![],
    }
}

fn single_matrix(
    name: &str,
    rows: u32,
    cols: u32,
    origin: Vec2,
    open_scratch: OpenScratchCore,
) -> ProjectDoc {
    let mut b = DemoBuilder::open(demo_shell(name), open_scratch).expect("open demo");
    let mut m = demo_matrix("main", Some("Main"), rows, cols, origin);
    m.row_layout = vec![
        RowLayout::default(),
        RowLayout {
            shift: Vec2 { x: 4.7625, y: 0.0 },
            ..Default::default()
        },
    ];
    b.commit(EditOperation::SetMatrix {
        matrix: m,
        definitions: None,
    })
    .expect("matrix");
    let mut doc = b.snapshot().expect("snap");
    doc.layouts.push(Layout {
        id: "layout".into(),
        name: "Main".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec!["main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    let mut doc = b
        .commit(EditOperation::ReplaceDocument {
            document: Box::new(doc),
        })
        .expect("replace");
    doc.revision = 0;
    doc
}

fn split42(name: &str, open_scratch: OpenScratchCore) -> ProjectDoc {
    let mut b = DemoBuilder::open(demo_shell(name), open_scratch).expect("open split42");
    let mut lm = demo_matrix(
        "left-main",
        Some("Left main"),
        3,
        6,
        Vec2 {
            x: -31.0,
            y: -19.05,
        },
    );
    lm.mirror = Some(Mirror::X);
    let shift = RowLayout {
        shift: Vec2 { x: -2.5, y: 0.0 },
        ..Default::default()
    };
    lm.row_layout = vec![RowLayout::default(), shift, shift];
    b.commit(EditOperation::SetMatrix {
        matrix: lm,
        definitions: None,
    })
    .expect("left-main");
    let mut doc = b.snapshot().expect("snap");
    doc.layouts.push(Layout {
        id: "left".into(),
        name: "Left".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec!["left-main".into()],
        part_ids: vec![],
        mirror_link: None,
    });
    b.commit(EditOperation::ReplaceDocument {
        document: Box::new(doc),
    })
    .expect("replace");
    let anchor = MatrixAnchor {
        matrix_id: "left-main".into(),
        row: 2,
        column: 2,
        follow: Follow {
            x: true,
            y: true,
            rotation: true,
        },
        aggregate: None,
    };
    b.commit(EditOperation::AddAnchoredMatrix {
        id: "left-thumb".into(),
        anchor: anchor.clone(),
    })
    .expect("thumb");
    let mut lt = demo_matrix(
        "left-thumb",
        Some("Left thumb"),
        1,
        3,
        Vec2 { x: 0.1, y: 20.95 },
    );
    lt.board_id = Some("demo-board".into());
    lt.mirror = Some(Mirror::X);
    lt.anchor = Some(anchor);
    let stag = ColumnLayout {
        stagger: 3.0,
        ..Default::default()
    };
    lt.column_layout = vec![ColumnLayout::default(), stag, stag];
    b.commit(EditOperation::SetMatrix {
        matrix: lt,
        definitions: None,
    })
    .expect("left-thumb");
    let mut doc = b.snapshot().expect("snap");
    doc.layouts.push(Layout {
        id: "right".into(),
        name: "Right".into(),
        board_id: "demo-board".into(),
        matrix_ids: vec![],
        part_ids: vec![],
        mirror_link: None,
    });
    doc.mirrored_pairs.push(MirroredPair {
        id: "right".into(),
        source: MirrorDestination::SameBoardLayout("left".into()),
        destination: MirrorDestination::SameBoardLayout("right".into()),
        mapping: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        overrides: vec![],
    });
    let mut doc = b
        .commit(EditOperation::ReplaceDocument {
            document: Box::new(doc),
        })
        .expect("replace");
    doc.revision = 0;
    doc
}

impl DemoProject {
    pub(crate) fn document(self, open_scratch: OpenScratchCore) -> ProjectDoc {
        match self.layout {
            DemoLayout::Studio60 => single_matrix(
                self.name,
                5,
                12,
                Vec2 {
                    x: -104.775,
                    y: -38.1,
                },
                open_scratch,
            ),
            DemoLayout::Macro12 => single_matrix(
                self.name,
                3,
                4,
                Vec2 {
                    x: -28.575,
                    y: -19.05,
                },
                open_scratch,
            ),
            DemoLayout::Split42 => split42(self.name, open_scratch),
        }
    }
}

#[cfg(test)]
mod demo_rebuild_tests {
    use super::*;
    use boardstudio_core_kernel::document_format::CURRENT_VERSION;

    fn build_demo(name: &str) -> ProjectDoc {
        DEMO_PROJECTS
            .iter()
            .find(|d| d.name == name)
            .expect("demo")
            .document(boardstudio_web_test_runtime::engine_core_services().open_scratch)
    }

    #[test]
    fn all_demos_build_through_core_with_expected_key_counts_and_invariants() {
        for (name, expected_keys) in [("Split 42", 42), ("Studio 60", 60), ("Macro 12", 12)] {
            let doc = build_demo(name);
            assert_eq!(doc.format_version, CURRENT_VERSION);
            assert_eq!((doc.boards.len(), doc.materials.len()), (1, 1));
            assert!(doc.hardware.is_none() && doc.nets.is_empty() && doc.modules.is_empty());
            assert_eq!(
                boardstudio_core::project_document(&doc)
                    .geometry()
                    .parts
                    .len(),
                expected_keys
            );
            for m in &doc.matrices {
                assert_eq!((m.pitch, m.edge_gap), (DEMO_PITCH, Some(DEMO_GAP)));
            }
        }
    }
}
