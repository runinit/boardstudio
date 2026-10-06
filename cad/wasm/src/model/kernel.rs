//! Narrow CAD kernel interface: build a case, import STEP, export STEP, mesh.
//!
//! Cadrum/OCCT sits behind [`CadrumKernel`]; the top-level case, assembly and STEP-import
//! operations are generic over [`CadKernel`] and keep their kernel-independent policy (STEP size
//! limit, error wording, mesh concatenation) outside it. The preview/export caches and keycap
//! generation still hold `cadrum::Solid`s directly: they are production-kernel optimisations, not
//! part of this seam.
//!
//! The acceptance gate for any implementation is the fixture expectations plus the OCCT
//! oracle's negative controls (`validation_tests::kernel_interface_meets_the_same_gate`), so a
//! refactor or a future kernel experiment is judged on the same evidence as production.

use super::*;
use std::io::Cursor;

/// A kernel's view of a successfully imported STEP file.
pub(super) struct Imported<M> {
    pub model: M,
    /// Millimetre bounds reported by the kernel itself.
    pub min: [f64; 3],
    pub max: [f64; 3],
    #[cfg(test)]
    pub solid_count: usize,
}

pub(super) trait CadKernel {
    /// Opaque geometry owned by the kernel (one case body, or everything read from one file).
    type Model;

    /// Builds the solids of one prepared case body.
    fn build_case(&self, case: &PreparedCase) -> Result<Self::Model, String>;
    /// Reads STEP bytes. The caller has already enforced the file-size policy.
    fn import_step(&self, bytes: &[u8]) -> Result<Imported<Self::Model>, String>;
    /// Writes all `parts` as one STEP file.
    fn export_step(&self, parts: &[Self::Model]) -> Result<Vec<u8>, String>;
    /// Tessellates one model into non-indexed position/normal buffers.
    fn mesh(&self, model: &Self::Model) -> Result<MeshData, String>;
}

pub(super) struct CadrumKernel;

impl CadKernel for CadrumKernel {
    type Model = Vec<Solid>;

    fn build_case(&self, case: &PreparedCase) -> Result<Self::Model, String> {
        super::construction::build_body(case)
    }

    fn import_step(&self, bytes: &[u8]) -> Result<Imported<Self::Model>, String> {
        let mut reader = Cursor::new(bytes);
        let solids = {
            let _stage = Stage::new("stepImport");
            Solid::read_step(&mut reader).map_err(|error| format!("STEP import failed: {error}"))?
        };
        if solids.is_empty() {
            return Err("STEP import failed: empty shape".into());
        }

        let mut min = DVec3::splat(f64::INFINITY);
        let mut max = DVec3::splat(f64::NEG_INFINITY);
        for solid in &solids {
            let [solid_min, solid_max] = solid.bounding_box();
            min = min.min(solid_min);
            max = max.max(solid_max);
        }
        if !min.is_finite() || !max.is_finite() || min.cmpgt(max).any() {
            return Err("STEP import failed: invalid bounds".into());
        }
        Ok(Imported {
            #[cfg(test)]
            solid_count: solids.len(),
            model: solids,
            min: min.to_array(),
            max: max.to_array(),
        })
    }

    fn export_step(&self, parts: &[Self::Model]) -> Result<Vec<u8>, String> {
        if parts.iter().all(Vec::is_empty) {
            return Err("OpenCascade returned an empty case solid".into());
        }
        let mut step = Vec::new();
        let _stage = Stage::new("stepSerialization");
        Solid::write_step(parts.iter().flatten(), &mut step)
            .map_err(|error| format!("OpenCascade STEP export failed: {error}"))?;
        Ok(step)
    }

    fn mesh(&self, model: &Self::Model) -> Result<MeshData, String> {
        mesh_data(model)
    }
}
