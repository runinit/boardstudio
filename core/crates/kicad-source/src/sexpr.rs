//! Small helpers over kiutils_sexpr's span-bearing CST.
//!
//! The CST is deliberately kept private to the artifact implementation. Source
//! spans, rather than a pretty-printer, are the authority for all edits.
//!
//! This module also hosts the shared arc-support and loop-chaining math used
//! by the footprint, board, and mechanical projections so they cannot drift.

use crate::model::Vec2;
use kiutils_sexpr::{Atom, Node, Span};

pub fn items(node: &Node) -> Option<&[Node]> {
    match node {
        Node::List { items, .. } => Some(items),
        Node::Atom { .. } => None,
    }
}

pub fn head(node: &Node) -> Option<&str> {
    items(node)
        .and_then(|children| children.first())
        .and_then(atom)
}

pub fn atom(node: &Node) -> Option<&str> {
    match node {
        Node::Atom {
            atom: Atom::Symbol(value) | Atom::Quoted(value),
            ..
        } => Some(value),
        Node::List { .. } => None,
    }
}

pub fn children<'a>(node: &'a Node, name: &str) -> impl Iterator<Item = &'a Node> {
    items(node)
        .into_iter()
        .flatten()
        .filter(move |child| head(child) == Some(name))
}

pub fn child<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    children(node, name).next()
}

pub fn atom_at(node: &Node, index: usize) -> Option<&str> {
    atom(items(node)?.get(index)?)
}

pub fn field<'a>(node: &'a Node, name: &str) -> Option<&'a str> {
    atom_at(child(node, name)?, 1)
}

pub fn is_back_layer(node: &Node) -> bool {
    field(node, "layer").is_some_and(|layer| layer.starts_with("B."))
}

#[derive(Debug)]
pub enum NumberError {
    Invalid,
    NonFinite,
}

pub fn parse_finite(value: &str) -> Result<f64, NumberError> {
    let number: f64 = value.parse().map_err(|_| NumberError::Invalid)?;
    if !number.is_finite() {
        return Err(NumberError::NonFinite);
    }
    Ok(number)
}

/// KiCad patches use six fractional digits and retain the sign when a nonzero
/// negative number rounds to zero.
pub fn format_decimal(value: f64) -> String {
    if value == 0.0 {
        return "0".into();
    }
    format!("{value:.6}")
        .trim_end_matches('0')
        .trim_end_matches('.')
        .to_owned()
}

pub fn span(node: &Node) -> Span {
    match node {
        Node::List { span, .. } | Node::Atom { span, .. } => *span,
    }
}

/// Decode a text atom from the raw source span, falling back to the CST text
/// for unquoted atoms. Quoted values must be re-decoded from source because
/// the generic CST decoder drops the slash from unknown escapes.
pub fn decoded_text(source: &str, node: &Node) -> Option<String> {
    kicad_quoted(source, span(node)).or_else(|| atom(node).map(str::to_owned))
}

