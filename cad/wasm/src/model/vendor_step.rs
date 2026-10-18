//! Complete source-preserving STEP assets for the staged provider adapter.

use boardstudio_cad::{
    import_step_with_options, StepAccuracy, StepFaceOccurrence, StepImportOptions, StepPrecision,
    StepSourceFace, StepUncertainty,
};
use js_sys::{Float32Array, Float64Array, Object, Uint32Array, Uint8Array};
use serde::{Deserialize, Serialize};
use wasm_bindgen::{prelude::wasm_bindgen, JsCast, JsValue};

const MAX_STEP_BYTES: u32 = 32 * 1024 * 1024;

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadStepOptions {
    source_sha256: String,
    preview_repair_budget_mm: Option<f64>,
    target_deflection_mm: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ImportOptionsDto {
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_repair_budget_mm: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_deflection_mm: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReadStepAsset {
    #[serde(skip)]
    analysis: boardstudio_cad::qualification::PreparedAnalysisMesh,
    mesh: MeshDto,
    bounds: BoundsDto,
    vendor_step: VendorStepDto,
}

#[derive(Serialize)]
struct MeshDto {
    #[serde(skip)]
    positions: Vec<f32>,
    #[serde(skip)]
    normals: Vec<f32>,
}

#[derive(Serialize)]
struct BoundsDto {
    min: [f64; 3],
    max: [f64; 3],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VendorStepDto {
    source_sha256: String,
    import_options: ImportOptionsDto,
    source_faces: Vec<SourceFaceDto>,
    face_occurrences: Vec<FaceOccurrenceDto>,
    precision: Vec<PrecisionDto>,
    accuracy: AccuracyDto,
    #[serde(skip)]
    surface_positions_mm_f64: Vec<f64>,
    #[serde(skip)]
    triangle_face_occurrence_indices: Vec<u32>,
    #[serde(skip)]
    line_segments_mm_f64: Vec<f64>,
    #[serde(skip)]
    line_render_positions: Vec<f32>,
    #[serde(skip)]
    line_occurrences: Vec<LineOccurrenceDto>,
    #[serde(skip)]
    segment_line_occurrence_indices: Vec<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceFaceDto {
    solid_id: String,
    shell_index: u32,
    face_id: String,
    support_id: String,
    support_kind: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FaceOccurrenceDto {
    source_face_index: u32,
    solid_occurrence_index: u32,
    representation_id: String,
    source_uncertainty_mm: Option<f64>,
    repair_displacement_mm: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LineOccurrenceDto {
    line_occurrence_index: u32,
    trimmed_curve_id: String,
    source_line_id: String,
    curve_set_id: String,
    representation_id: String,
    source_unit_to_mm: f64,
    #[serde(skip)]
    placement_f64: [f64; 16],
    declared_uncertainty_mm: Option<f64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrecisionDto {
    representation_id: String,
    context_id: String,
    geometric_dimension: Option<u32>,
    declarations: Vec<UncertaintyDto>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct UncertaintyDto {
    entity_id: String,
    length_unit_id: String,
    millimetres: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AccuracyDto {
    preview_deflection_mm: f64,
    preview_angular_rad: f64,
    analysis_error_bound_mm: Option<f64>,
    bounds_from_source_geometry: bool,
    mesh_closed_oriented: bool,
    preview_floor_mm: f64,
    preview_declared_multiplier: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    selected_repair_budget_mm: Option<f64>,
    max_repair_displacement_mm: Option<f64>,
    relaxed_beyond_declared_precision: bool,
    sampled_support_checks: u32,
    max_sampled_support_residual_mm: Option<f64>,
    fallback_meshed_faces: u32,
}

impl ReadStepAsset {
    fn import(bytes: Uint8Array, options: Option<JsValue>) -> Result<Self, String> {
        let options = options.ok_or("Invalid STEP options: sourceSha256 is required")?;
        if !options.is_object() || options.is_null() {
            return Err("Invalid STEP options: expected an options object".into());
        }
        let options_object: Object = options.clone().unchecked_into();
        let keys = Object::keys(&options_object);
        if keys.iter().any(|key| {
            !matches!(
                key.as_string().as_deref(),
                Some("sourceSha256" | "previewRepairBudgetMm" | "targetDeflectionMm")
            )
        }) {
            return Err("Invalid STEP options: unknown option".into());
        }
        let options: ReadStepOptions = serde_wasm_bindgen::from_value(options)
            .map_err(|error| format!("Invalid STEP options: {error}"))?;
        if bytes.length() == 0 || bytes.length() > MAX_STEP_BYTES {
            return Err("STEP import failed: invalid file size".into());
        }
        Self::import_bytes(bytes.to_vec(), options)
    }

    fn import_bytes(bytes: Vec<u8>, options: ReadStepOptions) -> Result<Self, String> {
        if !is_sha256(&options.source_sha256) {
            return Err(
                "Invalid STEP options: sourceSha256 must be 64 lowercase hexadecimal characters"
                    .into(),
            );
        }
        // This is a binding label; worker byte verification must precede activation.
        if let Some(budget) = options.preview_repair_budget_mm {
            if !budget.is_finite() || budget <= 0.0 {
                return Err(
                    "Invalid STEP options: previewRepairBudgetMm must be finite and positive"
                        .into(),
                );
            }
        }
        if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES as usize {
            return Err("STEP import failed: invalid file size".into());
        }

        let imported = import_step_with_options(
            bytes,
            StepImportOptions {
                preview_repair_budget_mm: options.preview_repair_budget_mm,
                target_deflection_mm: options.target_deflection_mm,
            },
        )?;
        let mesh = imported.mesh();
        let analysis = boardstudio_cad::qualification::prepare_vendor_analysis(&imported)?;
        let boardstudio_cad::qualification::PreparedAnalysisView::Vendor(view) =
            analysis.export_view()
        else {
            unreachable!()
        };
        let preview = view.positions_mm_f64;
        if mesh.positions.is_empty()
            || !mesh.positions.len().is_multiple_of(9)
            || mesh.normals.len() != mesh.positions.len()
            || preview.len() * 3 != mesh.positions.len()
            || imported.triangle_faces().len() != mesh.positions.len() / 9
            || mesh
                .positions
                .iter()
                .chain(&mesh.normals)
                .any(|value| !value.is_finite())
            || preview.iter().flatten().any(|value| !value.is_finite())
            || preview
                .iter()
                .flatten()
                .zip(&mesh.positions)
                .any(|(source, render)| *source as f32 != *render)
        {
            return Err("STEP importer returned inconsistent surface buffers".into());
        }

        let source_faces = view
            .source_faces
            .iter()
            .map(source_face_dto)
            .collect::<Result<Vec<_>, _>>()?;
        let face_occurrences = view
            .face_occurrences
            .iter()
            .map(face_occurrence_dto)
            .collect::<Result<Vec<_>, _>>()?;
        if face_occurrences
            .iter()
            .any(|face| face.source_face_index as usize >= source_faces.len())
        {
            return Err("STEP importer returned an invalid source face reference".into());
        }
        let triangle_face_occurrence_indices = view
            .triangle_face_occurrence_indices
            .iter()
            .map(|index| index_u32(*index, "triangle face occurrence"))
            .collect::<Result<Vec<_>, _>>()?;
        if triangle_face_occurrence_indices
            .iter()
            .any(|index| *index as usize >= face_occurrences.len())
        {
            return Err("STEP importer returned an invalid triangle face tag".into());
        }

        let surface_positions_mm_f64 = preview.iter().flatten().copied().collect::<Vec<_>>();
        let mut line_segments_mm_f64 = Vec::new();
        let mut line_render_positions = Vec::new();
        let mut segment_line_occurrence_indices = Vec::new();
        let lines = imported.line_presentation();
        let line_occurrences = lines
            .occurrences()
            .map(line_occurrence_dto)
            .collect::<Result<Vec<_>, _>>()?;
        if line_occurrences
            .iter()
            .enumerate()
            .any(|(i, line)| line.line_occurrence_index as usize != i)
        {
            return Err("STEP importer returned nonsequential line occurrences".into());
        }
        for segment in lines.segments_mm() {
            if segment
                .iter()
                .flatten()
                .any(|coordinate| !coordinate.is_finite())
            {
                return Err("STEP importer returned invalid source line data".into());
            }
            for point in segment {
                line_segments_mm_f64.extend_from_slice(&point);
            }
        }
        for point in lines.render_positions() {
            if point.iter().any(|coordinate| !coordinate.is_finite()) {
                return Err("STEP importer returned invalid source line data".into());
            }
            line_render_positions.extend_from_slice(point);
        }
        for index in lines.segment_occurrences() {
            segment_line_occurrence_indices.push(index_u32(*index, "line occurrence")?);
        }
        if line_segments_mm_f64.len() % 6 != 0
            || line_render_positions.len() != line_segments_mm_f64.len()
            || segment_line_occurrence_indices.len() != line_segments_mm_f64.len() / 6
            || segment_line_occurrence_indices
                .iter()
                .any(|index| *index as usize >= line_occurrences.len())
            || line_segments_mm_f64
                .iter()
                .zip(&line_render_positions)
                .any(|(source, render)| *source as f32 != *render)
        {
            return Err("STEP importer returned inconsistent source line buffers".into());
        }

        let bounds = imported.bounds();
        let bounds_tolerance = if imported.accuracy().bounds_from_source_geometry {
            0.03
        } else {
            0.0
        };
        if bounds.iter().flatten().any(|value| !value.is_finite())
            || (0..3).any(|axis| bounds[0][axis] > bounds[1][axis])
            || !within_bounds(&surface_positions_mm_f64, &bounds, bounds_tolerance)
            || !within_bounds(&line_segments_mm_f64, &bounds, bounds_tolerance)
        {
            return Err("STEP importer returned bounds that omit asset coordinates".into());
        }

        let mut accuracy = accuracy_dto(imported.accuracy())?;
        accuracy.analysis_error_bound_mm = analysis.source_qualification().total_bound_mm();
        Ok(Self {
            analysis,
            mesh: MeshDto {
                positions: mesh.positions.clone(),
                normals: mesh.normals.clone(),
            },
            bounds: BoundsDto {
                min: bounds[0],
                max: bounds[1],
            },
            vendor_step: VendorStepDto {
                source_sha256: options.source_sha256,
                import_options: ImportOptionsDto {
                    preview_repair_budget_mm: options.preview_repair_budget_mm,
                    target_deflection_mm: options.target_deflection_mm,
                },
                source_faces,
                face_occurrences,
                precision: imported.precision().iter().map(precision_dto).collect(),
                accuracy,
                surface_positions_mm_f64,
                triangle_face_occurrence_indices,
                line_segments_mm_f64,
                line_render_positions,
                line_occurrences,
                segment_line_occurrence_indices,
            },
        })
    }

    fn into_js_value(self) -> Result<JsValue, JsValue> {
        use super::set;
        let result = Object::new();
        let mesh = Object::new();
        set(
            &mesh,
            "positions",
            &Float32Array::from(self.mesh.positions.as_slice()).into(),
        )?;
        set(
            &mesh,
            "normals",
            &Float32Array::from(self.mesh.normals.as_slice()).into(),
        )?;
        set(&result, "mesh", &mesh.into())?;
        let bounds = to_js(&self.bounds)?;
        set(&result, "bounds", &bounds)?;
        let metadata = to_js(&self.vendor_step)?;
        let vendor_step = metadata
            .dyn_into::<Object>()
            .map_err(|_| JsValue::from_str("vendor STEP metadata was not an object"))?;
        set(
            &vendor_step,
            "surfacePositionsMmF64",
            &Float64Array::from(self.vendor_step.surface_positions_mm_f64.as_slice()).into(),
        )?;
        set(
            &vendor_step,
            "triangleFaceOccurrenceIndices",
            &Uint32Array::from(self.vendor_step.triangle_face_occurrence_indices.as_slice()).into(),
        )?;
        set(
            &vendor_step,
            "lineSegmentsMmF64",
            &Float64Array::from(self.vendor_step.line_segments_mm_f64.as_slice()).into(),
        )?;
        set(
            &vendor_step,
            "lineRenderPositions",
            &Float32Array::from(self.vendor_step.line_render_positions.as_slice()).into(),
        )?;
        let line_occurrences = js_sys::Array::new();
        for occurrence in &self.vendor_step.line_occurrences {
            let value = to_js(occurrence)?
                .dyn_into::<Object>()
                .map_err(|_| JsValue::from_str("source line metadata was not an object"))?;
            set(
                &value,
                "placementF64",
                &Float64Array::from(occurrence.placement_f64.as_slice()).into(),
            )?;
            line_occurrences.push(&value);
        }
        set(&vendor_step, "lineOccurrences", &line_occurrences)?;
        set(
            &vendor_step,
            "segmentLineOccurrenceIndices",
            &Uint32Array::from(self.vendor_step.segment_line_occurrence_indices.as_slice()).into(),
        )?;
        super::qualified::add_reports(&vendor_step, &self.analysis)?;
        let handle = super::qualified::vendor_handle(self.analysis, self.vendor_step.source_sha256);
        set(&vendor_step, "analysisHandle", &handle.into())?;
        set(&result, "vendorStep", &vendor_step.into())?;
        Ok(result.into())
    }
}

fn to_js(value: &impl Serialize) -> Result<JsValue, JsValue> {
    value
        .serialize(&serde_wasm_bindgen::Serializer::json_compatible())
        .map_err(super::js_error)
}

#[wasm_bindgen]
pub fn read_step_model(bytes: Uint8Array, options: Option<JsValue>) -> Result<JsValue, JsValue> {
    ReadStepAsset::import(bytes, options)
        .map_err(super::js_error)?
        .into_js_value()
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) struct StepTestData {
    pub(super) mesh: super::MeshData,
    pub(super) min: [f64; 3],
    pub(super) max: [f64; 3],
}

#[cfg(all(test, not(target_arch = "wasm32")))]
pub(super) fn read_step_model_data_for_test(
    bytes: Vec<u8>,
    source_sha256: String,
) -> Result<StepTestData, String> {
    let asset = ReadStepAsset::import_bytes(
        bytes,
        ReadStepOptions {
            source_sha256,
            preview_repair_budget_mm: None,
            target_deflection_mm: None,
        },
    )?;
    Ok(StepTestData {
        mesh: super::MeshData {
            positions: asset.mesh.positions,
            normals: asset.mesh.normals,
        },
        min: asset.bounds.min,
        max: asset.bounds.max,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn index_u32(index: usize, what: &str) -> Result<u32, String> {
    u32::try_from(index).map_err(|_| format!("STEP {what} index exceeds worker buffer range"))
}

fn source_face_dto(face: &StepSourceFace) -> Result<SourceFaceDto, String> {
    Ok(SourceFaceDto {
        solid_id: face.solid_id.to_string(),
        shell_index: index_u32(face.shell_index, "shell")?,
        face_id: face.face_id.to_string(),
        support_id: face.support_id.to_string(),
        support_kind: face.support_kind.clone(),
    })
}

fn face_occurrence_dto(face: &StepFaceOccurrence) -> Result<FaceOccurrenceDto, String> {
    Ok(FaceOccurrenceDto {
        source_face_index: index_u32(face.source_face, "source face")?,
        solid_occurrence_index: index_u32(face.occurrence, "solid occurrence")?,
        representation_id: face.representation_id.to_string(),
        source_uncertainty_mm: face.source_uncertainty_mm,
        repair_displacement_mm: None,
    })
}

fn line_occurrence_dto(
    line: boardstudio_cad::StepLineOccurrence,
) -> Result<LineOccurrenceDto, String> {
    if !line.source_unit_to_mm.is_finite()
        || line.source_unit_to_mm <= 0.0
        || line.placement_f64.iter().any(|value| !value.is_finite())
        || line
            .declared_uncertainty_mm
            .is_some_and(|value| !value.is_finite() || value <= 0.0)
    {
        return Err("STEP importer returned invalid source line metadata".into());
    }
    Ok(LineOccurrenceDto {
        line_occurrence_index: index_u32(line.line_occurrence_index, "line occurrence")?,
        trimmed_curve_id: line.trimmed_curve_id.to_string(),
        source_line_id: line.source_line_id.to_string(),
        curve_set_id: line.curve_set_id.to_string(),
        representation_id: line.representation_id.to_string(),
        source_unit_to_mm: line.source_unit_to_mm,
        placement_f64: line.placement_f64,
        declared_uncertainty_mm: line.declared_uncertainty_mm,
    })
}

fn precision_dto(precision: &StepPrecision) -> PrecisionDto {
    PrecisionDto {
        representation_id: precision.representation_id.to_string(),
        context_id: precision.context_id.to_string(),
        geometric_dimension: precision.geometric_dimension,
        declarations: precision.declarations.iter().map(uncertainty_dto).collect(),
    }
}

fn uncertainty_dto(uncertainty: &StepUncertainty) -> UncertaintyDto {
    UncertaintyDto {
        entity_id: uncertainty.entity_id.to_string(),
        length_unit_id: uncertainty.length_unit_id.to_string(),
        millimetres: uncertainty.millimetres,
    }
}

fn accuracy_dto(accuracy: &StepAccuracy) -> Result<AccuracyDto, String> {
    Ok(AccuracyDto {
        preview_deflection_mm: accuracy.preview_deflection_mm,
        preview_angular_rad: accuracy.preview_angular_rad,
        analysis_error_bound_mm: accuracy.analysis_error_bound_mm,
        bounds_from_source_geometry: accuracy.bounds_from_source_geometry,
        mesh_closed_oriented: accuracy.mesh_closed_oriented,
        preview_floor_mm: accuracy.preview_floor_mm,
        preview_declared_multiplier: accuracy.preview_declared_multiplier,
        selected_repair_budget_mm: accuracy.selected_repair_budget_mm,
        max_repair_displacement_mm: accuracy.max_repair_displacement_mm,
        relaxed_beyond_declared_precision: accuracy.relaxed_beyond_declared_precision,
        sampled_support_checks: index_u32(
            accuracy.sampled_support_checks,
            "sampled support check",
        )?,
        max_sampled_support_residual_mm: accuracy.max_sampled_support_residual_mm,
        fallback_meshed_faces: index_u32(accuracy.fallback_meshed_faces, "fallback meshed face")?,
    })
}

fn within_bounds(positions: &[f64], bounds: &[[f64; 3]; 2], tolerance: f64) -> bool {
    positions.len().is_multiple_of(3)
        && positions.as_chunks::<3>().0.iter().all(|point| {
            (0..3).all(|axis| {
                bounds[0][axis] - tolerance <= point[axis]
                    && point[axis] <= bounds[1][axis] + tolerance
            })
        })
}
