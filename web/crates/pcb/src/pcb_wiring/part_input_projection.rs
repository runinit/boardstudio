//! Host-compatible part-input projection for the PCB wiring panel.
//!
//! The production inspector (wasm) delegates to this module so the same
//! projection logic runs under `cargo test` without a browser.

use boardstudio_model::model::{Part, PartDefinition, PartKind, PressScanMode, ProjectDoc};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const ROTARY_ENCODER_SOURCE: &str = "ceoloide/rotary_encoder_ec11_ec12";

#[derive(Clone, Debug, PartialEq)]
pub struct PartInputProjection {
    pub part: Part,
    pub definition: PartDefinition,
    pub board_name: String,
    pub press_mode: Option<PressScanMode>,
    pub matrix_mode_enabled: bool,
    pub nets: Vec<NetChoice>,
    pub generator_parameters: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NetChoice {
    pub id: String,
    pub name: String,
}

pub fn project(
    document: &ProjectDoc,
    board_id: &str,
    part_id: &str,
) -> Option<PartInputProjection> {
    let board = document.boards.iter().find(|board| board.id == board_id)?;
    let part = document.parts.iter().find(|part| part.id == part_id)?;
    if !board.part_ids.iter().any(|id| id == part_id) {
        return None;
    }
    let definition = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)?;
    if matches!(definition.kind, PartKind::Switch | PartKind::Controller) {
        return None;
    }
    let source = definition
        .generator
        .as_ref()
        .map(|generator| generator.source.as_str());
    let press = definition
        .input_profile
        .as_ref()
        .and_then(|profile| profile.press.as_ref());
    let has_press = press.is_some() || source == Some(ROTARY_ENCODER_SOURCE);
    let in_matrix = document
        .matrices
        .iter()
        .any(|matrix| matrix.part_ids.iter().any(|id| id == part_id));
    let matrix_mode_enabled =
        has_press && in_matrix && press.is_none_or(|profile| profile.independent);
    let press_mode = has_press.then(|| {
        part.properties
            .as_ref()
            .and_then(|properties| properties.get("pressScanMode"))
            .and_then(|value| serde_json::from_value(value.clone()).ok())
            .unwrap_or(if in_matrix {
                PressScanMode::Matrix
            } else {
                PressScanMode::Direct
            })
    });
    let nets = super::part_connections::board_nets(document, board)
        .into_iter()
        .map(|net| NetChoice {
            id: net.id.clone(),
            name: net.name.clone(),
        })
        .collect();
    let generator_parameters = part.generator_parameters.clone().unwrap_or_default();

    Some(PartInputProjection {
        part: part.clone(),
        definition: definition.clone(),
        board_name: board.name.clone(),
        press_mode,
        matrix_mode_enabled,
        nets,
        generator_parameters,
    })
}

#[allow(clippy::too_many_arguments, dead_code)]
pub(crate) fn part_edit_matches_live(
    ui_scope: &boardstudio_application::Scope,
    board_id: &str,
    part_id: &str,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
    scope_generation: u64,
    live: &super::PcbWiringSource,
    selected_part_ids: &[String],
    current_generation: u64,
) -> bool {
    scope_generation == current_generation
        && live.scope_generation == current_generation
        && live.ui_scope == *ui_scope
        && live.identity.scope.board_id == board_id
        && live.identity.token == token
        && live.identity.revision == revision
        && live.active_part_id.as_deref() == Some(part_id)
        && selected_part_ids == [part_id]
}

pub fn binding_schema(
    schema: &BTreeMap<String, Value>,
    terminal_names: &BTreeSet<String>,
) -> Vec<(String, String)> {
    schema
        .iter()
        .filter_map(|(name, parameter)| {
            let kind = parameter.get("type")?.as_str()?;
            match kind {
                "net" if !terminal_names.contains(name) => Some((name.clone(), kind.to_owned())),
                "anchor" => Some((name.clone(), kind.to_owned())),
                _ => None,
            }
        })
        .collect()
}

