//! The content build emits the same default preview buffers as the live importer.
//! Only surface assets with an Unknown source proof can be baked by this format.
use super::*;
use boardstudio_cad::qualification::{self, SourceQualification};

const MAGIC: &[u8; 8] = b"BSSTEP01";
const MAX_BAKED_BYTES: usize = 32 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Metadata {
    bounds: BoundsDto,
    vendor: VendorStepDto,
    unknown_reasons: Vec<String>,
}

fn encode(bytes: Vec<u8>, source_sha256: String) -> Result<Vec<u8>, String> {
    let asset = ReadStepAsset::import_bytes(
        bytes,
        ReadStepOptions {
            source_sha256,
            preview_repair_budget_mm: None,
            target_deflection_mm: None,
        },
    )?;
    let SourceQualification::Unknown { reasons } = asset.analysis.source_qualification() else {
        return Err("preview bake requires an unqualified STEP source".into());
    };
    if !asset.vendor_step.line_occurrences.is_empty() {
        return Err("preview bake does not support source lines".into());
    }
    let metadata = serde_json::to_vec(&Metadata {
        bounds: asset.bounds,
        unknown_reasons: reasons.clone(),
        vendor: asset.vendor_step,
    })
    .map_err(|error| error.to_string())?;
    let qualification::PreparedAnalysisView::Vendor(view) = asset.analysis.export_view() else {
        unreachable!();
    };
    if view
        .face_occurrences
        .iter()
        .any(|face| face.repair_displacement_mm != 0.0)
    {
        return Err("preview bake does not support retained face movement".into());
    }
    let mut output = MAGIC.to_vec();
    let metadata_size =
        u32::try_from(metadata.len()).map_err(|_| "baked STEP metadata is too large")?;
    output.extend(metadata_size.to_le_bytes());
    output.extend(metadata);
    for value in view.positions_mm_f64.iter().flatten() {
        output.extend(value.to_le_bytes());
    }
    for value in asset.mesh.normals {
        output.extend(value.to_le_bytes());
    }
    for index in view.triangle_face_occurrence_indices {
        output.extend(index_u32(*index, "triangle face occurrence")?.to_le_bytes());
    }
    if output.len() > MAX_BAKED_BYTES {
        return Err("baked STEP preview exceeds the asset size limit".into());
    }
    Ok(output)
}

/// Content builds call this on the same packaged provider that handles live STEP imports.
#[wasm_bindgen]
pub fn bake_bundled_step_model(
    bytes: Uint8Array,
    source_sha256: String,
) -> Result<Uint8Array, JsValue> {
    encode(bytes.to_vec(), source_sha256)
        .map(|payload| Uint8Array::from(payload.as_slice()))
        .map_err(super::super::js_error)
}

#[wasm_bindgen]
pub fn read_baked_step_model(bytes: Uint8Array, source_sha256: &str) -> Result<JsValue, JsValue> {
    if bytes.length() as usize > MAX_BAKED_BYTES {
        return Err(super::super::js_error(
            "baked STEP preview exceeds the asset size limit",
        ));
    }
    decode(&bytes.to_vec(), source_sha256)
        .map_err(super::super::js_error)?
        .into_js_value()
}

fn decode(bytes: &[u8], expected_sha: &str) -> Result<ReadStepAsset, String> {
    let invalid = || "Invalid baked STEP preview".to_owned();
    let bytes = bytes.strip_prefix(MAGIC).ok_or_else(invalid)?;
    let (size, bytes) = bytes.split_first_chunk::<4>().ok_or_else(invalid)?;
    let (metadata, buffers) = bytes
        .split_at_checked(u32::from_le_bytes(*size) as usize)
        .ok_or_else(invalid)?;
    let Metadata {
        bounds,
        mut vendor,
        unknown_reasons,
    } = serde_json::from_slice(metadata).map_err(|_| invalid())?;
    if vendor.source_sha256 != expected_sha
        || !boardstudio_contracts::is_sha256_hex(expected_sha)
        || vendor.import_options.target_deflection_mm.is_some()
        || vendor.import_options.preview_repair_budget_mm.is_some()
        || vendor.accuracy.analysis_error_bound_mm.is_some()
    {
        return Err(invalid());
    }

    // Per triangle: 9 doubles, 9 floats, one occurrence index. The f32
    // positions are the same projection as the live import and need no disk copy.
    const STRIDE: usize = 9 * (8 + 4) + 4;
    if buffers.is_empty() || !buffers.len().is_multiple_of(STRIDE) {
        return Err(invalid());
    }
    let triangles = buffers.len() / STRIDE;
    let (positions, rest) = buffers.split_at(triangles * 9 * 8);
    let (normals, tags) = rest.split_at(triangles * 9 * 4);
    vendor.surface_positions_mm_f64 = positions
        .as_chunks::<8>()
        .0
        .iter()
        .copied()
        .map(f64::from_le_bytes)
        .collect();
    vendor.triangle_face_occurrence_indices = tags
        .as_chunks::<4>()
        .0
        .iter()
        .copied()
        .map(u32::from_le_bytes)
        .collect();
    let source_faces = vendor
        .source_faces
        .iter()
        .map(|face| {
            Ok(StepSourceFace {
                solid_id: face.solid_id.parse().map_err(|_| invalid())?,
                shell_index: face.shell_index as usize,
                face_id: face.face_id.parse().map_err(|_| invalid())?,
                support_id: face.support_id.parse().map_err(|_| invalid())?,
                support_kind: face.support_kind.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let face_occurrences = vendor
        .face_occurrences
        .iter()
        .map(|face| {
            Ok(StepFaceOccurrence {
                source_face: face.source_face_index as usize,
                occurrence: face.solid_occurrence_index as usize,
                representation_id: face.representation_id.parse().map_err(|_| invalid())?,
                source_uncertainty_mm: face.source_uncertainty_mm,
                // The importer keeps the complete-asset movement unknown on this route.
                repair_displacement_mm: 0.0,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let tags: Vec<_> = vendor
        .triangle_face_occurrence_indices
        .iter()
        .map(|index| *index as usize)
        .collect();
    let analysis = qualification::prepare_baked_vendor_preview(
        qualification::PreparedVendorView {
            positions_mm_f64: vendor.surface_positions_mm_f64.as_chunks::<3>().0,
            triangle_face_occurrence_indices: &tags,
            source_faces: &source_faces,
            face_occurrences: &face_occurrences,
        },
        vendor.accuracy.preview_deflection_mm,
        vendor.accuracy.preview_angular_rad,
        unknown_reasons,
    )?;
    Ok(ReadStepAsset {
        analysis,
        mesh: MeshDto {
            positions: vendor
                .surface_positions_mm_f64
                .iter()
                .map(|value| *value as f32)
                .collect(),
            normals: normals
                .as_chunks::<4>()
                .0
                .iter()
                .copied()
                .map(f32::from_le_bytes)
                .collect(),
        },
        bounds,
        vendor_step: vendor,
    })
}
