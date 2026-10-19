//! Build KiCad forms as `Expr` trees directly, so a generator never renders
//! text that the registry parses back. Numbers take `js_number` spelling where
//! they are created; items keep the order they are written in.
use crate::context::RenderContext;
use crate::number::js_number;
use crate::sexpr::Expr;

/// `(head items...)`.
pub fn form(head: &str, items: impl IntoIterator<Item = Expr>) -> Expr {
    Expr::List(std::iter::once(Expr::atom(head)).chain(items).collect())
}

pub fn num(value: f64) -> Expr {
    Expr::Atom(js_number(value))
}

pub fn atom(text: &str) -> Expr {
    Expr::atom(text)
}

pub fn string(text: &str) -> Expr {
    Expr::string(text)
}

/// `(head n...)` for a run of numbers, e.g. `(size 1 1)`.
pub fn nums(head: &str, values: &[f64]) -> Expr {
    form(head, values.iter().map(|&v| num(v)))
}

/// `(at x y r)` for a local offset.
pub fn at(x: f64, y: f64, rotation: f64) -> Expr {
    nums("at", &[x, y, rotation])
}

/// The part's own placement, `(at x y r)` in KiCad's downward Y.
pub fn placement(p: &RenderContext<'_>) -> Expr {
    at(p.x(), p.y(), p.rotation())
}

/// `(layer "<side>.<name>")`.
pub fn layer(p: &RenderContext<'_>, name: &str) -> Expr {
    form("layer", [string(&format!("{}.{name}", p.side()))])
}

/// `(effects (font (size s s) (thickness t)))`.
pub fn effects(size: f64, thickness: f64) -> Expr {
    form(
        "effects",
        [form("font", [nums("size", &[size, size]), nums("thickness", &[thickness])])],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sexpr::{parse_forms, serialize};

    #[test]
    fn builds_the_same_tree_the_text_path_parses() {
        let built = form(
            "pad",
            [string(""), atom("np_thru_hole"), at(0.0, 0.5, 90.0), effects(1.0, 0.15)],
        );
        let parsed =
            parse_forms(r#"(pad "" np_thru_hole (at 0 0.5 90) (effects (font (size 1 1) (thickness 0.15))))"#)
                .unwrap();
        assert_eq!(built, parsed[0]);
        assert_eq!(serialize(&built), serialize(&parsed[0]));
    }
}
