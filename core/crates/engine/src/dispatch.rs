use super::apply::{affects_outline, apply_with_ledger, validate_project_name};
use super::preview::{changed_ids, preview_electrical_loss_findings};
use super::scene_reuse::{
    SettledStagger, StaggerPreview, ValidatedBoardScene, is_column_stagger,
    replacement_keeps_scene, settled_outline_changes,
};
use super::*;

#[cfg(test)]
#[path = "scene_reuse/tests.rs"]
mod scene_reuse_tests;

impl CoreEngine {
    /// Validate a typed Open candidate before retiring editor work, without changing Core.
    pub fn validate_open(document: &ProjectDoc) -> Result<(), String> {
        Self::prepare_open(document).map(|_| ())
    }

    fn prepare_open(document: &ProjectDoc) -> Result<ProjectDoc, String> {
        if document.format != "boardstudio/v2" {
            return Err("Unsupported document format".into());
        }
        document_format::require_current_version(document.format_version)?;
        if document.revision == u64::MAX {
            return Err("Document revision is exhausted".into());
        }
        if let Some(message) = validate::duplicate_top_level_identity(document) {
            return Err(message);
        }
        if document
            .layouts
            .iter()
            .any(|layout| layout.mirror_link.is_some())
        {
            return Err("Layout mirror links are unsupported; use a mirrored pair".into());
        }
        mirrored_half::validate_pairs(document)?;
        let mut canonical = document.clone();
        canonical.scripts.clear();
        layout::resolve(&mut canonical)?;
        constraints::resolve(&mut canonical)?;
        generators::document_error(&canonical)?;
        if let Some(map) = &canonical.keymap {
            keymap::validate(map)?;
        }
        let resolved = mirrored_half::project_document(&canonical);
        if let Some(message) = validate::duplicate_top_level_identity(resolved.geometry()) {
            return Err(message);
        }
        Ok(canonical)
    }

