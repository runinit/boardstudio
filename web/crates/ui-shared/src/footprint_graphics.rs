//! Dioxus owns drawing; the Rust footprint generators own generator execution.
use crate::footprint_forms::{self as forms, Graphic, Shape};
use boardstudio_model::model::{PartDefinition, Side};
use dioxus::prelude::*;
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, VecDeque},
    rc::Rc,
};

pub type Drawings = Rc<Vec<Graphic>>;

pub use crate::generator_preview_defaults::GeneratorPreviewDefaults;
use crate::generator_preview_defaults::generator_preview_defaults_from_parameters;
thread_local! {
    // Keys include the complete definition and instance overrides. Pose stays in
    // the parent SVG transform. Bound retention when users edit many definitions.
    static CACHE: RefCell<VecDeque<(String, Drawings)>> = const { RefCell::new(VecDeque::new()) };
}

/// Read only the envelope defaults used by the Parts library preview. The
/// generator's declared parameters are the source of truth for its defaults.
pub async fn generator_preview_defaults(
    source: &str,
) -> Result<Option<GeneratorPreviewDefaults>, String> {
    if !boardstudio_core_kernel::generators::is_generator(source) {
        return Ok(None);
    }
    let parameters = boardstudio_core_kernel::generators::parameter_schema(source)?;
    Ok(Some(generator_preview_defaults_from_parameters(
        &serde_json::to_value(parameters).map_err(|error| error.to_string())?,
    )))
}

pub async fn generator_drawings(
    source_definition: PartDefinition,
    parameters: Option<BTreeMap<String, serde_json::Value>>,
    include_keycap: Option<bool>,
) -> Result<Option<Drawings>, String> {
    let mut definition = source_definition;
    let Some(generator) = definition.generator.as_mut() else {
        return Ok(None);
    };
    generator.parameters.extend(parameters.unwrap_or_default());
    if let Some(include_keycap) = include_keycap {
        generator.parameters.insert(
            "include_keycap".into(),
            serde_json::Value::Bool(include_keycap),
        );
    }
    let source = generator.source.clone();
    let key = serde_json::to_string(&definition).map_err(|e| e.to_string())?;
    if let Some(result) = CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.clone())
    }) {
        return Ok(Some(result));
    }
    if !boardstudio_core_kernel::generators::is_generator(&source) {
        return Ok(None);
    }
    let rendered = boardstudio_core_kernel::generators::render_forms(&definition, None)?;
    let result = Rc::new(forms::project(&rendered));
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 64 {
            cache.pop_front();
        }
        cache.push_back((key, result.clone()));
    });
    Ok(Some(result))
}

#[component]
pub fn FootprintGraphics(
    definition: PartDefinition,
    parameters: Option<BTreeMap<String, serde_json::Value>>,
    #[props(default)] hidden_layers: Option<BTreeSet<String>>,
    #[props(default)] part_side: Option<Side>,
) -> Element {
    let graphics = use_resource(use_reactive(
        (&definition, &parameters),
        |(definition, parameters)| generator_drawings(definition, parameters, Some(false)),
    ));
    match &*graphics.read() {
        Some(Ok(Some(items))) => rsx! {
            g { "data-status": "ready", "aria-hidden": "true",
                for (index, graphic) in items.iter().enumerate() {
                    {
                        let hidden = hidden_layers.as_ref().zip(part_side.as_ref()).is_some_and(|(hidden_layers, side)| {
                            hidden_layers.contains(&resolve_board_layer(&graphic.layer, side))
                        });
                        rsx! { GraphicElement { key: "{index}", graphic: graphic.clone(), hidden } }
                    }
                }
            }
        },
        Some(Ok(None)) => rsx! {
            g { "data-status": "ready", "aria-hidden": "true" }
        },
        Some(Err(error)) => rsx! {
            g { class: "fill-destructive [font:1.5px_var(--theme-font-sans)] pointer-events-none", "data-status": "error",
                title { "Footprint graphics unavailable: {error}. Toggle Footprints to retry." }
                text { x: 0, y: 3, transform: "scale(1,-1)", "Graphics unavailable" }
            }
        },
        None => rsx! { g { "data-status": "loading" } },
    }
}

pub use crate::board_layer::resolve_board_layer;

