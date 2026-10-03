//! Private full/draft KiCad board handoff built from the accepted Core export plan.
//!
//! Wiring mutations remain in Session (through export-owned commits); this
//! module owns only the existing Core → preview-generator → Core artifact
//! sequence and the two archive layers used by the reference handoff.

use boardstudio_application::{AcceptedSnapshot, OperationId, Scope};
use boardstudio_core::{
    electrical::ElectricalPlan,
    model::{
        ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, ExportTarget,
        FinishExportRequest, PrepareExportRequest,
    },
};
use boardstudio_web::host::{BrowserStore, CoreWorker};
use js_sys::Uint8Array;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, rc::Rc};

use crate::preview_generator::PreviewGeneratorClient;

pub(crate) struct HandoffSource<'a> {
    pub operation_id: OperationId,
    pub snapshot: &'a AcceptedSnapshot,
    pub scope: &'a Scope,
    pub electrical_plan: ElectricalPlan,
    pub populations: Vec<(String, ElectricalPlan)>,
    pub draft: bool,
}

pub(crate) struct HandoffPorts<'a> {
    pub core: &'a CoreWorker,
    pub store: &'a BrowserStore,
    pub executor_epoch: u64,
}

pub(crate) async fn build_handoff(
    source: HandoffSource<'_>,
    ports: HandoffPorts<'_>,
    is_current: impl Fn() -> Result<(), String>,
    preview_generator: impl FnOnce() -> Result<Rc<PreviewGeneratorClient>, String>,
) -> Result<Vec<u8>, String> {
    let HandoffSource {
        operation_id,
        snapshot,
        scope,
        electrical_plan,
        populations,
        draft,
    } = source;
    let HandoffPorts {
        core,
        store,
        executor_epoch,
    } = ports;
    let document = snapshot.document.as_ref();
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)
        .ok_or_else(|| "Select a board before export".to_owned())?;
    let used_ids = board
        .part_ids
        .iter()
        .filter_map(|id| document.parts.iter().find(|part| part.id == *id))
        .map(|part| part.definition_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut model_document = (*document).clone();
    model_document
        .definitions
        .retain(|definition| used_ids.contains(definition.id.as_str()));
    model_document
        .parts
        .retain(|part| board.part_ids.contains(&part.id));
    let model_ids = crate::bundled_models::footprint_export_model_ids(&model_document).await;
    is_current()?;
    let model_ids = model_ids?;
    let (model_paths, mut model_files) =
        load_model_files(document, store, &model_ids, &is_current).await?;

    let contours = snapshot
        .scene
        .board_contours
        .iter()
        .find(|entry| entry.board_id == scope.board_id)
        .map(|entry| entry.contours.clone())
        .unwrap_or_default();
    let target = ExportTarget::Board {
        board_id: scope.board_id.clone(),
    };
    let prepare_id = format!("pcb-handoff-{}-prepare", operation_id.0);
    let prepare = ArtifactRequest::PrepareExport {
        id: prepare_id.clone(),
        request: PrepareExportRequest {
            snapshot_token: snapshot.token.0.to_string(),
            expected_revision: snapshot.document.revision,
            document: document.clone(),
            target: target.clone(),
            contours,
            model_paths,
        },
    };
    is_current()?;
    let reply = core
        .artifact(&prepare_id, &executor_epoch.to_string(), &prepare)
        .await
        .map_err(|error| format!("KiCad export preparation failed: {error}"))?;
    is_current()?;
    let plan = match reply {
        ArtifactReply::PrepareExport { id, result } if id == prepare_id => *result,
        ArtifactReply::Error { id, error } if id == prepare_id => {
            return Err(format!("Core rejected KiCad export: {}", error.message));
        }
        ArtifactReply::PrepareExport { .. } | ArtifactReply::Error { .. } => {
            return Err("Core returned an export plan for another request.".into());
        }
        _ => return Err("Core returned an unexpected KiCad preparation reply.".into()),
    };
    if plan.snapshot_token != snapshot.token.0.to_string()
        || plan.revision != snapshot.document.revision
        || plan.target != target
        || plan.captured_document.id != snapshot.document.id
    {
        return Err("Core returned an export plan for another accepted board.".into());
    }

    let results = if plan.jobs.is_empty() {
        vec![]
    } else {
        if operation_id.0 == 0 || operation_id.0 > 9_007_199_254_740_991 {
            return Err("KiCad worker identity is outside the safe integer range.".into());
        }
        let worker = preview_generator();
        is_current()?;
        let worker = worker?;
        let request = json!({
            "kind": "generate-preview-jobs",
            "worker_generation": operation_id.0,
            "request_id": operation_id.0,
            "owner": {
                "scope": {
                    "sessionEpoch": scope.session_epoch.0,
                    "documentId": scope.document_id,
                    "boardId": scope.board_id,
                    "instanceId": scope.instance_id,
                },
                "token": snapshot.token.0.to_string(),
                "viewer_instance": 0,
                "projection_generation": operation_id.0,
            },
            "batch": {
                "accepted_revision": snapshot.document.revision,
                "batch_generation": operation_id.0,
            },
            "plan_key": {
                "snapshot_token": plan.snapshot_token,
                "revision": plan.revision,
                "job_ids": plan.jobs.iter().map(|job| job.job_id.clone()).collect::<Vec<_>>(),
            },
            "jobs": plan.jobs,
            "reserved_nets": plan.reserved_nets,
            "next_net_index": plan.next_net_index,
            "paths": plan.model_paths.iter().collect::<Vec<_>>(),
        });
        let response = worker.generate(operation_id.0, &request).await;
        is_current()?;
        let response = response?;
        crate::runtime::validate_preview_worker_envelope(
            &response,
            &request,
            operation_id.0,
            operation_id.0,
        )?;
        serde_json::from_value::<Vec<boardstudio_core::model::ErgogenJobResult>>(
            response
                .get("results")
                .cloned()
                .ok_or_else(|| "Ergogen worker returned no KiCad conversion results.".to_owned())?,
        )
        .map_err(|error| format!("Ergogen worker returned malformed KiCad results: {error}"))?
    };

    let finish_id = format!("pcb-handoff-{}-finish", operation_id.0);
    let finish = ArtifactRequest::FinishExport {
        id: finish_id.clone(),
        request: FinishExportRequest {
            plan: plan.clone(),
            results,
        },
    };
    is_current()?;
    let reply = core
        .artifact(&finish_id, &executor_epoch.to_string(), &finish)
        .await
        .map_err(|error| format!("KiCad serialization failed: {error}"))?;
    is_current()?;
    let exported = match reply {
        ArtifactReply::FinishExport { id, result } if id == finish_id => result,
        ArtifactReply::Error { id, error } if id == finish_id => {
            return Err(format!(
                "Core rejected KiCad serialization: {}",
                error.message
            ));
        }
        ArtifactReply::FinishExport { .. } | ArtifactReply::Error { .. } => {
            return Err("Core returned KiCad output for another request.".into());
        }
        _ => return Err("Core returned an unexpected KiCad serialization reply.".into()),
    };
    if exported.snapshot_token != snapshot.token.0.to_string()
        || exported.revision != snapshot.document.revision
    {
        return Err("Core returned KiCad output for another accepted revision.".into());
    }
    let board_file = exported
        .files
        .first()
        .ok_or_else(|| "Core returned no KiCad board file.".to_owned())?;
    push_file(
        &mut model_files,
        board_file.filename.clone(),
        board_file.content.as_bytes().to_vec(),
    )?;
    let board_archive = pack_files(
        operation_id,
        core,
        executor_epoch,
        "board",
        model_files,
        &is_current,
    )
    .await?;

    let mut handoff = vec![(format!("{}-kicad.zip", board.name), board_archive)];
    let report = serde_json::to_vec_pretty(&json!({
        "draft": draft,
        "plan": electrical_plan,
        "populations": populations.iter().map(|(name, plan)| json!({ "name": name, "plan": plan })).collect::<Vec<_>>(),
    }))
    .map_err(|error| format!("Could not encode the wiring report: {error}"))?;
    push_file(&mut handoff, "wiring-report.json".into(), report)?;
    let assemblies = if populations.is_empty() {
        vec![("PCB assembly", &electrical_plan)]
    } else {
        populations
            .iter()
            .map(|(name, plan)| (name.as_str(), plan))
            .collect()
    };
    let instructions = assemblies
        .iter()
        .map(|(name, plan)| assembly_instructions(name, plan, draft))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    push_file(
        &mut handoff,
        "ASSEMBLY.md".into(),
        instructions.into_bytes(),
    )?;
    for (population_index, (_, population)) in assemblies.iter().enumerate() {
        for (recipe_index, recipe) in population.jumpers.iter().enumerate() {
            push_file(
                &mut handoff,
                format!(
                    "jumpers/assembly-{}/part-{}.svg",
                    population_index + 1,
                    recipe_index + 1
                ),
                jumper_diagram(recipe).into_bytes(),
            )?;
        }
    }
    pack_files(
        operation_id,
        core,
        executor_epoch,
        "handoff",
        handoff,
        &is_current,
    )
    .await
}

