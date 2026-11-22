//! Atomic actions for a primary-key selection spanning several matrices.
use super::{SelectionAdapter, WorkspaceState, objects};
use crate::runtime::Runtime;
use boardstudio_application::{EditResolver, Event, ReadModel, Resolution, Scope, SelectionMode};
use boardstudio_core::model::EditOperation;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};

use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

type Cells = BTreeMap<String, Vec<(u32, u32)>>;

#[derive(Clone, PartialEq)]
struct Owner {
    scope: Scope,
    generation: u64,
    context: objects::ScopedTreeContext,
}

#[derive(Clone)]
struct Targets {
    cells: Cells,
    shapes: BTreeMap<String, (u32, u32)>,
    ids: Vec<String>,
}

#[derive(Clone, Copy, PartialEq)]
enum Action {
    Enable,
    Disable,
    Delete,
}

fn owner(
    runtime: &Runtime,
    adapter: &SelectionAdapter,
    workspace: Signal<&'static str>,
) -> Option<Owner> {
    if workspace() != "Layout" {
        return None;
    }
    let scope = runtime.scope()?;
    let context = (adapter.selected_context)()?;
    if context.scope != scope {
        return None;
    }
    Some(Owner {
        scope,
        generation: (adapter.generation)(),
        context,
    })
}

/// Only actual primary MatrixScene members participate. A companion or independent
/// part in the selection makes the entire bulk action ineligible.
fn targets(model: &ReadModel, ids: &[String]) -> Option<Targets> {
    if ids.is_empty() {
        return None;
    }
    let accepted = model.accepted.as_ref()?;
    let mut result = Targets {
        cells: BTreeMap::new(),
        shapes: BTreeMap::new(),
        ids: ids.to_vec(),
    };
    for id in ids {
        let (scene, cell) = accepted.scene.matrix_scenes.iter().find_map(|scene| {
            scene
                .cells
                .iter()
                .find(|cell| {
                    cell.member_id.as_ref() == Some(id)
                        || (!cell.enabled
                            && *id
                                == format!(
                                    "matrix/{}/r{}c{}",
                                    scene.matrix_id, cell.row, cell.column
                                ))
                })
                .map(|cell| (scene, cell))
        })?;
        let matrix = accepted.document.matrices.iter().find(|matrix| {
            matrix.id == scene.matrix_id
                && matrix.board_id.as_deref() == Some(model.active_board_id.as_str())
        })?;
        if matrix
            .cells
            .iter()
            .any(|saved| saved.row == cell.row && saved.column == cell.column && saved.deleted)
        {
            return None;
        }
        result
            .cells
            .entry(matrix.id.clone())
            .or_default()
            .push((cell.row, cell.column));
        result
            .shapes
            .insert(matrix.id.clone(), (matrix.rows, matrix.columns));
    }
    for cells in result.cells.values_mut() {
        cells.sort_unstable();
        cells.dedup();
    }
    Some(result)
}

fn topology_is_current(model: &ReadModel, captured: &Targets) -> bool {
    let Some(accepted) = model.accepted.as_ref() else {
        return false;
    };
    captured.shapes.iter().all(|(id, shape)| {
        accepted.document.matrices.iter().any(|matrix| {
            matrix.id == *id
                && (matrix.rows, matrix.columns) == *shape
                && matrix.board_id.as_deref() == Some(model.active_board_id.as_str())
        })
    }) && targets(model, &captured.ids).is_some_and(|current| current.cells == captured.cells)
}

fn retained_is_current(model: &ReadModel, captured: &Targets) -> bool {
    topology_is_current(model, captured)
        && model
            .selected_part_ids
            .iter()
            .all(|id| captured.ids.contains(id))
        && captured
            .ids
            .iter()
            .filter(|id| !model.selected_part_ids.contains(id))
            .all(|id| {
                model.accepted.as_ref().is_some_and(|accepted| {
                    accepted.scene.matrix_scenes.iter().any(|scene| {
                        scene.cells.iter().any(|cell| {
                            !cell.enabled
                                && *id
                                    == format!(
                                        "matrix/{}/r{}c{}",
                                        scene.matrix_id, cell.row, cell.column
                                    )
                        })
                    })
                })
            })
}

