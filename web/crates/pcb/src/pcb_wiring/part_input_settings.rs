use super::PcbWiringSource;
use super::part_input_projection::{PartInputProjection, binding_schema, project};
use crate::pcb_controls::{PcbChoice, PcbInput, PcbSection};
use boardstudio_model::model::PressScanMode;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::input::InputType;
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
        section { class: "flex flex-1 flex-col min-w-0 min-h-0 overflow-auto overscroll-contain p-4 text-foreground [&_h2]:mt-0 [&_h2]:mb-2 [&_h2]:text-lg [&_h3]:mt-[18px] [&_h3]:mb-2 [&_h3]:text-xs [&_h3]:text-muted-foreground [&_p]:my-2 [&_p]:text-xs [&_p]:leading-normal [&_p]:text-muted-foreground gap-1",
            if let Some(scan_mode) = scan_mode {
                PcbSection { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs", default_open: true, label: "Press input",
                    PcbChoice {
                        label: "Scan mode",
                        class: "grid grid-cols-[minmax(0,1fr)_minmax(120px,1.15fr)] items-center gap-2.5 py-2 text-foreground text-xs",
                        aria_label: "Press scan mode",
                        value: (scan_mode_text(scan_mode)).to_string(),
                        disabled: read_only,
                        on_change: scan_change,
                        options: vec![("matrix".into(), "Matrix key".into(), !matrix_enabled), ("direct".into(), "Direct GPIO".into(), false), ("unassigned".into(), "Unassigned".into(), false)],
                    }
                    p { class: "pb-2", "Rotation uses separate GPIOs. Apply the board wiring plan after changing the press connection." }
                }
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
                            PcbSection { class: "mt-2.5 pt-0.5 border-t border-border [&_ul]:grid [&_ul]:gap-[7px] [&_ul]:my-2 [&_ul]:p-0 [&_ul]:list-none [&_li]:flex [&_li]:justify-between [&_li]:gap-2 [&_li]:text-xs", default_open: true, label: "Generator bindings",
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
                                                PcbChoice { class: "grid grid-cols-[minmax(0,1fr)_minmax(120px,1.15fr)] items-center gap-2.5 py-2 text-foreground text-xs", key: "net-{name}", label: name.clone(),
                                                    aria_label: format!("Generator net {name}"), value: current, disabled: read_only, searchable: true, on_change,
                                                    options: std::iter::once((String::new(), "Default".into(), false)).chain(nets.into_iter().map(|net| (net.name.clone(), net.name, false))).collect(),
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
                                                let schema = schema.clone();
                                                let on_change = move |event: FormEvent| {
                                                    let raw = event.value();
                                                    let value = if raw.trim().is_empty() { None } else {
                                                        let Ok(coordinate) = raw.parse::<f64>() else { return; };
                                                        if !coordinate.is_finite() { return; }
                                                        Some(coordinate)
                                                    };
                                                    edit.call(PartInputEditRequest { identity: field_identity.clone(), schema: schema.clone(), intent: PartInputIntent::GeneratorAnchor { name: field_name.clone(), axis: field_axis.clone(), value } });
                                                };
                                                rsx! {
                                                    PcbInput { key: "{axis}", label: format!("{name} {}", axis_upper(axis)),
                                                        r#type: InputType::Number, step: Some("any".into()), aria_label: format!("Generator anchor {name} {}", axis_upper(axis)), value: number, disabled: read_only, oninput: on_change }
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
                            }
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
        executor_epoch: source.identity.executor_epoch,
        scope_generation: source.scope_generation,
    })
}

#[cfg(test)]
mod tests {
    use super::super::part_input_projection::project_fixture;
    use super::*;
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use wasm_bindgen_test::wasm_bindgen_test;

    #[derive(Clone)]
    struct Capture(std::rc::Rc<std::cell::RefCell<Option<PartInputEditRequest>>>);

    fn mounted_scan_mode() -> Element {
        let capture = use_context::<Capture>();
        let scope = boardstudio_application::Scope {
            session_epoch: boardstudio_application::SessionEpoch(1),
            document_id: "project".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let source = PcbWiringSource {
            identity: crate::firmware_position_projection::FirmwarePlanIdentity {
                scope: scope.clone(),
                token: boardstudio_application::SnapshotToken(1),
                revision: 0,
                executor_epoch: 0,
            },
            ui_scope: scope,
            scope_generation: 0,
            document: std::sync::Arc::new(project_fixture()),
            active_part_id: Some("matrix/main/r0c0".into()),
        };
        let actions = PartInputActions {
            failure: None,
            editable: true,
            on_edit: EventHandler::new(move |request| *capture.0.borrow_mut() = Some(request)),
        };
        rsx! { PartInputInspector { source, actions, board_details_and_connections: rsx!{} } }
    }

    #[wasm_bindgen_test]
    async fn scan_mode_list_is_named_and_selects_by_keyboard() {
        use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let capture = std::rc::Rc::new(std::cell::RefCell::new(None));
        let dom = VirtualDom::new(mounted_scan_mode);
        dom.provide_root_context(Capture(capture.clone()));
        let _mounted = MountedFixture::new(root.clone(), dom);
        wait_until("scan picker mounts", || {
            by_role_name(&root, "button", "Press scan mode").is_some()
        })
        .await;
        let picker = by_role_name(&root, "button", "Press scan mode").unwrap();
        picker.focus().unwrap();
        picker.click();
        wait_until("scan options open", || {
            by_role_name(&root, "listbox", "Press scan mode").is_some()
        })
        .await;
        let list = by_role_name(&root, "listbox", "Press scan mode").unwrap();
        assert_eq!(
            picker.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("Home");
        event.set_bubbles(true);
        event.set_cancelable(true);
        list.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
        )
        .unwrap();
        wait_until("first scan mode is focused", || {
            by_role_name(&root, "option", "Matrix key")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("ArrowDown");
        event.set_bubbles(true);
        event.set_cancelable(true);
        list.dispatch_event(
            &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event).unwrap(),
        )
        .unwrap();
        wait_until("direct mode is focused", || {
            by_role_name(&root, "option", "Direct GPIO")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        let direct = by_role_name(&root, "option", "Direct GPIO").unwrap();
        let event = web_sys::KeyboardEventInit::new();
        event.set_key("Enter");
        event.set_bubbles(true);
        event.set_cancelable(true);
        direct
            .dispatch_event(
                &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                    .unwrap(),
            )
            .unwrap();
        wait_until("direct mode request emitted", || {
            capture
                .borrow()
                .as_ref()
                .is_some_and(|request: &PartInputEditRequest| {
                    request.intent == PartInputIntent::ScanMode(PressScanMode::Direct)
                })
        })
        .await;
        wait_until("focus returns to scan picker", || {
            document
                .active_element()
                .is_some_and(|active| active == *picker)
        })
        .await;
        root.remove();
    }
}
