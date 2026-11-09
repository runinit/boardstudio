//! Borrow numeric spellings from already parsed STEP bytes; never rebuild entity ASTs.

pub(super) fn separators(mut input: &[u8]) -> Option<&[u8]> {
    loop {
        input = input.trim_ascii_start();
        if !input.starts_with(b"/*") {
            return Some(input);
        }
        let end = input[2..].windows(2).position(|pair| pair == b"*/")?;
        input = &input[end + 4..];
    }
}

pub(super) fn string(input: &[u8]) -> Option<&[u8]> {
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

pub(super) fn reference(input: &[u8]) -> Option<(&[u8], u64)> {
    let input = input.strip_prefix(b"#")?;
    let length = input
        .iter()
        .take_while(|byte| byte.is_ascii_digit())
        .count();
    let id = std::str::from_utf8(&input[..length]).ok()?.parse().ok()?;
    Some((&input[length..], id))
}

pub(super) fn number(input: &[u8]) -> Option<(&[u8], &str)> {
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

pub(super) struct Instance<'a> {
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

pub(super) fn instances(mut input: &[u8]) -> impl Iterator<Item = Option<Instance<'_>>> {
    std::iter::from_fn(move || {
        while !input.is_empty() {
            let Some(next) = separators(input) else {
                input = &[];
                return Some(None);
            };
            if next.is_empty() {
                input = &[];
                return None;
            }
            if next.starts_with(b"#") {
                return match instance(next) {
                    Some((rest, instance)) => {
                        input = rest;
                        Some(Some(instance))
                    }
                    None => {
                        input = &[];
                        Some(None)
                    }
                };
            }
            let rest = if next.starts_with(b"'") {
                string(next)
            } else {
                next.get(1..)
            };
            let Some(rest) = rest else {
                input = &[];
                return Some(None);
            };
            input = rest;
        }
        None
    })
}
