//! Private encoder binding rows, reusing the shared KeyBinding editor.
use super::binding_controller::EncoderEditorProjection;
use super::binding_editor::{
    BindingEditFeedback, BindingEditRequest, BindingEditor, BindingTarget,
};
use boardstudio_core::model::EncoderDirection;
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Props, Clone, PartialEq)]
pub struct EncoderEditorProps {
    pub projection: Rc<EncoderEditorProjection>,
    pub editor_instance_id: u64,
    pub request_sequence: Signal<u64>,
    pub enabled: bool,
    pub feedback: Option<BindingEditFeedback>,
    pub on_change: EventHandler<BindingEditRequest>,
}

#[component]
pub fn EncoderEditor(props: EncoderEditorProps) -> Element {
    let projection = props.projection;
    let rows = projection.rows.clone();
    let input_identity = projection.input_identity.clone();
    let active_layer_id = projection.effective_layer_id.clone();
    let layers = projection.layers.clone();
    let macros = projection.macros.clone();
    let editor_instance_id = props.editor_instance_id;
    let request_sequence = props.request_sequence;
    let enabled = props.enabled;
    let feedback = props.feedback.clone();
    let on_change = props.on_change;
    let row_count = rows.len();

    rsx! {
        section { class: "m1-keymap-encoder-editor", "aria-label": "Encoders",
            header { class: "m1-keymap-encoder-heading",
                h3 { "Encoders" }
                span { "{row_count}" }
            }
            if rows.is_empty() {
                p { class: "m1-keymap-empty", role: "status",
                    "Add a rotary encoder in Layout. Configure its GPIOs in PCB, then assign rotation here."
                }
            } else {
                for row in rows.iter() {
                    {
                        let encoder_id = row.id.to_string();
                        let label = row.label.to_string();
                        let clockwise_target = BindingTarget::EncoderRotation {
                            encoder_id: encoder_id.clone(),
                            direction: EncoderDirection::Clockwise,
                        };
                        let counterclockwise_target = BindingTarget::EncoderRotation {
                            encoder_id: encoder_id.clone(),
                            direction: EncoderDirection::Counterclockwise,
                        };
                        let row_key = encoder_id.clone();
                        let clockwise_key = clockwise_target.stable_key();
                        let counterclockwise_key = counterclockwise_target.stable_key();
                        let clockwise_component_key = format!("{editor_instance_id}-{clockwise_key}");
                        let counterclockwise_component_key = format!("{editor_instance_id}-{counterclockwise_key}");
                        let clockwise_label = format!("{label} clockwise");
                        let counterclockwise_label = format!("{label} counterclockwise");
                        let clockwise = row.clockwise.clone();
                        let counterclockwise = row.counterclockwise.clone();
                        let push = match (&row.push_key_id, &row.push_binding) {
                            (Some(key_id), Some(binding)) => Some((key_id.to_string(), binding.clone())),
                            _ => None,
                        };
                        rsx! {
                            div { key: "{row_key}", class: "m1-keymap-encoder-row",
                                h4 { "{label}" }
                                EncoderSection { key: "{clockwise_key}", label: clockwise_label.clone(), title: "Clockwise", default_open: true,
                                    for component_key in [clockwise_component_key] {
                                        { rsx! { BindingEditor {
                                            key: "{component_key}",
                                            scope: input_identity.scope.clone(),
                                            admission_token: input_identity.token,
                                            admission_revision: input_identity.revision,
                                            input_identity: Some(input_identity.clone()),
                                            active_layer_id: active_layer_id.clone(),
                                            target: clockwise_target.clone(),
                                            key_label: clockwise_label.clone(),
                                            editor_instance_id,
                                            request_sequence,
                                            value: clockwise.clone(),
                                            layers: layers.clone(),
                                            macros: macros.clone(),
                                            enabled,
                                            feedback: feedback.clone(),
                                            on_change,
                                        } } }
                                    }
                                }
                                EncoderSection { key: "{counterclockwise_key}", label: counterclockwise_label.clone(), title: "Counterclockwise", default_open: true,
                                    for component_key in [counterclockwise_component_key] {
                                        { rsx! { BindingEditor {
                                            key: "{component_key}",
                                            scope: input_identity.scope.clone(),
                                            admission_token: input_identity.token,
                                            admission_revision: input_identity.revision,
                                            input_identity: Some(input_identity.clone()),
                                            active_layer_id: active_layer_id.clone(),
                                            target: counterclockwise_target.clone(),
                                            key_label: counterclockwise_label.clone(),
                                            editor_instance_id,
                                            request_sequence,
                                            value: counterclockwise.clone(),
                                            layers: layers.clone(),
                                            macros: macros.clone(),
                                            enabled,
                                            feedback: feedback.clone(),
                                            on_change,
                                        } } }
                                    }
                                }
                                if let Some((key_id, binding)) = push {
                                    {
                                        let target = BindingTarget::EncoderPush {
                                            encoder_id: encoder_id.clone(),
                                            key_id: key_id.clone(),
                                        };
                                        let target_key = target.stable_key();
                                        let component_key = format!("{editor_instance_id}-{target_key}");
                                        let key_label = format!("{label} push button");
                                        rsx! {
                                            EncoderSection { key: "{target_key}", label: key_label.clone(), title: "Push button", default_open: false,
                                                for component_key in [component_key] {
                                                    { rsx! { BindingEditor {
                                                        key: "{component_key}",
                                                        scope: input_identity.scope.clone(),
                                                        admission_token: input_identity.token,
                                                        admission_revision: input_identity.revision,
                                                        input_identity: Some(input_identity.clone()),
                                                        active_layer_id: active_layer_id.clone(),
                                                        target: target.clone(),
                                                        key_label: key_label.clone(),
                                                        editor_instance_id,
                                                        request_sequence,
                                                        value: binding.clone(),
                                                        layers: layers.clone(),
                                                        macros: macros.clone(),
                                                        enabled,
                                                        feedback: feedback.clone(),
                                                        on_change,
                                                    } } }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Keep each encoder direction mounted while it is collapsed, as the native disclosure did.
#[component]
fn EncoderSection(label: String, title: String, default_open: bool, children: Element) -> Element {
    let mut open = use_signal(|| default_open);
    rsx! {
        Collapsible { open: open(), on_open_change: move |value| open.set(value), keep_mounted: true,
            CollapsibleTrigger { aria_label: label, class: "w-full p-3 bg-muted text-foreground max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", "{title}" }
            CollapsibleContent { hidden: !open(), inert: (!open()).then_some(""), {children} }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_tests {
    use super::*;
    use crate::keymap::binding_controller::EncoderBindingRow;
    use crate::keymap::binding_editor::EncoderInputIdentity;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::KeyBinding;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[component]
    fn fixture() -> Element {
        let sequence = use_signal(|| 0);
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let projection = EncoderEditorProjection {
            input_identity: EncoderInputIdentity {
                scope,
                token: SnapshotToken(1),
                revision: 1,
                projection_generation: 1,
                electrical_fingerprint: None,
            },
            effective_layer_id: "base".into(),
            rows: Rc::from([EncoderBindingRow {
                id: Rc::from("encoder"),
                label: Rc::from("Encoder 1"),
                clockwise: KeyBinding::None,
                counterclockwise: KeyBinding::None,
                push_key_id: Some(Rc::from("key")),
                push_binding: Some(KeyBinding::None),
            }]),
            layers: Rc::from([]),
            macros: Rc::from([]),
        };
        rsx! { EncoderEditor { projection: Rc::new(projection), editor_instance_id: 1, request_sequence: sequence, enabled: true, feedback: None, on_change: |_| {} } }
    }

    #[wasm_bindgen_test]
    async fn encoder_sections_have_distinct_names_and_retain_their_independent_states() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(fixture),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        gloo_timers::future::TimeoutFuture::new(50).await;
        let clockwise =
            by_role_name(&root, "button", "Encoder 1 clockwise").expect("clockwise section");
        let counter = by_role_name(&root, "button", "Encoder 1 counterclockwise")
            .expect("counterclockwise section");
        let push = by_role_name(&root, "button", "Encoder 1 push button").expect("push section");
        assert_eq!(
            clockwise.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert_eq!(
            counter.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert_eq!(
            push.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        clockwise.click();
        push.click();
        gloo_timers::future::TimeoutFuture::new(50).await;
        assert_eq!(
            clockwise.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        assert_eq!(
            counter.get_attribute("aria-expanded").as_deref(),
            Some("true")
        );
        assert_eq!(push.get_attribute("aria-expanded").as_deref(), Some("true"));
        root.remove();
    }
}
