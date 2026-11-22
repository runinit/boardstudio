use super::in_process_support::DeliveredArtifact;
use super::project_name_test_support as support;
use crate::runtime::{Runtime, test_hooks};
use boardstudio_application::Scope;
use boardstudio_application::{AcceptedSnapshot, Effect, ReadModel};
use boardstudio_model::firmware::FirmwarePackage;
use boardstudio_model::model::*;
use futures_channel::oneshot;
use std::{
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Resolution,
    Generation,
}

fn resolution(request: &CoreRequest) -> bool {
    matches!(request, CoreRequest::ResolveElectrical { .. })
}
fn generation(request: &CoreRequest) -> bool {
    matches!(request, CoreRequest::GenerateFirmware { .. })
}
fn generated(request: &CoreRequest) -> Option<CoreReply> {
    let CoreRequest::GenerateFirmware { id, .. } = request else {
        return None;
    };
    Some(CoreReply::FirmwareGenerated {
        id: id.clone(),
        package: FirmwarePackage {
            files: BTreeMap::from([(
                "config/boards/test.keymap".into(),
                "// generated test keymap".into(),
            )]),
            warnings: Vec::new(),
        },
    })
}

pub async fn fixture() -> (Rc<Runtime>, AcceptedSnapshot, Scope) {
    let runtime = support::new_runtime();
    support::open_document(&runtime, wired_document()).await;
    support::in_process_core(&runtime).script_replies(generated);
    let accepted = runtime.model().accepted.unwrap();
    let scope = runtime.scope().unwrap();
    (runtime, accepted, scope)
}

pub fn fail(runtime: &Runtime, stage: Stage) {
    let core = support::in_process_core(runtime);
    match stage {
        Stage::Resolution => {
            core.fail_matching_reply(resolution, "injected electrical-resolution failure")
        }
        Stage::Generation => {
            core.fail_matching_reply(generation, "injected firmware-generation failure")
        }
    }
}

pub fn gate(runtime: &Runtime, stage: Stage) -> (oneshot::Receiver<()>, oneshot::Sender<()>) {
    let core = support::in_process_core(runtime);
    match stage {
        Stage::Resolution => core.gate_matching_reply(resolution),
        Stage::Generation => core.gate_matching_reply(generation),
    }
}

pub async fn run_effects(runtime: &Rc<Runtime>, initial: Vec<Effect>) {
    let mut effects = VecDeque::from(initial);
    while let Some(effect) = effects.pop_front() {
        effects.extend(support::run_effect(runtime, effect).await);
    }
}

pub fn start_export(runtime: &Rc<Runtime>) -> Vec<Effect> {
    runtime.export_firmware();
    support::take_held_effects(runtime)
}
pub fn take_effects(runtime: &Runtime) -> Vec<Effect> {
    support::take_held_effects(runtime)
}
pub fn take_deliveries(runtime: &Runtime) -> Vec<DeliveredArtifact> {
    support::installed(runtime).take_deliveries()
}
pub fn session_model(runtime: &Runtime) -> ReadModel {
    runtime.model()
}
pub fn has_artifacts(runtime: &Runtime) -> bool {
    test_hooks::has_artifacts(runtime)
}

fn definition(
    id: &str,
    kind: PartKind,
    terminals: BTreeMap<String, Vec<String>>,
    source: Option<&str>,
) -> PartDefinition {
    let pads = terminals
        .values()
        .flatten()
        .map(|id| Pad {
            id: id.clone(),
            number: id.clone(),
            at: Vec2::default(),
            size: Vec2 { x: 1.0, y: 1.0 },
            shape: PadShape::Circle,
            drill: None,
            plated: Some(true),
            side: None,
            rotation: None,
            net_id: None,
        })
        .collect();
    PartDefinition {
        hardware_profile: None,
        input_profile: None,
        id: id.into(),
        name: id.into(),
        kind,
        keycap: None,
        envelope_source: None,
        kicad_source: None,
        terminals,
        matrix_terminals: Some(MatrixTerminals {
            row: "row".into(),
            column: "column".into(),
        }),
        envelope_notice: None,
        courtyard: vec![],
        pads,
        models: None,
        generator: source.map(|s| PartGenerator {
            source: s.into(),
            version: "test".into(),
            parameters: BTreeMap::new(),
        }),
        mechanical_profile: None,
    }
}

