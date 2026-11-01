pub mod archive;
pub mod artifact;
mod case;
mod constraints;
pub mod electrical;
pub mod electrical_jumpers;
pub mod electrical_peripherals;
pub mod electrical_profiles;
pub mod firmware;
pub mod generators;
mod geometry;
mod hardware;
pub mod hash;
mod inputs;
mod keycaps;
mod keymap;
mod matrix;
pub mod mechanical;
mod mechanical_checks;
pub mod migrate;
mod mirrored_half;
pub mod model;
mod modules;
mod outline_controls;
pub mod outline_edit;
mod outline_validation;
mod outline_versions;
mod script;
mod validate;

use geometry::{OutlineCache, outlines};
pub use matrix::kernel as layout_kernel;
use matrix::layout;
pub use matrix::layout::evaluated_part_pose;
pub use matrix::layout::half_offset;
pub use matrix::layout::is_matrix_key;
pub use matrix::layout::layout_half_offset;
pub use matrix::layout::part_correspondence;
pub use matrix::layout::proposed_creation_gap;
use model::*;
use std::collections::{BTreeMap, BTreeSet};
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub struct CoreEngine {
    document: ProjectDoc,
    undo: Vec<ProjectDoc>,
    redo: Vec<ProjectDoc>,
    contours: Vec<Contour>,
    findings: Vec<Finding>,
    outline_cache: OutlineCache,
    /// Session high-water marks; not part of any Undo snapshot or saved document.
    identity_ledger: mechanical::edit::MechanicalIdentityLedger,
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
impl CoreEngine {
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen(constructor))]
    pub fn new() -> Self {
        Self::default()
    }

    // Worker transport keeps the Rust and TypeScript protocols identical.
    pub fn request(&mut self, json: &str) -> String {
        let preflight = serde_json::from_str::<serde_json::Value>(json)
            .map_err(|error| error.to_string())
            .and_then(|value| {
                let candidate = if value["kind"] == "open" {
                    Some(&value["document"])
                } else if value["kind"] == "edit"
                    && value["command"]["operation"]["kind"] == "replace-document"
                {
                    Some(&value["command"]["operation"]["document"])
                } else {
                    None
                };
                candidate.map_or(Ok(()), migrate::require_current_value)
            });
        if let Err(message) = preflight {
            return serde_json::to_string(&CoreReply::Error {
                id: String::new(),
                message,
                revision: self.document.revision,
            })
            .expect("reply serializes");
        }
        let reply = match serde_json::from_str::<CoreRequest>(json) {
            Ok(request) => self.handle(request),
            Err(error) => CoreReply::Error {
                id: String::new(),
                message: error.to_string(),
                revision: self.document.revision,
            },
        };
        serde_json::to_string(&reply).expect("reply serializes")
    }
}

/// Handle an artifact request without mutating the stateful document engine.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn artifact_request(json: &str) -> String {
    artifact::request(json)
}

/// Whether saved Case geometry for a board is current and usable as a Case
/// input with a current outer board contour, independently of PCB-coupled
/// `BoardReadiness::case_ready`.
///
/// This is a derived capability, not part of the serialized readiness
/// contract: authored Case geometry can be prepared/exported without claiming
/// that the board's PCB is ready.
pub fn authored_case_geometry_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
) -> bool {
    if scene.revision != document.revision
        || !document.boards.iter().any(|board| board.id == board_id)
    {
        return false;
    }
    let Some(readiness) = scene
        .board_readiness
        .iter()
        .find(|readiness| readiness.board_id == board_id)
    else {
        return false;
    };
    if !readiness.outline
        || !scene
            .board_contours
            .iter()
            .find(|contours| contours.board_id == board_id)
            .is_some_and(|entry| entry.contours.iter().any(|contour| !contour.hole))
    {
        return false;
    }
    if !document
        .case_bodies
        .iter()
        .any(|body| body.board_id == board_id)
    {
        return false;
    }
    !scene.findings.iter().any(|finding| {
        finding.severity == Severity::Error
            && finding.scope == Scope::Case
            // Material references affect BOM metadata, not prepared solid
            // geometry; Core's case preparer intentionally accepts them.
            && !validate::is_case_material_finding_id(&finding.id)
            && (finding.target_ids.is_empty()
                || finding.target_ids.iter().any(|target| {
                    target == board_id
                        || document.case_bodies.iter().any(|body| {
                            body.board_id == board_id && body.id.as_str() == target.as_str()
                        })
                }))
    })
}

/// Returns Core's reason when a linked item cannot change companion placement.
/// Accepts either the source or mirror part id so Inspectors can normalize either selection.
pub fn companion_placement_disabled_reason(
    document: &ProjectDoc,
    pair_id: &str,
    item_id: &str,
) -> Option<&'static str> {
    mirrored_half::companion_placement_disabled_reason(document, pair_id, item_id)
}

/// Whether Case preparation may run for a selected board. A resolved
/// mechanical configuration remains sufficient; otherwise saved authored
/// Case geometry is admitted by its own capability above.
pub fn case_preparation_ready(
    document: &ProjectDoc,
    scene: &SceneDelta,
    board_id: &str,
    configured: bool,
) -> bool {
    scene
        .board_readiness
        .iter()
        .any(|readiness| readiness.board_id == board_id)
        && (configured || authored_case_geometry_ready(document, scene, board_id))
}

/// Archive payloads cross WASM as typed buffers, independently of JSON metadata.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn archive_request(metadata: &str, buffers: js_sys::Array) -> js_sys::Array {
    use wasm_bindgen::JsCast;
    let inputs: Result<Vec<Vec<u8>>, _> = buffers
        .iter()
        .map(|value| {
            value
                .dyn_into::<js_sys::Uint8Array>()
                .map(|bytes| bytes.to_vec())
        })
        .collect();
    let (reply, outputs) = match inputs {
        Ok(inputs) => archive::request(metadata, &inputs),
        Err(_) => (
            serde_json::to_string(&ArchiveReply::Error {
                message: "Archive buffers must be Uint8Array values".into(),
            })
            .expect("archive error serializes"),
            Vec::new(),
        ),
    };
    let result = js_sys::Array::new();
    result.push(&JsValue::from_str(&reply));
    let encoded = js_sys::Array::new();
    for bytes in outputs {
        encoded.push(&js_sys::Uint8Array::from(bytes.as_slice()));
    }
    result.push(&encoded);
    result
}

impl Default for CoreEngine {
    fn default() -> Self {
        Self {
            document: ProjectDoc::empty("untitled", "Untitled"),
            undo: vec![],
            redo: vec![],
            contours: vec![],
            findings: vec![],
            outline_cache: OutlineCache::default(),
            identity_ledger: mechanical::edit::MechanicalIdentityLedger::default(),
        }
    }
}

impl CoreEngine {
    /// Validate a typed Open candidate before retiring editor work, without changing Core.
    pub fn validate_open(document: &ProjectDoc) -> Result<(), String> {
        Self::prepare_open(document).map(|_| ())
    }

    fn prepare_open(document: &ProjectDoc) -> Result<(ProjectDoc, Vec<Finding>), String> {
        if document.format != "boardstudio/v2" {
            return Err("Unsupported document format".into());
        }
        migrate::require_current_version(document.format_version)?;
        if document.revision == u64::MAX {
            return Err("Document revision is exhausted".into());
        }
        if let Some(message) = validate::duplicate_top_level_identity(document) {
            return Err(message);
        }
        let prepared = mirrored_half::prepare_open(document, &[])?;
        if let Some(message) = validate::duplicate_top_level_identity(&prepared.0) {
            return Err(message);
        }
        Ok(prepared)
    }

    pub fn handle(&mut self, request: CoreRequest) -> CoreReply {
        match request {
            CoreRequest::ResolveModules {
                id,
                document,
                board_id,
                preview_top_z,
            } => {
                let mut result = modules::resolve(&document, &board_id);
                match modules::prepare_preview(
                    &document,
                    &board_id,
                    &mut result,
                    preview_top_z.unwrap_or(0.0),
                ) {
                    Ok(()) => CoreReply::ModulesResolved { id, result },
                    Err(message) => CoreReply::Error {
                        id,
                        message,
                        revision: document.revision,
                    },
                }
            }
            CoreRequest::ResolveKeycaps {
                id,
                document,
                board_id,
                cases,
            } => CoreReply::KeycapsResolved {
                id,
                result: keycaps::resolve(&document, &board_id, cases.as_ref()),
            },
            CoreRequest::GenerateFirmware { id, request } => match firmware::generate(&request) {
                Ok(package) => CoreReply::FirmwareGenerated { id, package },
                Err(message) => self.error(id, &message),
            },
            CoreRequest::Open { id, document } => {
                let (document, repaired_findings) = match Self::prepare_open(&document) {
                    Ok(prepared) => prepared,
                    Err(message) => return self.error(id, &message),
                };
                // A successful explicit Open starts a new document session. Failed
                // opens (above) leave the previous session's reservations intact.
                self.identity_ledger =
                    mechanical::edit::MechanicalIdentityLedger::seeded(&document);
                self.document = document;
                self.undo.clear();
                self.redo.clear();
                self.recompute();
                self.findings.extend(repaired_findings);
                self.scene(
                    id,
                    SceneInput {
                        transaction: "open",
                        changed: vec![],
                        document: &self.document,
                        contours: &self.contours,
                        geom_findings: &self.findings,
                        cache: &self.outline_cache,
                        kind: SceneKind::Committed,
                    },
                )
            }
            CoreRequest::Edit { id, command } => self.edit(id, command),
            CoreRequest::Undo { id } => self.history(id, History::Undo),
            CoreRequest::Redo { id } => self.history(id, History::Redo),
            CoreRequest::MechanicalProfile {
                id,
                definition_id,
                source,
                plate_to_pcb,
            } => match mechanical::builtin_profile(definition_id, source, plate_to_pcb) {
                Ok(profile) => CoreReply::MechanicalProfile { id, profile },
                Err(message) => CoreReply::Error {
                    id,
                    message,
                    revision: self.document.revision,
                },
            },
            CoreRequest::ResolveMechanical {
                id,
                document,
                contours,
            } => CoreReply::MechanicalResolved {
                id,
                assembly: mechanical::resolve(&document, &contours),
            },
            CoreRequest::PrepareCase { id, ir } => match case::prepare(&ir) {
                Ok(ir) => CoreReply::CasePrepared { id, ir },
                Err(message) => CoreReply::Error {
                    id,
                    message,
                    revision: ir.revision,
                },
            },
            CoreRequest::Snapshot { id } => self.scene(
                id,
                SceneInput {
                    transaction: "snapshot",
                    changed: vec![],
                    document: &self.document,
                    contours: &self.contours,
                    geom_findings: &self.findings,
                    cache: &self.outline_cache,
                    kind: SceneKind::Committed,
                },
            ),
            CoreRequest::ProjectMatrices {
                id,
                base_revision,
                matrices,
            } => {
                if base_revision != self.document.revision {
                    return self.error(id, "Stale base revision");
                }
                let mut projection_doc = self.document.clone();
                let mut seen = BTreeSet::new();
                for matrix in &matrices {
                    if !seen.insert(&matrix.id) {
                        return self.error(id, "Duplicate draft matrix identity");
                    }
                    if let Some(existing) = projection_doc
                        .matrices
                        .iter_mut()
                        .find(|existing| existing.id == matrix.id)
                    {
                        *existing = matrix.clone();
                    } else {
                        projection_doc.matrices.push(matrix.clone());
                    }
                }
                if let Err(message) = layout::refresh_existing(&mut projection_doc) {
                    return self.error(id, &message);
                }
                let mut scenes = Vec::with_capacity(matrices.len());
                for matrix in matrices {
                    if let Err(message) = matrix::valid_projection_matrix(&matrix) {
                        return self.error(id, &message);
                    }
                    match matrix::project_matrix(
                        &matrix,
                        &projection_doc,
                        matrix::ProjectionMode::Draft,
                    ) {
                        Ok(scene) => scenes.push(scene),
                        Err(message) => return self.error(id, &message),
                    }
                }
                CoreReply::MatrixProjections {
                    id,
                    revision: self.document.revision,
                    matrix_scenes: scenes,
                }
            }
            CoreRequest::ResolveElectrical { id, request } => CoreReply::ElectricalResolved {
                id,
                plan: electrical::resolve(request),
            },
            CoreRequest::ApplyElectrical {
                id,
                base_revision,
                plan,
                draft,
            } => {
                if base_revision != self.document.revision {
                    return self.error(id, "Stale base revision");
                }
                let reviewed = electrical::resolve(electrical::ElectricalPlanRequest {
                    document: self.document.clone(),
                    instance_id: plan.instance_id.clone(),
                    mode: plan.mode,
                    locks: Default::default(),
                    controller_profile: plan.controller_profile.clone(),
                    board_id: plan.board_id.clone(),
                    controller_part_id: plan.controller_part_id.clone(),
                });
                if reviewed.fingerprint != plan.fingerprint {
                    return self.error(
                        id,
                        "The wiring inputs changed; resolve again before applying",
                    );
                }
                let mut document = self.document.clone();
                if let Err(message) =
                    electrical::materialize_reviewed(&mut document, &reviewed, draft)
                {
                    return self.error(id, &message);
                }
                self.edit(
                    id,
                    EditCommand {
                        base_revision,
                        transaction_id: "apply-wiring".into(),
                        phase: EditPhase::Commit,
                        target_ids: vec![],
                        operation: EditOperation::ReplaceDocument {
                            document: Box::new(document),
                        },
                    },
                )
            }
            CoreRequest::ReviewElectricalRemap {
                id,
                base_revision,
                board_id,
                expected_fingerprint,
            } => {
                if base_revision != self.document.revision {
                    return self.error(id, "Stale base revision");
                }
                let Some(revision) = self.document.revision.checked_add(1) else {
                    return self.error(id, "Document revision is exhausted");
                };
                let Some(config) = self.document.hardware.as_mut().and_then(|hardware| {
                    hardware
                        .boards
                        .iter_mut()
                        .find(|board| board.board_id == board_id)
                }) else {
                    return self.error(id, "PCB wiring configuration is missing");
                };
                if config
                    .protected_handoff
                    .as_ref()
                    .map(|baseline| baseline.fingerprint.as_str())
                    != Some(expected_fingerprint.as_str())
                {
                    return self.error(id, "The PCB handoff changed; review it again");
                }
                config.protected_handoff = None;
                self.document.revision = revision;
                self.scene(
                    id,
                    SceneInput {
                        transaction: "review-remap",
                        changed: vec![],
                        document: &self.document,
                        contours: &self.contours,
                        geom_findings: &self.findings,
                        cache: &self.outline_cache,
                        kind: SceneKind::Committed,
                    },
                )
            }
            CoreRequest::ProtectElectricalHandoff {
                id,
                base_revision,
                board_id,
                plan,
            } => {
                if base_revision != self.document.revision {
                    return self.error(id, "Stale base revision");
                }
                if plan.revision != self.document.revision {
                    return self.error(id, "Wiring plan is stale");
                }
                if plan.board_id.as_deref() != Some(&board_id) {
                    return self.error(id, "The handoff belongs to a different PCB");
                }
                let reviewed = electrical::resolve(electrical::ElectricalPlanRequest {
                    document: self.document.clone(),
                    instance_id: None,
                    mode: plan.mode,
                    locks: Default::default(),
                    controller_profile: plan.controller_profile.clone(),
                    board_id: plan.board_id.clone(),
                    controller_part_id: plan.controller_part_id.clone(),
                });
                if plan.instance_id.is_some() || reviewed.fingerprint != plan.fingerprint {
                    return self.error(id, "The handoff no longer matches the PCB wiring");
                }
                let hw = self.document.hardware.get_or_insert_with(Default::default);
                let index = hw.boards.iter().position(|b| b.board_id == board_id);
                let index = index.unwrap_or_else(|| {
                    hw.boards.push(ElectricalBoardConfiguration {
                        board_id: board_id.clone(),
                        ..Default::default()
                    });
                    hw.boards.len() - 1
                });
                let board = &mut hw.boards[index];
                let mut assignments = board
                    .protected_handoff
                    .as_ref()
                    .map(|baseline| baseline.assignments.clone())
                    .unwrap_or_default();
                assignments.extend(electrical::handoff_assignments(&plan));
                board.protected_handoff = Some(ElectricalHandoffBaseline {
                    fingerprint: plan.fingerprint.clone(),
                    revision: self.document.revision,
                    assignments,
                });
                self.scene(
                    id,
                    SceneInput {
                        transaction: "protect-handoff",
                        changed: vec![],
                        document: &self.document,
                        contours: &self.contours,
                        geom_findings: &self.findings,
                        cache: &self.outline_cache,
                        kind: SceneKind::Committed,
                    },
                )
            }
        }
    }

