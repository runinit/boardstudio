use super::apply::{
    affects_outline, apply_with_ledger, edit_dispatch, has_native_mirror_pair,
    uses_prepared_transaction, validate_project_name,
};
use super::preview::{changed_ids, preview_electrical_loss_findings};
use super::scene_reuse::{is_column_stagger, replacement_keeps_scene, settled_outline_changes};
use super::*;

#[cfg(test)]
#[path = "scene_reuse/tests.rs"]
mod scene_reuse_tests;

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
                let cacheable = repaired_findings.is_empty();
                self.findings.extend(repaired_findings);
                self.project_committed_scene(id, "open", vec![], cacheable)
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
                self.reusable_scene = None;
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
                self.reusable_scene = None;
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
        if let EditOperation::RenameProject { name } = command.operation {
            return self.rename_project(id, command.transaction_id, command.phase, name);
        }
        // A wholesale replacement is validated before layout can normalize
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
                edit_dispatch(),
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
            if let Err(message) = settle_layout_dependents(
                &mut raw_next,
                &command.operation,
                &mut raw_changed,
                self.reusable_scene.is_some(),
            ) {
                return self.error(id, &message);
            }
            outline_controls::detach_removed(&self.document.parts, &mut raw_next);
            raw_changed.extend(outline_versions::prune_removed_components(
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
            let mut adapted = match layout::adapt_spacing(&mut next, &self.document) {
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
        let reuse_scene = command.phase == EditPhase::Commit
            && self.reusable_scene.is_some()
            && matches!(command.operation, EditOperation::ReplaceDocument { .. })
            && replacement_keeps_scene(&self.document, &mut next);
        let outline = if reuse_scene || (!prepared_route && !affects_outline(&command.operation)) {
            None
        } else if prepared_route {
            Some(outlines(&next, None, &[]))
        } else {
            Some(self.edited_outlines(&next, &command.operation))
        };
        if command.phase == EditPhase::Preview {
            let accepted = (&self.outline_cache, &self.contours, &self.findings);
            let (cache, contours, findings) = outline
                .as_ref()
                .map_or(accepted, |(cache, contours, findings)| {
                    (cache, contours, findings)
                });
            return self.scene(
                id,
                SceneInput {
                    transaction: &command.transaction_id,
                    changed,
                    document: &next,
                    contours,
                    geom_findings: findings,
                    cache,
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
                outline,
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
        let changed = if name_changed && !has_native_mirror_pair(&self.document) {
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
                }
                SceneKind::Committed
            }
        };
        // The accepted geometry, findings and identity ledger have no project-name dependency.
        self.scene_with_unchanged_geometry(id, &transaction, changed, kind)
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
        self.scene(
            id,
            SceneInput {
                transaction,
                changed,
                document: &self.document,
                contours: &self.contours,
                geom_findings: &self.findings,
                cache: &self.outline_cache,
                kind,
            },
        )
    }

    fn record_undo(&mut self, previous: ProjectDoc) {
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
        self.project_committed_scene(id, &transaction, changed, cacheable && rebuilt)
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
            edit_dispatch(),
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

    fn preview_edit(&self, id: String, command: EditCommand) -> CoreReply {
        let accepted = generators::parameter_problems(&self.document);
        // Layout and constraint settlement may rewrite anchors, change
        // membership, and prune outline versions, even for a single drag.
        let mut draft = self.document.clone();
        let result = apply_with_ledger(&mut draft, &[], &command.operation, &self.identity_ledger)
            .and_then(|mut changed| {
                // Preview must materialize the same dependant poses the commit
                // path publishes: anchored matrices follow from the anchor key's
                // edited pose, and `resolve` (which starts with `validate`)
                // refreshes every dependant exactly as commit does.
                layout::resolve(&mut draft)?;
                settle_layout_dependents(
                    &mut draft,
                    &command.operation,
                    &mut changed,
                    self.reusable_scene.is_some(),
                )?;
                outline_controls::detach_removed(&self.document.parts, &mut draft);
                changed.extend(outline_versions::prune_removed_components(
                    &self.document.parts,
                    &mut draft,
                ));
                changed.sort();
                changed.dedup();
                generators::introduced_error(&accepted, &draft)?;
                matrix::placement::validate_translation(
                    &self.document.parts,
                    &draft,
                    &command.operation,
                )?;
                Ok(changed)
            });
        let changed = match result {
            Ok(changed) => changed,
            Err(message) => return self.error(id, &message),
        };
        let (cache, contours, findings) = self.edited_outlines(&draft, &command.operation);
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

    fn edited_outlines(
        &self,
        draft: &ProjectDoc,
        operation: &EditOperation,
    ) -> (OutlineCache, Vec<Contour>, Vec<Finding>) {
        if let Some(moved) = self
            .reusable_scene
            .as_ref()
            .and_then(|_| settled_outline_changes(&self.document, draft, operation))
        {
            return outlines(draft, Some(&self.outline_cache), &moved);
        }
        outlines(draft, None, &[])
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
        self.project_committed_scene(id, transaction, changed, true)
    }

    fn project_committed_scene(
        &mut self,
        id: String,
        transaction: &str,
        changed: Vec<String>,
        cacheable: bool,
    ) -> CoreReply {
        let reply = self.scene(
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
        (self.outline_cache, self.contours, self.findings) = outlines(&self.document, None, &[]);
        outline_versions::refresh_recovery(&mut self.document, &self.outline_cache);
    }

    fn scene(&self, id: String, input: SceneInput<'_>) -> CoreReply {
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
        findings.extend(validate::validate(doc));
        let geometry::BoardSceneGeometry {
            contours: board_contours,
            outlines: board_outline_scenes,
            findings: board_findings,
            markers: corner_markers,
        } = geometry::board_scene_geometry(doc, cache);
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
                    matrix::project_valid_matrix(matrix, doc)
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
        && doc
            .layouts
            .iter()
            .all(|layout| layout.mirror_link.is_none())
        && doc.parts.iter().all(|part| part.anchor.is_none())
        && doc
            .matrices
            .iter()
            .all(|matrix| matrix.cells.iter().all(|cell| cell.assemblies.is_empty()))
    {
        return Ok(());
    }
    for _ in 0..2 {
        changed.extend(constraints::resolve_edit(doc, operation)?);
        changed.extend(layout::sync_components(doc)?);
    }
    Ok(())
}

enum History {
    Undo,
    Redo,
}

struct PreparedEdit {
    transaction: String,
    document: ProjectDoc,
    changed: Vec<String>,
    outline: Option<(OutlineCache, Vec<Contour>, Vec<Finding>)>,
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

#[doc(hidden)]
impl CoreEngine {
    pub fn with_document_for_test(document: ProjectDoc) -> Self {
        Self {
            document,
            ..Self::new()
        }
    }

    pub fn document_for_test(&self) -> &ProjectDoc {
        &self.document
    }

    pub fn undo_for_test(&self) -> &[ProjectDoc] {
        &self.undo
    }

    pub fn redo_for_test(&self) -> &[ProjectDoc] {
        &self.redo
    }

    pub fn outline_cache_for_test(&self) -> &OutlineCache {
        &self.outline_cache
    }

    #[allow(clippy::too_many_arguments)]
    pub fn publish_prepared_for_test(
        &mut self,
        id: String,
        transaction: String,
        document: ProjectDoc,
        changed: Vec<String>,
        cache: OutlineCache,
        contours: Vec<Contour>,
        findings: Vec<Finding>,
    ) -> CoreReply {
        self.publish_prepared_edit(
            id,
            PreparedEdit {
                transaction,
                document,
                changed,
                outline: Some((cache, contours, findings)),
                reuse_scene: false,
                cacheable: false,
            },
        )
    }
}