pub fn validate_parameter_value(
    kind: &str,
    value: Option<&Value>,
    projection: &PartInputProjection,
) -> Result<(), String> {
    match (kind, value) {
        ("net", None) => Ok(()),
        ("net", Some(Value::String(value)))
            if projection.nets.iter().any(|net| net.name == *value) =>
        {
            Ok(())
        }
        ("anchor", None) => Ok(()),
        ("anchor", Some(Value::Object(values)))
            if ["x", "y"].into_iter().all(|axis| {
                values
                    .get(axis)
                    .is_none_or(|value| value.as_f64().is_some_and(f64::is_finite))
            }) =>
        {
            Ok(())
        }
        ("net", _) => Err("Choose a net on the selected board.".into()),
        ("anchor", _) => Err("Anchor coordinates must be finite numbers.".into()),
        _ => Err("Unsupported generator binding type.".into()),
    }
}

#[cfg(test)]
pub(super) fn project_fixture() -> ProjectDoc {
    use boardstudio_model::model::{Board, Matrix};
    use serde_json::json;

    let mut document = ProjectDoc::empty("project", "Input fixture");
    document.boards.push(Board {
        id: "left".into(),
        name: "Left PCB".into(),
        outline_ids: vec![],
        part_ids: vec!["matrix/main/r0c0".into(), "switch".into(), "mcu".into()],
        net_ids: vec!["vcc".into()],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.nets = serde_json::from_value(json!([
        {"id":"vcc","name":"VCC","pins":[]},
        {"id":"legacy","name":"Legacy","pins":[{"partId":"mcu","padId":"P1"}]},
        {"id":"other","name":"Other board only","pins":[]}
    ]))
    .unwrap();
    document.boards.push(Board {
        id: "right".into(),
        name: "Right PCB".into(),
        outline_ids: vec![],
        part_ids: vec![],
        net_ids: vec!["other".into()],
        thickness: 1.6,
        traces: vec![],
        vias: vec![],
    });
    document.definitions = serde_json::from_value(json!([
        {"id":"encoder","name":"EC11/EC12","kind":"encoder","inputProfile":{"press":{"row":"S1","column":"S2","independent":true}},"courtyard":[],"pads":[],"terminals":{},"generator":{"source":"ceoloide/rotary_encoder_ec11_ec12","version":"bundled-1","parameters":{}}},
        {"id":"switch-def","name":"MX switch","kind":"switch","courtyard":[],"pads":[],"terminals":{}},
        {"id":"mcu-def","name":"MCU","kind":"controller","courtyard":[],"pads":[],"terminals":{}},
        {"id":"rotation-only","name":"Rotation only","kind":"encoder","inputProfile":{"rotary":{"a":"A","b":"B","common":"C"}},"courtyard":[],"pads":[],"terminals":{}}
    ])).unwrap();
    document.parts = serde_json::from_value(json!([
        {"id":"matrix/main/r0c0","definitionId":"encoder","reference":"ENC1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},
        {"id":"switch","definitionId":"switch-def","reference":"SW1","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"},
        {"id":"mcu","definitionId":"mcu-def","reference":"MCU","pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front"}
    ])).unwrap();
    document.matrices.push(Matrix {
        id: "main".into(),
        name: None,
        rows: 1,
        columns: 1,
        pitch: boardstudio_model::model::Vec2 { x: 19.0, y: 19.0 },
        origin: boardstudio_model::model::Vec2 { x: 0.0, y: 0.0 },
        definition_id: "encoder".into(),
        part_ids: vec!["matrix/main/r0c0".into()],
        board_id: Some("left".into()),
        mirror: None,
        rotation: None,
        edge_gap: None,
        diode_direction: None,
        row_layout: vec![],
        column_layout: vec![],
        anchor: None,
        cells: vec![],
    });
    document
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    pub fn projection_for_validation() -> PartInputProjection {
        project(&project_fixture(), "left", "matrix/main/r0c0").unwrap()
    }

    #[test]
    fn pcb_part_input_projection_preserves_ts_routing_and_matrix_predicate() {
        let document = project_fixture();
        let encoder = project(&document, "left", "matrix/main/r0c0").unwrap();
        assert_eq!(encoder.press_mode, Some(PressScanMode::Matrix));
        assert!(encoder.matrix_mode_enabled);
        assert_eq!(
            encoder
                .nets
                .iter()
                .map(|net| net.id.as_str())
                .collect::<Vec<_>>(),
            ["vcc", "legacy"]
        );
        assert!(project(&document, "left", "switch").is_none());
        assert!(project(&document, "left", "mcu").is_none());
        assert!(project(&document, "right", "matrix/main/r0c0").is_none());
    }

    #[test]
    fn pcb_part_input_matrix_choice_requires_membership_and_independent_press() {
        let mut document = project_fixture();
        document.matrices.clear();
        assert!(
            !project(&document, "left", "matrix/main/r0c0")
                .unwrap()
                .matrix_mode_enabled
        );
        document.matrices = project_fixture().matrices;
        document.definitions[0]
            .input_profile
            .as_mut()
            .unwrap()
            .press
            .as_mut()
            .unwrap()
            .independent = false;
        assert!(
            !project(&document, "left", "matrix/main/r0c0")
                .unwrap()
                .matrix_mode_enabled
        );
    }

    #[test]
    fn pcb_part_input_matrix_choice_uses_membership_without_assuming_a_part_id_format() {
        let mut document = project_fixture();
        document.parts[0].id = "user-owned-encoder".into();
        document.boards[0].part_ids[0] = "user-owned-encoder".into();
        document.matrices[0].part_ids[0] = "user-owned-encoder".into();
        let projection = project(&document, "left", "user-owned-encoder").unwrap();
        assert!(projection.matrix_mode_enabled);
        assert_eq!(projection.press_mode, Some(PressScanMode::Matrix));
    }

    #[test]
    fn pcb_part_input_schema_keeps_only_supported_nonterminal_net_and_anchor_fields() {
        let schema = BTreeMap::from([
            ("VCC".into(), json!({"type":"net"})),
            ("battery".into(), json!({"type":"net"})),
            ("anchor".into(), json!({"type":"anchor"})),
            ("width".into(), json!({"type":"number"})),
            ("options".into(), json!({"type":"object"})),
        ]);
        let fields = binding_schema(&schema, &BTreeSet::from(["VCC".into()]));
        assert_eq!(
            fields,
            [
                ("anchor".into(), "anchor".into()),
                ("battery".into(), "net".into())
            ]
        );
    }

    #[test]
    fn parameter_values_are_limited_to_board_nets_and_finite_anchor_coordinates() {
        let projection = projection_for_validation();
        assert!(validate_parameter_value("net", Some(&json!("VCC")), &projection).is_ok());
        assert!(
            validate_parameter_value("net", Some(&json!("Other board only")), &projection).is_err()
        );
        assert!(validate_parameter_value("anchor", Some(&json!({"x": 2.5})), &projection).is_ok());
        assert!(
            validate_parameter_value("anchor", Some(&json!({"x": f64::INFINITY})), &projection)
                .is_err()
        );
        assert!(validate_parameter_value("anchor", Some(&json!([])), &projection).is_err());
    }

    #[test]
    fn rendered_part_action_is_rejected_after_board_selection_or_snapshot_changes() {
        use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
        let scope = |board_id: &str| Scope {
            session_epoch: SessionEpoch(3),
            document_id: "project".into(),
            board_id: board_id.into(),
            instance_id: None,
        };
        let document = std::sync::Arc::new(project_fixture());
        let source = |board_id: &str, part_id: &str, token: u64, rev: u64, gen_id: u64| {
            super::super::PcbWiringSource {
                identity: super::super::WiringPlanIdentity {
                    scope: scope(board_id),
                    token: SnapshotToken(token),
                    revision: rev,
                },
                ui_scope: scope(board_id),
                scope_generation: gen_id,
                document: document.clone(),
                active_part_id: Some(part_id.into()),
            }
        };
        let base_scope = scope("left");
        let selected = [String::from("left-J2")];
        let matches = |live: &super::super::PcbWiringSource, sel: &[String], cur_gen: u64| {
            part_edit_matches_live(
                &base_scope,
                "left",
                "left-J2",
                SnapshotToken(7),
                12,
                4,
                live,
                sel,
                cur_gen,
            )
        };
        assert!(matches(&source("left", "left-J2", 7, 12, 4), &selected, 4));
        for (alt, sel, cur_gen) in [
            (source("right", "left-J2", 7, 12, 4), selected.as_slice(), 4),
            (source("left", "left-SW1", 7, 12, 4), selected.as_slice(), 4),
            (source("left", "left-J2", 8, 12, 4), selected.as_slice(), 4),
            (source("left", "left-J2", 7, 13, 4), selected.as_slice(), 4),
            (source("left", "left-J2", 7, 12, 5), selected.as_slice(), 4),
            (source("left", "left-J2", 7, 12, 4), selected.as_slice(), 5),
            (source("left", "left-J2", 7, 12, 4), &[], 4),
        ] {
            assert!(!matches(&alt, sel, cur_gen));
        }
    }
}
