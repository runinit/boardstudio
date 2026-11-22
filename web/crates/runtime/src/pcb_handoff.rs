//! Private full/draft KiCad board handoff built from the accepted Core export plan.
//!
//! Wiring mutations remain in Session (through export-owned commits); this
//! module owns only the single Core `ExportPcb` request
//! and the two archive layers used by the reference handoff.

use crate::{core_services::ArchiveHandler, portable_archive::pack_files};
use boardstudio_application::{AcceptedSnapshot, OperationId, Scope};
use boardstudio_core_kernel::hash::sha256_hex;
use boardstudio_core_mechanical::electrical::ElectricalPlan;
use boardstudio_model::model::{
    ArtifactReply, ArtifactRequest, ExportTarget, PrepareExportRequest,
};
use boardstudio_web_host::host::{BrowserStore, CoreExecutor};
use serde_json::json;
use std::collections::BTreeMap;

pub struct HandoffSource<'a> {
    pub operation_id: OperationId,
    pub snapshot: &'a AcceptedSnapshot,
    pub scope: &'a Scope,
    pub electrical_plan: ElectricalPlan,
    pub populations: Vec<(String, ElectricalPlan)>,
    pub draft: bool,
}

pub struct HandoffPorts<'a> {
    pub core: &'a dyn CoreExecutor,
    pub store: &'a BrowserStore,
    pub archive: ArchiveHandler,
    pub executor_epoch: u64,
}

pub async fn build_handoff(
    source: HandoffSource<'_>,
    ports: HandoffPorts<'_>,
    is_current: impl Fn() -> Result<(), String>,
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
        archive,
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
    let request_id = format!("pcb-handoff-{}", operation_id.0);
    let request = ArtifactRequest::ExportPcb {
        id: request_id.clone(),
        request: PrepareExportRequest {
            snapshot_token: snapshot.token.0.to_string(),
            expected_revision: snapshot.document.revision,
            document: document.clone(),
            target,
            contours,
            model_paths,
        },
    };
    is_current()?;
    let reply = core
        .artifact(&request_id, &executor_epoch.to_string(), &request)
        .await
        .map_err(|error| format!("KiCad export failed: {error}"))?;
    is_current()?;
    let exported = match reply {
        ArtifactReply::ExportPcb { id, result } if id == request_id => result,
        ArtifactReply::Error { id, error } if id == request_id => {
            return Err(format!("Core rejected KiCad export: {}", error.message));
        }
        ArtifactReply::ExportPcb { .. } | ArtifactReply::Error { .. } => {
            return Err("Core returned KiCad output for another request.".into());
        }
        _ => return Err("Core returned an unexpected KiCad export reply.".into()),
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
    is_current()?;
    let board_archive = pack_files(archive, model_files)?;

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
    is_current()?;
    pack_files(archive, handoff)
}

async fn load_model_files(
    document: &boardstudio_model::model::ProjectDoc,
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
        let actual_hash = sha256_hex(&bytes);
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

fn jumper_diagram(recipe: &boardstudio_model::electrical_jumpers::JumperRecipe) -> String {
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