fn part(id: &str, definition_id: &str) -> Part {
    Part {
        id: id.into(),
        definition_id: definition_id.into(),
        reference: id.into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        anchor: None,
        locked: None,
        keycap: None,
        outline: None,
        properties: None,
        generator_parameters: None,
    }
}

fn wired_document() -> ProjectDoc {
    let mut doc = ProjectDoc::empty("zmk-export-test", "ZMK export test");
    let mut sw_terms = BTreeMap::new();
    sw_terms.insert("row".into(), vec!["1".into()]);
    sw_terms.insert("column".into(), vec!["2".into()]);
    doc.definitions
        .push(definition("switch", PartKind::Switch, sw_terms, None));
    let mut diode_terms = BTreeMap::new();
    diode_terms.insert("anode".into(), vec!["A".into()]);
    diode_terms.insert("cathode".into(), vec!["K".into()]);
    doc.definitions
        .push(definition("diode", PartKind::Passive, diode_terms, None));
    let mut mcu_terms = BTreeMap::new();
    for p in [
        "GND", "P1", "P2", "P3", "P4", "P5", "P6", "P7", "P8", "P9", "P10", "P14", "P15", "P16",
        "P18", "P19", "P20", "P21",
    ] {
        mcu_terms.insert(p.into(), vec![p.into()]);
    }
    doc.definitions.push(definition(
        "mcu",
        PartKind::Controller,
        mcu_terms,
        Some("ceoloide/mcu_nice_nano"),
    ));
    let mut matrix_parts = vec![];
    for r in 0..2 {
        for c in 0..2 {
            let id = format!("matrix/m/r{r}c{c}");
            matrix_parts.push(id.clone());
            doc.parts.push(part(&id, "switch"));
            let did = format!("{id}/diode");
            matrix_parts.push(did.clone());
            doc.parts.push(part(&did, "diode"));
        }
    }
    doc.parts.push(part("mcu-left", "mcu"));
    doc.boards.push(Board {
        id: "board-a".into(),
        name: "A".into(),
        outline_ids: vec![],
        part_ids: doc.parts.iter().map(|p| p.id.clone()).collect(),
        net_ids: vec![],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    doc.matrices.push(Matrix {
        id: "m".into(),
        name: None,
        rows: 2,
        columns: 2,
        pitch: Vec2 { x: 19.0, y: 19.0 },
        origin: Vec2::default(),
        definition_id: "switch".into(),
        part_ids: matrix_parts,
        board_id: Some("board-a".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,

        diode_direction: Some(DiodeDirection::Row2col),
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: (0..2)
            .flat_map(|r| {
                (0..2).map(move |c| MatrixCell {
                    shift: None,
                    bend: None,
                    deleted: false,
                    row: r,
                    column: c,
                    enabled: true,

                    definition_id: Some("switch".into()),
                    variant: None,
                    offset: None,
                    rotation: None,
                    assemblies: vec![],
                    assemblies_local: None,
                })
            })
            .collect(),
    });
    doc.hardware = Some(HardwareConfiguration {
        topology: HardwareTopology::Unibody,
        transport: HardwareTransport::None,
        instances: Vec::new(),
        shared_construction: None,
        pairs: Vec::new(),
        boards: vec![ElectricalBoardConfiguration {
            board_id: "board-a".into(),
            controller_part_id: Some("mcu-left".into()),
            ..Default::default()
        }],
    });
    doc
}
