//! The selected part's declarative wiring section: press input, board
//! details, net connections and generator bindings, committed synchronously
//! through the inspector's plain edit handlers.
use super::part_connections::{board_nets, net_on_board};
use super::part_input_projection::{binding_schema, project, validate_parameter_value};
use super::{PartNetEditAction, PartNetEditRequest, PcbWiringSource, WIRING_CARD_CLS};
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_model::model::{EditOperation, PressScanMode};
use boardstudio_web_ui_model::editor::EditorContext;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TOUCH_CLS, TextField,
};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::input::InputType;
use boardstudio_web_ui_shared::ui::spinner::Spinner;
use dioxus::prelude::*;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartInputIdentity {
    pub ui_scope: Scope,
    pub board_id: String,
    pub part_id: String,
    pub definition_id: String,
    pub generator_source: Option<String>,
    pub token: SnapshotToken,
    pub revision: u64,
    pub scope_generation: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PartInputIntent {
    ScanMode(PressScanMode),
    GeneratorParameter {
        name: String,
        value: Option<Value>,
    },
    GeneratorAnchor {
        name: String,
        axis: String,
        value: Option<f64>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PartInputEditRequest {
    pub identity: PartInputIdentity,
    pub intent: PartInputIntent,
    pub(crate) schema: Rc<BTreeMap<String, Value>>,
}

#[derive(Props, Clone, PartialEq)]
pub struct PartSectionProps {
    pub source: PcbWiringSource,
    pub editable: bool,
}

#[component]
pub fn PartSection(props: PartSectionProps) -> Element {
    let editor = use_context::<EditorContext>();
    let mut new_net_name = use_signal(String::new);
    let mut net_failure = use_signal(|| None::<(String, String)>);
    let mut input_failure = use_signal(|| None::<(String, String)>);
    let mut create_pending = use_signal(|| false);
    let on_net_edit = EventHandler::new(move |request: PartNetEditRequest| {
        let runtime = editor.runtime();
        if !super::guard_current(editor, &runtime) {
            return;
        }
        let live_part = {
            let runtime = runtime.clone();
            move |part_id: &str| {
                super::mode::pcb_source(&runtime, (editor.scope_generation)())
                    .is_some_and(|(_, live)| live.active_part_id.as_deref() == Some(part_id))
            }
        };
        if !live_part(&request.part_id) {
            return;
        }
        let create = matches!(&request.action, PartNetEditAction::CreateNet { .. });
        let key = request.part_id.clone();
        let seed = runtime.operation().0;
        let Some(generator_source) = request.generator_source.clone() else {
            net_failure.set(
                runtime
                    .commit(
                        "pcb-part-net",
                        super::part_net_resolver(request, false, seed),
                    )
                    .err()
                    .map(|message| (key, message)),
            );
            return;
        };
        if create {
            if create_pending() {
                return;
            }
            create_pending.set(true);
        }
        // Classifying a generator source is genuinely async I/O; the commit
        // after it revalidates the part through its resolver.
        spawn(async move {
            let classified =
                crate::presentation::parts::is_generator_source(generator_source).await;
            if create {
                create_pending.set(false);
            }
            if !super::guard_current(editor, &runtime) || !live_part(&request.part_id) {
                return;
            }
            net_failure.set(
                classified
                    .and_then(|is_generator_source| {
                        runtime.commit(
                            "pcb-part-net",
                            super::part_net_resolver(request, is_generator_source, seed),
                        )
                    })
                    .err()
                    .map(|message| (key, message)),
            );
        });
    });
    let on_input_edit = EventHandler::new(move |request: PartInputEditRequest| {
        let runtime = editor.runtime();
        if !super::guard_current(editor, &runtime) {
            return;
        }
        let identity = &request.identity;
        let Some((_, live)) = super::mode::pcb_source(&runtime, (editor.scope_generation)()) else {
            return;
        };
        if identity.scope_generation != (editor.scope_generation)()
            || live.ui_scope != identity.ui_scope
            || live.identity.token != identity.token
            || live.identity.revision != identity.revision
            || runtime.model().selected_part_ids.as_slice() != [identity.part_id.as_str()]
        {
            return;
        }
        let key = identity.part_id.clone();
        input_failure.set(
            runtime
                .commit("pcb-part-settings", input_resolver(request))
                .err()
                .map(|message| (key, message)),
        );
    });
    let for_part = |failure: Option<(String, String)>| {
        failure
            .filter(|(id, _)| Some(id.as_str()) == props.source.active_part_id.as_deref())
            .map(|(_, message)| message)
    };
    let source = &props.source;
    let document = &source.document;
    let board_id = source.identity.scope.board_id.clone();
    // Hooks run before the unavailable-selection returns below.
    let generator_source = source
        .active_part_id
        .as_deref()
        .and_then(|part_id| document.parts.iter().find(|part| part.id == part_id))
        .and_then(|part| {
            document
                .definitions
                .iter()
                .find(|definition| definition.id == part.definition_id)
        })
        .and_then(|definition| definition.generator.as_ref())
        .map(|generator| generator.source.clone());
    let ergogen_source = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        match source {
            Some(source) => crate::presentation::parts::is_generator_source(source).await,
            None => Ok(false),
        }
    }));
    let Some(board) = document.boards.iter().find(|board| board.id == board_id) else {
        return rsx! {};
    };
    let Some(part_id) = source.active_part_id.as_deref() else {
        return rsx! {};
    };
    let Some(part) = document.parts.iter().find(|part| part.id == part_id) else {
        return rsx! {};
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return rsx! {};
    };
    let standalone_pads_allowed = matches!(ergogen_source.read().as_ref(), Some(Ok(false)));
    let nets = board_nets(document, board);
    let assigned_pins = nets
        .iter()
        .map(|net| {
            net.pins
                .iter()
                .filter(|pin| board.part_ids.contains(&pin.part_id))
                .count()
        })
        .sum::<usize>();
    let unique_net = |pad_ids: &[String]| {
        let assigned = nets
            .iter()
            .filter(|net| {
                net.pins
                    .iter()
                    .any(|pin| pin.part_id == part_id && pad_ids.contains(&pin.pad_id))
            })
            .map(|net| net.id.clone())
            .collect::<BTreeSet<_>>();
        (assigned.len() == 1)
            .then(|| assigned.into_iter().next())
            .flatten()
    };
    let connections: Vec<(String, Vec<String>, Option<String>, bool)> = definition
        .terminals
        .iter()
        .map(|(name, pad_ids)| {
            (
                format!("{name} terminal"),
                pad_ids.clone(),
                unique_net(pad_ids),
                false,
            )
        })
        .chain(
            definition
                .pads
                .iter()
                .filter(|pad| {
                    pad.plated != Some(false)
                        && !pad.number.is_empty()
                        && !definition
                            .terminals
                            .values()
                            .any(|ids| ids.contains(&pad.id))
                })
                .map(|pad| {
                    (
                        pad.number.clone(),
                        vec![pad.id.clone()],
                        unique_net(std::slice::from_ref(&pad.id)),
                        true,
                    )
                }),
        )
        .collect();
    let net_request = {
        let board_id = board_id.clone();
        let part_id = part_id.to_owned();
        let definition_id = definition.id.clone();
        let generator_source = generator_source.clone();
        move |action| PartNetEditRequest {
            board_id: board_id.clone(),
            part_id: part_id.clone(),
            definition_id: definition_id.clone(),
            generator_source: generator_source.clone(),
            action,
        }
    };
    let add_request = net_request.clone();
    let add_net = move |event: FormEvent| {
        event.prevent_default();
        let name = new_net_name().trim().to_owned();
        if !name.is_empty() {
            on_net_edit.call(add_request(PartNetEditAction::CreateNet { name }));
            new_net_name.set(String::new());
        }
    };
    let editable = props.editable;
    rsx! {
        section { class: "{super::WIRING_SECTION_CLS}",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{board.name} / PCB" }
            h2 { "{part.reference} · {definition.name}" }
            PartInputSection { source: props.source.clone(), editable, failure: for_part(input_failure()), on_edit: on_input_edit }
            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: false, title: "Board details",
                ul {
                    li { span { "Board thickness" } strong { "{board.thickness:.2} mm" } }
                    li { span { "Placed parts" } strong { "{board.part_ids.len()}" } }
                    li { span { "Net assignments" } strong { "{assigned_pins}" } }
                }
            } }
            div { class: "{WIRING_CARD_CLS}",
                h3 { "Connections" }
                if connections.is_empty() {
                    p { class: "text-muted-foreground", "This part has no assignable terminals or pads." }
                } else {
                    for (label, pad_ids, selected, standalone) in connections.iter().filter(|row| !row.3 || standalone_pads_allowed) {
                        {
                            let request = net_request.clone();
                            let pad_ids = pad_ids.clone();
                            let options = std::iter::once((String::new(), "Unmapped".into()))
                                .chain(nets.iter().map(|net| (net.id.clone(), net.name.clone()))).collect::<Vec<_>>();
                            let _ = standalone;
                            rsx! {
                                SelectField::<String> { key: "{label}",
                                    label: label.clone(), aria_label: format!("Net for {label}"),
                                    value: selected.clone().unwrap_or_default(), disabled: !editable,
                                    on_commit: move |value: String| {
                                        on_net_edit.call(request(PartNetEditAction::AssignPads {
                                            pad_ids: pad_ids.clone(),
                                            net_id: (!value.is_empty()).then_some(value),
                                        }));
                                    },
                                    options,
                                }
                            }
                        }
                    }
                }
            }
            form { class: "grid grid-cols-[minmax(0,1fr)_auto] items-baseline gap-x-2.5 gap-y-[3px] py-2 border-b border-border text-xs [&_strong]:[overflow-wrap:anywhere] [&_small]:col-span-full [&_small]:text-muted-foreground !items-center !gap-2 mt-2.5", onsubmit: add_net,
                TextField { id: "pcb-new-net".to_owned(), label: "New net name", r#type: InputType::Text,
                    placeholder: Some("New net name".into()), value: new_net_name(), disabled: !editable,
                    oninput: move |event: FormEvent| new_net_name.set(event.value()),
                }
                Button { button_type: "submit", variant: ButtonVariant::Secondary, class: TOUCH_CLS,
                    disabled: !editable || new_net_name().trim().is_empty() || create_pending(), "Add net" }
            }
            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: false, title: format!("Electrical nets ({})", nets.len()),
                ul {
                    for net in &nets {
                        li { key: "{net.id}", span { "{net.name}" }
                            small { "{net.pins.iter().filter(|pin| board.part_ids.contains(&pin.part_id)).count()} pins" } }
                    }
                    if nets.is_empty() { li { "No nets are assigned to this board." } }
                }
            } }
            if let Some(failure) = for_part(net_failure()) {
                Alert { variant: AlertVariant::Destructive, "{failure}" }
            }
        }
    }
}

