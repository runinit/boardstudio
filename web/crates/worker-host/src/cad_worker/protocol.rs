use super::*;

pub(super) fn decode_result(value: JsValue) -> Result<CadResult, String> {
    if Reflect::has(&value, &"queryQualification".into()).map_err(js_message)? {
        exact_fields(&value, &["revision", "collision", "queryQualification"])?;
    }
    let revision = Reflect::get(&value, &"revision".into())
        .map_err(js_message)?
        .as_f64()
        .ok_or_else(|| "CAD result omitted revision".to_string())?;
    if !revision.is_finite()
        || revision < 0.0
        || revision.fract() != 0.0
        || revision > crate::cad_jobs::MAX_CAD_REVISION as f64
    {
        return Err("CAD result revision is not an exact u53".into());
    }
    let revision = revision as u64;
    let step = read_u8_buffer(Reflect::get(&value, &"step".into()).map_err(js_message)?)?;
    let mesh_value = Reflect::get(&value, &"mesh".into()).map_err(js_message)?;
    let mesh = if mesh_value.is_null() || mesh_value.is_undefined() {
        None
    } else {
        Some(decode_mesh(mesh_value)?)
    };
    let bodies_value = Reflect::get(&value, &"bodies".into()).map_err(js_message)?;
    let body_values = if bodies_value.is_undefined() {
        Array::new()
    } else if Array::is_array(&bodies_value) {
        Array::from(&bodies_value)
    } else {
        return Err("CAD result bodies must be an array when present".into());
    };
    let mut bodies = Vec::with_capacity(body_values.length() as usize);
    for body in body_values.iter() {
        bodies.push(CadBodyMesh {
            id: string_field(&body, "id")?,
            name: string_field(&body, "name")?,
            positions: read_f32_buffer(
                Reflect::get(&body, &"positions".into()).map_err(js_message)?,
            )?,
            normals: read_f32_buffer(Reflect::get(&body, &"normals".into()).map_err(js_message)?)?,
        });
    }
    let bounds_value = Reflect::get(&value, &"bounds".into()).map_err(js_message)?;
    let bounds = if bounds_value.is_null() || bounds_value.is_undefined() {
        None
    } else {
        Some(serde_wasm_bindgen::from_value(bounds_value).map_err(|error| error.to_string())?)
    };
    let vendor_step_value = Reflect::get(&value, &"vendorStep".into()).map_err(js_message)?;
    let vendor_step = if vendor_step_value.is_null() || vendor_step_value.is_undefined() {
        None
    } else {
        Some(decode_vendor_step(vendor_step_value)?)
    };
    let collision_value = Reflect::get(&value, &"collision".into()).map_err(js_message)?;
    let collision = if collision_value.is_null() || collision_value.is_undefined() {
        None
    } else {
        Some(
            serde_wasm_bindgen::from_value::<CadCollisionObservation>(collision_value)
                .map_err(|error| error.to_string())?,
        )
    };
    Ok(CadResult {
        query_qualification: optional_strict_field(&value, "queryQualification")?,
        revision,
        step,
        mesh,
        bodies,
        bounds,
        vendor_step,
        collision,
        generated_analysis: {
            let analysis = Reflect::get(&value, &"generatedAnalysis".into()).map_err(js_message)?;
            if analysis.is_null() || analysis.is_undefined() {
                None
            } else {
                Some(decode_generated_analysis(analysis)?)
            }
        },
    })
}