    fn edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        if command.base_revision != self.document.revision {
            return self.error(id, "Stale base revision");
        }
        // A wholesale replacement is validated before scripts or layout can normalize
        // the incoming identities; publication validates its generated result again.
        if let EditOperation::ReplaceDocument { document } = &command.operation
            && let Some(message) = validate::duplicate_top_level_identity(document)
        {
            return self.error(id, &message);
        }
        let prepared_route = uses_prepared_transaction(&self.document, &command.operation);
        if command.phase == EditPhase::Preview && prepared_route {
            return self.preview_prepared_edit(id, command);
        }
        if command.phase == EditPhase::Preview
            && matches!(
                &command.operation,
                EditOperation::MoveParts { .. }
                    | EditOperation::SetKeycapSize { .. }
                    | EditOperation::SetMatrix { .. }
                    | EditOperation::SetMatrixSplay { .. }
                    | EditOperation::SetMatrixPlacement { .. }
            )
        {
            return self.preview_edit(id, command);
        }
        let prepared_mechanical_contours = if prepared_route
            && command.phase == EditPhase::Commit
            && matches!(command.operation, EditOperation::ApplyMechanicalEdit { .. })
        {
            Some(geometry::board_contours(&self.document, &self.outline_cache).0)
        } else {
            None
        };
        if let (Some(board_contours), EditOperation::ApplyMechanicalEdit { edit }) =
            (&prepared_mechanical_contours, &command.operation)
            && matches!(
                mechanical::evaluate_edit(&self.document, board_contours, edit),
                Ok(mechanical::MechanicalEditDisposition::Unchanged)
            )
        {
            return self.scene(
                id,
                SceneInput {
                    transaction: &command.transaction_id,
                    changed: vec![],
                    document: &self.document,
                    contours: &self.contours,
                    geom_findings: &self.findings,
                    cache: &self.outline_cache,
                    kind: SceneKind::Committed,
                },
            );
        }
        let mut next;
        let mut changed;
        if prepared_route {
            let accepted_contours;
            let board_contours: &[BoardContours] =
                if matches!(command.operation, EditOperation::ApplyMechanicalEdit { .. }) {
                    if let Some(board_contours) = &prepared_mechanical_contours {
                        board_contours
                    } else {
                        accepted_contours =
                            geometry::board_contours(&self.document, &self.outline_cache).0;
                        &accepted_contours
                    }
                } else {
                    &[]
                };
            match mirrored_half::prepare_transaction_with_ledger(
                &self.document,
                board_contours,
                &command.operation,
                &self.identity_ledger,
            ) {
                Ok((draft, prepared_changed)) => {
                    next = draft;
                    changed = prepared_changed;
                }
                Err(message) => return self.error(id, &message),
            }
        } else {
            let mut raw_next = self.document.clone();
            // Mechanical edits resolve against the accepted outline projection the user
            // sees, not a stale or re-derived one, so the accepted cache supplies it.
            let accepted_contours;
            let board_contours: &[BoardContours] =
                if matches!(command.operation, EditOperation::ApplyMechanicalEdit { .. }) {
                    accepted_contours =
                        geometry::board_contours(&self.document, &self.outline_cache).0;
                    &accepted_contours
                } else {
                    &[]
                };
            let raw_changed = match apply_with_ledger(
                &mut raw_next,
                board_contours,
                &command.operation,
                &self.identity_ledger,
            ) {
                Ok(changed) => changed,
                Err(message) => return self.error(id, &message),
            };
            if let Some(map) = &raw_next.keymap
                && let Err(message) = keymap::validate(map)
            {
                return self.error(id, &message);
            }
            let mut raw_changed = raw_changed;
            electrical::preserve_handoff(&self.document, &mut raw_next);
            if let Err(message) = layout::resolve(&mut raw_next) {
                return self.error(id, &message);
            }
            match constraints::resolve_edit(&mut raw_next, &command.operation) {
                Ok(ids) => raw_changed.extend(ids),
                Err(message) => return self.error(id, &message),
            }
            match layout::sync_components(&mut raw_next) {
                Ok(ids) => raw_changed.extend(ids),
                Err(message) => return self.error(id, &message),
            }
            match constraints::resolve_edit(&mut raw_next, &command.operation) {
                Ok(ids) => raw_changed.extend(ids),
                Err(message) => return self.error(id, &message),
            }
            match layout::sync_components(&mut raw_next) {
                Ok(ids) => raw_changed.extend(ids),
                Err(message) => return self.error(id, &message),
            }
            outline_controls::detach_removed(&self.document.parts, &mut raw_next);
            raw_changed.extend(prune_removed_components(
                &self.document.parts,
                &mut raw_next,
            ));
            raw_changed.sort();
            raw_changed.dedup();
            // ReplaceDocument is judged by its own arm in `apply`, which allows warnings.
            if !matches!(command.operation, EditOperation::ReplaceDocument { .. })
                && let Err(message) = generators::introduced_error(
                    &generators::parameter_problems(&self.document),
                    &raw_next,
                )
            {
                return self.error(id, &message);
            }
            next = raw_next;
            changed = raw_changed;
        }
        if let Err(message) =
            matrix::placement::validate_translation(&self.document.parts, &next, &command.operation)
        {
            return self.error(id, &message);
        }
        // Ticket 10: every committed edit that changed the document re-runs the
        // adaptive spacing pass. It runs after resolve so the minimum sees the
        // final geometry, and before the no-op check so a genuine edit that
        // only moves the halves still publishes. Skipped when the edit changed
        // nothing, so no-op commits stay no-ops. Same Undo entry: the spacing
        // lands in `next` before the undo snapshot is pushed. Creation owns
        // its own gap (ticket 11); the pass never re-spaces a new pair.
        if command.phase == EditPhase::Commit
            && next != self.document
            && !matches!(
                command.operation,
                EditOperation::CreateMirroredPair { .. } | EditOperation::CreateMirroredHalf { .. }
            )
        {
            let accepted = self.document.clone();
            let mut adapted = match layout::adapt_spacing(&mut next, &accepted) {
                Ok(adapted) => adapted,
                Err(message) => return self.error(id, &message),
            };
            if !adapted.is_empty() {
                if let Err(message) = layout::resolve(&mut next) {
                    return self.error(id, &message);
                }
                changed.append(&mut adapted);
                changed.sort();
                changed.dedup();
            }
        }
        // Prepared edits can change document-level fields that have no entity ID. Use the
        // typed document as the no-op oracle rather than treating an empty changed-ID list as
        // proof that nothing changed. Preserve the legacy changed-ID policy outside this route.
        if command.phase == EditPhase::Commit
            && (((prepared_route
                || matches!(
                    command.operation,
                    EditOperation::SetMatrixPlacement { .. }
                        | EditOperation::SetMatrixSplay { .. }
                        | EditOperation::AddAnchoredMatrix { .. }
                        | EditOperation::SetMatrixAnchor { .. }
                        | EditOperation::DetachMatrixAnchor { .. }
                        | EditOperation::MoveParts { .. }
                ))
                && next == self.document)
                || (!prepared_route
                    && changed.is_empty()
                    && matches!(
                        command.operation,
                        EditOperation::ApplyMechanicalEdit { .. }
                            | EditOperation::ApplyOutlineEdit { .. }
                    )))
        {
            return self.scene(
                id,
                SceneInput {
                    transaction: &command.transaction_id,
                    changed,
                    document: &self.document,
                    contours: &self.contours,
                    geom_findings: &self.findings,
                    cache: &self.outline_cache,
                    kind: SceneKind::Committed,
                },
            );
        }
        let (cache, contours, findings) = if prepared_route || affects_outline(&command.operation) {
            let previous = if matches!(command.operation, EditOperation::MoveParts { .. }) {
                Some(&self.outline_cache)
            } else {
                None
            };
            outlines(&next, previous, &changed)
        } else {
            (
                self.outline_cache.clone(),
                self.contours.clone(),
                self.findings.clone(),
            )
        };
        if command.phase == EditPhase::Preview {
            return self.scene(
                id,
                SceneInput {
                    transaction: &command.transaction_id,
                    changed,
                    document: &next,
                    contours: &contours,
                    geom_findings: &findings,
                    cache: &cache,
                    kind: SceneKind::Preview,
                },
            );
        }
        self.publish_prepared_edit(
            id,
            PreparedEdit {
                transaction: command.transaction_id,
                document: next,
                changed,
                cache,
                contours,
                findings,
            },
        )
    }

    fn publish_prepared_edit(&mut self, id: String, prepared: PreparedEdit) -> CoreReply {
        let PreparedEdit {
            transaction,
            document: mut next,
            changed,
            cache,
            contours,
            findings,
        } = prepared;
        if let Some(message) = validate::duplicate_top_level_identity(&next) {
            return self.error(id, &message);
        }
        let Some(revision) = self.document.revision.checked_add(1) else {
            return self.error(id, "Document revision is exhausted");
        };
        next.revision = revision;
        outline_versions::refresh_recovery(&mut next, &cache);
        // Keep the latest 100 committed edits without retaining an unbounded document history.
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(self.document.clone());
        self.redo.clear();
        self.document = next;
        self.identity_ledger.capture(&self.document);
        self.contours = contours;
        self.findings = findings;
        self.outline_cache = cache;
        self.scene(
            id,
            SceneInput {
                transaction: &transaction,
                changed,
                document: &self.document,
                contours: &self.contours,
                geom_findings: &self.findings,
                cache: &self.outline_cache,
                kind: SceneKind::Committed,
            },
        )
    }

    fn preview_prepared_edit(&self, id: String, command: EditCommand) -> CoreReply {
        let accepted_contours;
        let board_contours: &[BoardContours] =
            if matches!(command.operation, EditOperation::ApplyMechanicalEdit { .. }) {
                accepted_contours = geometry::board_contours(&self.document, &self.outline_cache).0;
                &accepted_contours
            } else {
                &[]
            };
        let (draft, changed) = match mirrored_half::prepare_transaction_with_ledger(
            &self.document,
            board_contours,
            &command.operation,
            &self.identity_ledger,
        ) {
            Ok(prepared) => prepared,
            Err(message) => return self.error(id, &message),
        };
        if let Err(message) = matrix::placement::validate_translation(
            &self.document.parts,
            &draft,
            &command.operation,
        ) {
            return self.error(id, &message);
        }
        let (cache, contours, mut findings) = outlines(&draft, None, &changed);
        findings.extend(preview_electrical_loss_findings(&self.document, &draft));
        self.scene(
            id,
            SceneInput {
                transaction: &command.transaction_id,
                changed,
                document: &draft,
                contours: &contours,
                geom_findings: &findings,
                cache: &cache,
                kind: SceneKind::Preview,
            },
        )
    }

    fn preview_edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        let accepted = generators::parameter_problems(&self.document);
        let backup = PreviewBackup::capture(&self.document, &command.operation);
        let attachments = outline_controls::attached_parts(&self.document);
        let before_parts = self.document.parts.clone();
        let result = apply_with_ledger(
            &mut self.document,
            &[],
            &command.operation,
            &self.identity_ledger,
        )
        .and_then(|mut changed| {
            // Preview must materialize the same dependant poses the commit
            // path publishes: anchored matrices follow from the anchor key's
            // edited pose, and `resolve` (which starts with `validate`)
            // refreshes every dependant exactly as commit does.
            layout::resolve(&mut self.document)?;
            changed.extend(constraints::resolve_edit(
                &mut self.document,
                &command.operation,
            )?);
            changed.extend(layout::sync_components(&mut self.document)?);
            changed.extend(constraints::resolve_edit(
                &mut self.document,
                &command.operation,
            )?);
            changed.extend(layout::sync_components(&mut self.document)?);
            outline_controls::detach_removed(&attachments, &mut self.document);
            changed.extend(prune_removed_components(&before_parts, &mut self.document));
            changed.sort();
            changed.dedup();
            generators::introduced_error(&accepted, &self.document)?;
            matrix::placement::validate_translation(
                &before_parts,
                &self.document,
                &command.operation,
            )?;
            Ok(changed)
        });
        let reply = match result {
            Err(message) => self.error(id, &message),
            Ok(changed) => {
                let previous = matches!(&command.operation, EditOperation::MoveParts { .. })
                    .then_some(&self.outline_cache);
                let (cache, contours, findings) = outlines(&self.document, previous, &changed);
                self.scene(
                    id,
                    SceneInput {
                        transaction: &command.transaction_id,
                        changed,
                        document: &self.document,
                        contours: &contours,
                        geom_findings: &findings,
                        cache: &cache,
                        kind: SceneKind::Preview,
                    },
                )
            }
        };
        backup.restore(&mut self.document);
        reply
    }

    fn history(&mut self, id: String, direction: History) -> CoreReply {
        let Some(revision) = self.document.revision.checked_add(1) else {
            return self.error(id, "Document revision is exhausted");
        };
        let Some(mut next) = match direction {
            History::Undo => self.undo.last(),
            History::Redo => self.redo.last(),
        }
        .cloned() else {
            return self.error(id, "History is empty");
        };
        let changed = changed_ids(&self.document, &next);
        electrical::preserve_handoff(&self.document, &mut next);
        let (from, to) = match direction {
            History::Undo => (&mut self.undo, &mut self.redo),
            History::Redo => (&mut self.redo, &mut self.undo),
        };
        from.pop();
        to.push(self.document.clone());
        next.revision = revision;
        self.document = next;
        self.identity_ledger.capture(&self.document);
        self.recompute();
        let transaction = match direction {
            History::Undo => "undo",
            History::Redo => "redo",
        };
        self.scene(
            id,
            SceneInput {
                transaction,
                changed,
                document: &self.document,
                contours: &self.contours,
                geom_findings: &self.findings,
                cache: &self.outline_cache,
                kind: SceneKind::Committed,
            },
        )
    }

    fn recompute(&mut self) {
        (self.outline_cache, self.contours, self.findings) = outlines(&self.document, None, &[]);
        outline_versions::refresh_recovery(&mut self.document, &self.outline_cache);
    }

    fn scene(&self, id: String, input: SceneInput<'_>) -> CoreReply {
        let SceneInput {
            transaction,
            changed,
            document: doc,
            contours,
            geom_findings,
            cache,
            kind,
        } = input;
        let matrix_order = match matrix::evaluation::order(doc) {
            Ok(order) => order,
            Err(message) => return self.error(id, &message),
        };
        let mut findings = geom_findings.to_vec();
        let definition_ids: BTreeSet<_> =
            doc.definitions.iter().map(|def| def.id.as_str()).collect();
        for part in &doc.parts {
            if !definition_ids.contains(part.definition_id.as_str()) {
                findings.push(Finding {
                    id: format!("part:{}:definition", part.id),
                    severity: Severity::Error,
                    scope: Scope::Layout,
                    message: "Part definition is missing".into(),
                    target_ids: vec![part.id.clone()],
                });
            }
        }
        findings.extend(validate::validate(doc));
        let (board_contours, board_findings, corner_markers) = geometry::board_contours(doc, cache);
        let fitted_targets: BTreeSet<_> = board_findings
            .iter()
            .filter(|finding| finding.id.ends_with(":feature:outline:corners:fitted"))
            .flat_map(|finding| finding.target_ids.iter().cloned())
            .collect();
        findings.retain(|finding| {
            finding.id != "outline:corners:fitted"
                || !finding
                    .target_ids
                    .iter()
                    .all(|id| fitted_targets.contains(id))
        });
        findings.extend(board_findings);
        // Mechanical pair fit findings recomputed from accepted state, so readiness and
        // the selected export gates see them instead of only a transient assembly
        // diagnostic. A Case-scope finding blocks its affected assembly's output; a
        // fabricated-drill violation is Pcb scope and blocks only that board.
        findings.extend(crate::mechanical::inputs::pair_findings(
            doc,
            &board_contours,
        ));
        let board_outline_scenes = geometry::board_outline_scenes(doc, cache);
        let (outline_findings, mut finding_markers) =
            outline_validation::validate(doc, &board_contours, &board_outline_scenes);
        findings.extend(outline_findings);
        finding_markers.extend(inputs::finding_markers(doc, &findings));
        finding_markers.extend(hardware::nominal_fit(doc).1);
        finding_markers.extend(modules::finding_markers(doc, &findings));
        let mut generic_markers: BTreeMap<(String, String), Vec<Contour>> = BTreeMap::new();
        for marker in outline_validation::feature_markers(doc, &findings) {
            generic_markers
                .entry((marker.finding_id, marker.board_id))
                .or_default()
                .extend(marker.contours);
        }
        for ((finding_id, board_id), contours) in generic_markers {
            if !finding_markers
                .iter()
                .any(|marker| marker.finding_id == finding_id && marker.board_id == board_id)
            {
                finding_markers.push(FindingMarker {
                    finding_id,
                    board_id,
                    contours,
                });
            }
        }
        finding_markers.extend(corner_markers);
        finding_markers.extend(keycaps::finding_markers(doc, &findings));
        for board in &board_contours {
            if board.contours.is_empty() {
                findings.push(Finding {
                    id: format!("board:{}:empty-outline", board.board_id),
                    severity: Severity::Error,
                    scope: Scope::Pcb,
                    message: "Board outline resolves to no area".into(),
                    target_ids: vec![board.board_id.clone()],
                });
            }
        }
        // A finding attributed to a linked alternative the board has not selected is
        // visible but never blocks the selected outline or another board (ADR-0001/0002).
        let active =
            |finding: &Finding| !outline_versions::is_inactive_version_finding(doc, &finding.id);
        let layout = !findings
            .iter()
            .filter(|finding| active(finding))
            .any(|f| matches!(f.scope, Scope::Layout) && matches!(f.severity, Severity::Error));
        let outline = !contours.is_empty()
            && !findings.iter().filter(|finding| active(finding)).any(|f| {
                matches!(f.scope, Scope::Outline) && matches!(f.severity, Severity::Error)
            });
        let pcb = layout
            && !doc.boards.is_empty()
            && !findings
                .iter()
                .filter(|finding| active(finding))
                .any(|f| matches!(f.scope, Scope::Pcb) && matches!(f.severity, Severity::Error));
        let board_readiness: Vec<BoardReadiness> = board_contours
            .iter()
            .map(|board| {
                let owner = doc
                    .boards
                    .iter()
                    .find(|item| item.id == board.board_id)
                    .unwrap();
                let relevant = |finding: &Finding| {
                    finding.target_ids.is_empty()
                        || finding.target_ids.iter().any(|id| {
                            id == &board.board_id
                                || owner.outline_ids.contains(id)
                                || owner.part_ids.contains(id)
                                || owner.net_ids.contains(id)
                        })
                };
                let invalid_pcb =
                    findings
                        .iter()
                        .filter(|finding| active(finding))
                        .any(|finding| {
                            matches!(finding.severity, Severity::Error)
                                && matches!(finding.scope, Scope::Pcb)
                                && relevant(finding)
                        });
                let invalid_outline =
                    findings
                        .iter()
                        .filter(|finding| active(finding))
                        .any(|finding| {
                            matches!(finding.severity, Severity::Error)
                                && matches!(finding.scope, Scope::Pcb)
                                && finding
                                    .id
                                    .starts_with(&format!("board:{}:", board.board_id))
                                && (finding.id.contains(":outline")
                                    || finding.id.contains(":feature:")
                                    || finding.id.ends_with(":empty-outline"))
                        });
                let outline = !board.contours.is_empty() && !invalid_outline;
                let valid_layout =
                    !findings
                        .iter()
                        .filter(|finding| active(finding))
                        .any(|finding| {
                            finding.severity == Severity::Error
                                && finding.scope == Scope::Layout
                                && relevant(finding)
                        });
                let pcb = valid_layout && outline && !invalid_pcb;
                let bodies: Vec<_> = doc
                    .case_bodies
                    .iter()
                    .filter(|body| body.board_id == board.board_id)
                    .collect();
                let invalid_case = findings.iter().any(|finding| {
                    matches!(finding.severity, Severity::Error)
                        && matches!(finding.scope, Scope::Case)
                        && (finding.target_ids.is_empty()
                            || finding.target_ids.contains(&board.board_id)
                            || bodies
                                .iter()
                                .any(|body| finding.target_ids.contains(&body.id)))
                });
                BoardReadiness {
                    board_id: board.board_id.clone(),
                    outline,
                    pcb,
                    case_ready: pcb && !bodies.is_empty() && !invalid_case,
                }
            })
            .collect();
        let case_ready = !doc.case_bodies.is_empty()
            && doc.case_bodies.iter().all(|body| {
                board_readiness
                    .iter()
                    .any(|board: &BoardReadiness| board.board_id == body.board_id && board.pcb)
            })
            && !findings
                .iter()
                .any(|f| matches!(f.scope, Scope::Case) && matches!(f.severity, Severity::Error));
        let scene = SceneDelta {
            module_scenes: doc
                .boards
                .iter()
                .flat_map(|b| modules::resolve(doc, &b.id).modules)
                .collect(),
            revision: doc.revision,
            transaction_id: transaction.into(),
            changed_ids: changed,
            transforms: doc
                .parts
                .iter()
                .map(|part| Transform {
                    id: part.id.clone(),
                    // Ticket 12: the canvas draws half-frame parts at their
                    // evaluated board poses; storage stays within-half.
                    pose: layout::evaluated_part_pose(doc, part),
                })
                .collect(),
            matrix_scenes: matrix_order
                .iter()
                .map(|index| &doc.matrices[*index])
                .filter_map(|matrix| {
                    if matrix::valid_matrix(matrix, doc).is_err() {
                        return None;
                    }
                    matrix::project_matrix(matrix, doc, matrix::ProjectionMode::Actual)
                        .map(|mut scene| {
                            scene.companions = matrix::scene_companions(matrix, doc);
                            scene
                        })
                        .ok()
                })
                .collect(),
            contours: contours.to_vec(),
            board_contours,
            board_readiness,
            board_outline_scenes,
            finding_markers,
            findings,
            readiness: Readiness {
                layout,
                outline,
                pcb,
                case_ready,
            },
        };
        match kind {
            SceneKind::Preview => CoreReply::Preview { id, scene },
            SceneKind::Committed => CoreReply::Scene {
                id,
                scene,
                document: Box::new(doc.clone()),
            },
        }
    }

    fn error(&self, id: String, message: &str) -> CoreReply {
        CoreReply::Error {
            id,
            message: message.into(),
            revision: self.document.revision,
        }
    }
}

