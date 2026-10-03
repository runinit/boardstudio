//! Editor-lifetime owner for the Case mechanical-settings controller.
//!
//! The mounted Case panel receives only accepted mechanical values and a typed
//! request handler. Runtime remains the source of truth and the controller
//! admits and commits every request against a fresh accepted snapshot.
use super::case_viewer::CaseSelection;
use super::mechanical_settings::{
    MechanicalBoardMismatch, MechanicalFindingRow, MechanicalGasketSupportRow, MechanicalLayerRow,
    MechanicalProfileChoice, MechanicalSettingsFeedback, MechanicalSettingsFeedbackState,
    MechanicalSettingsIdentity, MechanicalSettingsProps, MechanicalSettingsValues,
};
use super::mechanical_settings_controller::{
    MechanicalResolution, MechanicalSettingsController, MechanicalSettingsCurrent,
    MechanicalSettingsPorts,
};
use super::{InstanceSelection, parts};
use crate::mechanical_feedback::{
    FeedbackRecord, field_feedback, relevant_summary, same_feedback_owner,
};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::{AcceptedSnapshot, Durability, Event, Lifecycle, OperationId, Scope};
use boardstudio_core::model::{
    EditCommand, EditOperation, EditPhase, HardwareTransport, MechanicalAssembly,
    MechanicalBottomStyle, MechanicalConfiguration, MechanicalMount, Mount, Part, PartKind,
    ProjectDoc,
};
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

type LocalFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;
type MechanicalResolver = Rc<
    dyn Fn(
        AcceptedSnapshot,
        Scope,
        ProjectDoc,
    ) -> LocalFuture<Result<MechanicalResolution, String>>,
>;
type MountingHoleLoader =
    Rc<dyn Fn() -> LocalFuture<Result<Rc<boardstudio_core::model::PartDefinition>, String>>>;