    pub fn handle(&mut self, request: CoreRequest) -> CoreReply {
        match request {
            CoreRequest::ResolveModules {
                id,
                document,
                board_id,
                preview_top_z,
            } => {
                let resolved = mirrored_half::project_document(&document);
                let document = resolved.geometry();
                let mut result = modules::resolve(document, &board_id);
                match modules::prepare_preview(
                    document,
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
                result: keycaps::resolve(
                    mirrored_half::project_document(&document).geometry(),
                    &board_id,
                    cases.as_ref(),
                ),
            },
            CoreRequest::GenerateFirmware { id, request } => match firmware::generate(&request) {
                Ok(package) => CoreReply::FirmwareGenerated { id, package },
                Err(message) => self.error(id, &message),
            },
            CoreRequest::Open { id, document } => {
                self.move_preview = None;
                let document = match Self::prepare_open(&document) {
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
                // Open publishes the recomputed projection unchanged, so it is cacheable.
                self.project_committed_scene(id, "open", vec![], true, None)
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
                assembly: mechanical::resolve(
                    mirrored_half::project_document(&document).geometry(),
                    &contours,
                ),
            },
            CoreRequest::PrepareCase { id, ir } => match case::prepare(&ir) {
                Ok(ir) => CoreReply::CasePrepared { id, ir },
                Err(message) => CoreReply::Error {
                    id,
                    message,
                    revision: ir.revision,
                },
            },
            CoreRequest::Snapshot { id } => {
                self.scene_with_unchanged_geometry(id, "snapshot", vec![], SceneKind::Committed)
            }
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
            CoreRequest::ResolveElectrical { id, mut request } => {
                request.document = mirrored_half::project_document(&request.document)
                    .geometry()
                    .clone();
                CoreReply::ElectricalResolved {
                    id,
                    plan: electrical::resolve(request),
                }
            }
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
                    document: self.resolved.geometry().clone(),
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
                self.reusable_scene = None;
                self.stagger_preview = None;
                self.move_preview = None;
                self.document.revision = revision;
                self.refresh_resolved();
                self.scene(
                    id,
                    self.accepted_scene_input("review-remap", vec![], SceneKind::Committed),
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
                    document: self.resolved.geometry().clone(),
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
                self.reusable_scene = None;
                self.stagger_preview = None;
                self.move_preview = None;
                board.protected_handoff = Some(ElectricalHandoffBaseline {
                    fingerprint: plan.fingerprint.clone(),
                    revision: self.document.revision,
                    assignments,
                });
                self.refresh_resolved();
                self.scene(
                    id,
                    self.accepted_scene_input("protect-handoff", vec![], SceneKind::Committed),
                )
            }
        }
    }

    fn edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        let move_preview = self
            .move_preview
            .take()
            .filter(|preview| command.phase == EditPhase::Commit && preview.matches(&command));
        if command.base_revision != self.document.revision {
            return self.error(id, "Stale base revision");
        }
        if let EditOperation::RenameProject { name } = command.operation {
            return self.rename_project(id, command.transaction_id, command.phase, name);
        }
        if command.phase == EditPhase::Commit
            && let Some(settled) = self
                .stagger_preview
                .as_ref()
                .and_then(|preview| preview.settled.as_ref())
            && settled.matches(&command.operation)
            && settled.document != self.document
        {
            // The settled Preview already applied and validated this pose-only edit.
            // Commit additionally checks the keymap, and publishes through normal history.
            if let Some(map) = &settled.document.keymap
                && let Err(message) = keymap::validate(map)
            {
                return self.error(id, &message);
            }
            let mut preview = self.stagger_preview.take().unwrap();
            let SettledStagger {
                document,
                mut scene,
                ..
            } = preview.settled.take().unwrap();
            return self.publish_prepared_edit(
                id,
                PreparedEdit {
                    transaction: command.transaction_id,
                    document,
                    changed: std::mem::take(&mut scene.changed_ids),
                    outline: Some(preview.projection().0),
                    board_geometry: None,
                    preview_scene: Some(scene),
                    reuse_scene: false,
                    cacheable: true,
                },
            );
        }
        // A wholesale replacement is validated before layout can normalize
        // the incoming identities; publication validates its generated result again.
        if let EditOperation::ReplaceDocument { document } = &command.operation
            && let Some(message) = validate::duplicate_top_level_identity(document)
        {
            return self.error(id, &message);
        }
        if let Err(refusal) = mirrored_half::admit_edit(&self.document, &command.operation) {
            return self.error(id, &refusal.to_string());
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
        let (mut next, mut changed, layout_settled) =
            match self.settle_edit_draft(&command.operation, false) {
                Ok(prepared) => prepared,
                Err(message) => return self.error(id, &message),
            };
        // Ticket 10: every committed edit that changed the document re-runs the
        // adaptive spacing pass. It runs after resolve so the minimum sees the
        // final geometry, and before the no-op check so a genuine edit that
        // only moves the halves still publishes. Skipped when the edit changed
        // nothing, so no-op commits stay no-ops. Same Undo entry: the spacing
        // lands in `next` before the undo snapshot is pushed. Creation owns
        // its own gap (ticket 11). The settled stagger route excludes linked
        // layouts, so there is no half-frame or spacing adjustment to run.
        let mut spacing_changed = false;
        if command.phase == EditPhase::Commit
            && !layout_settled
            && next != self.document
            && !matches!(
                command.operation,
                EditOperation::CreateMirroredPair { .. } | EditOperation::CreateMirroredHalf { .. }
            )
        {
            let mut adapted = match layout::adapt_spacing(&mut next, &self.document) {
                Ok(adapted) => adapted,
                Err(message) => return self.error(id, &message),
            };
            spacing_changed = !adapted.is_empty();
            if spacing_changed {
                if let Err(message) = layout::resolve(&mut next) {
                    return self.error(id, &message);
                }
                changed.append(&mut adapted);
                changed.sort();
                changed.dedup();
            }
        }
        if command.phase == EditPhase::Commit
            && next == self.document
            && !matches!(command.operation, EditOperation::ReplaceDocument { .. })
        {
            return self.scene(
                id,
                self.accepted_scene_input(&command.transaction_id, changed, SceneKind::Committed),
            );
        }
        let reuse_scene = command.phase == EditPhase::Commit
            && self.reusable_scene.is_some()
            && matches!(command.operation, EditOperation::ReplaceDocument { .. })
            && replacement_keeps_scene(&self.document, &mut next);
        if let Err(message) = mirrored_half::validate_pairs(&next) {
            return self.error(id, &message);
        }
        let resolved = match self.project_settled_draft(&next, &mut changed) {
            Ok(resolved) => resolved,
            Err(message) => return self.error(id, &message),
        };
        let move_preview = move_preview.filter(|_| {
            !spacing_changed
                && self.document.mirrored_pairs.is_empty()
                && move_scene_can_publish(&next)
        });
        let (outline, board_geometry, preview_scene) = if let Some(preview) = move_preview {
            (Some(preview.outline), None, Some(preview.scene))
        } else if !next.mirrored_pairs.is_empty() || !self.document.mirrored_pairs.is_empty() {
            (Some(outlines(resolved.geometry(), None, &[])), None, None)
        } else if reuse_scene || !affects_outline(&command.operation) {
            (None, None, None)
        } else {
            let (outline, board_geometry) = self.edited_outlines(&next, &command, layout_settled);
            (Some(outline), board_geometry, None)
        };
        if command.phase == EditPhase::Preview {
            let accepted = (&self.outline_cache, &self.contours, &self.findings);
            let (cache, contours, findings) = outline
                .as_ref()
                .map_or(accepted, |(cache, contours, findings)| {
                    (cache, contours, findings)
                });
            let mut findings = findings.clone();
            findings.extend(preview_electrical_loss_findings(
                self.resolved.geometry(),
                resolved.geometry(),
            ));
            return self.scene_with_board_geometry(
                id,
                SceneInput {
                    transaction: &command.transaction_id,
                    changed,
                    document: resolved.geometry(),
                    contours,
                    geom_findings: &findings,
                    cache,
                    kind: SceneKind::Preview,
                },
                board_geometry,
            );
        }
        self.publish_prepared_edit(
            id,
            PreparedEdit {
                transaction: command.transaction_id,
                document: next,
                changed,
                outline,
                board_geometry,
                preview_scene,
                reuse_scene,
                cacheable: !matches!(command.operation, EditOperation::MoveParts { .. }),
            },
        )
    }

    fn rename_project(
        &mut self,
        id: String,
        transaction: String,
        phase: EditPhase,
        name: String,
    ) -> CoreReply {
        if let Err(message) = validate_project_name(&name) {
            return self.error(id, &message);
        }
        let name_changed = self.document.name != name;
        let changed = if name_changed && self.document.mirrored_pairs.is_empty() {
            // Non-prepared replacements report every Part and Outline ID. The
            // native-pair path diffs individual entities, so a rename reports none.
            changed_ids(&self.document, &self.document)
        } else {
            vec![]
        };
        let kind = match phase {
            EditPhase::Preview => SceneKind::Preview,
            EditPhase::Commit => {
                if name_changed {
                    let Some(revision) = self.document.revision.checked_add(1) else {
                        return self.error(id, "Document revision is exhausted");
                    };
                    self.record_undo(self.document.clone());
                    self.document.name = name;
                    self.document.revision = revision;
                    self.refresh_resolved();
                }
                SceneKind::Committed
            }
        };
        // The accepted geometry, findings and identity ledger have no project-name dependency.
        self.scene_with_unchanged_geometry(id, &transaction, changed, kind)
    }

    fn accepted_scene_input<'a>(
        &'a self,
        transaction: &'a str,
        changed: Vec<String>,
        kind: SceneKind,
    ) -> SceneInput<'a> {
        SceneInput {
            transaction,
            changed,
            document: self.resolved.geometry(),
            contours: &self.contours,
            geom_findings: &self.findings,
            cache: &self.outline_cache,
            kind,
        }
    }

