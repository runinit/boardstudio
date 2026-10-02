//! Center-workspace manual mechanical profile draft and accepted edit.

use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, SessionEpoch};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, MechanicalPartProfile, MechanicalSwitchFamily,
    PartDefinition,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProfileDefinitionSource {
    Ergogen,
    Imported,
    Project,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileEditOwner {
    pub(crate) view_id: OperationId,
    pub(crate) scope: Option<Scope>,
    pub(crate) session_epoch: SessionEpoch,
    pub(crate) document_id: String,
    pub(crate) definition_id: String,
    pub(crate) source: ProfileDefinitionSource,
}

impl ProfileEditOwner {
    pub(crate) fn new(
        view_id: OperationId,
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        source: ProfileDefinitionSource,
        definition: &PartDefinition,
    ) -> Self {
        Self {
            view_id,
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            definition_id: definition.id.clone(),
            source,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProfileEditCapture {
    owner: ProfileEditOwner,
    definition: PartDefinition,
}

#[derive(Clone)]
pub(crate) struct PendingProfileEdit {
    owner: ProfileEditOwner,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

impl PendingProfileEdit {
    pub(crate) fn new(
        owner: ProfileEditOwner,
        outcome: crate::operation_outcomes::OutcomeSlot,
    ) -> Self {
        Self { owner, outcome }
    }

    /// Retire this exact operation observation once it settles or its Parts view
    /// owner changes. No terminal branch publishes feedback in the editor.
    pub(crate) fn should_retire(
        &self,
        current_owner: &ProfileEditOwner,
        current_scope: &Option<Scope>,
    ) -> bool {
        self.owner != *current_owner
            || self.owner.scope != *current_scope
            || self.outcome.borrow().is_some()
    }
}

pub(crate) struct ProfileEditContext<'a> {
    pub(crate) snapshot: &'a AcceptedSnapshot,
    pub(crate) owner: &'a ProfileEditOwner,
    pub(crate) runtime_scope: Option<Scope>,
    pub(crate) selection: Option<(Option<Scope>, String)>,
    pub(crate) definition: &'a PartDefinition,
}

impl ProfileEditCapture {
    pub(crate) fn new(owner: ProfileEditOwner, definition: PartDefinition) -> Self {
        Self { owner, definition }
    }
}

/// Resolve the local draft's commit against the latest accepted document. The
/// selected source definition is immutable; unrelated accepted edits rebase,
/// while a changed target or owner is silently rejected.
pub(crate) fn prepare_profile_edit(
    current: ProfileEditContext<'_>,
    capture: &ProfileEditCapture,
    profile: MechanicalPartProfile,
    operation_id: OperationId,
) -> Option<Event> {
    let definition_id = &capture.definition.id;
    if capture.owner.scope.is_none()
        || current.owner != &capture.owner
        || current.runtime_scope != capture.owner.scope
        || current.snapshot.session_epoch != capture.owner.session_epoch
        || current.snapshot.document.id != capture.owner.document_id
        || current.selection != Some((capture.owner.scope.clone(), definition_id.clone()))
        || current.definition != &capture.definition
        || profile.definition_id != *definition_id
    {
        return None;
    }

    let mut replacement = current.snapshot.document.as_ref().clone();
    match capture.owner.source {
        ProfileDefinitionSource::Project => {
            let definition = replacement
                .definitions
                .iter_mut()
                .find(|definition| definition.id == *definition_id)?;
            if *definition != capture.definition {
                return None;
            }
            definition.mechanical_profile = Some(profile);
        }
        ProfileDefinitionSource::Ergogen | ProfileDefinitionSource::Imported => {
            if replacement
                .definitions
                .iter()
                .any(|definition| definition.id == *definition_id)
            {
                return None;
            }
            let mut definition = capture.definition.clone();
            definition.mechanical_profile = Some(profile);
            replacement.definitions.push(definition);
        }
    }

    Some(Event::Edit {
        operation_id,
        command: EditCommand {
            base_revision: current.snapshot.document.revision,
            transaction_id: format!("parts-mechanical-profile-{}", operation_id.0),
            phase: EditPhase::Commit,
            target_ids: vec![definition_id.clone()],
            operation: EditOperation::ReplaceDocument {
                document: Box::new(replacement),
            },
        },
    })
}

pub(crate) fn initial_profile(
    definition: &PartDefinition,
    existing: Option<&MechanicalPartProfile>,
) -> MechanicalPartProfile {
    if let Some(profile) = existing {
        return profile.clone();
    }
    let switch_family = infer_switch_family(definition);
    let plate_to_pcb = switch_family.map_or(0.0, |family| {
        let (datum, thickness) = match family {
            MechanicalSwitchFamily::Mx | MechanicalSwitchFamily::ChocV2 => (5.0, 1.5),
            MechanicalSwitchFamily::ChocV1 => (3.5, 1.3),
        };
        datum - thickness
    });
    MechanicalPartProfile {
        source_geometry: None,
        pcb_holes: None,
        clearance_volumes: None,
        openings: None,
        clearances: None,
        supported_thickness: None,
        switch_family,
        definition_id: definition.id.clone(),
        source: "Parts library".into(),
        cutouts: Vec::new(),
        plate_to_pcb,
    }
}

pub(crate) fn displayed_mounting_gap(profile: &MechanicalPartProfile) -> Option<String> {
    profile
        .switch_family
        .map(|_| format!("{:.2}", profile.plate_to_pcb))
}

fn infer_switch_family(definition: &PartDefinition) -> Option<MechanicalSwitchFamily> {
    let generator = definition.generator.as_ref()?;
    let source = generator.source.to_lowercase();
    if source == "ceoloide/switch_mx" {
        return Some(MechanicalSwitchFamily::Mx);
    }
    if !source.ends_with("/switch_choc_v1_v2") {
        return None;
    }
    let get_enabled = |name: &str| {
        generator.parameters.get(name).and_then(|value| {
            value
                .as_bool()
                .or_else(|| value.get("value").and_then(serde_json::Value::as_bool))
        })
    };
    let v1 = get_enabled("choc_v1_support").unwrap_or(true);
    let v2 = get_enabled("choc_v2_support").unwrap_or(true);
    match (v1, v2) {
        (true, false) => Some(MechanicalSwitchFamily::ChocV1),
        (false, true) => Some(MechanicalSwitchFamily::ChocV2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::{
        AcceptedSnapshot, Completion, Effect, Event, OperationId, SaveResult, Session,
        TerminalOutcome,
    };
    use boardstudio_core::model::{
        EditOperation, EditPhase, MechanicalPartProfile, MechanicalProfileSource, PartDefinition,
    };
    use boardstudio_core::{
        CoreEngine,
        model::{ProjectDoc, Vec2},
    };

    fn definition(id: &str, name: &str) -> PartDefinition {
        serde_json::from_value(serde_json::json!({
            "id": id,
            "name": name,
            "kind": "custom",
            "courtyard": [{"x": -5.0, "y": -3.0}, {"x": 5.0, "y": -3.0}, {"x": 5.0, "y": 3.0}],
            "pads": []
        }))
        .unwrap()
    }

    fn profile(id: &str, source: &str) -> MechanicalPartProfile {
        MechanicalPartProfile {
            source_geometry: Some(MechanicalProfileSource {
                text: "source geometry".into(),
                sha256: "a".repeat(64),
                mappings: vec![],
                source_ids: vec!["source-1".into()],
            }),
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: Some(vec![vec![
                Vec2 { x: -2.5, y: -2.5 },
                Vec2 { x: 2.5, y: -2.5 },
            ]]),
            supported_thickness: None,
            switch_family: None,
            definition_id: id.into(),
            source: source.into(),
            cutouts: vec![vec![Vec2 { x: -1.0, y: -1.0 }, Vec2 { x: 1.0, y: -1.0 }]],
            plate_to_pcb: 0.8,
        }
    }

    fn document(definitions: Vec<PartDefinition>) -> ProjectDoc {
        let mut document = ProjectDoc::empty("parts-fit", "Parts fit fixture");
        document.definitions = definitions;
        document
    }

    fn advance(session: &mut Session, core: &mut CoreEngine, initial: Vec<Effect>) {
        let mut pending = initial;
        while let Some(effect) = pending.pop() {
            match effect {
                Effect::Core {
                    request_id,
                    executor_epoch,
                    request,
                    ..
                } => {
                    let reply = core.handle(*request);
                    pending.extend(session.complete(Completion::Core {
                        request_id,
                        executor_epoch,
                        reply: Box::new(reply),
                    }));
                }
                Effect::Persist {
                    save_attempt_id, ..
                } => {
                    pending.extend(session.complete(Completion::Persist {
                        save_attempt_id,
                        result: SaveResult::Committed,
                    }));
                }
                _ => {}
            }
        }
    }

    fn open(document: ProjectDoc) -> (Session, CoreEngine) {
        let mut session = Session::new();
        let mut core = CoreEngine::new();
        let effects = session.submit(Event::Open {
            operation_id: OperationId(1),
            document,
        });
        advance(&mut session, &mut core, effects);
        (session, core)
    }

    fn accepted(session: &Session) -> AcceptedSnapshot {
        session
            .read_model()
            .accepted
            .clone()
            .expect("open accepted")
    }

    fn edit_owner(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        source: ProfileDefinitionSource,
        definition: &PartDefinition,
        view_id: u64,
    ) -> ProfileEditOwner {
        ProfileEditOwner::new(OperationId(view_id), snapshot, scope, source, definition)
    }

    #[test]
    fn manual_profile_edit_rebases_latest_project_document_and_round_trips_history() {
        let mut target = definition("switch", "Switch");
        let existing_profile = profile("switch", "Original source");
        target.mechanical_profile = Some(existing_profile.clone());
        let original = document(vec![target.clone(), definition("other", "Other")]);
        let (mut session, mut core) = open(original.clone());
        let initial = accepted(&session);
        let scope = session.scope();
        let owner = edit_owner(
            &initial,
            scope.clone(),
            ProfileDefinitionSource::Project,
            &target,
            100,
        );
        let capture = ProfileEditCapture::new(owner.clone(), target.clone());

        let mut unrelated = initial.document.as_ref().clone();
        unrelated.name = "Renamed project during draft".into();
        let unrelated_event = Event::Edit {
            operation_id: OperationId(2),
            command: boardstudio_core::model::EditCommand {
                base_revision: initial.document.revision,
                transaction_id: "unrelated-project-rename".into(),
                phase: EditPhase::Commit,
                target_ids: vec![initial.document.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated.clone()),
                },
            },
        };
        let effects = session.submit(unrelated_event);
        advance(&mut session, &mut core, effects);

        let latest = accepted(&session);
        let current_target = latest
            .document
            .definitions
            .iter()
            .find(|entry| entry.id == "switch")
            .unwrap()
            .clone();
        let edited_profile = profile("switch", "Parts library");
        let event = prepare_profile_edit(
            ProfileEditContext {
                snapshot: &latest,
                owner: &owner,
                runtime_scope: session.scope(),
                selection: Some((session.scope(), current_target.id.clone())),
                definition: &current_target,
            },
            &capture,
            edited_profile.clone(),
            OperationId(3),
        )
        .expect("the still-selected definition can save against the latest document");
        let Event::Edit { command, .. } = &event else {
            panic!("profile save uses normal Session edit");
        };
        assert_eq!(command.base_revision, latest.document.revision);
        assert_eq!(command.phase, EditPhase::Commit);
        assert_eq!(command.target_ids, vec!["switch"]);
        let EditOperation::ReplaceDocument { document: proposed } = &command.operation else {
            panic!("profile save uses the existing replace-document edit");
        };
        assert_eq!(proposed.name, "Renamed project during draft");
        assert_eq!(
            proposed.definitions[0].mechanical_profile,
            Some(edited_profile.clone())
        );
        assert_eq!(proposed.definitions[1], original.definitions[1]);

        let effects = session.submit(event);
        advance(&mut session, &mut core, effects);
        assert_eq!(
            accepted(&session).document.definitions[0].mechanical_profile,
            Some(edited_profile.clone())
        );

        let effects = session.submit(Event::Undo {
            operation_id: OperationId(4),
        });
        advance(&mut session, &mut core, effects);
        assert_eq!(
            accepted(&session).document.definitions[0].mechanical_profile,
            Some(existing_profile)
        );
        let effects = session.submit(Event::Redo {
            operation_id: OperationId(5),
        });
        advance(&mut session, &mut core, effects);
        assert_eq!(
            accepted(&session).document.definitions[0].mechanical_profile,
            Some(edited_profile)
        );
    }

    #[test]
    fn saving_a_selected_catalogue_definition_materializes_only_that_definition() {
        let original = document(vec![definition("other", "Other")]);
        let (session, _core) = open(original.clone());
        let snapshot = accepted(&session);
        let scope = session.scope();
        let selected_definition = definition("bundled-switch", "Bundled switch");
        let owner = edit_owner(
            &snapshot,
            scope.clone(),
            ProfileDefinitionSource::Ergogen,
            &selected_definition,
            200,
        );
        let capture = ProfileEditCapture::new(owner.clone(), selected_definition.clone());
        let wrong_source_owner = edit_owner(
            &snapshot,
            scope.clone(),
            ProfileDefinitionSource::Imported,
            &selected_definition,
            200,
        );
        let draft = profile("bundled-switch", "Parts library");

        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &snapshot,
                    owner: &wrong_source_owner,
                    runtime_scope: scope.clone(),
                    selection: Some((scope.clone(), "bundled-switch".into())),
                    definition: &selected_definition,
                },
                &capture,
                draft.clone(),
                OperationId(3),
            )
            .is_none()
        );

        let event = prepare_profile_edit(
            ProfileEditContext {
                snapshot: &snapshot,
                owner: &owner,
                runtime_scope: scope.clone(),
                selection: Some((scope, "bundled-switch".into())),
                definition: &selected_definition,
            },
            &capture,
            draft.clone(),
            OperationId(2),
        )
        .expect("a current bundled entry can be saved as a project override");
        let Event::Edit { command, .. } = event else {
            panic!("profile save uses normal Session edit");
        };
        let EditOperation::ReplaceDocument { document: proposed } = command.operation else {
            panic!("profile save uses replace-document");
        };
        assert_eq!(proposed.definitions.len(), 2);
        assert_eq!(proposed.definitions[0], original.definitions[0]);
        assert_eq!(proposed.definitions[1].id, "bundled-switch");
        assert_eq!(proposed.definitions[1].mechanical_profile, Some(draft));
        assert_eq!(proposed.name, original.name);
    }

    #[test]
    fn manual_profile_defaults_and_contour_edits_preserve_optional_profile_data() {
        let definition = definition("custom", "Custom part");
        let defaults = initial_profile(&definition, None);
        assert_eq!(defaults.definition_id, "custom");
        assert_eq!(defaults.source, "Parts library");
        assert_eq!(defaults.plate_to_pcb, 0.0);
        assert!(defaults.cutouts.is_empty());
        assert!(defaults.clearances.is_none());
        assert!(defaults.source_geometry.is_none());

        let existing = profile("custom", "Existing evidence");
        let mut edited = initial_profile(&definition, Some(&existing));
        edited.cutouts[0][0].x = -3.0;
        assert_eq!(edited.source_geometry, existing.source_geometry);
        assert_eq!(edited.clearances, existing.clearances);
        assert_eq!(edited.cutouts[0][0].x, -3.0);
        assert_eq!(edited.cutouts[0][0].y, existing.cutouts[0][0].y);
    }

    #[test]
    fn existing_switch_profile_displays_its_saved_gap_instead_of_a_default_gap() {
        let mut saved = profile("switch", "Parts library");
        saved.switch_family = Some(MechanicalSwitchFamily::Mx);
        saved.plate_to_pcb = 0.8;
        assert_eq!(displayed_mounting_gap(&saved).as_deref(), Some("0.80"));
    }

    #[test]
    fn profile_save_rejects_a_changed_selection_or_changed_target() {
        let target = definition("switch", "Switch");
        let original = document(vec![target.clone(), definition("other", "Other")]);
        let (session, _core) = open(original);
        let current = accepted(&session);
        let scope = session.scope();
        let owner = edit_owner(
            &current,
            scope.clone(),
            ProfileDefinitionSource::Project,
            &target,
            300,
        );
        let capture = ProfileEditCapture::new(owner.clone(), target.clone());
        let proposed_profile = profile("switch", "Parts library");

        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    owner: &owner,
                    runtime_scope: scope.clone(),
                    selection: Some((scope.clone(), "other".into())),
                    definition: &target,
                },
                &capture,
                proposed_profile.clone(),
                OperationId(2),
            )
            .is_none()
        );

        let mut changed_scope = scope.clone();
        changed_scope
            .as_mut()
            .expect("an open Parts view has a scope")
            .board_id = "another-board".into();
        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    owner: &owner,
                    runtime_scope: changed_scope,
                    selection: Some((scope.clone(), "switch".into())),
                    definition: &target,
                },
                &capture,
                proposed_profile.clone(),
                OperationId(4),
            )
            .is_none()
        );

        let mut changed_target = target;
        changed_target.name = "Changed after editor opened".into();
        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    owner: &owner,
                    runtime_scope: scope.clone(),
                    selection: Some((scope, "switch".into())),
                    definition: &changed_target,
                },
                &capture,
                proposed_profile,
                OperationId(3),
            )
            .is_none()
        );
    }

    #[test]
    fn profile_terminal_observation_is_bound_to_its_operation_and_parts_view_owner() {
        use crate::operation_outcomes::OperationOutcomes;

        let (session, _core) = open(document(vec![definition("switch", "Switch")]));
        let snapshot = accepted(&session);
        let scope = session.scope();
        let target = snapshot.document.definitions[0].clone();
        let owner = edit_owner(
            &snapshot,
            scope.clone(),
            ProfileDefinitionSource::Project,
            &target,
            400,
        );

        for terminal in [
            TerminalOutcome::Completed,
            TerminalOutcome::Rejected("late rejection".into()),
        ] {
            let operations = OperationOutcomes::default();
            let operation_id = OperationId(401);
            let pending = PendingProfileEdit::new(owner.clone(), operations.observe(operation_id));
            assert!(!pending.should_retire(&owner, &scope));
            assert!(!operations.settle(OperationId(402), TerminalOutcome::Completed,));

            let mut changed_runtime_scope = scope.clone();
            changed_runtime_scope
                .as_mut()
                .expect("an open Parts view has a scope")
                .board_id = "runtime-moved-board".into();
            assert!(pending.should_retire(&owner, &changed_runtime_scope));

            let mut changed_scope_owner = owner.clone();
            changed_scope_owner
                .scope
                .as_mut()
                .expect("an open Parts view has a scope")
                .board_id = "another-board".into();
            assert!(pending.should_retire(&changed_scope_owner, &scope));
            drop(pending);
            assert!(
                !operations.settle(operation_id, terminal),
                "retiring the old Parts owner drops only its observation; Session still owns the edit"
            );
        }

        let operations = OperationOutcomes::default();
        let operation_id = OperationId(403);
        let pending = PendingProfileEdit::new(owner.clone(), operations.observe(operation_id));
        let mut new_view = owner.clone();
        new_view.view_id = OperationId(404);
        assert!(pending.should_retire(&new_view, &scope));
        drop(pending);
        assert!(!operations.settle(operation_id, TerminalOutcome::Completed));

        let operations = OperationOutcomes::default();
        let operation_id = OperationId(405);
        let pending = PendingProfileEdit::new(owner.clone(), operations.observe(operation_id));
        assert!(operations.settle(operation_id, TerminalOutcome::Completed));
        assert!(pending.should_retire(&owner, &scope));
    }
}
