//! Private Case assembly and physical-model layer controls.
//!
//! The caller supplies the currently accepted projection. This module owns
//! only viewer display preferences; it does not request previews or models.

use super::{case_display::CaseDisplay, model_delivery::ModelDeliveryRows};
use boardstudio_core::model::PcbModel;
use dioxus::prelude::*;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LayerAvailability {
    Available,
    Pending,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaseAssemblyLayer {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) availability: LayerAvailability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaseComponentLayer {
    pub(crate) id: String,
    pub(crate) reference: String,
    pub(crate) filename: String,
    pub(crate) availability: LayerAvailability,
}

impl CaseComponentLayer {
    fn label(&self) -> String {
        format!("{} · {}", self.reference, self.filename)
    }
}

pub(crate) fn standard_assembly_layers(
    generated: impl IntoIterator<Item = (String, String)>,
) -> Vec<CaseAssemblyLayer> {
    let mut layers = [
        ("PCB", "PCB"),
        ("Copper", "Copper"),
        ("Mask", "Mask openings"),
        ("Silkscreen", "Silkscreen"),
        ("Models", "Models"),
        ("Keycaps", "Keycaps"),
    ]
    .into_iter()
    .map(|(id, label)| CaseAssemblyLayer {
        id: id.to_owned(),
        label: label.to_owned(),
        availability: LayerAvailability::Available,
    })
    .collect::<Vec<_>>();
    for (id, label) in generated {
        if id != "pcb" && !layers.iter().any(|layer| layer.id == id) {
            let label = assembly_layer_label(&id, &label);
            layers.push(CaseAssemblyLayer {
                id,
                label,
                availability: LayerAvailability::Available,
            });
        }
    }
    layers
}

/// Keep configured mechanical stack entries visible even when the current CAD
/// result has no corresponding body. Only generated bodies can be toggled.
pub(crate) fn assembly_layers_with_stack(
    generated: impl IntoIterator<Item = (String, String)>,
    configured_stack_ids: impl IntoIterator<Item = String>,
) -> Vec<CaseAssemblyLayer> {
    let generated = generated.into_iter().collect::<Vec<_>>();
    let generated_ids = generated
        .iter()
        .map(|(id, _)| id.clone())
        .collect::<Vec<_>>();
    let mut layers = standard_assembly_layers(generated);

    for id in configured_stack_ids {
        let row_id = if id == "pcb" { "PCB" } else { id.as_str() };
        if layers.iter().any(|layer| layer.id == row_id) {
            continue;
        }
        let label = assembly_layer_label(&id, &id);
        layers.push(CaseAssemblyLayer {
            id: row_id.to_owned(),
            label,
            availability: if generated_ids.contains(&id) || id == "pcb" {
                LayerAvailability::Available
            } else {
                LayerAvailability::Unavailable("Not generated".to_owned())
            },
        });
    }
    layers
}

fn assembly_layer_label(id: &str, fallback: &str) -> String {
    match id {
        "plate" => "Plate".to_owned(),
        "plate-foam" => "Plate foam".to_owned(),
        "bottom-foam" => "Bottom foam".to_owned(),
        "bottom" => "Bottom".to_owned(),
        _ => fallback.to_owned(),
    }
}

/// Project the accepted preview rows into layer controls without changing their
/// order or substituting references/asset paths for renderer model IDs.
pub(crate) fn physical_component_layers(
    models: &[PcbModel],
    delivery: Option<&ModelDeliveryRows>,
) -> Vec<CaseComponentLayer> {
    models
        .iter()
        .map(|model| {
            let availability = match delivery {
                Some(rows) if rows.delivered.iter().any(|row| row.id == model.id) => {
                    LayerAvailability::Available
                }
                Some(rows) if rows.pending.iter().any(|id| id == &model.id) => {
                    LayerAvailability::Pending
                }
                Some(_) => LayerAvailability::Unavailable("Model is not available".to_owned()),
                None => LayerAvailability::Pending,
            };
            CaseComponentLayer {
                id: model.id.clone(),
                reference: model.reference.clone(),
                filename: model
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&model.path)
                    .to_owned(),
                availability,
            }
        })
        .collect()
}

