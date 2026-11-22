use super::*;

fn worker_metadata_storage(
    reference: &AnalysisRef,
    descriptor: &AnalysisMesh,
    report: &SourceQualification,
    request: &WireRequest,
) -> Option<usize> {
    let mut strings = vec![
        &reference.worker_generation,
        &reference.mesh_identity,
        &reference.derivation_identity,
        &descriptor.version,
        &descriptor.mesh_identity,
        &descriptor.source_proof,
        &request.header.identity.document_id,
        &request.header.identity.board_id,
        &request.header.request_id,
        &request.header.job_id,
    ];
    if let Some(instance) = &request.header.identity.instance_id {
        strings.push(instance);
    }
    match &reference.source {
        Source::VendorStep { source_sha256, .. } => strings.push(source_sha256),
        Source::GeneratedBody {
            body_id,
            geometry_identity,
            ..
        } => strings.extend([body_id, geometry_identity]),
    }
    let reason_storage = match report {
        SourceQualification::Unknown { reasons } => {
            strings.extend(reasons);
            reasons
                .capacity()
                .checked_mul(std::mem::size_of::<String>())?
        }
        SourceQualification::Bounded {
            class,
            mesh_identity,
            accounting,
            ..
        } => {
            strings.extend([class, mesh_identity, accounting]);
            0
        }
    };
    // Include Rc bookkeeping and a conservative per-entry container allowance;
    // the provider accounts its own entire proof/catalogue allocation separately.
    let inline = std::mem::size_of::<OwnedAnalysis>() + 2 * std::mem::size_of::<usize>() + 1024;
    let bytes = strings
        .into_iter()
        .try_fold(inline.checked_add(reason_storage)?, |n, s| {
            n.checked_add(s.capacity())
        })?;
    bytes.checked_add(reference.mesh_identity.len().checked_mul(2)?)
}

pub(super) fn stage_analysis(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: &WireRequest,
    payload: &JsValue,
    source: Source,
) -> Result<Rc<OwnedAnalysis>, String> {
    let raw_handle = Reflect::get(payload, &"analysisHandle".into()).map_err(js_message)?;
    let class = Reflect::get(module, &"AnalysisHandle".into()).map_err(js_message)?;
    let is_owned = Function::new_with_args(
        "handle,owner",
        "return typeof owner === 'function' && handle instanceof owner",
    )
    .call2(&JsValue::NULL, &raw_handle, &class)
    .map_err(js_message)?
    .as_bool()
        == Some(true);
    if !is_owned {
        return Err("provider must return its opaque AnalysisHandle".into());
    }
    let handle = ProviderHandle(raw_handle);
    Reflect::delete_property(payload.unchecked_ref::<Object>(), &"analysisHandle".into())
        .map_err(js_message)?;
    let descriptor: AnalysisMesh =
        strict_metadata(Reflect::get(payload, &"analysisMesh".into()).map_err(js_message)?)?;
    let report: SourceQualification =
        strict_metadata(Reflect::get(payload, &"sourceQualification".into()).map_err(js_message)?)?;
    let reference = AnalysisRef {
        worker_generation: queue.borrow().generation.clone(),
        mesh_identity: descriptor.mesh_identity.clone(),
        derivation_identity: qualification::derivation_identity(&source, &descriptor),
        source,
    };
    let bound_value = match reference.source {
        Source::GeneratedBody { .. } => {
            Reflect::get(payload, &"analysisErrorBoundMm".into()).map_err(js_message)?
        }
        Source::VendorStep { .. } => Reflect::get(
            &Reflect::get(payload, &"accuracy".into()).map_err(js_message)?,
            &"analysisErrorBoundMm".into(),
        )
        .map_err(js_message)?,
    };
    let bound = if bound_value.is_null() {
        None
    } else {
        Some(bound_value.as_f64().ok_or("invalid source bound")?)
    };
    qualification::validate_analysis(&reference, &descriptor, &report, bound)?;
    let method = Reflect::get(&handle.0, &"retainedStorageBytes".into())
        .map_err(js_message)?
        .dyn_into::<Function>()
        .map_err(js_message)?;
    let cost = method.call0(&handle.0).map_err(js_message)?;
    let positions_value =
        Reflect::get(payload, &"surfacePositionsMmF64".into()).map_err(js_message)?;
    let tags_value =
        Reflect::get(payload, &"triangleFaceOccurrenceIndices".into()).map_err(js_message)?;
    validate_analysis_buffers(&positions_value, &tags_value)?;
    let positions = positions_value.unchecked_into::<Float64Array>().to_vec();
    let tags = tags_value.unchecked_into::<Uint32Array>().to_vec();
    let retained_bytes = if cost.is_null() {
        None
    } else {
        let number = cost.as_f64().ok_or("invalid retained storage cost")?;
        if !number.is_finite()
            || number < 0.0
            || number.fract() != 0.0
            || number > crate::cad_jobs::MAX_CAD_REVISION as f64
        {
            return Err("invalid retained storage cost".into());
        }
        (number as usize)
            .checked_add(positions.capacity().saturating_mul(8))
            .and_then(|n| n.checked_add(tags.capacity().saturating_mul(4)))
            .and_then(|n| {
                n.checked_add(worker_metadata_storage(
                    &reference,
                    &descriptor,
                    &report,
                    request,
                )?)
            })
    };
    Reflect::set(
        payload,
        &"analysisRef".into(),
        &js_sys::JSON::parse(&serde_json::to_string(&reference).map_err(|e| e.to_string())?)
            .map_err(js_message)?,
    )
    .map_err(js_message)?;
    Ok(Rc::new(OwnedAnalysis {
        handle,
        reference,
        descriptor,
        source: report,
        scope: request.header.identity.clone(),
        task: request.header.request_id.clone(),
        job: request.header.job_id.clone(),
        retained_bytes,
        positions,
        tags,
    }))
}
