//! Borrow numeric spellings from already parsed STEP bytes; never rebuild entity ASTs.

pub(in crate::step) fn separators(mut input: &[u8]) -> Option<&[u8]> {
    loop {
        input = input.trim_ascii_start();
        if !input.starts_with(b"/*") {
            return Some(input);
        }
        let end = input[2..].windows(2).position(|pair| pair == b"*/")?;
        input = &input[end + 4..];
    }
}

pub(in crate::step) fn string(input: &[u8]) -> Option<&[u8]> {
    let mut offset = 1;
    if input.first() != Some(&b'\'') {
        return None;
    }
    while offset < input.len() {
        if input[offset] == b'\'' {
            if input.get(offset + 1) != Some(&b'\'') {
                return Some(&input[offset + 1..]);
            }
            offset += 1;
        }
        offset += 1;
    }
    None
}

pub(in crate::step) fn reference(input: &[u8]) -> Option<(&[u8], u64)> {
    let input = input.strip_prefix(b"#")?;
    let length = input
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let id = std::str::from_utf8(&input[..length]).ok()?.parse().ok()?;
    Some((&input[length..], id))
}

pub(in crate::step) fn number(input: &[u8]) -> Option<(&[u8], &str)> {
    let mut offset = usize::from(matches!(input.first(), Some(b'+' | b'-')));
    let start = offset;
    while input.get(offset).is_some_and(u8::is_ascii_digit) {
        offset += 1;
    }
    if offset == start {
        return None;
    }
    if input.get(offset) == Some(&b'.') {
        offset += 1;
        while input.get(offset).is_some_and(u8::is_ascii_digit) {
            offset += 1;
        }
        if input.get(offset) == Some(&b'E') {
            offset += 1;
            offset += usize::from(matches!(input.get(offset), Some(b'+' | b'-')));
            let start = offset;
            while input.get(offset).is_some_and(u8::is_ascii_digit) {
                offset += 1;
            }
            if offset == start {
                return None;
            }
        }
    }
    Some((
        &input[offset..],
        std::str::from_utf8(&input[..offset]).ok()?,
    ))
}

pub(in crate::step) struct Instance<'a> {
    pub id: u64,
    pub kind: &'a [u8],
    pub parameters: &'a [u8],
}

fn instance(input: &[u8]) -> Option<(&[u8], Instance<'_>)> {
    let (rest, id) = reference(input)?;
    let rest = separators(rest)?.strip_prefix(b"=")?;
    let rest = separators(rest)?;
    let kind_len = rest
        .iter()
        .take_while(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit() || **byte == b'_')
        .count();
    let kind = &rest[..kind_len];
    let parameters = &rest[kind_len..];
    let mut rest = parameters;
    let mut depth = 0_usize;
    loop {
        rest = separators(rest)?;
        match rest.first()? {
            b'\'' => rest = string(rest)?,
            b'(' => {
                depth = depth.checked_add(1)?;
                rest = &rest[1..];
            }
            b')' => {
                depth = depth.checked_sub(1)?;
                rest = &rest[1..];
            }
            b';' if depth == 0 => {
                let parameters = &parameters[..parameters.len() - rest.len()];
                return Some((
                    &rest[1..],
                    Instance {
                        id,
                        kind,
                        parameters,
                    },
                ));
            }
            b';' => return None,
            _ => rest = &rest[1..],
        }
    }
}

/// Entity instances of the one DATA section, located by the exchange structure
/// the parser accepts (ISO 10303-21 `exchange_file`). Header records, ANCHOR
/// names, REFERENCE resources, comments, strings and trailing signatures never
/// supply an instance. Any structure this walk cannot follow yields one `None`.
pub(in crate::step) fn instances(input: &[u8]) -> impl Iterator<Item = Option<Instance<'_>>> {
    let mut body = data_body(input).ok_or(());
    std::iter::from_fn(move || {
        let rest = match body {
            Ok(rest) => rest,
            Err(()) => {
                body = Ok(&[]);
                return Some(None);
            }
        };
        let next = separators(rest)?;
        if next.is_empty() {
            return None;
        }
        let Some((rest, instance)) = instance(next) else {
            body = Ok(&[]);
            return Some(None);
        };
        body = Ok(rest);
        Some(Some(instance))
    })
}

