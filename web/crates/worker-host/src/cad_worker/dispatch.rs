use super::*;

#[allow(clippy::result_large_err)]
pub(super) async fn run_request(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: WireRequest,
    bytes: Vec<u8>,
) -> Result<(WireReply, JsValue), (WireRequest, String, bool)> {
    let canceled = || queue.borrow().cancelled.contains(&request.job_id);
    if canceled() {
        return Err((request, "CAD job cancelled".into(), true));
    }
    let invoke = |name: &str, args: &[JsValue]| -> Result<JsValue, String> {
        let function = Reflect::get(module, &JsValue::from_str(name))
            .map_err(js_message)?
            .dyn_into::<Function>()
            .map_err(js_message)?;
        let args = Array::from_iter(args.iter());
        function.apply(&JsValue::NULL, &args).map_err(js_message)
    };
    let prepared_value = || -> Result<JsValue, String> {
        let prepared = request
            .prepared
            .as_ref()
            .ok_or_else(|| "CAD request omitted prepared case".to_owned())?;
        let text = serde_json::to_string(prepared).map_err(|error| error.to_string())?;
        js_sys::JSON::parse(&text).map_err(js_message)
    };
    let mut staged = None;
    let mut pinned = Vec::new();
    let result = match request.operation {
        CadOperation::AnalyzeGeneratedBody => {
            let metadata = request.generated_analysis_request.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "generated analysis omitted metadata".into(),
                    false,
                )
            })?;
            let body = crate::cad_jobs::validate_generated_analysis_request(
                request.prepared.as_ref(),
                request.identity.revision,
                metadata,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let body_value = js_sys::JSON::parse(
                &serde_json::to_string(body)
                    .map_err(|error| (request.clone(), error.to_string(), false))?,
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            let options = Object::new();
            if let Some(target) = metadata.target_deflection_mm {
                Reflect::set(&options, &"targetDeflectionMm".into(), &target.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            let analysis = invoke("analyze_generated_body", &[body_value, options.into()])
                .map_err(|error| (request.clone(), error, false))?;
            staged = Some(
                stage_analysis(
                    queue,
                    module,
                    &request,
                    &analysis,
                    Source::GeneratedBody {
                        body_id: metadata.body_id.clone(),
                        revision: request.identity.revision,
                        geometry_identity: metadata.geometry_identity.clone(),
                    },
                )
                .map_err(|error| (request.clone(), error, false))?,
            );
            if string_field(&analysis, "bodyId").ok().as_deref() != Some(metadata.body_id.as_str())
                || Reflect::get(&analysis, &"revision".into())
                    .ok()
                    .and_then(|v| v.as_f64())
                    != Some(request.identity.revision as f64)
            {
                return Err((
                    request.clone(),
                    "generated provider returned another body or revision".into(),
                    false,
                ));
            }
            Reflect::set(
                &analysis,
                &"geometryIdentity".into(),
                &metadata.geometry_identity.clone().into(),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            let decoded = decode_generated_analysis(analysis.clone())
                .map_err(|error| (request.clone(), error, false))?;
            crate::cad_jobs::validate_generated_analysis(
                metadata,
                request.identity.revision,
                &decoded,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &(request.identity.revision as f64).into(),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            Reflect::set(&wrapper, &"generatedAnalysis".into(), &analysis)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            wrapper.into()
        }
        CadOperation::Preview | CadOperation::PrintKeycaps => {
            if let Some(keycaps) = request.keycaps.as_ref() {
                // Match React's buildKeycaps adapter: yield between groups so the
                // dedicated worker can receive cancellation before starting more CAD.
                let bodies = Array::new();
                let printable = request.operation == CadOperation::PrintKeycaps;
                let batch_size = if printable {
                    1
                } else {
                    KEYCAPS_PREVIEW_BATCH_SIZE
                };
                for specs in keycaps.specs.chunks(batch_size) {
                    if canceled() {
                        return Err((request, "CAD job cancelled".into(), true));
                    }
                    let input = serde_json::to_string(&serde_json::json!({
                        "revision": keycaps.revision,
                        "specs": specs,
                        "printable": printable,
                    }))
                    .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let input = js_sys::JSON::parse(&input)
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    let batch = invoke("build_keycaps", &[input])
                        .map_err(|error| (request.clone(), error, false))?;
                    let revision = Reflect::get(&batch, &"revision".into())
                        .map_err(|error| (request.clone(), js_message(error), false))?
                        .as_f64();
                    if revision != Some(request.identity.revision as f64) {
                        return Err((
                            request,
                            "Keycap preview provider returned another revision".into(),
                            false,
                        ));
                    }
                    let batch_bodies = Reflect::get(&batch, &"bodies".into())
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    for body in Array::from(&batch_bodies).iter() {
                        bodies.push(&body);
                    }
                    yield_to_worker().await;
                }
                if canceled() {
                    return Err((request, "CAD job cancelled".into(), true));
                }
                let result = Object::new();
                Reflect::set(
                    &result,
                    &"revision".into(),
                    &JsValue::from_f64(request.identity.revision as f64),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&result, &"bodies".into(), &bodies)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                result.into()
            } else {
                let prepared = request.prepared.as_ref().ok_or_else(|| {
                    (
                        request.clone(),
                        "CAD request omitted prepared case".to_owned(),
                        false,
                    )
                })?;
                let bodies = Array::new();
                for body in &prepared.bodies {
                    if canceled() {
                        return Err((request, "CAD job cancelled".into(), true));
                    }
                    let body_json = serde_json::to_string(body)
                        .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let body_value = js_sys::JSON::parse(&body_json)
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    let mut body_geometry = serde_json::to_value(&body.body)
                        .map_err(|error| (request.clone(), error.to_string(), false))?;
                    if let Some(fields) = body_geometry.as_object_mut() {
                        fields.remove("id");
                        fields.remove("name");
                    }
                    let cache_input = serde_json::to_vec(&serde_json::json!({
                        "body": body_geometry,
                        "regions": &body.regions,
                    }))
                    .map_err(|error| (request.clone(), error.to_string(), false))?;
                    let digest = sha256_hex(&cache_input);
                    let key = format!(
                        "m1/{}/{}/{}/{}/{digest}",
                        request.identity.session_epoch,
                        request.identity.document_id,
                        request.identity.board_id,
                        request.identity.instance_id.as_deref().unwrap_or("board")
                    );
                    let progress = Closure::<dyn FnMut(JsValue, JsValue)>::new(|_, _| {});
                    let output = invoke(
                        "preview_body",
                        &[
                            body_value,
                            JsValue::from_str(&key),
                            progress.as_ref().clone(),
                        ],
                    )
                    .map_err(|error| (request.clone(), error, false))?;
                    let object = Object::new();
                    Reflect::set(&object, &"id".into(), &JsValue::from_str(&body.body.id))
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    Reflect::set(&object, &"name".into(), &JsValue::from_str(&body.body.name))
                        .map_err(|error| (request.clone(), js_message(error), false))?;
                    for field in ["positions", "normals"] {
                        let value = Reflect::get(&output, &field.into())
                            .map_err(|error| (request.clone(), js_message(error), false))?;
                        Reflect::set(&object, &field.into(), &value)
                            .map_err(|error| (request.clone(), js_message(error), false))?;
                    }
                    bodies.push(&object);
                    yield_to_worker().await;
                }
                let result = Object::new();
                Reflect::set(
                    &result,
                    &"revision".into(),
                    &JsValue::from_f64(prepared.revision as f64),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&result, &"bodies".into(), &bodies)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                result.into()
            }
        }
        CadOperation::Exact => invoke(
            "build_assembly",
            &[prepared_value().map_err(|error| (request.clone(), error, false))?],
        )
        .map_err(|error| (request.clone(), error, false))?,
        CadOperation::ExportStep => {
            let prepared = prepared_value().map_err(|error| (request.clone(), error, false))?;
            Reflect::set(&prepared, &"stepOnly".into(), &JsValue::TRUE)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            let bodies = Array::from(
                &Reflect::get(&prepared, &"bodies".into())
                    .map_err(|error| (request.clone(), js_message(error), false))?,
            );
            // This worker owns a fresh module/cache. Empty keys intentionally
            // miss preview meshes while the provider rebuilds exact solids
            // from the same prepared regions without allocating triangulations.
            let keys = Array::new_with_length(bodies.length());
            for index in 0..bodies.length() {
                keys.set(index, JsValue::from_str("m1-independent-step-cache-miss"));
            }
            invoke("export_cached_assembly", &[prepared, keys.into()])
                .map_err(|error| (request.clone(), error, false))?
        }
        CadOperation::ReadStep => {
            if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
                return Err((
                    request,
                    "STEP import failed: invalid file size".into(),
                    false,
                ));
            }
            let expected = request.source_sha256.as_deref().unwrap_or_default();
            if !crate::cad_jobs::is_sha256_hex(expected) || sha256_hex(&bytes) != expected {
                return Err((
                    request,
                    "STEP source failed SHA-256 verification".into(),
                    false,
                ));
            }
            let step_options = request
                .step_import_options
                .unwrap_or_default()
                .validate()
                .map_err(|error| (request.clone(), error, false))?;
            let options = Object::new();
            Reflect::set(
                &options,
                &"sourceSha256".into(),
                &JsValue::from_str(expected),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            if let Some(budget) = step_options.preview_repair_budget_mm {
                Reflect::set(
                    &options,
                    &"previewRepairBudgetMm".into(),
                    &JsValue::from_f64(budget),
                )
                .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            if let Some(target) = step_options.target_deflection_mm {
                Reflect::set(&options, &"targetDeflectionMm".into(), &target.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            let result = invoke(
                "read_step_model",
                &[Uint8Array::from(bytes.as_slice()).into(), options.into()],
            )
            .map_err(|error| (request.clone(), error, false))?;
            let vendor = Reflect::get(&result, &"vendorStep".into())
                .map_err(|e| (request.clone(), js_message(e), false))?;
            staged = Some(
                stage_analysis(
                    queue,
                    module,
                    &request,
                    &vendor,
                    Source::VendorStep {
                        source_sha256: expected.into(),
                        import_options: step_options.into(),
                    },
                )
                .map_err(|error| (request.clone(), error, false))?,
            );
            exact_fields(&result, &["mesh", "bounds", "vendorStep"])
                .map_err(|error| (request.clone(), error, false))?;
            let decoded =
                decode_vendor_step(vendor).map_err(|error| (request.clone(), error, false))?;
            let mesh = decode_mesh(
                Reflect::get(&result, &"mesh".into())
                    .map_err(|e| (request.clone(), js_message(e), false))?,
            )
            .map_err(|error| (request.clone(), error, false))?;
            let bounds = strict_metadata(
                Reflect::get(&result, &"bounds".into())
                    .map_err(|e| (request.clone(), js_message(e), false))?,
            )
            .map_err(|error| (request.clone(), error, false))?;
            crate::cad_jobs::validate_vendor_step(expected, step_options, &mesh, &bounds, &decoded)
                .map_err(|error| (request.clone(), error, false))?;
            result
        }
        CadOperation::CollisionQuery => {
            let query = request.collision_query.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "collision query omitted metadata".into(),
                    false,
                )
            })?;
            validate_collision_query(query).map_err(|error| (request.clone(), error, false))?;
            let meshes = request.collision_meshes.as_ref().ok_or_else(|| {
                (
                    request.clone(),
                    "collision query omitted mesh buffers".into(),
                    false,
                )
            })?;
            validate_collision_mesh_pair(meshes)
                .map_err(|error| (request.clone(), error, false))?;
            let query_json = serde_json::to_string(query)
                .map_err(|error| (request.clone(), error.to_string(), false))?;
            let query_value = js_sys::JSON::parse(&query_json)
                .map_err(|error| (request.clone(), js_message(error), false))?;
            let left = queue
                .borrow_mut()
                .lookup(
                    &query.left,
                    &request.identity,
                    &meshes.left_positions_mm_f64,
                    &meshes.left_triangle_face_occurrence_indices,
                )
                .map_err(|error| (request.clone(), error, false))?;
            let right = queue
                .borrow_mut()
                .lookup(
                    &query.right,
                    &request.identity,
                    &meshes.right_positions_mm_f64,
                    &meshes.right_triangle_face_occurrence_indices,
                )
                .map_err(|error| (request.clone(), error, false))?;
            let generation = queue.borrow().generation.clone();
            let expected = qualification::expected_binding(query, &request.identity, &generation)
                .map_err(|error| (request.clone(), error, false))?;
            let result = if let (Some(left), Some(right)) = (left, right) {
                let buffers = Object::new();
                for (key, value) in [
                    (
                        "leftPositions",
                        Float64Array::from(meshes.left_positions_mm_f64.as_slice()).into(),
                    ),
                    (
                        "leftTags",
                        Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice())
                            .into(),
                    ),
                    (
                        "rightPositions",
                        Float64Array::from(meshes.right_positions_mm_f64.as_slice()).into(),
                    ),
                    (
                        "rightTags",
                        Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice())
                            .into(),
                    ),
                ] {
                    Reflect::set(&buffers, &key.into(), &value)
                        .map_err(|e| (request.clone(), js_message(e), false))?;
                }
                let context = js_sys::JSON::parse(&serde_json::json!({"snapshot": qualification::Snapshot::from(&request.identity), "workerGeneration": generation}).to_string())
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                let result = invoke(
                    "query_qualified_collision_mesh_pair",
                    &[
                        buffers.into(),
                        query_value,
                        context,
                        left.handle.0.clone(),
                        right.handle.0.clone(),
                    ],
                )
                .map_err(|error| (request.clone(), error, false))?;
                let report: qualification::QueryQualification = strict_metadata(
                    Reflect::get(&result, &"queryQualification".into())
                        .map_err(|e| (request.clone(), js_message(e), false))?,
                )
                .map_err(|error| (request.clone(), error, false))?;
                qualification::validate_query_qualification(
                    report,
                    &expected,
                    &query.qualification_limits,
                    meshes,
                    Some([&left.source, &right.source]),
                )
                .map_err(|error| (request.clone(), error, false))?;
                pinned.extend([left, right]);
                result
            } else {
                let raw = invoke(
                    "query_collision_mesh_pair",
                    &[
                        Float64Array::from(meshes.left_positions_mm_f64.as_slice()).into(),
                        Uint32Array::from(meshes.left_triangle_face_occurrence_indices.as_slice())
                            .into(),
                        Float64Array::from(meshes.right_positions_mm_f64.as_slice()).into(),
                        Uint32Array::from(meshes.right_triangle_face_occurrence_indices.as_slice())
                            .into(),
                        raw_query_metadata(query)
                            .map_err(|error| (request.clone(), error, false))?,
                    ],
                )
                .map_err(|error| (request.clone(), error, false))?;
                let result = Object::new();
                Reflect::set(&result, &"observation".into(), &raw)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                let unknown =
                    qualification::unavailable(expected, query.qualification_limits.clone());
                let report = js_sys::JSON::parse(
                    &serde_json::to_string(&unknown)
                        .map_err(|e| (request.clone(), e.to_string(), false))?,
                )
                .map_err(|e| (request.clone(), js_message(e), false))?;
                Reflect::set(&result, &"queryQualification".into(), &report)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                result.into()
            };
            let object = result.dyn_ref::<Object>().ok_or_else(|| {
                (
                    request.clone(),
                    "qualified result must be an object".into(),
                    false,
                )
            })?;
            if Object::keys(object).length() != 2
                || !Object::has_own(object, &"observation".into())
                || !Object::has_own(object, &"queryQualification".into())
            {
                return Err((
                    request.clone(),
                    "qualified provider result has unexpected keys".into(),
                    false,
                ));
            }
            result
        }
    };
    // Let queued cancel messages run after synchronous CAD calls before deciding
    // whether their result may cross the worker boundary.
    yield_to_worker().await;
    if canceled() {
        return Err((
            request,
            "CAD job cancelled after the synchronous kernel call".into(),
            true,
        ));
    }
    if queue
        .borrow()
        .latest_snapshot
        .as_ref()
        .is_some_and(|latest| latest != &request.identity)
    {
        return Err((
            request,
            "CAD result became stale after kernel yield".into(),
            false,
        ));
    }
    if let Some(entry) = staged {
        queue
            .borrow_mut()
            .retain_analysis(entry)
            .map_err(|e| (request.clone(), e, false))?;
    }
    drop(pinned);
    let result = match request.operation {
        CadOperation::ReadStep => {
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &JsValue::from_f64(request.identity.revision as f64),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            for field in ["mesh", "bounds", "vendorStep"] {
                let value = Reflect::get(&result, &field.into())
                    .map_err(|error| (request.clone(), js_message(error), false))?;
                Reflect::set(&wrapper, &field.into(), &value)
                    .map_err(|error| (request.clone(), js_message(error), false))?;
            }
            wrapper.into()
        }
        CadOperation::CollisionQuery => {
            let wrapper = Object::new();
            Reflect::set(
                &wrapper,
                &"revision".into(),
                &JsValue::from_f64(request.identity.revision as f64),
            )
            .map_err(|error| (request.clone(), js_message(error), false))?;
            for (target, source) in [
                ("collision", "observation"),
                ("queryQualification", "queryQualification"),
            ] {
                let value = Reflect::get(&result, &source.into())
                    .map_err(|e| (request.clone(), js_message(e), false))?;
                Reflect::set(&wrapper, &target.into(), &value)
                    .map_err(|e| (request.clone(), js_message(e), false))?;
            }
            wrapper.into()
        }
        _ => result,
    };
    let wire = WireReply {
        request_id: request.request_id.clone(),
        job_id: request.job_id.clone(),
        identity: request.identity.clone(),
        operation: request.operation,
        outcome: CadReplyOutcome::Completed,
        error: None,
    };
    Ok((wire, result))
}

pub(super) async fn yield_to_worker() {
    if let Some(promise) =
        Function::new_no_args("return new Promise(resolve => setTimeout(resolve, 0))")
            .call0(&JsValue::NULL)
            .ok()
            .and_then(|value| value.dyn_into::<Promise>().ok())
    {
        let _ = JsFuture::from(promise).await;
    }
}