    fn scene_with_unchanged_geometry(
        &self,
        id: String,
        transaction: &str,
        changed: Vec<String>,
        kind: SceneKind,
    ) -> CoreReply {
        if let Some(mut scene) = self.reusable_scene.clone() {
            scene.revision = self.document.revision;
            transaction.clone_into(&mut scene.transaction_id);
            scene.changed_ids = changed;
            return match kind {
                SceneKind::Preview => CoreReply::Preview { id, scene },
                SceneKind::Committed => CoreReply::Scene {
                    id,
                    scene,
                    document: Box::new(self.document.clone()),
                },
            };
        }
        self.scene(id, self.accepted_scene_input(transaction, changed, kind))
    }

    fn record_undo(&mut self, previous: ProjectDoc) {
        self.stagger_preview = None;
        self.move_preview = None;
        // Keep the latest 100 committed edits without retaining an unbounded document history.
        if self.undo.len() == 100 {
            self.undo.remove(0);
        }
        self.undo.push(previous);
        self.redo.clear();
    }

    fn publish_prepared_edit(&mut self, id: String, prepared: PreparedEdit) -> CoreReply {
        let PreparedEdit {
            transaction,
            document: mut next,
            changed,
            outline,
            board_geometry,
            preview_scene,
            reuse_scene,
            cacheable,
        } = prepared;
        if let Some(message) = validate::duplicate_top_level_identity(&next) {
            return self.error(id, &message);
        }
        let Some(revision) = self.document.revision.checked_add(1) else {
            return self.error(id, "Document revision is exhausted");
        };
        next.revision = revision;
        if !reuse_scene {
            let cache = outline
                .as_ref()
                .map_or(&self.outline_cache, |(cache, _, _)| cache);
            outline_versions::refresh_recovery(&mut next, cache);
        }
        let previous = std::mem::replace(&mut self.document, next);
        self.record_undo(previous);
        self.refresh_resolved();
        self.identity_ledger.capture(&self.document);
        if reuse_scene {
            return self.scene_with_unchanged_geometry(
                id,
                &transaction,
                changed,
                SceneKind::Committed,
            );
        }
        let rebuilt = outline.is_some();
        if let Some((cache, contours, findings)) = outline {
            self.outline_cache = cache;
            self.contours = contours;
            self.findings = findings;
        }
        if let Some(mut scene) = preview_scene {
            scene.revision = revision;
            scene.transaction_id = transaction;
            scene.changed_ids = changed;
            self.reusable_scene = cacheable.then(|| scene.clone());
            return CoreReply::Scene {
                id,
                scene,
                document: Box::new(self.document.clone()),
            };
        }
        self.project_committed_scene(
            id,
            &transaction,
            changed,
            cacheable && rebuilt,
            board_geometry,
        )
    }

