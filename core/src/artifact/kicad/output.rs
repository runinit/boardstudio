use super::*;
use sha2::{Digest, Sha256};

pub fn finish_export(request: FinishExportRequest) -> Result<ExportArtifact, ArtifactError> {
    validate_plan(&request.plan)?;
    planning::require_plan_qualification(&request.plan)?;
    finish(request, false)
}

pub fn finish_preview(
    request: FinishExportRequest,
) -> Result<PcbPreview, ArtifactError> {
    if !matches!(request.plan.target, ExportTarget::Board { .. }) {
        return Err(validation("Preview requires a board"));
    }
    let revision = request.plan.revision;
    let board_id = match &request.plan.target {
        ExportTarget::Board { board_id } => board_id.clone(),
        _ => return Err(validation("Preview requires a board")),
    };
    let support_diagnostics =
        missing_module_support_diagnostics(&request.plan.captured_document, &board_id);
    let artifact = finish(request, true)?;
    let board = artifact
        .files
        .first()
        .ok_or_else(|| validation("Preview returned no board"))?;
    let mut preview = crate::artifact::preview::board(&board.content, revision)?;
    preview.diagnostics.extend(support_diagnostics);
    preview.diagnostics.sort();
    preview.diagnostics.dedup();
    Ok(preview)
}

fn missing_module_support_diagnostics(doc: &ProjectDoc, board_id: &str) -> Vec<String> {
    let resolved = crate::modules::resolve(doc, board_id);
    doc.modules
        .iter()
        .filter(|instance| {
            instance.host_board_id == board_id
                && !instance.detached
                && instance.attachment == crate::model::ModuleAttachment::Board
        })
        .filter_map(|instance| {
            let definition = doc
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)?;
            if definition.mounts.is_empty() {
                return None;
            }
            let module = resolved
                .modules
                .iter()
                .find(|module| module.id == instance.id)?;
            let missing = module.mounts.len().saturating_sub(module.mount_supports.len());
            (missing > 0).then(|| {
                format!(
                    "Preview omits {missing} unconfigured source mounting drill(s) for module {}; configure standoffs before fabrication.",
                    instance.id
                )
            })
        })
        .collect()
}

fn finish(
    request: FinishExportRequest,
    allow_incomplete_module_supports: bool,
) -> Result<ExportArtifact, ArtifactError> {
    let plan = request.plan;
    validate_plan(&plan)?;
    if request.results.len() != plan.jobs.len() {
        return Err(err(
            ArtifactErrorCode::MismatchedResults,
            "Missing or extra Ergogen results",
        ));
    }
    let mut completed = Vec::new();
    let mut current_nets = plan.reserved_nets.clone();
    for (job, result) in plan.jobs.iter().zip(request.results.iter()) {
        if result.snapshot_token != plan.snapshot_token || result.revision != plan.revision {
            return Err(err(
                ArtifactErrorCode::StaleResult,
                "Ergogen result belongs to a stale export snapshot",
            ));
        }
        if result.job_id != job.job_id {
            return Err(err(
                ArtifactErrorCode::MismatchedResults,
                "Ergogen results are missing, duplicated, or reordered",
            ));
        }
        validate_net_transition(&current_nets, &result.nets)?;
        validate_generated_forms(&result.source, &result.nets)?;
        current_nets = result.nets.clone();
        completed.push(result.source.clone());
    }
    let (files, skipped_utilities) = match &plan.target {
        ExportTarget::Board { board_id } => (
            finish_board(
                &plan,
                board_id,
                &completed,
                &current_nets,
                allow_incomplete_module_supports,
            )?,
            Vec::new(),
        ),
        ExportTarget::StandaloneFootprints { definition_ids } => {
            finish_standalone(&plan, definition_ids, &request.results)?
        }
    };
    Ok(ExportArtifact {
        snapshot_token: plan.snapshot_token,
        revision: plan.revision,
        files,
        skipped_utilities,
    })
}

