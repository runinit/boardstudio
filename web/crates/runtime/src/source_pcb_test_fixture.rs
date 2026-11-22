use boardstudio_core::model::{
    Asset, Board, BoardReference, Operation, OutlineFeature, Pose2, ProjectDoc, Vec2,
};

pub fn rejected_sources() -> [(Vec<u8>, bool, &'static str); 7] {
    let valid = r#"(kicad_pcb (general (thickness 1.6))
        (gr_rect (start 0 0) (end 10 10) (layer "Edge.Cuts") (tstamp board)))"#;
    let incomplete = format!(
        r#"{} (gr_curve (pts (xy 0 0) (xy 1 1) (xy 2 2) (xy 3 3)) (layer "Edge.Cuts") (tstamp extra)))"#,
        valid.strip_suffix(')').unwrap()
    ).into_bytes();
    [
        (valid.as_bytes().to_vec(), true, "accepted SHA-256"),
        (vec![0xff], false, "not valid UTF-8"),
        (
            valid.replace("1.6", "1.2").into_bytes(),
            false,
            "does not match",
        ),
        (incomplete, false, "Unsupported PCB Edge.Cuts"),
        (format!(r#"{} (via blind (at 5 5) (size 0.8) (drill 0.4) (layers "F.Cu" "In1.Cu") (uuid blind-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "blind-via"),
        (format!(r#"{} (via micro (at 5 5) (size 0.8) (drill 0.4) (layers "F.Cu" "In1.Cu") (tstamp micro-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "micro-via"),
        (format!(r#"{} (via blind (at 5 5) (size 0.8) (drill 0.4) (layers "In1.Cu" "In2.Cu") (uuid buried-via)))"#, valid.strip_suffix(')').unwrap()).into_bytes(), false, "buried-via"),
    ]
}

pub fn document(sha256: &str) -> ProjectDoc {
    let mut document = ProjectDoc::empty("source-pcb", "Source PCB");
    let board_id = "board".to_owned();
    document.boards.push(Board {
        id: board_id.clone(),
        name: "Board".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.boards[0].outline_ids.push("outline".into());
    document.outline.push(OutlineFeature::Rect {
        anchor_part_id: None,
        id: "outline".into(),
        center: Vec2 { x: 0., y: 0. },
        size: Vec2 { x: 20., y: 20. },
        radius: 0.,
        rotation: None,
        operation: Operation::Add,
    });
    document.mechanical = Some(
        serde_json::from_value(serde_json::json!({
            "boardId":board_id,"method":"printed","mount":"rigid",
            "plateThickness":1.5,"plateFoamThickness":0.5,"pcbThickness":1.6,
            "bottomFoamThickness":0.4,"batteryHeight":4.,"bottomThickness":3.,
            "plateToPcb":3.5,"wallThickness":2.,"clearance":0.3,
            "bottomStyle":"shell","middleFrame":false,"openingAllowance":0.25,
            "mounts":[],"closureMounts":[],"profiles":[]
        }))
        .unwrap(),
    );
    document.assets.push(Asset {
        id: "pcb".into(),
        name: "source.kicad_pcb".into(),
        media_type: "application/vnd.kicad.pcb".into(),
        sha256: sha256.to_owned(),
        license: None,
        source: None,
    });
    document.board_references.push(BoardReference {
        id: "reference".into(),
        board_id,
        asset_id: "pcb".into(),
        enabled: true,
        pose: Pose2 {
            at: Vec2 { x: 0., y: 0. },
            rotation: 0.,
        },
        elevation: 0.,
        model_assets: Default::default(),
    });
    document
}