pub(super) fn decode_generated_analysis(
    value: JsValue,
) -> Result<crate::cad_jobs::CadGeneratedAnalysis, String> {
    validate_generated_payload_fields(&value, true)?;
    let positions_value =
        Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?;
    let tags_value =
        Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?;
    if !positions_value.is_instance_of::<Float64Array>()
        || !tags_value.is_instance_of::<Uint32Array>()
    {
        return Err(
            "generated analysis requires Float64Array positions and Uint32Array tags".into(),
        );
    }
    let positions = positions_value.unchecked_into::<Float64Array>();
    let tags = tags_value.unchecked_into::<Uint32Array>();
    validate_analysis_buffers(positions.as_ref(), tags.as_ref())?;
    if (positions.byte_length() as usize).saturating_add(tags.byte_length() as usize)
        > MAX_COLLISION_QUERY_BYTES
    {
        return Err("generated analysis exceeds the 64 MiB mesh limit".into());
    }
    // Deserialize metadata separately so large typed arrays retain their transfer path.
    let metadata = Object::new();
    for field in [
        "bodyId",
        "revision",
        "geometryIdentity",
        "sourceFaces",
        "faceOccurrences",
        "analysisErrorBoundMm",
        "meshClosedOriented",
        "deflectionMm",
        "analysisMesh",
        "sourceQualification",
        "analysisRef",
    ] {
        if Reflect::get(&value, &field.into())
            .map_err(js_message)?
            .is_undefined()
        {
            continue;
        }
        Reflect::set(
            &metadata,
            &field.into(),
            &Reflect::get(&value, &field.into()).map_err(js_message)?,
        )
        .map_err(js_message)?;
    }
    Reflect::set(&metadata, &"surfacePositionsMmF64".into(), &Array::new()).map_err(js_message)?;
    Reflect::set(
        &metadata,
        &"triangleFaceOccurrenceIndices".into(),
        &Array::new(),
    )
    .map_err(js_message)?;
    let mut analysis: crate::cad_jobs::CadGeneratedAnalysis =
        serde_wasm_bindgen::from_value(metadata.into()).map_err(|error| error.to_string())?;
    analysis.analysis_mesh = optional_strict_field(&value, "analysisMesh")?;
    analysis.source_qualification = optional_strict_field(&value, "sourceQualification")?;
    analysis.analysis_ref = optional_strict_field(&value, "analysisRef")?;
    analysis.surface_positions_mm_f64 = positions.to_vec();
    analysis.triangle_face_occurrence_indices = tags.to_vec();
    analysis.source_faces =
        strict_metadata(Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?)?;
    analysis.face_occurrences =
        strict_metadata(Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?)?;
    Ok(analysis)
}

pub(super) fn validate_generated_payload_fields(
    value: &JsValue,
    has_identity: bool,
) -> Result<(), String> {
    let mut fields = vec![
        "bodyId",
        "revision",
        "surfacePositionsMmF64",
        "triangleFaceOccurrenceIndices",
        "sourceFaces",
        "faceOccurrences",
        "analysisErrorBoundMm",
        "meshClosedOriented",
        "deflectionMm",
    ];
    if has_identity {
        fields.push("geometryIdentity");
    }
    if Reflect::has(value, &"analysisMesh".into()).unwrap_or(false) {
        fields.extend(["analysisMesh", "sourceQualification"]);
        fields.push(if has_identity {
            "analysisRef"
        } else {
            "analysisHandle"
        });
    }
    let object = value
        .dyn_ref::<Object>()
        .ok_or("generated analysis must be an object")?;
    let keys = Object::keys(object);
    if keys.length() as usize != fields.len()
        || fields
            .iter()
            .any(|field| !Reflect::has(value, &(*field).into()).unwrap_or(false))
        || keys.iter().any(|key| {
            key.as_string()
                .is_none_or(|key| !fields.contains(&key.as_str()))
        })
    {
        return Err("generated analysis has missing or unexpected fields".into());
    }
    Ok(())
}