#[component]
pub(crate) fn CaseAssemblyLayers(
    assembly: Vec<CaseAssemblyLayer>,
    components: Vec<CaseComponentLayer>,
    display: CaseDisplay,
    on_display_change: EventHandler<CaseDisplay>,
) -> Element {
    let mut open = use_signal(|| false);
    let display_for_toggle = display.clone();
    let toggle = move |id: String| {
        let mut next = display_for_toggle.clone();
        if next.hidden.iter().any(|hidden| hidden == &id) {
            next.hidden.retain(|hidden| hidden != &id);
        } else {
            next.hidden.push(id);
        }
        on_display_change.call(next);
    };
    let keydown = move |event: KeyboardEvent| {
        if event.data().key().to_string() == "Escape" && open() {
            event.prevent_default();
            open.set(false);
            focus_layers_trigger();
        }
    };
    rsx! {
        section {
            class: "m1-case-assembly-layers",
            "aria-label": "Assembly layers",
            "data-open": "{open()}",
            onkeydown: keydown,
            button {
                id: "m1-case-assembly-layers-trigger",
                class: "m1-case-assembly-layers-trigger",
                "aria-expanded": "{open()}",
                "aria-controls": "m1-case-assembly-layers-list",
                onclick: move |_| open.set(!open()),
                "Layers",
                svg { view_box: "0 0 20 20", "aria-hidden": "true",
                    path { d: if open() { "m5 12 5-5 5 5" } else { "m5 8 5 5 5-5" } }
                }
            }
            div {
                id: "m1-case-assembly-layers-list",
                class: "m1-case-assembly-layers-list",
                hidden: !open(),
                if !assembly.is_empty() {
                    div { class: "m1-case-assembly-layer-group", role: "group", "aria-label": "Assembly",
                        div { class: "m1-case-assembly-layer-heading", "Assembly" }
                        for layer in assembly {
                            { let toggle = toggle.clone(); rsx! { CaseLayerButton { layer, display: display.clone(), on_toggle: toggle } } }
                        }
                    }
                }
                if !components.is_empty() {
                    div { class: "m1-case-assembly-layer-group", role: "group", "aria-label": "Components",
                        div { class: "m1-case-assembly-layer-heading", "Components" }
                        for layer in components {
                            { let toggle = toggle.clone(); rsx! { ComponentLayerButton { layer, display: display.clone(), on_toggle: toggle } } }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn CaseLayerButton(
    layer: CaseAssemblyLayer,
    display: CaseDisplay,
    on_toggle: EventHandler<String>,
) -> Element {
    let available = layer.availability == LayerAvailability::Available;
    let visible = available && !display.hidden.iter().any(|id| id == &layer.id);
    let unavailable = match &layer.availability {
        LayerAvailability::Available => None,
        LayerAvailability::Pending => Some("Not ready"),
        LayerAvailability::Unavailable(reason) => Some(reason.as_str()),
    };
    let id = layer.id.clone();
    rsx! {
        button {
            class: "m1-case-assembly-layer-row",
            "aria-pressed": "{visible}",
            "aria-label": if visible { "Hide {layer.label}" } else { "Show {layer.label}" },
            disabled: unavailable.is_some(),
            title: unavailable.unwrap_or(&layer.label),
            onclick: move |_| on_toggle.call(id.clone()),
            span { class: "m1-case-assembly-layer-swatch", "data-layer": "{layer.label}" }
            span { class: "m1-case-assembly-layer-label", "{layer.label}" }
            if let Some(state) = unavailable { small { "{state}" } }
            LayerEye { visible }
        }
    }
}

#[component]
fn ComponentLayerButton(
    layer: CaseComponentLayer,
    display: CaseDisplay,
    on_toggle: EventHandler<String>,
) -> Element {
    let available = layer.availability == LayerAvailability::Available;
    let visible = available && !display.hidden.iter().any(|id| id == &layer.id);
    let (unavailable, tooltip) = match &layer.availability {
        LayerAvailability::Available => (None, layer.label()),
        LayerAvailability::Pending => (
            Some("Loading model…"),
            format!("{} — Loading model", layer.label()),
        ),
        LayerAvailability::Unavailable(reason) => (
            Some("Missing model"),
            format!("{} — {reason}", layer.label()),
        ),
    };
    let id = layer.id.clone();
    let label = layer.label();
    rsx! {
        button {
            class: "m1-case-assembly-layer-row is-component",
            "aria-pressed": "{visible}",
            "aria-label": if visible { "Hide {label}" } else { "Show {label}" },
            disabled: unavailable.is_some(),
            title: "{tooltip}",
            onclick: move |_| on_toggle.call(id.clone()),
            span { class: "m1-case-assembly-layer-swatch is-component", "data-layer": "Components" }
            span { class: "m1-case-assembly-layer-label", "{label}" }
            if let Some(state) = unavailable { small { "{state}" } }
            LayerEye { visible }
        }
    }
}

#[component]
fn LayerEye(visible: bool) -> Element {
    rsx! { svg { view_box: "0 0 20 20", "aria-hidden": "true",
        path { d: "M2 10q8-12 16 0-8 12-16 0Z" }
        circle { cx: "10", cy: "10", r: "2.5" }
        if !visible { path { d: "m3 17 14-14" } }
    } }
}

fn focus_layers_trigger() {
    if let Some(trigger) = web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.get_element_by_id("m1-case-assembly-layers-trigger"))
        .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
    {
        let _ = trigger.focus();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presentation::model_delivery::{DeliveredModel, ValidatedMesh};
    use std::rc::Rc;
    use wasm_bindgen::JsCast;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);
    use boardstudio_core::model::{PcbModel, Pose2, Side, Vec2, Vec3};

    fn model(id: &str, reference: &str, path: &str) -> PcbModel {
        PcbModel {
            id: id.into(),
            reference: reference.into(),
            path: path.into(),
            pose: Pose2 {
                at: Vec2 { x: 1.0, y: 2.0 },
                rotation: 0.25,
            },
            side: Side::Front,
            offset: Vec3::default(),
            rotation: Vec3::default(),
            scale: Vec3 {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
        }
    }

    fn layer_menu_composition() -> Element {
        let mut display = use_signal(CaseDisplay::default);
        let assembly = assembly_layers_with_stack(
            [("plate".into(), "Plate body".into())],
            ["battery".into(), "plate".into()],
        );
        let components = vec![
            CaseComponentLayer {
                id: "switch-mesh-1".into(),
                reference: "SW1".into(),
                filename: "switch.step".into(),
                availability: LayerAvailability::Available,
            },
            CaseComponentLayer {
                id: "diode-mesh-1".into(),
                reference: "D1".into(),
                filename: "diode.step".into(),
                availability: LayerAvailability::Unavailable("Missing model".into()),
            },
        ];
        rsx! {
            CaseAssemblyLayers {
                assembly,
                components,
                display: display(),
                on_display_change: move |next| display.set(next),
            }
        }
    }

    fn element(selector: &str) -> web_sys::HtmlElement {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(selector)
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap()
    }

    fn computed_display(element: &web_sys::Element) -> String {
        web_sys::window()
            .unwrap()
            .get_computed_style(element)
            .unwrap()
            .unwrap()
            .get_property_value("display")
            .unwrap()
    }

    async fn rendered() {
        gloo_timers::future::TimeoutFuture::new(60).await;
    }

    #[wasm_bindgen_test]
    fn physical_rows_keep_preview_order_and_exact_model_id_join() {
        let preview_models = vec![
            model("mesh-id-switch", "SW1", "models/switch.step"),
            model("mesh-id-diode", "D1", "models/diode.step"),
        ];
        let delivered = ModelDeliveryRows {
            delivered: vec![DeliveredModel {
                id: "mesh-id-diode".into(),
                mesh: Rc::new(ValidatedMesh {
                    positions: Rc::from([0.0_f32; 9]),
                    normals: Rc::from([0.0_f32; 9]),
                    colors: None,
                }),
            }],
            pending: vec!["mesh-id-switch".into()],
            failures: Vec::new(),
        };

        let rows = physical_component_layers(&preview_models, Some(&delivered));

        assert_eq!(
            rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>(),
            ["mesh-id-switch", "mesh-id-diode"]
        );
        assert_eq!(rows[0].label(), "SW1 · switch.step");
        assert_eq!(rows[0].availability, LayerAvailability::Pending);
        assert_eq!(rows[1].label(), "D1 · diode.step");
        assert_eq!(rows[1].availability, LayerAvailability::Available);
    }

    #[wasm_bindgen_test]
    fn missing_delivery_never_marks_a_preview_model_available() {
        let models = [model("mesh-id", "SW1", "assets/switch.step")];
        let rows = physical_component_layers(&models, None);
        assert_eq!(rows[0].availability, LayerAvailability::Pending);
    }

    #[wasm_bindgen_test]
    fn assembly_controls_keep_distinct_visibility_ids_and_generated_rows() {
        let layers = standard_assembly_layers([
            ("pcb".into(), "legacy PCB alias".into()),
            ("plate:lower".into(), "Plate".into()),
        ]);
        assert_eq!(layers.len(), 7);
        assert_eq!(layers[0].id, "PCB");
        assert_eq!(layers[1].id, "Copper");
        assert_eq!(layers[2].label, "Mask openings");
        assert_eq!(layers.last().unwrap().id, "plate:lower");
    }

    #[wasm_bindgen_test]
    fn configured_stack_keeps_unmeshed_battery_unavailable_and_generated_plate_available() {
        let layers = assembly_layers_with_stack(
            [("plate".into(), "Plate body".into())],
            ["battery".into(), "plate".into()],
        );

        let battery = layers.iter().find(|layer| layer.id == "battery").unwrap();
        assert_eq!(battery.label, "battery");
        assert_eq!(
            battery.availability,
            LayerAvailability::Unavailable("Not generated".into())
        );
        let plate = layers.iter().find(|layer| layer.id == "plate").unwrap();
        assert_eq!(plate.label, "Plate");
        assert_eq!(plate.availability, LayerAvailability::Available);
    }

    #[wasm_bindgen_test]
    async fn layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked() {
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(include_str!("../../assets/m1.css")));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-assembly-layer-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(layer_menu_composition),
            dioxus_web::Config::new().rootnode(root.into()),
        );
        rendered().await;

        let list = element("#m1-case-assembly-layers-list");
        assert!(list.has_attribute("hidden"));
        assert_eq!(computed_display(&list), "none");
        assert_eq!(list.get_bounding_client_rect().height(), 0.0);
        let battery = element("#m1-case-assembly-layers-list [aria-label='Show battery']");
        assert!(battery.has_attribute("disabled"));
        assert_eq!(
            battery.get_attribute("aria-pressed").as_deref(),
            Some("false")
        );
        let plate = element("#m1-case-assembly-layers-list [aria-label='Hide Plate']");
        assert!(!plate.has_attribute("disabled"));
        assert_eq!(plate.get_attribute("aria-pressed").as_deref(), Some("true"));

        element("#m1-case-assembly-layers-trigger").click();
        rendered().await;
        assert!(!list.has_attribute("hidden"));
        assert_eq!(computed_display(&list), "grid");
        assert!(list.get_bounding_client_rect().height() > 0.0);
        let hidden_component =
            element("#m1-case-assembly-layers-list [aria-label='Hide SW1 · switch.step']");
        hidden_component.click();
        rendered().await;
        assert_eq!(
            element("#m1-case-assembly-layers-list [aria-label='Show SW1 · switch.step']")
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("false")
        );
        let unavailable =
            element("#m1-case-assembly-layers-list [aria-label='Show D1 · diode.step']");
        assert!(unavailable.has_attribute("disabled"));
        assert_eq!(
            unavailable.get_attribute("aria-pressed").as_deref(),
            Some("false")
        );

        let event_init = web_sys::KeyboardEventInit::new();
        event_init.set_key("Escape");
        event_init.set_bubbles(true);
        let event =
            web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event_init)
                .unwrap();
        element("#m1-case-assembly-layers-list [aria-label='Show SW1 · switch.step']")
            .dispatch_event(&event)
            .unwrap();
        rendered().await;
        assert!(list.has_attribute("hidden"));
        assert_eq!(computed_display(&list), "none");
        assert_eq!(list.get_bounding_client_rect().height(), 0.0);
        assert_eq!(
            document.active_element().unwrap().id(),
            "m1-case-assembly-layers-trigger"
        );
    }
}
