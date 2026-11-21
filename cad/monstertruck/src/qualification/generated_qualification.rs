//! Qualification verdict shared by the native and fixture proofs.

#[derive(Debug, Clone, PartialEq)]
pub enum GeneratedQualification {
    Bounded {
        version: u32,
        class: &'static str,
        direct_bound_mm: f64,
        source_precision_mm: f64,
        total_bound_mm: f64,
    },
    Unknown {
        reasons: Vec<String>,
    },
}

#[cfg(test)]
mod vendor_fixture;
