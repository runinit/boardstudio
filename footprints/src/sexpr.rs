//! The KiCad S-expression tree generators emit. A quoted string and a bare atom
//! are different nodes: some checks (pad `layers`, `shape`) only match bare atoms
//! and the output keeps its quoting.
use crate::error::{GeneratorError, Result};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Atom(String),
    Str(String),
    List(Vec<Expr>),
}

impl Expr {
    pub fn atom(text: &str) -> Self {
        Self::Atom(text.to_owned())
    }

    pub fn string(text: &str) -> Self {
        Self::Str(text.to_owned())
    }

    pub fn as_list(&self) -> Option<&[Expr]> {
        match self {
            Self::List(items) => Some(items),
            _ => None,
        }
    }

    /// The scalar text of an atom or string; an error for lists.
    pub fn value(&self) -> Result<&str> {
        match self {
            Self::Atom(text) | Self::Str(text) => Ok(text),
            Self::List(_) => Err(scalar_error()),
        }
    }
}

pub(crate) fn scalar_error() -> GeneratorError {
    GeneratorError::Geometry("Expected Ergogen scalar".into())
}

/// A list whose first item is the bare atom `name`.
fn is_named(node: &Expr, name: &str) -> bool {
    matches!(node, Expr::List(items) if matches!(items.first(), Some(Expr::Atom(head)) if head == name))
}

/// The first child list named `name`.
pub fn child<'a>(node: &'a [Expr], name: &str) -> Option<&'a [Expr]> {
    node.iter()
        .find(|item| is_named(item, name))
        .and_then(Expr::as_list)
}

pub fn children<'a>(node: &'a [Expr], name: &'a str) -> impl Iterator<Item = &'a [Expr]> {
    node.iter()
        .filter(move |item| is_named(item, name))
        .filter_map(Expr::as_list)
}

/// JavaScript `\s`.
fn is_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

/// JavaScript `.` does not match line terminators.
fn is_line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

#[derive(Debug)]
enum Token {
    Open,
    Close,
    Quoted(String),
    Bare(String),
}

/// `/\s*(\(|\)|"(?:\\.|[^"\\])*"|[^\s()]+)/y`, repeated. A string without a
/// closing quote falls back to a bare run starting at the quote.
fn tokens(source: &str) -> Result<Vec<Token>> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        while i < chars.len() && is_space(chars[i]) {
            i += 1;
        }
        if i >= chars.len() {
            break;
        }
        match chars[i] {
            '(' => {
                tokens.push(Token::Open);
                i += 1;
            }
            ')' => {
                tokens.push(Token::Close);
                i += 1;
            }
            '"' => {
                let mut j = i + 1;
                let mut closed = false;
                while j < chars.len() {
                    match chars[j] {
                        '"' => {
                            closed = true;
                            break;
                        }
                        '\\' if j + 1 < chars.len() && !is_line_terminator(chars[j + 1]) => j += 2,
                        '\\' => break,
                        _ => j += 1,
                    }
                }
                if closed {
                    tokens.push(Token::Quoted(chars[i..=j].iter().collect()));
                    i = j + 1;
                } else {
                    let mut j = i;
                    while j < chars.len()
                        && !is_space(chars[j])
                        && chars[j] != '('
                        && chars[j] != ')'
                    {
                        j += 1;
                    }
                    tokens.push(Token::Bare(chars[i..j].iter().collect()));
                    i = j;
                }
            }
            _ => {
                let mut j = i;
                while j < chars.len() && !is_space(chars[j]) && chars[j] != '(' && chars[j] != ')' {
                    j += 1;
                }
                tokens.push(Token::Bare(chars[i..j].iter().collect()));
                i = j;
            }
        }
    }
    Ok(tokens)
}

fn unbalanced() -> GeneratorError {
    GeneratorError::Parse("Unbalanced Ergogen output".into())
}

fn string_literal(token: &str) -> Result<String> {
    serde_json::from_str::<String>(token)
        .map_err(|_| GeneratorError::Parse(format!("Invalid Ergogen string literal {token}")))
}

/// Parse generator output into top-level forms.
pub fn parse_forms(source: &str) -> Result<Vec<Expr>> {
    fn read(tokens: &[Token], cursor: &mut usize) -> Result<Expr> {
        let token = tokens.get(*cursor).ok_or_else(unbalanced)?;
        *cursor += 1;
        match token {
            Token::Close => Err(unbalanced()),
            Token::Quoted(text) => Ok(Expr::Str(string_literal(text)?)),
            Token::Bare(text) if text.starts_with('"') => Ok(Expr::Str(string_literal(text)?)),
            Token::Bare(text) => Ok(Expr::Atom(text.clone())),
            Token::Open => {
                let mut items = Vec::new();
                loop {
                    match tokens.get(*cursor) {
                        None => return Err(unbalanced()),
                        Some(Token::Close) => break,
                        Some(_) => items.push(read(tokens, cursor)?),
                    }
                }
                *cursor += 1;
                Ok(Expr::List(items))
            }
        }
    }
    let tokens = tokens(source)?;
    let mut cursor = 0;
    let mut forms = Vec::new();
    while cursor < tokens.len() {
        match read(&tokens, &mut cursor)? {
            form @ Expr::List(_) => forms.push(form),
            _ => {
                return Err(GeneratorError::Parse(
                    "Ergogen output must contain KiCad forms".into(),
                ));
            }
        }
    }
    Ok(forms)
}

/// Serialize a node back to text; strings use JSON quoting.
pub fn serialize(node: &Expr) -> String {
    match node {
        Expr::List(items) => {
            let inner: Vec<String> = items.iter().map(serialize).collect();
            format!("({})", inner.join(" "))
        }
        Expr::Str(text) => serde_json::to_string(text).expect("strings serialize"),
        Expr::Atom(text) => text.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_forms_and_keeps_quoting() {
        let forms = parse_forms(r#"(a (b "c d" e) "f\"g") (h)"#).unwrap();
        assert_eq!(forms.len(), 2);
        assert_eq!(serialize(&forms[0]), r#"(a (b "c d" e) "f\"g")"#);
        let list = forms[0].as_list().unwrap();
        assert_eq!(list[2], Expr::string("f\"g"));
        assert_eq!(child(list, "b").unwrap()[1], Expr::string("c d"));
    }

    #[test]
    fn rejects_unbalanced_and_non_form_output() {
        for source in ["(footprint", ")", "(a (b)"] {
            assert_eq!(
                parse_forms(source).unwrap_err().message(),
                "Unbalanced Ergogen output"
            );
        }
        for source in ["footprint", "\"standalone\""] {
            assert_eq!(
                parse_forms(source).unwrap_err().message(),
                "Ergogen output must contain KiCad forms"
            );
        }
        assert!(parse_forms("   ").unwrap().is_empty());
    }

    #[test]
    fn an_unterminated_string_is_not_a_valid_literal() {
        assert!(matches!(
            parse_forms("(a \"b)"),
            Err(GeneratorError::Parse(_))
        ));
    }
}
