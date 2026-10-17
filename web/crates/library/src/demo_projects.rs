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
