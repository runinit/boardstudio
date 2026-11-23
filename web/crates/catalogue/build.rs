fn main() {
    println!("cargo::rerun-if-changed=../../../catalogue/parts/imported-parts.json");
    let mut out = String::new();
    let (mut s, mut e) = (false, false);
    for c in include_str!("../../../catalogue/parts/imported-parts.json").chars() {
        if s || c == '"' || !c.is_whitespace() {
            out.push(c);
        }
        if e {
            e = false;
        } else if c == '\\' {
            e = s;
        } else if c == '"' {
            s = !s;
        }
    }
    let path = format!(
        "{}/imported-parts.min.json",
        std::env::var("OUT_DIR").unwrap()
    );
    std::fs::write(path, out).unwrap();
}