fn validate_net_transition(
    previous: &[ReservedNet],
    current: &[ReservedNet],
) -> Result<(), ArtifactError> {
    if current.len() < previous.len() {
        return Err(validation("Generated net table removed reserved nets"));
    }
    if current[..previous.len()] != *previous {
        return Err(validation(
            "Generated net table changed reserved net allocation",
        ));
    }
    let previous_names = previous
        .iter()
        .map(|net| net.name.as_str())
        .collect::<HashSet<_>>();
    let mut names = HashSet::new();
    let mut indexes = HashSet::new();
    for (position, net) in current.iter().enumerate() {
        let repeated_reserved_name = position < previous.len();
        if net.index == 0
            || (!repeated_reserved_name
                && (!names.insert(net.name.as_str()) || previous_names.contains(net.name.as_str())))
            || !indexes.insert(net.index)
        {
            return Err(validation(
                "Generated net table contains duplicate names or indexes",
            ));
        }
    }
    for (offset, net) in current.iter().skip(previous.len()).enumerate() {
        let expected =
            previous.iter().map(|entry| entry.index).max().unwrap_or(0) + 1 + offset as u32;
        if net.index != expected {
            return Err(validation("Generated net allocator is not contiguous"));
        }
    }
    Ok(())
}

fn validate_generated_forms(
    source_text: &str,
    nets: &[ReservedNet],
) -> Result<Vec<String>, ArtifactError> {
    let parsed = kiutils_sexpr::parse_rootless(source_text)
        .map_err(|error| err(ArtifactErrorCode::ParseError, error.to_string()))?;
    let known = nets
        .iter()
        .map(|net| (net.index, net.name.as_str()))
        .collect::<HashMap<_, _>>();
    let mut forms = Vec::new();
    for node in &parsed.nodes {
        let safe_node = restore_kicad_quoted_atoms(node, source_text)?;
        let form = CstDocument {
            raw: source_text.to_owned(),
            nodes: vec![safe_node],
        }
        .to_canonical_string()
        .trim_end()
        .to_owned();
        let kind = list_head(node).unwrap_or("");
        if !matches!(
            kind,
            "footprint"
                | "module"
                | "segment"
                | "via"
                | "zone"
                | "gr_text"
                | "gr_line"
                | "gr_arc"
                | "gr_circle"
                | "gr_poly"
                | "gr_rect"
        ) {
            return Err(err(
                ArtifactErrorCode::Unsupported,
                format!("Unsupported Ergogen output form: {kind}"),
            ));
        }
        validate_net_refs(node, &known, source_text)?;
        forms.push(form);
    }
    Ok(forms)
}

fn restore_kicad_quoted_atoms(node: &Node, source_text: &str) -> Result<Node, ArtifactError> {
    match node {
        Node::Atom {
            atom: Atom::Quoted(_),
            span,
        } => {
            let value = sexpr::kicad_quoted(source_text, *span)
                .ok_or_else(|| err(ArtifactErrorCode::ParseError, "Invalid quoted KiCad text"))?;
            Ok(Node::Atom {
                atom: Atom::Quoted(value),
                span: *span,
            })
        }
        Node::Atom { atom, span } => Ok(Node::Atom {
            atom: atom.clone(),
            span: *span,
        }),
        Node::List { items, span } => Ok(Node::List {
            items: items
                .iter()
                .map(|item| restore_kicad_quoted_atoms(item, source_text))
                .collect::<Result<_, _>>()?,
            span: *span,
        }),
    }
}

fn validate_net_refs(
    node: &Node,
    known: &HashMap<u32, &str>,
    source_text: &str,
) -> Result<(), ArtifactError> {
    match node {
        Node::Atom { .. } => Ok(()),
        Node::List { items, .. } => {
            if list_head(node) == Some("net") {
                let index = items
                    .get(1)
                    .and_then(node_atom)
                    .and_then(|text| text.parse::<u32>().ok())
                    .ok_or_else(|| validation("Malformed generated net reference"))?;
                if index != 0 {
                    let name = known.get(&index).ok_or_else(|| {
                        validation(format!(
                            "Generated form references undeclared net index {index}"
                        ))
                    })?;
                    if let Some(label_node) = items.get(2) {
                        let label = match label_node {
                            Node::Atom {
                                atom: Atom::Quoted(_),
                                span,
                            } => sexpr::kicad_quoted(source_text, *span),
                            _ => node_atom(label_node).map(str::to_owned),
                        };
                        if let Some(label) = label
                            && label != *name
                        {
                            return Err(validation(format!(
                                "Generated net reference {index} has mismatched name"
                            )));
                        }
                    }
                }
            }
            for child in items {
                validate_net_refs(child, known, source_text)?;
            }
            Ok(())
        }
    }
}