enum History {
    Undo,
    Redo,
}

struct PreparedEdit {
    transaction: String,
    document: ProjectDoc,
    changed: Vec<String>,
    cache: OutlineCache,
    contours: Vec<Contour>,
    findings: Vec<Finding>,
}

enum SceneKind {
    Preview,
    Committed,
}

struct SceneInput<'a> {
    transaction: &'a str,
    changed: Vec<String>,
    document: &'a ProjectDoc,
    contours: &'a [Contour],
    geom_findings: &'a [Finding],
    cache: &'a OutlineCache,
    kind: SceneKind,
}

type SavedPartPose = (
    usize,
    Pose2,
    Option<std::collections::BTreeMap<String, serde_json::Value>>,
);

enum PreviewBackup {
    Parts(Vec<SavedPartPose>),
    Matrix {
        definition_len: usize,
        parts: Vec<Part>,
        matrices: Vec<Matrix>,
        layouts: Vec<Layout>,
        nets: Vec<Net>,
        outline: Vec<OutlineFeature>,
        boards: Vec<Board>,
        constraints: Vec<Constraint>,
    },
}

impl PreviewBackup {
    // Save only fields a preview can change so pointer moves avoid cloning the document.
    fn capture(doc: &ProjectDoc, operation: &EditOperation) -> Self {
        if let EditOperation::MoveParts { positions } = operation
            && !layout::has_linked_positions(doc, positions)
        {
            let mut affected: BTreeSet<String> = positions
                .iter()
                .map(|position| position.id.clone())
                .collect();
            loop {
                let prior = affected.len();
                for constraint in &doc.constraints {
                    if affected.contains(constraint.source()) {
                        affected.insert(constraint.target().into());
                    }
                }
                if affected.len() == prior {
                    break;
                }
            }
            // Settling a constrained key can rewrite its matrix and regenerate members.
            let reaches_matrix_key = affected
                .iter()
                .any(|id| id.starts_with("matrix/") && layout::is_matrix_key(doc, id));
            if !reaches_matrix_key {
                return Self::Parts(
                    doc.parts
                        .iter()
                        .enumerate()
                        .filter(|(_, part)| affected.contains(&part.id))
                        .map(|(index, part)| (index, part.pose, part.properties.clone()))
                        .collect(),
                );
            }
        }
        Self::Matrix {
            definition_len: doc.definitions.len(),
            parts: doc.parts.clone(),
            matrices: doc.matrices.clone(),
            layouts: doc.layouts.clone(),
            nets: doc.nets.clone(),
            outline: doc.outline.clone(),
            boards: doc.boards.clone(),
            constraints: doc.constraints.clone(),
        }
    }

