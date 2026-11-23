//! Macro edit requests and their resolver.
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope};
use boardstudio_model::model::{EditOperation, KeymapChange, KeymapMacro, MacroChange, MacroStep};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroEditAction {
    Add {
        tap_ms: u32,
        wait_ms: u32,
        steps: Vec<MacroStep>,
    },
    Remove {
        macro_id: String,
    },
    Change {
        macro_id: String,
        change: MacroChange,
    },
}

pub(crate) fn macro_resolver(scope: Scope, action: MacroEditAction, seed: u64) -> EditResolver {
    EditResolver::new("keymap-macro", move |accepted: &AcceptedSnapshot| {
        if !accepted
            .document
            .boards
            .iter()
            .any(|b| b.id == scope.board_id)
        {
            return Resolution::Retire("This board no longer exists.".into());
        }
        let macros = accepted
            .document
            .keymap
            .as_ref()
            .map_or(&[][..], |m| m.macros.as_slice());
        let change = match &action {
            MacroEditAction::Add {
                tap_ms,
                wait_ms,
                steps,
            } => {
                if macros.len() >= 128 {
                    return Resolution::Retire("The keymap already has 128 macros.".into());
                }
                let mut id = format!("keymap-macro-{seed}");
                let mut suffix = 0u64;
                while macros.iter().any(|m| m.id == id) {
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
            MacroEditAction::Remove { macro_id } => {
                if !macros.iter().any(|m| m.id == *macro_id) {
                    return Resolution::Retire("This macro no longer exists.".into());
                }
                KeymapChange::RemoveMacro {
                    id: macro_id.clone(),
                }
            }
            MacroEditAction::Change { macro_id, change } => {
                if !macros.iter().any(|m| m.id == *macro_id) {
                    return Resolution::Retire("This macro no longer exists.".into());
                }
                KeymapChange::EditMacro {
                    macro_id: macro_id.clone(),
                    change: change.clone(),
                }
            }
        };
        Resolution::submit(
            vec![scope.board_id.clone()],
            EditOperation::EditKeymap { change },
        )
    })
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use boardstudio_application::Event;
    use boardstudio_model::model::{KeyBinding, ProjectDoc};
    use boardstudio_web_runtime::runtime::Runtime;

    #[test]
    fn add_edit_and_remove_macro_round_trip() {
        let mut doc = ProjectDoc::empty("macro-edits", "Macros");
        doc.boards.push(
            serde_json::from_value(serde_json::json!({
                "id": "board", "name": "Board", "outlineIds": [], "partIds": [],
                "netIds": [], "thickness": 1.6, "traces": [], "vias": []
            }))
            .unwrap(),
        );
        let rt = Runtime::with_core(boardstudio_web_test_runtime::engine_core_services());
        rt.submit(Event::Open {
            operation_id: rt.operation(),
            document: doc,
        });
        let mut scope = rt.scope().unwrap();
        scope.board_id = "board".into();
        let add = MacroEditAction::Add {
            tap_ms: 40,
            wait_ms: 40,
            steps: vec![MacroStep::Tap {
                binding: KeyBinding::KeyPress {
                    keycode: "A".into(),
                },
            }],
        };
        assert!(
            rt.commit("keymap-macro", macro_resolver(scope.clone(), add, 1))
                .is_ok()
        );
        let id = rt
            .model()
            .accepted
            .unwrap()
            .document
            .keymap
            .as_ref()
            .unwrap()
            .macros[0]
            .id
            .clone();
        assert!(
            rt.commit(
                "keymap-macro",
                macro_resolver(
                    scope.clone(),
                    MacroEditAction::Change {
                        macro_id: id.clone(),
                        change: MacroChange::Name {
                            value: "Copy".into()
                        }
                    },
                    2
                )
            )
            .is_ok()
        );
        assert_eq!(
            rt.model()
                .accepted
                .unwrap()
                .document
                .keymap
                .as_ref()
                .unwrap()
                .macros[0]
                .name,
            "Copy"
        );
        assert!(
            rt.commit(
                "keymap-macro",
                macro_resolver(scope, MacroEditAction::Remove { macro_id: id }, 3)
            )
            .is_ok()
        );
        assert!(
            rt.model()
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
}
