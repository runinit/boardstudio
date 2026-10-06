use super::*;

pub fn prepare_export(request: PrepareExportRequest) -> Result<ExportPlan, ArtifactError> {
    prepare(request, true)
}

pub fn prepare_preview(
    request: PrepareExportRequest,
) -> Result<ExportPlan, ArtifactError> {
    prepare(request, false)
}

fn prepare(request: PrepareExportRequest, fabrication: bool) -> Result<ExportPlan, ArtifactError> {
    let doc = &request.document;
    if doc.format != "boardstudio/v2" {
        return Err(validation("Export requires a v2 document"));
    }
    for definition in &doc.definitions {
        crate::artifact::validate_source_version(definition)?;
    }
    if request.expected_revision != doc.revision {
        return Err(err(
            ArtifactErrorCode::StaleResult,
            "Export requires a committed current v2 revision",
        ));
    }
    let (jobs, reserved_nets, next_net_index) = match &request.target {
        ExportTarget::Board { board_id } => {
            if doc.revision == u64::MAX {
                return Err(validation("Invalid document revision"));
            }
            let (board, parts, definitions) = board_and_maps(doc, board_id)?;
            let (net_table, pin_map) = net_lookup(doc, board)?;
            let mut jobs = Vec::new();
            for part_id in &board.part_ids {
                let part = parts
                    .get(part_id.as_str())
                    .ok_or_else(|| validation(format!("Missing part or definition: {part_id}")))?;
                let definition = definitions
                    .get(part.definition_id.as_str())
                    .ok_or_else(|| validation(format!("Missing part or definition: {part_id}")))?;
                if fabrication {
                    require_footprint_qualification(definition)?;
                }
                if is_generated(definition) {
                    jobs.push(RenderJob {
                        job_id: format!("{}:{}", board.id, part.id),
                        definition: (*definition).clone(),
                        part: terminal_parameters(definition, part, doc, board, &pin_map)?,
                    });
                } else if is_imported(definition) {
                    // Validate imports before workers start; raw source is retained in the captured document.
                    source::import_footprint(
                        &definition.kicad_source.as_ref().expect("checked").source,
                        &definition.id,
                    )?;
                } else {
                    let compiled = compile::compile_authored(definition, part.side.clone())
                        .map_err(validation)?;
                    assert_geometry(&compiled.geometry)?;
                }
            }
            validate_contours(&request.contours)?;
            let next = net_table.iter().map(|net| net.index).max().unwrap_or(0) + 1;
            (jobs, net_table, next)
        }
        ExportTarget::StandaloneFootprints { definition_ids } => {
            if definition_ids.is_empty() {
                return Err(validation("No footprint definitions selected"));
            }
            let defs = doc
                .definitions
                .iter()
                .map(|definition| (definition.id.as_str(), definition))
                .collect::<HashMap<_, _>>();
            unique_ids(definition_ids.iter().map(String::as_str), "definition")?;
            let mut jobs = Vec::new();
            for id in definition_ids {
                let definition = defs.get(id.as_str()).ok_or_else(|| {
                    err(
                        ArtifactErrorCode::NotFound,
                        format!("Missing definition: {id}"),
                    )
                })?;
                if fabrication {
                    require_footprint_qualification(definition)?;
                }
                if is_generated(definition) {
                    jobs.push(RenderJob {
                        job_id: format!("definition:{}", definition.id),
                        definition: (*definition).clone(),
                        part: standalone_part(definition),
                    });
                } else if is_imported(definition) {
                    source::import_footprint(
                        &definition.kicad_source.as_ref().expect("checked").source,
                        &definition.id,
                    )?;
                } else {
                    let compiled =
                        compile::compile_authored(definition, Side::Front).map_err(validation)?;
                    assert_geometry(&compiled.geometry)?;
                }
            }
            (jobs, vec![], 1)
        }
    };
    let mut plan = ExportPlan {
        snapshot_token: request.snapshot_token.clone(),
        fingerprint: String::new(),
        revision: doc.revision,
        target: request.target.clone(),
        jobs,
        reserved_nets,
        next_net_index,
        contours: request.contours.clone(),
        captured_document: doc.clone(),
        model_paths: request.model_paths.clone(),
    };
    plan.fingerprint = plan_fingerprint(&plan)?;
    validate_plan(&plan)?;
    Ok(plan)
}