/// Press-scan mode and generator binding fields for parts that expose them.
#[component]
fn PartInputSection(
    source: PcbWiringSource,
    editable: bool,
    failure: Option<String>,
    on_edit: EventHandler<PartInputEditRequest>,
) -> Element {
    let board_id = source.identity.scope.board_id.clone();
    let projection = source
        .active_part_id
        .as_deref()
        .and_then(|part_id| project(&source.document, &board_id, part_id));
    // The schema resource is unconditional so hook order survives parts
    // without an input projection.
    let generator_source = projection.as_ref().and_then(|projection| {
        projection
            .definition
            .generator
            .as_ref()
            .map(|generator| generator.source.clone())
    });
    let schema = use_resource(use_reactive((&generator_source,), |(source,)| async move {
        match source {
            Some(source) => crate::presentation::parts::generator_parameter_schema(source).await,
            None => Ok(BTreeMap::new()),
        }
    }));
    let Some(projection) = projection else {
        return rsx! {};
    };
    let identity = PartInputIdentity {
        ui_scope: source.ui_scope.clone(),
        board_id,
        part_id: projection.part.id.clone(),
        definition_id: projection.definition.id.clone(),
        generator_source: generator_source.clone(),
        token: source.identity.token,
        revision: source.identity.revision,
        scope_generation: source.scope_generation,
    };
    let schema_state = schema.read().clone();
    let read_only = !editable || projection.part.locked == Some(true);
    let matrix_enabled = projection.matrix_mode_enabled;
    let scan_identity = identity.clone();
    let scan_change = move |value: String| {
        let mode = match value.as_str() {
            "matrix" if matrix_enabled => PressScanMode::Matrix,
            "direct" => PressScanMode::Direct,
            "unassigned" => PressScanMode::Unassigned,
            _ => return,
        };
        on_edit.call(PartInputEditRequest {
            identity: scan_identity.clone(),
            intent: PartInputIntent::ScanMode(mode),
            schema: Rc::default(),
        });
    };
    rsx! {
        if let Some(scan_mode) = projection.press_mode {
            div { class: "{WIRING_CARD_CLS}", InspectorSection { default_open: true, title: "Press input",
                SelectField::<String> {
                    label: "Scan mode", aria_label: "Press scan mode",
                    value: scan_mode_text(scan_mode).to_owned(), disabled: read_only,
                    on_commit: scan_change, placeholder: "Matrix key unavailable",
                    options: matrix_enabled.then(|| ("matrix".into(), "Matrix key".into())).into_iter()
                        .chain([("direct".into(), "Direct GPIO".into()), ("unassigned".into(), "Unassigned".into())]).collect(),
                }
                p { class: "pb-2", "Rotation uses separate GPIOs. Apply the board wiring plan after changing the press connection." }
            } }
        }
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
                                        let nets = projection.nets.clone();
                                        let schema = schema.clone();
                                        rsx! {
                                            SelectField::<String> { key: "net-{name}", label: name.clone(),
                                                aria_label: format!("Generator net {name}"), value: current, disabled: read_only,
                                                on_commit: move |value: String| on_edit.call(PartInputEditRequest {
                                                    identity: field_identity.clone(), schema: schema.clone(),
                                                    intent: PartInputIntent::GeneratorParameter { name: field_name.clone(), value: (!value.is_empty()).then_some(Value::String(value)) },
                                                }),
                                                options: std::iter::once((String::new(), "Default".into())).chain(nets.into_iter().map(|net| (net.name.clone(), net.name))).collect(),
                                            }
                                        }
                                    }
                                } else {
                                    {
                                        let current = projection.generator_parameters.get(&name).and_then(Value::as_object).cloned().unwrap_or_default();
                                        let render_axis = |axis: &'static str| {
                                            let number = current.get(axis).and_then(Value::as_f64);
                                            let field_identity = identity.clone();
                                            let field_name = name.clone();
                                            let schema = schema.clone();
                                            let clear_identity = field_identity.clone();
                                            let clear_name = field_name.clone();
                                            let clear_schema = schema.clone();
                                            rsx! {
                                                NumericField { key: "{axis}", label: format!("{name} {}", axis_upper(axis)),
                                                    aria_label: format!("Generator anchor {name} {}", axis_upper(axis)),
                                                    value: number.unwrap_or_default(), mixed: number.is_none(), placeholder: "Unset",
                                                    disabled_reason: read_only.then(|| "Generator settings are read-only".to_owned()),
                                                    on_commit: move |value| on_edit.call(PartInputEditRequest {
                                                        identity: field_identity.clone(), schema: schema.clone(),
                                                        intent: PartInputIntent::GeneratorAnchor { name: field_name.clone(), axis: axis.to_owned(), value: Some(value) },
                                                    }),
                                                }
                                                if number.is_some() {
                                                    Button { disabled: read_only,
                                                        onclick: move |_| on_edit.call(PartInputEditRequest {
                                                            identity: clear_identity.clone(), schema: clear_schema.clone(),
                                                            intent: PartInputIntent::GeneratorAnchor { name: clear_name.clone(), axis: axis.to_owned(), value: None },
                                                        }), "Clear {axis_upper(axis)}" }
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
        if let Some(failure) = &failure {
            Alert { variant: AlertVariant::Destructive, "{failure}" }
        }
    }
}

/// Switches inherit wiring from their key assembly; this section only shows
/// the named terminals and their inherited nets.
#[component]
pub fn SwitchSection(source: PcbWiringSource, on_edit_board_wiring: EventHandler<()>) -> Element {
    let document = &source.document;
    let Some(board) = document
        .boards
        .iter()
        .find(|board| board.id == source.identity.scope.board_id)
    else {
        return rsx! {};
    };
    let Some(part) = source
        .active_part_id
        .as_deref()
        .and_then(|part_id| document.parts.iter().find(|part| part.id == part_id))
    else {
        return rsx! {};
    };
    let Some(definition) = document
        .definitions
        .iter()
        .find(|definition| definition.id == part.definition_id)
    else {
        return rsx! {};
    };
    rsx! {
        section { class: "{super::WIRING_SECTION_CLS}",
            p { class: "!mt-0 !mb-3 !text-[11px]", "{board.name} / PCB" }
            h2 { "{part.reference} · {definition.name}" }
            p { "Switch wiring is inherited from its key assembly." }
            div { class: "{WIRING_CARD_CLS}",
                h3 { "Named terminals" }
                if definition.terminals.is_empty() {
                    p { class: "text-muted-foreground", "This switch has no named terminals." }
                } else {
                    ul {
                        for (name, pad_ids) in &definition.terminals {
                            li { key: "{name}",
                                strong { "{name}" }
                                span { " → " }
                                span { "{document.nets.iter().find(|net| net_on_board(net, board) && net.pins.iter().any(|pin| pin.part_id == part.id && pad_ids.contains(&pin.pad_id))).map_or(\"Unmapped\", |net| net.name.as_str())}" }
                            }
                        }
                    }
                }
            }
            Button { variant: ButtonVariant::Secondary, class: TOUCH_CLS, onclick: move |_| on_edit_board_wiring.call(()), "Edit board wiring" }
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

pub(super) fn input_resolver(request: PartInputEditRequest) -> EditResolver {
    EditResolver::new("pcb-part-settings", move |accepted: &AcceptedSnapshot| {
        let identity = &request.identity;
        let Some(projection) = project(&accepted.document, &identity.board_id, &identity.part_id)
        else {
            return Resolution::Retire(
                "The part was deleted or no longer supports these controls.".into(),
            );
        };
        if projection.part.locked == Some(true)
            || projection.definition.id != identity.definition_id
            || projection
                .definition
                .generator
                .as_ref()
                .map(|generator| &generator.source)
                != identity.generator_source.as_ref()
        {
            return Resolution::Retire("The part is locked or its definition changed.".into());
        }
        let schema = &request.schema;
        // Rebuild the document with one generator-parameter map edit, or
        // report `Unchanged` when the mutation leaves the map as accepted.
        let with_parameters = |edit: &dyn Fn(&mut BTreeMap<String, Value>)| -> Resolution {
            let mut document = (*accepted.document).clone();
            let Some(part) = document
                .parts
                .iter_mut()
                .find(|part| part.id == identity.part_id)
            else {
                return Resolution::Retire("The part was deleted.".into());
            };
            let mut parameters = part.generator_parameters.clone().unwrap_or_default();
            let before = parameters.clone();
            edit(&mut parameters);
            if parameters == before {
                return Resolution::Unchanged;
            }
            part.generator_parameters = Some(parameters);
            Resolution::submit(
                vec![identity.part_id.clone()],
                EditOperation::ReplaceDocument {
                    document: Box::new(document),
                },
            )
        };
        match &request.intent {
            PartInputIntent::ScanMode(mode) => {
                if projection.press_mode.is_none()
                    || (*mode == PressScanMode::Matrix && !projection.matrix_mode_enabled)
                {
                    return Resolution::Retire(
                        "This part cannot use the selected press scan mode.".into(),
                    );
                }
                if projection.press_mode == Some(*mode) {
                    return Resolution::Unchanged;
                }
                Resolution::submit(
                    vec![identity.part_id.clone()],
                    EditOperation::SetInputScanMode {
                        part_id: identity.part_id.clone(),
                        mode: *mode,
                    },
                )
            }
            PartInputIntent::GeneratorParameter { name, value } => {
                let terminals = projection
                    .definition
                    .terminals
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                let fields = binding_schema(schema, &terminals);
                let Some((_, kind)) = fields.iter().find(|(field, _)| field == name) else {
                    return Resolution::Retire(
                        "This generator parameter is no longer an editable PCB binding.".into(),
                    );
                };
                if let Err(reason) = validate_parameter_value(kind, value.as_ref(), &projection) {
                    return Resolution::Retire(reason);
                }
                with_parameters(&|parameters| match value {
                    Some(value) => {
                        parameters.insert(name.clone(), value.clone());
                    }
                    None => {
                        parameters.remove(name);
                    }
                })
            }
            PartInputIntent::GeneratorAnchor { name, axis, value } => {
                if !matches!(axis.as_str(), "x" | "y")
                    || value.is_some_and(|value| !value.is_finite())
                {
                    return Resolution::Retire("Anchor coordinates must be finite numbers.".into());
                }
                let terminals = projection
                    .definition
                    .terminals
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>();
                if !binding_schema(schema, &terminals)
                    .iter()
                    .any(|(field, kind)| field == name && kind == "anchor")
                {
                    return Resolution::Retire(
                        "This generator anchor is no longer editable.".into(),
                    );
                }
                with_parameters(&|parameters| {
                    let mut anchor = parameters
                        .get(name)
                        .and_then(Value::as_object)
                        .cloned()
                        .unwrap_or_default();
                    match value {
                        Some(value) => {
                            anchor.insert(axis.clone(), Value::from(*value));
                        }
                        None => {
                            anchor.remove(axis);
                        }
                    }
                    if anchor.is_empty() {
                        parameters.remove(name);
                    } else {
                        parameters.insert(name.clone(), Value::Object(anchor));
                    }
                })
            }
        }
    })
}