type PresentationKey = (&'static str, u64, Option<Scope>);

struct CurrentSettingsOwner {
    runtime: Rc<Runtime>,
    alive: Rc<Cell<bool>>,
    source_projection: Memo<Option<MechanicalSettingsSourceProjection>>,
    editor_instance_id: u64,
    base_generation: Signal<u64>,
    presentation_generation: Rc<Cell<u64>>,
    presentation_key: Rc<RefCell<Option<PresentationKey>>>,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
}

#[derive(Clone, PartialEq)]
pub(crate) struct MechanicalSettingsMount {
    pub(crate) props: Option<MechanicalSettingsProps>,
    pub(crate) generation_ready: bool,
}

/// Keep the controller and its request sequence alive for the whole Editor,
/// even when the Case workspace or its panel is hidden.
pub(crate) fn use_mechanical_settings_mount(
    runtime: Rc<Runtime>,
    generation: Signal<u64>,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
    case_selection: CaseSelection,
    on_show_finding: EventHandler<String>,
    on_show_configured_board: EventHandler<String>,
) -> MechanicalSettingsMount {
    let editor_instance_id = use_hook({
        let runtime = runtime.clone();
        move || runtime.operation().0
    });
    let request_sequence = use_signal(|| 0_u64);
    let feedback = use_signal(|| Rc::<[FeedbackRecord]>::from([]));
    let presentation_generation = use_hook(|| Rc::new(Cell::new(0_u64)));
    let presentation_key = use_hook(|| Rc::new(RefCell::new(None::<PresentationKey>)));
    let attempted_closure_initialization =
        use_hook(|| Rc::new(RefCell::new(Vec::<(u64, Scope)>::new())));
    let alive = use_hook(|| Rc::new(Cell::new(true)));
    use_drop({
        let alive = alive.clone();
        move || alive.set(false)
    });

    let source_key = current_source_key(&runtime);
    let source_projection = use_memo(use_reactive((&source_key,), {
        let runtime = runtime.clone();
        move |(key,)| project_settings_source(&runtime, key.as_ref())
    }));

    let current: Rc<dyn Fn() -> Option<MechanicalSettingsCurrent>> = {
        let owner = CurrentSettingsOwner {
            runtime: runtime.clone(),
            alive: alive.clone(),
            source_projection,
            editor_instance_id,
            base_generation: generation,
            presentation_generation: presentation_generation.clone(),
            presentation_key: presentation_key.clone(),
            workspace,
            instance_selection,
        };
        Rc::new(move || current_settings(&owner))
    };

    let controller = use_hook({
        let runtime_for_resolve = runtime.clone();
        let runtime_for_operation = runtime.clone();
        let alive = alive.clone();
        let current = current.clone();
        move || {
            let resolve: MechanicalResolver = Rc::new(
                move |accepted: AcceptedSnapshot, scope: Scope, proposed: ProjectDoc| {
                    let runtime = runtime_for_resolve.clone();
                    Box::pin(async move {
                        let (assembly, effective_configuration) = runtime
                            .resolve_mechanical_settings(accepted, scope, proposed)
                            .await?;
                        Ok(MechanicalResolution {
                            assembly,
                            effective_configuration,
                        })
                    })
                },
            );
            let load_mounting_hole: MountingHoleLoader =
                Rc::new(|| Box::pin(parts::load_mounting_hole_definition()));
            let submit_current = current.clone();
            let submit_runtime = runtime_for_operation.clone();
            let submit_alive = alive.clone();
            let submit_replace = Rc::new(
                move |operation_id: OperationId, base_revision: u64, document: ProjectDoc| {
                    if !submit_alive.get()
                        || submit_current().is_none_or(|current| {
                            !current.editable
                                || current.accepted.document.revision != base_revision
                                || current.accepted.document.id != document.id
                        })
                    {
                        return Err("The accepted Case document changed before the mechanical edit could be submitted.".into());
                    }
                    let outcome = submit_runtime.observe_operation(operation_id);
                    submit_runtime.submit(Event::Edit {
                        operation_id,
                        command: EditCommand {
                            base_revision,
                            transaction_id: format!(
                                "mechanical-settings-{}-{base_revision}",
                                operation_id.0
                            ),
                            phase: EditPhase::Commit,
                            target_ids: Vec::new(),
                            operation: EditOperation::ReplaceDocument {
                                document: Box::new(document),
                            },
                        },
                    });
                    Ok(outcome)
                },
            );
            let project_closure_clearance =
                Rc::new(crate::closure_clearance::project_owned_closure_clearance);
            let publish_alive = alive.clone();
            let publish_current = current.clone();
            let publish = Rc::new(move |entry: MechanicalSettingsFeedback| {
                let mut feedback = feedback;
                if !publish_alive.get() {
                    return;
                }
                let basis = publish_current()
                    .filter(|current| same_feedback_owner(&entry.identity, &current.identity))
                    .and_then(|current| current.configuration);
                let record = FeedbackRecord {
                    feedback: entry,
                    basis,
                };
                let mut entries = feedback.read().to_vec();
                if let Some(existing) = entries.iter_mut().find(|existing| {
                    existing.feedback.identity == record.feedback.identity
                        && existing.feedback.request_id == record.feedback.request_id
                        && existing.feedback.field_id == record.feedback.field_id
                }) {
                    *existing = record;
                } else {
                    entries.push(record);
                }
                entries.sort_by_key(|record| record.feedback.request_id);
                if entries.len() > 12 {
                    let active_pending = entries
                        .iter()
                        .rposition(|feedback| {
                            feedback.feedback.state == MechanicalSettingsFeedbackState::Pending
                        })
                        .map(|index| entries.remove(index));
                    if let Some(active_pending) = active_pending {
                        if entries.len() > 11 {
                            entries.drain(..entries.len() - 11);
                        }
                        entries.push(active_pending);
                        entries.sort_by_key(|record| record.feedback.request_id);
                    } else {
                        entries.drain(..entries.len() - 12);
                    }
                }
                feedback.set(Rc::from(entries));
            });
            MechanicalSettingsController::new(MechanicalSettingsPorts {
                current: current.clone(),
                resolve,
                load_mounting_hole,
                next_operation: {
                    let runtime = runtime_for_operation.clone();
                    Rc::new(move || runtime.operation())
                },
                submit_replace,
                project_closure_clearance,
                publish,
            })
        }
    });

    let version = use_context::<Signal<u64>>();
    let mut resolved_mechanical = use_signal(|| None::<MechanicalSettingsResolvedProjection>);
    let resolving_mechanical = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(SettingsSourceKey, MechanicalSettingsIdentity)>,
        ))
    });
    let settled_mechanical = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(SettingsSourceKey, MechanicalSettingsIdentity)>,
        ))
    });
    use_effect(use_reactive((&version(), &workspace()), {
        let runtime = runtime.clone();
        let current = current.clone();
        let alive = alive.clone();
        let resolving = resolving_mechanical.clone();
        let settled = settled_mechanical.clone();
        move |_| {
            if workspace() != "Case" {
                return;
            }
            let Some(current_settings) = current() else {
                return;
            };
            if current_settings.configuration.is_none() {
                return;
            }
            let Some(key) = current_source_key(&runtime) else {
                return;
            };
            let identity = current_settings.identity.clone();
            let request_key = (key.clone(), identity.clone());
            if settled.borrow().as_ref() == Some(&request_key) {
                return;
            }
            {
                let mut pending = resolving.borrow_mut();
                if pending.as_ref() == Some(&request_key) {
                    return;
                }
                *pending = Some(request_key.clone());
            }

            let accepted = current_settings.accepted.clone();
            let proposed = accepted.document.as_ref().clone();
            let scope = key.scope.clone();
            let runtime = runtime.clone();
            let current = current.clone();
            let alive = alive.clone();
            let resolving = resolving.clone();
            let settled = settled.clone();
            let mut resolved = resolved_mechanical;
            spawn_local(async move {
                let result = runtime
                    .resolve_mechanical_settings(accepted, scope, proposed)
                    .await;
                if !alive.get() {
                    return;
                }
                let still_current = current().is_some_and(|live| live.identity == identity)
                    && current_source_key(&runtime).as_ref() == Some(&key);
                {
                    let mut pending = resolving.borrow_mut();
                    if pending.as_ref() == Some(&request_key) {
                        *pending = None;
                    }
                }
                if !still_current {
                    return;
                }
                *settled.borrow_mut() = Some(request_key);
                if let Ok((assembly, effective_configuration)) = result {
                    resolved.set(Some(MechanicalSettingsResolvedProjection {
                        key,
                        identity,
                        resolution: Rc::new(MechanicalResolution {
                            assembly,
                            effective_configuration,
                        }),
                    }));
                } else {
                    resolved.set(None);
                }
            });
        }
    }));
    use_effect(use_reactive((&version(), &workspace()), {
        let controller = controller.clone();
        let runtime = runtime.clone();
        let current = current.clone();
        let attempted = attempted_closure_initialization.clone();
        let mut request_sequence = request_sequence;
        move |_| {
            // Observe navigation synchronously before settling: callback identities from the
            // previous Case mount must be stale even when Runtime Scope itself did not change.
            let _ = current();
            controller.settle();
            if controller.is_busy() {
                return;
            }
            let Some(current) = current() else {
                return;
            };
            if !current.editable {
                return;
            }
            let Some(_configuration) = current.configuration.as_ref().filter(|configuration| {
                configuration.board_id == current.identity.active_board_id
                    && configuration.closure_mounts.is_none()
                    && configuration.mount != MechanicalMount::Gasket
            }) else {
                return;
            };
            let Some(_scene) = runtime.cad_scene().filter(|scene| {
                scene.scope == current.identity.scope
                    && scene.token == current.identity.snapshot_token
                    && scene.exact
                    && scene
                        .mechanical
                        .as_ref()
                        .is_some_and(|assembly| !assembly.suggested_mounts.is_empty())
            }) else {
                return;
            };
            let owner = (
                current.identity.editor_instance_id,
                current.identity.scope.clone(),
            );
            if attempted.borrow().contains(&owner) {
                return;
            }
            let Some(request_id) = request_sequence().checked_add(1) else {
                return;
            };
            request_sequence.set(request_id);
            let request = super::mechanical_settings::MechanicalSettingsRequest {
                identity: current.identity,
                request_id,
                field_id: "initialize-closures".into(),
                patch: super::mechanical_settings::MechanicalSettingsPatch::InitializeClosures,
            };
            let admitted = controller.submit(request);
            // submit() sets the in-flight owner synchronously only after all admission guards
            // pass. A rejected/busy/unready auto-attempt remains eligible for a later render.
            if admitted {
                let mut attempted = attempted.borrow_mut();
                attempted.push(owner);
            }
        }
    }));

    let current_projection = current();
    let current_reader = current.clone();
    let scene = current_projection.as_ref().and_then(|current| {
        runtime.cad_scene().filter(|scene| {
            scene.scope == current.identity.scope
                && scene.token == current.identity.snapshot_token
                && scene.exact
        })
    });
    let display_scene = current_projection.as_ref().and_then(|current| {
        runtime.cad_scene().filter(|scene| {
            crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                scene.exact,
                &scene.scope,
                &current.identity.scope,
            )
        })
    });
    let generation_ready = current_projection.as_ref().is_some_and(|current| {
        let requires_initialization = current.configuration.as_ref().is_some_and(|configuration| {
            configuration.board_id == current.identity.active_board_id
                && configuration.closure_mounts.is_none()
                && configuration.mount != MechanicalMount::Gasket
                && scene.as_ref().is_some_and(|scene| {
                    scene
                        .mechanical
                        .as_ref()
                        .is_some_and(|assembly| !assembly.suggested_mounts.is_empty())
                })
        });
        let model = runtime.model();
        workspace() == "Case"
            && instance_selection.is_current(&model)
            && crate::case_generation_admission::is_ready(
                &current.accepted.scene,
                &current.identity.active_board_id,
                current
                    .configuration
                    .as_deref()
                    .is_some_and(|configuration| {
                        configuration.board_id == current.identity.active_board_id
                    }),
            )
            && current.lifecycle == Lifecycle::Ready
            && current.durability
                == (Durability::Saved {
                    revision: current.accepted.document.revision,
                })
            && model.display_preview.is_none()
            && model.gesture.is_none()
            && !controller.is_busy()
            && !requires_initialization
    });
    let internal_gasket = current_projection
        .as_ref()
        .and_then(|current| current.configuration.as_ref())
        .is_some_and(|configuration| configuration.internal_gasket.is_some());
    let previous_geometry = current_projection
        .as_ref()
        .zip(display_scene.as_ref())
        .is_some_and(|(current, scene)| scene.token != current.identity.snapshot_token);
    let current_resolution = current_projection.as_ref().and_then(|current| {
        resolved_mechanical
            .read()
            .as_ref()
            .filter(|projection| {
                projection.identity == current.identity
                    && projection.key.scope == current.identity.scope
                    && projection.key.token == current.identity.snapshot_token
                    && projection.key.revision == current.identity.revision
            })
            .cloned()
    });
    let scene_rows = use_memo(use_reactive(
        (
            &display_scene,
            &scene,
            &current_resolution,
            &internal_gasket,
            &previous_geometry,
        ),
        |(display, current, resolved, gasket, is_previous)| {
            project_scene_rows(
                display.as_ref(),
                current.as_ref(),
                resolved.as_ref().map(|projection| &projection.resolution),
                gasket,
                is_previous,
            )
        },
    ));
    let props = current_projection.map(|current| {
        let configuration = current.configuration.as_ref();
        let mismatch = configuration
            .filter(|configuration| configuration.board_id != current.identity.active_board_id)
            .map(|configuration| MechanicalBoardMismatch {
                board_id: configuration.board_id.clone(),
                board_name: current
                    .accepted
                    .document
                    .boards
                    .iter()
                    .find(|board| board.id == configuration.board_id)
                    .map_or_else(
                        || configuration.board_id.clone(),
                        |board| board.name.clone(),
                    ),
            });
        let profiles = source_projection
            .read()
            .as_ref()
            .filter(|projection| {
                projection.key.scope == current.identity.scope
                    && projection.key.token == current.identity.snapshot_token
                    && projection.key.revision == current.identity.revision
            })
            .map(|projection| projection.profiles.clone())
            .unwrap_or_else(|| Rc::from([]));
        let scene_rows = scene_rows.read();
        let layers = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.layers.clone());
        let findings = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.findings.clone());
        let gasket_supports = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.gasket_supports.clone());
        let suggested_mounts = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.suggested_mounts.clone());
        drop(scene_rows);
        let mut selected_layer = case_selection.layer_id(&current.identity.scope);
        let selected_gasket_support = gasket_supports.iter().any(|support| {
            selected_layer == format!("gasket:{}:lower", support.id)
                || selected_layer == format!("gasket:{}:upper", support.id)
        });
        if !layers.iter().any(|layer| layer.id == selected_layer)
            && !selected_gasket_support
            && !(selected_layer == "gaskets" && !gasket_supports.is_empty())
        {
            selected_layer.clear();
        }
        let enabled = current.editable && !controller.is_busy();
        let disabled_reason = disabled_reason(
            &runtime,
            workspace,
            instance_selection,
            &current,
            controller.is_busy(),
        );
        let on_request = EventHandler::new({
            let controller = controller.clone();
            let alive = alive.clone();
            move |request| {
                if alive.get() {
                    controller.submit(request);
                }
            }
        });
        let on_select_layer = EventHandler::new({
            let runtime = runtime.clone();
            let current_reader = current_reader.clone();
            let alive = alive.clone();
            let rendered_identity = current.identity.clone();
            move |id: String| {
                if !alive.get() {
                    return;
                }
                let Some(live) = current_reader() else {
                    return;
                };
                if live.identity != rendered_identity {
                    return;
                }
                if workspace() != "Case" || !instance_selection.is_current(&runtime.model()) {
                    return;
                }
                let Some(scene) = runtime.cad_scene().filter(|scene| {
                    crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                        scene.exact,
                        &scene.scope,
                        &live.identity.scope,
                    )
                }) else {
                    return;
                };
                let exists = scene.mechanical.as_ref().is_some_and(|assembly| {
                    assembly.stack.iter().any(|layer| layer.id == id)
                        || (id == "gaskets" && !assembly.gasket_supports.is_empty())
                        || (assembly.gasket_supports.iter().any(|support| {
                            id == format!("gasket:{}:lower", support.id)
                                || id == format!("gasket:{}:upper", support.id)
                        }) && scene.result.bodies.iter().any(|body| body.id == id))
                });
                if !id.is_empty() && !exists {
                    return;
                }
                case_selection.select_layer(rendered_identity.scope.clone(), id);
            }
        });
        let feedback_records = feedback.read();
        // Field owners settle by their exact submitted identity. In particular a rejected B
        // must reach its field while A is Pending, and a reconciliation failure must survive
        // the accepted token advancing. Only the global summary applies relevance filtering.
        let visible_feedback = field_feedback(&feedback_records, &current.identity);
        let summary_feedback = relevant_summary(
            &feedback_records,
            &current.identity,
            current.configuration.as_deref(),
            controller.is_busy(),
        );
        drop(feedback_records);
        let on_show_finding = EventHandler::new({
            let current_reader = current_reader.clone();
            let rendered_identity = current.identity.clone();
            let alive = alive.clone();
            move |id: String| {
                if alive.get()
                    && workspace() == "Case"
                    && current_reader().is_some_and(|live| live.identity == rendered_identity)
                {
                    on_show_finding.call(id);
                }
            }
        });
        let on_show_configured_board = EventHandler::new({
            let current_reader = current_reader.clone();
            let rendered_identity = current.identity.clone();
            let alive = alive.clone();
            move |id: String| {
                if alive.get()
                    && workspace() == "Case"
                    && current_reader().is_some_and(|live| live.identity == rendered_identity)
                {
                    on_show_configured_board.call(id);
                }
            }
        });
        let transport = current
            .accepted
            .document
            .hardware
            .as_ref()
            .map_or(HardwareTransport::None, |hardware| hardware.transport);
        MechanicalSettingsProps {
            identity: current.identity.clone(),
            request_sequence,
            values: configuration
                .map(|configuration| settings_values(configuration.as_ref(), transport)),
            profiles,
            layers,
            gasket_supports,
            suggested_mounts,
            findings,
            selected_layer,
            mismatch,
            editable: enabled,
            disabled_reason,
            feedback: visible_feedback,
            summary_feedback,
            on_request,
            on_select_layer,
            on_show_finding,
            on_show_configured_board,
        }
    });

    MechanicalSettingsMount {
        props,
        generation_ready,
    }
}

