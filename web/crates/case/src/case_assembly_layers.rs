//! Private Case assembly and physical-model layer controls.
//!
//! The caller supplies the currently accepted projection. This module owns
//! only viewer display preferences; it does not request previews or models.

use super::{case_display::CaseDisplay, model_delivery::ModelDeliveryRows};
use boardstudio_core::model::PcbModel;
use boardstudio_web_ui_shared::ui::{
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    icons::ChevronDown,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use std::collections::HashSet;
use wasm_bindgen::JsCast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerAvailability {
    Available,
    Pending,
    Unavailable(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseAssemblyLayer {
    pub id: String,
    pub label: String,
    pub availability: LayerAvailability,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseComponentLayer {
    pub id: String,
    pub reference: String,
    pub filename: String,
    pub availability: LayerAvailability,
}

impl CaseComponentLayer {
    fn label(&self) -> String {
        format!("{} · {}", self.reference, self.filename)
    }
}

pub fn standard_assembly_layers(
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
pub fn assembly_layers_with_stack(
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
pub fn physical_component_layers(
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
pub fn CaseAssemblyLayers(
    #[props(default)] initially_open: bool,
    assembly: Vec<CaseAssemblyLayer>,
    components: Vec<CaseComponentLayer>,
    selectable_layers: Vec<String>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
    display: CaseDisplay,
    on_display_change: EventHandler<CaseDisplay>,
) -> Element {
    let mut open = use_signal(|| initially_open);
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
            "aria-label": "Assembly visibility",
            "data-open": "{open()}",
            onkeydown: keydown,
            Collapsible { keep_mounted: true, open: Some(open()), on_open_change: move |next| open.set(next),
            CollapsibleTrigger {
                id: "m1-case-assembly-layers-trigger",
                aria_label: "Assembly visibility",
                class: "min-h-11 w-full justify-between px-3 py-2 text-sm font-medium",
                span { "Assembly visibility" }
                ChevronDown { class: if open() { "size-4 shrink-0 transition-transform rotate-180" } else { "size-4 shrink-0 transition-transform" } }
            }
            CollapsibleContent {
                id: "m1-case-assembly-layers-list",
                class: "m1-case-assembly-layers-list",
                hidden: !open(),
                inert: (!open()).then_some(""),
                if !assembly.is_empty() {
                    div { class: "m1-case-assembly-layer-group", role: "group", "aria-label": "Assembly",
                        div { class: "m1-case-assembly-layer-heading", "Assembly" }
                        for layer in assembly {
                            {
                                let toggle = toggle.clone();
                                let selectable_layers = selectable_layers.clone();
                                let on_select_layer = on_select_layer;
                                let selected_layer = selected_layer.clone();
                                rsx! { CaseLayerButton {
                                    layer,
                                    selectable_layers,
                                    selected_layer,
                                    on_select_layer,
                                    display: display.clone(),
                                    on_toggle: toggle,
                                } }
                            }
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
}

#[component]
fn CaseLayerButton(
    layer: CaseAssemblyLayer,
    selectable_layers: Vec<String>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
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
    let selection_id = case_display_selection_id(&layer.id, &selectable_layers);
    let selectable = selection_id.is_some();
    let selected = selection_id.as_deref() == Some(selected_layer.as_str());
    let selected_id = selection_id.clone();
    let selection_label = layer.label.clone();
    let selection_aria_label = if selectable {
        format!("Select {selection_label} for appearance")
    } else {
        selection_label.clone()
    };
    let visibility_label = if visible { format!("Hide {}", layer.label) } else { format!("Show {}", layer.label) };
    let row_disabled = !selectable || unavailable.is_some();
    let visibility_disabled = unavailable.is_some();
    rsx! {
        div {
            class: "grid grid-cols-[minmax(0,1fr)_auto] items-center gap-1",
            ToggleGroup {
                aria_label: selection_label.clone(),
                disabled: row_disabled,
                pressed: Some(if selected { HashSet::from([0usize]) } else { HashSet::new() }),
                on_pressed_change: move |next: HashSet<usize>| {
                    if next.contains(&0) != selected
                        && let Some(id) = selected_id.clone()
                    {
                        on_select_layer.call(id);
                    }
                },
                style: "width: 100%",
                ToggleItem {
                    index: 0usize,
                    aria_label: selection_aria_label,
                    disabled: row_disabled,
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    style: "width: 100%; justify-content: flex-start;",
                    span { class: "m1-case-assembly-layer-swatch", "data-layer": "{layer.label}" }
                    span { class: "m1-case-assembly-layer-label flex-1 text-left", "{layer.label}" }
                    if let Some(state) = unavailable { small { class: "whitespace-nowrap text-xs text-muted-foreground", "{state}" } }
                }
            }
            ToggleGroup {
                aria_label: format!("{} visibility", layer.label),
                disabled: visibility_disabled,
                pressed: Some(if visible { HashSet::from([0usize]) } else { HashSet::new() }),
                on_pressed_change: move |next: HashSet<usize>| {
                    if next.contains(&0) != visible { on_toggle.call(id.clone()); }
                },
                Tooltip {
                    TooltipTrigger { r#as: {
                        let visibility_label = visibility_label.clone();
                        move |attributes: Vec<Attribute>| rsx! {
                            ToggleItem { index: 0usize, aria_label: visibility_label.clone(),
                                disabled: visibility_disabled,
                                class: "max-[960px]:min-h-11 max-[960px]:min-w-11 [@media(pointer:coarse)]:min-h-11 [@media(pointer:coarse)]:min-w-11",
                                attributes,
                                LayerEye { visible }
                            }
                        }
                    } }
                    TooltipContent { "{visibility_label}" }
                }
            }
        }
    }
}

fn case_display_selection_id(id: &str, selectable_layers: &[String]) -> Option<String> {
    let id = match id {
        "PCB" => "pcb",
        "Gaskets" => "gaskets",
        id => id,
    };
    selectable_layers
        .iter()
        .any(|selectable| selectable == id)
        .then(|| id.to_owned())
}

#[component]
fn ComponentLayerButton(
    layer: CaseComponentLayer,
    display: CaseDisplay,
    on_toggle: EventHandler<String>,
) -> Element {
    let available = layer.availability == LayerAvailability::Available;
    let visible = available && !display.hidden.iter().any(|id| id == &layer.id);
    let unavailable = match &layer.availability {
        LayerAvailability::Available => None,
        LayerAvailability::Pending => Some("Loading model…"),
        LayerAvailability::Unavailable(_) => Some("Missing model"),
    };
    let id = layer.id.clone();
    let label = layer.label();
    let toggle_label = if visible { format!("Hide {label}") } else { format!("Show {label}") };
    let row_disabled = unavailable.is_some();
    rsx! {
        ToggleGroup {
            aria_label: format!("{label} visibility"),
            disabled: row_disabled,
            pressed: Some(if visible { HashSet::from([0usize]) } else { HashSet::new() }),
            on_pressed_change: move |next: HashSet<usize>| {
                if next.contains(&0) != visible { on_toggle.call(id.clone()); }
            },
            style: "width: 100%",
            ToggleItem {
                index: 0usize,
                aria_label: toggle_label,
                disabled: row_disabled,
                class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                style: "width: 100%; justify-content: flex-start;",
                span { class: "m1-case-assembly-layer-swatch is-component", "data-layer": "Components" }
                span { class: "m1-case-assembly-layer-label flex-1 text-left", "{label}" }
                if let Some(state) = unavailable { small { class: "whitespace-nowrap text-xs text-muted-foreground", "{state}" } }
                LayerEye { visible }
            }
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
    use boardstudio_web_ui_shared::{test_role::by_role_name, test_wait::wait_until};
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
        let mut selected_layer = use_signal(String::new);
        let assembly = assembly_layers_with_stack(
            [("plate".into(), "Plate body".into())],
            ["battery".into(), "plate".into()],
        );
        let models = vec![
            model("switch-mesh-1", "SW1", "models/switch.step"),
            model("companion-mesh-1", "SW1", "models/companion.step"),
            model("diode-mesh-1", "D1", "models/diode.step"),
        ];
        let delivery = ModelDeliveryRows {
            delivered: models[..2]
                .iter()
                .map(|model| DeliveredModel {
                    id: model.id.clone(),
                    mesh: Rc::new(ValidatedMesh {
                        positions: Rc::from([0.0_f32; 9]),
                        normals: Rc::from([0.0_f32; 9]),
                        colors: None,
                        step_bounds: None,
                        vendor_step: None,
                    }),
                    matrix: None,
                })
                .collect(),
            pending: Vec::new(),
            failures: Vec::new(),
        };
        let components = physical_component_layers(&models, Some(&delivery));
        rsx! {
            CaseAssemblyLayers {
                assembly,
                components,
                selectable_layers: vec!["pcb".into(), "plate".into()],
                selected_layer: selected_layer(),
                on_select_layer: move |id| selected_layer.set(id),
                display: display(),
                on_display_change: move |next| display.set(next),
            }
            output { "aria-label": "Selected layer", "{selected_layer()}" }
            output { "aria-label": "Hidden model ids", {display().hidden.join(",")} }
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
                    step_bounds: None,
                    vendor_step: None,
                }),
                matrix: None,
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
    fn display_selection_normalizes_pcb_and_only_exposes_supported_layers() {
        let selectable = vec!["pcb".to_owned(), "plate".to_owned()];
        assert_eq!(
            case_display_selection_id("PCB", &selectable),
            Some("pcb".into())
        );
        assert_eq!(
            case_display_selection_id("plate", &selectable),
            Some("plate".into())
        );
        assert_eq!(case_display_selection_id("Copper", &selectable), None);
    }

    #[wasm_bindgen_test]
    async fn layer_menu_toggles_exact_rows_and_unavailable_rows_are_not_checked() {
        let document = web_sys::window().unwrap().document().unwrap();
        let stylesheet = document.create_element("style").unwrap();
        stylesheet.set_text_content(Some(include_str!("../../../assets/m1.css")));
        document.head().unwrap().append_child(&stylesheet).unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("case-assembly-layer-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        dioxus_web::launch::launch_virtual_dom(
            VirtualDom::new(layer_menu_composition),
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        rendered().await;

        let scope = root.clone();
        let list = element("#m1-case-assembly-layers-list");
        assert!(list.has_attribute("hidden"));
        assert_eq!(computed_display(&list), "none");
        assert_eq!(list.get_bounding_client_rect().height(), 0.0);
        let trigger = by_role_name(&scope, "button", "Assembly visibility")
            .expect("assembly visibility trigger");
        assert_eq!(
            trigger.text_content().unwrap().trim(),
            "Assembly visibility"
        );
        assert_eq!(
            trigger.get_attribute("aria-expanded").as_deref(),
            Some("false")
        );
        let battery = by_role_name(&scope, "button", "Show battery")
            .expect("unavailable battery row");
        assert!(battery.has_attribute("disabled"));
        assert_eq!(
            battery.get_attribute("aria-pressed").as_deref(),
            Some("false")
        );
        let plate = by_role_name(&scope, "button", "Hide Plate").expect("plate visibility toggle");
        assert!(!plate.has_attribute("disabled"));
        assert_eq!(plate.get_attribute("aria-pressed").as_deref(), Some("true"));

        trigger.click();
        rendered().await;
        assert!(!list.has_attribute("hidden"));
        assert_eq!(computed_display(&list), "grid");
        assert!(list.get_bounding_client_rect().height() > 0.0);
        assert_eq!(
            by_role_name(&scope, "button", "Assembly visibility")
                .unwrap()
                .get_attribute("aria-expanded")
                .as_deref(),
            Some("true")
        );
        // The icon-only visibility toggle describes itself through its tooltip.
        plate.focus().unwrap();
        wait_until("visibility tooltip describes the toggle", || {
            by_role_name(&scope, "button", "Hide Plate").is_some_and(|toggle| {
                toggle
                    .get_attribute("aria-describedby")
                    .and_then(|id| document.get_element_by_id(&id))
                    .is_some_and(|tip| {
                        tip.text_content().unwrap_or_default().contains("Hide Plate")
                    })
            })
        })
        .await;
        let select_plate = by_role_name(&scope, "button", "Select Plate for appearance")
            .expect("plate appearance row");
        select_plate.click();
        rendered().await;
        assert_eq!(
            element("#case-assembly-layer-test-root output[aria-label='Selected layer']")
                .text_content()
                .as_deref(),
            Some("plate")
        );
        assert_eq!(
            by_role_name(&scope, "button", "Select Plate for appearance")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        assert_eq!(
            by_role_name(&scope, "button", "Hide Plate")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        let hidden_component = by_role_name(&scope, "button", "Hide SW1 · switch.step")
            .expect("component visibility toggle");
        assert_eq!(
            hidden_component.get_attribute("aria-pressed").as_deref(),
            Some("true")
        );
        assert_eq!(
            by_role_name(&scope, "button", "Hide SW1 · companion.step")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        hidden_component.click();
        rendered().await;
        assert_eq!(
            by_role_name(&scope, "button", "Show SW1 · switch.step")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("false")
        );
        assert_eq!(
            by_role_name(&scope, "button", "Hide SW1 · companion.step")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true"),
            "hiding one model must keep its same-reference sibling visible"
        );
        assert_eq!(
            element("#case-assembly-layer-test-root output[aria-label='Hidden model ids']")
                .text_content()
                .as_deref(),
            Some("switch-mesh-1")
        );
        by_role_name(&scope, "button", "Show SW1 · switch.step")
            .unwrap()
            .click();
        rendered().await;
        assert_eq!(
            by_role_name(&scope, "button", "Hide SW1 · switch.step")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        assert_eq!(
            element("#case-assembly-layer-test-root output[aria-label='Hidden model ids']")
                .text_content()
                .as_deref(),
            Some("")
        );
        assert_eq!(
            by_role_name(&scope, "button", "Hide SW1 · companion.step")
                .unwrap()
                .get_attribute("aria-pressed")
                .as_deref(),
            Some("true")
        );
        let unavailable = by_role_name(&scope, "button", "Show D1 · diode.step")
            .expect("missing model row");
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
        by_role_name(&scope, "button", "Hide SW1 · switch.step")
            .unwrap()
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
        root.remove();
        stylesheet.remove();
    }
}
