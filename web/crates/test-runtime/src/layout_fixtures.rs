use boardstudio_core::model::{
    Board, Part, PartDefinition, PartKind, Pose2, ProjectDoc, Side, Vec2,
};

fn part(id: &str, x: f64) -> Part {
    Part {
        keycap: None,
        outline: None,
        id: id.into(),
        definition_id: "switch:base".into(),
        reference: id.to_uppercase(),
        pose: Pose2 {
            at: Vec2 { x, y: 0.0 },
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        properties: None,
        generator_parameters: None,
    }
}

pub fn position_edit_document() -> ProjectDoc {
    let mut document = ProjectDoc::empty("remainder-doc", "Remainder");
    document.boards.push(Board {
        id: "board".into(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec!["a".into(), "b".into()],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions.push(PartDefinition {
        id: "switch:base".into(),
        name: "MX switch".into(),
        kind: PartKind::Switch,
        courtyard: vec![
            Vec2 { x: -3.0, y: -3.0 },
            Vec2 { x: 3.0, y: -3.0 },
            Vec2 { x: 3.0, y: 3.0 },
        ],
        hardware_profile: None,
        input_profile: None,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals: Default::default(),
        matrix_terminals: None,
        envelope_notice: None,
        pads: vec![],
        models: None,
        generator: None,
        mechanical_profile: None,
    });
    document.parts.push(part("a", 0.0));
    document.parts.push(part("b", 40.0));
    document
}

pub fn part_position(
    runtime: &boardstudio_web_runtime::runtime::Runtime,
    id: &str,
) -> Option<Vec2> {
    runtime
        .model()
        .accepted?
        .document
        .parts
        .iter()
        .find(|part| part.id == id)
        .map(|part| part.pose.at)
}
