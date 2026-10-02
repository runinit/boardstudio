use super::PcbWiringSource;
use boardstudio_core::model::{Part, PartDefinition, PartKind, PressScanMode, ProjectDoc};
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

mod owner;
pub(in crate::presentation) use owner::{
    PartInputActions, PartInputEditRequest, PartInputFeedbackState, PartInputIdentity,
    PartInputIntent, use_part_input_edits,
};

const ROTARY_ENCODER_SOURCE: &str = "ceoloide/rotary_encoder_ec11_ec12";

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PartInputProjection {
    pub part: Part,
    pub definition: PartDefinition,
    pub board_name: String,
    pub press_mode: Option<PressScanMode>,
    pub matrix_mode_enabled: bool,
    pub nets: Vec<NetChoice>,
    pub generator_parameters: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct NetChoice {
    pub id: String,
    pub name: String,
}

pub(super) fn project(
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
    let board_net_ids = board.net_ids.iter().collect::<BTreeSet<_>>();
    let nets = document
        .nets
        .iter()
        .filter(|net| {
            board_net_ids.contains(&net.id)
                || net
                    .pins
                    .iter()
                    .any(|pin| board.part_ids.iter().any(|part_id| part_id == &pin.part_id))
        })
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

pub(super) fn binding_schema(
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

#[derive(Props, Clone, PartialEq)]
pub(super) struct PartInputInspectorProps {
    pub source: PcbWiringSource,
    pub actions: PartInputActions,
    pub board_details_and_connections: Element,
}

#[component]
pub(super) fn PartInputInspector(props: PartInputInspectorProps) -> Element {
    let Some(part_id) = props.source.active_part_id.as_deref() else {
        return rsx! {};
    };
    let Some(projection) = project(
        &props.source.document,
        &props.source.identity.scope.board_id,
        part_id,
    ) else {
        return rsx! {};
    };
    let Some(identity) = edit_identity(&props.source, &projection) else {
        return rsx! {};
    };
    let generator_source = projection
        .definition
        .generator
        .as_ref()
        .map(|generator| generator.source.clone());
    let schema = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        match source {
            Some(source) => crate::presentation::parts::ergogen_parameter_schema(source).await,
            None => Ok(BTreeMap::new()),
        }
    }));
    let schema_state = schema.read().clone();
    let matching_feedback = props
        .actions
        .feedback
        .clone()
        .filter(|feedback| feedback.identity == identity);
    let read_only = !props.actions.editable || projection.part.locked == Some(true);
    let on_edit = props.actions.on_edit;
    let scan_mode = projection.press_mode;
    let matrix_enabled = projection.matrix_mode_enabled;
    let scan_identity = identity.clone();
    let scan_change = move |event: FormEvent| {
        let mode = match event.value().as_str() {
            "matrix" => PressScanMode::Matrix,
            "direct" => PressScanMode::Direct,
            "unassigned" => PressScanMode::Unassigned,
            _ => return,
        };
        if mode == PressScanMode::Matrix && !matrix_enabled {
            return;
        }
        on_edit.call(PartInputEditRequest {
            identity: scan_identity.clone(),
            intent: PartInputIntent::ScanMode(mode),
        });
    };

    rsx! {
        section { class: "m1-pcb-wiring m1-pcb-part-input",
            if let Some(scan_mode) = scan_mode {
                details { class: "m1-pcb-wiring-section", open: true,
                    summary { "Press input" }
                    label { class: "m1-pcb-part-input-field",
                        span { "Scan mode" }
                        select {
                            aria_label: "Press scan mode",
                            value: scan_mode_text(scan_mode),
                            disabled: read_only,
                            onchange: scan_change,
                            option { value: "matrix", disabled: !matrix_enabled, "Matrix key" }
                            option { value: "direct", "Direct GPIO" }
                            option { value: "unassigned", "Unassigned" }
                        }
                    }
                    p { class: "m1-pcb-part-input-note", "Rotation uses separate GPIOs. Apply the board wiring plan after changing the press connection." }
                }
            }
            {props.board_details_and_connections}
            match schema_state {
                Some(Ok(schema)) => {
                    let terminals = projection.definition.terminals.keys().cloned().collect::<BTreeSet<_>>();
                    let fields = binding_schema(&schema, &terminals);
                    if fields.is_empty() {
                        rsx! {}
                    } else {
                        rsx! {
                            details { class: "m1-pcb-wiring-section m1-pcb-part-bindings", open: true,
                                summary { "Ergogen bindings" }
                                for (name, kind) in fields {
                                    if kind == "net" {
                                        {
                                            let current = projection.generator_parameters.get(&name).and_then(Value::as_str).unwrap_or("").to_owned();
                                            let field_identity = identity.clone();
                                            let field_name = name.clone();
                                            let edit = props.actions.on_edit;
                                            let nets = projection.nets.clone();
                                            let on_change = move |event: FormEvent| {
                                                let value = event.value();
                                                edit.call(PartInputEditRequest {
                                                    identity: field_identity.clone(),
                                                    intent: PartInputIntent::GeneratorParameter {
                                                        name: field_name.clone(),
                                                        value: (!value.is_empty()).then_some(Value::String(value)),
                                                    },
                                                });
                                            };
                                            rsx! {
                                                label { class: "m1-pcb-part-input-field", key: "net-{name}",
                                                    span { "{name}" }
                                                    select {
                                                        aria_label: "Ergogen net {name}",
                                                        value: current,
                                                        disabled: read_only,
                                                        onchange: on_change,
                                                        option { value: "", "Default" }
                                                        for net in nets {
                                                            option { key: "{net.id}", value: "{net.name}", "{net.name}" }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            let current = projection.generator_parameters.get(&name)
                                                .and_then(Value::as_object).cloned().unwrap_or_default();
                                            let render_axis = |axis: &'static str| {
                                                let number = current.get(axis).and_then(Value::as_f64)
                                                    .map(|value| value.to_string()).unwrap_or_default();
                                                let field_identity = identity.clone();
                                                let field_name = name.clone();
                                                let field_axis = axis.to_owned();
                                                let edit = props.actions.on_edit;
                                                let preserved = current.clone();
                                                let on_change = move |event: FormEvent| {
                                                    let raw = event.value();
                                                    let mut next = preserved.clone();
                                                    if raw.trim().is_empty() {
                                                        next.remove(&field_axis);
                                                    } else {
                                                        let Ok(coordinate) = raw.parse::<f64>() else { return; };
                                                        if !coordinate.is_finite() { return; }
                                                        next.insert(field_axis.clone(), Value::from(coordinate));
                                                    }
                                                    edit.call(PartInputEditRequest {
                                                        identity: field_identity.clone(),
                                                        intent: PartInputIntent::GeneratorParameter {
                                                            name: field_name.clone(),
                                                            value: (!next.is_empty()).then_some(Value::Object(next)),
                                                        },
                                                    });
                                                };
                                                rsx! {
                                                    label { class: "m1-pcb-part-input-axis", key: "{axis}",
                                                        span { "{name} {axis_upper(axis)}" }
                                                        input { type: "number", step: "any", aria_label: "Ergogen anchor {name} {axis_upper(axis)}", value: number, disabled: read_only, oninput: on_change }
                                                    }
                                                }
                                            };
                                            rsx! {
                                                div { class: "m1-pcb-part-input-anchor", key: "anchor-{name}",
                                                    span { "{name}" }
                                                    {render_axis("x")}
                                                    {render_axis("y")}
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                Some(Err(error)) => rsx! { p { role: "alert", "Ergogen settings could not be loaded: {error}" } },
                None if generator_source.is_some() => rsx! { p { role: "status", "Loading Ergogen settings…" } },
                None => rsx! {},
            }
            if let Some(feedback) = matching_feedback {
                match feedback.state {
                    PartInputFeedbackState::Preparing => rsx! { p { role: "status", "Preparing PCB setting…" } },
                    PartInputFeedbackState::Pending => rsx! { p { role: "status", "Saving PCB setting…" } },
                    PartInputFeedbackState::Saved => rsx! { p { role: "status", "PCB setting saved." } },
                    PartInputFeedbackState::Failed(message) => rsx! { p { role: "alert", "{message}" } },
                }
            }
        }
    }
}

fn scan_mode_text(mode: PressScanMode) -> &'static str {
    match mode {
        PressScanMode::Matrix => "matrix",
        PressScanMode::Direct => "direct",
        PressScanMode::Unassigned => "unassigned",
    }
}

fn axis_upper(axis: &str) -> &'static str {
    if axis == "x" { "X" } else { "Y" }
}

fn edit_identity(
    source: &PcbWiringSource,
    projection: &PartInputProjection,
) -> Option<PartInputIdentity> {
    let generator_source = projection
        .definition
        .generator
        .as_ref()
        .map(|generator| generator.source.clone());
    Some(PartInputIdentity {
        ui_scope: source.ui_scope.clone(),
        board_id: source.identity.scope.board_id.clone(),
        part_id: projection.part.id.clone(),
        definition_id: projection.definition.id.clone(),
        generator_source,
        token: source.identity.token,
        revision: source.identity.revision,
        executor_epoch: source.identity.executor_epoch,
        scope_generation: source.scope_generation,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::{model::Board, model::Matrix};
    use serde_json::json;
    use wasm_bindgen_test::wasm_bindgen_test;

    fn project_fixture() -> ProjectDoc {
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
            pitch: boardstudio_core::model::Vec2 { x: 19.0, y: 19.0 },
            origin: boardstudio_core::model::Vec2 { x: 0.0, y: 0.0 },
            definition_id: "encoder".into(),
            part_ids: vec!["matrix/main/r0c0".into()],
            board_id: Some("left".into()),
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_offsets: vec![],
            column_offsets: vec![],
            column_staggers: vec![],
            column_splays: vec![],
            column_origins: vec![],
            cells: vec![],
        });
        document
    }

    pub(super) fn projection_for_validation() -> PartInputProjection {
        project(&project_fixture(), "left", "matrix/main/r0c0").unwrap()
    }

    #[test]
    #[wasm_bindgen_test]
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
    #[wasm_bindgen_test]
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

    #[wasm_bindgen_test]
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
    #[wasm_bindgen_test]
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
}