    fn apply_layout_edit(
        &self,
        draft: &mut ProjectDoc,
        board_contours: &[BoardContours],
        operation: &EditOperation,
    ) -> Result<(Vec<String>, bool), String> {
        if let EditOperation::SetMatrixPlacement {
            matrix_id,
            change: MatrixPlacementChange::ColumnStagger { column, value },
        } = operation
            && self.reusable_scene.as_ref().is_some_and(|scene| {
                scene.matrix_scenes.len() == draft.matrices.len()
                    && scene
                        .matrix_scenes
                        .iter()
                        .any(|matrix| matrix.matrix_id == *matrix_id)
            })
            && let Some(changed) = matrix::placement::try_settled_column_stagger(
                draft,
                &self.document.parts,
                matrix_id,
                *column,
                *value,
            )
        {
            return Ok((changed, true));
        }
        apply_with_ledger(draft, board_contours, operation, &self.identity_ledger)
            .map(|changed| (changed, false))
    }

    /// Apply an operation to a fresh draft and settle layout dependents.
    ///
    /// Both commit (`edit`) and drag-preview (`preview_edit`) settlement flow
    /// through here. Previews skip keymap validation since they never publish,
    /// and preserve the electrical handoff only for part moves (other previews
    /// must materialize the same dependant poses the commit path publishes
    /// without carrying handoff state). Generator checks run on every preview,
    /// even settled ones, while commits skip them for settled or wholesale
    /// replacements (`ReplaceDocument` is judged by its own arm in `apply`,
    /// which allows warnings).
    fn settle_edit_draft(
        &self,
        operation: &EditOperation,
        preview: bool,
    ) -> Result<(ProjectDoc, Vec<String>, bool), String> {
        let mut draft = self.document.clone();
        // Mechanical edits resolve against the accepted outline projection the user
        // sees, not a stale or re-derived one, so the accepted cache supplies it.
        let accepted_contours;
        let board_contours: &[BoardContours] =
            if matches!(operation, EditOperation::ApplyMechanicalEdit { .. }) {
                accepted_contours = geometry::board_contours(&self.document, &self.outline_cache).0;
                &accepted_contours
            } else {
                &[]
            };
        let (mut changed, layout_settled) =
            self.apply_layout_edit(&mut draft, board_contours, operation)?;
        if !preview
            && let Some(map) = &draft.keymap
            && let Err(message) = keymap::validate(map)
        {
            return Err(message);
        }
        if !preview || matches!(operation, EditOperation::MoveParts { .. }) {
            electrical::preserve_handoff(&self.document, &mut draft);
        }
        if !layout_settled {
            layout::resolve(&mut draft)?;
            settle_layout_dependents(
                &mut draft,
                operation,
                &mut changed,
                self.reusable_scene.is_some(),
            )?;
            // The settled stagger path preserves part identities.
            outline_controls::detach_removed(&self.document.parts, &mut draft);
            changed.extend(outline_versions::prune_removed_components(
                &self.document.parts,
                &mut draft,
            ));
        }
        changed.sort();
        changed.dedup();
        if preview
            || (!layout_settled && !matches!(operation, EditOperation::ReplaceDocument { .. }))
        {
            generators::introduced_error(&generators::parameter_problems(&self.document), &draft)?;
        }
        matrix::placement::validate_translation(&self.document.parts, &draft, operation)?;
        Ok((draft, changed, layout_settled))
    }