#[component]
pub(super) fn SelectedKeysActions() -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let adapter = use_context::<SelectionAdapter>();
    let workspace = use_context::<WorkspaceState>().0;
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let retained = use_hook(|| Rc::new(RefCell::new(None::<(Owner, Targets)>)));
    let mut failure = use_signal(|| None::<String>);
    use_effect({
        let retained = retained.clone();
        let selected_context = adapter.selected_context;
        move || {
            let _selection = selected_context.read();
            retained.borrow_mut().take();
            failure.set(None);
        }
    });
    let current_owner = owner(&runtime, &adapter, workspace);
    let model = runtime.model();
    if retained
        .borrow()
        .as_ref()
        .is_some_and(|(held_owner, captured)| {
            current_owner.as_ref() != Some(held_owner) || !retained_is_current(&model, captured)
        })
    {
        retained.borrow_mut().take();
    }
    let projected = current_owner
        .as_ref()
        .and_then(|current| {
            let held = retained.borrow();
            if let Some((held_owner, held_targets)) = held.as_ref()
                && held_owner == current
                && retained_is_current(&model, held_targets)
            {
                return Some(held_targets.clone());
            }
            targets(&model, &model.selected_part_ids)
        })
        .filter(|selection| selection.cells.len() > 1);
    let (Some(current_owner), Some(projected)) = (current_owner, projected) else {
        return rsx! {};
    };
    let count = projected.cells.values().map(Vec::len).sum::<usize>();
    let matrix_count = projected.cells.len();
    let on_action = EventHandler::new(move |action: Action| {
        if owner(&runtime, &adapter, workspace).as_ref() != Some(&current_owner) {
            return;
        }
        let captured = projected.clone();
        let model = runtime.model();
        let current_targets = retained
            .borrow()
            .as_ref()
            .filter(|(held_owner, held)| {
                held_owner == &current_owner && retained_is_current(&model, held)
            })
            .map(|(_, held)| held.clone())
            .or_else(|| targets(&model, &model.selected_part_ids));
        if !topology_is_current(&model, &captured)
            || current_targets.is_none_or(|current| current.cells != captured.cells)
        {
            return;
        }
        *retained.borrow_mut() = Some((current_owner.clone(), captured.clone()));
        let selected_ids = captured.ids.clone();
        let board_id = current_owner.scope.board_id.clone();
        let resolver = EditResolver::new("selected-keys-action", move |accepted| {
            let accepted_model = ReadModel {
                accepted: Some(accepted.clone()),
                active_board_id: board_id.clone(),
                ..ReadModel::default()
            };
            if !topology_is_current(&accepted_model, &captured) {
                return Resolution::Retire(
                    "The selected matrix structure changed before this edit ran.".into(),
                );
            }
            let operation = match action {
                Action::Enable | Action::Disable => EditOperation::SetMatrixCellsEnabled {
                    cells: captured.cells.clone(),
                    enabled: action == Action::Enable,
                },
                Action::Delete => EditOperation::RemoveSelectedMatrixCells {
                    cells: captured.cells.clone(),
                },
            };
            Resolution::submit(captured.cells.keys().cloned().collect(), operation)
        });
        let accepted = match runtime.commit("selected-keys-action", resolver) {
            Ok(accepted) => accepted,
            Err(message) => {
                failure.set(Some(message));
                return;
            }
        };
        failure.set(None);
        let ids = match action {
            Action::Delete => {
                retained.borrow_mut().take();
                let mut selected_context = adapter.selected_context;
                selected_context.set(None);
                Vec::new()
            }
            Action::Enable | Action::Disable => selected_ids
                .into_iter()
                .filter(|id| {
                    accepted.scene.matrix_scenes.iter().any(|scene| {
                        scene
                            .cells
                            .iter()
                            .any(|cell| cell.enabled && cell.member_id.as_ref() == Some(id))
                    })
                })
                .collect(),
        };
        if runtime.model().selected_part_ids != ids {
            runtime.submit(Event::SelectParts {
                operation_id: runtime.operation(),
                part_ids: ids,
                range_part_ids: vec![],
                mode: SelectionMode::Replace,
            });
        }
    });
    rsx! {
        section {
            class: "grid content-start gap-3 min-w-0 px-4 pt-3 pb-5 text-foreground",
            aria_label: "Selected keys across matrices",
            onkeydown: move |event: KeyboardEvent| {
                if event.key().to_string() == " " {
                    event.stop_propagation();
                    if let Some(raw) = event.data().try_as_web_event() {
                        raw.stop_propagation();
                    }
                }
            },
            h3 { class: "m-0", "{count} keys across {matrix_count} matrices" }
            div {
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    aria_label: "Enable selected keys",
                    onclick: move |_| on_action.call(Action::Enable),
                    "Enable"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    aria_label: "Disable selected keys",
                    onclick: move |_| on_action.call(Action::Disable),
                    "Disable"
                }
                Button {
                    variant: ButtonVariant::Secondary,
                    button_type: "button",
                    aria_label: "Delete selected keys",
                    onclick: move |_| on_action.call(Action::Delete),
                    "Delete keys"
                }
            }
            if let Some(message) = failure() {
                Alert { variant: AlertVariant::Destructive,AlertDescription {  "{message}"  } }
            }
        }
    }
}

#[cfg(test)]
mod tests;