    fn restore(self, doc: &mut ProjectDoc) {
        match self {
            Self::Parts(parts) => {
                for (index, pose, properties) in parts {
                    doc.parts[index].pose = pose;
                    doc.parts[index].properties = properties;
                }
            }
            Self::Matrix {
                definition_len,
                parts,
                matrices,
                layouts,
                nets,
                outline,
                boards,
                constraints,
            } => {
                doc.definitions.truncate(definition_len);
                doc.parts = parts;
                doc.matrices = matrices;
                doc.layouts = layouts;
                doc.nets = nets;
                doc.outline = outline;
                doc.boards = boards;
                doc.constraints = constraints;
            }
        }
    }
}

#[cfg(test)]
mod preview_backup_tests {
    use super::*;

    fn part(id: &str) -> Part {
        Part {
            anchor: None,
            keycap: None,
            outline: None,
            id: id.into(),
            definition_id: "switch".into(),
            reference: id.into(),
            pose: Pose2 {
                at: Vec2::default(),
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        }
    }

    fn matrix() -> Matrix {
        Matrix {
            id: "main".into(),
            name: None,
            rows: 1,
            columns: 1,
            pitch: Vec2 { x: 19.0, y: 19.0 },
            origin: Vec2::default(),
            definition_id: "switch".into(),
            part_ids: vec!["matrix/main/r0c0".into()],
            board_id: None,
            mirror: None,
            rotation: None,
            edge_gap: None,
            diode_direction: None,
            row_layout: vec![],
            column_layout: vec![],
            anchor: None,
            cells: vec![MatrixCell {
                shift: None,
                bend: None,
                deleted: false,
                row: 0,
                column: 0,
                enabled: true,
                definition_id: None,
                variant: None,
                offset: None,
                rotation: None,
                assemblies: vec![],
                assemblies_local: None,
            }],
        }
    }

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("preview-backup", "Preview backup");
        document.parts = vec![part("anchor"), part("matrix/main/r0c0")];
        document.matrices.push(matrix());
        document.layouts.push(Layout {
            id: "layout:main".into(),
            name: "Main".into(),
            board_id: "board:main".into(),
            matrix_ids: vec!["main".into()],
            part_ids: vec!["layout-component".into()],
            mirror_link: None,
        });
        document.constraints.push(Constraint::Offset {
            id: "key-anchor".into(),
            source_part_id: "anchor".into(),
            target_part_id: "matrix/main/r0c0".into(),
            offset: Vec2::default(),
            rotation: 0.0,
        });
        document
    }

    #[test]
    fn matrix_reaching_preview_restores_layout_membership() {
        let mut document = document();
        let accepted_layouts = document.layouts.clone();
        let backup = PreviewBackup::capture(
            &document,
            &EditOperation::MoveParts {
                positions: vec![Position {
                    id: "anchor".into(),
                    at: Vec2 { x: 3.0, y: 4.0 },
                }],
            },
        );
        assert!(matches!(&backup, PreviewBackup::Matrix { .. }));

        document.layouts[0]
            .part_ids
            .push("adopted-during-preview".into());
        backup.restore(&mut document);

        assert_eq!(document.layouts, accepted_layouts);
    }

    #[test]
    fn unrelated_part_move_keeps_the_parts_only_backup() {
        let mut document = document();
        document.constraints.clear();
        let backup = PreviewBackup::capture(
            &document,
            &EditOperation::MoveParts {
                positions: vec![Position {
                    id: "anchor".into(),
                    at: Vec2 { x: 3.0, y: 4.0 },
                }],
            },
        );

        assert!(matches!(&backup, PreviewBackup::Parts(_)));
    }
}

fn uses_prepared_transaction(document: &ProjectDoc, operation: &EditOperation) -> bool {
    if matches!(
        operation,
        EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::SetCompanionPlacement { .. }
            | EditOperation::SetHalfException { .. }
            | EditOperation::UnlinkHalves { .. }
    ) {
        return true;
    }
    document.hardware.as_ref().is_some_and(|hardware| {
        hardware.pairs.iter().any(|pair| {
            pair.halves
                .as_ref()
                .is_some_and(|halves| halves.source_board_id != halves.mirror_board_id)
        })
    })
}