fn current_settings(owner: &CurrentSettingsOwner) -> Option<MechanicalSettingsCurrent> {
    let CurrentSettingsOwner {
        runtime,
        alive,
        source_projection,
        editor_instance_id,
        base_generation,
        presentation_generation,
        presentation_key,
        workspace,
        instance_selection,
    } = owner;
    if !alive.get() {
        return None;
    }
    let scope = runtime.scope();
    let observed_key = (workspace(), base_generation(), scope.clone());
    let presentation_generation = {
        let mut previous = presentation_key.borrow_mut();
        if previous.as_ref() != Some(&observed_key) {
            if previous.is_some() {
                presentation_generation.set(
                    presentation_generation
                        .get()
                        .checked_add(1)
                        .expect("Case presentation identity exhausted"),
                );
            }
            *previous = Some(observed_key);
        }
        presentation_generation.get()
    };
    let model = runtime.model();
    let accepted = model.accepted.clone()?;
    let scope = scope?;
    if scope.session_epoch != accepted.session_epoch
        || scope.document_id != accepted.document.id
        || scope.board_id != model.active_board_id
        || scope.instance_id != model.active_instance_id
    {
        return None;
    }
    let key = SettingsSourceKey {
        scope: scope.clone(),
        token: accepted.token,
        revision: accepted.document.revision,
    };
    let projection = source_projection.read();
    let projection = projection
        .as_ref()
        .filter(|projection| projection.key == key)?;
    let configuration = projection.configuration.clone();
    let configuration_board_id = configuration.as_ref().map_or_else(
        || scope.board_id.clone(),
        |configuration| configuration.board_id.clone(),
    );
    let saved = model.lifecycle == Lifecycle::Ready
        && model.durability
            == (Durability::Saved {
                revision: accepted.document.revision,
            });
    let editable = workspace() == "Case"
        && instance_selection.is_current(&model)
        && saved
        && model.display_preview.is_none()
        && model.gesture.is_none()
        && configuration_board_id == scope.board_id;
    Some(MechanicalSettingsCurrent {
        identity: MechanicalSettingsIdentity {
            editor_instance_id: *editor_instance_id,
            scope_generation: base_generation(),
            presentation_generation,
            scope: scope.clone(),
            snapshot_token: accepted.token,
            revision: accepted.document.revision,
            active_board_id: model.active_board_id.clone(),
            configuration_board_id,
        },
        accepted,
        configuration,
        editable,
        lifecycle: model.lifecycle,
        durability: model.durability,
    })
}

