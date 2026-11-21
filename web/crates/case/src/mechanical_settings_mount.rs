//! Editor-lifetime owner for the Case mechanical-settings controller.
//!
//! The mounted Case panel receives only accepted mechanical values and a typed
//! request handler. Runtime remains the source of truth and the controller
//! admits and commits every request against a fresh accepted snapshot.
use super::case_viewer::CaseSelection;
use super::mechanical_settings::{
    MechanicalBoardMismatch, MechanicalFindingRow, MechanicalFitPart, MechanicalGasketSupportRow,
    MechanicalHardwareMount, MechanicalLayerRow, MechanicalProcessTarget, MechanicalProfileChoice,
    MechanicalProfileExtractionPort, MechanicalProfileTargetChoice, MechanicalSettingsFeedback,
    MechanicalSettingsFeedbackState, MechanicalSettingsIdentity, MechanicalSettingsProps,
    MechanicalSettingsValues, MechanicalStabilizerFit,
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
use boardstudio_application::{AcceptedSnapshot, Durability, Lifecycle, Scope, SnapshotToken};
use boardstudio_model::model::{
    HardwareTransport, MechanicalAssembly, MechanicalBottomStyle, MechanicalConfiguration,
    MechanicalMount, MechanicalStabilizerKind, MechanicalStabilizerOverride, Mount, Part, PartKind,
    ProjectDoc,
};
#[cfg(test)]
use boardstudio_web_ui_shared::ui::button::Button;
use dioxus::prelude::*;
use std::{
    cell::{Cell, RefCell},
    future::Future,
    pin::Pin,
    rc::Rc,
};
use wasm_bindgen_futures::spawn_local;

type LocalFuture<T> = Pin<Box<dyn Future<Output = T> + 'static>>;
type MountingHoleLoader =
    Rc<dyn Fn() -> LocalFuture<Result<Rc<boardstudio_model::model::PartDefinition>, String>>>;
type PresentationKey = (&'static str, u64, Option<Scope>);

#[cfg(test)]
thread_local! {
    static SOURCE_PROJECTIONS: Cell<usize> = const { Cell::new(0) };
}

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
pub struct MechanicalSettingsMount {
    pub props: Option<MechanicalSettingsProps>,
    pub generation_ready: bool,
}

#[derive(Clone)]
pub struct MechanicalFindingNavigation {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
    pub resolution: Option<Rc<MechanicalResolution>>,
    pub finding_id: String,
}

/// Keep the controller and its request sequence alive for the whole Editor,
/// even when the Case workspace or its panel is hidden.
pub fn use_mechanical_settings_mount(
    runtime: Rc<Runtime>,
    generation: Signal<u64>,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
    case_selection: CaseSelection,
    on_show_finding: EventHandler<MechanicalFindingNavigation>,
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

    // The reviewed link/rejoin projection needs the normalized mounting-hole template
    // whenever the reviewed result creates generated closure holes. Load it once, like
    // the controller does per request, so the review can show the complete result.
    let review_mounting_hole = use_signal(|| None::<Rc<boardstudio_model::model::PartDefinition>>);
    use_hook({
        let mut review_mounting_hole = review_mounting_hole;
        move || {
            spawn_local(async move {
                if let Ok(definition) = parts::load_mounting_hole_definition().await {
                    review_mounting_hole.set(Some(definition));
                }
            });
        }
    });
    let source_key = (workspace() == "Case")
        .then(|| current_source_key(&runtime))
        .flatten();
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
        let runtime_for_operation = runtime.clone();
        let alive = alive.clone();
        let current = current.clone();
        move || {
            let load_mounting_hole: MountingHoleLoader =
                Rc::new(|| Box::pin(parts::load_mounting_hole_definition()));
            let submit_runtime = runtime_for_operation.clone();
            let edit_port: Rc<dyn boardstudio_web_runtime::edit_ticket::EditTicketPort> =
                Rc::new(submit_runtime);
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
                let record_is_pending_submission =
                    record.feedback.state == MechanicalSettingsFeedbackState::Pending;
                let pending_identity = record.feedback.identity.clone();
                let pending_field = record.feedback.field_id.clone();
                let pending_request = record.feedback.request_id;
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
                if record_is_pending_submission {
                    // A newer submission for a field supersedes the pending feedback of its
                    // older requests, so pending entries cannot accumulate across edits.
                    entries.retain(|entry| {
                        !(entry.feedback.state == MechanicalSettingsFeedbackState::Pending
                            && entry.feedback.identity == pending_identity
                            && entry.feedback.field_id == pending_field
                            && entry.feedback.request_id < pending_request)
                    });
                }
                entries.sort_by_key(|record| record.feedback.request_id);
                // Keep every pending request; bound only terminal feedback history.
                while entries
                    .iter()
                    .filter(|entry| {
                        entry.feedback.state != MechanicalSettingsFeedbackState::Pending
                    })
                    .count()
                    > 12
                {
                    if let Some(index) = entries.iter().position(|entry| {
                        entry.feedback.state != MechanicalSettingsFeedbackState::Pending
                    }) {
                        entries.remove(index);
                    }
                }
                feedback.set(Rc::from(entries));
            });
            MechanicalSettingsController::new(MechanicalSettingsPorts {
                current: current.clone(),
                load_mounting_hole,
                edit_port,
                publish,
            })
        }
    });
    use_context_provider(|| controller.clone());

    let version = use_context::<Signal<u64>>();
    let resolved_mechanical = use_signal(|| None::<MechanicalSettingsResolvedProjection>);
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
                reviewed_fingerprint: None,
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
                &current.accepted.document,
                &current.accepted.scene,
                &current.identity.active_board_id,
                current
                    .configuration
                    .as_deref()
                    .is_some_and(|configuration| {
                        configuration.board_id == current.identity.active_board_id
                    }),
                current.identity.scope.instance_id.as_deref(),
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
    // Keep settlement alive in every workspace; prepare panel data only while Case is visible.
    let props = current_projection.filter(|_| workspace() == "Case").map(|current| {
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
        let profile_targets = profile_targets(
            &current.accepted,
            &current.identity.active_board_id,
            configuration.map(|value| &**value),
        );
        let process_targets = configuration.map_or_else(Vec::new, |configuration| {
            process_targets(&current.accepted.document, configuration)
        });
        let stabilizer_fits = configuration.map_or_else(Vec::new, |configuration| {
            stabilizer_fits(&current.accepted.document, configuration)
        });
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
        let fit_parts = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.fit_parts.clone());
        let fit_parts_resolved = scene_rows
            .as_ref()
            .is_some_and(|rows| rows.fit_parts_resolved);
        let hardware_mounts = scene_rows
            .as_ref()
            .map_or_else(|| Rc::from([]), |rows| rows.hardware_mounts.clone());
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
        let enabled = current.editable;
        let disabled_reason = disabled_reason(
            &runtime,
            workspace,
            instance_selection,
            &current,
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
            let current_resolution = current_resolution.clone();
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
                let resolved = current_resolution.as_ref().filter(|projection| {
                    projection_matches_identity(projection, &rendered_identity)
                });
                let scene = runtime.cad_scene().filter(|scene| {
                    scene.exact
                        && scene.scope == live.identity.scope
                        && scene.token == live.identity.snapshot_token
                        && scene.snapshot.document.revision == live.identity.revision
                });
                let exists = resolved.is_some_and(|projection| {
                    resolved_projection_layer_is_selectable(projection, &rendered_identity, &id)
                }) || scene.as_ref().is_some_and(|scene| {
                    scene.mechanical.as_ref().is_some_and(|assembly| {
                        mechanical_assembly_has_layer(assembly, &id)
                            && (!id.starts_with("gasket:")
                                || scene.result.bodies.iter().any(|body| body.id == id))
                    })
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
            let current_resolution = current_resolution.clone();
            move |id: String| {
                if alive.get()
                    && workspace() == "Case"
                    && current_reader().is_some_and(|live| live.identity == rendered_identity)
                {
                    let resolution = current_resolution
                        .as_ref()
                        .filter(|projection| {
                            projection.identity == rendered_identity
                                && projection.key.scope == rendered_identity.scope
                                && projection.key.token == rendered_identity.snapshot_token
                                && projection.key.revision == rendered_identity.revision
                        })
                        .map(|projection| projection.resolution.clone());
                    if resolution.as_ref().is_some_and(|resolution| {
                        !resolution
                            .assembly
                            .diagnostics
                            .iter()
                            .any(|finding| finding.id == id)
                    }) {
                        return;
                    }
                    on_show_finding.call(MechanicalFindingNavigation {
                        scope: rendered_identity.scope.clone(),
                        token: rendered_identity.snapshot_token,
                        revision: rendered_identity.revision,
                        resolution,
                        finding_id: id,
                    });
                }
            }
        });
        let extract_profile = {
            let runtime = runtime.clone();
            let alive = alive.clone();
            let current_reader = current_reader.clone();
            MechanicalProfileExtractionPort(Rc::new(move |definition_id: String, mappings: Vec<boardstudio_model::model::MechanicalPurposeMapping>| -> LocalFuture<Result<boardstudio_model::model::MechanicalExtraction, String>> {
                let Some(before) = current_reader() else {
                    return Box::pin(async { Err("The accepted Case profile scope is no longer available.".into()) });
                };
                let Some(definition) = before.accepted.document.definitions.iter().find(|definition| definition.id == definition_id) else {
                    return Box::pin(async { Err("The selected KiCad part definition is no longer available.".into()) });
                };
                let Some(source) = definition.kicad_source.as_ref().map(|source| source.source.clone()) else {
                    return Box::pin(async { Err("The selected part has no KiCad source geometry.".into()) });
                };
                let identity = before.identity.clone();
                let operation_id = runtime.operation().0;
                let request_id = format!("case-profile-extract-{operation_id}");
                let runtime = runtime.clone();
                let alive = alive.clone();
                let current_reader = current_reader.clone();
                Box::pin(async move {
                    let result = runtime.extract_mechanical_profile(request_id, source, mappings, 0.005).await?;
                    let still_current = alive.get() && current_reader().is_some_and(|current| {
                        current.identity == identity
                            && current.configuration.as_ref().is_some_and(|configuration| configuration.profiles.iter().any(|profile| profile.definition_id == definition_id))
                    });
                    if !still_current {
                        return Err("The Case scope or selected profile changed while reading KiCad geometry.".into());
                    }
                    Ok(result)
                })
            }))
        };
        let transport = current
            .accepted
            .document
            .hardware
            .as_ref()
            .map_or(HardwareTransport::None, |hardware| hardware.transport);
        // The selected half's pair features come from the accepted pair store, so the
        // controls render the same states Core resolves without a second store.
        let pair_features = current
            .identity
            .scope
            .instance_id
            .as_deref()
            .and_then(|instance_id| {
                crate::mechanical_pair::pair_projection(
                    &current.accepted.document,
                    &current.accepted.scene.board_contours,
                    instance_id,
                )
            })
            .map(|projection| Rc::<[crate::mechanical_pair::MechanicalPairFeatureRow]>::from(
                projection.features,
            ));
        // A paired half disables by unlinking and disabling only itself; the panel needs
        // the counterpart to say so explicitly.
        let pair_counterpart = current
            .identity
            .scope
            .instance_id
            .as_deref()
            .and_then(|instance_id| {
                crate::mechanical_pair::pair_projection(
                    &current.accepted.document,
                    &current.accepted.scene.board_contours,
                    instance_id,
                )
            })
            .and_then(|projection| {
                projection
                    .members
                    .iter()
                    .find(|member| {
                        Some(member.as_str()) != current.identity.scope.instance_id.as_deref()
                    })
                    .cloned()
            });
        // Whether the selected half still uses the legacy shared-construction overlay.
        let construction_is_legacy = current
            .identity
            .scope
            .instance_id
            .as_deref()
            .and_then(|instance_id| {
                current
                    .accepted
                    .document
                    .hardware
                    .as_ref()
                    .and_then(|hardware| {
                        hardware
                            .instances
                            .iter()
                            .find(|instance| instance.id == instance_id)
                    })
            })
            .is_some_and(|instance| {
                instance.construction_ownership
                    == boardstudio_model::model::MechanicalConstructionOwnership::Legacy
            });
        // The review's read-only projection: the mount owns the accepted document, its
        // contours and the normalized template, so the panel never carries them and the
        // transient projection result never reaches Session or the accepted state.
        let pair_projection_port = crate::mechanical_pair::MechanicalPairProjectionPort(
            Rc::new({
                let document = current.accepted.document.clone();
                let board_contours = current.accepted.scene.board_contours.clone();
                let mounting_hole = review_mounting_hole.read().clone();
                let open_scratch = runtime.core_services().open_scratch;
                move |change: &boardstudio_core_mechanical::mechanical::change::MechanicalChange| {
                    crate::mechanical_pair::link_preview(
                        &document,
                        &board_contours,
                        change,
                        mounting_hole.as_deref(),
                        open_scratch,
                    )
                }
            }),
        );
        // An unpaired half offers the candidate links it could establish; a paired
        // half uses its per-feature actions instead.
        let pair_link_options = current.identity.scope.instance_id.as_deref().map(|instance_id| {
            Rc::<[crate::mechanical_pair::MechanicalPairLinkOption]>::from(
                crate::mechanical_pair::pair_link_options(
                    &current.accepted.document,
                    &current.accepted.scene.board_contours,
                    instance_id,
                    review_mounting_hole.read().as_deref(),
                    runtime.core_services().open_scratch,
                ),
            )
        });
        let preview_values = if configuration.is_none() {
            boardstudio_core_mechanical::mechanical::effective_inputs(&current.accepted.document, &current.accepted.scene.board_contours, &boardstudio_core_mechanical::mechanical::MechanicalTarget::Board { board_id: current.identity.active_board_id.clone() }).ok().map(|inputs| settings_values(&inputs.configuration, transport))
        } else { None };
        MechanicalSettingsProps {
            preview_values,
            identity: current.identity.clone(),
            request_sequence,
            values: configuration
                .map(|configuration| settings_values(configuration.as_ref(), transport)),
            profiles,
            extract_profile: Some(extract_profile),
            profile_targets: Rc::from(profile_targets),
            process_targets: Rc::from(process_targets),
            stabilizer_fits: Rc::from(stabilizer_fits),
            layers,
            gasket_supports,
            fit_parts,
            fit_parts_resolved,
            hardware_mounts,
            suggested_mounts,
            findings,
            pair_features,
            pair_link_options,
            pair_counterpart,
            construction_is_legacy,
            pair_projection_port: Some(pair_projection_port),
            selected_layer,
            mismatch,
            editable: enabled,
            disabled_reason,
            feedback: visible_feedback,
            summary_feedback,
            on_request,
            on_select_layer,
            on_show_finding,
        }
    });

    MechanicalSettingsMount {
        props,
        generation_ready,
    }
}

#[path = "mechanical_settings_mount/projection.rs"]
mod projection;
#[cfg(test)]
use projection::project_layer_rows;
use projection::{
    MechanicalSettingsResolvedProjection, MechanicalSettingsSourceProjection, SettingsSourceKey,
    current_settings, current_source_key, disabled_reason, mechanical_assembly_has_layer,
    process_targets, profile_targets, project_scene_rows, project_settings_source,
    projection_matches_identity, resolved_projection_layer_is_selectable, settings_values,
    stabilizer_fits,
};

#[cfg(test)]
#[path = "mechanical_settings_mount/tests.rs"]
mod tests;