fn affects_outline(op: &EditOperation) -> bool {
    if let EditOperation::ApplyMechanicalEdit { edit } = op {
        return crate::mechanical::edit::affects_outline(&edit.change);
    }
    if let EditOperation::ApplyOutlineEdit { edit } = op {
        return crate::outline_edit::affects_outline(&edit.change);
    }
    matches!(
        op,
        EditOperation::SetMountedModule {
            host_connector_definition: Some(_),
            ..
        } | EditOperation::MoveParts { .. }
            | EditOperation::SetKeycapSize { .. }
            | EditOperation::AddPart { .. }
            | EditOperation::AddMirrorOnlyPart { .. }
            | EditOperation::RemoveParts { .. }
            | EditOperation::RemoveMatrix { .. }
            | EditOperation::SetMatrixCellsEnabled { .. }
            | EditOperation::RemoveSelectedMatrixCells { .. }
            | EditOperation::RemoveMatrixCells { .. }
            | EditOperation::RemoveMatrixRow { .. }
            | EditOperation::RemoveMatrixColumn { .. }
            | EditOperation::SetMatrix { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::SetMatrixPlacement { .. }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::CreateMirroredHalf { .. }
            | EditOperation::SetLayout { .. }
            | EditOperation::SetSplitGap { .. }
            | EditOperation::SetMirroredPart { .. }
            | EditOperation::SetPartAnchor { .. }
            | EditOperation::SetPartAnchorFollow { .. }
            | EditOperation::DetachPartAnchor { .. }
            | EditOperation::SetPartAnchorValues { .. }
            | EditOperation::SetPartRotation { .. }
            | EditOperation::SetConstraint { .. }
            | EditOperation::RemoveConstraint { .. }
            | EditOperation::ReplaceDocument { .. }
    )
}

/// Linked outline versions drop the authored choices that required a component the edit
/// removed, whichever operation removed it, inside that same accepted edit and Undo step.
fn prune_removed_components(before: &[Part], after: &mut ProjectDoc) -> Vec<String> {
    let present: BTreeSet<&str> = after.parts.iter().map(|part| part.id.as_str()).collect();
    let removed: Vec<String> = before
        .iter()
        .filter(|part| !present.contains(part.id.as_str()))
        .map(|part| part.id.clone())
        .collect();
    outline_versions::prune_removed_sources(after, &removed)
}

fn add_part_placement(
    document: &mut ProjectDoc,
    part: &Part,
    board_id: Option<&str>,
    definition: Option<&PartDefinition>,
    layout_id: Option<&str>,
) -> Result<Vec<String>, String> {
    if document.parts.iter().any(|existing| existing.id == part.id) {
        return Err(format!("Duplicate part {}", part.id));
    }
    if let Some(definition) = definition {
        if definition.id != part.definition_id {
            return Err(format!(
                "Definition {:?} does not match part definition {:?}",
                definition.id, part.definition_id
            ));
        }
        let mut existing = document
            .definitions
            .iter()
            .filter(|item| item.id == definition.id);
        match (existing.next(), existing.next()) {
            (Some(current), None) if current == definition => {}
            (None, None) => document.definitions.push(definition.clone()),
            _ => {
                return Err(format!(
                    "Definition {:?} already exists with different or ambiguous contents",
                    definition.id
                ));
            }
        }
    }
    if !geometry::part_valid(part, document) {
        return Err(format!("Unknown definition {}", part.definition_id));
    }

    let board_id = board_id
        .map(str::to_owned)
        .or_else(|| document.boards.first().map(|board| board.id.clone()));
    let board_index = board_id
        .as_deref()
        .map(|id| {
            let mut boards = document
                .boards
                .iter()
                .enumerate()
                .filter(|(_, board)| board.id == id);
            let (index, _) = boards.next().ok_or_else(|| format!("Unknown board {id}"))?;
            if boards.next().is_some() {
                return Err(format!("Board {id:?} is ambiguous"));
            }
            Ok(index)
        })
        .transpose()?;
    let board_outline = board_index
        .and_then(|index| document.boards.get(index))
        .map(|board| board.outline_ids.clone())
        .unwrap_or_default();
    if let Some(layout_id) = layout_id {
        let mut layouts = document
            .layouts
            .iter()
            .filter(|layout| layout.id == layout_id);
        let layout = layouts
            .next()
            .ok_or_else(|| format!("Unknown layout {layout_id}"))?;
        if layouts.next().is_some() {
            return Err(format!("Layout {layout_id:?} is ambiguous"));
        }
        if Some(layout.board_id.as_str()) != board_id.as_deref() {
            return Err(format!(
                "Layout {layout_id:?} does not belong to selected board"
            ));
        }
        if layout.part_ids.contains(&part.id) {
            return Err(format!(
                "Layout {layout_id:?} already contains part {:?}",
                part.id
            ));
        }
        if layout.matrix_ids.is_empty()
            || layout.matrix_ids.iter().any(|id| {
                !document.matrices.iter().any(|matrix| {
                    &matrix.id == id && matrix.board_id.as_ref() == Some(&layout.board_id)
                })
            })
        {
            return Err(format!(
                "Layout {layout_id:?} has inconsistent matrix ownership"
            ));
        }
    }

    if let Some(index) = board_index {
        document.boards[index].part_ids.push(part.id.clone());
    }
    let mut attached = false;
    for feature in &mut document.outline {
        if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature
            && board_outline.contains(id)
        {
            part_ids.push(part.id.clone());
            attached = true;
        }
    }
    if !attached
        && document.boards.is_empty()
        && let Some(OutlineFeature::PartEnvelope { part_ids, .. }) = document
            .outline
            .iter_mut()
            .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
    {
        part_ids.push(part.id.clone());
    }
    if let Some(layout_id) = layout_id {
        document
            .layouts
            .iter_mut()
            .find(|layout| layout.id == layout_id)
            .ok_or_else(|| format!("Unknown layout {layout_id}"))?
            .part_ids
            .push(part.id.clone());
    }
    // Ticket 12: drops onto a stored-gap half arrive board-space; storage is
    // half-frame, so convert at insertion. Rotations are frame-free.
    let mut stored = part.clone();
    // Ticket 13: an independent drop without an anchor takes the nearest key
    // of its half, keeping the drop point upright. Members, companions and
    // driven parts stay unanchored; a preset anchor is validated on use.
    if stored.anchor.is_none()
        && let Some(layout_id) = layout_id
        && !matrix::layout::is_member_or_companion(document, &part.id)
        && document
            .constraint_driving_part(&part.id)
            .ok()
            .flatten()
            .is_none()
    {
        stored.anchor = matrix::part_anchors::drop_anchor(document, layout_id, part.pose);
    }
    // A preset anchor is validated like a Pick: a stale key refuses instead
    // of silently falling back to the stored pose.
    if let Some(anchor) = stored.anchor.as_ref() {
        matrix::part_anchors::validate_target(document, &stored, anchor)?;
    }
    if let Some(layout_id) = layout_id
        && let Some(offset) = layout::layout_half_offset(document, layout_id)
        && !layout::is_member_or_companion(document, &part.id)
    {
        stored.pose.at.x -= offset.x;
        stored.pose.at.y -= offset.y;
    }
    document.parts.push(stored);
    Ok(vec![part.id.clone()])
}

fn apply(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
) -> Result<Vec<String>, String> {
    apply_with_ledger(
        doc,
        board_contours,
        op,
        &mechanical::edit::MechanicalIdentityLedger::default(),
    )
}

fn apply_with_ledger(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
    identities: &mechanical::edit::MechanicalIdentityLedger,
) -> Result<Vec<String>, String> {
    let mut changed = match op {
        EditOperation::SetModuleDefinition { definition } => {
            modules::set_definition(doc, definition)
        }
        EditOperation::SetMountedModule {
            instance,
            definition,
            host_connector_definition,
        } => modules::set(
            doc,
            instance,
            definition.as_deref(),
            host_connector_definition.as_deref(),
        ),
        EditOperation::RemoveMountedModule { id } => modules::remove(doc, id),
        EditOperation::EmbedModuleCircuit {
            id,
            definition,
            host_board_id,
            pose,
            side,
            joins,
        } => modules::embed(
            doc,
            id,
            definition,
            host_board_id,
            *pose,
            side.clone(),
            joins,
        ),
        EditOperation::RemoveEmbeddedCircuit { id } => modules::remove_circuit(doc, id),
        EditOperation::SetInputScanMode { part_id, mode } => {
            inputs::set_scan_mode(doc, part_id, *mode)
        }
        EditOperation::SetWiringMode { board_id, mode } => {
            if !doc.boards.iter().any(|board| board.id == *board_id) {
                return Err(format!("Unknown board {board_id}"));
            }
            if doc
                .hardware
                .as_ref()
                .and_then(|hardware| {
                    hardware
                        .boards
                        .iter()
                        .find(|configuration| configuration.board_id == *board_id)
                })
                .map_or(electrical::ElectricalMode::Matrix, |configuration| {
                    configuration.mode
                })
                == *mode
            {
                return Ok(Vec::new());
            }
            let hardware = doc.hardware.get_or_insert_with(Default::default);
            if let Some(configuration) = hardware
                .boards
                .iter_mut()
                .find(|configuration| configuration.board_id == *board_id)
            {
                configuration.mode = *mode;
            } else {
                hardware.boards.push(ElectricalBoardConfiguration {
                    board_id: board_id.clone(),
                    mode: *mode,
                    ..Default::default()
                });
            }
            Ok(vec![board_id.clone()])
        }
        EditOperation::EditKeymap { change } => keymap::apply_edit(doc, change),
        EditOperation::SetKeyBinding { .. }
        | EditOperation::SetKeycapBoard { .. }
        | EditOperation::SetMatrixKeycaps { .. }
        | EditOperation::SetKeycapKey { .. } => keycaps::apply_edit(doc, op),
        EditOperation::SetKeycapSize {
            sizes,
            moves,
            offsets,
        } => {
            // Validate every entry before writing anything: one atomic typed
            // entry or nothing.
            for (id, size) in sizes {
                doc.parts
                    .iter()
                    .find(|part| &part.id == id)
                    .ok_or_else(|| format!("Unknown key {id}"))?;
                if !size.x.is_finite() || !size.y.is_finite() || size.x <= 0.0 || size.y <= 0.0 {
                    return Err(format!("Keycap size for {id} must be positive and finite"));
                }
            }
            for position in moves {
                doc.parts
                    .iter()
                    .find(|part| part.id == position.id)
                    .ok_or_else(|| format!("Unknown key {}", position.id))?;
                if !position.at.x.is_finite() || !position.at.y.is_finite() {
                    return Err(format!("Keycap pose for {} must be finite", position.id));
                }
            }
            for cell in offsets {
                doc.matrices
                    .iter()
                    .find(|matrix| matrix.id == cell.matrix_id)
                    .ok_or_else(|| format!("Unknown matrix {}", cell.matrix_id))?;
                if let Some(offset) = &cell.offset
                    && (!offset.x.is_finite() || !offset.y.is_finite())
                {
                    return Err(format!(
                        "Keycap cell offset for {} is not finite",
                        cell.matrix_id
                    ));
                }
            }
            let mut changed = BTreeSet::new();
            for (id, size) in sizes {
                let part = doc
                    .parts
                    .iter_mut()
                    .find(|part| &part.id == id)
                    .expect("validated");
                part.keycap = Some(*size);
                changed.insert(id.clone());
            }
            for position in moves {
                let part = doc
                    .parts
                    .iter_mut()
                    .find(|part| part.id == position.id)
                    .expect("validated");
                part.pose.at = position.at;
                changed.insert(position.id.clone());
            }
            for cell in offsets {
                let matrix = doc
                    .matrices
                    .iter_mut()
                    .find(|matrix| matrix.id == cell.matrix_id)
                    .expect("validated");
                if let Some(existing) = matrix
                    .cells
                    .iter_mut()
                    .find(|candidate| candidate.row == cell.row && candidate.column == cell.column)
                {
                    existing.offset = cell.offset;
                } else {
                    matrix.cells.push(MatrixCell {
                        shift: None,
                        bend: None,
                        deleted: false,
                        row: cell.row,
                        column: cell.column,
                        enabled: true,
                        definition_id: None,
                        variant: None,
                        offset: cell.offset,
                        rotation: None,
                        assemblies: vec![],
                        assemblies_local: None,
                    });
                }
                changed.insert(cell.matrix_id.clone());
            }
            Ok(changed.into_iter().collect())
        }
        EditOperation::MoveParts { positions } => {
            let (mut changed, handled) = layout::move_keys(doc, positions)?;
            for position in positions {
                if handled.contains(&position.id) {
                    continue;
                }
                let Some(index) = doc.parts.iter().position(|part| part.id == position.id) else {
                    return Err(format!("Unknown part {}", position.id));
                };
                if doc.parts[index].locked == Some(true) {
                    return Err(format!("Part {} is locked", position.id));
                }
                if !position.at.x.is_finite() || !position.at.y.is_finite() {
                    return Err("Position must be finite".into());
                }
                // Ticket 12: pair parts cannot cross the split axis; the drag
                // stops at it. The clamp is the identity off a mirrored half.
                // Drags arrive board-space while storage is half-frame, so the
                // retune delta measures against the evaluated pose.
                let at = layout::clamp_part_to_half(doc, &position.id, position.at);
                // Ticket 13: an anchored part re-solves Spread/Stagger from
                // the clamped drop instead of writing the pose.
                if doc.parts[index].anchor.is_some() {
                    changed.extend(matrix::part_anchors::move_anchored(doc, &position.id, at)?);
                    continue;
                }
                let current = layout::evaluated_part_pose(doc, &doc.parts[index]);
                let delta = Vec2 {
                    x: at.x - current.at.x,
                    y: at.y - current.at.y,
                };
                if let Some(constraint) = doc
                    .constraints
                    .iter_mut()
                    .find(|constraint| constraint.target() == position.id)
                {
                    if constraints::retune_offset(&doc.parts, constraint, delta, None)?
                        == constraints::Retune::Mirror
                    {
                        return Err(constraints::mirror_drag_refusal(&position.id));
                    }
                    changed.push(position.id.clone());
                    continue;
                }
                // Ticket 12: a correspondence-mirrored part on a stored-gap
                // pair writes both copies through, so either copy edits both.
                if let Some(mirrored) = layout::move_pair_part(doc, &position.id, at)? {
                    changed.extend(mirrored);
                    continue;
                }
                if let Some(linked) = layout::move_linked_component(doc, &position.id, at)? {
                    changed.extend(linked);
                    continue;
                }
                // Ticket 12: the drop commits board-space; storage converts.
                let stored = layout::stored_part_pose(
                    doc,
                    &position.id,
                    Pose2 {
                        at,
                        rotation: doc.parts[index].pose.rotation,
                    },
                );
                let part = &mut doc.parts[index];
                part.pose.at = stored.at;
                if doc
                    .matrices
                    .iter()
                    .any(|matrix| matrix.part_ids.contains(&part.id))
                {
                    matrix::mark_override(part);
                }
                changed.push(part.id.clone());
            }
            Ok(changed)
        }
        EditOperation::AddPart { .. } | EditOperation::AddMirrorOnlyPart { .. } => match op {
            EditOperation::AddPart {
                part,
                board_id,
                definition,
                layout_id,
            } => {
                let mut changed = add_part_placement(
                    doc,
                    part,
                    board_id.as_deref(),
                    definition.as_ref(),
                    layout_id.as_deref(),
                )?;
                // Ticket 12: a part added to a stored-gap half mirrors by
                // correspondence in the same Undo entry; legacy pairs mirror
                // through the resolve sync. Mirror-only adds stay single.
                changed.extend(layout::mirror_new_pair_part(doc, &part.id)?);
                Ok(changed)
            }
            EditOperation::AddMirrorOnlyPart {
                part,
                board_id,
                definition,
                layout_id,
            } => add_part_placement(
                doc,
                part,
                Some(board_id),
                definition.as_ref(),
                layout_id.as_deref(),
            ),
            _ => unreachable!("the placement arm matches both Part addition operations"),
        },
        EditOperation::SetMirroredPart { part_id, mirrored } => {
            layout::set_part_mirrored(doc, part_id, *mirrored)
        }
        EditOperation::SetPartAnchor { part_id, anchor } => {
            matrix::part_anchors::set_anchor(doc, part_id, anchor)
        }
        EditOperation::SetPartAnchorFollow { part_id, follow } => {
            matrix::part_anchors::set_follow(doc, part_id, follow)
        }
        EditOperation::DetachPartAnchor { part_id } => matrix::part_anchors::detach(doc, part_id),
        EditOperation::SetPartAnchorValues { part_id, change } => {
            matrix::part_anchors::set_value(doc, part_id, change)
        }
        EditOperation::SetPartRotation { part_id, rotation } => {
            layout::set_detached_part_rotation(doc, part_id, *rotation)
        }
        EditOperation::RemoveMatrix { id } => {
            let ids = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == *id)
                .ok_or("Matrix does not exist")?
                .part_ids
                .clone();
            // Dependants keep their current world frame instead of dangling.
            let mut detached = matrix::anchors::detach_dependants(doc, id)?;
            // Ticket 21: linked partners follow the deletion or survive it
            // complete; only defunct pair records retire.
            let follow = layout::plan_linked_matrix_removal(doc, id, &mut detached)?;
            for gone in std::iter::once(id.clone())
                .chain(follow.matrices.clone())
                .chain(follow.retired.clone())
            {
                layout::remove_matrix(doc, &gone);
            }
            let mut ids = ids;
            ids.extend(follow.part_ids);
            let mut changed = apply(doc, &[], &EditOperation::RemoveParts { ids })?;
            changed.append(&mut detached);
            doc.matrices.retain(|matrix| {
                matrix.id != *id
                    && !follow.matrices.contains(&matrix.id)
                    && !follow.retired.contains(&matrix.id)
            });
            let mut empty_owned: std::collections::BTreeSet<_> = std::collections::BTreeSet::new();
            for gone in std::iter::once(id.clone()).chain(follow.matrices.clone()) {
                let prefix = format!("matrix/{gone}/net/");
                empty_owned.extend(
                    doc.nets
                        .iter()
                        .filter(|net| net.id.starts_with(&prefix) && net.pins.is_empty())
                        .map(|net| net.id.clone()),
                );
            }
            doc.nets.retain(|net| !empty_owned.contains(&net.id));
            for board in &mut doc.boards {
                board.net_ids.retain(|id| !empty_owned.contains(id));
            }
            changed.push(id.clone());
            changed.extend(follow.matrices);
            changed.extend(follow.retired);
            Ok(changed)
        }
        EditOperation::SetMatrixCellsEnabled { cells, enabled } => {
            matrix::deletion::edit_selected_cells(doc, cells, Some(*enabled))
        }
        EditOperation::RemoveSelectedMatrixCells { cells } => {
            matrix::deletion::edit_selected_cells(doc, cells, None)
        }
        EditOperation::RemoveMatrixCells { matrix_id, cells } => {
            matrix::deletion::remove_cells(doc, matrix_id, cells)
        }
        EditOperation::RemoveMatrixRow { matrix_id, row } => {
            matrix::deletion::remove_axis(doc, matrix_id, *row, true)
        }
        EditOperation::RemoveMatrixColumn { matrix_id, column } => {
            matrix::deletion::remove_axis(doc, matrix_id, *column, false)
        }
        EditOperation::RemoveParts { ids } => {
            let affected: Vec<_> = doc
                .matrices
                .iter()
                .filter(|matrix| matrix.part_ids.iter().any(|id| ids.contains(id)))
                .map(|matrix| matrix.id.clone())
                .collect();
            let ids = layout::mirrored_component_ids(doc, ids);
            let ids = matrix::removed_members(doc, &ids);
            doc.constraints.retain(|constraint| {
                !ids.iter()
                    .any(|id| id == constraint.source() || id == constraint.target())
            });
            doc.parts.retain(|part| !ids.contains(&part.id));
            for board in &mut doc.boards {
                board.part_ids.retain(|id| !ids.contains(id));
            }
            for net in &mut doc.nets {
                net.pins.retain(|pin| !ids.contains(&pin.part_id));
            }
            for feature in &mut doc.outline {
                if let OutlineFeature::PartEnvelope { part_ids, .. } = feature {
                    part_ids.retain(|id| !ids.contains(id));
                }
            }
            for matrix in &mut doc.matrices {
                matrix.part_ids.retain(|id| !ids.contains(id));
            }
            for layout in &mut doc.layouts {
                layout.part_ids.retain(|id| !ids.contains(id));
            }
            // A linked refinement drops exactly the authored choices that required a
            // removed component, inside this same accepted edit, so the cleanup and the
            // removal share one Undo step and never retarget onto another component.
            // Ticket 12: removing either copy of a mirrored part drops its Part
            // correspondence, so no dangling record survives the removal.
            layout::retire_part_correspondences(doc, &ids);
            let mut changed = ids;
            let cleanup = outline_versions::prune_removed_sources(doc, &changed);
            changed.extend(cleanup);
            for matrix_id in affected {
                changed.extend(layout::sync(doc, &matrix_id)?);
            }
            Ok(changed)
        }
        EditOperation::SetCase { body } => {
            if let Some(current) = doc.case_bodies.iter_mut().find(|item| item.id == body.id) {
                *current = body.clone();
            } else {
                doc.case_bodies.push(body.clone());
            }
            Ok(vec![body.id.clone()])
        }
        EditOperation::SetMatrix {
            matrix,
            definitions,
        } => matrix::placement::carrying_authored_companions(
            doc,
            matrix::placement::Reported::PreserveEditReport,
            |doc| {
                let mut changed = vec![];
                let mut provided = BTreeSet::new();
                for definition in definitions.iter().flatten() {
                    if definition.id.trim().is_empty() || !provided.insert(&definition.id) {
                        return Err("Matrix definition ID is invalid or duplicated".into());
                    }
                    if let Some(existing) =
                        doc.definitions.iter().find(|item| item.id == definition.id)
                    {
                        if existing != definition {
                            return Err(format!("Conflicting definition {}", definition.id));
                        }
                        continue;
                    }
                    doc.definitions.push(definition.clone());
                    changed.push(definition.id.clone());
                }
                // Ticket 21: an absent matrix has no independent storage
                // state; refuse before any detach side effect.
                layout::refuse_omitted_storage_edit(doc, &matrix.id, &[])?;
                if let Some(previous) = doc
                    .matrices
                    .iter()
                    .find(|item| item.id == matrix.id)
                    .cloned()
                {
                    // Shrinking never retargets an anchor onto a new key at an old
                    // coordinate: dependants of removed slots detach with their frame.
                    changed.extend(matrix::anchors::detach_out_of_range(
                        doc,
                        &matrix.id,
                        matrix.rows,
                        matrix.columns,
                    )?);
                    matrix::anchors::check_anchored_frame_change(doc, &previous, matrix)?;
                }
                changed.extend(matrix::set_matrix(doc, matrix)?);
                changed.extend(layout::sync(doc, &matrix.id)?);
                layout::sync_member_pairs(doc, &matrix.id)?;
                Ok(changed)
            },
        ),
        EditOperation::SetMatrixSplay {
            matrix_id,
            column,
            change,
        } => {
            // Ticket 21: an absent matrix has no independent storage state.
            layout::refuse_omitted_storage_edit(doc, matrix_id, &[])?;
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .ok_or("Matrix not found")?;
            let matrix = matrix::splay::update(current, *column, change, doc)?;
            if matrix == *current {
                return Ok(Vec::new());
            }
            // A splay reports every member it touched, not only those that moved.
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Touched,
                |doc| {
                    let mut changed = matrix::set_matrix(doc, &matrix)?;
                    changed.extend(layout::sync(doc, matrix_id)?);
                    Ok(changed)
                },
            )
        }
        EditOperation::SetMatrixPlacement { matrix_id, change } => {
            // Ticket 21: an absent matrix has no independent storage state.
            layout::refuse_omitted_storage_edit(doc, matrix_id, &[])?;
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .cloned()
                .ok_or("Matrix not found")?;
            let matrix = matrix::placement::update(&current, change, doc)?;
            // Setting the current value is a valid no-op.
            if matrix == current {
                return Ok(Vec::new());
            }
            // Report only members whose projection actually moved.
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| {
                    let mut changed = matrix::set_matrix(doc, &matrix)?;
                    changed.extend(layout::sync(doc, matrix_id)?);
                    Ok(changed)
                },
            )
        }
        EditOperation::AddAnchoredMatrix { id, anchor } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::add(doc, id, anchor),
            )
        }
        EditOperation::SetMatrixAnchor { matrix_id, anchor } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::set_anchor(doc, matrix_id, anchor),
            )
        }
        EditOperation::DetachMatrixAnchor { matrix_id } => {
            matrix::placement::carrying_authored_companions(
                doc,
                matrix::placement::Reported::Moved,
                |doc| matrix::anchors::detach(doc, matrix_id),
            )
        }
        EditOperation::DetachMatrixAnchorConfirmed {
            matrix_id,
            local_matrix_id,
        } => matrix::placement::carrying_authored_companions(
            doc,
            matrix::placement::Reported::Moved,
            |doc| matrix::anchors::detach_confirmed(doc, matrix_id, local_matrix_id),
        ),
        EditOperation::CreateMirroredPair {
            left,
            right,
            matrix,
            definitions,
        } => {
            let link = right
                .mirror_link
                .as_ref()
                .ok_or("A new mirrored pair must be linked")?;
            if left.mirror_link.is_some()
                || link.source_id != left.id
                || left.id == right.id
                || left.matrix_ids != [matrix.id.clone()]
                || right.matrix_ids.len() != 1
                || left.matrix_ids == right.matrix_ids
                || matrix.board_id.as_ref() != Some(&left.board_id)
                || left.board_id != right.board_id
                || !left.part_ids.is_empty()
                || !right.part_ids.is_empty()
                || !matrix.part_ids.is_empty()
                || doc
                    .layouts
                    .iter()
                    .any(|layout| layout.id == left.id || layout.id == right.id)
                || doc.matrices.iter().any(|item| {
                    left.matrix_ids.contains(&item.id) || right.matrix_ids.contains(&item.id)
                })
            {
                return Err(
                    "Mirrored pair requires two new layouts and distinct matrices on one board"
                        .into(),
                );
            }
            // New pairs store the half frame directly: the source origin is
            // re-expressed half-local and the mirror reflects it, so a later
            // Split gap change moves both halves without rewriting storage.
            // Anchored matrices stay legacy (no stored gap); their first edit
            // adopts only when an unanchored source matrix exists.
            // Ticket 11: the conversion still uses the requested placement,
            // while the stored gap is max(requested, 40 mm, minimum), so the
            // halves widen symmetrically to the stored gap on evaluation.
            let requested_gap = if matrix.anchor.is_none() {
                let gap = layout::implied_gap(link.axis_x, matrix);
                if !gap.is_finite() {
                    return Err("Split gap must be finite".into());
                }
                Some(gap)
            } else {
                None
            };
            let stored_gap = match requested_gap {
                Some(_) => Some(layout::proposed_creation_gap(doc, matrix, link.axis_x)?),
                None => None,
            };
            let mut source_half = matrix.as_ref().clone();
            if let Some(gap) = requested_gap {
                source_half.origin.x -= link.axis_x - gap / 2.0;
            }
            let mut changed = apply(
                doc,
                &[],
                &EditOperation::SetMatrix {
                    matrix: source_half.clone(),
                    definitions: definitions.clone(),
                },
            )?;
            let mut target = source_half.clone();
            target.id = right.matrix_ids[0].clone();
            target.name = Some(right.name.clone());
            for cell in &mut target.cells {
                for assembly in &mut cell.assemblies {
                    assembly.offset.x = -assembly.offset.x;
                    assembly.rotation = assembly.rotation.map(|angle| -angle);
                }
            }
            let target = layout::reflected(
                &source_half,
                &target,
                layout::axis_mapping(link.axis_x),
                false,
                stored_gap.is_some(),
            )?;
            changed.extend(matrix::set_matrix(doc, &target)?);
            let mut follower = right.clone();
            let link = follower
                .mirror_link
                .as_mut()
                .ok_or("A new mirrored pair must be linked")?;
            link.split_gap = stored_gap;
            link.correspondences.clear();
            link.correspondences.push(MirroredCorrespondence {
                kind: MirroredItemKind::Matrix,
                source_id: matrix.id.clone(),
                mirror_id: target.id.clone(),
                companion_mode: None,
                companion_tweak: None,
            });
            link.correspondences
                .extend(layout::part_correspondences(layout::member_pairs(
                    matrix,
                    &target.id,
                    Some(&target),
                )));
            doc.layouts.extend([left.clone(), follower]);
            layout::validate(doc)?;
            changed.extend([left.id.clone(), right.id.clone()]);
            Ok(changed)
        }
        EditOperation::CreateMirroredHalf {
            pair_id,
            source_instance_id,
            mode,
            labels,
            physical_setup,
            companion_mode,
            companion_tweak,
            mapping,
            id_rule,
            mounting_hole,
        } => match physical_setup {
            Some(setup) => mirrored_half::create_with_physical_setup(
                doc,
                pair_id,
                source_instance_id,
                mode,
                *mapping,
                *id_rule,
                mounting_hole.as_deref(),
                *companion_mode,
                *companion_tweak,
                labels.as_ref(),
                setup,
            ),
            None => mirrored_half::create(
                doc,
                pair_id,
                source_instance_id,
                mode,
                *mapping,
                *id_rule,
                mounting_hole.as_deref(),
                *companion_mode,
                *companion_tweak,
                labels.as_ref(),
            ),
        },
        EditOperation::SetLayout { layout: incoming } => {
            let mut incoming = incoming.clone();
            if incoming.mirror_link.is_some()
                && doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == incoming.id)
                    .is_some_and(|layout| layout.mirror_link.is_none())
            {
                layout::populate_new_link(doc, &mut incoming)?;
            }
            let (old_gap, was_linked_target, source_layout_id) = {
                let current = doc
                    .layouts
                    .iter()
                    .find(|layout| layout.id == incoming.id)
                    .ok_or("Layout does not exist")?;
                if current.board_id != incoming.board_id
                    || current.matrix_ids != incoming.matrix_ids
                {
                    return Err("Layout ownership cannot be reassigned".into());
                }
                let was_linked_target =
                    current.mirror_link.is_some() && incoming.mirror_link.is_none();
                let source_layout_id = was_linked_target
                    .then(|| {
                        current
                            .mirror_link
                            .as_ref()
                            .map(|link| link.source_id.clone())
                    })
                    .flatten();
                (
                    current.mirror_link.as_ref().and_then(|link| link.split_gap),
                    was_linked_target,
                    source_layout_id,
                )
            };
            // Half-frame gap transitions run before the record is replaced so
            // the conversion still sees the previous gap: adding a gap adopts
            // the board placement first, clearing one restores it.
            let new_gap = incoming
                .mirror_link
                .as_ref()
                .and_then(|link| link.split_gap);
            match (old_gap, new_gap) {
                (None, Some(_)) => {
                    layout::adopt_half_frames(doc)?;
                }
                (Some(_), None) => {
                    layout::unadopt_pair(doc, &incoming.id)?;
                }
                _ => {}
            }
            // Ticket 12: parts joining or leaving a stored-gap half reframe
            // with it, preserving evaluated poses. The unlink transition
            // skips leaving parts: unadopt above already restored them.
            let unlinking = matches!((old_gap, new_gap), (Some(_), None));
            if let Some(previous) = doc
                .layouts
                .iter()
                .find(|layout| layout.id == incoming.id)
                .cloned()
            {
                layout::reframe_moved_parts(doc, &previous, &incoming, unlinking)?;
            }
            let current = doc
                .layouts
                .iter_mut()
                .find(|layout| layout.id == incoming.id)
                .ok_or("Layout does not exist")?;
            let previous = current.clone();
            *current = incoming.clone();
            // Ticket 21: run the same-board exception lifecycle (Local/Omitted
            // transitions, Rejoin materialization) before resolve adopts the
            // current source state.
            let mut transitioned = layout::apply_exception_transitions(doc, &previous, &incoming)?;
            let mut detached = if was_linked_target {
                layout::unlink_components(doc, &incoming.id)
            } else {
                Vec::new()
            };
            if let Some(source_layout_id) = source_layout_id {
                detached.extend(layout::unlink_components(doc, &source_layout_id));
            }
            layout::resolve(doc)?;
            if !matches!((old_gap, new_gap), (Some(_), None)) {
                transitioned.extend(layout::adopt_half_frames(doc)?);
            }
            transitioned.append(&mut detached);
            Ok([
                [vec![incoming.id.clone()], incoming.matrix_ids.clone()].concat(),
                transitioned,
            ]
            .concat())
        }
        EditOperation::SetSplitGap {
            layout_id,
            split_gap,
        } => {
            if !split_gap.is_finite() || *split_gap < 0.0 {
                return Err("The Split gap must be a finite non-negative distance".into());
            }
            let paired = doc.layouts.iter().any(|layout| {
                layout.id == *layout_id && layout.mirror_link.is_some()
                    || layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == *layout_id)
            });
            if !paired {
                return Err(format!("Layout {layout_id} is not part of a mirrored pair"));
            }
            // A legacy board-coordinate pair adopts the half frame first, so
            // the gap change itself moves the halves without rewriting them.
            let mut changed = layout::adopt_half_frames(doc)?;
            changed.extend(layout::set_pair_gap(doc, layout_id, *split_gap)?);
            layout::resolve(doc)?;
            let root = doc
                .layouts
                .iter()
                .find(|layout| layout.id == *layout_id)
                .and_then(|layout| {
                    layout
                        .mirror_link
                        .as_ref()
                        .map(|link| link.source_id.clone())
                })
                .unwrap_or(layout_id.clone());
            for layout in doc.layouts.iter() {
                let in_pair = layout.id == root
                    || layout
                        .mirror_link
                        .as_ref()
                        .is_some_and(|link| link.source_id == root);
                if in_pair {
                    changed.push(layout.id.clone());
                    for matrix_id in &layout.matrix_ids {
                        changed.push(matrix_id.clone());
                        if let Some(matrix) =
                            doc.matrices.iter().find(|matrix| matrix.id == *matrix_id)
                        {
                            changed.extend(matrix.part_ids.iter().cloned());
                        }
                    }
                }
            }
            changed.sort();
            changed.dedup();
            Ok(changed)
        }
        EditOperation::ConvertOffsetConstraint { constraint_id, to } => {
            constraints::convert_offset(doc, constraint_id, *to)
        }
        EditOperation::SetConstraint { constraint } => {
            if let Some(reason) = matrix::anchors::constraining_refusal(doc, constraint) {
                return Err(reason);
            }
            if let Some(current) = doc
                .constraints
                .iter_mut()
                .find(|item| item.id() == constraint.id())
            {
                *current = constraint.clone();
            } else {
                doc.constraints.push(constraint.clone());
            }
            Ok(vec![constraint.id().into()])
        }
        EditOperation::RemoveConstraint { id } => {
            if !doc.constraints.iter().any(|item| item.id() == id) {
                return Err(format!("Unknown constraint {id}"));
            }
            doc.constraints.retain(|item| item.id() != id);
            Ok(vec![id.clone()])
        }
        EditOperation::ApplyMechanicalEdit { edit } => {
            crate::mechanical::edit::apply_with_ledger(doc, board_contours, edit, identities)
        }
        EditOperation::ApplyOutlineEdit { edit } => crate::outline_edit::apply(doc, edit),
        EditOperation::SetHalfException { .. }
        | EditOperation::SetCompanionPlacement { .. }
        | EditOperation::UnlinkHalves { .. } => {
            Err("Mirrored-half lifecycle edits require prepared transaction dispatch".into())
        }
        EditOperation::ReplaceDocument { document } => {
            if document.format != "boardstudio/v2" {
                return Err("Unsupported document format".into());
            }
            migrate::require_current_version(document.format_version)?;
            let accepted = doc.clone();
            let mut changed = changed_ids(doc, document);
            let mut prepared = document.as_ref().clone();
            script::apply_scripts(&mut prepared)?;
            layout::resolve(&mut prepared)?;
            generators::document_error(&prepared)?;
            *doc = prepared;
            // A wholesale replacement can add or remove a physical member without the
            // typed pair operations. Reconcile ownership against the accepted state so
            // the replacement can never leave a dangling pair or lose the survivor's
            // visible geometry and construction.
            changed.extend(crate::mechanical::edit::reconcile_physical_topology(
                doc, &accepted,
            )?);
            changed.sort();
            changed.dedup();
            Ok(changed)
        }
    }?;
    // The first committed geometry edit adopts the half frame: legacy
    // board-coordinate pairs store their implied Split gap and re-express
    // unanchored origins as half-frame values, preserving every world pose.
    // SetLayout and SetSplitGap manage their own gap transitions;
    // CreateMirroredPair stores the frame directly; parts adopt with 12.
    if matches!(
        op,
        EditOperation::SetMatrix { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::SetMatrixPlacement { .. }
            | EditOperation::AddAnchoredMatrix { .. }
            | EditOperation::SetMatrixAnchor { .. }
            | EditOperation::DetachMatrixAnchor { .. }
            | EditOperation::DetachMatrixAnchorConfirmed { .. }
            | EditOperation::RemoveMatrix { .. }
            | EditOperation::SetMatrixCellsEnabled { .. }
            | EditOperation::RemoveMatrixCells { .. }
            | EditOperation::RemoveMatrixRow { .. }
            | EditOperation::RemoveMatrixColumn { .. }
            | EditOperation::RemoveSelectedMatrixCells { .. }
    ) {
        changed.extend(layout::adopt_half_frames(doc)?);
    }
    Ok(changed)
}