fn list_head(node: &Node) -> Option<&str> {
    match node {
        Node::List { items, .. } => items.first().and_then(node_atom),
        Node::Atom { .. } => None,
    }
}

fn node_atom(node: &Node) -> Option<&str> {
    match node {
        Node::Atom {
            atom: Atom::Symbol(value) | Atom::Quoted(value),
            ..
        } => Some(value),
        Node::List { .. } => None,
    }
}

fn normalize_generated_footprint(form: &str) -> String {
    if form.starts_with("(module ") {
        form.replacen("(module", "(footprint", 1)
    } else {
        form.to_owned()
    }
}

fn standalone_footprint_names(definitions: &[&PartDefinition]) -> Vec<String> {
    let bases = definitions
        .iter()
        .map(|definition| safe_name(&definition.name))
        .collect::<Vec<_>>();
    let mut counts = HashMap::<String, usize>::new();
    for name in &bases {
        *counts.entry(name.clone()).or_default() += 1;
    }
    // Reserve unambiguous names first so a disambiguated collision can never
    // steal a legacy filename that already belongs to another definition.
    let mut used = bases
        .iter()
        .filter(|name| counts.get(name.as_str()) == Some(&1))
        .cloned()
        .collect::<HashSet<_>>();
    definitions
        .iter()
        .zip(bases)
        .map(|(definition, base)| {
            if counts.get(&base) == Some(&1) {
                return base;
            }
            let digest = Sha256::digest(definition.id.as_bytes())
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>();
            let mut length = 10;
            loop {
                let candidate = format!("{base}__{}", &digest[..length]);
                if used.insert(candidate.clone()) {
                    return candidate;
                }
                if length < digest.len() {
                    length = (length + 4).min(digest.len());
                } else {
                    // Document definition IDs are unique; this final suffix
                    // still makes malformed repeated IDs safe and deterministic.
                    let mut ordinal = 2;
                    loop {
                        let candidate = format!("{base}__{digest}_{ordinal}");
                        if used.insert(candidate.clone()) {
                            return candidate;
                        }
                        ordinal += 1;
                    }
                }
            }
        })
        .collect()
}

fn rename_standalone_footprint(form: &str, name: &str) -> Result<String, ArtifactError> {
    let document = kiutils_sexpr::parse_one(form)
        .map_err(|error| err(ArtifactErrorCode::ParseError, error.to_string()))?;
    let root = document
        .nodes
        .first()
        .ok_or_else(|| validation("Expected standalone footprint"))?;
    if !matches!(sexpr::head(root), Some("footprint" | "module")) {
        return Err(validation("Expected standalone footprint"));
    }
    let source_name = sexpr::items(root)
        .and_then(|items| items.get(1))
        .ok_or_else(|| validation("Standalone footprint has no name"))?;
    sexpr::replace_spans(form, vec![(sexpr::span(source_name), sexpr::quote(name))])
        .ok_or_else(|| validation("Could not rename standalone footprint"))
}

fn disambiguate_standalone_footprint(
    form: String,
    definition: &PartDefinition,
    output_name: &str,
) -> Result<String, ArtifactError> {
    if output_name == safe_name(&definition.name) {
        Ok(form)
    } else {
        rename_standalone_footprint(&form, output_name)
    }
}

