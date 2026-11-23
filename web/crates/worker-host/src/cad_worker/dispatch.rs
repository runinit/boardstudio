use super::*;
use boardstudio_model::model::PreparedCaseIR;

type OperationError = (String, bool);

fn failed(reason: impl Into<String>) -> OperationError {
    (reason.into(), false)
}

#[allow(clippy::result_large_err)]
pub(super) async fn run_request(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: WireRequest,
    bytes: Vec<u8>,
) -> Result<(WireReply, JsValue), (CadEnvelope, String, bool)> {
    let mut staged = None;
    let mut pinned = Vec::new();
    let result = async {
        let result =
            execute_operation(queue, module, &request, &bytes, &mut staged, &mut pinned).await?;
        // Let queued cancel messages run after synchronous CAD calls before publishing.
        yield_to_worker().await;
        if queue.borrow().cancelled.contains(&request.header.job_id) {
            return Err((
                "CAD job cancelled after the synchronous kernel call".into(),
                true,
            ));
        }
        if queue
            .borrow()
            .latest_snapshot
            .as_ref()
            .is_some_and(|latest| latest != &request.header.identity)
        {
            return Err(failed("CAD result became stale after kernel yield"));
        }
        if let Some(entry) = staged {
            queue.borrow_mut().retain_analysis(entry).map_err(failed)?;
        }
        drop(pinned);
        let fields: &[(&str, &str)] = match request.header.operation {
            CadOperation::ReadStep => &[
                ("mesh", "mesh"),
                ("bounds", "bounds"),
                ("vendorStep", "vendorStep"),
            ],
            CadOperation::CollisionQuery => &[
                ("collision", "observation"),
                ("queryQualification", "queryQualification"),
            ],
            _ => return Ok(result),
        };
        let reply = revision_object(request.header.identity.revision)?;
        for &(field, source) in fields {
            js_set(&reply, field, &js_get(&result, source).map_err(failed)?)?;
        }
        Ok(reply.into())
    }
    .await
    .map_err(|(reason, cancelled)| (request.header.clone(), reason, cancelled))?;
    Ok((
        WireReply {
            header: request.header,
            outcome: CadReplyOutcome::Completed,
            error: None,
        },
        result,
    ))
}