    /// Project a settled draft and extend the change list with affected mirror targets.
    fn project_settled_draft(
        &self,
        draft: &ProjectDoc,
        changed: &mut Vec<String>,
    ) -> Result<mirrored_half::ResolvedDoc, String> {
        let resolved = mirrored_half::project_document(draft);
        let affected: Vec<_> = self
            .resolved
            .affected_targets(changed)
            .chain(resolved.affected_targets(changed))
            .cloned()
            .collect();
        changed.extend(affected);
        changed.sort();
        changed.dedup();
        if let Some(message) = validate::duplicate_top_level_identity(resolved.geometry()) {
            return Err(message);
        }
        Ok(resolved)
    }

    fn preview_edit(&mut self, id: String, command: EditCommand) -> CoreReply {
        // Layout and constraint settlement may rewrite anchors, change
        // membership, and prune outline versions, even for a single drag.
        let (draft, mut changed, layout_settled) =
            match self.settle_edit_draft(&command.operation, true) {
                Ok(prepared) => prepared,
                Err(message) => return self.error(id, &message),
            };
        let resolved_draft = match self.project_settled_draft(&draft, &mut changed) {
            Ok(resolved) => resolved,
            Err(message) => return self.error(id, &message),
        };
        let ((cache, contours, mut findings), board_geometry) =
            self.edited_outlines(resolved_draft.geometry(), &command, layout_settled);
        if !draft.mirrored_pairs.is_empty() {
            findings.extend(preview_electrical_loss_findings(
                self.resolved.geometry(),
                resolved_draft.geometry(),
            ));
        }
        let reply = self.scene_with_board_geometry(
            id,
            SceneInput {
                transaction: &command.transaction_id,
                changed,
                document: resolved_draft.geometry(),
                contours: &contours,
                geom_findings: &findings,
                cache: &cache,
                kind: SceneKind::Preview,
            },
            board_geometry,
        );
        if matches!(command.operation, EditOperation::MoveParts { .. })
            && self.document.mirrored_pairs.is_empty()
            && move_scene_can_publish(&draft)
            && let CoreReply::Preview { scene, .. } = &reply
        {
            self.move_preview = Some(MovePreview {
                command,
                scene: scene.clone(),
                outline: (cache, contours, findings),
            });
        } else if layout_settled
            && let Some(preview) = &mut self.stagger_preview
            && preview.matches(&draft)
            && let CoreReply::Preview { scene, .. } = &reply
        {
            preview.settled = SettledStagger::new(&command.operation, draft, scene);
        }
        reply
    }

    fn edited_outlines(
        &mut self,
        draft: &ProjectDoc,
        command: &EditCommand,
        layout_settled: bool,
    ) -> (geometry::OutlineProjection, Option<ValidatedBoardScene>) {
        if let Some(moved) = self
            .reusable_scene
            .as_ref()
            .and_then(|_| settled_outline_changes(&self.document, draft, &command.operation))
        {
            if layout_settled && is_column_stagger(&command.operation) {
                return self.settled_stagger_projection(draft, &moved, command.phase);
            }
            return (outlines(draft, Some(&self.outline_cache), &moved), None);
        }
        (outlines(draft, None, &[]), None)
    }