#[derive(Clone, Debug, PartialEq)]
struct SettingsSourceKey {
    scope: Scope,
    token: boardstudio_application::SnapshotToken,
    revision: u64,
}

#[derive(Clone, PartialEq)]
struct MechanicalSettingsSourceProjection {
    key: SettingsSourceKey,
    configuration: Option<Rc<MechanicalConfiguration>>,
    profiles: Rc<[MechanicalProfileChoice]>,
}

#[derive(Clone)]
struct MechanicalSettingsResolvedProjection {
    key: SettingsSourceKey,
    identity: MechanicalSettingsIdentity,
    resolution: Rc<MechanicalResolution>,
}

impl PartialEq for MechanicalSettingsResolvedProjection {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key
            && self.identity == other.identity
            && Rc::ptr_eq(&self.resolution, &other.resolution)
    }
}

#[derive(Clone, PartialEq)]
struct MechanicalSceneRows {
    layers: Rc<[MechanicalLayerRow]>,
    gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    suggested_mounts: Rc<[Mount]>,
    findings: Rc<[MechanicalFindingRow]>,
}

fn project_scene_rows(
    display_scene: Option<&Rc<CadScene>>,
    current_scene: Option<&Rc<CadScene>>,
    current_resolution: Option<&Rc<MechanicalResolution>>,
    internal_gasket: bool,
    is_previous: bool,
) -> Option<MechanicalSceneRows> {
    let display_assembly = display_scene.and_then(|scene| scene.mechanical.as_ref());
    let current_assembly: Option<&MechanicalAssembly> = current_resolution
        .map(|resolution| &resolution.assembly)
        .or_else(|| current_scene.and_then(|scene| scene.mechanical.as_ref()));
    if display_assembly.is_none() && current_assembly.is_none() {
        return None;
    }
    let layers: Vec<_> = display_assembly
        .into_iter()
        .flat_map(|assembly| assembly.stack.iter().map(move |layer| (assembly, layer)))
        .map(|(assembly, layer)| MechanicalLayerRow {
            id: layer.id.clone(),
            label: mechanical_layer_label(&layer.id, internal_gasket),
            z: layer.z,
            thickness: layer.thickness,
            resolved_body_thickness: assembly
                .case
                .bodies
                .iter()
                .find(|body| body.body.id == layer.id)
                .map(|body| body.body.thickness),
            is_previous,
        })
        .collect();
    let support_assembly = current_assembly.or(display_assembly);
    let supports_are_previous = current_assembly.is_none() && is_previous;
    let gasket_supports: Vec<_> = support_assembly
        .into_iter()
        .flat_map(|assembly| assembly.gasket_supports.iter())
        .map(|support| MechanicalGasketSupportRow {
            id: support.id.clone(),
            region_id: support.region_id.clone(),
            outline_key: support.outline_key.clone(),
            anchor: support.anchor,
            pair_id: support.pair_id.clone(),
            length: support.length,
            width: support.width,
            unlinked: support.unlinked,
            fit_error: support.fit_error.clone(),
            is_previous: supports_are_previous,
        })
        .collect();
    let findings: Vec<_> = current_assembly
        .into_iter()
        .flat_map(|assembly| assembly.diagnostics.iter())
        .map(|finding| MechanicalFindingRow {
            id: finding.id.clone(),
            severity: finding.severity.clone(),
            message: finding.message.clone(),
        })
        .collect();
    let suggested_mounts = current_assembly
        .map(|assembly| assembly.suggested_mounts.clone())
        .unwrap_or_default();
    Some(MechanicalSceneRows {
        layers: Rc::from(layers),
        gasket_supports: Rc::from(gasket_supports),
        suggested_mounts: Rc::from(suggested_mounts),
        findings: Rc::from(findings),
    })
}