fn changed_ids(a: &ProjectDoc, b: &ProjectDoc) -> Vec<String> {
    let mut ids = BTreeSet::new();
    for part in a.parts.iter().chain(b.parts.iter()) {
        ids.insert(part.id.clone());
    }
    for feature in a.outline.iter().chain(b.outline.iter()) {
        ids.insert(feature.id().to_string());
    }
    ids.into_iter().collect()
}

fn preview_electrical_loss_findings(accepted: &ProjectDoc, staged: &ProjectDoc) -> Vec<Finding> {
    let mut accepted_nets: BTreeMap<&str, &str> = BTreeMap::new();
    let mut accepted_pins = BTreeSet::new();
    for net in &accepted.nets {
        accepted_nets
            .entry(&net.id)
            .and_modify(|name| *name = (*name).min(&net.name))
            .or_insert(&net.name);
        for pin in &net.pins {
            accepted_pins.insert((net.id.as_str(), pin.part_id.as_str(), pin.pad_id.as_str()));
        }
    }

    let staged_nets: BTreeSet<_> = staged.nets.iter().map(|net| net.id.as_str()).collect();
    let staged_pins: BTreeSet<_> = staged
        .nets
        .iter()
        .flat_map(|net| {
            net.pins
                .iter()
                .map(move |pin| (net.id.as_str(), pin.part_id.as_str(), pin.pad_id.as_str()))
        })
        .collect();
    let parts: BTreeMap<&str, &str> =
        accepted
            .parts
            .iter()
            .fold(BTreeMap::new(), |mut parts, part| {
                parts
                    .entry(&part.id)
                    .and_modify(|reference| *reference = (*reference).min(&part.reference))
                    .or_insert(&part.reference);
                parts
            });
    let boards: BTreeMap<&str, &str> =
        accepted
            .boards
            .iter()
            .fold(BTreeMap::new(), |mut boards, board| {
                boards
                    .entry(&board.id)
                    .and_modify(|name| *name = (*name).min(&board.name))
                    .or_insert(&board.name);
                boards
            });
    let staged_registrations: BTreeSet<_> = staged
        .boards
        .iter()
        .flat_map(|board| {
            board
                .net_ids
                .iter()
                .map(move |net_id| (board.id.as_str(), net_id.as_str()))
        })
        .collect();

    let mut findings = BTreeMap::new();
    for (net_id, part_id, pad_id) in accepted_pins.difference(&staged_pins) {
        let net_name = accepted_nets.get(net_id).copied().unwrap_or(net_id);
        let reference = parts.get(part_id).copied().unwrap_or(part_id);
        let finding = Finding {
            id: preview_electrical_notice_id("pin", &[net_id, part_id, pad_id]),
            severity: Severity::Info,
            scope: Scope::Pcb,
            message: format!(
                "Preview removes pad {pad_id} on {reference} ({part_id}) from net {net_name} ({net_id})."
            ),
            target_ids: vec![(*net_id).to_string(), (*part_id).to_string()],
        };
        findings.insert(finding.id.clone(), finding);
    }

    for (net_id, net_name) in &accepted_nets {
        if !staged_nets.contains(net_id) {
            let finding = Finding {
                id: preview_electrical_notice_id("net", &[net_id]),
                severity: Severity::Info,
                scope: Scope::Pcb,
                message: format!("Preview removes net {net_name} ({net_id})."),
                target_ids: vec![(*net_id).to_string()],
            };
            findings.insert(finding.id.clone(), finding);
        }
    }

    let accepted_registrations: BTreeSet<_> = accepted
        .boards
        .iter()
        .flat_map(|board| {
            board
                .net_ids
                .iter()
                .map(move |net_id| (board.id.as_str(), net_id.as_str()))
        })
        .collect();
    for (board_id, net_id) in accepted_registrations.difference(&staged_registrations) {
        let board_name = boards.get(board_id).copied().unwrap_or(board_id);
        let net_name = accepted_nets.get(net_id).copied().unwrap_or(net_id);
        let finding = Finding {
            id: preview_electrical_notice_id("registration", &[board_id, net_id]),
            severity: Severity::Info,
            scope: Scope::Pcb,
            message: format!(
                "Preview removes net {net_name} ({net_id}) from board {board_name} ({board_id})."
            ),
            target_ids: vec![(*board_id).to_string(), (*net_id).to_string()],
        };
        findings.insert(finding.id.clone(), finding);
    }

    findings.into_values().collect()
}

