//! Leaf types and checks shared by the host and the CAD worker for collision queries.
use serde::{Deserialize, Serialize};

/// Options that choose how a vendor STEP is imported for a collision operand.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CadStepImportOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_repair_budget_mm: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_deflection_mm: Option<f64>,
}

impl CadStepImportOptions {
    pub fn validate(self) -> Result<Self, String> {
        if self
            .target_deflection_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
        {
            return Err("STEP targetDeflectionMm must be finite and positive".into());
        }
        if let Some(budget) = self.preview_repair_budget_mm
            && (!budget.is_finite() || budget <= 0.0)
        {
            return Err("STEP previewRepairBudgetMm must be finite and positive".into());
        }
        Ok(self)
    }

    pub fn same_identity(self, other: Self) -> bool {
        if self.target_deflection_mm.map(f64::to_bits)
            != other.target_deflection_mm.map(f64::to_bits)
            || self.validate().is_err()
            || other.validate().is_err()
        {
            return false;
        }
        match (
            self.preview_repair_budget_mm,
            other.preview_repair_budget_mm,
        ) {
            (None, None) => true,
            (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        }
    }
}

/// True for a lowercase hexadecimal SHA-256 digest.
pub fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AffineTransformError {
    NonFinite,
    NotAffine,
    Singular,
}

/// Checks that a row-major 4x4 placement is finite, affine and invertible.
pub fn validate_affine_row_transform(matrix: &[f64; 16]) -> Result<(), AffineTransformError> {
    if matrix.iter().any(|value| !value.is_finite()) {
        return Err(AffineTransformError::NonFinite);
    }
    if [matrix[3], matrix[7], matrix[11], matrix[15]] != [0.0, 0.0, 0.0, 1.0] {
        return Err(AffineTransformError::NotAffine);
    }
    let det = matrix[0] * (matrix[5] * matrix[10] - matrix[6] * matrix[9])
        - matrix[4] * (matrix[1] * matrix[10] - matrix[2] * matrix[9])
        + matrix[8] * (matrix[1] * matrix[6] - matrix[2] * matrix[5]);
    if !det.is_finite() || det.abs() <= 1e-12 {
        return Err(AffineTransformError::Singular);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY: [f64; 16] = [
        1., 0., 0., 0., //
        0., 1., 0., 0., //
        0., 0., 1., 0., //
        0., 0., 0., 1.,
    ];

    #[test]
    fn sha256_accepts_only_64_lowercase_hex_digits() {
        assert!(is_sha256_hex(&"a1".repeat(32)));
        assert!(!is_sha256_hex(&"A1".repeat(32)), "uppercase");
        assert!(!is_sha256_hex(&"a1".repeat(31)), "short");
        assert!(!is_sha256_hex(&format!("{}g", "a".repeat(63))), "non-hex");
    }

    #[test]
    fn affine_transform_accepts_a_rigid_motion_and_names_each_rejection() {
        let mut moved = IDENTITY;
        moved[12] = 5.;
        assert_eq!(validate_affine_row_transform(&moved), Ok(()));

        let mut not_finite = IDENTITY;
        not_finite[12] = f64::NAN;
        assert_eq!(
            validate_affine_row_transform(&not_finite),
            Err(AffineTransformError::NonFinite)
        );

        let mut projective = IDENTITY;
        projective[3] = 0.5;
        assert_eq!(
            validate_affine_row_transform(&projective),
            Err(AffineTransformError::NotAffine)
        );

        let mut flat = IDENTITY;
        flat[10] = 0.;
        assert_eq!(
            validate_affine_row_transform(&flat),
            Err(AffineTransformError::Singular)
        );
    }

    #[test]
    fn step_import_options_reject_non_positive_values_and_compare_by_bits() {
        let valid = CadStepImportOptions {
            preview_repair_budget_mm: Some(0.03),
            target_deflection_mm: Some(0.01),
        };
        assert_eq!(valid.validate(), Ok(valid));
        for bad in [0., -1., f64::NAN, f64::INFINITY] {
            let target = CadStepImportOptions {
                target_deflection_mm: Some(bad),
                ..Default::default()
            };
            let budget = CadStepImportOptions {
                preview_repair_budget_mm: Some(bad),
                ..Default::default()
            };
            assert!(target.validate().is_err(), "target {bad}");
            assert!(budget.validate().is_err(), "budget {bad}");
        }
        assert!(valid.same_identity(valid));
        let finer = CadStepImportOptions {
            target_deflection_mm: Some(0.005),
            ..valid
        };
        assert!(!valid.same_identity(finer));
    }
}
