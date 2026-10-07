//! Center-workspace manual mechanical profile draft and accepted edit.

use boardstudio_application::{
    AcceptedSnapshot, EditResolver, OperationId, Resolution, Scope, SessionEpoch,
};
use boardstudio_core::model::{
    CoreReply, EditOperation, MechanicalBuiltinProfile, MechanicalPartProfile,
    MechanicalSwitchFamily, PartDefinition,
};

use crate::parts_custom_definition::replacement_commit;

pub(crate) const PROFILE_GONE: &str = "This part definition no longer exists.";

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

/// Resolve one manual profile save against the accepted document at execution: a
/// project-owned definition must still exist, a catalogue definition materializes as a
/// project override (or updates one that appeared meanwhile), and the replacement is
/// cloned from the accepted document, so a save queued behind another edit never
/// reverts it.
pub fn mechanical_profile_resolver(
    source: ProfileDefinitionSource,
    definition_id: String,
    definition: PartDefinition,
    profile: MechanicalPartProfile,
) -> EditResolver {
    EditResolver::new(
        "parts-mechanical-profile",
        move |accepted: &AcceptedSnapshot| {
            let document = &accepted.document;
            let mut replacement = document.as_ref().clone();
            match source {
                ProfileDefinitionSource::Project => {
                    let Some(target) = replacement
                        .definitions
                        .iter_mut()
                        .find(|item| item.id == definition_id)
                    else {
                        return Resolution::Retire(PROFILE_GONE.into());
                    };
                    target.mechanical_profile = Some(profile.clone());
                }
                ProfileDefinitionSource::Generator | ProfileDefinitionSource::Imported => {
                    // The captured catalogue definition is only the seed: if the project
                    // gained this definition while the editor was open, the accepted one
                    // wins and only the profile is set on it.
                    if let Some(target) = replacement
                        .definitions
                        .iter_mut()
                        .find(|item| item.id == definition_id)
                    {
                        target.mechanical_profile = Some(profile.clone());
                    } else {
                        let mut materialized = definition.clone();
                        materialized.mechanical_profile = Some(profile.clone());
                        replacement.definitions.push(materialized);
                    }
                }
            }
            // The clone is built from the accepted document, so equality means this
            // profile already holds: resolve Unchanged, not a landing heuristic
            // (ADR-0005).
            if replacement == **document {
                return Resolution::Unchanged;
            }
            replacement_commit(
                EditOperation::ReplaceDocument {
                    document: Box::new(replacement),
                },
                vec![definition_id.clone()],
            )
        },
    )
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
    #[cfg(not(target_arch = "wasm32"))]
    use boardstudio_application::{
        AcceptedSnapshot, EditResolver, Event, OperationId, Resolution, TerminalOutcome,
    };
    #[cfg(not(target_arch = "wasm32"))]
    use boardstudio_core::model::{EditOperation, EditPhase};
    use boardstudio_core::model::{MechanicalPartProfile, MechanicalProfileSource, PartDefinition};
    use boardstudio_core::model::{ProjectDoc, Vec2};
    #[cfg(not(target_arch = "wasm32"))]
    use std::rc::Rc;

    #[cfg(not(target_arch = "wasm32"))]
    fn fixed_commit(
        operation_id: OperationId,
        command: boardstudio_core::model::EditCommand,
    ) -> Event {
        Event::ResolveEdit {
            operation_id,
            label: "test fixed command".into(),
            resolver: EditResolver::new("test fixed command", move |_| {
                Resolution::Submit(command.clone())
            }),
        }
    }

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

    #[cfg(not(target_arch = "wasm32"))]
    fn document(definitions: Vec<PartDefinition>) -> ProjectDoc {
        let mut document = ProjectDoc::empty("parts-fit", "Parts fit fixture");
        document.definitions = definitions;
        document
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn open(document: ProjectDoc) -> Rc<boardstudio_web_runtime::runtime::Runtime> {
        let runtime = boardstudio_web_runtime::runtime::Runtime::new();
        runtime.submit(Event::Open {
            operation_id: OperationId(1),
            document,
        });
        runtime
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn accepted(runtime: &boardstudio_web_runtime::runtime::Runtime) -> AcceptedSnapshot {
        runtime.model().accepted.clone().expect("open accepted")
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn edit_owner(
        snapshot: &AcceptedSnapshot,
        scope: Option<Scope>,
        source: ProfileDefinitionSource,
        definition: &PartDefinition,
        view_id: u64,
    ) -> ProfileEditOwner {
        ProfileEditOwner::new(OperationId(view_id), snapshot, scope, source, definition)
    }

    /// Submit one profile save to the real native Runtime and read its terminal outcome.
    #[cfg(not(target_arch = "wasm32"))]
    fn resolve_profile(
        runtime: &boardstudio_web_runtime::runtime::Runtime,
        operation: u64,
        source: ProfileDefinitionSource,
        definition: &PartDefinition,
        profile: MechanicalPartProfile,
    ) -> Vec<TerminalOutcome> {
        let operation_id = OperationId(operation);
        let outcome = runtime.observe_operation(operation_id);
        runtime.submit(Event::ResolveEdit {
            operation_id,
            label: "parts-mechanical-profile".into(),
            resolver: mechanical_profile_resolver(
                source,
                definition.id.clone(),
                definition.clone(),
                profile,
            ),
        });
        vec![outcome.borrow().clone().expect("Runtime settled the edit")]
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn profile_save_lands_against_the_latest_accepted_document_and_round_trips_history() {
        let mut target = definition("switch", "Switch");
        let existing_profile = profile("switch", "Original source");
        target.mechanical_profile = Some(existing_profile.clone());
        let original = document(vec![target.clone(), definition("other", "Other")]);
        let runtime = open(original.clone());

        // An unrelated accepted edit lands while the profile editor is open.
        let mut unrelated = runtime
            .model()
            .accepted
            .as_ref()
            .unwrap()
            .document
            .as_ref()
            .clone();
        unrelated.name = "Renamed project during draft".into();
        runtime.submit(fixed_commit(
            OperationId(2),
            boardstudio_core::model::EditCommand {
                base_revision: 0,
                transaction_id: "unrelated-project-rename".into(),
                phase: EditPhase::Commit,
                target_ids: vec![unrelated.id.clone()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(unrelated),
                },
            },
        ));

        let edited_profile = profile("switch", "Parts library");
        assert_eq!(
            resolve_profile(
                &runtime,
                3,
                ProfileDefinitionSource::Project,
                &target,
                edited_profile.clone(),
            ),
            vec![TerminalOutcome::Completed],
            "the profile save lands against the renamed document"
        );
        let document = accepted(&runtime).document;
        assert_eq!(document.name, "Renamed project during draft");
        assert_eq!(
            document.definitions[0].mechanical_profile,
            Some(edited_profile.clone())
        );
        assert_eq!(document.definitions[1], original.definitions[1]);

        runtime.submit(Event::Undo {
            operation_id: OperationId(4),
        });
        assert_eq!(
            accepted(&runtime).document.definitions[0].mechanical_profile,
            Some(existing_profile)
        );
        runtime.submit(Event::Redo {
            operation_id: OperationId(5),
        });
        assert_eq!(
            accepted(&runtime).document.definitions[0].mechanical_profile,
            Some(edited_profile)
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn saving_a_selected_catalogue_definition_materializes_only_that_definition() {
        let original = document(vec![definition("other", "Other")]);
        let runtime = open(original.clone());
        let selected_definition = definition("bundled-switch", "Bundled switch");
        let draft = profile("bundled-switch", "Parts library");

        assert_eq!(
            resolve_profile(
                &runtime,
                2,
                ProfileDefinitionSource::Generator,
                &selected_definition,
                draft.clone(),
            ),
            vec![TerminalOutcome::Completed],
            "a bundled entry saves as a project override"
        );
        let proposed = accepted(&runtime).document;
        assert_eq!(proposed.definitions.len(), 2);
        assert_eq!(proposed.definitions[0], original.definitions[0]);
        assert_eq!(proposed.definitions[1].id, "bundled-switch");
        assert_eq!(proposed.definitions[1].mechanical_profile, Some(draft));
        assert_eq!(proposed.name, original.name);
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn profile_save_retires_a_removed_project_definition_and_skips_unchanged_profiles() {
        let target = definition("switch", "Switch");
        let runtime = open(document(vec![target.clone()]));
        assert_eq!(
            resolve_profile(
                &runtime,
                2,
                ProfileDefinitionSource::Project,
                &target,
                profile("switch", "Parts library"),
            ),
            vec![TerminalOutcome::Completed],
            "the first save lands"
        );
        // Saving the identical profile again resolves Unchanged without a revision.
        assert_eq!(
            resolve_profile(
                &runtime,
                3,
                ProfileDefinitionSource::Project,
                &target,
                profile("switch", "Parts library"),
            ),
            vec![TerminalOutcome::Completed],
            "an unchanged save completes quietly"
        );
        assert_eq!(accepted(&runtime).document.revision, 1);

        // Remove the definition, then queue another save of it.
        let mut without = accepted(&runtime).document.as_ref().clone();
        without.definitions.clear();
        runtime.submit(fixed_commit(
            OperationId(4),
            boardstudio_core::model::EditCommand {
                base_revision: 1,
                transaction_id: "remove-definition".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["switch".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(without),
                },
            },
        ));
        assert_eq!(
            resolve_profile(
                &runtime,
                5,
                ProfileDefinitionSource::Project,
                &target,
                profile("switch", "Parts library"),
            ),
            vec![TerminalOutcome::Rejected(PROFILE_GONE.into())],
            "a save on a deleted definition retires with the reason"
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn profile_save_updates_a_definition_materialized_while_the_editor_was_open() {
        let selected = definition("bundled-switch", "Bundled switch");
        let runtime = open(document(vec![definition("other", "Other")]));
        // Another edit materializes the same catalogue definition first.
        let mut materialized = accepted(&runtime).document.as_ref().clone();
        materialized.definitions.push(selected.clone());
        runtime.submit(fixed_commit(
            OperationId(2),
            boardstudio_core::model::EditCommand {
                base_revision: 0,
                transaction_id: "materialize".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["bundled-switch".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(materialized),
                },
            },
        ));
        let mut latest = accepted(&runtime).document.as_ref().clone();
        latest.definitions[1].name = "Edited meanwhile".into();
        runtime.submit(fixed_commit(
            OperationId(3),
            boardstudio_core::model::EditCommand {
                base_revision: 1,
                transaction_id: "rename".into(),
                phase: EditPhase::Commit,
                target_ids: vec!["bundled-switch".into()],
                operation: EditOperation::ReplaceDocument {
                    document: Box::new(latest),
                },
            },
        ));

        assert_eq!(
            resolve_profile(
                &runtime,
                4,
                ProfileDefinitionSource::Generator,
                &selected,
                profile("bundled-switch", "Parts library"),
            ),
            vec![TerminalOutcome::Completed]
        );
        let document = accepted(&runtime).document;
        assert_eq!(
            document.definitions.len(),
            2,
            "no duplicate materialization"
        );
        assert_eq!(document.definitions[1].name, "Edited meanwhile");
        assert_eq!(
            document.definitions[1].mechanical_profile,
            Some(profile("bundled-switch", "Parts library"))
        );
    }

    #[test]
    #[cfg(not(target_arch = "wasm32"))]
    fn profile_save_queued_behind_a_custom_definition_field_edit_keeps_the_field_edit() {
        use crate::parts_custom_definition::{DefinitionEdit, definition_field_resolver};

        let mut target = definition("switch", "Switch");
        target.courtyard = vec![
            Vec2 { x: -5.0, y: -3.0 },
            Vec2 { x: 5.0, y: -3.0 },
            Vec2 { x: 5.0, y: 3.0 },
            Vec2 { x: -5.0, y: 3.0 },
        ];
        let runtime = open(document(vec![target.clone()]));
        runtime.hold_next_core();
        let field_outcome = runtime.observe_operation(OperationId(2));
        let profile_outcome = runtime.observe_operation(OperationId(3));

        runtime.submit(Event::ResolveEdit {
            operation_id: OperationId(2),
            label: "parts-definition-field".into(),
            resolver: definition_field_resolver(
                "switch".into(),
                DefinitionEdit::CourtyardWidth("12.5".into()),
                0,
            ),
        });
        assert!(runtime.core_entered(), "the field edit reached held Core");

        runtime.submit(Event::ResolveEdit {
            operation_id: OperationId(3),
            label: "parts-mechanical-profile".into(),
            resolver: mechanical_profile_resolver(
                ProfileDefinitionSource::Project,
                "switch".into(),
                target,
                profile("switch", "Parts library"),
            ),
        });
        assert!(
            profile_outcome.borrow().is_none(),
            "the profile save queues behind Core"
        );

        runtime.release_core();
        assert_eq!(
            field_outcome.borrow().clone(),
            Some(TerminalOutcome::Completed)
        );
        assert_eq!(
            profile_outcome.borrow().clone(),
            Some(TerminalOutcome::Completed)
        );
        let document = accepted(&runtime).document;
        assert_eq!(
            crate::parts_custom_definition::courtyard_bounds(&document.definitions[0].courtyard).0,
            12.5,
            "the queued field edit survives the whole-document profile save"
        );
        assert_eq!(
            document.definitions[0].mechanical_profile,
            Some(profile("switch", "Parts library"))
        );

        runtime.submit(Event::Undo {
            operation_id: OperationId(4),
        });
        assert_eq!(
            accepted(&runtime).document.definitions[0].mechanical_profile,
            None
        );
        assert_eq!(
            crate::parts_custom_definition::courtyard_bounds(
                &accepted(&runtime).document.definitions[0].courtyard
            )
            .0,
            12.5
        );
        runtime.submit(Event::Undo {
            operation_id: OperationId(5),
        });
        assert_eq!(
            crate::parts_custom_definition::courtyard_bounds(
                &accepted(&runtime).document.definitions[0].courtyard
            )
            .0,
            10.0,
            "the second Undo removes the field edit"
        );
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
    #[cfg(not(target_arch = "wasm32"))]
    fn standard_profile_request_rejects_superseded_operation_and_returned_view_aba() {
        let target = definition("bundled-switch", "Switch");
        let runtime = open(document(vec![]));
        let snapshot = accepted(&runtime);
        let scope = runtime.scope();
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
