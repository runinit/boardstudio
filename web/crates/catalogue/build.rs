use std::path::Path;

fn main() {
    let src =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../catalogue/parts/imported-parts.json");
    println!("cargo::rerun-if-changed={}", src.display());
    let input = std::fs::read_to_string(&src).unwrap();
    let mut out = String::with_capacity(input.len());
    let (mut in_string, mut escaped) = (false, false);
    for c in input.chars() {
        if in_string || c == '"' || !c.is_whitespace() {
            out.push(c);
        }
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = in_string;
        } else if c == '"' {
            in_string = !in_string;
        }
    }
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap()).join("imported-parts.min.json");
    std::fs::write(dest, out).unwrap();
}