/// Decode the C-style escapes accepted by KiCad's DSN lexer. This deliberately
/// operates on the raw source span because the generic CST decoder drops the
/// slash from unknown escapes.
pub fn kicad_quoted(raw: &str, span: Span) -> Option<String> {
    let token = raw.get(span.start..span.end)?;
    if !token.starts_with('"') || !token.ends_with('"') || token.len() < 2 {
        return None;
    }
    let body = &token[1..token.len() - 1];
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        let escaped = chars.next()?;
        match escaped {
            'a' => out.push('\u{7}'),
            'b' => out.push('\u{8}'),
            'f' => out.push('\u{c}'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            't' => out.push('\t'),
            'v' => out.push('\u{b}'),
            '"' | '\\' => out.push(escaped),
            'u' => {
                let mut value = 0u32;
                let mut count = 0;
                while count < 4 {
                    let Some(next) = chars.clone().next() else {
                        break;
                    };
                    let Some(digit) = next.to_digit(16) else {
                        break;
                    };
                    chars.next();
                    value = value * 16 + digit;
                    count += 1;
                }
                if count == 4 {
                    if let Some(ch) = char::from_u32(value) {
                        out.push(ch);
                    }
                } else {
                    out.push('u');
                }
            }
            'x' => {
                let mut value = 0u32;
                let mut count = 0;
                while count < 2 {
                    let Some(next) = chars.clone().next() else {
                        break;
                    };
                    let Some(digit) = next.to_digit(16) else {
                        break;
                    };
                    chars.next();
                    value = value * 16 + digit;
                    count += 1;
                }
                if count == 0 {
                    out.push('x');
                } else if let Some(ch) = char::from_u32(value) {
                    out.push(ch);
                }
            }
            oct @ '0'..='7' => {
                let mut value = oct.to_digit(8).unwrap_or(0);
                for _ in 0..2 {
                    let Some(next) = chars.clone().next() else {
                        break;
                    };
                    let Some(digit) = next.to_digit(8) else { break };
                    chars.next();
                    value = value * 8 + digit;
                }
                if let Some(ch) = char::from_u32(value) {
                    out.push(ch);
                }
            }
            other => {
                out.push('\\');
                out.push(other);
            }
        }
    }
    Some(out)
}

pub fn quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '"' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '\u{7}' => out.push_str("\\a"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{b}' => out.push_str("\\v"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

pub fn replace_spans(source: &str, mut replacements: Vec<(Span, String)>) -> Option<String> {
    replacements.sort_by_key(|(span, _)| span.start);
    let mut output = String::with_capacity(source.len());
    let mut cursor = 0;
    for (span, value) in replacements {
        if span.start < cursor || span.end > source.len() || span.start > span.end {
            return None;
        }
        output.push_str(source.get(cursor..span.start)?);
        output.push_str(&value);
        cursor = span.end;
    }
    output.push_str(source.get(cursor..)?);
    Some(output)
}

/// Circumcenter and radius of the circle through three points.
///
/// Returns `None` when the doubled signed area falls below `degenerate_below`;
/// near-collinear inputs have no usable circular support.
pub fn circumcenter(a: Vec2, b: Vec2, c: Vec2, degenerate_below: f64) -> Option<(Vec2, f64)> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < degenerate_below {
        return None;
    }
    let aa = a.x * a.x + a.y * a.y;
    let bb = b.x * b.x + b.y * b.y;
    let cc = c.x * c.x + c.y * c.y;
    let center = Vec2 {
        x: (aa * (b.y - c.y) + bb * (c.y - a.y) + cc * (a.y - b.y)) / d,
        y: (aa * (c.x - b.x) + bb * (a.x - c.x) + cc * (b.x - a.x)) / d,
    };
    Some((center, (a.x - center.x).hypot(a.y - center.y)))
}

/// Start angle and signed sweep of the arc from `start` to `end` around
/// `center` passing through `mid`.
pub fn arc_sweep_through_mid(center: Vec2, start: Vec2, mid: Vec2, end: Vec2) -> (f64, f64) {
    let angle = |point: Vec2| (point.y - center.y).atan2(point.x - center.x);
    let first = angle(start);
    let middle = angle(mid);
    let last = angle(end);
    let ccw = (middle - first).rem_euclid(std::f64::consts::TAU)
        <= (last - first).rem_euclid(std::f64::consts::TAU);
    let sweep = if ccw {
        (last - first).rem_euclid(std::f64::consts::TAU)
    } else {
        -((first - last).rem_euclid(std::f64::consts::TAU))
    };
    (first, sweep)
}

/// Circular support in the input frame with the caller's degeneracy threshold.
pub fn three_point_arc(
    start: Vec2,
    mid: Vec2,
    end: Vec2,
    threshold: f64,
) -> Option<(Vec2, f64, f64, f64)> {
    let (center, radius) = circumcenter(start, mid, end, threshold)?;
    let (first, sweep) = arc_sweep_through_mid(center, start, mid, end);
    Some((center, first, sweep, radius))
}

