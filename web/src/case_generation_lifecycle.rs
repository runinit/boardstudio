//! Admission state for automatic Case preview generation.
//!
//! Session remains the authority for accepting a generation request. This
//! state only prevents the presentation from repeatedly dispatching the same
//! request after status notifications.
#[cfg(feature = "page")]
use boardstudio_application::AcceptedSnapshot;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{ProjectDoc, SceneDelta};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CaseGenerationOwner {
    pub(crate) scope: Scope,
    pub(crate) token: SnapshotToken,
    pub(crate) revision: u64,
}

#[derive(Default)]
pub(crate) struct AutomaticCaseGeneration {
    enabled: bool,
    attempted: Option<CaseGenerationOwner>,
}

/// Fingerprint the committed physical inputs that determine a Case result.
/// Snapshot tokens/revisions are intentionally excluded; owner identity is checked
/// separately before a completed result can be rebound. In-flight work never uses
/// this relaxation.
#[cfg(feature = "page")]
pub(crate) fn physical_case_fingerprint(
    snapshot: &AcceptedSnapshot,
    scope: &Scope,
) -> Option<[u8; 32]> {
    if snapshot.document.id != scope.document_id
        || snapshot.document.revision != snapshot.scene.revision
        || snapshot.session_epoch != scope.session_epoch
    {
        return None;
    }
    let document = boardstudio_web::cad_jobs::captured_case_document(snapshot, scope).ok()?;
    let scene = boardstudio_web::cad_jobs::captured_case_scene(snapshot, scope).ok()?;
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
            if let Some(generator) = value.get_mut("generator") {
                if let Some(parameters) = generator.get_mut("parameters") {
                    filter_geometry_parameters(
                        parameters,
                        terminal_names.get(definition.id.as_str()),
                    );
                }
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
    pub(crate) fn new() -> Self {
        Self {
            enabled: true,
            attempted: None,
        }
    }

    pub(crate) fn observe(
        &mut self,
        owner: Option<CaseGenerationOwner>,
        eligible: bool,
        has_reusable_result: bool,
        busy: bool,
    ) -> bool {
        if !self.enabled || !eligible || has_reusable_result || busy {
            if !eligible {
                self.attempted = None;
            }
            return false;
        }
        let Some(owner) = owner else {
            self.attempted = None;
            return false;
        };
        if self.attempted.as_ref() == Some(&owner) {
            return false;
        }
        self.attempted = Some(owner);
        true
    }

    pub(crate) fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.attempted = None;
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn enabled(&self) -> bool {
        self.enabled
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
