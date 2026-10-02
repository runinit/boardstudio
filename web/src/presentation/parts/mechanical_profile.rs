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

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProfileEditCapture {
    source: ProfileDefinitionSource,
    scope: Option<Scope>,
    session_epoch: SessionEpoch,
    document_id: String,
    definition: PartDefinition,
}

pub(crate) struct ProfileEditContext<'a> {
    pub(crate) snapshot: &'a AcceptedSnapshot,
    pub(crate) scope: Option<Scope>,
    pub(crate) selection: Option<(Option<Scope>, String)>,
    pub(crate) source: ProfileDefinitionSource,
    pub(crate) definition: &'a PartDefinition,
}

impl ProfileEditCapture {
    pub(crate) fn new(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        source: ProfileDefinitionSource,
        definition: PartDefinition,
    ) -> Self {
        Self {
            source,
            scope,
            session_epoch: snapshot.session_epoch,
            document_id: snapshot.document.id.clone(),
            definition,
        }
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
    if capture.scope.is_none()
        || current.scope != capture.scope
        || current.snapshot.session_epoch != capture.session_epoch
        || current.snapshot.document.id != capture.document_id
        || current.selection != Some((capture.scope.clone(), definition_id.clone()))
        || current.source != capture.source
        || current.definition != &capture.definition
        || profile.definition_id != *definition_id
    {
        return None;
    }

    let mut replacement = current.snapshot.document.as_ref().clone();
    match capture.source {
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

    #[test]
    fn manual_profile_edit_rebases_latest_project_document_and_round_trips_history() {
        let mut target = definition("switch", "Switch");
        let existing_profile = profile("switch", "Original source");
        target.mechanical_profile = Some(existing_profile.clone());
        let original = document(vec![target.clone(), definition("other", "Other")]);
        let (mut session, mut core) = open(original.clone());
        let initial = accepted(&session);
        let scope = session.scope();
        let capture = ProfileEditCapture::new(
            &initial,
            scope.clone(),
            ProfileDefinitionSource::Project,
            target.clone(),
        );

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
                scope: session.scope(),
                selection: Some((session.scope(), current_target.id.clone())),
                source: ProfileDefinitionSource::Project,
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
        let capture = ProfileEditCapture::new(
            &snapshot,
            scope.clone(),
            ProfileDefinitionSource::Ergogen,
            selected_definition.clone(),
        );
        let draft = profile("bundled-switch", "Parts library");

        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &snapshot,
                    scope: scope.clone(),
                    selection: Some((scope.clone(), "bundled-switch".into())),
                    source: ProfileDefinitionSource::Imported,
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
                scope: scope.clone(),
                selection: Some((scope, "bundled-switch".into())),
                source: ProfileDefinitionSource::Ergogen,
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
    fn profile_save_rejects_a_changed_selection_or_changed_target() {
        let target = definition("switch", "Switch");
        let original = document(vec![target.clone(), definition("other", "Other")]);
        let (session, _core) = open(original);
        let current = accepted(&session);
        let scope = session.scope();
        let capture = ProfileEditCapture::new(
            &current,
            scope.clone(),
            ProfileDefinitionSource::Project,
            target.clone(),
        );
        let proposed_profile = profile("switch", "Parts library");

        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    scope: scope.clone(),
                    selection: Some((scope.clone(), "other".into())),
                    source: ProfileDefinitionSource::Project,
                    definition: &target,
                },
                &capture,
                proposed_profile.clone(),
                OperationId(2),
            )
            .is_none()
        );

        let mut changed_target = target;
        changed_target.name = "Changed after editor opened".into();
        assert!(
            prepare_profile_edit(
                ProfileEditContext {
                    snapshot: &current,
                    scope: scope.clone(),
                    selection: Some((scope, "switch".into())),
                    source: ProfileDefinitionSource::Project,
                    definition: &changed_target,
                },
                &capture,
                proposed_profile,
                OperationId(3),
            )
            .is_none()
        );
    }
}