fn preview_electrical_notice_id(kind: &str, identity: &[&str]) -> String {
    let mut id = format!("preview:electrical:{kind}");
    for component in identity {
        id.push(':');
        id.push_str(&component.len().to_string());
        id.push('-');
        for byte in component.as_bytes() {
            use std::fmt::Write as _;
            write!(id, "{byte:02x}").expect("writing to a string cannot fail");
        }
    }
    id
}

#[cfg(test)]
mod preview_electrical_loss_tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.parts.push(Part {
            anchor: None,
            keycap: None,
            outline: None,
            id: "part:one".into(),
            definition_id: "definition".into(),
            reference: "U1".into(),
            pose: Pose2 {
                at: Vec2 { x: 0.0, y: 0.0 },
                rotation: 0.0,
            },
            side: Side::Front,
            locked: None,
            properties: None,
            generator_parameters: None,
        });
        document.nets.push(Net {
            id: "net:one".into(),
            name: "CTRL".into(),
            pins: vec![
                Pin {
                    part_id: "part:one".into(),
                    pad_id: "pad:1".into(),
                },
                Pin {
                    part_id: "part:one".into(),
                    pad_id: "pad-2".into(),
                },
            ],
        });
        document.boards.push(Board {
            id: "board:one".into(),
            name: "Right".into(),
            outline_ids: vec![],
            part_ids: vec!["part:one".into()],
            net_ids: vec!["net:one".into()],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    #[test]
    fn whole_net_removal_reports_each_distinct_pin_net_and_registration() {
        let accepted = document();
        let mut staged = accepted.clone();
        staged.nets.clear();
        staged.boards[0].net_ids.clear();

        let findings = preview_electrical_loss_findings(&accepted, &staged);

        assert_eq!(findings.len(), 4);
        let messages: BTreeSet<_> = findings
            .iter()
            .map(|finding| finding.message.as_str())
            .collect();
        assert!(
            messages
                .contains("Preview removes pad pad:1 on U1 (part:one) from net CTRL (net:one).")
        );
        assert!(
            messages
                .contains("Preview removes pad pad-2 on U1 (part:one) from net CTRL (net:one).")
        );
        assert!(messages.contains("Preview removes net CTRL (net:one)."));
        assert!(
            messages.contains("Preview removes net CTRL (net:one) from board Right (board:one).")
        );
        assert!(findings.iter().all(|finding| {
            finding.id.starts_with("preview:electrical:")
                && finding.severity == Severity::Info
                && finding.scope == Scope::Pcb
        }));
        assert_eq!(
            findings,
            preview_electrical_loss_findings(&accepted, &staged)
        );
    }

    #[test]
    fn retained_net_and_board_registration_report_only_the_removed_pin() {
        let accepted = document();
        let mut staged = accepted.clone();
        staged.nets[0].pins.retain(|pin| pin.pad_id != "pad:1");

        let findings = preview_electrical_loss_findings(&accepted, &staged);

        assert_eq!(findings.len(), 1);
        assert_eq!(
            findings[0].message,
            "Preview removes pad pad:1 on U1 (part:one) from net CTRL (net:one)."
        );
        assert!(preview_electrical_loss_findings(&accepted, &accepted).is_empty());
    }

    #[test]
    fn repeated_identities_are_deduplicated_and_notice_ids_are_collision_safe() {
        let mut accepted = document();
        let repeated_pin = accepted.nets[0].pins[0].clone();
        accepted.nets[0].pins.push(repeated_pin);
        accepted.boards[0].net_ids.push("net:one".into());
        let mut staged = accepted.clone();
        staged.nets.clear();
        staged.boards[0].net_ids.clear();

        let findings = preview_electrical_loss_findings(&accepted, &staged);
        let ids: BTreeSet<_> = findings.iter().map(|finding| finding.id.as_str()).collect();
        assert_eq!(findings.len(), 4);
        assert_eq!(ids.len(), findings.len());
        assert_ne!(
            preview_electrical_notice_id("pin", &["a:b", "c"]),
            preview_electrical_notice_id("pin", &["a", "b:c"])
        );
        assert_ne!(
            preview_electrical_notice_id("pin", &["é", "pad"]),
            preview_electrical_notice_id("pin", &["e", "\u{301}pad"])
        );
    }
}

#[cfg(test)]
mod matrix_protocol_tests {
    use super::*;
    use serde_json::json;

    fn draft(id: &str, rows: u64) -> serde_json::Value {
        json!({ "id": id, "rows": rows, "columns": 2, "pitch": {"x": 19.0, "y": 19.0}, "origin": {"x": 0.0, "y": 0.0}, "definitionId": "missing", "partIds": [] })
    }

    #[test]
    fn draft_request_is_stateless_and_invalid_batches_fail_atomically() {
        let mut engine = CoreEngine::default();
        let request = json!({ "kind": "project-matrices", "id": "p", "baseRevision": 0, "matrices": [draft("ok", 1)] });
        let reply: CoreReply = serde_json::from_str(&engine.request(&request.to_string())).unwrap();
        let CoreReply::MatrixProjections {
            revision,
            matrix_scenes,
            ..
        } = reply
        else {
            panic!("expected projections")
        };
        assert_eq!(revision, 0);
        assert_eq!(matrix_scenes.len(), 1);
        let invalid = json!({ "kind": "project-matrices", "id": "p2", "baseRevision": 0, "matrices": [draft("ok", 1), draft("bad", 0)] });
        let reply: CoreReply = serde_json::from_str(&engine.request(&invalid.to_string())).unwrap();
        assert!(matches!(reply, CoreReply::Error { .. }));
        assert_eq!(engine.document.revision, 0);
        assert!(engine.undo.is_empty());
        assert!(engine.redo.is_empty());
    }
}

#[cfg(test)]
mod set_wiring_mode_tests {
    use super::*;

    fn document() -> ProjectDoc {
        let mut document = ProjectDoc::empty("project", "Test");
        document.boards.push(Board {
            id: "left".into(),
            name: "Left".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document
    }

    fn set_mode(mode: electrical::ElectricalMode) -> EditOperation {
        EditOperation::SetWiringMode {
            board_id: "left".into(),
            mode,
        }
    }

    #[test]
    fn setting_direct_creates_board_configuration_and_reports_only_the_board() {
        let mut document = document();

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Direct),
        )
        .unwrap();

        assert_eq!(changed, ["left"]);
        let configuration = document.hardware.unwrap().boards.pop().unwrap();
        assert_eq!(
            configuration,
            ElectricalBoardConfiguration {
                board_id: "left".into(),
                mode: electrical::ElectricalMode::Direct,
                ..Default::default()
            }
        );
    }

    #[test]
    fn setting_the_current_mode_is_a_no_op() {
        let mut document = document();
        let operation = set_mode(electrical::ElectricalMode::Direct);
        apply(&mut document, &[], &operation).unwrap();
        let before = document.clone();

        let changed = apply(&mut document, &[], &operation).unwrap();

        assert!(changed.is_empty());
        assert_eq!(document, before);
    }

    #[test]
    fn changing_mode_preserves_the_board_configuration() {
        let mut document = document();
        let mut configuration = ElectricalBoardConfiguration {
            board_id: "left".into(),
            controller_part_id: Some("controller".into()),
            mode: electrical::ElectricalMode::Matrix,
            ..Default::default()
        };
        configuration.locks.insert("row".into(), "P1".into());
        document
            .hardware
            .get_or_insert_with(Default::default)
            .boards
            .push(configuration.clone());

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Direct),
        )
        .unwrap();

        assert_eq!(changed, ["left"]);
        configuration.mode = electrical::ElectricalMode::Direct;
        assert_eq!(document.hardware.unwrap().boards, [configuration]);
    }

    #[test]
    fn setting_the_default_mode_is_a_no_op() {
        let mut document = document();

        let changed = apply(
            &mut document,
            &[],
            &set_mode(electrical::ElectricalMode::Matrix),
        )
        .unwrap();

        assert!(changed.is_empty());
        assert!(document.hardware.is_none());
    }

    #[test]
    fn setting_mode_for_an_unknown_board_is_an_edit_error() {
        let mut document = document();
        let operation = EditOperation::SetWiringMode {
            board_id: "missing".into(),
            mode: electrical::ElectricalMode::Direct,
        };

        assert_eq!(
            apply(&mut document, &[], &operation),
            Err("Unknown board missing".into())
        );
    }

    #[test]
    fn setting_wiring_mode_does_not_recompute_outlines() {
        assert!(!affects_outline(&set_mode(
            electrical::ElectricalMode::Direct
        )));
    }
}

