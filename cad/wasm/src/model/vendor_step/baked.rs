//! The content build emits the same default preview buffers as the live importer.
//! Only surface assets with an Unknown source proof can be baked by this format.
use super::*;
use boardstudio_cad::{
    mesh::{Point3, degraded_triangle_normal, triangle_normal},
    qualification::{self, SourceQualification},
};
use std::collections::HashMap;

const MAGIC: &[u8; 8] = b"BSSTEP03";
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
    if view.positions_mm_f64.is_empty()
        || !view.positions_mm_f64.len().is_multiple_of(3)
        || view.positions_mm_f64.len() / 3 != view.triangle_face_occurrence_indices.len()
    {
        return Err("preview bake received misaligned triangle buffers".into());
    }
    let mut unique_vertices: Vec<[f64; 3]> = Vec::new();
    let mut vertex_lookup: HashMap<[u64; 3], u32> = HashMap::new();
    let mut triangle_indices: Vec<u32> = Vec::with_capacity(view.positions_mm_f64.len());
    for &point in view.positions_mm_f64 {
        let key = point.map(f64::to_bits);
        let index = match vertex_lookup.get(&key) {
            Some(&index) => index,
            None => {
                let index = index_u32(unique_vertices.len(), "baked vertex")?;
                unique_vertices.push(point);
                vertex_lookup.insert(key, index);
                index
            }
        };
        triangle_indices.push(index);
    }
    let mut runs: Vec<(u32, u32)> = Vec::new();
    for &face in view.triangle_face_occurrence_indices {
        let face_u32 = index_u32(face, "triangle face occurrence")?;
        match runs.last_mut() {
            Some((last_face, count)) if *last_face == face_u32 => {
                *count = count
                    .checked_add(1)
                    .ok_or("triangle face occurrence run overflow")?;
            }
            _ => runs.push((face_u32, 1)),
        }
    }

    let mut output = MAGIC.to_vec();
    let metadata_size =
        u32::try_from(metadata.len()).map_err(|_| "baked STEP metadata is too large")?;
    output.extend(metadata_size.to_le_bytes());
    output.extend(metadata);
    // Private measurements follow the face occurrence table in the same order.
    for face in view.face_occurrences {
        let movement = face.repair_displacement_mm;
        if !valid_movement(movement) {
            return Err("preview bake received invalid retained face movement".into());
        }
        output.extend(movement.to_le_bytes());
    }
    let vertex_count = index_u32(unique_vertices.len(), "baked vertex count")?;
    let triangle_count = index_u32(
        view.triangle_face_occurrence_indices.len(),
        "baked triangle count",
    )?;
    let run_count = index_u32(runs.len(), "baked face occurrence run count")?;
    output.extend(vertex_count.to_le_bytes());
    output.extend(triangle_count.to_le_bytes());
    output.extend(run_count.to_le_bytes());
    for point in &unique_vertices {
        for coord in point {
            output.extend(coord.to_le_bytes());
        }
    }
    if unique_vertices.len() <= usize::from(u16::MAX) {
        for &index in &triangle_indices {
            output.extend((index as u16).to_le_bytes());
        }
    } else {
        for &index in &triangle_indices {
            output.extend(index.to_le_bytes());
        }
    }
    for (face, count) in runs {
        output.extend(face.to_le_bytes());
        output.extend(count.to_le_bytes());
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
        || vendor.accuracy.selected_repair_budget_mm.is_some()
        || vendor.accuracy.analysis_error_bound_mm.is_some()
    {
        return Err(invalid());
    }

    let movement_bytes = vendor
        .face_occurrences
        .len()
        .checked_mul(8)
        .ok_or_else(invalid)?;
    let (movements, buffers) = buffers
        .split_at_checked(movement_bytes)
        .ok_or_else(invalid)?;

    let (header, buffers) = buffers.split_first_chunk::<12>().ok_or_else(invalid)?;
    let vertex_count = u32::from_le_bytes(header[0..4].try_into().unwrap()) as usize;
    let triangle_count = u32::from_le_bytes(header[4..8].try_into().unwrap()) as usize;
    let run_count = u32::from_le_bytes(header[8..12].try_into().unwrap()) as usize;
    if vertex_count == 0
        || triangle_count == 0
        || run_count == 0
        || run_count > triangle_count
    {
        return Err(invalid());
    }
    let vertex_bytes = vertex_count.checked_mul(24).ok_or_else(invalid)?;
    let index_stride = if vertex_count <= usize::from(u16::MAX) {
        2
    } else {
        4
    };
    let index_count = triangle_count.checked_mul(3).ok_or_else(invalid)?;
    let index_bytes = index_count.checked_mul(index_stride).ok_or_else(invalid)?;
    let run_bytes = run_count.checked_mul(8).ok_or_else(invalid)?;
    let expected_len = vertex_bytes
        .checked_add(index_bytes)
        .and_then(|n| n.checked_add(run_bytes))
        .ok_or_else(invalid)?;
    if buffers.len() != expected_len {
        return Err(invalid());
    }
    let (raw_vertices, rest) = buffers.split_at(vertex_bytes);
    let (raw_indices, raw_runs) = rest.split_at(index_bytes);
    let vertices: Vec<[f64; 3]> = raw_vertices
        .as_chunks::<24>()
        .0
        .iter()
        .map(|chunk| {
            [
                f64::from_le_bytes(chunk[0..8].try_into().unwrap()),
                f64::from_le_bytes(chunk[8..16].try_into().unwrap()),
                f64::from_le_bytes(chunk[16..24].try_into().unwrap()),
            ]
        })
        .collect();
    if vertices
        .iter()
        .flatten()
        .any(|coord| !coord.is_finite())
    {
        return Err(invalid());
    }
    let mut positions_mm_f64 = Vec::with_capacity(index_count * 3);
    if index_stride == 2 {
        for chunk in raw_indices.as_chunks::<2>().0 {
            let idx = usize::from(u16::from_le_bytes(*chunk));
            let point = vertices.get(idx).ok_or_else(invalid)?;
            positions_mm_f64.extend_from_slice(point);
        }
    } else {
        for chunk in raw_indices.as_chunks::<4>().0 {
            let idx = u32::from_le_bytes(*chunk) as usize;
            let point = vertices.get(idx).ok_or_else(invalid)?;
            positions_mm_f64.extend_from_slice(point);
        }
    }
    let mut occurrence_indices = Vec::with_capacity(triangle_count);
    for chunk in raw_runs.as_chunks::<8>().0 {
        let face = u32::from_le_bytes(chunk[0..4].try_into().unwrap());
        let count = u32::from_le_bytes(chunk[4..8].try_into().unwrap()) as usize;
        if count == 0
            || (face as usize) >= vendor.face_occurrences.len()
            || occurrence_indices.len().saturating_add(count) > triangle_count
        {
            return Err(invalid());
        }
        occurrence_indices.resize(occurrence_indices.len() + count, face);
    }
    if occurrence_indices.len() != triangle_count {
        return Err(invalid());
    }
    let mut normals = Vec::with_capacity(index_count * 3);
    for tri in positions_mm_f64.as_chunks::<9>().0 {
        let points = [
            Point3::new(tri[0], tri[1], tri[2]),
            Point3::new(tri[3], tri[4], tri[5]),
            Point3::new(tri[6], tri[7], tri[8]),
        ];
        let normal = triangle_normal(points)
            .or_else(|_| degraded_triangle_normal(points))
            .map_err(|_| invalid())?;
        let n = [normal.x as f32, normal.y as f32, normal.z as f32];
        if n.iter().any(|v| !v.is_finite()) {
            return Err(invalid());
        }
        for _ in 0..3 {
            normals.extend_from_slice(&n);
        }
    }
    let positions: Vec<f32> = positions_mm_f64.iter().map(|value| *value as f32).collect();
    if positions.iter().any(|v| !v.is_finite()) {
        return Err(invalid());
    }
    vendor.surface_positions_mm_f64 = positions_mm_f64;
    vendor.triangle_face_occurrence_indices = occurrence_indices;
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
                color: face.color,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let face_occurrences = vendor
        .face_occurrences
        .iter()
        .zip(movements.as_chunks::<8>().0)
        .map(|(face, movement)| {
            let repair_displacement_mm = f64::from_le_bytes(*movement);
            if !valid_movement(repair_displacement_mm) || face.repair_displacement_mm.is_some() {
                return Err(invalid());
            }
            Ok(StepFaceOccurrence {
                source_face: face.source_face_index as usize,
                occurrence: face.solid_occurrence_index as usize,
                representation_id: face.representation_id.parse().map_err(|_| invalid())?,
                source_uncertainty_mm: face.source_uncertainty_mm,
                repair_displacement_mm,
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
        mesh: MeshDto { positions, normals },
        bounds,
        vendor_step: vendor,
    })
}

fn valid_movement(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests;
