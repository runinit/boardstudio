//! Macro edit requests and their resolver. The module compiles natively so its tests
//! drive the real Session and Core through the native Runtime; the Keymap panel binds
//! the resolver to its macro controls.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_core::model::{EditOperation, KeymapChange, KeymapMacro, MacroChange, MacroStep};
use std::rc::Rc;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MacroEditTarget {
    AddMacro,
    RemoveMacro,
    Name,
    TapMs,
    WaitMs,
    AddStep,
    RemoveStep { index: usize },
    StepKind { index: usize },
    StepDelay { index: usize },
    StepKeycode { index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroEditRequest {
    pub scope: Scope,
    pub scope_generation: u64,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub macro_id: Option<String>,
    pub target: MacroEditTarget,
    pub step_sequence: Option<Rc<[MacroStep]>>,
    pub change: MacroEditChange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroEditChange {
    Add {
        name: String,
        tap_ms: u32,
        wait_ms: u32,
        steps: Vec<MacroStep>,
    },
    Remove,
    Change(MacroChange),
}

#[derive(Clone, Copy)]
pub(crate) enum PrecedingStructure {
    Append,
    Remove(usize),
    Ambiguous,
}

pub(crate) fn macro_resolver(
    request: MacroEditRequest,
    seed: u64,
    preceding_structure: Option<PrecedingStructure>,
) -> EditResolver {
    EditResolver::new("keymap-macro", move |accepted: &AcceptedSnapshot| {
        if !accepted
            .document
            .boards
            .iter()
            .any(|board| board.id == request.scope.board_id)
        {
            return Resolution::Retire("This board no longer exists.".into());
        }
        let macros = accepted
            .document
            .keymap
            .as_ref()
            .map_or(&[][..], |map| map.macros.as_slice());
        let change = match (&request.target, &request.change) {
            (
                MacroEditTarget::AddMacro,
                MacroEditChange::Add {
                    tap_ms,
                    wait_ms,
                    steps,
                    ..
                },
            ) if request.macro_id.is_none() => {
                if macros.len() >= 128 {
                    return Resolution::Retire("The keymap already has 128 macros.".into());
                }
                let mut id = format!("keymap-macro-{seed}");
                let mut suffix = 0u64;
                while macros.iter().any(|item| item.id == id) {
                    suffix += 1;
                    id = format!("keymap-macro-{seed}-{suffix}");
                }
                KeymapChange::SaveMacro {
                    value: KeymapMacro {
                        id,
                        name: format!("Macro {}", macros.len() + 1),
                        tap_ms: *tap_ms,
                        wait_ms: *wait_ms,
                        steps: steps.clone(),
                    },
                }
            }
            _ => {
                let Some(item) = request
                    .macro_id
                    .as_ref()
                    .and_then(|id| macros.iter().find(|item| &item.id == id))
                else {
                    return Resolution::Retire("This macro no longer exists.".into());
                };
                // Appending, or removing a later step, keeps this positional target.
                // A removed/shifted target is no longer eligible; a failed structural
                // operation leaves the original position available.
                if let MacroEditTarget::RemoveStep { index }
                | MacroEditTarget::StepKind { index }
                | MacroEditTarget::StepDelay { index }
                | MacroEditTarget::StepKeycode { index } = request.target
                {
                    let valid =
                        request.step_sequence.as_ref().is_some_and(
                            |steps| match preceding_structure {
                                Some(PrecedingStructure::Append) => {
                                    item.steps.len() == steps.len()
                                        || item.steps.len() == steps.len() + 1
                                }
                                Some(PrecedingStructure::Remove(removed)) => {
                                    item.steps.len() == steps.len()
                                        || (item.steps.len() + 1 == steps.len() && index < removed)
                                }
                                Some(PrecedingStructure::Ambiguous) => false,
                                None => item.steps.len() == steps.len(),
                            },
                        );
                    if !valid {
                        return Resolution::Retire("This macro step is no longer available because the steps changed. Select the step again.".into());
                    }
                }
                match (&request.target, &request.change) {
                    (MacroEditTarget::RemoveMacro, MacroEditChange::Remove) => {
                        KeymapChange::RemoveMacro {
                            id: item.id.clone(),
                        }
                    }
                    (target, MacroEditChange::Change(change)) => {
                        let valid = match (target, change) {
                            (MacroEditTarget::Name, MacroChange::Name { value }) => {
                                if item.name == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::TapMs, MacroChange::TapMs { value }) => {
                                if item.tap_ms == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::WaitMs, MacroChange::WaitMs { value }) => {
                                if item.wait_ms == *value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            (MacroEditTarget::AddStep, MacroChange::AddStep { .. }) => {
                                item.steps.len() < 128
                            }
                            (
                                MacroEditTarget::RemoveStep { index },
                                MacroChange::RemoveStep { index: changed },
                            ) => {
                                index == changed
                                    && *index < item.steps.len()
                                    && item.steps.len() > 1
                            }
                            (
                                MacroEditTarget::StepKind { index }
                                | MacroEditTarget::StepDelay { index }
                                | MacroEditTarget::StepKeycode { index },
                                MacroChange::Step {
                                    index: changed,
                                    value,
                                },
                            ) => {
                                let Some(current) = item.steps.get(*index) else {
                                    return Resolution::Retire(
                                        "This macro step no longer exists.".into(),
                                    );
                                };
                                if index != changed {
                                    return Resolution::Retire(
                                        "This macro step is no longer available.".into(),
                                    );
                                }
                                if !matches!(target, MacroEditTarget::StepKind { .. })
                                    && std::mem::discriminant(current)
                                        != std::mem::discriminant(value)
                                {
                                    return Resolution::Retire(
                                        "This macro step field is no longer available.".into(),
                                    );
                                }
                                if current == value {
                                    return Resolution::Unchanged;
                                }
                                true
                            }
                            _ => false,
                        };
                        if !valid {
                            return Resolution::Retire(
                                "This macro field or step is no longer available.".into(),
                            );
                        }
                        KeymapChange::EditMacro {
                            macro_id: item.id.clone(),
                            change: change.clone(),
                        }
                    }
                    _ => {
                        return Resolution::Retire(
                            "This macro edit is no longer available.".into(),
                        );
                    }
                }
            }
        };
        Resolution::submit(
            vec![request.scope.board_id.clone()],
            EditOperation::EditKeymap { change },
        )
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_core::model::{KeyBinding, ProjectDoc};
    use boardstudio_web_runtime::pending_edits::{PendingEditResult, PendingEdits};
    use boardstudio_web_runtime::runtime::Runtime;

    fn open() -> (Rc<Runtime>, Scope) {
        let mut document = ProjectDoc::empty("macro-edits", "Macros");
        document.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        let runtime = Runtime::new();
        runtime.submit(Event::Open {
            operation_id: runtime.operation(),
            document,
        });
        let mut scope = runtime.scope().expect("open document has a scope");
        scope.board_id = "board".into();
        (runtime, scope)
    }

    fn request(
        runtime: &Runtime,
        scope: &Scope,
        macro_id: Option<&str>,
        target: MacroEditTarget,
        step_sequence: Option<Rc<[MacroStep]>>,
        change: MacroEditChange,
    ) -> MacroEditRequest {
        let accepted = runtime.model().accepted.unwrap();
        MacroEditRequest {
            scope: scope.clone(),
            scope_generation: 0,
            admission_token: accepted.token,
            admission_revision: accepted.document.revision,
            editor_instance_id: 1,
            request_id: 1,
            macro_id: macro_id.map(str::to_owned),
            target,
            step_sequence,
            change,
        }
    }

    fn settle(
        runtime: &Rc<Runtime>,
        request: MacroEditRequest,
        seed: u64,
        preceding: Option<PrecedingStructure>,
    ) -> PendingEditResult<u8> {
        let mut edits = PendingEdits::default();
        edits.begin(
            runtime,
            0u8,
            "keymap-macro",
            Some("macro".into()),
            macro_resolver(request, seed, preceding),
        );
        let mut results = edits.settle(true);
        assert_eq!(results.len(), 1, "{results:?}");
        results.remove(0)
    }

    fn tap() -> MacroStep {
        MacroStep::Tap {
            binding: KeyBinding::None,
        }
    }

    fn add(runtime: &Rc<Runtime>, scope: &Scope) -> String {
        let added = settle(
            runtime,
            request(
                runtime,
                scope,
                None,
                MacroEditTarget::AddMacro,
                None,
                MacroEditChange::Add {
                    name: "ignored".into(),
                    tap_ms: 30,
                    wait_ms: 40,
                    steps: vec![tap(), MacroStep::Wait { ms: 5 }],
                },
            ),
            11,
            None,
        );
        assert!(
            matches!(added, PendingEditResult::Landed { .. }),
            "{added:?}"
        );
        runtime
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .id
            .clone()
    }

    #[test]
    fn add_name_and_remove_land_and_unchanged_name_does_not_revise() {
        let (runtime, scope) = open();
        let id = add(&runtime, &scope);

        let renamed = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some(&id),
                MacroEditTarget::Name,
                None,
                MacroEditChange::Change(MacroChange::Name {
                    value: "Chord".into(),
                }),
            ),
            12,
            None,
        );
        assert!(
            matches!(renamed, PendingEditResult::Landed { .. }),
            "{renamed:?}"
        );
        let accepted = runtime.model().accepted.unwrap();
        assert_eq!(
            accepted.document.keymap.as_ref().unwrap().macros[0].name,
            "Chord"
        );
        let revision = accepted.document.revision;

        let unchanged = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some(&id),
                MacroEditTarget::Name,
                None,
                MacroEditChange::Change(MacroChange::Name {
                    value: "Chord".into(),
                }),
            ),
            13,
            None,
        );
        assert_eq!(unchanged, PendingEditResult::Landed { key: 0, revision });

        let removed = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some(&id),
                MacroEditTarget::RemoveMacro,
                None,
                MacroEditChange::Remove,
            ),
            14,
            None,
        );
        assert!(
            matches!(removed, PendingEditResult::Landed { .. }),
            "{removed:?}"
        );
        assert!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros
                .is_empty()
        );
    }

    #[test]
    fn a_step_edit_fails_when_the_sequence_changed_since_admission() {
        let (runtime, scope) = open();
        let id = add(&runtime, &scope);
        let stale: Rc<[MacroStep]> = Rc::from(vec![tap()]);
        let result = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some(&id),
                MacroEditTarget::StepDelay { index: 1 },
                Some(stale),
                MacroEditChange::Change(MacroChange::Step {
                    index: 1,
                    value: MacroStep::Wait { ms: 9 },
                }),
            ),
            15,
            None,
        );
        assert!(
            matches!(&result, PendingEditResult::Failed { message, .. } if message.contains("steps changed")),
            "{result:?}"
        );
    }

    #[test]
    fn appending_before_a_step_edit_keeps_its_position_and_missing_macros_fail() {
        let (runtime, scope) = open();
        let id = add(&runtime, &scope);
        let current: Rc<[MacroStep]> = Rc::from(vec![tap(), MacroStep::Wait { ms: 5 }]);
        let edited = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some(&id),
                MacroEditTarget::StepDelay { index: 1 },
                Some(current),
                MacroEditChange::Change(MacroChange::Step {
                    index: 1,
                    value: MacroStep::Wait { ms: 9 },
                }),
            ),
            16,
            Some(PrecedingStructure::Append),
        );
        assert!(
            matches!(edited, PendingEditResult::Landed { .. }),
            "{edited:?}"
        );
        assert_eq!(
            runtime
                .model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros[0]
                .steps[1],
            MacroStep::Wait { ms: 9 }
        );

        let missing = settle(
            &runtime,
            request(
                &runtime,
                &scope,
                Some("ghost"),
                MacroEditTarget::Name,
                None,
                MacroEditChange::Change(MacroChange::Name { value: "x".into() }),
            ),
            17,
            None,
        );
        assert!(
            matches!(&missing, PendingEditResult::Failed { message, .. } if message.contains("no longer exists")),
            "{missing:?}"
        );
    }
}
