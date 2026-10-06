//! Center-workspace manual mechanical profile draft and accepted edit.

use boardstudio_application::{AcceptedSnapshot, Event, OperationId, Scope, SessionEpoch};
use boardstudio_core::model::{
    CoreReply, EditCommand, EditOperation, EditPhase, MechanicalBuiltinProfile,
    MechanicalPartProfile, MechanicalSwitchFamily, PartDefinition,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfileDefinitionSource {
    Generator,
    Imported,
    Project,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileEditOwner {
    pub view_id: OperationId,
    pub scope: Option<Scope>,
    pub session_epoch: SessionEpoch,
    pub document_id: String,
    pub definition_id: String,
    pub source: ProfileDefinitionSource,
}

impl ProfileEditOwner {
    pub fn new(
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
pub struct ProfileEditCapture {
    owner: ProfileEditOwner,
    definition: PartDefinition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StandardProfileRequestCapture {
    pub operation_id: OperationId,
    pub owner: ProfileEditOwner,
    pub definition: PartDefinition,
    pub family: MechanicalSwitchFamily,
    pub plate_to_pcb: f64,
    pub scope_generation: u64,
    pub selection_generation: u64,
}

impl StandardProfileRequestCapture {
    pub fn new(
        operation_id: OperationId,
        owner: ProfileEditOwner,
        definition: PartDefinition,
        family: MechanicalSwitchFamily,
        plate_to_pcb: f64,
        scope_generation: u64,
        selection_generation: u64,
    ) -> Self {
        Self {
            operation_id,
            owner,
            definition,
            family,
            plate_to_pcb,
            scope_generation,
            selection_generation,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn standard_profile_request_is_current(
    request: &StandardProfileRequestCapture,
    pending_operation: Option<OperationId>,
    current_owner: &ProfileEditOwner,
    runtime_scope: Option<&Scope>,
    snapshot: &AcceptedSnapshot,
    selection: &Option<(Option<Scope>, String)>,
    definition: &PartDefinition,
    scope_generation: u64,
    selection_generation: u64,
    workspace: &str,
    mounted: bool,
) -> bool {
    mounted
        && workspace == "Parts"
        && pending_operation == Some(request.operation_id)
        && current_owner == &request.owner
        && request.owner.scope.as_ref() == runtime_scope
        && snapshot.session_epoch == request.owner.session_epoch
        && snapshot.document.id == request.owner.document_id
        && selection.as_ref() == Some(&(request.owner.scope.clone(), request.definition.id.clone()))
        && definition == &request.definition
        && request.owner.definition_id == request.definition.id
        && scope_generation == request.scope_generation
        && selection_generation == request.selection_generation
}

pub fn standard_profile_source_and_gap(
    family: MechanicalSwitchFamily,
) -> (MechanicalBuiltinProfile, f64) {
    match family {
        MechanicalSwitchFamily::Mx => (MechanicalBuiltinProfile::MxSwitch, 3.5),
        MechanicalSwitchFamily::ChocV1 => (MechanicalBuiltinProfile::ChocV1Switch, 2.2),
        MechanicalSwitchFamily::ChocV2 => (MechanicalBuiltinProfile::ChocV2Switch, 3.5),
    }
}

/// Keep the selector and the reusable built-in action's eligibility separate:
/// any switch can select a family, while any saved family can reload its fit.
pub fn standard_profile_controls(
    definition: &PartDefinition,
    profile: &MechanicalPartProfile,
) -> (bool, bool) {
    (
        definition.kind == boardstudio_core::model::PartKind::Switch,
        profile.switch_family.is_some(),
    )
}

pub fn dispatch_standard_profile_family(
    value: &str,
    load_standard: impl FnOnce(MechanicalSwitchFamily),
) -> bool {
    let family = match value {
        "mx" => MechanicalSwitchFamily::Mx,
        "choc-v1" => MechanicalSwitchFamily::ChocV1,
        "choc-v2" => MechanicalSwitchFamily::ChocV2,
        _ => return false,
    };
    load_standard(family);
    true
}

pub fn standard_profile_reply_matches(
    reply: CoreReply,
    request_id: &str,
    definition_id: &str,
    family: MechanicalSwitchFamily,
    plate_to_pcb: f64,
) -> Result<MechanicalPartProfile, String> {
    match reply {
        CoreReply::MechanicalProfile { id, profile } if id == request_id => {
            if profile.definition_id != definition_id
                || profile.switch_family != Some(family)
                || (profile.plate_to_pcb - plate_to_pcb).abs() > f64::EPSILON
            {
                return Err("Core returned a different standard switch fit.".into());
            }
            Ok(profile)
        }
        CoreReply::Error { id, message, .. } if id == request_id => Err(message),
        CoreReply::MechanicalProfile { .. } | CoreReply::Error { .. } => {
            Err("Core returned a stale standard switch fit reply.".into())
        }
        _ => Err("Core returned an unexpected standard switch fit reply.".into()),
    }
}

pub fn merge_standard_profile(
    draft: &mut MechanicalPartProfile,
    mut loaded: MechanicalPartProfile,
    family: MechanicalSwitchFamily,
    plate_to_pcb: f64,
) {
    // Core's optional fields are sparse: a missing value means retain the
    // user's current draft, including manually entered provenance and details.
    if loaded.source_geometry.is_some() {
        draft.source_geometry = loaded.source_geometry.take();
    }
    if loaded.pcb_holes.is_some() {
        draft.pcb_holes = loaded.pcb_holes.take();
    }
    if loaded.clearance_volumes.is_some() {
        draft.clearance_volumes = loaded.clearance_volumes.take();
    }
    if loaded.openings.is_some() {
        draft.openings = loaded.openings.take();
    }
    if loaded.clearances.is_some() {
        draft.clearances = loaded.clearances.take();
    }
    if loaded.supported_thickness.is_some() {
        draft.supported_thickness = loaded.supported_thickness.take();
    }
    draft.cutouts = loaded.cutouts;
    draft.source = loaded.source;
    draft.definition_id = loaded.definition_id;
    draft.switch_family = Some(family);
    draft.plate_to_pcb = plate_to_pcb;
}

#[derive(Clone)]
pub struct PendingProfileEdit {
    owner: ProfileEditOwner,
    outcome: crate::operation_outcomes::OutcomeSlot,
}

impl PendingProfileEdit {
    pub fn new(
        owner: ProfileEditOwner,
        outcome: crate::operation_outcomes::OutcomeSlot,
    ) -> Self {
        Self { owner, outcome }
    }

    /// Retire this exact operation observation once it settles or its Parts view
    /// owner changes. No terminal branch publishes feedback in the editor.
    pub fn should_retire(
        &self,
        current_owner: &ProfileEditOwner,
        current_scope: &Option<Scope>,
    ) -> bool {
        self.owner != *current_owner
            || self.owner.scope != *current_scope
            || self.outcome.borrow().is_some()
    }
}

pub struct ProfileEditContext<'a> {
    pub snapshot: &'a AcceptedSnapshot,
    pub owner: &'a ProfileEditOwner,
    pub runtime_scope: Option<Scope>,
    pub selection: Option<(Option<Scope>, String)>,
    pub definition: &'a PartDefinition,
}

impl ProfileEditCapture {
    pub fn new(owner: ProfileEditOwner, definition: PartDefinition) -> Self {
        Self { owner, definition }
    }
}

/// Resolve the local draft's commit against the latest accepted document. The
/// selected source definition is immutable; unrelated accepted edits rebase,
/// while a changed target or owner is silently rejected.
pub fn prepare_profile_edit(
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
        ProfileDefinitionSource::Generator | ProfileDefinitionSource::Imported => {
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

pub fn initial_profile(
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

pub fn displayed_mounting_gap(profile: &MechanicalPartProfile) -> Option<String> {
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
            ProfileDefinitionSource::Generator,
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

    #[test]
    fn standard_profile_request_rejects_superseded_operation_and_returned_view_aba() {
        let target = definition("bundled-switch", "Switch");
        let (session, _) = open(document(vec![]));
        let snapshot = accepted(&session);
        let scope = session.scope();
        let owner = edit_owner(
            &snapshot,
            scope.clone(),
            ProfileDefinitionSource::Generator,
            &target,
            300,
        );
        let request = StandardProfileRequestCapture::new(
            OperationId(301),
            owner.clone(),
            target.clone(),
            boardstudio_core::model::MechanicalSwitchFamily::Mx,
            3.5,
            8,
            4,
        );
        let selection = Some((scope.clone(), target.id.clone()));

        let is_current = |pending, scope_generation, selection_generation| {
            standard_profile_request_is_current(
                &request,
                pending,
                &owner,
                scope.as_ref(),
                &snapshot,
                &selection,
                &target,
                scope_generation,
                selection_generation,
                "Parts",
                true,
            )
        };

        assert!(is_current(Some(OperationId(301)), 8, 4));
        assert!(!is_current(Some(OperationId(302)), 8, 4));
        // A→B→A has the original Scope and selection again, but both monotonic owners advanced.
        assert!(!is_current(Some(OperationId(301)), 10, 6));
        assert!(!is_current(Some(OperationId(301)), 8, 6));
        assert!(!is_current(Some(OperationId(301)), 10, 4));
        assert!(!standard_profile_request_is_current(
            &request,
            Some(OperationId(301)),
            &owner,
            scope.as_ref(),
            &snapshot,
            &selection,
            &target,
            8,
            4,
            "Layout",
            true,
        ));
        assert!(!standard_profile_request_is_current(
            &request,
            Some(OperationId(301)),
            &owner,
            scope.as_ref(),
            &snapshot,
            &selection,
            &target,
            8,
            4,
            "Parts",
            false,
        ));
        let mut another_owner = owner.clone();
        another_owner.source = ProfileDefinitionSource::Project;
        assert!(!standard_profile_request_is_current(
            &request,
            Some(OperationId(301)),
            &another_owner,
            scope.as_ref(),
            &snapshot,
            &selection,
            &target,
            8,
            4,
            "Parts",
            true,
        ));
        let mut another_definition = target.clone();
        another_definition.name = "Newer selected definition".into();
        assert!(!standard_profile_request_is_current(
            &request,
            Some(OperationId(301)),
            &owner,
            scope.as_ref(),
            &snapshot,
            &selection,
            &another_definition,
            8,
            4,
            "Parts",
            true,
        ));
    }

    #[test]
    fn standard_profile_request_matches_react_family_sources_and_plate_gaps() {
        use boardstudio_core::model::{MechanicalBuiltinProfile, MechanicalSwitchFamily};

        assert_eq!(
            standard_profile_source_and_gap(MechanicalSwitchFamily::Mx),
            (MechanicalBuiltinProfile::MxSwitch, 3.5)
        );
        assert_eq!(
            standard_profile_source_and_gap(MechanicalSwitchFamily::ChocV1),
            (MechanicalBuiltinProfile::ChocV1Switch, 2.2)
        );
        assert_eq!(
            standard_profile_source_and_gap(MechanicalSwitchFamily::ChocV2),
            (MechanicalBuiltinProfile::ChocV2Switch, 3.5)
        );
    }

    #[test]
    fn standard_profile_merge_preserves_sparse_user_fields_and_replaces_fit_geometry() {
        let mut current = profile("switch", "manual provenance");
        let old_cutouts = current.cutouts.clone();
        let loaded = MechanicalPartProfile {
            source_geometry: None,
            pcb_holes: None,
            clearance_volumes: None,
            openings: None,
            clearances: None,
            supported_thickness: None,
            switch_family: Some(MechanicalSwitchFamily::Mx),
            definition_id: "switch".into(),
            source: "Kailh MX standard".into(),
            cutouts: vec![vec![Vec2 { x: -7.0, y: -7.0 }, Vec2 { x: 7.0, y: -7.0 }]],
            plate_to_pcb: 3.5,
        };
        let new_cutouts = loaded.cutouts.clone();
        merge_standard_profile(&mut current, loaded, MechanicalSwitchFamily::Mx, 3.5);
        assert_eq!(
            current.source_geometry.as_ref().unwrap().text,
            "source geometry"
        );
        assert_eq!(current.clearances, profile("switch", "").clearances);
        assert_eq!(current.cutouts, new_cutouts);
        assert_ne!(current.cutouts, old_cutouts);
        assert_eq!(current.source, "Kailh MX standard");
        assert_eq!(current.switch_family, Some(MechanicalSwitchFamily::Mx));
        assert_eq!(current.plate_to_pcb, 3.5);
    }

    #[test]
    fn standard_profile_reply_rejects_wrong_request_owner_family_or_gap() {
        use boardstudio_core::model::CoreReply;
        let mut expected = profile("switch", "standard");
        expected.switch_family = Some(MechanicalSwitchFamily::Mx);
        expected.plate_to_pcb = 3.5;
        assert!(
            standard_profile_reply_matches(
                CoreReply::MechanicalProfile {
                    id: "right".into(),
                    profile: expected.clone(),
                },
                "wrong",
                "switch",
                MechanicalSwitchFamily::Mx,
                3.5,
            )
            .is_err()
        );
        assert!(
            standard_profile_reply_matches(
                CoreReply::MechanicalProfile {
                    id: "right".into(),
                    profile: expected.clone(),
                },
                "right",
                "another-switch",
                MechanicalSwitchFamily::Mx,
                3.5,
            )
            .is_err()
        );
        assert!(
            standard_profile_reply_matches(
                CoreReply::MechanicalProfile {
                    id: "right".into(),
                    profile: expected.clone(),
                },
                "right",
                "switch",
                MechanicalSwitchFamily::ChocV2,
                3.5,
            )
            .is_err()
        );
        assert!(
            standard_profile_reply_matches(
                CoreReply::MechanicalProfile {
                    id: "right".into(),
                    profile: expected,
                },
                "right",
                "switch",
                MechanicalSwitchFamily::Mx,
                2.2,
            )
            .is_err()
        );
    }

    #[test]
    fn standard_profile_selector_and_action_follow_distinct_issue15_predicates() {
        let mut switch = definition("switch", "Project switch");
        switch.kind = boardstudio_core::model::PartKind::Switch;
        let mut family_profile = profile("switch", "saved fit");
        family_profile.switch_family = Some(MechanicalSwitchFamily::ChocV1);
        assert_eq!(
            standard_profile_controls(&switch, &family_profile),
            (true, true)
        );

        let mut non_switch = definition("controller", "Controller with saved switch fit");
        non_switch.kind = boardstudio_core::model::PartKind::Controller;
        assert_eq!(
            standard_profile_controls(&non_switch, &family_profile),
            (false, true)
        );

        family_profile.switch_family = None;
        assert_eq!(
            standard_profile_controls(&non_switch, &family_profile),
            (false, false)
        );
    }

    #[test]
    fn standard_profile_family_selection_dispatches_a_load_request() {
        let selected = std::cell::Cell::new(None);
        assert!(dispatch_standard_profile_family("choc-v1", |family| {
            selected.set(Some(family));
        }));
        assert_eq!(selected.get(), Some(MechanicalSwitchFamily::ChocV1));

        assert!(!dispatch_standard_profile_family(
            "unknown",
            |_| unreachable!()
        ));
    }
}