async fn execute_operation(
    queue: &Rc<RefCell<WorkerQueue>>,
    module: &JsValue,
    request: &WireRequest,
    bytes: &[u8],
    staged: &mut Option<Rc<OwnedAnalysis>>,
    pinned: &mut Vec<Rc<OwnedAnalysis>>,
) -> Result<JsValue, OperationError> {
    let identity = &request.header.identity;
    let check_cancelled = || {
        if queue.borrow().cancelled.contains(&request.header.job_id) {
            Err(("CAD job cancelled".into(), true))
        } else {
            Ok(())
        }
    };
    check_cancelled()?;
    let get = |value: &JsValue, field: &str| js_get(value, field).map_err(failed);
    let invoke = |name: &str, args: &[JsValue]| {
        get(module, name)?
            .dyn_into::<Function>()
            .and_then(|function| function.apply(&JsValue::NULL, &Array::from_iter(args.iter())))
            .map_err(|error| failed(js_message(error)))
    };
    let prepare = || {
        request
            .prepared
            .as_ref()
            .ok_or_else(|| failed("CAD request omitted prepared case"))
    };
    Ok(match request.header.operation {
        CadOperation::AnalyzeGeneratedBody => {
            let metadata = request
                .generated_analysis_request
                .as_ref()
                .ok_or_else(|| failed("generated analysis omitted metadata"))?;
            let body = crate::cad_jobs::validate_generated_analysis_request(
                request.prepared.as_ref(),
                identity.revision,
                metadata,
            )
            .map_err(failed)?;
            let options = Object::new();
            if let Some(target) = metadata.target_deflection_mm {
                js_set(&options, "targetDeflectionMm", &target.into())?;
            }
            let analysis = invoke(
                "analyze_generated_body",
                &[json_to_js(body)?, options.into()],
            )?;
            *staged = Some(
                stage_analysis(
                    queue,
                    module,
                    request,
                    &analysis,
                    Source::GeneratedBody {
                        body_id: metadata.body_id.clone(),
                        revision: identity.revision,
                        geometry_identity: metadata.geometry_identity.clone(),
                    },
                )
                .map_err(failed)?,
            );
            if string_field(&analysis, "bodyId").ok().as_deref() != Some(metadata.body_id.as_str())
                || get(&analysis, "revision").ok().and_then(|v| v.as_f64())
                    != Some(identity.revision as f64)
            {
                return Err(failed(
                    "generated provider returned another body or revision",
                ));
            }
            js_set(
                &analysis,
                "geometryIdentity",
                &JsValue::from_str(&metadata.geometry_identity),
            )?;
            let decoded = decode_generated_analysis(analysis.clone()).map_err(failed)?;
            crate::cad_jobs::validate_generated_analysis(metadata, identity.revision, &decoded)
                .map_err(failed)?;
            let result = revision_object(identity.revision)?;
            js_set(&result, "generatedAnalysis", &analysis)?;
            result.into()
        }
        CadOperation::Preview | CadOperation::PrintKeycaps => {
            let bodies = Array::new();
            let revision = if let Some(keycaps) = request.keycaps.as_ref() {
                // Yield between batches so the worker can receive cancellation before more CAD.
                let printable = request.header.operation == CadOperation::PrintKeycaps;
                let batch_size = if printable {
                    1
                } else {
                    KEYCAPS_PREVIEW_BATCH_SIZE
                };
                for specs in keycaps.specs.chunks(batch_size) {
                    check_cancelled()?;
                    let input = json_to_js(&serde_json::json!({
                        "revision": keycaps.revision,
                        "specs": specs,
                        "printable": printable,
                    }))?;
                    let batch = invoke("build_keycaps", &[input])?;
                    if get(&batch, "revision")?.as_f64() != Some(identity.revision as f64) {
                        return Err(failed("Keycap preview provider returned another revision"));
                    }
                    for body in Array::from(&get(&batch, "bodies")?).iter() {
                        bodies.push(&body);
                    }
                    yield_to_worker().await;
                }
                check_cancelled()?;
                identity.revision
            } else {
                let prepared = prepare()?;
                for body in &prepared.bodies {
                    check_cancelled()?;
                    let key = prepared_body_cache_key(identity, body).map_err(failed)?;
                    let progress = Closure::<dyn FnMut(JsValue, JsValue)>::new(|_, _| {});
                    let output = invoke(
                        "preview_body",
                        &[
                            json_to_js(body)?,
                            JsValue::from_str(&key),
                            progress.as_ref().clone(),
                        ],
                    )?;
                    let object = Object::new();
                    js_set(&object, "id", &JsValue::from_str(&body.body.id))?;
                    js_set(&object, "name", &JsValue::from_str(&body.body.name))?;
                    for field in ["positions", "normals"] {
                        js_set(&object, field, &get(&output, field)?)?;
                    }
                    bodies.push(&object);
                    yield_to_worker().await;
                }
                prepared.revision
            };
            let result = revision_object(revision)?;
            js_set(&result, "bodies", &bodies)?;
            result.into()
        }
        CadOperation::Exact | CadOperation::ExportStep => {
            let prepared = prepare()?;
            let input = json_to_js(prepared)?;
            let keys = Array::new();
            let step_only = request.header.operation == CadOperation::ExportStep;
            js_set(&input, "stepOnly", &JsValue::from_bool(step_only))?;
            for body in &prepared.bodies {
                // ADR-0016: STEP-only export runs independently and deliberately misses.
                let key = if step_only {
                    "m1-independent-step-cache-miss".into()
                } else {
                    prepared_body_cache_key(identity, body).map_err(failed)?
                };
                keys.push(&JsValue::from_str(&key));
            }
            invoke("export_cached_assembly", &[input, keys.into()])?
        }
        CadOperation::ReadStep => {
            if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
                return Err(failed("STEP import failed: invalid file size"));
            }
            let expected = request.source_sha256.as_deref().unwrap_or_default();
            if !crate::cad_jobs::is_sha256_hex(expected) || sha256_hex(bytes) != expected {
                return Err(failed("STEP source failed SHA-256 verification"));
            }
            let step_options = request
                .step_import_options
                .unwrap_or_default()
                .validate()
                .map_err(failed)?;
            let options = Object::new();
            js_set(&options, "sourceSha256", &expected.into())?;
            if let Some(budget) = step_options.preview_repair_budget_mm {
                js_set(&options, "previewRepairBudgetMm", &budget.into())?;
            }
            if let Some(target) = step_options.target_deflection_mm {
                js_set(&options, "targetDeflectionMm", &target.into())?;
            }
            let loader = queue.borrow().load_baked_step.clone();
            let baked = if request.packaged_step && step_options == CadStepImportOptions::default()
            {
                match loader {
                    Some(loader) => load_baked_step(&loader, expected).await,
                    None => None,
                }
            } else {
                None
            };
            check_cancelled()?;
            let baked = baked
                .and_then(|data| invoke("read_baked_step_model", &[data, expected.into()]).ok());
            let result = match baked {
                Some(result) => result,
                None => invoke(
                    "read_step_model",
                    &[Uint8Array::from(bytes).into(), options.into()],
                )?,
            };
            let vendor = get(&result, "vendorStep")?;
            *staged = Some(
                stage_analysis(
                    queue,
                    module,
                    request,
                    &vendor,
                    Source::VendorStep {
                        source_sha256: expected.into(),
                        import_options: step_options.into(),
                    },
                )
                .map_err(failed)?,
            );
            exact_fields(&result, &["mesh", "bounds", "vendorStep"]).map_err(failed)?;
            let decoded = decode_vendor_step(vendor).map_err(failed)?;
            let mesh = decode_mesh(get(&result, "mesh")?).map_err(failed)?;
            let bounds = strict_metadata(get(&result, "bounds")?).map_err(failed)?;
            crate::cad_jobs::validate_vendor_step(expected, step_options, &mesh, &bounds, &decoded)
                .map_err(failed)?;
            result
        }
        CadOperation::CollisionQuery => {
            let query = request
                .collision_query
                .as_ref()
                .ok_or_else(|| failed("collision query omitted metadata"))?;
            validate_collision_query(query).map_err(failed)?;
            let meshes = request
                .collision_meshes
                .as_ref()
                .ok_or_else(|| failed("collision query omitted mesh buffers"))?;
            let left = queue
                .borrow_mut()
                .lookup(
                    &query.left,
                    identity,
                    &meshes.left_positions_mm_f64,
                    &meshes.left_triangle_face_occurrence_indices,
                )
                .map_err(failed)?;
            let right = queue
                .borrow_mut()
                .lookup(
                    &query.right,
                    identity,
                    &meshes.right_positions_mm_f64,
                    &meshes.right_triangle_face_occurrence_indices,
                )
                .map_err(failed)?;
            let generation = queue.borrow().generation.clone();
            let expected =
                qualification::expected_binding(query, identity, &generation).map_err(failed)?;
            let buffers = collision_mesh_buffers(meshes);
            let result = if let (Some(left), Some(right)) = (left, right) {
                let object = Object::new();
                for (field, array, _) in buffers {
                    js_set(&object, field, &array)?;
                }
                let context = json_to_js(&serde_json::json!({
                    "snapshot": qualification::Snapshot::from(identity),
                    "workerGeneration": generation,
                }))?;
                let result = invoke(
                    "query_qualified_collision_mesh_pair",
                    &[
                        object.into(),
                        json_to_js(query)?,
                        context,
                        left.handle.0.clone(),
                        right.handle.0.clone(),
                    ],
                )?;
                let report =
                    strict_metadata(get(&result, "queryQualification")?).map_err(failed)?;
                qualification::validate_query_qualification_for_checked_meshes(
                    report,
                    &expected,
                    &query.qualification_limits,
                    meshes,
                    Some([&left.source, &right.source]),
                )
                .map_err(failed)?;
                pinned.extend([left, right]);
                result
            } else {
                let [left_pos, left_tags, right_pos, right_tags] =
                    buffers.map(|(_, array, _)| array);
                let raw = invoke(
                    "query_collision_mesh_pair",
                    &[
                        left_pos,
                        left_tags,
                        right_pos,
                        right_tags,
                        raw_query_metadata(query).map_err(failed)?,
                    ],
                )?;
                let result = Object::new();
                js_set(&result, "observation", &raw)?;
                let report =
                    qualification::unavailable(expected, query.qualification_limits.clone());
                js_set(&result, "queryQualification", &json_to_js(&report)?)?;
                result.into()
            };
            exact_fields(&result, &["observation", "queryQualification"]).map_err(failed)?;
            result
        }
    })
}

