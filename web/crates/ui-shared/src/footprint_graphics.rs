//! KiCad S-expression footprint projection (native + wasm) and SVG graphics rendering (wasm).
use boardstudio_core_kernel::generators::Expr;
use boardstudio_model::model::Side;
#[cfg(target_arch = "wasm32")]
use {
    crate::generator_preview_defaults::generator_preview_defaults_from_parameters,
    boardstudio_model::model::PartDefinition,
    dioxus::prelude::*,
    std::{
        cell::RefCell,
        collections::{BTreeMap, BTreeSet, VecDeque},
        rc::Rc,
    },
};

pub use crate::board_layer::resolve_board_layer;
#[cfg(target_arch = "wasm32")]
pub use crate::generator_preview_defaults::GeneratorPreviewDefaults;

#[derive(Clone, Debug, PartialEq)]
pub struct Point(pub f64, pub f64);

#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Line(Point, Point),
    Arc(Point, Point, Point),
    Rect(Point, Point),
    Circle(Point, f64),
    Polygon(Vec<Point>, bool),
    Text(Point, String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Graphic {
    pub layer: String,
    pub shape: Shape,
}

fn child<'a>(form: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    boardstudio_core_kernel::generators::child(form, name)
}

fn scalar(form: &[Expr], index: usize) -> Option<&str> {
    form.get(index)?.value().ok()
}

fn number(form: &[Expr], index: usize) -> Option<f64> {
    scalar(form, index)?
        .parse::<f64>()
        .ok()
        .filter(|v| v.is_finite())
}

fn point(form: Option<&[Expr]>) -> Option<Point> {
    let form = form?;
    Some(Point(number(form, 1)?, -number(form, 2)?))
}

fn descendant<'a>(form: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    child(form, name).or_else(|| {
        form.iter()
            .filter_map(Expr::as_list)
            .find_map(|v| descendant(v, name))
    })
}

fn shape(form: &[Expr], kind: &str) -> Option<Shape> {
    let start = || point(child(form, "start"));
    let end = || point(child(form, "end"));
    if kind.ends_with("_line") {
        Some(Shape::Line(start()?, end()?))
    } else if kind.ends_with("_arc") {
        let (a, b) = (start()?, end()?);
        if let Some(mid) = point(child(form, "mid")) {
            Some(Shape::Arc(a, mid, b))
        } else {
            let angle = -number(child(form, "angle")?, 1)?.to_radians();
            let rotate = |portion: f64| {
                let (s, c) = (angle * portion).sin_cos();
                Point(
                    a.0 + (b.0 - a.0) * c - (b.1 - a.1) * s,
                    a.1 + (b.0 - a.0) * s + (b.1 - a.1) * c,
                )
            };
            Some(Shape::Arc(b.clone(), rotate(0.5), rotate(1.0)))
        }
    } else if kind.ends_with("_rect") {
        Some(Shape::Rect(start()?, end()?))
    } else if kind.ends_with("_circle") {
        let (c, e) = (point(child(form, "center"))?, end()?);
        Some(Shape::Circle(c.clone(), (e.0 - c.0).hypot(e.1 - c.1)))
    } else if kind.ends_with("_poly") || kind == "zone" {
        let pts: Vec<_> = descendant(form, "pts")?
            .iter()
            .filter_map(Expr::as_list)
            .filter(|v| scalar(v, 0) == Some("xy"))
            .filter_map(|v| point(Some(v)))
            .collect();
        (pts.len() >= 3).then(|| Shape::Polygon(pts, child(form, "keepout").is_some()))
    } else if kind.ends_with("_text") {
        if form
            .iter()
            .any(|v| matches!(v, Expr::Atom(t) if t == "hide"))
            || (kind == "fp_text" && scalar(form, 1) == Some("reference"))
        {
            return None;
        }
        let text = scalar(form, if kind == "fp_text" { 2 } else { 1 })?;
        (!text.is_empty()).then(|| Some(Shape::Text(point(child(form, "at"))?, text.to_owned())))?
    } else {
        None
    }
}

pub fn project(forms: &[Expr]) -> Vec<Graphic> {
    fn visit(node: &Expr, output: &mut Vec<Graphic>) {
        let Some(form) = node.as_list() else { return };
        if let Some(kind) = scalar(form, 0)
            && matches!(
                kind,
                "fp_line"
                    | "gr_line"
                    | "fp_arc"
                    | "gr_arc"
                    | "fp_rect"
                    | "gr_rect"
                    | "fp_circle"
                    | "gr_circle"
                    | "fp_poly"
                    | "gr_poly"
                    | "fp_text"
                    | "gr_text"
                    | "zone"
            )
            && let Some(shape) = shape(form, kind)
        {
            output.push(Graphic {
                layer: child(form, "layer")
                    .and_then(|v| scalar(v, 1))
                    .unwrap_or("Other graphics")
                    .to_owned(),
                shape,
            });
        }
        for item in form {
            visit(item, output);
        }
    }
    let mut output = Vec::new();
    for form in forms {
        visit(form, &mut output);
    }
    output
}

