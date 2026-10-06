//! Private full/draft KiCad board handoff built from the accepted Core export plan.
//!
//! Wiring mutations remain in Session (through export-owned commits); this
//! module owns only the single Core `ExportPcb` request
//! and the two archive layers used by the reference handoff.

use boardstudio_application::{AcceptedSnapshot, OperationId, Scope};
use boardstudio_core::{
    electrical::ElectricalPlan,
    model::{
        ArchiveEntry, ArchiveReply, ArchiveRequest, ArtifactReply, ArtifactRequest, ExportTarget,
        PrepareExportRequest,
    },
};
use boardstudio_web_host::host::{BrowserStore, CoreExecutor};
use js_sys::Uint8Array;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, future::Future};

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
    pub executor_epoch: u64,
}

/// Keep the irreversible protection commit behind successful package creation
/// and the same captured-owner checks used by the real Runtime path.
pub async fn package_then_protect<
    T,
    Package,
    IsCurrentBeforeProtect,
    Protect,
    ProtectFuture,
    IsCurrentBeforeDelivery,
>(
    package: Package,
    is_current_before_protect: IsCurrentBeforeProtect,
    protect: Protect,
    is_current_before_delivery: IsCurrentBeforeDelivery,
) -> Result<(Vec<u8>, T), String>
where
    Package: Future<Output = Result<Vec<u8>, String>>,
    IsCurrentBeforeProtect: FnOnce() -> Result<(), String>,
    Protect: FnOnce() -> ProtectFuture,
    ProtectFuture: Future<Output = Result<T, String>>,
    IsCurrentBeforeDelivery: FnOnce(&T) -> Result<(), String>,
{
    let bytes = package.await?;
    is_current_before_protect()?;
    let protection = protect().await?;
    is_current_before_delivery(&protection)?;
    Ok((bytes, protection))
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
    core: &dyn CoreExecutor,
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

#[cfg(all(test, target_arch = "wasm32"))]
mod orchestration_tests {
    use super::package_then_protect;
    use boardstudio_application::{
        AcceptedSnapshot, Completion, Effect, Event, ExportCommitRequest, OperationId, Scope,
        Session, SnapshotToken,
    };
    use boardstudio_core::{
        CoreEngine,
        electrical::{ElectricalMode, ElectricalPlan, ElectricalPlanRequest},
        model::{CoreReply, CoreRequest, ProjectDoc},
    };
    use std::{
        cell::{Cell, RefCell},
        collections::VecDeque,
        rc::Rc,
    };
    use wasm_bindgen_test::wasm_bindgen_test;

    type SharedSession = Rc<RefCell<Session>>;
    type SharedEngine = Rc<RefCell<CoreEngine>>;

    #[wasm_bindgen_test]
    async fn failed_package_retry_and_stale_owner_use_session_protection_order() {
        let (session, engine, accepted, scope) = session_fixture();
        let first_export = OperationId(901);
        let first_token = start_export(&session, first_export, scope.clone());
        let protection_calls = Cell::new(0);

        let failed = package_then_protect(
            async { Err("injected archive packaging failure".to_owned()) },
            || require_current(&session, first_export, first_token, &scope),
            || async {
                protection_calls.set(protection_calls.get() + 1);
                commit_protection(
                    &session,
                    &engine,
                    first_export,
                    first_token,
                    &scope,
                    resolve_plan(&engine, &accepted, &scope),
                )
                .await
            },
            |token| require_current(&session, first_export, *token, &scope),
        )
        .await;
        assert_eq!(failed, Err("injected archive packaging failure".to_owned()));
        assert_eq!(protection_calls.get(), 0);
        assert!(!has_protection(&session, &scope));

        let failure_effects = session.borrow_mut().complete(Completion::ExportFailed {
            operation_id: first_export,
            reason: "archive packaging failed".into(),
        });
        settle_session(&session, &engine, failure_effects);
        let retry_export = OperationId(902);
        let retry_token = start_export(&session, retry_export, scope.clone());
        let retry_accepted = session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .expect("accepted project for retry")
            .clone();
        let retry_plan = resolve_plan(&engine, &retry_accepted, &scope);
        let retried = package_then_protect(
            async { Ok(vec![0x50, 0x4b]) },
            || require_current(&session, retry_export, retry_token, &scope),
            || async {
                protection_calls.set(protection_calls.get() + 1);
                commit_protection(
                    &session,
                    &engine,
                    retry_export,
                    retry_token,
                    &scope,
                    retry_plan,
                )
                .await
            },
            |token| require_current(&session, retry_export, *token, &scope),
        )
        .await
        .expect("the same export action can retry after package failure");
        assert_eq!(retried.0, vec![0x50, 0x4b]);
        assert_eq!(protection_calls.get(), 1);
        assert!(has_protection(&session, &scope));

        let (stale_session, stale_engine, _stale_accepted, stale_scope) = session_fixture();
        let stale_export = OperationId(903);
        let stale_token = start_export(&stale_session, stale_export, stale_scope.clone());
        let stale_protection_calls = Cell::new(0);
        let stale_mutation_pending = Cell::new(false);
        let stale_accepted = stale_session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .expect("accepted project for stale export")
            .clone();
        let stale_plan = resolve_plan(&stale_engine, &stale_accepted, &stale_scope);
        let stale_result = package_then_protect(
            async {
                stale_mutation_pending.set(true);
                Ok(vec![0x50, 0x4b])
            },
            || {
                if stale_mutation_pending.replace(false) {
                    reopen_current_document(&stale_session, &stale_engine);
                }
                require_current(&stale_session, stale_export, stale_token, &stale_scope)
            },
            || async {
                stale_protection_calls.set(stale_protection_calls.get() + 1);
                commit_protection(
                    &stale_session,
                    &stale_engine,
                    stale_export,
                    stale_token,
                    &stale_scope,
                    stale_plan,
                )
                .await
            },
            |token| require_current(&stale_session, stale_export, *token, &stale_scope),
        )
        .await;
        assert_eq!(
            stale_result,
            Err("KiCad export was superseded before wiring protection.".to_owned())
        );
        assert_eq!(stale_protection_calls.get(), 0);
        assert!(!has_protection(&stale_session, &stale_scope));
    }

    fn session_fixture() -> (SharedSession, SharedEngine, AcceptedSnapshot, Scope) {
        let (session, accepted, scope) =
            crate::runtime::firmware_export_test_support::opened_session();
        let mut engine = CoreEngine::new();
        let _ = engine.handle(CoreRequest::Open {
            id: "seed-session-core".into(),
            document: (*accepted.document).clone(),
        });
        (
            Rc::new(RefCell::new(session)),
            Rc::new(RefCell::new(engine)),
            accepted,
            scope,
        )
    }

    fn start_export(
        session: &SharedSession,
        operation_id: OperationId,
        scope: Scope,
    ) -> SnapshotToken {
        session
            .borrow_mut()
            .submit(Event::StartExport {
                operation_id,
                scope,
            })
            .into_iter()
            .find_map(|effect| match effect {
                Effect::RunExport { snapshot, .. } => Some(snapshot.token),
                _ => None,
            })
            .expect("Session starts the accepted export owner")
    }

    fn resolve_plan(
        engine: &SharedEngine,
        accepted: &AcceptedSnapshot,
        scope: &Scope,
    ) -> ElectricalPlan {
        match engine.borrow_mut().handle(CoreRequest::ResolveElectrical {
            id: "pcb-handoff-test-resolve".into(),
            request: ElectricalPlanRequest {
                document: (*accepted.document).clone(),
                instance_id: None,
                mode: ElectricalMode::Matrix,
                locks: Default::default(),
                controller_profile: None,
                board_id: Some(scope.board_id.clone()),
                controller_part_id: None,
            },
        }) {
            CoreReply::ElectricalResolved { plan, .. } => plan,
            other => panic!("expected a resolved electrical plan, got {other:?}"),
        }
    }

    async fn commit_protection(
        session: &SharedSession,
        engine: &SharedEngine,
        export_operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
        plan: ElectricalPlan,
    ) -> Result<SnapshotToken, String> {
        let effects = session.borrow_mut().submit(Event::ExportCommit {
            operation_id: OperationId(export_operation_id.0 + 1_000),
            export_operation_id,
            token,
            scope: scope.clone(),
            commit: ExportCommitRequest::ProtectElectricalHandoff { plan },
        });
        settle_session(session, engine, effects);
        let token = session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .map(|accepted| accepted.token)
            .ok_or_else(|| "Session lost its accepted snapshot".to_owned())?;
        if !has_protection(session, scope) {
            return Err("Session did not accept PCB handoff protection".into());
        }
        Ok(token)
    }

    fn settle_session(session: &SharedSession, engine: &SharedEngine, effects: Vec<Effect>) {
        let mut pending = VecDeque::from(effects);
        while let Some(effect) = pending.pop_front() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => {
                    let reply = engine.borrow_mut().handle(*request);
                    pending.extend(session.borrow_mut().complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }));
                }
                Effect::Persist {
                    save_attempt_id, ..
                } => {
                    pending.extend(session.borrow_mut().complete(Completion::Persist {
                        save_attempt_id,
                        result: boardstudio_application::SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
    }

    fn require_current(
        session: &SharedSession,
        operation_id: OperationId,
        token: SnapshotToken,
        scope: &Scope,
    ) -> Result<(), String> {
        if session
            .borrow()
            .export_is_current(operation_id, token, scope)
        {
            Ok(())
        } else {
            Err("KiCad export was superseded before wiring protection.".into())
        }
    }

    fn reopen_current_document(session: &SharedSession, engine: &SharedEngine) {
        let document: ProjectDoc = session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .expect("accepted project before reopen")
            .document
            .as_ref()
            .clone();
        let effects = session.borrow_mut().submit(Event::Open {
            operation_id: OperationId(999),
            document,
        });
        settle_session(session, engine, effects);
    }

    fn has_protection(session: &SharedSession, scope: &Scope) -> bool {
        session
            .borrow()
            .read_model()
            .accepted
            .as_ref()
            .and_then(|accepted| accepted.document.hardware.as_ref())
            .is_some_and(|hardware| {
                hardware.boards.iter().any(|board| {
                    board.board_id == scope.board_id && board.protected_handoff.is_some()
                })
            })
    }
}