async fn load_model_files(
    document: &boardstudio_core::model::ProjectDoc,
    store: &BrowserStore,
    ids: &[String],
    is_current: &impl Fn() -> Result<(), String>,
) -> Result<(BTreeMap<String, String>, Vec<(String, Vec<u8>)>), String> {
    let mut paths = BTreeMap::new();
    let mut files = Vec::new();
    for id in ids {
        is_current()?;
        let asset = document.assets.iter().find(|asset| asset.id == *id);
        let bundled = asset
            .is_none()
            .then(|| crate::bundled_models::bundled_model(id))
            .flatten();
        let filename = asset
            .map(|asset| asset.name.as_str())
            .or_else(|| bundled.map(|model| model.filename))
            .ok_or_else(|| format!("Model asset {id} needs a STEP, STP, STL, or WRL filename"))?;
        let extension = filename
            .rsplit_once('.')
            .map(|(_, extension)| extension.to_ascii_lowercase())
            .filter(|extension| matches!(extension.as_str(), "step" | "stp" | "stl" | "wrl"))
            .ok_or_else(|| {
                format!("Model asset {filename} needs a STEP, STP, STL, or WRL filename")
            })?;
        let (expected_hash, bytes) = if let Some(asset) = asset {
            let bytes = store
                .load_asset(asset.sha256.clone())
                .await
                .map_err(|error| format!("Could not load model asset {}: {error}", asset.name))?
                .ok_or_else(|| format!("Model asset {} is missing", asset.name))?
                .to_vec();
            is_current()?;
            (asset.sha256.to_ascii_lowercase(), bytes)
        } else {
            let model =
                bundled.ok_or_else(|| format!("Bundled model asset {id} is unavailable"))?;
            let bytes = crate::bundled_models::bundled_model_bytes(id).await?;
            is_current()?;
            (model.sha256.to_ascii_lowercase(), bytes)
        };
        let actual_hash = Sha256::digest(&bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        if actual_hash != expected_hash {
            return Err(format!(
                "Model asset {filename} failed its content hash check"
            ));
        }
        let path = format!("models/{actual_hash}.{extension}");
        paths.insert(id.clone(), path.clone());
        push_file(&mut files, path, bytes)?;
    }
    Ok((paths, files))
}

async fn pack_files(
    operation_id: OperationId,
    core: &CoreWorker,
    executor_epoch: u64,
    label: &str,
    files: Vec<(String, Vec<u8>)>,
    is_current: &impl Fn() -> Result<(), String>,
) -> Result<Vec<u8>, String> {
    is_current()?;
    let entries = files
        .iter()
        .enumerate()
        .map(|(index, (path, _))| {
            Ok(ArchiveEntry {
                path: path.clone(),
                buffer_index: u32::try_from(index)
                    .map_err(|_| "Too many KiCad handoff files.".to_owned())?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let metadata = serde_json::to_string(&ArchiveRequest::PackFiles { entries })
        .map_err(|error| format!("Could not prepare KiCad archive request: {error}"))?;
    let buffers = files
        .iter()
        .map(|(_, bytes)| Uint8Array::from(bytes.as_slice()))
        .collect();
    let request_id = format!("pcb-handoff-{}-{label}-archive", operation_id.0);
    let packed = core
        .archive(&request_id, &executor_epoch.to_string(), &metadata, buffers)
        .await
        .map_err(|error| format!("KiCad archive packaging failed: {error}"))?;
    is_current()?;
    match serde_json::from_str::<ArchiveReply>(&packed.metadata)
        .map_err(|error| format!("Could not read KiCad archive result: {error}"))?
    {
        ArchiveReply::Packed => packed
            .buffers
            .first()
            .map(Uint8Array::to_vec)
            .filter(|bytes| !bytes.is_empty())
            .ok_or_else(|| "KiCad archive provider returned no bytes.".into()),
        ArchiveReply::Error { message } => {
            Err(format!("KiCad archive packaging failed: {message}"))
        }
        ArchiveReply::Unpacked { .. } => {
            Err("Core returned an unexpected unpacked KiCad archive.".into())
        }
    }
}

fn push_file(
    files: &mut Vec<(String, Vec<u8>)>,
    path: String,
    bytes: Vec<u8>,
) -> Result<(), String> {
    if files.iter().any(|(existing, _)| existing == &path) {
        return Err(format!("KiCad handoff path repeats: {path}"));
    }
    files.push((path, bytes));
    Ok(())
}

fn assembly_instructions(name: &str, plan: &ElectricalPlan, draft: bool) -> String {
    let mut lines = vec![
        format!("# {name}"),
        String::new(),
        "# PCB wiring and assembly".into(),
        String::new(),
        if draft {
            "DRAFT: review the unresolved findings below before fabrication.".into()
        } else {
            "Ready for routing. This package does not certify routed-board DRC or fabrication readiness.".into()
        },
        String::new(),
        format!("PCB: {}", plan.board_id.as_deref().unwrap_or("Unresolved")),
        format!("Wiring revision: {}", plan.revision),
        format!("Controller: {}", plan.controller_part_id.as_deref().unwrap_or("Unresolved")),
        String::new(),
        "The PCB file retains separate local nets on the two sides of every open solder gap. Close only the bridges listed below for this population. Leave the opposite-face bridges open.".into(),
        String::new(),
    ];
    for recipe in &plan.jumpers {
        lines.extend([
            format!("## {}", recipe.part_id),
            String::new(),
            format!("Footprint: {}", recipe.source),
            String::new(),
            "| Face | Local socket net | Signal | Action |".into(),
            "| --- | --- | --- | --- |".into(),
        ]);
        for site in &recipe.sites {
            lines.push(format!(
                "| {} | {} | {} | {}{} |",
                site.face,
                site.local_net_id,
                site.signal_terminal,
                if site.close { "Bridge" } else { "Leave open" },
                if site.routing_required {
                    "; route local connection first"
                } else {
                    ""
                }
            ));
        }
        lines.push(String::new());
    }
    if !plan.diagnostics.is_empty() {
        lines.extend(["## Findings".into(), String::new()]);
        lines.extend(
            plan.diagnostics
                .iter()
                .map(|finding| format!("- {}: {}", finding.severity, finding.message)),
        );
    }
    lines.extend([
        String::new(),
        "For a wired split, use local power on each half and a straight TRRS cable: tip carries central TX / peripheral RX; ring 2 carries peripheral TX / central RX; sleeve is ground; ring 1 is unused. Disconnect power before plugging or unplugging.".into(),
    ]);
    lines.join("\n")
}

fn jumper_diagram(recipe: &boardstudio_core::electrical_jumpers::JumperRecipe) -> String {
    let min_y = recipe.sites.iter().map(|site| site.y).fold(0.0, f64::min);
    let max_y = recipe.sites.iter().map(|site| site.y).fold(1.0, f64::max);
    let height = ((max_y - min_y) * 18.0 + 110.0).max(200.0);
    let sites = recipe
        .sites
        .iter()
        .map(|site| {
            let back = site.face == "back";
            let x = (if back { 460.0 } else { 160.0 })
                + (if back { -site.x } else { site.x }) * 12.0;
            let y = (site.y - min_y) * 18.0 + 70.0;
            let color = if site.close { "#126b4b" } else { "#64748b" };
            let fill = if site.close { color } else { "white" };
            format!(
                "<g><circle cx=\"{x}\" cy=\"{y}\" r=\"5\" fill=\"{fill}\" stroke=\"{color}\"/><text x=\"{x}\" y=\"{}\" text-anchor=\"middle\" font-size=\"10\">{}</text></g>",
                y - 10.0,
                escape_xml(&site.signal_terminal)
            )
        })
        .collect::<String>();
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"620\" height=\"{height}\" viewBox=\"0 0 620 {height}\"><rect width=\"620\" height=\"{height}\" fill=\"white\"/><g font-family=\"sans-serif\" fill=\"#172033\"><text x=\"160\" y=\"25\" text-anchor=\"middle\">Front · viewed from front</text><text x=\"460\" y=\"25\" text-anchor=\"middle\">Back · viewed from back</text>{sites}<text x=\"20\" y=\"{}\" font-size=\"12\">Filled: bridge · Hollow: leave open · Route flagged local connections first</text></g></svg>",
        height - 20.0
    )
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