#[cfg(target_arch = "wasm32")]
pub type Drawings = Rc<Vec<Graphic>>;

#[cfg(target_arch = "wasm32")]
thread_local! {
    static CACHE: RefCell<VecDeque<(String, Drawings)>> = const { RefCell::new(VecDeque::new()) };
}

#[cfg(target_arch = "wasm32")]
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

#[cfg(target_arch = "wasm32")]
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
    let result = Rc::new(project(&rendered));
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if cache.len() >= 64 {
            cache.pop_front();
        }
        cache.push_back((key, result.clone()));
    });
    Ok(Some(result))
}

#[cfg(target_arch = "wasm32")]
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
                        let hidden = hidden_layers.as_ref().zip(part_side.as_ref()).is_some_and(|(h, s)| h.contains(&resolve_board_layer(&graphic.layer, s)));
                        rsx! { GraphicElement { key: "{index}", graphic: graphic.clone(), hidden } }
                    }
                }
            }
        },
        Some(Ok(None)) => rsx! { g { "data-status": "ready", "aria-hidden": "true" } },
        Some(Err(error)) => rsx! {
            g { class: "fill-destructive [font:1.5px_var(--theme-font-sans)] pointer-events-none", "data-status": "error",
                title { "Footprint graphics unavailable: {error}. Toggle Footprints to retry." }
                text { x: 0, y: 3, transform: "scale(1,-1)", "Graphics unavailable" }
            }
        },
        None => rsx! { g { "data-status": "loading" } },
    }
}

#[cfg(target_arch = "wasm32")]
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
            rsx! { path { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, d: format!("M {} {} Q {} {} {} {}", a.0, a.1, 2.0 * m.0 - (a.0 + b.0) / 2.0, 2.0 * m.1 - (a.1 + b.1) / 2.0, b.0, b.1) } }
        }
        Shape::Rect(a, b) => {
            rsx! { rect { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, x: a.0.min(b.0), y: a.1.min(b.1), width: (b.0 - a.0).abs(), height: (b.1 - a.1).abs() } }
        }
        Shape::Circle(c, r) => {
            rsx! { circle { class: "fill-none stroke-geometry-graphic [stroke-width:0.15] [stroke-linecap:round] [stroke-linejoin:round] pointer-events-none", "data-layer": layer, cx: c.0, cy: c.1, r } }
        }
        Shape::Polygon(pts, keepout) => {
            rsx! { polygon { class: if keepout { "fill-none stroke-geometry-graphic [stroke-width:0.12] [stroke-dasharray:0.5_0.5] pointer-events-none" } else { "fill-geometry-zone stroke-geometry-graphic [stroke-width:0.12] pointer-events-none" }, "data-layer": layer, points: pts.iter().map(|p| format!("{},{}", p.0, p.1)).collect::<Vec<_>>().join(" ") } }
        }
        Shape::Text(at, text) => {
            rsx! { text { class: "fill-geometry-label [font:0.8px_var(--theme-font-mono)] pointer-events-none", "data-layer": layer, transform: "scale(1,-1)", x: at.0, y: -at.1, "{text}" } }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core_kernel::generators::parse_forms;

    #[test]
    fn projects_nested_graphics_arcs_text_and_keepouts() {
        let forms = parse_forms(
            r#"(footprint part
                (fp_line (start 2 3) (end 4 -5) (layer "F.SilkS"))
                (fp_text reference REF** (at 0 0))
                (fp_text user "Hello" (at 1 2))
                (fp_text user hidden (at 1 2) hide)
                (fp_line (start NaN 0) (end 1 2))
                (fp_arc (start 1 0) (mid 0 1) (end -1 0))
                (gr_arc (start 0 0) (end 1 0) (angle 180))
                (zone (keepout) (polygon (pts (xy 0 0) (xy 1 0) (xy 1 2)))))"#,
        )
        .unwrap();
        let res = project(&forms);
        assert_eq!(
            res[0],
            Graphic {
                layer: "F.SilkS".into(),
                shape: Shape::Line(Point(2.0, -3.0), Point(4.0, 5.0))
            }
        );
        assert_eq!(res[1].shape, Shape::Text(Point(1.0, -2.0), "Hello".into()));
        assert_eq!(
            res[2].shape,
            Shape::Arc(Point(1.0, 0.0), Point(0.0, -1.0), Point(-1.0, 0.0))
        );
        assert!(matches!(res[3].shape, Shape::Arc(_, _, _)));
        assert!(matches!(res[4].shape, Shape::Polygon(ref pts, true) if pts.len() == 3));
    }
}
