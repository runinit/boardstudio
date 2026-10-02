//! Dioxus owns drawing; the unchanged catalogue service owns generator execution.
use crate::footprint_forms::{self as forms, Graphic, Shape};
use boardstudio_core::model::PartDefinition;
use dioxus::prelude::*;
use js_sys::{Function, Promise, Reflect};
use std::{
    cell::RefCell,
    collections::{BTreeMap, VecDeque},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

pub(super) type Drawings = Rc<Vec<Graphic>>;
thread_local! {
    // Keys include the complete definition and instance overrides. Pose stays in
    // the parent SVG transform. Bound retention when users edit many definitions.
    static CACHE: RefCell<VecDeque<(String, Drawings)>> = const { RefCell::new(VecDeque::new()) };
}

fn js_error(error: JsValue) -> String {
    format!("{error:?}")
}

pub(super) async fn generator_drawings(
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
    let url = crate::runtime::resource_url("assets/layout-generators.js")?;
    let import = Function::new_with_args("url", "return import(url)");
    let module = JsFuture::from(
        import
            .call1(&JsValue::NULL, &url.into())
            .map_err(js_error)?
            .dyn_into::<Promise>()
            .map_err(js_error)?,
    )
    .await
    .map_err(js_error)?;
    // Concurrent instances can finish importing the same module together.
    if let Some(result) = CACHE.with(|cache| {
        cache
            .borrow()
            .iter()
            .find(|(k, _)| k == &key)
            .map(|(_, v)| v.clone())
    }) {
        return Ok(Some(result));
    }
    let is_ergogen = Reflect::get(&module, &"isErgogen".into())
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(js_error)?;
    if !is_ergogen
        .call1(&JsValue::NULL, &source.into())
        .map_err(js_error)?
        .as_bool()
        .unwrap_or(false)
    {
        return Ok(None);
    }
    let render = Reflect::get(&module, &"render".into())
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(js_error)?;
    let result = render
        .call1(
            &JsValue::NULL,
            &js_sys::JSON::parse(&key).map_err(js_error)?,
        )
        .map_err(js_error)?;
    let result = js_sys::JSON::stringify(&result)
        .map_err(js_error)?
        .as_string()
        .ok_or("Generator returned no drawing")?;
    let result = Rc::new(forms::project(
        &serde_json::from_str(&result).map_err(|e| e.to_string())?,
    ));
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
pub(super) fn FootprintGraphics(
    definition: PartDefinition,
    parameters: Option<BTreeMap<String, serde_json::Value>>,
) -> Element {
    let graphics = use_resource(use_reactive(
        (&definition, &parameters),
        |(definition, parameters)| generator_drawings(definition, parameters, Some(false)),
    ));
    match &*graphics.read() {
        Some(Ok(Some(items))) => rsx! {
            g { class: "m1-footprint-graphics", "data-status": "ready", "aria-hidden": "true",
                for (index, graphic) in items.iter().enumerate() {
                    GraphicElement { key: "{index}", graphic: graphic.clone(), hidden: false }
                }
            }
        },
        Some(Ok(None)) => rsx! {
            g { class: "m1-footprint-graphics", "data-status": "ready", "aria-hidden": "true" }
        },
        Some(Err(error)) => rsx! {
            g { class: "m1-footprint-graphics-error", "data-status": "error",
                title { "Footprint graphics unavailable: {error}. Toggle Footprints to retry." }
                text { x: 0, y: 3, transform: "scale(1,-1)", "Graphics unavailable" }
            }
        },
        None => rsx! { g { class: "m1-footprint-graphics", "data-status": "loading" } },
    }
}

#[component]
pub(super) fn GraphicElement(graphic: Graphic, hidden: bool) -> Element {
    if hidden {
        return rsx! {};
    }
    let layer = graphic.layer;
    match graphic.shape {
        Shape::Line(a, b) => {
            rsx! { line { class: "m1-footprint-graphic", "data-layer": layer, x1: a.0, y1: a.1, x2: b.0, y2: b.1 } }
        }
        Shape::Arc(a, m, b) => {
            rsx! { path { class: "m1-footprint-graphic", "data-layer": layer, d: format!("M {} {} Q {} {} {} {}", a.0,a.1,2.0*m.0-(a.0+b.0)/2.0,2.0*m.1-(a.1+b.1)/2.0,b.0,b.1) } }
        }
        Shape::Rect(a, b) => {
            rsx! { rect { class: "m1-footprint-graphic", "data-layer": layer, x: a.0.min(b.0), y: a.1.min(b.1), width: (b.0-a.0).abs(), height: (b.1-a.1).abs() } }
        }
        Shape::Circle(c, r) => {
            rsx! { circle { class: "m1-footprint-graphic", "data-layer": layer, cx: c.0, cy: c.1, r } }
        }
        Shape::Polygon(points, keepout) => {
            rsx! { polygon { class: if keepout { "m1-footprint-keepout" } else { "m1-footprint-zone" }, "data-layer": layer, points: points.iter().map(|p|format!("{},{}",p.0,p.1)).collect::<Vec<_>>().join(" ") } }
        }
        Shape::Text(at, text) => {
            rsx! { text { class: "m1-footprint-text", "data-layer": layer, transform: "scale(1,-1)", x: at.0, y: -at.1, "{text}" } }
        }
    }
}