#[component]
pub fn GraphicElement(graphic: Graphic, hidden: bool) -> Element {
    if hidden {
        return rsx! {};
    }
    let layer = graphic.layer;
    match graphic.shape {
        Shape::Line(a, b) => {
            rsx! { line { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, x1: a.0, y1: a.1, x2: b.0, y2: b.1 } }
        }
        Shape::Arc(a, m, b) => {
            rsx! { path { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, d: format!("M {} {} Q {} {} {} {}", a.0,a.1,2.0*m.0-(a.0+b.0)/2.0,2.0*m.1-(a.1+b.1)/2.0,b.0,b.1) } }
        }
        Shape::Rect(a, b) => {
            rsx! { rect { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, x: a.0.min(b.0), y: a.1.min(b.1), width: (b.0-a.0).abs(), height: (b.1-a.1).abs() } }
        }
        Shape::Circle(c, r) => {
            rsx! { circle { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, cx: c.0, cy: c.1, r } }
        }
        Shape::Polygon(points, keepout) => {
            rsx! { polygon { class: if keepout { "fill-none stroke-geometry-graphic [stroke-width:0.12] [stroke-dasharray:0.5_0.5] pointer-events-none" } else { "fill-geometry-zone stroke-geometry-graphic [stroke-width:0.12] pointer-events-none" }, "data-layer": layer, points: points.iter().map(|p|format!("{},{}",p.0,p.1)).collect::<Vec<_>>().join(" ") } }
        }
        Shape::Text(at, text) => {
            rsx! { text { class: "fill-geometry-label [font:0.8px_var(--theme-font-mono)] pointer-events-none", "data-layer": layer, transform: "scale(1,-1)", x: at.0, y: -at.1, "{text}" } }
        }
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod graphic_style_tests {
    use super::*;
    use crate::footprint_forms::Point;
    use crate::test_mount::MountedFixture;
    use crate::test_wait::wait_until;
    use wasm_bindgen_test::*;

    fn fixture() -> Element {
        let p = Point(0.0, 0.0);
        let shapes = [
            ("line", Shape::Line(p.clone(), Point(1.0, 1.0))),
            (
                "arc",
                Shape::Arc(p.clone(), Point(1.0, 1.0), Point(2.0, 0.0)),
            ),
            ("rect", Shape::Rect(p.clone(), Point(1.0, 1.0))),
            ("circle", Shape::Circle(p.clone(), 1.0)),
            (
                "zone",
                Shape::Polygon(vec![p.clone(), Point(1.0, 0.0), Point(0.0, 1.0)], false),
            ),
            (
                "keepout",
                Shape::Polygon(vec![p.clone(), Point(1.0, 0.0), Point(0.0, 1.0)], true),
            ),
            ("text", Shape::Text(p, "Label".into())),
        ];
        rsx! {
            svg {
                for (layer, shape) in shapes {
                    GraphicElement { graphic: Graphic { layer: layer.into(), shape }, hidden: false }
                }
            }
        }
    }

    fn error_fixture() -> Element {
        let definition: PartDefinition = serde_json::from_value(serde_json::json!({
            "id": "invalid-generator", "name": "Invalid", "kind": "switch",
            "courtyard": [], "pads": [],
            "generator": {"source": "ceoloide/switch_mx", "version": "unsupported", "parameters": {}}
        })).unwrap();
        rsx! { svg { FootprintGraphics { definition, parameters: None } } }
    }

    #[wasm_bindgen_test]
    async fn unavailable_graphics_keep_the_error_style_without_legacy_selectors() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(error_fixture));
        wait_until("graphics error renders", || {
            root.query_selector("[data-status=error]")
                .unwrap()
                .is_some()
        })
        .await;
        let error = root.query_selector("[data-status=error]").unwrap().unwrap();
        let classes = error.get_attribute("class").unwrap_or_default();
        for class in [
            "fill-destructive",
            "[font:1.5px_var(--theme-font-sans)]",
            "pointer-events-none",
        ] {
            assert!(
                classes.split_whitespace().any(|actual| actual == class),
                "missing {class}"
            );
        }
        assert!(
            error
                .text_content()
                .unwrap_or_default()
                .contains("Graphics unavailable")
        );
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn graphic_shapes_keep_their_styling_without_legacy_selectors() {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        document.body().unwrap().append_child(&root).unwrap();
        let _mounted = MountedFixture::new(root.clone(), VirtualDom::new(fixture));
        wait_until("graphics mount", || {
            root.query_selector("[data-layer=text]").unwrap().is_some()
        })
        .await;
        let graphic: &[&str] = &[
            "fill-none",
            "stroke-geometry-graphic",
            "[stroke-width:0.15]",
            "[stroke-linecap:round]",
            "[stroke-linejoin:round]",
            "pointer-events-none",
        ];
        for (layer, expected) in [
            ("line", graphic),
            ("arc", graphic),
            ("rect", graphic),
            ("circle", graphic),
            (
                "zone",
                &[
                    "fill-geometry-zone",
                    "stroke-geometry-graphic",
                    "[stroke-width:0.12]",
                    "pointer-events-none",
                ][..],
            ),
            (
                "keepout",
                &[
                    "fill-none",
                    "stroke-geometry-graphic",
                    "[stroke-width:0.12]",
                    "[stroke-dasharray:0.5_0.5]",
                    "pointer-events-none",
                ][..],
            ),
            (
                "text",
                &[
                    "fill-geometry-label",
                    "[font:0.8px_var(--theme-font-mono)]",
                    "pointer-events-none",
                ][..],
            ),
        ] {
            let shape = root
                .query_selector(&format!("[data-layer={layer}]"))
                .unwrap()
                .unwrap();
            let classes = shape.get_attribute("class").unwrap_or_default();
            for class in expected {
                assert!(
                    classes.split_whitespace().any(|actual| actual == *class),
                    "{layer}: missing {class}"
                );
            }
        }
        root.remove();
    }
}
