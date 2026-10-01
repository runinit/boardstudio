pub mod archive;
pub mod artifact;
mod case;
mod constraints;
pub mod electrical;
pub mod electrical_jumpers;
pub mod electrical_peripherals;
pub mod electrical_profiles;
pub mod firmware;
mod geometry;
mod inputs;
mod keycaps;
mod keymap;
mod matrix;
pub mod mechanical;
mod mechanical_checks;
pub mod model;
mod outline_controls;
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
                if let Some(map) = &document.keymap {
                    if let Err(message) = keymap::validate(map) {
                        return self.error(id, &message);
                    }
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
                self.document = document;
                self.undo.clear();
                self.redo.clear();
                self.recompute();
                self.scene(
                    id,
                    "open",
                    vec![],
                    &self.document,
                    &self.contours,
                    &self.findings,
                    &self.outline_cache,
                    SceneKind::Committed,
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
                "snapshot",
                vec![],
                &self.document,
                &self.contours,
                &self.findings,
                &self.outline_cache,
                SceneKind::Committed,
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
                        operation: EditOperation::ReplaceDocument { document },
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
                    "review-remap",
                    vec![],
                    &self.document,
                    &self.contours,
                    &self.findings,
                    &self.outline_cache,
                    SceneKind::Committed,
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
                    "protect-handoff",
                    vec![],
                    &self.document,
                    &self.contours,
                    &self.findings,
                    &self.outline_cache,
                    SceneKind::Committed,
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
            )
        {
            return self.preview_edit(id, command);
        }
        let mut next = self.document.clone();
        let changed = match apply(&mut next, &command.operation) {
            Ok(changed) => changed,
            Err(message) => return self.error(id, &message),
        };
        if let Some(map) = &next.keymap {
            if let Err(message) = keymap::validate(map) {
                return self.error(id, &message);
            }
        }
        let mut changed = changed;
        electrical::preserve_handoff(&self.document, &mut next);
        if let Err(message) = layout::validate(&next) {
            return self.error(id, &message);
        }
        match constraints::resolve(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match layout::sync_components(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match constraints::resolve(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        match layout::sync_components(&mut next) {
            Ok(ids) => changed.extend(ids),
            Err(message) => return self.error(id, &message),
        }
        outline_controls::detach_removed(&self.document.parts, &mut next);
        changed.sort();
        changed.dedup();
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
                &command.transaction_id,
                changed,
                &next,
                &contours,
                &findings,
                &cache,
                SceneKind::Preview,
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
            &command.transaction_id,
            changed,
            &self.document,
            &self.contours,
            &self.findings,
            &self.outline_cache,
            SceneKind::Committed,
        )
    }

    fn preview_edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        let backup = PreviewBackup::capture(&self.document, &command.operation);
        let attachments = outline_controls::attached_parts(&self.document);
        let result = apply(&mut self.document, &command.operation).and_then(|mut changed| {
            layout::validate(&self.document)?;
            changed.extend(constraints::resolve(&mut self.document)?);
            changed.extend(layout::sync_components(&mut self.document)?);
            changed.extend(constraints::resolve(&mut self.document)?);
            changed.extend(layout::sync_components(&mut self.document)?);
            outline_controls::detach_removed(&attachments, &mut self.document);
            changed.sort();
            changed.dedup();
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
                    &command.transaction_id,
                    changed,
                    &self.document,
                    &contours,
                    &findings,
                    &cache,
                    SceneKind::Preview,
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
            transaction,
            changed,
            &self.document,
            &self.contours,
            &self.findings,
            &self.outline_cache,
            SceneKind::Committed,
        )
    }

    fn recompute(&mut self) {
        (self.outline_cache, self.contours, self.findings) = outlines(&self.document, None, &[]);
        outline_versions::refresh_recovery(&mut self.document, &self.outline_cache);
    }

    fn scene(
        &self,
        id: String,
        transaction: &str,
        changed: Vec<String>,
        doc: &ProjectDoc,
        contours: &[Contour],
        geom_findings: &[Finding],
        cache: &OutlineCache,
        kind: SceneKind,
    ) -> CoreReply {
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
        let board_outline_scenes = geometry::board_outline_scenes(doc, cache);
        let (outline_findings, mut finding_markers) =
            outline_validation::validate(doc, &board_contours, &board_outline_scenes);
        findings.extend(outline_findings);
        finding_markers.extend(inputs::finding_markers(doc, &findings));
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
        let layout = !findings
            .iter()
            .any(|f| matches!(f.scope, Scope::Layout) && matches!(f.severity, Severity::Error));
        let outline = !contours.is_empty()
            && !findings.iter().any(|f| {
                matches!(f.scope, Scope::Outline) && matches!(f.severity, Severity::Error)
            });
        let pcb = layout
            && !doc.boards.is_empty()
            && !findings
                .iter()
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
                let invalid_pcb = findings.iter().any(|finding| {
                    matches!(finding.severity, Severity::Error)
                        && matches!(finding.scope, Scope::Pcb)
                        && relevant(finding)
                });
                let invalid_outline = findings.iter().any(|finding| {
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
                let valid_layout = !findings.iter().any(|finding| {
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
                    matrix::project_matrix(matrix, &doc.parts, matrix::ProjectionMode::Actual).ok()
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
                document: doc.clone(),
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

enum PreviewBackup {
    Parts(
        Vec<(
            usize,
            Pose2,
            Option<std::collections::BTreeMap<String, serde_json::Value>>,
        )>,
    ),
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
    matches!(
        op,
        EditOperation::MoveParts { .. }
            | EditOperation::SetOutline { .. }
            | EditOperation::CopyOutline { .. }
            | EditOperation::SelectOutline { .. }
            | EditOperation::RemoveOutline { .. }
            | EditOperation::AddPart { .. }
            | EditOperation::RemoveParts { .. }
            | EditOperation::RemoveMatrix { .. }
            | EditOperation::SetMatrix { .. }
            | EditOperation::SetMatrixSplay { .. }
            | EditOperation::CreateMirroredPair { .. }
            | EditOperation::SetLayout { .. }
            | EditOperation::SetConstraint { .. }
            | EditOperation::RemoveConstraint { .. }
            | EditOperation::ReplaceDocument { .. }
    )
}

fn apply(doc: &mut ProjectDoc, op: &EditOperation) -> Result<Vec<String>, String> {
    match op {
        EditOperation::SetInputScanMode { part_id, mode } => {
            inputs::set_scan_mode(doc, part_id, *mode)
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
                    match constraint {
                        Constraint::Offset { offset, .. } => {
                            offset.x += delta.x;
                            offset.y += delta.y;
                            changed.push(position.id.clone());
                            continue;
                        }
                        Constraint::Mirror { .. } => {
                            return Err(format!(
                                "Part {} is controlled by a mirror constraint and cannot be dragged",
                                position.id
                            ));
                        }
                    }
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
                if let OutlineFeature::PartEnvelope { id, part_ids, .. } = feature {
                    if board_outline.contains(id) {
                        part_ids.push(part.id.clone());
                        attached = true;
                    }
                }
            }
            if !attached && doc.boards.is_empty() {
                if let Some(OutlineFeature::PartEnvelope { part_ids, .. }) = doc
                    .outline
                    .iter_mut()
                    .find(|feature| matches!(feature, OutlineFeature::PartEnvelope { .. }))
                {
                    part_ids.push(part.id.clone());
                }
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
            let mut changed = apply(doc, &EditOperation::RemoveParts { ids })?;
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
            let mut changed = ids;
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
            if let Some(board) = doc.boards.first_mut() {
                if !board.net_ids.contains(&net.id) {
                    board.net_ids.push(net.id.clone());
                }
            }
            Ok(vec![net.id.clone()])
        }
        EditOperation::SetMechanical { configuration } => {
            doc.mechanical = configuration.clone();
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
            let mut changed = matrix::set_matrix(doc, &matrix)?;
            changed.extend(layout::sync(doc, matrix_id)?);
            Ok(changed)
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
                &EditOperation::SetMatrix {
                    matrix: matrix.clone(),
                    definitions: definitions.clone(),
                },
            )?;
            let mut target = matrix.clone();
            target.id = right.matrix_id.clone();
            target.name = Some(right.name.clone());
            for cell in &mut target.cells {
                for assembly in &mut cell.assemblies {
                    assembly.offset.x = -assembly.offset.x;
                    assembly.rotation = assembly.rotation.map(|angle| -angle);
                }
            }
            let target = layout::reflected(matrix, &target, link.axis_x)?;
            changed.extend(matrix::set_matrix(doc, &target)?);
            doc.layouts.extend([left.clone(), right.clone()]);
            layout::validate(doc)?;
            changed.extend([left.id.clone(), right.id.clone()]);
            Ok(changed)
        }
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
        EditOperation::ReplaceDocument { document } => {
            if document.format != "boardstudio/v2" {
                return Err("Unsupported document format".into());
            }
            let changed = changed_ids(doc, document);
            let mut prepared = document.clone();
            script::apply_scripts(&mut prepared)?;
            layout::resolve(&mut prepared)?;
            *doc = prepared;
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