fn finish_standalone(
    plan: &ExportPlan,
    definition_ids: &[String],
    results: &[ErgogenJobResult],
) -> Result<(Vec<ArtifactFile>, Vec<String>), ArtifactError> {
    let definitions = plan
        .captured_document
        .definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<HashMap<_, _>>();
    let selected_definitions = definition_ids
        .iter()
        .map(|id| {
            definitions.get(id.as_str()).copied().ok_or_else(|| {
                err(
                    ArtifactErrorCode::NotFound,
                    format!("Missing definition: {id}"),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let output_names = standalone_footprint_names(&selected_definitions);
    let mut files = Vec::new();
    let mut skipped_utilities = Vec::new();
    let mut result_index = 0;
    for (definition, output_name) in selected_definitions.iter().zip(output_names) {
        let content = if is_ergogen(definition) {
            let result = results.get(result_index).ok_or_else(|| {
                err(
                    ArtifactErrorCode::MismatchedResults,
                    "Missing Ergogen result",
                )
            })?;
            let forms = validate_generated_forms(&result.source, &result.nets)?;
            result_index += 1;
            let footprints = forms
                .iter()
                .filter(|form| form.starts_with("(footprint ") || form.starts_with("(module "))
                .collect::<Vec<_>>();
            if footprints.is_empty()
                && matches!(definition.kind, PartKind::Utility)
                && definition_ids.len() > 1
            {
                skipped_utilities.push(definition.name.clone());
                continue;
            }
            if footprints.len() != 1 {
                return Err(err(
                    ArtifactErrorCode::Unsupported,
                    format!(
                        "{} emits board objects and must be exported on a board",
                        definition.name
                    ),
                ));
            }
            // A standalone library file contains the footprint artifact only;
            // generators can also emit board-level routing or helper graphics.
            format!(
                "{}\n",
                override_models(
                    disambiguate_standalone_footprint(
                        normalize_generated_footprint(footprints[0]),
                        definition,
                        &output_name,
                    )?,
                    definition,
                    &plan.model_paths
                )?
            )
        } else if is_imported(definition) {
            override_models(
                source::patch_footprint(
                    &definition.kicad_source.as_ref().expect("checked").source,
                    &FootprintPatch {
                        footprint_name: Some(output_name.clone()),
                        reference: Some("REF**".into()),
                        value: Some(definition.name.clone()),
                        placement: Some(Pose2 {
                            at: Vec2::default(),
                            rotation: 0.0,
                        }),
                        side: Side::Front,
                        pad_nets: BTreeMap::new(),
                        uuid_scope: format!("definition:{}", definition.id),
                        model_forms: managed_model_forms(definition, &plan.model_paths)?,
                    },
                )?,
                definition,
                &plan.model_paths,
            )?
        } else {
            format!(
                "{}\n",
                disambiguate_standalone_footprint(
                    native_footprint(
                        definition,
                        &format!("definition:{}", definition.id),
                        None,
                        &BTreeMap::new(),
                        &plan.model_paths
                    )?,
                    definition,
                    &output_name
                )?
            )
        };
        files.push(ArtifactFile {
            filename: format!("{output_name}.kicad_mod"),
            content,
        });
    }
    Ok((files, skipped_utilities))
}

fn finish_board(
    plan: &ExportPlan,
    board_id: &str,
    results: &[String],
    generated_nets: &[ReservedNet],
    allow_incomplete_module_supports: bool,
) -> Result<Vec<ArtifactFile>, ArtifactError> {
    let doc = &plan.captured_document;
    let (board, parts, definitions) = board_and_maps(doc, board_id)?;
    validate_contours(&plan.contours)?;
    let (_, pin_map) = net_lookup(doc, board)?;
    let board_net_index = board
        .net_ids
        .iter()
        .enumerate()
        .map(|(index, id)| (id.as_str(), index as u32 + 1))
        .collect::<HashMap<_, _>>();
    let mut ergogen_result_index = 0;
    let mut footprints = Vec::new();
    let mut generated_objects = Vec::new();
    let mut generated_traces = Vec::new();
    let mut generated_vias = Vec::new();
    for part_id in &board.part_ids {
        let part = parts
            .get(part_id.as_str())
            .ok_or_else(|| validation(format!("Missing part or definition: {part_id}")))?;
        let definition = definitions
            .get(part.definition_id.as_str())
            .ok_or_else(|| validation(format!("Missing part or definition: {part_id}")))?;
        let scope = format!("{board_id}:part:{part_id}");
        if is_ergogen(definition) {
            let forms = validate_generated_forms(&results[ergogen_result_index], generated_nets)?;
            ergogen_result_index += 1;
            let mut board_objects_here = Vec::new();
            for form in forms {
                if form.starts_with("(footprint ") || form.starts_with("(module ") {
                    footprints.push(override_models(
                        normalize_generated_footprint(&form),
                        definition,
                        &plan.model_paths,
                    )?);
                } else {
                    board_objects_here.push(render(&form)?);
                }
            }
            if !board_objects_here.is_empty() && part.pose.at != Vec2::default() {
                // Ergogen owns its world-space transform through the trusted adapter.
            }
            generated_objects.extend(board_objects_here);
            continue;
        }
        if is_imported(definition) {
            let imported = source::import_footprint(
                &definition.kicad_source.as_ref().expect("checked").source,
                &definition.id,
            )?;
            let mut pad_nets = BTreeMap::new();
            for pad in &imported.definition.pads {
                let net_id = pin_map.get(&format!("{}\0{}", part_id, pad.id));
                if let Some(net_id) = net_id {
                    let index = board_net_index
                        .get(net_id.as_str())
                        .copied()
                        .ok_or_else(|| {
                            validation(format!(
                                "Pad {part_id}/{} references a net outside board",
                                pad.id
                            ))
                        })?;
                    let net = net_by_id(&doc.nets, net_id).expect("net was indexed");
                    pad_nets.insert(pad.id.clone(), (index, net.name.clone()));
                }
            }
            for key in pin_map
                .keys()
                .filter(|key| key.starts_with(&format!("{}\0", part_id)))
            {
                let pad_id = key.split('\0').nth(1).unwrap_or_default();
                if !imported.definition.pads.iter().any(|pad| pad.id == pad_id) {
                    return Err(validation(format!(
                        "Net references missing pad on part {part_id}"
                    )));
                }
            }
            footprints.push(override_models(
                source::patch_footprint(
                    &definition.kicad_source.as_ref().expect("checked").source,
                    &FootprintPatch {
                        footprint_name: Some(safe_name(&definition.name)),
                        reference: Some(part.reference.clone()),
                        value: Some(definition.name.clone()),
                        placement: Some(part.pose),
                        side: part.side.clone(),
                        pad_nets,
                        uuid_scope: scope,
                        model_forms: managed_model_forms(definition, &plan.model_paths)?,
                    },
                )?,
                definition,
                &plan.model_paths,
            )?);
            continue;
        }
        let compiled =
            compile::compile_authored(definition, part.side.clone()).map_err(validation)?;
        let geometry = &compiled.geometry;
        let mut pad_net_map = BTreeMap::new();
        for pad in &geometry.pads {
            let pin_net = pin_map.get(&format!("{}\0{}", part_id, pad.id));
            let authored = definition
                .pads
                .iter()
                .find(|item| item.id == pad.id)
                .and_then(|pad| pad.net_id.as_deref());
            if pin_net.is_some() && authored.is_some_and(|authored| authored != pin_net.unwrap()) {
                return Err(validation(format!(
                    "Conflicting net for {part_id}/{}",
                    pad.id
                )));
            }
            let selected = pin_net.map(String::as_str).or(authored);
            if let Some(net_id) = selected {
                let index = board_net_index.get(net_id).copied().ok_or_else(|| {
                    validation(format!(
                        "Pad {part_id}/{} references a net outside board",
                        pad.id
                    ))
                })?;
                let name = net_by_id(&doc.nets, net_id)
                    .ok_or_else(|| {
                        validation(format!("Pad {part_id}/{} references a missing net", pad.id))
                    })?
                    .name
                    .clone();
                let net = ReservedNet { name, index };
                pad_net_map.insert(pad.id.clone(), net.clone());
            }
        }
        for key in pin_map
            .keys()
            .filter(|key| key.starts_with(&format!("{}\0", part_id)))
        {
            let pad_id = key.split('\0').nth(1).unwrap_or_default();
            if !geometry.pads.iter().any(|pad| pad.id == pad_id) {
                return Err(validation(format!(
                    "Net references missing pad on part {part_id}"
                )));
            }
        }
        footprints.push(native_footprint(
            definition,
            &scope,
            Some(part),
            &pad_net_map,
            &plan.model_paths,
        )?);
        let angle = part.pose.rotation.to_radians();
        let transform = |point: Vec2| -> Vec2 {
            let x = if matches!(part.side, Side::Back) {
                -point.x
            } else {
                point.x
            };
            Vec2 {
                x: part.pose.at.x + x * angle.cos() - point.y * angle.sin(),
                y: part.pose.at.y + x * angle.sin() + point.y * angle.cos(),
            }
        };
        for trace in &geometry.traces {
            let layer = if matches!(part.side, Side::Back) {
                Side::Back
            } else {
                trace.layer.clone()
            };
            let net_id = trace
                .pad_id
                .as_ref()
                .and_then(|pad| pin_map.get(&format!("{}\0{}", part_id, pad)).cloned())
                .or_else(|| {
                    trace.pad_id.as_ref().and_then(|pad| {
                        definition
                            .pads
                            .iter()
                            .find(|item| &item.id == pad)
                            .and_then(|item| item.net_id.clone())
                    })
                });
            generated_traces.push(CopperTrace {
                id: format!("{part_id}:{}", trace.id),
                start: transform(trace.start),
                end: transform(trace.end),
                width: trace.width,
                layer,
                net_id,
            });
        }
        for via in &geometry.vias {
            let net_id = via
                .pad_id
                .as_ref()
                .and_then(|pad| pin_map.get(&format!("{}\0{}", part_id, pad)).cloned())
                .or_else(|| {
                    via.pad_id.as_ref().and_then(|pad| {
                        definition
                            .pads
                            .iter()
                            .find(|item| &item.id == pad)
                            .and_then(|item| item.net_id.clone())
                    })
                });
            generated_vias.push(CopperVia {
                id: format!("{part_id}:{}", via.id),
                at: transform(via.at),
                size: via.size,
                drill: via.drill,
                net_id,
            });
        }
    }
    let mut tracks = board.traces.clone();
    tracks.extend(generated_traces);
    let mut vias = board.vias.clone();
    vias.extend(generated_vias);
    unique_ids(
        tracks
            .iter()
            .map(|item| item.id.as_str())
            .chain(vias.iter().map(|item| item.id.as_str())),
        "copper",
    )?;
    let module_resolution = crate::modules::resolve(doc, board_id);
    for instance in doc.modules.iter().filter(|instance| {
        instance.host_board_id == board_id
            && !instance.detached
            && instance.attachment == crate::model::ModuleAttachment::Board
            && doc
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)
                .is_some_and(|definition| !definition.mounts.is_empty())
    }) {
        let module = module_resolution
            .modules
            .iter()
            .find(|module| module.id == instance.id)
            .ok_or_else(|| {
                validation(format!(
                    "Cannot export host drills for unresolved module support {}",
                    instance.id
                ))
            })?;
        if module.mount_supports.len() != module.mounts.len() {
            if !allow_incomplete_module_supports {
                return Err(validation(format!(
                    "Every source mount on board-attached module {} needs a configured standoff and host drill",
                    instance.id
                )));
            }
            for support in &module.mount_supports {
                footprints.push(module_mount_hole_footprint(board_id, module, support)?);
            }
            continue;
        }
        for support in &module.mount_supports {
            footprints.push(module_mount_hole_footprint(board_id, module, support)?);
        }
    }
    let net_number = |net_id: Option<&str>| -> Result<u32, ArtifactError> {
        let Some(net_id) = net_id else {
            return Ok(0);
        };
        board_net_index
            .get(net_id)
            .copied()
            .or_else(|| {
                doc.nets
                    .iter()
                    .find(|net| net.id == net_id)
                    .and_then(|net| board_net_index.get(net.id.as_str()).copied())
            })
            .ok_or_else(|| validation(format!("Copper references a net outside board: {net_id}")))
    };
    let mut copper = Vec::new();
    for trace in tracks {
        if trace.width <= 0.0 || !trace.width.is_finite() || trace.start == trace.end {
            return Err(validation(format!("Invalid trace geometry: {}", trace.id)));
        }
        let layer = if matches!(trace.layer, Side::Back) {
            "B.Cu"
        } else {
            "F.Cu"
        };
        copper.push(render(&format!(
            "(segment (start {}) (end {}) (width {}) (layer {}) (net {}) (uuid {}))",
            xy(trace.start)?,
            xy(trace.end)?,
            num(trace.width)?,
            q(layer)?,
            net_number(trace.net_id.as_deref())?,
            uuid_text(&board.id, "trace", &trace.id)?
        ))?);
    }
    for via in vias {
        if via.size <= 0.0 || via.drill <= 0.0 || via.drill >= via.size {
            return Err(validation(format!("Invalid via geometry: {}", via.id)));
        }
        copper.push(render(&format!(
            "(via (at {}) (size {}) (drill {}) (layers \"F.Cu\" \"B.Cu\") (net {}) (uuid {}))",
            xy(via.at)?,
            num(via.size)?,
            num(via.drill)?,
            net_number(via.net_id.as_deref())?,
            uuid_text(&board.id, "via", &via.id)?
        ))?);
    }
    let edges = edge_forms(&plan.contours, &board.id)?;
    let mut all_nets = BTreeMap::<u32, String>::new();
    for id in &board.net_ids {
        let net = net_by_id(&doc.nets, id).expect("validated");
        all_nets.insert(board_net_index[id.as_str()], net.name.clone());
    }
    for net in generated_nets {
        all_nets.insert(net.index, net.name.clone());
    }
    let net_forms = all_nets
        .iter()
        .map(|(index, name)| render(&format!("(net {index} {})", q(name)?)))
        .collect::<Result<Vec<_>, ArtifactError>>()?;
    let content = format!(
        "(kicad_pcb (version {FILE_VERSION}) (generator \"BoardStudio\")\n  (general (thickness {}))\n  (paper \"A4\")\n  (layers (0 \"F.Cu\" signal) (31 \"B.Cu\" signal) (44 \"Edge.Cuts\" user) (46 \"B.CrtYd\" user) (47 \"F.CrtYd\" user) (48 \"B.Fab\" user) (49 \"F.Fab\" user))\n  (net 0 \"\")\n  {}\n  {}\n  {}\n  {}\n  {}\n)\n",
        num(board.thickness)?,
        net_forms.join("\n  "),
        footprints.join("\n  "),
        generated_objects.join("\n  "),
        copper.join("\n  "),
        edges.join("\n  ")
    );
    Ok(vec![ArtifactFile {
        filename: format!("{}.kicad_pcb", safe_name(&board.name)),
        content,
    }])
}

fn module_mount_hole_footprint(
    board_id: &str,
    module: &crate::model::ResolvedModule,
    support: &crate::model::ModuleSupportGeometry,
) -> Result<String, ArtifactError> {
    if !support.hole_diameter.is_finite() || support.hole_diameter <= 0.0 {
        return Err(validation(
            "Module host PCB drill diameter must be positive and finite",
        ));
    }
    let id = format!("{}/{}", module.id, support.mount_id);
    let reference = q(&format!("MH_{}", safe_name(&id)))?;
    let diameter = num(support.hole_diameter)?;
    let at = xy(support.at)?;
    render(&format!(
        "(footprint \"BoardStudio_ModuleMountHole\" (layer \"F.Cu\") (at {at}) (attr exclude_from_pos_files exclude_from_bom) (fp_text reference {reference} (at 0 0) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.15)) hide) (uuid {})) (fp_text value \"Designer-selected module mount drill\" (at 0 0) (layer \"F.Fab\") (effects (font (size 1 1) (thickness 0.15)) hide) (uuid {})) (pad \"\" np_thru_hole circle (at 0 0) (size {diameter} {diameter}) (drill {diameter}) (layers \"*.Cu\" \"*.Mask\") (uuid {})) (uuid {}))",
        uuid_text(board_id, "module-support-reference", &id)?,
        uuid_text(board_id, "module-support-value", &id)?,
        uuid_text(board_id, "module-support-drill", &id)?,
        uuid_text(board_id, "module-support-footprint", &id)?,
    ))
}

fn edge_forms(contours: &[Contour], scope: &str) -> Result<Vec<String>, ArtifactError> {
    let mut forms = Vec::new();
    for (contour_index, contour) in contours.iter().enumerate() {
        for index in 0..contour.points.len() {
            let start = contour.points[index];
            let end = contour.points[(index + 1) % contour.points.len()];
            if start == end {
                continue;
            }
            forms.push(render(&format!("(gr_line (start {}) (end {}) (stroke (width {}) (type solid)) (layer \"Edge.Cuts\") (uuid {}))", xy(start)?, xy(end)?, num(EDGE_WIDTH)?, uuid_text(scope, "edge", &format!("{contour_index}:{index}"))?))?);
        }
    }
    Ok(forms)
}

#[cfg(test)]
mod tests;
