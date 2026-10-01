fn main() {
    let mut bits = 0x123456789abcdef0_u64;
    let mut checked = 0;
    let mut failures = Vec::new();
    for _ in 0..10000 {
        bits = bits.wrapping_mul(6364136223846793005).wrapping_add(1);
        let number = f64::from_bits(bits);
        if !number.is_finite() {
            continue;
        }
        let original = serde_json::json!(number);
        let text = serde_json::to_string(&original).unwrap();
        let decoded: serde_json::Value = serde_json::from_str(&text).unwrap();
        checked += 1;
        if decoded.as_f64().unwrap().to_bits() != number.to_bits() && failures.len() < 3 {
            failures.push(serde_json::json!({"text":text,"before_bits":format!("{:016x}",number.to_bits()),"after_bits":format!("{:016x}",decoded.as_f64().unwrap().to_bits())}));
        }
    }
    println!(
        "{}",
        serde_json::json!({"checked":checked,"counterexamples":failures})
    );
    if !failures.is_empty() {
        std::process::exit(1)
    }
}
