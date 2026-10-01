//! Small helpers over kiutils_sexpr's span-bearing CST.
//!
//! The CST is deliberately kept private to the artifact implementation. Source
//! spans, rather than a pretty-printer, are the authority for all edits.

use kiutils_sexpr::{Atom, Node, Span};

pub(super) fn items(node: &Node) -> Option<&[Node]> {
    match node {
        Node::List { items, .. } => Some(items),
        Node::Atom { .. } => None,
    }
}

pub(super) fn head(node: &Node) -> Option<&str> {
    items(node)
        .and_then(|children| children.first())
        .and_then(atom)
}

pub(super) fn atom(node: &Node) -> Option<&str> {
    match node {
        Node::Atom {
            atom: Atom::Symbol(value) | Atom::Quoted(value),
            ..
        } => Some(value),
        Node::List { .. } => None,
    }
}

pub(super) fn children<'a>(node: &'a Node, name: &str) -> impl Iterator<Item = &'a Node> {
    items(node)
        .into_iter()
        .flatten()
        .filter(move |child| head(child) == Some(name))
}

pub(super) fn child<'a>(node: &'a Node, name: &str) -> Option<&'a Node> {
    children(node, name).next()
}

pub(super) fn span(node: &Node) -> Span {
    match node {
        Node::List { span, .. } | Node::Atom { span, .. } => *span,
    }
}

/// Decode the C-style escapes accepted by KiCad's DSN lexer. This deliberately
/// operates on the raw source span because the generic CST decoder drops the
/// slash from unknown escapes.
pub(super) fn kicad_quoted(raw: &str, span: Span) -> Option<String> {
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

pub(super) fn quote(value: &str) -> String {
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

pub(super) fn replace_spans(source: &str, mut replacements: Vec<(Span, String)>) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

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