/// Sample a circular arc using the caller's subdivision count.
pub fn sample_arc(center: Vec2, start: f64, sweep: f64, radius: f64, steps: usize) -> Vec<Vec2> {
    (0..=steps)
        .map(|i| {
            let angle = start + sweep * i as f64 / steps as f64;
            Vec2 {
                x: center.x + radius * angle.cos(),
                y: center.y + radius * angle.sin(),
            }
        })
        .collect()
}

/// Extend `chain` with adjoining edges until it closes, consuming them from
/// `edges`. Edges may be shuffled or reversed; endpoints within `tolerance`
/// join. Returns `None` when an open chain has no adjoining edge left.
pub fn chain_loop_from(
    mut chain: Vec<Vec2>,
    edges: &mut Vec<Vec<Vec2>>,
    tolerance: f64,
) -> Option<Vec<Vec2>> {
    let first = *chain.first()?;
    while !within(*chain.last()?, first, tolerance) {
        let current = *chain.last()?;
        let index = edges.iter().position(|edge| {
            let (Some(a), Some(b)) = (edge.first(), edge.last()) else {
                return false;
            };
            within(*a, current, tolerance) || within(*b, current, tolerance)
        })?;
        let mut edge = edges.remove(index);
        if within(*edge.last()?, current, tolerance) {
            edge.reverse();
        }
        chain.extend(edge.into_iter().skip(1));
    }
    Some(chain)
}

fn within(a: Vec2, b: Vec2, tolerance: f64) -> bool {
    (a.x - b.x).hypot(a.y - b.y) <= tolerance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_sample_preserves_the_projection_evaluation_order() {
        // Forming the subdivision fraction first changes the first sample's Y.
        let (start, sweep, steps) = (0.01_f64, 1.1_f64, 7);
        let angle = start + sweep / steps as f64;
        let points = sample_arc(Vec2::default(), start, sweep, 1.0, steps);
        assert_eq!(
            points[1],
            Vec2 {
                x: angle.cos(),
                y: angle.sin()
            }
        );
    }

    #[test]
    fn kicad_escape_decoder_handles_c_escapes_and_unicode() {
        let source =
            r#"(property "Value" "quote: \" slash: \\ newline: \n tab: \t \x41 \101 café")"#;
        let doc = kiutils_sexpr::parse_one(source).expect("valid source");
        let node = &items(&doc.nodes[0]).expect("root")[2];
        assert_eq!(
            kicad_quoted(source, span(node)).as_deref(),
            Some("quote: \" slash: \\ newline: \n tab: \t A A café")
        );
    }

    #[test]
    fn kicad_escape_decoder_reads_json_unicode_escapes() {
        let source = r#"(property "Value" "control: \u0001 accent: \u00e9")"#;
        let doc = kiutils_sexpr::parse_one(source).expect("valid source");
        let node = &items(&doc.nodes[0]).expect("root")[2];
        assert_eq!(
            kicad_quoted(source, span(node)).as_deref(),
            Some("control: \u{1} accent: é")
        );
    }

    #[test]
    fn unchanged_source_spans_remain_byte_identical() {
        let source = "(footprint \"café\"\n  (property \"Value\" \"custom\\nvalue\")\n  (unknown \"keep me\")\n)";
        let doc = kiutils_sexpr::parse_one(source).expect("valid source");
        let unknown = children(&doc.nodes[0], "unknown")
            .next()
            .expect("unknown metadata");
        let raw_unknown = &source[span(unknown).start..span(unknown).end];
        let changed = replace_spans(
            source,
            vec![(
                span(&items(&doc.nodes[0]).unwrap()[2]),
                "(property \"Value\" \"new\")".into(),
            )],
        )
        .unwrap();
        assert!(changed.contains(raw_unknown));
        assert!(!changed.contains("(property \"Value\" \"custom\\nvalue\")"));
    }
}
