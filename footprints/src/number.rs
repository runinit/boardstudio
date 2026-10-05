//! JavaScript number semantics the generators' output depends on: `Number` to
//! string, string to `Number`, `encodeURIComponent`, and the trigonometry used
//! for poses. Generated text is compared as exact text, so these must agree with
//! V8 digit for digit; the recorded vectors in `tests/golden/numeric_vectors.json`
//! pin that down.
use std::f64::consts::PI;

/// `String(value)` for a JavaScript number.
pub fn js_number(value: f64) -> String {
    if value.is_nan() {
        return "NaN".into();
    }
    if value == 0.0 {
        return "0".into();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    // Rust's `{:e}` prints the shortest digits that round-trip, as V8 does.
    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific
        .split_once('e')
        .expect("LowerExp output has an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let k = digits.len() as i32;
    let n = exponent + 1;
    let mut out = String::new();
    if value < 0.0 {
        out.push('-');
    }
    if k <= n && n <= 21 {
        out.push_str(&digits);
        out.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        out.push_str(&digits[..n as usize]);
        out.push('.');
        out.push_str(&digits[n as usize..]);
    } else if -6 < n && n <= 0 {
        out.push_str("0.");
        out.extend(std::iter::repeat_n('0', (-n) as usize));
        out.push_str(&digits);
    } else {
        out.push_str(&digits[..1]);
        if k > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push('e');
        out.push(if n - 1 < 0 { '-' } else { '+' });
        out.push_str(&(n - 1).abs().to_string());
    }
    out
}

fn is_js_space(c: char) -> bool {
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

/// `Number(text)`: NaN when the text is not a numeric literal.
pub fn js_to_number(text: &str) -> f64 {
    let text = text.trim_matches(is_js_space);
    if text.is_empty() {
        return 0.0;
    }
    match text {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    let radix = |prefixes: [&str; 2], base: u32| {
        let rest = prefixes.iter().find_map(|p| text.strip_prefix(p))?;
        if rest.is_empty() {
            return Some(f64::NAN);
        }
        let mut value = 0.0_f64;
        for c in rest.chars() {
            let Some(digit) = c.to_digit(base) else {
                return Some(f64::NAN);
            };
            value = value * f64::from(base) + f64::from(digit);
        }
        Some(value)
    };
    if let Some(value) = radix(["0x", "0X"], 16)
        .or_else(|| radix(["0o", "0O"], 8))
        .or_else(|| radix(["0b", "0B"], 2))
    {
        return value;
    }
    let bytes = text.as_bytes();
    let mut i = 0;
    if matches!(bytes.first(), Some(b'+' | b'-')) {
        i += 1;
    }
    let integer_start = i;
    while bytes.get(i).is_some_and(u8::is_ascii_digit) {
        i += 1;
    }
    let mut digits = i - integer_start;
    if bytes.get(i) == Some(&b'.') {
        i += 1;
        let fraction_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        digits += i - fraction_start;
    }
    if digits == 0 {
        return f64::NAN;
    }
    if matches!(bytes.get(i), Some(b'e' | b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(b'+' | b'-')) {
            i += 1;
        }
        let exponent_start = i;
        while bytes.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        if i == exponent_start {
            return f64::NAN;
        }
    }
    if i != bytes.len() {
        return f64::NAN;
    }
    text.parse().unwrap_or(f64::NAN)
}

/// `encodeURIComponent`.
pub fn encode_uri_component(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// `decodeURIComponent`; `None` on malformed input, as JavaScript throws.
pub fn decode_uri_component(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// `degrees * Math.PI / 180`, evaluated in the same order.
pub fn radians(degrees: f64) -> f64 {
    degrees * PI / 180.0
}

/// `Math.cos`, bit-identical across native and WASM builds.
pub fn cos(angle: f64) -> f64 {
    libm::cos(angle)
}

/// `Math.sin`, bit-identical across native and WASM builds.
pub fn sin(angle: f64) -> f64 {
    libm::sin(angle)
}

/// `Math.hypot` for two arguments, using V8's scaled and compensated sum so the
/// result agrees with it to the last bit (it is not correctly rounded).
pub fn hypot(x: f64, y: f64) -> f64 {
    let (x, y) = (x.abs(), y.abs());
    if x.is_infinite() || y.is_infinite() {
        return f64::INFINITY;
    }
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let max = x.max(y);
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0_f64, 0.0_f64);
    for value in [x, y] {
        let n = value / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}