    fn settled_stagger_projection(
        &mut self,
        draft: &ProjectDoc,
        moved: &[String],
        phase: EditPhase,
    ) -> (geometry::OutlineProjection, Option<ValidatedBoardScene>) {
        let preview = phase == EditPhase::Preview;
        let previous = match self.stagger_preview.take() {
            Some(previous) if previous.matches(draft) => {
                if !preview {
                    return previous.projection();
                }
                let projection = previous.clone_projection();
                self.stagger_preview = Some(previous);
                return projection;
            }
            other => other,
        };
        #[cfg(test)]
        self.stagger_outline_projections
            .set(self.stagger_outline_projections.get() + 1);
        let mut envelopes = previous
            .map(|previous| previous.envelopes)
            .unwrap_or_default();
        let (outline, board) =
            geometry::outlines_with_board_reuse(draft, &self.outline_cache, moved, &mut envelopes);
        let board = board.map(|board| ValidatedBoardScene::new(draft, board));
        // Shared board geometry rules out saved versions and generated
        // recovery from Keep gap, which publication could otherwise refresh.
        if preview && let Some(board) = &board {
            self.stagger_preview = Some(StaggerPreview::new(
                draft,
                outline.clone(),
                board.clone(),
                envelopes,
            ));
        }
        (outline, board)
    }

    fn history(&mut self, id: String, direction: History) -> CoreReply {
        self.move_preview = None;
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
        let mut changed = changed_ids(&self.document, &next);
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
        let targets: Vec<_> = self.resolved.affected_targets(&changed).cloned().collect();
        changed.extend(targets);
        changed.sort();
        changed.dedup();
        let transaction = match direction {
            History::Undo => "undo",
            History::Redo => "redo",
        };
        self.project_committed_scene(id, transaction, changed, true, None)
    }

    fn project_committed_scene(
        &mut self,
        id: String,
        transaction: &str,
        changed: Vec<String>,
        cacheable: bool,
        board_geometry: Option<ValidatedBoardScene>,
    ) -> CoreReply {
        let reply = self.scene_with_board_geometry(
            id,
            self.accepted_scene_input(transaction, changed, SceneKind::Committed),
            board_geometry,
        );
        // Recovery and linked alternatives can select snapshots that differ from
        // the inputs used to build the outline cache. Keep those on the full path.
        self.reusable_scene = match &reply {
            CoreReply::Scene { scene, .. }
                if cacheable && self.document.board_outlines.is_empty() =>
            {
                Some(scene.clone())
            }
            _ => None,
        };
        reply
    }

    fn recompute(&mut self) {
        self.stagger_preview = None;
        self.refresh_resolved();
        (self.outline_cache, self.contours, self.findings) =
            outlines(self.resolved.geometry(), None, &[]);
        outline_versions::refresh_recovery(&mut self.document, &self.outline_cache);
        self.refresh_resolved();
    }

    fn scene(&self, id: String, input: SceneInput<'_>) -> CoreReply {
        self.scene_with_board_geometry(id, input, None)
    }