fn prepared_body_cache_key(
    identity: &CadSnapshotIdentity,
    body: &PreparedCaseIR,
) -> Result<String, String> {
    let mut geometry = serde_json::to_value(&body.body).map_err(|error| error.to_string())?;
    if let Some(fields) = geometry.as_object_mut() {
        fields.remove("id");
        fields.remove("name");
    }
    let input = serde_json::to_vec(&serde_json::json!({
        "body": geometry,
        "regions": &body.regions,
    }))
    .map_err(|error| error.to_string())?;
    let digest = sha256_hex(&input);
    Ok(format!(
        "m1/{}/{}/{}/{}/{digest}",
        identity.session_epoch,
        identity.document_id,
        identity.board_id,
        identity.instance_id.as_deref().unwrap_or("board"),
    ))
}

fn js_set(target: &JsValue, field: &str, value: &JsValue) -> Result<(), OperationError> {
    Reflect::set(target, &field.into(), value)
        .map(|_| ())
        .map_err(|error| failed(js_message(error)))
}

fn json_to_js(value: &impl Serialize) -> Result<JsValue, OperationError> {
    let text = serde_json::to_string(value).map_err(|error| failed(error.to_string()))?;
    js_sys::JSON::parse(&text).map_err(|error| failed(js_message(error)))
}

fn revision_object(revision: u64) -> Result<Object, OperationError> {
    let object = Object::new();
    js_set(&object, "revision", &JsValue::from_f64(revision as f64))?;
    Ok(object)
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

async fn load_baked_step(loader: &Function, sha256: &str) -> Option<JsValue> {
    let pending = loader.call1(&JsValue::NULL, &sha256.into()).ok()?;
    let bytes = JsFuture::from(pending.dyn_into::<Promise>().ok()?)
        .await
        .ok()?;
    bytes.dyn_into::<Uint8Array>().ok().map(Into::into)
}