fn terminal_parameters(
    definition: &PartDefinition,
    part: &Part,
    doc: &ProjectDoc,
    board: &Board,
    pin_map: &HashMap<String, String>,
) -> Result<Part, ArtifactError> {
    let mut part = part.clone();
    let mut parameters = part.generator_parameters.take().unwrap_or_default();
    let mut consumed = HashSet::new();
    for (terminal, pads) in &definition.terminals {
        let assigned = pads
            .iter()
            .filter_map(|pad| pin_map.get(&format!("{}\0{}", part.id, pad)))
            .collect::<HashSet<_>>();
        if assigned.len() > 1 {
            return Err(validation(format!(
                "Terminal {}/{} has conflicting pad net assignments",
                part.id, terminal
            )));
        }
        let current = parameters.get(terminal).or_else(|| {
            definition
                .generator
                .as_ref()
                .and_then(|generator| generator.parameters.get(terminal))
        });
        let explicit = current
            .and_then(serde_json::Value::as_str)
            .filter(|name| !name.trim().is_empty());
        let explicit_net = explicit
            .map(|name| {
                let matches = board
                    .net_ids
                    .iter()
                    .filter_map(|id| net_by_id(&doc.nets, id))
                    .filter(|net| net.name == name)
                    .collect::<Vec<_>>();
                match matches.as_slice() {
                    [net] => Ok(Some(*net)),
                    [] => Err(validation(format!(
                        "Terminal {}/{} references a net outside board: {name}",
                        part.id, terminal
                    ))),
                    _ => Err(validation(format!(
                        "Net name is ambiguous on board: {name}"
                    ))),
                }
            })
            .transpose()?
            .flatten();
        let assigned_net = assigned
            .iter()
            .next()
            .copied()
            .and_then(|id| doc.nets.iter().find(|net| net.id == *id));
        if explicit_net
            .is_some_and(|explicit| assigned_net.is_some_and(|assigned| explicit.id != assigned.id))
        {
            return Err(validation(format!(
                "Terminal {}/{} has conflicting explicit and document net assignments",
                part.id, terminal
            )));
        }
        let effective = assigned_net.or(explicit_net);
        if let Some(net) = effective {
            parameters.insert(
                terminal.clone(),
                serde_json::Value::String(net.name.clone()),
            );
        }
        consumed.extend(pads.iter().map(|pad| format!("{}\0{}", part.id, pad)));
    }
    for key in pin_map
        .keys()
        .filter(|key| key.starts_with(&format!("{}\0", part.id)))
    {
        if !consumed.contains(key) {
            return Err(validation(format!(
                "Net references an unmappable Ergogen pad on part {}",
                part.id
            )));
        }
    }
    part.generator_parameters = Some(parameters);
    Ok(part)
}

fn standalone_part(definition: &PartDefinition) -> Part {
    Part {
        id: format!("definition:{}", definition.id),
        definition_id: definition.id.clone(),
        reference: "REF**".into(),
        pose: Pose2 {
            at: Vec2::default(),
            rotation: 0.0,
        },
        side: Side::Front,
        keycap: None,
        outline: None,
        locked: None,
        properties: None,
        generator_parameters: Some(
            definition
                .generator
                .as_ref()
                .map(|g| g.parameters.clone())
                .unwrap_or_default(),
        ),
    }
}

// Enforce the same recorded blockers for stateless exports and UI handoffs.
fn require_footprint_qualification(definition: &PartDefinition) -> Result<(), ArtifactError> {
    if let Some(gate) = definition
        .hardware_profile
        .iter()
        .flat_map(|profile| &profile.gates)
        .find(|gate| {
            matches!(
                gate.output,
                HardwareOutput::Footprint | HardwareOutput::Electrical
            )
        })
    {
        return Err(validation(format!("{}: {}", definition.name, gate.message)));
    }
    Ok(())
}

pub(super) fn require_plan_qualification(plan: &ExportPlan) -> Result<(), ArtifactError> {
    let document = &plan.captured_document;
    let ids = match &plan.target {
        ExportTarget::Board { board_id } => {
            let (board, parts, _) = board_and_maps(document, board_id)?;
            board
                .part_ids
                .iter()
                .map(|id| {
                    parts
                        .get(id.as_str())
                        .map(|part| part.definition_id.as_str())
                        .ok_or_else(|| validation("Missing board part"))
                })
                .collect::<Result<Vec<_>, _>>()?
        }
        ExportTarget::StandaloneFootprints { definition_ids } => {
            definition_ids.iter().map(String::as_str).collect()
        }
    };
    for id in ids {
        let definition = document
            .definitions
            .iter()
            .find(|definition| definition.id == id)
            .ok_or_else(|| validation("Missing definition"))?;
        require_footprint_qualification(definition)?;
    }
    Ok(())
}
