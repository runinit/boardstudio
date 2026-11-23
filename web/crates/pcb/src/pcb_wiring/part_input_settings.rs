use super::part_input_projection::{PartInputProjection, binding_schema, project};
use super::{PcbWiringSource, WIRING_CARD_CLS, WIRING_SECTION_CLS};
use boardstudio_model::model::PressScanMode;
use boardstudio_web_ui_shared::inspector_fields::NumericField;
use boardstudio_web_ui_shared::inspector_fields::{InspectorSection, SelectField};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::Button;
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use dioxus::prelude::*;
use serde_json::Value;
use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

mod owner;
pub use owner::{
    PartInputActions, PartInputEditRequest, PartInputIdentity, PartInputIntent,
    use_part_input_edits,
};

#[derive(Props, Clone, PartialEq)]
pub struct PartInputInspectorProps {
    pub source: PcbWiringSource,
    pub actions: PartInputActions,
    pub board_details_and_connections: Element,
}

#[component]
pub fn PartInputInspector(props: PartInputInspectorProps) -> Element {
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
            Some(source) => crate::presentation::parts::generator_parameter_schema(source).await,
            None => Ok(BTreeMap::new()),
        }
    }));
    let schema_state = schema.read().clone();
    let read_only = !props.actions.editable || projection.part.locked == Some(true);
    let on_edit = props.actions.on_edit;
    let scan_mode = projection.press_mode;
    let matrix_enabled = projection.matrix_mode_enabled;
    let scan_identity = identity.clone();
    let scan_change = move |value: String| {
        let mode = match value.as_str() {
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
            schema: Rc::default(),
        });
    };

    rsx! {
        section { class: "{WIRING_SECTION_CLS} gap-1",
            if let Some(scan_mode) = scan_mode {
                div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: true, title: "Press input",
                    SelectField::<String> {
                        label: "Scan mode",

                        aria_label: "Press scan mode",
                        value: (scan_mode_text(scan_mode)).to_string(),
                        disabled: read_only,
                        on_commit: scan_change,
                        placeholder: "Matrix key unavailable",
                        options: matrix_enabled.then(|| ("matrix".into(), "Matrix key".into())).into_iter()
                            .chain([("direct".into(), "Direct GPIO".into()), ("unassigned".into(), "Unassigned".into())]).collect(),
                    }
                    p { class: "pb-2", "Rotation uses separate GPIOs. Apply the board wiring plan after changing the press connection." }
                } }
            }
            {props.board_details_and_connections}
            match schema_state {
                Some(Ok(schema)) => {
                    let terminals = projection.definition.terminals.keys().cloned().collect::<BTreeSet<_>>();
                    let fields = binding_schema(&schema, &terminals);
                    let schema = Rc::new(schema);
                    if fields.is_empty() {
                        rsx! {}
                    } else {
                        rsx! {
                            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: true, title: "Generator bindings",
                                for (name, kind) in fields {
                                    if kind == "net" {
                                        {
                                            let current = projection.generator_parameters.get(&name).and_then(Value::as_str).unwrap_or("").to_owned();
                                            let field_identity = identity.clone();
                                            let field_name = name.clone();
                                            let edit = props.actions.on_edit;
                                            let nets = projection.nets.clone();
                                            let schema = schema.clone();
                                            let on_change = move |value: String| {
                                                edit.call(PartInputEditRequest {
                                                    identity: field_identity.clone(),
                                                    schema: schema.clone(),
                                                    intent: PartInputIntent::GeneratorParameter {
                                                        name: field_name.clone(),
                                                        value: (!value.is_empty()).then_some(Value::String(value)),
                                                    },
                                                });
                                            };
                                            rsx! {
                                                SelectField::<String> { key: "net-{name}", label: name.clone(),
                                                    aria_label: format!("Generator net {name}"), value: current, disabled: read_only, on_commit: on_change,
                                                    options: std::iter::once((String::new(), "Default".into())).chain(nets.into_iter().map(|net| (net.name.clone(), net.name))).collect(),
                                                }
                                            }
                                        }
                                    } else {
                                        {
                                            let current = projection.generator_parameters.get(&name)
                                                .and_then(Value::as_object).cloned().unwrap_or_default();
                                            let render_axis = |axis: &'static str| {
                                                let number = current.get(axis).and_then(Value::as_f64);
                                                let field_identity = identity.clone();
                                                let field_name = name.clone();
                                                let field_axis = axis.to_owned();
                                                let edit = props.actions.on_edit;
                                                let schema = schema.clone();
                                                let on_change = EventHandler::new(move |value: Option<f64>| {
                                                    edit.call(PartInputEditRequest { identity: field_identity.clone(), schema: schema.clone(), intent: PartInputIntent::GeneratorAnchor { name: field_name.clone(), axis: field_axis.clone(), value } });
                                                });
                                                rsx! {
                                                    NumericField { key: "{axis}", label: format!("{name} {}", axis_upper(axis)),
                                                        aria_label: format!("Generator anchor {name} {}", axis_upper(axis)),
                                                        value: number.unwrap_or_default(), mixed: number.is_none(), placeholder: "Unset",
                                                        disabled_reason: read_only.then(|| "Generator settings are read-only".to_owned()),
                                                        on_commit: move |value| on_change.call(Some(value)),
                                                    }
                                                    if number.is_some() {
                                                        Button { disabled: read_only, onclick: move |_| on_change.call(None), "Clear {axis_upper(axis)}" }
                                                    }
                                                }
                                            };
                                            rsx! {
                                                div { class: "grid grid-cols-[minmax(70px,0.7fr)_repeat(2,minmax(70px,1fr))] items-end gap-2 py-2 text-foreground text-xs", key: "anchor-{name}",
                                                    span { "{name}" }
                                                    {render_axis("x")}
                                                    {render_axis("y")}
                                                }
                                            }
                                        }
                                    }
                                }
                            } }
                        }
                    }
                }
                Some(Err(error)) => rsx! { Alert { variant: AlertVariant::Destructive, "Generator settings could not be loaded: {error}" } },
                None if generator_source.is_some() => rsx! { p { role: "status", Spinner {} "Loading generator settings…" } },
                None => rsx! {},
            }
            if let Some(failure) = props.actions.failure.clone() {
                Alert { variant: AlertVariant::Destructive, "{failure.message}" }
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
        scope_generation: source.scope_generation,
    })
}
