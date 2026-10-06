//! Presentation projection of a generator's KiCad forms, in Y-up units.
use boardstudio_core::generators::Expr;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Point(pub f64, pub f64);

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Shape {
    Line(Point, Point),
    Arc(Point, Point, Point),
    Rect(Point, Point),
    Circle(Point, f64),
    Polygon(Vec<Point>, bool),
    Text(Point, String),
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Graphic {
    pub layer: String,
    pub shape: Shape,
}

fn child<'a>(form: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    boardstudio_core::generators::child(form, name)
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
                let (sin, cos) = (angle * portion).sin_cos();
                Point(
                    a.0 + (b.0 - a.0) * cos - (b.1 - a.1) * sin,
                    a.1 + (b.0 - a.0) * sin + (b.1 - a.1) * cos,
                )
            };
            Some(Shape::Arc(b.clone(), rotate(0.5), rotate(1.0)))
        }
    } else if kind.ends_with("_rect") {
        Some(Shape::Rect(start()?, end()?))
    } else if kind.ends_with("_circle") {
        let (center, end) = (point(child(form, "center"))?, end()?);
        let radius = (end.0 - center.0).hypot(end.1 - center.1);
        Some(Shape::Circle(center, radius))
    } else if kind.ends_with("_poly") || kind == "zone" {
        let points: Vec<_> = descendant(form, "pts")?
            .iter()
            .filter_map(Expr::as_list)
            .filter(|v| scalar(v, 0) == Some("xy"))
            .filter_map(|v| point(Some(v)))
            .collect();
        (points.len() >= 3).then(|| Shape::Polygon(points, child(form, "keepout").is_some()))
    } else if kind.ends_with("_text") {
        if form
            .iter()
            .any(|v| matches!(v, Expr::Atom(text) if text == "hide"))
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

pub(super) fn project(forms: &[Expr]) -> Vec<Graphic> {
    fn visit(node: &Expr, output: &mut Vec<Graphic>) {
        let Some(form) = node.as_list() else {
            return;
        };
        if let Some(kind) = scalar(form, 0)
            && [
                "fp_line",
                "gr_line",
                "fp_arc",
                "gr_arc",
                "fp_rect",
                "gr_rect",
                "fp_circle",
                "gr_circle",
                "fp_poly",
                "gr_poly",
                "fp_text",
                "gr_text",
                "zone",
            ]
            .contains(&kind)
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

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_core::generators::parse_forms;

    fn forms(source: &str) -> Vec<Expr> {
        parse_forms(source).expect("forms parse")
    }

    #[test]
    fn projects_nested_graphics_into_pad_coordinates_and_decodes_text() {
        let result = project(&forms(
            r#"(footprint part
                (fp_line (start 2 3) (end 4 -5) (layer "F.SilkS"))
                (fp_text reference REF** (at 0 0))
                (fp_text user "Hello <world>" (at 1 2))
                (fp_text user hidden (at 1 2) hide)
                (fp_line (start NaN 0) (end 1 2)))"#,
        ));
        assert_eq!(result.len(), 2);
        assert_eq!(
            result[0],
            Graphic {
                layer: "F.SilkS".into(),
                shape: Shape::Line(Point(2.0, -3.0), Point(4.0, 5.0))
            }
        );
        assert_eq!(
            result[1].shape,
            Shape::Text(Point(1.0, -2.0), "Hello <world>".into())
        );
    }

    #[test]
    fn projects_native_and_legacy_arcs_and_nested_keepouts() {
        let result = project(&forms(
            r#"(fp_arc (start 1 0) (mid 0 1) (end -1 0))
               (gr_arc (start 0 0) (end 1 0) (angle 180))
               (zone (keepout) (polygon (pts (xy 0 0) (xy 1 0) (xy 1 2))))"#,
        ));
        assert_eq!(
            result[0].shape,
            Shape::Arc(Point(1.0, 0.0), Point(0.0, -1.0), Point(-1.0, 0.0))
        );
        let Shape::Arc(start, mid, end) = &result[1].shape else {
            panic!("legacy arc missing")
        };
        assert_eq!(*start, Point(1.0, 0.0));
        assert!(mid.0.abs() < 1e-10 && (mid.1 + 1.0).abs() < 1e-10);
        assert!((end.0 + 1.0).abs() < 1e-10 && end.1.abs() < 1e-10);
        assert_eq!(
            result[2].shape,
            Shape::Polygon(
                vec![Point(0.0, 0.0), Point(1.0, 0.0), Point(1.0, -2.0)],
                true
            )
        );
    }
}