pub(super) fn decode_vendor_step(value: JsValue) -> Result<CadVendorStep, String> {
    let v2 = Reflect::has(&value, &"analysisMesh".into()).map_err(js_message)?;
    if v2 {
        exact_fields(
            &value,
            &[
                "sourceSha256",
                "importOptions",
                "sourceFaces",
                "faceOccurrences",
                "precision",
                "accuracy",
                "surfacePositionsMmF64",
                "triangleFaceOccurrenceIndices",
                "lineSegmentsMmF64",
                "lineRenderPositions",
                "lineOccurrences",
                "segmentLineOccurrenceIndices",
                "analysisMesh",
                "sourceQualification",
                "analysisRef",
            ],
        )?;
        validate_analysis_buffers(
            &Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?,
            &Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepSourceFace>>(
            Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepFaceOccurrence>>(
            Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?,
        )?;
        strict_metadata::<Vec<CadStepPrecision>>(
            Reflect::get(&value, &"precision".into()).map_err(js_message)?,
        )?;
        strict_step_accuracy(Reflect::get(&value, &"accuracy".into()).map_err(js_message)?)?;
    }
    let source_sha256 = string_field(&value, "sourceSha256")?;
    let options_value = Reflect::get(&value, &"importOptions".into()).map_err(js_message)?;
    if v2 && !options_value.is_null() && !options_value.is_undefined() {
        allowed_fields(
            &options_value,
            &["previewRepairBudgetMm", "targetDeflectionMm"],
        )?;
    }
    let import_options = if options_value.is_null() || options_value.is_undefined() {
        CadStepImportOptions::default()
    } else {
        serde_wasm_bindgen::from_value::<CadStepImportOptions>(options_value)
            .map_err(|error| error.to_string())?
    };
    let surface_positions_mm_f64 = read_f64_buffer(
        Reflect::get(&value, &"surfacePositionsMmF64".into()).map_err(js_message)?,
    )?;
    let source_faces: Vec<CadStepSourceFace> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"sourceFaces".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let face_occurrences: Vec<CadStepFaceOccurrence> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"faceOccurrences".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let triangle_face_occurrence_indices = read_u32_buffer(
        Reflect::get(&value, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?,
    )?;
    let line_segments_mm_f64 =
        read_f64_buffer(Reflect::get(&value, &"lineSegmentsMmF64".into()).map_err(js_message)?)?;
    let line_render_positions =
        read_f32_buffer(Reflect::get(&value, &"lineRenderPositions".into()).map_err(js_message)?)?;
    let line_occurrences_value =
        Reflect::get(&value, &"lineOccurrences".into()).map_err(js_message)?;
    if !Array::is_array(&line_occurrences_value) {
        return Err("STEP vendorStep.lineOccurrences must be an array".into());
    }
    let line_occurrences_array = Array::from(&line_occurrences_value);
    let mut line_occurrences = Vec::with_capacity(line_occurrences_array.length() as usize);
    for item in line_occurrences_array.iter() {
        if v2 {
            exact_fields(
                &item,
                &[
                    "lineOccurrenceIndex",
                    "trimmedCurveId",
                    "sourceLineId",
                    "curveSetId",
                    "representationId",
                    "sourceUnitToMm",
                    "placementF64",
                    "declaredUncertaintyMm",
                ],
            )?;
        }
        let placement_vec =
            read_f64_buffer(Reflect::get(&item, &"placementF64".into()).map_err(js_message)?)?;
        let placement_f64: [f64; 16] = placement_vec
            .try_into()
            .map_err(|_| "STEP lineOccurrence placementF64 must have 16 values".to_string())?;
        let line_occurrence_index = Reflect::get(&item, &"lineOccurrenceIndex".into())
            .map_err(js_message)?
            .as_f64()
            .ok_or_else(|| "STEP lineOccurrence omitted lineOccurrenceIndex".to_string())?;
        if !line_occurrence_index.is_finite()
            || line_occurrence_index < 0.0
            || line_occurrence_index > u32::MAX as f64
            || line_occurrence_index.fract() != 0.0
        {
            return Err("STEP lineOccurrence lineOccurrenceIndex is invalid".into());
        }
        let source_unit_to_mm = Reflect::get(&item, &"sourceUnitToMm".into())
            .map_err(js_message)?
            .as_f64()
            .ok_or_else(|| "STEP lineOccurrence omitted sourceUnitToMm".to_string())?;
        let declared_uncertainty_value =
            Reflect::get(&item, &"declaredUncertaintyMm".into()).map_err(js_message)?;
        let declared_uncertainty_mm =
            if declared_uncertainty_value.is_null() || declared_uncertainty_value.is_undefined() {
                None
            } else {
                Some(declared_uncertainty_value.as_f64().ok_or_else(|| {
                    "STEP lineOccurrence declaredUncertaintyMm must be a number or null".to_string()
                })?)
            };
        line_occurrences.push(CadStepLineOccurrence {
            line_occurrence_index: line_occurrence_index as u32,
            trimmed_curve_id: string_field(&item, "trimmedCurveId")?,
            source_line_id: string_field(&item, "sourceLineId")?,
            curve_set_id: string_field(&item, "curveSetId")?,
            representation_id: string_field(&item, "representationId")?,
            source_unit_to_mm,
            placement_f64,
            declared_uncertainty_mm,
        });
    }
    let segment_line_occurrence_indices = read_u32_buffer(
        Reflect::get(&value, &"segmentLineOccurrenceIndices".into()).map_err(js_message)?,
    )?;
    let precision: Vec<CadStepPrecision> = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"precision".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    let accuracy: CadStepAccuracy = serde_wasm_bindgen::from_value(
        Reflect::get(&value, &"accuracy".into()).map_err(js_message)?,
    )
    .map_err(|error| error.to_string())?;
    Ok(CadVendorStep {
        analysis_ref: optional_strict_field(&value, "analysisRef")?,
        analysis_mesh: optional_strict_field(&value, "analysisMesh")?,
        source_qualification: optional_strict_field(&value, "sourceQualification")?,
        source_sha256,
        import_options,
        surface_positions_mm_f64,
        source_faces,
        face_occurrences,
        triangle_face_occurrence_indices,
        line_segments_mm_f64,
        line_render_positions,
        line_occurrences,
        segment_line_occurrence_indices,
        precision,
        accuracy,
    })
}

pub(super) fn strict_step_accuracy(value: JsValue) -> Result<CadStepAccuracy, String> {
    // This pre-v2 field is omitted by the producer when no repair was requested.
    // Normalize only that established omission; v2 nullable fields stay required.
    let object = value
        .dyn_ref::<Object>()
        .ok_or("accuracy object required")?;
    let normalized = Object::assign(&Object::new(), object);
    let key = JsValue::from_str("selectedRepairBudgetMm");
    if !Object::has_own(&normalized, &key) {
        Reflect::set(&normalized, &key, &JsValue::NULL).map_err(js_message)?;
    }
    strict_metadata(normalized.into())
}

pub(super) fn strict_metadata<T: serde::de::DeserializeOwned + Serialize>(
    value: JsValue,
) -> Result<T, String> {
    let decoded: T = serde_wasm_bindgen::from_value(value.clone()).map_err(|e| e.to_string())?;
    let expected =
        js_sys::JSON::parse(&serde_json::to_string(&decoded).map_err(|e| e.to_string())?)
            .map_err(js_message)?;
    fn keys(actual: &JsValue, expected: &JsValue) -> Result<(), String> {
        if Array::is_array(expected) {
            if !Array::is_array(actual) {
                return Err("metadata array required".into());
            }
            let a = Array::from(actual);
            let e = Array::from(expected);
            if a.length() != e.length() {
                return Err("metadata array length mismatch".into());
            }
            for i in 0..a.length() {
                keys(&a.get(i), &e.get(i))?;
            }
        } else if expected.is_object() && !expected.is_null() {
            let a = actual
                .dyn_ref::<Object>()
                .ok_or("metadata object required")?;
            let e = expected.unchecked_ref::<Object>();
            let names = Object::keys(e);
            if Object::keys(a).length() != names.length() {
                return Err("missing or unexpected metadata fields".into());
            }
            for name in names.iter() {
                if !Object::has_own(a, &name) {
                    return Err("missing own metadata field".into());
                }
                keys(
                    &Reflect::get(actual, &name).map_err(js_message)?,
                    &Reflect::get(expected, &name).map_err(js_message)?,
                )?;
            }
        } else if expected.is_null() && !actual.is_null() {
            return Err("explicit nullable metadata required".into());
        }
        Ok(())
    }
    keys(&value, &expected)?;
    Ok(decoded)
}

pub(super) fn exact_fields(value: &JsValue, fields: &[&str]) -> Result<(), String> {
    let object = value
        .dyn_ref::<Object>()
        .ok_or("metadata object required")?;
    if Object::keys(object).length() as usize != fields.len()
        || fields
            .iter()
            .any(|field| !Object::has_own(object, &(*field).into()))
    {
        return Err("missing or unexpected metadata fields".into());
    }
    Ok(())
}

fn allowed_fields(value: &JsValue, fields: &[&str]) -> Result<(), String> {
    let object = value
        .dyn_ref::<Object>()
        .ok_or("metadata object required")?;
    if Object::keys(object).iter().any(|key| {
        !key.as_string()
            .is_some_and(|key| fields.contains(&key.as_str()))
    }) {
        return Err("unexpected metadata field".into());
    }
    Ok(())
}

pub(super) fn raw_query_metadata(query: &CadCollisionQuery) -> Result<JsValue, String> {
    // The retained raw export has strict v1 keys. Selectors stay in the worker;
    // dropping them here cannot confer qualification on this fallback.
    let mut value = serde_json::to_value(query).map_err(|e| e.to_string())?;
    let object = value.as_object_mut().ok_or("raw query metadata object")?;
    object.remove("qualificationLimits");
    for side in ["left", "right"] {
        let operand = object
            .get_mut(side)
            .and_then(serde_json::Value::as_object_mut)
            .ok_or("raw query operand object")?;
        operand.remove("analysisRef");
        operand.remove("analysisMesh");
    }
    js_sys::JSON::parse(&value.to_string()).map_err(js_message)
}

fn optional_strict_field<T: serde::de::DeserializeOwned + Serialize>(
    value: &JsValue,
    field: &str,
) -> Result<Option<T>, String> {
    let value = Reflect::get(value, &field.into()).map_err(js_message)?;
    if value.is_undefined() {
        Ok(None)
    } else {
        strict_metadata(value).map(Some)
    }
}

pub(super) fn decode_mesh(value: JsValue) -> Result<CadMesh, String> {
    Ok(CadMesh {
        positions: read_f32_buffer(Reflect::get(&value, &"positions".into()).map_err(js_message)?)?,
        normals: read_f32_buffer(Reflect::get(&value, &"normals".into()).map_err(js_message)?)?,
    })
}

pub(super) fn string_field(value: &JsValue, name: &str) -> Result<String, String> {
    Reflect::get(value, &name.into())
        .map_err(js_message)?
        .as_string()
        .ok_or_else(|| format!("CAD result omitted {name}"))
}

fn read_f32_buffer(value: JsValue) -> Result<Vec<f32>, String> {
    if value.is_instance_of::<Float32Array>() {
        Ok(value.unchecked_into::<Float32Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn read_f64_buffer(value: JsValue) -> Result<Vec<f64>, String> {
    if value.is_instance_of::<Float64Array>() {
        Ok(value.unchecked_into::<Float64Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

pub(super) fn validate_analysis_buffers(positions: &JsValue, tags: &JsValue) -> Result<(), String> {
    let positions = positions
        .dyn_ref::<Float64Array>()
        .ok_or("analysis positions require Float64Array")?;
    let tags = tags
        .dyn_ref::<Uint32Array>()
        .ok_or("analysis tags require Uint32Array")?;
    if !positions.buffer().is_instance_of::<js_sys::ArrayBuffer>()
        || !tags.buffer().is_instance_of::<js_sys::ArrayBuffer>()
        || positions.length() == 0
        || positions.length() % 9 != 0
        || tags.length() != positions.length() / 9
        || (positions.byte_length() as usize).saturating_add(tags.byte_length() as usize)
            > MAX_COLLISION_QUERY_BYTES
    {
        return Err("detached/shared/malformed or oversized analysis buffers".into());
    }
    Ok(())
}

fn read_u32_buffer(value: JsValue) -> Result<Vec<u32>, String> {
    if value.is_instance_of::<Uint32Array>() {
        Ok(value.unchecked_into::<Uint32Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}

fn read_u8_buffer(value: JsValue) -> Result<Vec<u8>, String> {
    if value.is_null() || value.is_undefined() {
        return Ok(Vec::new());
    }
    if value.is_instance_of::<Uint8Array>() {
        Ok(value.unchecked_into::<Uint8Array>().to_vec())
    } else {
        serde_wasm_bindgen::from_value(value).map_err(|error| error.to_string())
    }
}