fn current_source_key(runtime: &Runtime) -> Option<SettingsSourceKey> {
    let model = runtime.model();
    let accepted = model.accepted.as_ref()?;
    let scope = runtime.scope()?;
    (scope.session_epoch == accepted.session_epoch
        && scope.document_id == accepted.document.id
        && scope.board_id == model.active_board_id
        && scope.instance_id == model.active_instance_id
        && accepted.scene.revision == accepted.document.revision)
        .then(|| SettingsSourceKey {
            scope,
            token: accepted.token,
            revision: accepted.document.revision,
        })
}

fn project_settings_source(
    runtime: &Runtime,
    key: Option<&SettingsSourceKey>,
) -> Option<MechanicalSettingsSourceProjection> {
    let key = key?;
    let model = runtime.model();
    let accepted = model.accepted.as_ref()?;
    if current_source_key(runtime).as_ref() != Some(key) {
        return None;
    }
    let effective = boardstudio_web::cad_jobs::captured_case_document(accepted, &key.scope).ok()?;
    let configuration = effective.mechanical.clone().map(Rc::new);
    let profiles = configuration
        .as_ref()
        .map_or_else(Vec::new, |configuration| {
            profile_choices(accepted, configuration)
        });
    Some(MechanicalSettingsSourceProjection {
        key: key.clone(),
        configuration,
        profiles: Rc::from(profiles),
    })
}

