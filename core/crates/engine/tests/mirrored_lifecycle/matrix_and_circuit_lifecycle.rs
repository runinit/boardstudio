use super::*;
use crate::independent_exceptions::independent_lifecycle_fixture;

pub(crate) fn circuit_lifecycle_fixture() -> ProjectDoc {
    let mut document = independent_lifecycle_fixture();
    document.definitions.push(serde_json::from_value(json!({"id":"circuit-part","name":"Circuit part","kind":"custom","courtyard":[],
        "pads":[{"id":"pad-1","number":"1","at":{"x":0,"y":0},"size":{"x":1,"y":1},"shape":"circle"}]})).unwrap());
    for part in document
        .parts
        .iter_mut()
        .filter(|part| part.id.ends_with("-screen"))
    {
        part.definition_id = "circuit-part".into();
    }
    for member in ["U1", "U2"] {
        for (half, board_id) in [("source", "source-board"), ("mirror", "mirror-board")] {
            let id = format!("embedded/{half}-circuit/{member}");
            let mut part = document.parts[0].clone();
            part.id = id.clone();
            part.reference = format!("{half}-{member}");
            part.definition_id = "circuit-part".into();
            part.properties = Some(BTreeMap::from([("revision".into(), json!(1))]));
            document.parts.push(part);
            document.boards.by_id_mut(board_id).part_ids.push(id);
        }
    }
    for half in ["source", "mirror"] {
        let private = format!("embedded/{half}-circuit/N1");
        let joined = format!("{half}-host-net");
        document.nets.extend([
            Net {
                id: private.clone(),
                name: "PRIVATE".into(),
                pins: vec![Pin {
                    part_id: format!("embedded/{half}-circuit/U1"),
                    pad_id: "pad-1".into(),
                }],
            },
            Net {
                id: joined.clone(),
                name: format!("{half} HOST"),
                pins: vec![
                    Pin {
                        part_id: format!("embedded/{half}-circuit/U1"),
                        pad_id: "pad-1".into(),
                    },
                    Pin {
                        part_id: format!("embedded/{half}-circuit/U2"),
                        pad_id: "pad-1".into(),
                    },
                    Pin {
                        part_id: format!("{half}-screen"),
                        pad_id: "pad-1".into(),
                    },
                ],
            },
        ]);
        let board_id = format!("{half}-board");
        document
            .boards
            .by_id_mut(&board_id)
            .net_ids
            .extend([private.clone(), joined.clone()]);
        document.embedded_circuits.push(EmbeddedCircuit {
            id: format!("{half}-circuit"),
            definition_id: "circuit-module".into(),
            host_board_id: board_id,
            part_ids: ["U1", "U2"]
                .map(|member| format!("embedded/{half}-circuit/{member}"))
                .to_vec(),
            net_ids: vec![private, joined.clone()],
            ports: BTreeMap::from([("signal".into(), joined)]),
        });
    }
    document.module_definitions.push(serde_json::from_value(json!({
        "id":"circuit-module","name":"Circuit module","family":"fixture","variant":"fixture",
        "source":{"repository":"https://example.invalid/fixture","revision":"fixture","path":"fixture.kicad_pcb","license":"MIT"},
        "board":{"contours":[]},"mounts":[],"volumes":[],"openings":[],"models":[],"gates":[],"interfaces":[],
        "electrical":{"protocol":"pass-through","requiredSignals":[],"logicVoltage":3.3},"constituents":[]
    })).unwrap());
    for half in ["source", "mirror"] {
        document.modules.push(serde_json::from_value(json!({
            "id":format!("{half}-module"),"definitionId":"circuit-module","hostBoardId":format!("{half}-board"),
            "hostFace":"front","facingFace":"back","at":{"x":4,"y":3},"rotation":12,"gap":0.4,"attachment":"board",
            "connection":{"hostConnectorPartId":format!("{half}-screen"),"modulePortId":"host","busId":"","cableType":"fixture","assignments":{},"railVoltages":{}}
        })).unwrap());
    }
    document.layouts.push(Layout {
        id: "independent-layout".into(),
        name: "Independent".into(),
        board_id: "mirror-board".into(),
        matrix_ids: vec!["mirror-matrix".into()],
        part_ids: vec!["mirror-screen".into()],
        mirror_link: None,
    });
    add_link(
        &mut document,
        MirroredItemKind::EmbeddedCircuit,
        "source-circuit",
        "mirror-circuit",
    );
    for member in ["U1", "U2"] {
        add_link(
            &mut document,
            MirroredItemKind::Part,
            &format!("embedded/source-circuit/{member}"),
            &format!("embedded/mirror-circuit/{member}"),
        );
    }
    add_link(
        &mut document,
        MirroredItemKind::Net,
        "embedded/source-circuit/N1",
        "embedded/mirror-circuit/N1",
    );
    add_link(
        &mut document,
        MirroredItemKind::Net,
        "source-host-net",
        "mirror-host-net",
    );
    add_link(
        &mut document,
        MirroredItemKind::Module,
        "source-module",
        "mirror-module",
    );
    boardstudio_core_mechanical::mechanical::halves::validate_document(&document).unwrap();
    document
}