    fn scene_with_board_geometry(
        &self,
        id: String,
        input: SceneInput<'_>,
        board_geometry: Option<ValidatedBoardScene>,
    ) -> CoreReply {
        #[cfg(test)]
        self.full_scene_projections
            .set(self.full_scene_projections.get() + 1);
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
        let validate::SceneDiagnostics {
            findings: document_findings,
            hardware_markers,
            keycaps,
            modules: module_scenes,
            matrices: mut matrix_scenes,
        } = validate::scene_diagnostics(doc);
        findings.extend(document_findings);
        let ValidatedBoardScene {
            geometry:
                geometry::BoardSceneGeometry {
                    contours: board_contours,
                    outlines: board_outline_scenes,
                    findings: board_findings,
                    markers: corner_markers,
                },
            findings: outline_findings,
            markers: mut finding_markers,
        } = board_geometry.unwrap_or_else(|| {
            #[cfg(test)]
            self.independent_board_projections
                .set(self.independent_board_projections.get() + 1);
            ValidatedBoardScene::new(doc, geometry::board_scene_geometry(doc, cache))
        });
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
        findings.extend(outline_findings);
        finding_markers.extend(inputs::finding_markers(doc, &findings));
        finding_markers.extend(hardware_markers);
        finding_markers.extend(modules::finding_markers(doc, &findings));
        let mut generic_markers: BTreeMap<(String, String), Vec<Contour>> = BTreeMap::new();
        for marker in outline_validation::feature_markers(doc, &findings, &finding_markers) {
            generic_markers
                .entry((marker.finding_id, marker.board_id))
                .or_default()
                .extend(marker.contours);
        }
        for ((finding_id, board_id), contours) in generic_markers {
            finding_markers.push(FindingMarker {
                finding_id,
                board_id,
                contours,
            });
        }
        finding_markers.extend(corner_markers);
        for (board, keycaps) in doc.boards.iter().zip(keycaps) {
            finding_markers.extend(keycaps.finding_markers(&board.id, &findings));
        }
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
        let (readiness, board_readiness) =
            compute_readiness(doc, contours, &board_contours, &findings);
        let scene = SceneDelta {
            module_scenes,
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
                .into_iter()
                .filter_map(|index| matrix_scenes[index].take())
                .collect(),
            contours: contours.to_vec(),
            board_contours,
            board_readiness,
            board_outline_scenes,
            finding_markers,
            findings,
            readiness,
        };
        match kind {
            SceneKind::Preview => CoreReply::Preview { id, scene },
            SceneKind::Committed => CoreReply::Scene {
                id,
                scene,
                document: Box::new(self.document.clone()),
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

/// Pure export-readiness projection over the assembled scene findings.
fn compute_readiness(
    doc: &ProjectDoc,
    contours: &[Contour],
    board_contours: &[BoardContours],
    findings: &[Finding],
) -> (Readiness, Vec<BoardReadiness>) {
    // A finding attributed to a linked alternative the board has not selected is
    // visible but never blocks the selected outline or another board (ADR-0001/0002).
    let active =
        |finding: &Finding| !outline_versions::is_inactive_version_finding(doc, &finding.id);
    let layout = !findings
        .iter()
        .filter(|finding| active(finding))
        .any(|f| matches!(f.scope, Scope::Layout) && matches!(f.severity, Severity::Error));
    let outline = !contours.is_empty()
        && !findings
            .iter()
            .filter(|finding| active(finding))
            .any(|f| matches!(f.scope, Scope::Outline) && matches!(f.severity, Severity::Error));
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
            let invalid_pcb = findings
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
            let valid_layout = !findings
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
    (
        Readiness {
            layout,
            outline,
            pcb,
            case_ready,
        },
        board_readiness,
    )
}

fn settle_layout_dependents(
    doc: &mut ProjectDoc,
    operation: &EditOperation,
    changed: &mut Vec<String>,
    accepted_projection: bool,
) -> Result<(), String> {
    // Unlinked, unanchored parts with no constraint or assembly driver keep
    // their stored poses. The solver and link sync would have nothing to write.
    if accepted_projection
        && is_column_stagger(operation)
        && doc.constraints.is_empty()
        && doc.parts.iter().all(|part| part.anchor.is_none())
        && doc
            .matrices
            .iter()
            .all(|matrix| matrix.cells.iter().all(|cell| cell.assemblies.is_empty()))
    {
        return Ok(());
    }
    changed.extend(constraints::resolve_edit(doc, operation)?);
    Ok(())
}

enum History {
    Undo,
    Redo,
}

pub(super) struct MovePreview {
    command: EditCommand,
    scene: SceneDelta,
    outline: geometry::OutlineProjection,
}

impl MovePreview {
    fn matches(&self, commit: &EditCommand) -> bool {
        let (
            EditOperation::MoveParts { positions },
            EditOperation::MoveParts { positions: preview },
        ) = (&commit.operation, &self.command.operation)
        else {
            return false;
        };
        self.command.base_revision == commit.base_revision
            && self.command.transaction_id == commit.transaction_id
            && self.command.target_ids == commit.target_ids
            && positions
                .iter()
                .map(position_bits)
                .eq(preview.iter().map(position_bits))
    }
}

fn position_bits(position: &Position) -> (&str, u64, u64) {
    (
        &position.id,
        position.at.x.to_bits(),
        position.at.y.to_bits(),
    )
}

fn move_scene_can_publish(doc: &ProjectDoc) -> bool {
    // Linked electrical findings are preview-only. Protected gaps can refresh
    // saved recovery during publication and change the board projection.
    doc.mirrored_pairs.is_empty()
        && doc.board_outlines.is_empty()
        && !doc.outline.iter().any(|feature| {
            matches!(feature, OutlineFeature::PartEnvelope { settings, .. }
                if settings.repair.as_ref().is_some_and(|repair| !repair.keep_gaps.is_empty()))
        })
}

struct PreparedEdit {
    transaction: String,
    document: ProjectDoc,
    changed: Vec<String>,
    outline: Option<geometry::OutlineProjection>,
    board_geometry: Option<ValidatedBoardScene>,
    preview_scene: Option<SceneDelta>,
    reuse_scene: bool,
    cacheable: bool,
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
