/// Byte scanner over raw fixture JSON: just enough of a lexer to find one
/// object and list its top-level keys without parsing whole values.
struct OrderScanner<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl OrderScanner<'_> {
    fn skip_whitespace(&mut self) {
        while self.pos < self.bytes.len()
            && matches!(self.bytes[self.pos], b' ' | b'\t' | b'\n' | b'\r')
        {
            self.pos += 1;
        }
    }

    fn expect(&mut self, byte: u8) -> Result<(), String> {
        self.skip_whitespace();
        match self.bytes.get(self.pos) {
            Some(&found) if found == byte => {
                self.pos += 1;
                Ok(())
            }
            Some(&found) => Err(format!(
                "expected '{}' but found '{}' at byte {}",
                byte.escape_ascii(),
                found.escape_ascii(),
                self.pos
            )),
            None => Err(format!(
                "expected '{}' but found end of input",
                byte.escape_ascii()
            )),
        }
    }

    /// Parse one JSON string literal and decode it exactly as `serde_json`
    /// would, so escaped key names (`\"`, `\\`, `\uXXXX`, …) decode
    /// identically to the parsed fixture `Value`.
    fn parse_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.bytes.get(self.pos) != Some(&b'"') {
            return Err(format!("expected string at byte {}", self.pos));
        }
        let start = self.pos;
        self.pos += 1;
        let mut end = None;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                // Skip the escaped character; validity is checked by
                // `serde_json` below. `\uXXXX` digits cannot contain `"`
                // or `\`, so landing inside the escape is harmless.
                b'\\' => self.pos += 2,
                b'"' => {
                    self.pos += 1;
                    end = Some(self.pos);
                    break;
                }
                _ => self.pos += 1,
            }
        }
        let end = end.ok_or_else(|| "unterminated string literal".to_owned())?;
        let literal = std::str::from_utf8(&self.bytes[start..end])
            .map_err(|error| format!("invalid UTF-8 in string literal: {error}"))?;
        serde_json::from_str::<String>(literal)
            .map_err(|error| format!("invalid string literal {literal:?}: {error}"))
    }

    /// Skip one JSON value of any shape, so nested `{ ... }` objects and
    /// `[ ... ]` arrays inside values never leak their inner keys.
    fn skip_value(&mut self) -> Result<(), String> {
        self.skip_whitespace();
        match self.bytes.get(self.pos) {
            Some(b'"') => {
                self.parse_string().map(|_| ())?;
                Ok(())
            }
            Some(b'{') | Some(b'[') => {
                let close = if self.bytes[self.pos] == b'{' {
                    b'}'
                } else {
                    b']'
                };
                let is_object = close == b'}';
                self.pos += 1;
                self.skip_whitespace();
                if self.bytes.get(self.pos) == Some(&close) {
                    self.pos += 1;
                    return Ok(());
                }
                loop {
                    if is_object {
                        self.parse_string()?;
                        self.expect(b':')?;
                    }
                    self.skip_value()?;
                    self.skip_whitespace();
                    match self.bytes.get(self.pos) {
                        Some(b',') => self.pos += 1,
                        Some(&found) if found == close => {
                            self.pos += 1;
                            break;
                        }
                        Some(&found) => {
                            return Err(format!(
                                "expected ',' or '{}' but found '{}' at byte {}",
                                close.escape_ascii(),
                                found.escape_ascii(),
                                self.pos
                            ));
                        }
                        None => return Err("unexpected end of input".to_owned()),
                    }
                }
                Ok(())
            }
            Some(_) => {
                // `true` / `false` / `null` / number: run to the delimiter.
                while self.pos < self.bytes.len()
                    && !matches!(
                        self.bytes[self.pos],
                        b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r'
                    )
                {
                    self.pos += 1;
                }
                Ok(())
            }
            None => Err("unexpected end of input".to_owned()),
        }
    }
}

/// Top-level key names of the fixture's `"parameters"` object, in file order.
///
/// The parsed fixture `Value` intentionally forgets object key order (without
/// `serde_json/preserve_order` the map is sorted), so the schema-order check
/// reads the order straight from the raw fixture text instead. Duplicate keys
/// inside `"parameters"` are reported twice (failing the order check), and a
/// duplicate top-level `"parameters"` key returns the first object rather than
/// `serde_json`'s last-one-wins.
pub(super) fn parameter_key_order(fixture_raw: &str) -> Result<Vec<String>, String> {
    let mut scan = OrderScanner {
        bytes: fixture_raw.as_bytes(),
        pos: 0,
    };
    scan.expect(b'{')?;
    scan.skip_whitespace();
    if scan.bytes.get(scan.pos) == Some(&b'}') {
        return Err("fixture has no \"parameters\" object".to_owned());
    }
    loop {
        let key = scan.parse_string()?;
        scan.expect(b':')?;
        if key == "parameters" {
            scan.expect(b'{')?;
            let mut keys = Vec::new();
            scan.skip_whitespace();
            if scan.bytes.get(scan.pos) == Some(&b'}') {
                return Ok(keys);
            }
            loop {
                let name = scan.parse_string()?;
                scan.expect(b':')?;
                scan.skip_value()?;
                keys.push(name);
                scan.skip_whitespace();
                match scan.bytes.get(scan.pos) {
                    Some(b',') => scan.pos += 1,
                    Some(b'}') => break,
                    Some(&found) => {
                        return Err(format!(
                            "expected ',' or '}}' but found '{}' at byte {}",
                            found.escape_ascii(),
                            scan.pos
                        ));
                    }
                    None => return Err("unexpected end of input".to_owned()),
                }
            }
            return Ok(keys);
        }
        scan.skip_value()?;
        scan.skip_whitespace();
        match scan.bytes.get(scan.pos) {
            Some(b',') => scan.pos += 1,
            Some(b'}') => break,
            Some(&found) => {
                return Err(format!(
                    "expected ',' or '}}' but found '{}' at byte {}",
                    found.escape_ascii(),
                    scan.pos
                ));
            }
            None => return Err("unexpected end of input".to_owned()),
        }
    }
    Err("fixture has no \"parameters\" object".to_owned())
}