fn settings_values(
    configuration: &MechanicalConfiguration,
    transport: HardwareTransport,
) -> MechanicalSettingsValues {
    MechanicalSettingsValues {
        board_id: configuration.board_id.clone(),
        transport,
        battery: configuration.battery.clone(),
        suspension_mounts: configuration.mounts.clone(),
        closure_mounts: configuration.closure_mounts.clone(),
        method: configuration.method.clone(),
        mount: configuration.mount.clone(),
        bottom_style: configuration
            .bottom_style
            .clone()
            .unwrap_or(MechanicalBottomStyle::Shell),
        middle_frame: configuration.middle_frame.unwrap_or(false),
        integrated_plate_frame: configuration.integrated_plate_frame,
        plate_thickness: configuration.plate_thickness,
        plate_foam_thickness: configuration.plate_foam_thickness,
        pcb_thickness: configuration.pcb_thickness,
        bottom_foam_thickness: configuration.bottom_foam_thickness,
        bottom_thickness: configuration.bottom_thickness,
        wall_thickness: configuration.wall_thickness,
        clearance: configuration.clearance,
        opening_allowance: configuration.opening_allowance.unwrap_or(0.0),
        internal_gasket: configuration.internal_gasket.is_some(),
        plate_to_pcb: configuration.plate_to_pcb,
        battery_height: configuration.battery_height,
    }
}

