//! Admission state for automatic Case preview generation.
//!
//! Session remains the authority for accepting a generation request. This
//! state only prevents the presentation from repeatedly dispatching the same
//! request after status notifications.
use boardstudio_application::AcceptedSnapshot;
use boardstudio_application::{GenerationStatus, Scope, SnapshotToken};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseGenerationOwner {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

#[derive(Default)]
pub struct AutomaticCaseGeneration {
    enabled: bool,
    attempted: Option<CaseGenerationOwner>,
}

/// Fingerprint the committed physical inputs that determine a Case result.
/// Snapshot tokens/revisions are intentionally excluded; owner identity is checked
/// separately before a completed result can be rebound. In-flight work never uses
/// this relaxation.
pub fn physical_case_fingerprint(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Option<[u8; 32]> {
    if snapshot.document.id != scope.document_id
        || snapshot.document.revision != snapshot.scene.revision
        || snapshot.session_epoch != scope.session_epoch
    {
        return None;
    }
    let document = boardstudio_web_host::cad_jobs::captured_case_document(snapshot, scope).ok()?;
    let scene = boardstudio_web_host::cad_jobs::captured_case_scene(snapshot, scope).ok()?;
    case_fingerprint_for_inputs(&document, &scene, scope)
}

fn case_fingerprint_for_inputs(
    document: &ProjectDoc,
    scene: &SceneDelta,
    scope: &Scope,
) -> Option<[u8; 32]> {
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scope.board_id)?;
    let part_ids = board.part_ids.iter().cloned().collect::<BTreeSet<_>>();
    let parts = document
        .parts
        .iter()
        .filter(|part| part_ids.contains(&part.id))
        .collect::<Vec<_>>();
    let definition_ids = parts
        .iter()
        .map(|part| part.definition_id.as_str())
        .collect::<BTreeSet<_>>();
    let definitions = document
        .definitions
        .iter()
        .filter(|definition| definition_ids.contains(definition.id.as_str()))
        .collect::<Vec<_>>();
    let terminal_names = definitions
        .iter()
        .map(|definition| {
            (
                definition.id.as_str(),
                definition
                    .terminals
                    .keys()
                    .cloned()
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    let parts = parts
        .iter()
        .map(|part| {
            let mut value = serde_json::to_value(part).ok()?;
            retain_fields(
                &mut value,
                &[
                    "id",
                    "definitionId",
                    "pose",
                    "side",
                    "keycap",
                    "outline",
                    "generatorParameters",
                ],
            );
            if let Some(parameters) = value.get_mut("generatorParameters") {
                filter_geometry_parameters(
                    parameters,
                    terminal_names.get(part.definition_id.as_str()),
                );
            }
            Some(value)
        })
        .collect::<Option<Vec<_>>>()?;

    let definitions = definitions
        .iter()
        .map(|definition| {
            let mut value = serde_json::to_value(definition).ok()?;
            retain_fields(
                &mut value,
                &[
                    "id",
                    "kind",
                    "pads",
                    "courtyard",
                    "mechanicalProfile",
                    "generator",
                ],
            );
            if let Some(generator) = value.get_mut("generator")
                && let Some(parameters) = generator.get_mut("parameters")
            {
                filter_geometry_parameters(parameters, terminal_names.get(definition.id.as_str()));
            }
            Some(value)
        })
        .collect::<Option<Vec<_>>>()?;

    let flipped = scope.instance_id.as_deref().and_then(|instance_id| {
        document
            .hardware
            .as_ref()?
            .instances
            .iter()
            .find(|instance| instance.id == instance_id)
            .map(|instance| instance.flipped)
    });
    let contours = scene
        .board_contours
        .iter()
        .find(|contours| contours.board_id == scope.board_id)
        .map(|contours| &contours.contours)?;
    let transforms = scene
        .transforms
        .iter()
        .filter(|transform| part_ids.contains(&transform.id))
        .collect::<Vec<_>>();
    let bodies = document
        .case_bodies
        .iter()
        .filter(|body| body.board_id == scope.board_id)
        .collect::<Vec<_>>();

    let fingerprint = json!({
        "documentId": document.id,
        "boardId": scope.board_id,
        "instanceId": scope.instance_id,
        "flipped": flipped,
        "thickness": board.thickness,
        "mechanical": document.mechanical,
        "parts": parts,
        "definitions": definitions,
        "contours": contours,
        "transforms": transforms,
        "bodies": bodies,
        "modules": document.modules,
        "moduleDefinitions": document.module_definitions,
    });
    let bytes = serde_json::to_vec(&fingerprint).ok()?;
    Some(Sha256::digest(bytes).into())
}

fn retain_fields(value: &mut Value, fields: &[&str]) {
    if let Value::Object(object) = value {
        object.retain(|key, _| fields.contains(&key.as_str()));
    }
}

fn filter_geometry_parameters(value: &mut Value, terminals: Option<&BTreeSet<String>>) {
    if let Value::Object(parameters) = value {
        parameters.retain(|key, parameter| {
            let terminal = terminals.is_some_and(|terminals| terminals.contains(key));
            let net = parameter
                .get("type")
                .and_then(Value::as_str)
                .is_some_and(|kind| kind == "net");
            !terminal && !net
        });
    }
}

impl AutomaticCaseGeneration {
    #[cfg(target_arch = "wasm32")]
    pub fn new() -> Self {
        Self {
            enabled: true,
            attempted: None,
        }
    }

    pub fn observe(
        &mut self,
        owner: Option<CaseGenerationOwner>,
        eligible: bool,
        has_reusable_result: bool,
        busy: bool,
    ) -> bool {
        if !self.enabled || !eligible || has_reusable_result || busy {
            return false;
        }
        let Some(owner) = owner else {
            return false;
        };
        if self.attempted.as_ref() == Some(&owner) {
            return false;
        }
        self.attempted = Some(owner);
        true
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.attempted = None;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

pub fn may_rebind_completed_case_result(
    exact: bool,
    scene_scope: &Scope,
    scene_token: SnapshotToken,
    scene_fingerprint: Option<[u8; 32]>,
    current_scope: &Scope,
    current_token: SnapshotToken,
    current_fingerprint: Option<[u8; 32]>,
) -> bool {
    exact
        && scene_scope == current_scope
        && scene_token != current_token
        && scene_fingerprint.is_some()
        && scene_fingerprint == current_fingerprint
}

pub fn same_owner_completed_scene_for_display(
    exact: bool,
    scene_scope: &Scope,
    current_scope: &Scope,
) -> bool {
    exact && scene_scope == current_scope
}

#[derive(Clone, Copy)]
pub enum CaseGeometryStatus {
    Missing,
    Previous,
    Preview,
    CurrentExact,
}

pub fn case_generation_title(
    generation: &GenerationStatus,
    geometry: CaseGeometryStatus,
) -> String {
    match generation {
        GenerationStatus::Preparing { .. } | GenerationStatus::Running { .. } => {
            "Generating case…".to_owned()
        }
        // Completed output can be rebound after Undo restores the same physical
        // inputs. The last request's terminal status then describes an older
        // edit, while strict current-owner output describes what is ready now.
        _ if matches!(geometry, CaseGeometryStatus::CurrentExact) => {
            "Exact case geometry ready.".to_owned()
        }
        GenerationStatus::Blocked { reason, .. } => format!("Case generation blocked: {reason}"),
        GenerationStatus::Failed { reason, .. } => format!("Case generation failed: {reason}"),
        GenerationStatus::Cancelled { .. } => "Case generation cancelled.".to_owned(),
        _ => match geometry {
            CaseGeometryStatus::Previous => {
                "Previous case geometry — regenerate for current changes.".to_owned()
            }
            CaseGeometryStatus::CurrentExact => "Exact case geometry ready.".to_owned(),
            CaseGeometryStatus::Preview => {
                "Case preview ready; exact assembly is still being built.".to_owned()
            }
            CaseGeometryStatus::Missing => "Generate a case from the saved keyboard.".to_owned(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;
    use boardstudio_core::model::{Board, BoardContours, BoardReadiness, Readiness};

    fn owner(token: u64, revision: u64) -> CaseGenerationOwner {
        CaseGenerationOwner {
            scope: Scope {
                session_epoch: SessionEpoch(2),
                document_id: "keyboard".into(),
                board_id: "left".into(),
                instance_id: Some("left-half".into()),
            },
            token: SnapshotToken(token),
            revision,
        }
    }

    #[test]
    fn undo_to_reusable_current_exact_geometry_supersedes_prior_block() {
        let prior_block = GenerationStatus::Blocked {
            job_id: boardstudio_application::JobId(3),
            reason: "mechanical findings block case generation".into(),
        };
        assert_eq!(
            case_generation_title(&prior_block, CaseGeometryStatus::CurrentExact),
            "Exact case geometry ready.",
        );
    }

    #[test]
    fn active_generation_and_current_blocks_keep_their_status() {
        let job_id = boardstudio_application::JobId(3);
        for status in [
            GenerationStatus::Preparing { job_id },
            GenerationStatus::Running { job_id },
        ] {
            assert_eq!(
                case_generation_title(&status, CaseGeometryStatus::CurrentExact),
                "Generating case…",
            );
        }
        let block = GenerationStatus::Blocked {
            job_id,
            reason: "mechanical findings block case generation".into(),
        };
        for geometry in [
            CaseGeometryStatus::Missing,
            CaseGeometryStatus::Previous,
            CaseGeometryStatus::Preview,
        ] {
            assert_eq!(
                case_generation_title(&block, geometry),
                "Case generation blocked: mechanical findings block case generation",
            );
        }
    }

    #[test]
    fn live_generation_starts_once_for_each_eligible_owner_and_honors_pause() {
        let first = owner(10, 4);
        let second = owner(11, 5);
        let mut state = AutomaticCaseGeneration {
            enabled: true,
            attempted: None,
        };

        assert!(state.observe(Some(first.clone()), true, false, false));
        assert!(!state.observe(Some(first.clone()), true, false, false));
        assert!(!state.observe(Some(first), true, false, false));
        assert!(state.observe(Some(second), true, false, false));

        state.set_enabled(false);
        assert!(!state.observe(Some(owner(12, 6)), true, false, false));
        state.set_enabled(true);
        assert!(state.observe(Some(owner(12, 6)), true, false, false));
    }

    #[test]
    fn ineligible_busy_or_reusable_contexts_do_not_start_generation() {
        let mut state = AutomaticCaseGeneration {
            enabled: true,
            attempted: None,
        };
        let current = owner(10, 4);

        assert!(!state.observe(Some(current.clone()), false, false, false));
        assert!(!state.observe(Some(current.clone()), true, false, true));
        assert!(!state.observe(Some(current.clone()), true, true, false));
        assert!(state.observe(Some(current), true, false, false));
    }

    #[test]
    fn transient_ineligibility_does_not_retry_a_terminal_owner() {
        let current = owner(10, 4);
        let mut state = AutomaticCaseGeneration {
            enabled: true,
            attempted: None,
        };

        assert!(state.observe(Some(current.clone()), true, false, false));
        assert!(!state.observe(Some(current.clone()), false, false, false));
        assert!(!state.observe(Some(current), true, false, false));
    }

    #[test]
    fn only_exact_completed_same_owner_output_rebinds() {
        let scope = owner(10, 4).scope;
        let fingerprint = Some([7; 32]);
        assert!(may_rebind_completed_case_result(
            true,
            &scope,
            SnapshotToken(10),
            fingerprint,
            &scope,
            SnapshotToken(11),
            fingerprint,
        ));
        assert!(!may_rebind_completed_case_result(
            false,
            &scope,
            SnapshotToken(10),
            fingerprint,
            &scope,
            SnapshotToken(11),
            fingerprint,
        ));
        assert!(!may_rebind_completed_case_result(
            true,
            &scope,
            SnapshotToken(10),
            fingerprint,
            &scope,
            SnapshotToken(11),
            Some([8; 32]),
        ));
    }

    #[test]
    fn previous_geometry_is_displayable_only_in_its_original_scope() {
        let original = owner(10, 4).scope;
        let mut other_instance = original.clone();
        other_instance.instance_id = Some("right-half".into());
        assert!(same_owner_completed_scene_for_display(
            true, &original, &original,
        ));
        assert!(!same_owner_completed_scene_for_display(
            false, &original, &original,
        ));
        assert!(!same_owner_completed_scene_for_display(
            true,
            &original,
            &other_instance,
        ));
    }

    #[test]
    fn physical_fingerprint_ignores_revision_but_tracks_board_geometry_inputs() {
        let scope = Scope {
            session_epoch: SessionEpoch(2),
            document_id: "keyboard".into(),
            board_id: "left".into(),
            instance_id: None,
        };
        let mut document = ProjectDoc::empty("keyboard", "Test");
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
        let scene = |revision| SceneDelta {
            module_scenes: vec![],
            revision,
            transaction_id: format!("revision-{revision}"),
            changed_ids: vec![],
            transforms: vec![],
            matrix_scenes: vec![],
            contours: vec![],
            board_contours: vec![BoardContours {
                board_id: "left".into(),
                contours: vec![],
            }],
            board_readiness: vec![BoardReadiness {
                board_id: "left".into(),
                outline: true,
                pcb: true,
                case_ready: true,
            }],
            board_outline_scenes: vec![],
            finding_markers: vec![],
            findings: vec![],
            readiness: Readiness {
                layout: true,
                outline: true,
                pcb: true,
                case_ready: true,
            },
        };
        let original = case_fingerprint_for_inputs(&document, &scene(4), &scope).unwrap();
        document.revision = 5;
        document.name = "Renamed without physical change".into();
        let next_revision = case_fingerprint_for_inputs(&document, &scene(5), &scope).unwrap();
        assert_eq!(original, next_revision);

        document.boards[0].thickness = 1.2;
        let changed_geometry = case_fingerprint_for_inputs(&document, &scene(5), &scope).unwrap();
        assert_ne!(original, changed_geometry);
    }
}
