//! Shared layer control presentation backed by the editor-owned visibility state.
use super::LayerVisibility;
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerTarget {
    Hidden(String),
    ModuleHidden(String),
    LayoutFootprints,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasLayer {
    pub id: String,
    pub label: String,
    pub target: LayerTarget,
}

impl CanvasLayer {
    pub fn hidden(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: label.into(),
            target: LayerTarget::Hidden(id.clone()),
            id,
        }
    }

    pub fn layout_footprints() -> Self {
        Self {
            id: "Footprints".into(),
            label: "Footprints".into(),
            target: LayerTarget::LayoutFootprints,
        }
    }

    pub fn module_hidden(id: impl Into<String>, label: impl Into<String>) -> Self {
        let id = id.into();
        Self {
            label: label.into(),
            target: LayerTarget::ModuleHidden(id.clone()),
            id,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanvasLayerGroup {
    pub title: Option<String>,
    pub layers: Vec<CanvasLayer>,
}

impl CanvasLayerGroup {
    pub fn ungrouped(layers: Vec<CanvasLayer>) -> Self {
        Self {
            title: None,
            layers,
        }
    }

    pub fn titled(title: impl Into<String>, layers: Vec<CanvasLayer>) -> Self {
        Self {
            title: Some(title.into()),
            layers,
        }
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct CanvasLayersProps {
    #[props(default)]
    pub initially_open: bool,
    pub trigger_label: String,
    pub trigger_id: String,
    pub list_id: String,
    pub groups: Vec<CanvasLayerGroup>,
}

/// One layer row. The owner decides what a toggle changes.
#[component]
pub fn LayerToggleButton(
    label: String,
    swatch: String,
    aria_label: String,
    visible: bool,
    on_toggle: EventHandler<()>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            aria_pressed: "{visible}",
            aria_label,
            onclick: move |_| on_toggle.call(()),
            span { class: "m1-layer-swatch", "data-layer": "{swatch}" }
            span { class: "m1-layer-label", "{label}" }
            svg { view_box: "0 0 20 20", "aria-hidden": "true",
                path { d: "M2 10q8-12 16 0-8 12-16 0Z" }
                circle { cx: "10", cy: "10", r: "2.5" }
                if !visible { path { d: "m3 17 14-14" } }
            }
        }
    }
}

/// The shared layers disclosure shell; callers own the layer rows and their visibility state.
#[component]
pub fn LayerPanel(
    #[props(default)] initially_open: bool,
    trigger_label: String,
    trigger_id: String,
    list_id: String,
    children: Element,
) -> Element {
    let keydown_trigger_id = trigger_id.clone();
    let onkeydown = move |event: KeyboardEvent| {
        use wasm_bindgen::JsCast;
        if event.data().key().to_string() != "Escape" {
            return;
        }
        if let Some(trigger) = web_sys::window()
            .and_then(|window| window.document())
            .and_then(|document| document.get_element_by_id(&keydown_trigger_id))
            .and_then(|element| element.dyn_into::<web_sys::HtmlElement>().ok())
            .filter(|trigger| trigger.get_attribute("aria-expanded").as_deref() == Some("true"))
        {
            event.prevent_default();
            trigger.click();
            let _ = trigger.focus();
        }
    };
    rsx! {
        super::controls::BoardAccordionSection {
            id: trigger_id, label: trigger_label.clone(), class: "m1-layers",
            initially_open, onkeydown,
            heading: rsx! { span { "{trigger_label}" } },
            div { id: "{list_id}", class: "m1-layer-list", {children} }
        }
    }
}

#[component]
pub fn CanvasLayers(props: CanvasLayersProps) -> Element {
    let layers = use_context::<LayerVisibility>();
    let hidden = (layers.hidden)();
    let modules_hidden = (layers.modules_hidden)();
    let footprints = (layers.footprints)();
    let toggle = move |target: LayerTarget| match target {
        LayerTarget::Hidden(id) => {
            let mut hidden = layers.hidden;
            let mut next = (hidden)();
            if !next.insert(id.clone()) {
                next.remove(&id);
            }
            hidden.set(next);
        }
        LayerTarget::LayoutFootprints => {
            let mut visible = layers.footprints;
            visible.set(!visible());
        }
        LayerTarget::ModuleHidden(id) => {
            let mut hidden = layers.modules_hidden;
            let mut next = hidden();
            if !next.insert(id.clone()) {
                next.remove(&id);
            }
            hidden.set(next);
        }
    };
    rsx! {
        LayerPanel {
            initially_open: props.initially_open,
            trigger_label: props.trigger_label.clone(),
            trigger_id: props.trigger_id.clone(),
            list_id: props.list_id.clone(),
            for (group_index, group) in props.groups.iter().enumerate() {
                    {
                        let group_title = group.title.clone();
                        let controls = group.layers.iter().map(|layer| {
                            let visible = match &layer.target {
                                LayerTarget::Hidden(id) => !hidden.contains(id),
                                LayerTarget::ModuleHidden(id) => !modules_hidden.contains(id),
                                LayerTarget::LayoutFootprints => footprints && !hidden.contains("Footprints"),
                            };
                            let action = if visible { "Hide" } else { "Show" };
                            let accessibility_label = match &layer.target {
                                LayerTarget::ModuleHidden(_) => &layer.label,
                                LayerTarget::Hidden(id) if id == "Keys" => &layer.label,
                                _ => &layer.id,
                            };
                            let aria_label = format!("{action} {accessibility_label}");
                            let swatch_label = match &layer.target {
                                LayerTarget::ModuleHidden(_) => layer.label.clone(),
                                _ => layer.id.clone(),
                            };
                            let control = layer.clone();
                            let on_toggle = toggle;
                            rsx! {
                                LayerToggleButton {
                                    key: "{layer.id}",
                                    label: layer.label.clone(),
                                    swatch: swatch_label,
                                    aria_label,
                                    visible,
                                    on_toggle: move |_| on_toggle(control.target.clone()),
                                }
                            }
                        });
                        rsx! {
                            div {
                                key: "group-{group_index}",
                                class: "m1-layer-group",
                                role: "group",
                                aria_label: "{group_title.clone().unwrap_or_else(|| \"Other layers\".into())}",
                                if let Some(title) = group_title.as_ref() {
                                    div { class: "m1-layer-group-heading", "{title}" }
                                }
                                for control in controls { {control} }
                            }
                        }
                    }
            }
        }
    }
}

pub fn layout_groups() -> Vec<CanvasLayerGroup> {
    vec![CanvasLayerGroup::ungrouped(vec![
        CanvasLayer::hidden("Keys", "Switches"),
        CanvasLayer::hidden("Components", "Components"),
        CanvasLayer::hidden("Keycaps", "Keycaps"),
        CanvasLayer::layout_footprints(),
        CanvasLayer::hidden("Board", "Board"),
    ])]
}