#[cfg(test)]
mod mechanical_edit_engine_tests {
    use super::*;
    use crate::mechanical::change::MechanicalDimension;
    use crate::mechanical::{MechanicalChange, MechanicalEdit, MechanicalTarget};

    fn board() -> Board {
        Board {
            id: "board".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        }
    }

    fn configuration(board_id: &str) -> MechanicalConfiguration {
        serde_json::from_value(serde_json::json!({
            "boardId": board_id, "method": "printed", "mount": "rigid",
            "plateThickness": 1.5, "plateFoamThickness": 0.5, "pcbThickness": 1.6,
            "bottomFoamThickness": 0.4, "batteryHeight": 0.0, "bottomThickness": 3.0,
            "plateToPcb": 3.5, "wallThickness": 2.0, "clearance": 0.3,
            "mounts": [], "closureMounts": [], "profiles": []
        }))
        .unwrap()
    }

    fn template() -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id":"generator:ceoloide/mounting_hole_npth", "name":"mounting hole npth",
            "kind":"custom", "pads":[{"id":"pad-0", "number":"", "at":{"x":0,"y":0},
                "size":{"x":2.2,"y":2.2}, "shape":"circle", "drill":2.2,"plated":false,"rotation":0}],
            "courtyard":[{"x":-1.1,"y":-1.1},{"x":1.1,"y":-1.1},{"x":1.1,"y":1.1},{"x":-1.1,"y":1.1}],
            "envelopeSource":{"courtyard":"generated"},
            "envelopeNotice":"No closed courtyard is available; the outline uses physical graphics and pad extents.",
            "generator":{"source":"ceoloide/mounting_hole_npth", "version":"bundled-1", "parameters":{}}
        }))
        .unwrap()
    }

    fn edit_op(board_id: &str, change: MechanicalChange, mounting_hole: bool) -> EditOperation {
        EditOperation::ApplyMechanicalEdit {
            edit: Box::new(MechanicalEdit {
                target: MechanicalTarget::Board {
                    board_id: board_id.into(),
                },
                change,
                mounting_hole: mounting_hole.then(|| Box::new(template())),
            }),
        }
    }

    fn command(base: u64, operation: EditOperation) -> CoreRequest {
        CoreRequest::Edit {
            id: "mechanical".into(),
            command: EditCommand {
                base_revision: base,
                transaction_id: "mechanical".into(),
                phase: EditPhase::Commit,
                target_ids: vec![],
                operation,
            },
        }
    }

    fn open(engine: &mut CoreEngine, document: ProjectDoc) -> SceneDelta {
        match engine.handle(CoreRequest::Open {
            id: "open".into(),
            document,
        }) {
            CoreReply::Scene { scene, .. } => scene,
            reply => panic!("open failed: {reply:?}"),
        }
    }

    fn scene(reply: CoreReply) -> (SceneDelta, ProjectDoc) {
        match reply {
            CoreReply::Scene {
                scene, document, ..
            } => (scene, *document),
            reply => panic!("expected scene: {reply:?}"),
        }
    }

    fn generated_parts(document: &ProjectDoc) -> Vec<&str> {
        document
            .parts
            .iter()
            .map(|part| part.id.as_str())
            .filter(|id| id.starts_with("case-closure/"))
            .collect()
    }

    fn generated_definitions(document: &ProjectDoc) -> Vec<&str> {
        document
            .definitions
            .iter()
            .map(|definition| definition.id.as_str())
            .filter(|id| id.starts_with("assembly-closure/definition/"))
            .collect()
    }

    #[test]
    fn explicit_board_clear_preserves_other_boards_configuration() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let mut other = board();
        other.id = "other".into();
        document.boards.push(other);
        document.mechanical = Some(configuration("board"));
        let before = document.clone();

        // Ticket 37: raw SetMechanical is deleted; Disable is the live typed clear.
        apply(
            &mut document,
            &[],
            &edit_op("other", MechanicalChange::Disable, false),
        )
        .expect("clearing another board is an unchanged explicit target");
        assert_eq!(
            document, before,
            "the existing slot belongs to a different board"
        );

        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Disable, false),
        )
        .expect("clearing the slot's own board removes its configuration");
        assert!(document.mechanical.is_none());
    }

    #[test]
    fn targeted_instance_set_mechanical_requires_preparation_before_activation() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let before = document.clone();

        // Ticket 37: raw SetMechanical is deleted; an instance-targeted typed
        // edit through raw apply must not activate instance configuration.
        let refusal = apply(
            &mut document,
            &[],
            &EditOperation::ApplyMechanicalEdit {
                edit: Box::new(MechanicalEdit {
                    target: MechanicalTarget::Instance {
                        instance_id: "physical-half".into(),
                    },
                    change: MechanicalChange::Disable,
                    mounting_hole: None,
                }),
            },
        )
        .expect_err("a missing instance stays refused through raw apply");

        assert!(refusal.contains("Physical instances are unavailable"));
        assert_eq!(document, before);
    }

    #[test]
    fn explicit_board_set_mechanical_keeps_board_slot_semantics() {
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        let mut other = board();
        other.id = "other".into();
        document.boards.push(other);
        // Ticket 37: raw SetMechanical is deleted; Enable/Disable are the live
        // typed board-slot writes.
        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Enable, false),
        )
        .expect("an explicit Board target uses the existing board slot");
        assert_eq!(
            document
                .mechanical
                .as_ref()
                .map(|item| item.board_id.as_str()),
            Some("board")
        );

        let before = document.clone();
        apply(
            &mut document,
            &[],
            &edit_op("other", MechanicalChange::Disable, false),
        )
        .expect("disabling another board leaves this slot alone");
        assert_eq!(
            document, before,
            "a target mismatch leaves the document alone"
        );

        apply(
            &mut document,
            &[],
            &edit_op("board", MechanicalChange::Disable, false),
        )
        .expect("disabling the slot's own board clears its configuration");
        assert!(document.mechanical.is_none());
    }

    /// Case 3: a no-op mechanical edit adds no history step and no revision bump.
    #[test]
    fn a_no_op_mechanical_edit_keeps_revision_and_history() {
        let mut engine = CoreEngine::new();
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.boards.push(board());
        document.mechanical = Some(configuration("board"));
        open(&mut engine, document);

        let revision = engine.document.revision;
        let undo = engine.undo.len();
        let redo = engine.redo.len();
        let reply = engine.handle(command(
            revision,
            edit_op(
                "board",
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 2.0,
                },
                false,
            ),
        ));
        let CoreReply::Scene {
            scene, document, ..
        } = reply
        else {
            panic!("expected scene")
        };
        assert!(scene.changed_ids.is_empty());
        assert_eq!(document.revision, revision, "no revision bump");
        assert_eq!(engine.document.revision, revision);
        assert_eq!(engine.undo.len(), undo, "no undo entry");
        assert_eq!(engine.redo.len(), redo, "redo is not cleared");
    }

    /// Case 2: generated clearance parts, definitions and memberships appear in
    /// the same reply and disappear together on one Undo.
    #[test]
    fn closure_projection_lands_atomically_and_undoes_in_one_step() {
        let mut engine = CoreEngine::new();
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.outline.push(OutlineFeature::Rect {
            anchor_part_id: None,
            id: "outline".into(),
            center: Vec2 { x: 0.0, y: 0.0 },
            size: Vec2 { x: 80.0, y: 60.0 },
            radius: 0.0,
            rotation: None,
            operation: Operation::Add,
        });
        let mut board = board();
        board.outline_ids = vec!["outline".into()];
        document.boards.push(board);
        // Absent closure mounts are initialized by the edit.
        document.mechanical = Some(configuration("board"));
        document.mechanical.as_mut().unwrap().closure_mounts = None;
        open(&mut engine, document);

        let revision = engine.document.revision;
        let (reply_scene, projected) = scene(engine.handle(command(
            revision,
            edit_op("board", MechanicalChange::InitializeClosures, true),
        )));

        let parts = generated_parts(&projected);
        let definitions = generated_definitions(&projected);
        assert!(!parts.is_empty(), "generated clearance parts appear");
        assert!(
            !definitions.is_empty(),
            "generated clearance definitions appear"
        );
        assert!(
            projected.boards[0]
                .part_ids
                .iter()
                .any(|id| id.starts_with("case-closure/")),
            "the board membership includes the generated parts"
        );
        for id in parts.iter().chain(definitions.iter()) {
            assert!(
                reply_scene.changed_ids.contains(&(*id).to_string()),
                "changed ids name generated {id}"
            );
        }
        assert!(reply_scene.changed_ids.contains(&"board".to_string()));

        let undo_reply = engine.handle(CoreRequest::Undo { id: "undo".into() });
        let (_, undone) = scene(undo_reply);
        assert!(
            generated_parts(&undone).is_empty(),
            "one Undo removes every generated part"
        );
        assert!(
            generated_definitions(&undone).is_empty(),
            "one Undo removes every generated definition"
        );
        assert!(
            undone.boards[0]
                .part_ids
                .iter()
                .all(|id| !id.starts_with("case-closure/")),
            "one Undo removes the generated memberships"
        );
        assert_eq!(
            undone.mechanical.as_ref().unwrap().closure_mounts,
            None,
            "one Undo restores the absent closure mounts"
        );
    }

    /// Case 6: a pure settings edit does not rebuild board outlines; the
    /// outline-version recovery snapshot is the observable.
    #[test]
    fn pure_settings_edits_preserve_the_outline_recovery_snapshot() {
        let mut source: ProjectDoc =
            serde_json::from_str(include_str!("../tests/fixtures/synthetic-outline.json")).unwrap();
        source.format_version = crate::migrate::CURRENT_VERSION;
        // Use only two authored square courtyards with a 4 mm gap. This creates
        // an independent automatic bridge pocket for the recovery contract.
        source
            .parts
            .iter_mut()
            .find(|part| part.id == "main/SW2")
            .expect("the second synthetic switch exists")
            .pose
            .at = Vec2 { x: 24.0, y: 0.0 };
        let protected_parts = vec!["main/SW1".to_string(), "main/SW2".to_string()];
        let OutlineFeature::PartEnvelope {
            part_ids,
            margin,
            settings,
            ..
        } = &mut source.outline[0]
        else {
            unreachable!("the synthetic outline is a part envelope")
        };
        *part_ids = protected_parts.clone();
        *margin = 1.0;
        settings.bridge_width = 10.0;
        source.boards[0].part_ids = protected_parts;
        // Protect a generated gap so opening the project writes a recovery snapshot.
        let mut probe = CoreEngine::new();
        let probe_scene = open(&mut probe, source.clone());
        let gap = probe_scene.board_outline_scenes[0]
            .gaps
            .first()
            .cloned()
            .expect("the repaired outline exposes a gap");
        if let OutlineFeature::PartEnvelope { settings, .. } = &mut source.outline[0] {
            settings.repair = Some(OutlineRepairSettings {
                keep_gaps: vec![ProtectedOutlineGap {
                    id: gap.id,
                    points: gap.points,
                }],
                ..Default::default()
            });
        }
        source.mechanical = Some(configuration("main"));

        let mut engine = CoreEngine::new();
        open(&mut engine, source);
        assert!(
            engine
                .document
                .board_outlines
                .iter()
                .any(|state| state.generated_last_valid.is_some()),
            "the protected outline has a recovery snapshot to observe"
        );
        let revision = engine.document.revision;
        let before = engine.document.board_outlines.clone();
        let (_, document) = scene(engine.handle(command(
            revision,
            edit_op(
                "main",
                MechanicalChange::SetDimension {
                    field: MechanicalDimension::WallThickness,
                    value: 2.5,
                },
                false,
            ),
        )));
        assert_eq!(document.revision, revision + 1, "the settings edit commits");
        assert_eq!(
            document.board_outlines, before,
            "a pure settings edit leaves the outline recovery snapshot untouched"
        );
    }
}