fn mechanical_layer_label(id: &str, internal_gasket: bool) -> String {
    if id == "retainer" && internal_gasket {
        return "Top case".into();
    }
    if id == "pcb" {
        return "PCB".into();
    }
    let label = id.replace('-', " ");
    let mut chars = label.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn profile_choices(
    accepted: &AcceptedSnapshot,
    configuration: &MechanicalConfiguration,
) -> Vec<MechanicalProfileChoice> {
    let Some(board) = accepted
        .document
        .boards
        .iter()
        .find(|board| board.id == configuration.board_id)
    else {
        return Vec::new();
    };
    configuration
        .profiles
        .iter()
        .map(|profile| {
            let definition = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == profile.definition_id);
            let placed_switch = accepted.document.parts.iter().any(|part: &Part| {
                board.part_ids.contains(&part.id)
                    && part.definition_id == profile.definition_id
                    && definition.is_some_and(|definition| {
                        definition.kind == PartKind::Switch
                            || definition.generator.as_ref().is_some_and(|generator| {
                                generator.source.ends_with("/switch_choc_v1_v2")
                            })
                    })
            });
            MechanicalProfileChoice {
                definition_id: profile.definition_id.clone(),
                name: definition.map_or_else(
                    || profile.definition_id.clone(),
                    |definition| definition.name.clone(),
                ),
                source: Some(profile.source.clone()),
                family: profile.switch_family,
                plate_to_pcb: Some(profile.plate_to_pcb),
                supported_thickness: profile.supported_thickness,
                switch_family_selectable: placed_switch,
            }
        })
        .collect()
}