fn data_body(input: &[u8]) -> Option<&[u8]> {
    let mut rest = token(input, b"ISO-10303-21;")?;
    rest = statements(token(rest, b"HEADER;")?)?;
    for section in [&b"ANCHOR;"[..], b"REFERENCE;"] {
        if let Some(inner) = token(rest, section) {
            rest = statements(inner)?;
        }
    }
    rest = separators(token(rest, b"DATA")?)?;
    if let Some(meta) = rest.strip_prefix(b"(") {
        rest = statement(meta, b')')?;
    }
    let start = token(rest, b";")?;
    rest = start;
    loop {
        let next = separators(rest)?;
        if let Some(after) = next.strip_prefix(b"ENDSEC;") {
            // A second DATA section is not this single-section grammar.
            token(after, b"END-ISO-10303-21;")?;
            return Some(&start[..start.len() - next.len()]);
        }
        rest = instance(next)?.0;
    }
}

fn token<'a>(input: &'a [u8], keyword: &[u8]) -> Option<&'a [u8]> {
    separators(input)?.strip_prefix(keyword)
}

/// Header, ANCHOR and REFERENCE statements through their `ENDSEC;`.
fn statements(mut input: &[u8]) -> Option<&[u8]> {
    loop {
        let next = separators(input)?;
        if let Some(rest) = next.strip_prefix(b"ENDSEC;") {
            return Some(rest);
        }
        input = statement(next, b';')?;
    }
}

/// Skips to just past `end` at depth zero. A resource or anchor name runs to
/// its first `>` and a string to its closing quote, whatever they contain.
fn statement(mut input: &[u8], end: u8) -> Option<&[u8]> {
    let mut depth = 0_usize;
    loop {
        input = separators(input)?;
        match *input.first()? {
            b'\'' => input = string(input)?,
            b'<' => {
                let close = input.iter().position(|byte| *byte == b'>')?;
                input = &input[close + 1..];
            }
            byte if byte == end && depth == 0 => return Some(&input[1..]),
            b'(' => {
                depth = depth.checked_add(1)?;
                input = &input[1..];
            }
            b')' => {
                depth = depth.checked_sub(1)?;
                input = &input[1..];
            }
            b';' => return None,
            _ => input = &input[1..],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(text: &str) -> Option<Vec<u64>> {
        instances(text.as_bytes())
            .map(|instance| instance.map(|instance| instance.id))
            .collect()
    }

    #[test]
    fn only_the_data_section_supplies_instances() {
        let smuggled = "#2=CARTESIAN_POINT('',(9.,0.,0.));";
        let text = format!(
            "ISO-10303-21;HEADER;/* {smuggled} */FILE_DESCRIPTION(('{smuggled}'),'2;1');\
             FILE_NAME('a','','',(),'','','');FILE_SCHEMA(('X'));ENDSEC;\
             ANCHOR;<{smuggled}>=#1{{t:'{smuggled}'}};<b>=(1,'x;y',$);ENDSEC;\
             REFERENCE;#900=<{smuggled}>;ENDSEC;\
             DATA(('meta;{smuggled}'));/* {smuggled} */#1=CARTESIAN_POINT('{smuggled}',(0.,0.,0.));\
             #3=(A()B('{smuggled}'));ENDSEC;END-ISO-10303-21;SIGNATURE {smuggled} ENDSEC;"
        );
        assert_eq!(ids(&text), Some(vec![1, 3]));
        let kinds: Vec<_> = instances(text.as_bytes())
            .map(|instance| instance.unwrap().kind.to_vec())
            .collect();
        assert_eq!(kinds, [b"CARTESIAN_POINT".to_vec(), Vec::new()]);
    }

    #[test]
    fn structure_outside_the_single_data_grammar_yields_no_instances() {
        let data = "DATA;#1=A();ENDSEC;";
        let header = "ISO-10303-21;HEADER;FILE_SCHEMA(('X'));ENDSEC;";
        assert_eq!(
            ids(&format!("{header}{data}END-ISO-10303-21;")),
            Some(vec![1])
        );
        for text in [
            format!("{header}{data}{data}END-ISO-10303-21;"),
            format!("{header}{data}"),
            format!("HEADER;FILE_SCHEMA(('X'));ENDSEC;{data}END-ISO-10303-21;"),
            format!("{header}REFERENCE;#9=<unterminated;ENDSEC;{data}END-ISO-10303-21;"),
            format!("{header}DATA;#1=A();#2=B(;ENDSEC;END-ISO-10303-21;"),
            format!("ISO-10303-21;HEADER;FILE_SCHEMA(('X');ENDSEC;{data}END-ISO-10303-21;"),
        ] {
            assert_eq!(ids(&text), None, "{text}");
        }
    }
}
