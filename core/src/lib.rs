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
use matrix::layout;
use model::*;
use std::collections::{BTreeMap, BTreeSet};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct CoreEngine {
    document: ProjectDoc,
    undo: Vec<ProjectDoc>,
    redo: Vec<ProjectDoc>,
    contours: Vec<Contour>,
    findings: Vec<Finding>,
    outline_cache: OutlineCache,
}

#[wasm_bindgen]
impl CoreEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self::default()
    }

    // Worker transport keeps the Rust and TypeScript protocols identical.
    pub fn request(&mut self, json: &str) -> String {
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
#[wasm_bindgen]
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
        }
    }
}

impl CoreEngine {
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
            CoreRequest::Open { id, mut document } => {
                if document.format != "boardstudio/v2" {
                    return self.error(id, "Unsupported document format");
                }
                if document.format_version != migrate::CURRENT_VERSION {
                    return self.error(id, "Document must be migrated before it is opened");
                }
                if let Some(map) = &document.keymap
                    && let Err(message) = keymap::validate(map)
                {
                    return self.error(id, &message);
                }
                if let Err(message) = script::apply_scripts(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = layout::resolve(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = constraints::resolve(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = layout::sync_components(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = constraints::resolve(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = layout::sync_components(&mut document) {
                    return self.error(id, &message);
                }
                if let Err(message) = generators::document_error(&document) {
                    return self.error(id, &message);
                }
                if let Err(message) = mechanical::halves::validate_document(&document) {
                    return self.error(id, &message);
                }
                self.document = document;
                self.undo.clear();
                self.redo.clear();
                self.recompute();
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
                let mut scenes = Vec::with_capacity(matrices.len());
                for matrix in matrices {
                    if let Err(message) = matrix::valid_projection_matrix(&matrix) {
                        return self.error(id, &message);
                    }
                    match matrix::project_matrix(
                        &matrix,
                        &self.document.parts,
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
                self.document.revision += 1;
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
        if command.phase == EditPhase::Preview
            && matches!(
                &command.operation,
                EditOperation::MoveParts { .. }
                    | EditOperation::SetMatrix { .. }
                    | EditOperation::SetMatrixSplay { .. }
                    | EditOperation::SetMatrixPlacement { .. }
            )
        {
            return self.preview_edit(id, command);
        }
        let mut next = self.document.clone();
        // Mechanical edits resolve against the accepted outline projection the user
        // sees, not a stale or re-derived one, so the accepted cache supplies it.
        let accepted_contours;
        let board_contours: &[BoardContours] =
            if matches!(command.operation, EditOperation::ApplyMechanicalEdit { .. }) {
                accepted_contours = geometry::board_contours(&self.document, &self.outline_cache).0;
                &accepted_contours
            } else {
                &[]
            };
        let changed = match apply(&mut next, board_contours, &command.operation) {
            Ok(changed) => changed,
            Err(message) => return self.error(id, &message),
        };
        if let Some(map) = &next.keymap
            && let Err(message) = keymap::validate(map)
        {
            return self.error(id, &message);
        }
        let mut changed = changed;
        electrical::preserve_handoff(&self.document, &mut next);
        if let Err(message) = layout::validate(&next) {
            return self.error(id, &message);
        }
        match constraints::resolve_edit(&mut next, &command.operation) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match layout::sync_components(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match constraints::resolve_edit(&mut next, &command.operation) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match layout::sync_components(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        outline_controls::detach_removed(&self.document.parts, &mut next);
        changed.extend(prune_removed_components(&self.document.parts, &mut next));
        changed.sort();
        changed.dedup();
        // ReplaceDocument is judged by its own arm in `apply`, which allows warnings.
        if !matches!(command.operation, EditOperation::ReplaceDocument { .. })
            && let Err(message) =
                generators::introduced_error(&generators::parameter_problems(&self.document), &next)
        {
            return self.error(id, &message);
        }
        // A committed mechanical edit that reports nothing changed is a no-op: return
        // the current committed scene without a revision bump, an undo entry or a redo
        // clear. The no-op policy of every other operation is unchanged.
        if changed.is_empty()
            && command.phase == EditPhase::Commit
            && matches!(
                command.operation,
                EditOperation::ApplyMechanicalEdit { .. } | EditOperation::ApplyOutlineEdit { .. }
            )
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
        let (cache, contours, findings) = if affects_outline(&command.operation) {
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
        next.revision = self.document.revision + 1;
        outline_versions::refresh_recovery(&mut next, &cache);
        self.undo.push(self.document.clone());
        self.redo.clear();
        self.document = next;
        self.contours = contours;
        self.findings = findings;
        self.outline_cache = cache;
        self.scene(
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
        )
    }

    fn preview_edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        let accepted = generators::parameter_problems(&self.document);
        let backup = PreviewBackup::capture(&self.document, &command.operation);
        let attachments = outline_controls::attached_parts(&self.document);
        let before_parts = self.document.parts.clone();
        let result = apply(&mut self.document, &[], &command.operation).and_then(|mut changed| {
            layout::validate(&self.document)?;
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
        let (from, to) = match direction {
            History::Undo => (&mut self.undo, &mut self.redo),
            History::Redo => (&mut self.redo, &mut self.undo),
        };
        let Some(mut next) = from.pop() else {
            return self.error(id, "History is empty");
        };
        let changed = changed_ids(&self.document, &next);
        electrical::preserve_handoff(&self.document, &mut next);
        to.push(self.document.clone());
        next.revision = self.document.revision + 1;
        self.document = next;
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
                    pose: part.pose,
                })
                .collect(),
            matrix_scenes: doc
                .matrices
                .iter()
                .filter_map(|matrix| {
                    if matrix::valid_matrix(matrix, doc).is_err() {
                        return None;
                    }
                    matrix::project_matrix(matrix, &doc.parts, matrix::ProjectionMode::Actual)
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
            return Self::Parts(
                doc.parts
                    .iter()
                    .enumerate()
                    .filter(|(_, part)| affected.contains(&part.id))
                    .map(|(index, part)| (index, part.pose, part.properties.clone()))
                    .collect(),
            );
        }
        Self::Matrix {
            definition_len: doc.definitions.len(),
            parts: doc.parts.clone(),
            matrices: doc.matrices.clone(),
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
                nets,
                outline,
                boards,
                constraints,
            } => {
                doc.definitions.truncate(definition_len);
                doc.parts = parts;
                doc.matrices = matrices;
                doc.nets = nets;
                doc.outline = outline;
                doc.boards = boards;
                doc.constraints = constraints;
            }
        }
    }
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
            | EditOperation::SetOutline { .. }
            | EditOperation::CopyOutline { .. }
            | EditOperation::SelectOutline { .. }
            | EditOperation::RemoveOutline { .. }
            | EditOperation::AddPart { .. }
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

fn apply(
    doc: &mut ProjectDoc,
    board_contours: &[BoardContours],
    op: &EditOperation,
) -> Result<Vec<String>, String> {
    match op {
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
                let delta = Vec2 {
                    x: position.at.x - doc.parts[index].pose.at.x,
                    y: position.at.y - doc.parts[index].pose.at.y,
                };
                if let Some(constraint) = doc
                    .constraints
                    .iter_mut()
                    .find(|constraint| constraint.target() == position.id)
                {
                    if matches!(constraint, Constraint::Mirror { .. }) {
                        return Err(format!(
                            "Part {} is controlled by a mirror constraint and cannot be dragged",
                            position.id
                        ));
                    }
                    let source = doc
                        .parts
                        .iter()
                        .find(|part| part.id == constraint.source())
                        .ok_or("Constraint source is missing")?
                        .pose;
                    crate::constraints::retune_offset(constraint, source, delta, 0.0);
                    changed.push(position.id.clone());
                    continue;
                }
                if let Some(linked) = layout::move_linked_component(doc, &position.id, position.at)?
                {
                    changed.extend(linked);
                    continue;
                }
                let part = &mut doc.parts[index];
                part.pose.at = position.at;
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
        EditOperation::SetOutline { feature } => {
            let id = feature.id().to_string();
            let current = doc
                .board_outlines
                .iter_mut()
                .flat_map(|state| &mut state.versions)
                .flat_map(|version| &mut version.geometry.features)
                .find(|item| item.id() == id)
                .or_else(|| doc.outline.iter_mut().find(|item| item.id() == id));
            if let Some(current) = current {
                *current = feature.clone();
            } else {
                doc.outline.push(feature.clone());
            }
            Ok(vec![id])
        }
        operation @ (EditOperation::CopyOutline { .. }
        | EditOperation::SelectOutline { .. }
        | EditOperation::RenameOutline { .. }
        | EditOperation::RemoveOutline { .. }) => outline_versions::apply(doc, operation),
        EditOperation::AddPart { part, board_id } => {
            if doc.parts.iter().any(|item| item.id == part.id) {
                return Err(format!("Duplicate part {}", part.id));
            }
            if !geometry::part_valid(part, doc) {
                return Err(format!("Unknown definition {}", part.definition_id));
            }
            let board_index = if let Some(id) = board_id {
                Some(
                    doc.boards
                        .iter()
                        .position(|board| &board.id == id)
                        .ok_or_else(|| format!("Unknown board {id}"))?,
                )
            } else if doc.boards.is_empty() {
                None
            } else {
                Some(0)
            };
            let board_outline = board_index
                .and_then(|index| doc.boards.get(index))
                .map(|board| board.outline_ids.clone())
                .unwrap_or_default();
            if let Some(board) = board_index.and_then(|index| doc.boards.get_mut(index)) {
                board.part_ids.push(part.id.clone());
            }
            let mut attached = false;
            for feature in &mut doc.outline {
                if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature
                    && board_outline.contains(id)
                {
                    part_ids.push(part.id.clone());
                    attached = true;
                }
            }
            if !attached
                && doc.boards.is_empty()
                && let Some(OutlineFeature::PartEnvelope { part_ids, .. }) = doc
                    .outline
                    .iter_mut()
                    .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
            {
                part_ids.push(part.id.clone());
            }
            doc.parts.push(part.clone());
            Ok(vec![part.id.clone()])
        }
        EditOperation::RemoveMatrix { id } => {
            let ids = doc
                .matrices
                .iter()
                .find(|matrix| matrix.id == *id)
                .ok_or("Matrix does not exist")?
                .part_ids
                .clone();
            layout::remove_matrix(doc, id);
            let mut changed = apply(doc, &[], &EditOperation::RemoveParts { ids })?;
            doc.matrices.retain(|matrix| matrix.id != *id);
            let prefix = format!("matrix/{id}/net/");
            let empty_owned: std::collections::BTreeSet<_> = doc
                .nets
                .iter()
                .filter(|net| net.id.starts_with(&prefix) && net.pins.is_empty())
                .map(|net| net.id.clone())
                .collect();
            doc.nets.retain(|net| !empty_owned.contains(&net.id));
            for board in &mut doc.boards {
                board.net_ids.retain(|id| !empty_owned.contains(id));
            }
            changed.push(id.clone());
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
            let mut changed = ids;
            let cleanup = outline_versions::prune_removed_sources(doc, &changed);
            changed.extend(cleanup);
            for matrix_id in affected {
                changed.extend(layout::sync(doc, &matrix_id)?);
            }
            Ok(changed)
        }
        EditOperation::SetNet { net } => {
            if let Some(current) = doc.nets.iter_mut().find(|item| item.id == net.id) {
                *current = net.clone();
            } else {
                doc.nets.push(net.clone());
            }
            if let Some(board) = doc.boards.first_mut()
                && !board.net_ids.contains(&net.id)
            {
                board.net_ids.push(net.id.clone());
            }
            Ok(vec![net.id.clone()])
        }
        EditOperation::SetMechanical { configuration } => {
            doc.mechanical = configuration.as_deref().cloned();
            Ok(vec![])
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
        } => {
            let mut changed = vec![];
            let mut provided = BTreeSet::new();
            for definition in definitions.iter().flatten() {
                if definition.id.trim().is_empty() || !provided.insert(&definition.id) {
                    return Err("Matrix definition ID is invalid or duplicated".into());
                }
                if let Some(existing) = doc.definitions.iter().find(|item| item.id == definition.id)
                {
                    if existing != definition {
                        return Err(format!("Conflicting definition {}", definition.id));
                    }
                    continue;
                }
                doc.definitions.push(definition.clone());
                changed.push(definition.id.clone());
            }
            changed.extend(matrix::set_matrix(doc, matrix)?);
            changed.extend(layout::sync(doc, &matrix.id)?);
            Ok(changed)
        }
        EditOperation::SetMatrixSplay {
            matrix_id,
            column,
            change,
        } => {
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .ok_or("Matrix not found")?;
            let matrix = matrix::splay::update(current, *column, change)?;
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
            let current = doc
                .matrices
                .iter()
                .find(|matrix| &matrix.id == matrix_id)
                .cloned()
                .ok_or("Matrix not found")?;
            let matrix = matrix::placement::update(&current, change)?;
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
                || matrix.id != left.matrix_id
                || left.matrix_id == right.matrix_id
                || matrix.board_id.as_ref() != Some(&left.board_id)
                || left.board_id != right.board_id
                || !left.part_ids.is_empty()
                || !right.part_ids.is_empty()
                || !matrix.part_ids.is_empty()
                || doc
                    .layouts
                    .iter()
                    .any(|layout| layout.id == left.id || layout.id == right.id)
                || doc
                    .matrices
                    .iter()
                    .any(|item| item.id == left.matrix_id || item.id == right.matrix_id)
            {
                return Err(
                    "Mirrored pair requires two new layouts and distinct matrices on one board"
                        .into(),
                );
            }
            let mut changed = apply(
                doc,
                &[],
                &EditOperation::SetMatrix {
                    matrix: matrix.as_ref().clone(),
                    definitions: definitions.clone(),
                },
            )?;
            let mut target = matrix.as_ref().clone();
            target.id = right.matrix_id.clone();
            target.name = Some(right.name.clone());
            for cell in &mut target.cells {
                for assembly in &mut cell.assemblies {
                    assembly.offset.x = -assembly.offset.x;
                    assembly.rotation = assembly.rotation.map(|angle| -angle);
                }
            }
            let target =
                layout::reflected(matrix, &target, layout::axis_mapping(link.axis_x), false)?;
            changed.extend(matrix::set_matrix(doc, &target)?);
            doc.layouts.extend([left.clone(), right.clone()]);
            layout::validate(doc)?;
            changed.extend([left.id.clone(), right.id.clone()]);
            Ok(changed)
        }
        EditOperation::CreateMirroredHalf {
            pair_id,
            source_instance_id,
            mode,
            labels,
            companion_mode,
            companion_tweak,
            mapping,
            id_rule,
            mounting_hole,
        } => mirrored_half::create(
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
        EditOperation::SetLayout { layout: incoming } => {
            let current = doc
                .layouts
                .iter_mut()
                .find(|layout| layout.id == incoming.id)
                .ok_or("Layout does not exist")?;
            if current.board_id != incoming.board_id || current.matrix_id != incoming.matrix_id {
                return Err("Layout ownership cannot be reassigned".into());
            }
            let was_linked_target = current.mirror_link.is_some() && incoming.mirror_link.is_none();
            let source_layout_id = was_linked_target
                .then(|| {
                    current
                        .mirror_link
                        .as_ref()
                        .map(|link| link.source_id.clone())
                })
                .flatten();
            *current = incoming.clone();
            let mut detached = if was_linked_target {
                layout::unlink_components(doc, &incoming.id)
            } else {
                Vec::new()
            };
            if let Some(source_layout_id) = source_layout_id {
                detached.extend(layout::unlink_components(doc, &source_layout_id));
            }
            layout::resolve(doc)?;
            Ok([
                vec![incoming.id.clone(), incoming.matrix_id.clone()],
                detached,
            ]
            .concat())
        }
        EditOperation::SetConstraint { constraint } => {
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
            crate::mechanical::edit::apply(doc, board_contours, edit)
        }
        EditOperation::ApplyOutlineEdit { edit } => crate::outline_edit::apply(doc, edit),
        EditOperation::ReplaceDocument { document } => {
            if document.format != "boardstudio/v2" {
                return Err("Unsupported document format".into());
            }
            if document.format_version != migrate::CURRENT_VERSION {
                return Err("Document must be migrated before it replaces the open one".into());
            }
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
    }
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