fn disabled_reason(
    runtime: &Runtime,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
    current: &MechanicalSettingsCurrent,
    busy: bool,
) -> Option<String> {
    if workspace() != "Case" {
        return Some("Mechanical settings are available in the Case workspace.".into());
    }
    if !instance_selection.is_current(&runtime.model()) {
        return Some(
            "Wait for physical assembly selection before changing mechanical settings.".into(),
        );
    }
    if current.identity.configuration_board_id != current.identity.active_board_id {
        return Some("Show the configured board before changing its mechanical settings.".into());
    }
    if busy {
        return Some("Wait for the current mechanical settings change to finish.".into());
    }
    if current.lifecycle != Lifecycle::Ready {
        return Some(
            "Wait for the accepted document to finish opening before editing mechanical settings."
                .into(),
        );
    }
    let saved_revision = match &current.durability {
        Durability::Saved { revision } => Some(*revision),
        _ => None,
    };
    if saved_revision != Some(current.accepted.document.revision) {
        return Some(
            "Wait for the accepted document to finish saving before editing mechanical settings."
                .into(),
        );
    }
    let model = runtime.model();
    if model.display_preview.is_some() || model.gesture.is_some() {
        return Some(
            "Finish or cancel the active edit before changing mechanical settings.".into(),
        );
    }
    None
}
