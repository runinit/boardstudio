//! Case mechanical inspector: declarative field sections over the accepted configuration.
//!
//! `use_mechanical_settings` projects the accepted snapshot into `MechanicalSettingsProps` and
//! commits each request synchronously through `Runtime::commit`; `MechanicalSettings` renders
//! the sections directly from those props. The 3D viewer reads the same props for its handles.
use crate::mechanical_fits::{ClosureHardwareSection, HardwareAndFitsSection};
use crate::mechanical_layers::{
    ContextualLayerInspector, GasketSupportInspector, ResolvedStack, StackupPreview,
};
use crate::mechanical_pair::{
    MechanicalPairFeatureRow, MechanicalPairLinkOption, MechanicalPairProjectionPort, link_preview,
    pair_link_options, pair_projection, reviewed_fingerprint,
};
pub use crate::mechanical_profiles::MechanicalProfileExtractionPort;
use crate::mechanical_profiles::{ManufacturingSection, ProfileSection};
pub use crate::mechanical_projection::*;
pub use crate::mechanical_rules::MechanicalSettingsPatchExt;
use crate::mechanical_rules::dimension_range;
use crate::pair_controls::{PairLinkControls, PairMemberRemoval, PairRowControls};
use crate::shared_viewer::CaseSelection;
use crate::{InstanceSelection, parts};
use boardstudio_application::{
    AcceptedSnapshot, Durability, EditResolver, Lifecycle, Resolution, Scope, SnapshotToken,
};
pub use boardstudio_core_mechanical::mechanical::change::{
    MechanicalChange as MechanicalSettingsPatch, MechanicalDimension, MechanicalMountCollection,
};
use boardstudio_core_mechanical::mechanical::change::{MechanicalEdit, MechanicalTarget};
use boardstudio_core_mechanical::mechanical::{MechanicalEditDisposition, evaluate_edit};
use boardstudio_model::model::{
    EditOperation, Finding, GasketPlacement, HardwareTransport, MechanicalBottomStyle,
    MechanicalConfiguration, MechanicalConstructionOwnership, MechanicalGasketAnchor,
    MechanicalGasketSupport, MechanicalMount, Mount, MountKind, PartDefinition, PlateMethod,
    Severity,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, ToggleField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
    callout::{Callout, CalloutVariant},
};
use dioxus::prelude::*;
use std::rc::Rc;
use wasm_bindgen_futures::spawn_local;

use crate::runtime::Runtime;

/// The accepted snapshot a request was rendered from; stale requests are refused.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsIdentity {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsRequest {
    pub identity: MechanicalSettingsIdentity,
    pub request_id: u64,
    pub field_id: String,
    pub edit: MechanicalSettingsEdit,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MechanicalSettingsEdit {
    Change(MechanicalSettingsPatch),
    // Link/rejoin actions are admitted by their reviewed inputs across unrelated edits.
    Reviewed {
        change: MechanicalSettingsPatch,
        fingerprint: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechanicalSettingsFeedbackState {
    Landed,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsFeedback {
    pub request_id: u64,
    pub field_id: String,
    pub state: MechanicalSettingsFeedbackState,
    pub message: Option<String>,
}

/// One gasket support the shared viewer selects and the inspector resizes or unlinks.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalGasketSupportRow {
    support: MechanicalGasketSupport,
    pub is_previous: bool,
}

impl std::ops::Deref for MechanicalGasketSupportRow {
    type Target = MechanicalGasketSupport;

    fn deref(&self) -> &Self::Target {
        &self.support
    }
}

impl MechanicalGasketSupportRow {
    pub fn saved_anchor(&self) -> MechanicalGasketAnchor {
        gasket_anchor(self)
    }
}

pub(crate) fn gasket_anchor(support: &MechanicalGasketSupport) -> MechanicalGasketAnchor {
    MechanicalGasketAnchor {
        id: support.id.clone(),
        region_id: support.region_id.clone(),
        outline_key: support.outline_key.clone(),
        anchor: support.anchor,
        length: Some(support.length),
        width: Some(support.width),
        placement: Some(GasketPlacement::User),
        unlinked: support.unlinked,
    }
}

pub(crate) fn gasket_support_id(layer: &str) -> Option<&str> {
    let (id, side) = layer.strip_prefix("gasket:")?.rsplit_once(':')?;
    matches!(side, "lower" | "upper").then_some(id)
}

pub(crate) fn gasket_group<T>(supports: &[T]) -> Option<&'static str> {
    (!supports.is_empty()).then_some("gaskets")
}

pub(crate) fn is_gasket_layer<T>(layer: &str, supports: &[T], id: impl Fn(&T) -> &str) -> bool {
    gasket_group(supports) == Some(layer)
        || gasket_support_id(layer)
            .is_some_and(|key| supports.iter().any(|support| id(support) == key))
}

#[derive(Props, Clone, PartialEq)]
pub struct MechanicalSettingsProps {
    pub identity: MechanicalSettingsIdentity,
    pub request_sequence: Signal<u64>,
    pub configuration: Option<Rc<MechanicalConfiguration>>,
    /// The suggested stack shown before the board is configured.
    pub preview: Option<Rc<MechanicalConfiguration>>,
    pub transport: HardwareTransport,
    pub profiles: Rc<[MechanicalProfileChoice]>,
    #[props(default)]
    pub extract_profile: Option<MechanicalProfileExtractionPort>,
    pub profile_targets: Rc<[MechanicalProfileTargetChoice]>,
    pub process_targets: Rc<[MechanicalProcessTarget]>,
    pub stabilizer_fits: Rc<[MechanicalStabilizerFit]>,
    pub layers: Rc<[MechanicalLayerRow]>,
    pub gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    pub fit_parts: Rc<[MechanicalFitPart]>,
    pub fit_parts_resolved: bool,
    pub hardware_mounts: Rc<[MechanicalHardwareMount]>,
    pub suggested_mounts: Rc<[Mount]>,
    pub findings: Rc<[Finding]>,
    #[props(default)]
    pub pair_features: Option<Rc<[MechanicalPairFeatureRow]>>,
    #[props(default)]
    pub pair_link_options: Option<Rc<[MechanicalPairLinkOption]>>,
    #[props(default)]
    pub pair_counterpart: Option<String>,
    #[props(default)]
    pub pair_projection_port: Option<MechanicalPairProjectionPort>,
    #[props(default)]
    pub construction_is_legacy: bool,
    pub selected_layer: String,
    /// Name of the board the stack is configured for when it is not the active one.
    pub mismatch: Option<String>,
    pub editable: bool,
    pub disabled_reason: Option<String>,
    pub feedback: Rc<[MechanicalSettingsFeedback]>,
    pub on_request: EventHandler<MechanicalSettingsRequest>,
    pub on_select_layer: EventHandler<String>,
    #[props(default)]
    pub on_show_finding: EventHandler<String>,
}

#[derive(Clone, PartialEq)]
pub struct MechanicalSettingsState {
    pub props: Option<MechanicalSettingsProps>,
    /// The Case preview may generate: nothing it depends on is still being prepared or edited.
    pub generation_ready: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct SourceKey {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    hole_loaded: bool,
}

/// Everything projected from the accepted document for one scope, token and revision.
#[derive(Clone, PartialEq)]
struct Source {
    configuration: Option<Rc<MechanicalConfiguration>>,
    preview: Option<Rc<MechanicalConfiguration>>,
    profiles: Rc<[MechanicalProfileChoice]>,
    profile_targets: Rc<[MechanicalProfileTargetChoice]>,
    process_targets: Rc<[MechanicalProcessTarget]>,
    stabilizer_fits: Rc<[MechanicalStabilizerFit]>,
    pair_features: Option<Rc<[MechanicalPairFeatureRow]>>,
    pair_counterpart: Option<String>,
    pair_link_options: Option<Rc<[MechanicalPairLinkOption]>>,
    pair_port: Option<MechanicalPairProjectionPort>,
    construction_is_legacy: bool,
}

fn project_source(
    runtime: &Runtime,
    key: &SourceKey,
    hole: Option<Rc<PartDefinition>>,
) -> Option<Source> {
    let accepted = runtime.model().accepted?;
    if accepted.token != key.token {
        return None;
    }
    let effective = captured_case_document(&accepted, &key.scope).ok()?;
    let resolved = boardstudio_application::resolved_document(&accepted.document);
    let configuration = effective.mechanical.clone().map(Rc::new);
    let instance_id = key.scope.instance_id.as_deref();
    let contours = &accepted.scene.board_contours;
    let projection = instance_id.and_then(|id| pair_projection(&accepted.document, contours, id));
    let preview = configuration.is_none().then(|| {
        boardstudio_core_mechanical::mechanical::effective_inputs(
            &resolved,
            contours,
            &MechanicalTarget::Board {
                board_id: key.scope.board_id.clone(),
            },
        )
        .ok()
        .map(|inputs| Rc::new(inputs.configuration))
    });
    let (document, board_contours, hole_template) =
        (accepted.document.clone(), contours.clone(), hole.clone());
    Some(Source {
        profiles: configuration
            .as_deref()
            .map_or_else(Vec::new, |config| profile_choices(&effective, config))
            .into(),
        profile_targets: profile_targets(&resolved, &key.scope.board_id, configuration.as_deref())
            .into(),
        process_targets: configuration
            .as_deref()
            .map_or_else(Vec::new, |config| process_targets(&resolved, config))
            .into(),
        stabilizer_fits: configuration
            .as_deref()
            .map_or_else(Vec::new, |config| stabilizer_fits(&resolved, config))
            .into(),
        pair_counterpart: projection.as_ref().and_then(|projection| {
            projection
                .members
                .iter()
                .find(|member| Some(member.as_str()) != instance_id)
                .cloned()
        }),
        pair_features: projection.map(|projection| projection.features.into()),
        pair_link_options: instance_id
            .map(|id| pair_link_options(&accepted.document, contours, id, hole.as_deref()).into()),
        pair_port: Some(MechanicalPairProjectionPort(Rc::new(move |change| {
            link_preview(&document, &board_contours, change, hole_template.as_deref())
        }))),
        construction_is_legacy: instance_id
            .and_then(|id| {
                accepted
                    .document
                    .hardware
                    .as_ref()?
                    .instances
                    .iter()
                    .find(|instance| instance.id == id)
            })
            .is_some_and(|instance| {
                instance.construction_ownership == MechanicalConstructionOwnership::Legacy
            }),
        preview: preview.flatten(),
        configuration,
    })
}

/// Keep this alive for the whole Editor so request ids and feedback survive a hidden panel.
pub fn use_mechanical_settings(
    runtime: Rc<Runtime>,
    workspace: Signal<Workspace>,
    instance_selection: InstanceSelection,
    case_selection: CaseSelection,
    on_show_finding: EventHandler<String>,
) -> MechanicalSettingsState {
    let version = use_context::<Signal<u64>>();
    let _ = version();
    let mut request_sequence = use_signal(|| 0_u64);
    let mut feedback = use_signal(|| Rc::<[MechanicalSettingsFeedback]>::from([]));
    let mut hole = use_signal(|| None::<Rc<PartDefinition>>);
    use_hook(|| {
        spawn_local(async move {
            if let Ok(definition) = parts::load_mounting_hole_definition().await {
                hole.set(Some(definition));
            }
        });
    });

    let model = runtime.model();
    let current = model
        .accepted
        .clone()
        .zip(runtime.scope())
        .filter(|(accepted, scope)| {
            scope.session_epoch == accepted.session_epoch
                && scope.document_id == accepted.document.id
                && scope.board_id == model.active_board_id
                && scope.instance_id == model.active_instance_id
                && accepted.scene.revision == accepted.document.revision
        });
    let in_case = workspace() == Workspace::Case;
    let key = current
        .as_ref()
        .filter(|_| in_case)
        .map(|(accepted, scope)| SourceKey {
            scope: scope.clone(),
            token: accepted.token,
            revision: accepted.document.revision,
            hole_loaded: hole().is_some(),
        });
    let source = use_memo(use_reactive((&key,), {
        let runtime = runtime.clone();
        move |(key,)| project_source(&runtime, &key?, hole.peek().clone())
    }))();
    let resolution = use_memo(use_reactive((&key,), {
        let runtime = runtime.clone();
        move |(key,)| {
            let key = key?;
            let accepted = runtime.model().accepted?;
            let proposed = accepted.document.as_ref().clone();
            let (assembly, _) = runtime
                .resolve_mechanical_settings(accepted, key.scope, proposed)
                .ok()?;
            Some(Rc::new(assembly))
        }
    }))();
    let configuration = source
        .as_ref()
        .and_then(|source| source.configuration.clone());

    let saved = model.lifecycle == Lifecycle::Ready
        && matches!(
            model.durability,
            Durability::Saved { .. } | Durability::Saving { .. }
        );
    let admitted = in_case && instance_selection.is_current(&model);
    let editable = admitted && saved && model.display_preview.is_none() && model.gesture.is_none();

    let scene = runtime.cad_scene();
    let exact_scene = current.as_ref().and_then(|(accepted, scope)| {
        scene
            .clone()
            .filter(|scene| scene.exact && scene.scope == *scope && scene.token == accepted.token)
    });
    let display_scene = current.as_ref().and_then(|(_, scope)| {
        scene.clone().filter(|scene| {
            crate::case_generation_lifecycle::same_owner_completed_scene_for_display(
                scene.exact,
                &scene.scope,
                scope,
            )
        })
    });
    let previous_geometry = current
        .as_ref()
        .zip(display_scene.as_ref())
        .is_some_and(|((accepted, _), scene)| scene.token != accepted.token);
    let current_assembly = resolution.clone().or_else(|| {
        exact_scene
            .as_ref()
            .and_then(|scene| scene.mechanical.clone().map(Rc::new))
    });
    let display_assembly = display_scene
        .as_ref()
        .and_then(|scene| scene.mechanical.clone());
    let internal_gasket = configuration
        .as_ref()
        .is_some_and(|config| config.internal_gasket.is_some());
    let rows = project_scene_rows(
        display_assembly.as_ref(),
        current_assembly.as_deref(),
        internal_gasket,
        previous_geometry,
    )
    .unwrap_or_default();
    let suggested: Rc<[Mount]> = current_assembly
        .as_ref()
        .map(|assembly| assembly.suggested_mounts.clone())
        .unwrap_or_default()
        .into();
    let needs_closures = configuration.as_ref().is_some_and(|config| {
        config.closure_mounts.is_none()
            && config.mount != MechanicalMount::Gasket
            && !suggested.is_empty()
    }) && exact_scene.as_ref().is_some_and(|scene| {
        scene
            .mechanical
            .as_ref()
            .is_some_and(|assembly| !assembly.suggested_mounts.is_empty())
    });

    let on_request = EventHandler::new({
        let runtime = runtime.clone();
        move |request: MechanicalSettingsRequest| {
            let result = commit_patch(&runtime, &request, hole.peek().clone());
            let (state, message) = match result {
                Ok(()) => (MechanicalSettingsFeedbackState::Landed, None),
                Err(message) => (MechanicalSettingsFeedbackState::Failed, Some(message)),
            };
            let mut entries = feedback.peek().to_vec();
            entries.retain(|entry| entry.field_id != request.field_id);
            entries.push(MechanicalSettingsFeedback {
                request_id: request.request_id,
                field_id: request.field_id,
                state,
                message,
            });
            feedback.set(entries.into());
        }
    });

    // Closure mounts are suggested once generation finds them; adopt them as soon as the
    // accepted snapshot is saved and editable.
    let init_key = key
        .clone()
        .filter(|key| editable && needs_closures && key.hole_loaded);
    let durability_version = try_use_context::<boardstudio_web_ui_shared::DurabilityVersion>();
    let runtime_for_closures = runtime.clone();
    let mut last_closure_init = use_signal(|| None::<SourceKey>);
    use_effect(use_reactive((&init_key,), move |(init,)| {
        let _ = durability_version.map(|d| d.0());
        let Some(key) = init else { return };
        if !runtime_for_closures.with_model(|m| {
            m.durability
                == (Durability::Saved {
                    revision: key.revision,
                })
        }) {
            return;
        }
        if last_closure_init.peek().as_ref() == Some(&key) {
            return;
        }
        last_closure_init.set(Some(key.clone()));
        let id = request_sequence.peek().saturating_add(1);
        request_sequence.set(id);
        on_request.call(MechanicalSettingsRequest {
            identity: MechanicalSettingsIdentity {
                scope: key.scope,
                snapshot_token: key.token,
                revision: key.revision,
            },
            request_id: id,
            field_id: "InitializeClosures".into(),
            edit: MechanicalSettingsEdit::Change(MechanicalSettingsPatch::InitializeClosures),
        });
    }));

    let generation_ready = current.as_ref().is_some_and(|(accepted, scope)| {
        let resolved = boardstudio_application::resolved_document(&accepted.document);
        admitted
            && crate::case_generation_admission::is_ready(
                &resolved,
                &accepted.scene,
                &scope.board_id,
                configuration
                    .as_deref()
                    .is_some_and(|config| config.board_id == scope.board_id),
                scope.instance_id.as_deref(),
            )
            && model.lifecycle == Lifecycle::Ready
            && matches!(
                model.durability,
                Durability::Saved { revision } | Durability::Saving { revision }
                    if revision == accepted.document.revision
            )
            && model.display_preview.is_none()
            && model.gesture.is_none()
            && !needs_closures
    });

    let props = current.filter(|_| in_case).zip(source).map(|((accepted, scope), source)| {
        let mismatch = configuration
            .as_ref()
            .filter(|config| config.board_id != scope.board_id)
            .map(|config| accepted.document.boards.iter().find(|board| board.id == config.board_id).map_or_else(|| config.board_id.clone(), |board| board.name.clone()));
        let supports = current_assembly.as_deref().or(display_assembly.as_ref()).into_iter().flat_map(|assembly| assembly.gasket_supports.iter());
        let supports_previous = current_assembly.is_none() && previous_geometry;
        let gasket_supports: Vec<_> = supports.cloned().map(|support| MechanicalGasketSupportRow {
            support,
            is_previous: supports_previous,
        }).collect();
        let mut selected_layer = case_selection.layer_id(&scope);
        let selectable = rows.layers.iter().any(|layer| layer.id == selected_layer)
            || is_gasket_layer(&selected_layer, &gasket_supports, |support| &support.id);
        if !selectable {
            selected_layer.clear();
        }
        let identity = MechanicalSettingsIdentity { scope: scope.clone(), snapshot_token: accepted.token, revision: accepted.document.revision };
        let disabled_reason = (!editable).then(|| {
            if !in_case { "Mechanical settings are available in the Case workspace." }
            else if !admitted { "Wait for physical assembly selection before changing mechanical settings." }
            else if !saved { "Wait for the accepted document to finish opening or recover it before editing mechanical settings." }
            else { "Finish or cancel the active edit before changing mechanical settings." }
        }.to_owned()).or_else(|| mismatch.as_ref().map(|_| "Show the configured board before changing its mechanical settings.".to_owned()));
        MechanicalSettingsProps {
            transport: accepted.document.hardware.as_ref().map_or(HardwareTransport::None, |hardware| hardware.transport),
            identity,
            request_sequence,
            configuration,
            preview: source.preview,
            profiles: source.profiles,
            extract_profile: Some(extract_port(&runtime)),
            profile_targets: source.profile_targets,
            process_targets: source.process_targets,
            stabilizer_fits: source.stabilizer_fits,
            layers: rows.layers.into(),
            gasket_supports: gasket_supports.into(),
            fit_parts: rows.fit_parts.into(),
            fit_parts_resolved: current_assembly.is_some(),
            hardware_mounts: rows.hardware_mounts.into(),
            suggested_mounts: suggested,
            findings: current_assembly.as_ref().map(|assembly| assembly.diagnostics.clone()).unwrap_or_default().into(),
            pair_features: source.pair_features,
            pair_link_options: source.pair_link_options,
            pair_counterpart: source.pair_counterpart,
            pair_projection_port: source.pair_port,
            construction_is_legacy: source.construction_is_legacy,
            selected_layer,
            editable: editable && mismatch.is_none(),
            disabled_reason,
            mismatch,
            feedback: feedback(),
            on_request,
            on_select_layer: EventHandler::new(move |id: String| case_selection.select_layer(scope.clone(), id)),
            on_show_finding,
        }
    });
    MechanicalSettingsState {
        props,
        generation_ready,
    }
}

/// Reads one KiCad part's geometry layers for the custom-profile editor.
fn extract_port(runtime: &Rc<Runtime>) -> MechanicalProfileExtractionPort {
    let runtime = runtime.clone();
    MechanicalProfileExtractionPort(Rc::new(move |definition_id, mappings| {
        let runtime = runtime.clone();
        Box::pin(async move {
            let accepted = runtime
                .model()
                .accepted
                .ok_or("The accepted Case profile scope is no longer available.")?;
            let source = accepted
                .document
                .definitions
                .iter()
                .find(|definition| definition.id == definition_id)
                .and_then(|definition| definition.kicad_source.as_ref())
                .map(|source| source.source.clone())
                .ok_or("The selected part has no KiCad source geometry.")?;
            let request_id = format!("case-profile-extract-{}", runtime.operation().0);
            runtime.extract_mechanical_profile(request_id, source, mappings, 0.005)
        })
    }))
}

/// Plan the edit against the accepted snapshot and commit it synchronously.
fn commit_patch(
    runtime: &Rc<Runtime>,
    request: &MechanicalSettingsRequest,
    hole: Option<Rc<PartDefinition>>,
) -> Result<(), String> {
    let identity = &request.identity;
    let accepted = runtime.model().accepted.ok_or("No accepted keyboard.")?;
    let (change, review) = match &request.edit {
        MechanicalSettingsEdit::Change(change) => (change, None),
        MechanicalSettingsEdit::Reviewed {
            change,
            fingerprint,
        } => (change, Some(fingerprint)),
    };
    if matches!(
        change,
        MechanicalSettingsPatch::RejoinFeatureMember { .. }
            | MechanicalSettingsPatch::LinkMirrorPair { .. }
    ) {
        let current =
            reviewed_fingerprint(&accepted.document, &accepted.scene.board_contours, change)?;
        if review != Some(&current) {
            return Err("The reviewed inputs changed since this pair action was submitted; review it again.".into());
        }
    } else if accepted.token != identity.snapshot_token
        || accepted.document.revision != identity.revision
    {
        return Err("The keyboard changed; review the settings and try again.".into());
    }
    let (target, owner) = match &identity.scope.instance_id {
        Some(id) => (
            MechanicalTarget::Instance {
                instance_id: id.clone(),
            },
            id.clone(),
        ),
        None => (
            MechanicalTarget::Board {
                board_id: identity.scope.board_id.clone(),
            },
            identity.scope.board_id.clone(),
        ),
    };
    let edit = MechanicalEdit {
        target,
        change: change.clone(),
        mounting_hole: hole.map(|definition| Box::new((*definition).clone())),
    };
    let resolver = EditResolver::new("mechanical-settings", move |accepted: &AcceptedSnapshot| {
        match evaluate_edit(&accepted.document, &accepted.scene.board_contours, &edit) {
            Ok(MechanicalEditDisposition::Unchanged) => return Resolution::Unchanged,
            Ok(MechanicalEditDisposition::Retire) => {
                return Resolution::Retire("The mechanical target is no longer available.".into());
            }
            // A malformed value stays a Core edit error: submitting lets Core report it.
            Ok(MechanicalEditDisposition::Changed) | Err(_) => {}
        }
        Resolution::submit(
            vec![owner.clone()],
            EditOperation::ApplyMechanicalEdit {
                edit: Box::new(edit.clone()),
            },
        )
    });
    runtime.commit("mechanical-settings", resolver).map(|_| ())
}

/// Sends one patch with a fresh request id; `Copy` so every field closure can own it.
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Sender {
    identity: ReadSignal<MechanicalSettingsIdentity>,
    sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

impl Sender {
    pub(crate) fn send(self, patch: MechanicalSettingsPatch) {
        let field = patch.field_id();
        self.submit(field, MechanicalSettingsEdit::Change(patch));
    }

    pub(crate) fn send_reviewed(self, patch: MechanicalSettingsPatch, fingerprint: Option<String>) {
        if let Some(fingerprint) = fingerprint {
            let field = patch.field_id();
            self.submit(
                field,
                MechanicalSettingsEdit::Reviewed {
                    change: patch,
                    fingerprint,
                },
            );
        }
    }

    fn submit(self, field_id: String, edit: MechanicalSettingsEdit) {
        let identity = self.identity.peek().clone();
        submit_settings_request(&identity, self.sequence, self.on_request, field_id, edit);
    }

    pub(crate) fn instance_id(self) -> String {
        self.identity
            .peek()
            .scope
            .instance_id
            .clone()
            .unwrap_or_default()
    }
}

pub(crate) fn submit_settings_patch(
    settings: &MechanicalSettingsProps,
    patch: MechanicalSettingsPatch,
) -> bool {
    settings.editable
        && submit_settings_request(
            &settings.identity,
            settings.request_sequence,
            settings.on_request,
            patch.field_id(),
            MechanicalSettingsEdit::Change(patch),
        )
}

fn submit_settings_request(
    identity: &MechanicalSettingsIdentity,
    mut sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    field_id: String,
    edit: MechanicalSettingsEdit,
) -> bool {
    let Some(request_id) = sequence.peek().checked_add(1) else {
        return false;
    };
    sequence.set(request_id);
    on_request.call(MechanicalSettingsRequest {
        identity: identity.clone(),
        request_id,
        field_id,
        edit,
    });
    true
}

/// A numeric dimension field using the range Core accepts for `field`.
pub(crate) fn dimension(
    label: &str,
    field: MechanicalDimension,
    value: f64,
    on_commit: impl FnMut(f64) + 'static,
) -> Element {
    let (min, max, step) = dimension_range(field);
    let unit = if field == MechanicalDimension::HardwareQuantity {
        "pcs"
    } else {
        "mm"
    };
    rsx! { NumericField { key: "{label}", label: label.to_owned(), unit, value, min, max, step, on_commit: EventHandler::new(on_commit) } }
}

/// Fields inside are natively disabled while the stack is read-only.
#[component]
pub(crate) fn Fields(editable: bool, children: Element) -> Element {
    rsx! { fieldset { class: "m-0 grid min-w-0 gap-2.5 border-0 p-0", disabled: !editable, {children} } }
}

pub(crate) fn finding_list(
    findings: &[Finding],
    on_show: EventHandler<String>,
    errors_only: bool,
) -> Element {
    rsx! {
        for finding in findings.iter().filter(|finding| !errors_only || finding.severity == Severity::Error).cloned() {
            div { key: "{finding.id}", class: "flex items-center gap-2",
                Badge { variant: if finding.severity == Severity::Error { BadgeVariant::Destructive } else { BadgeVariant::Secondary }, "{finding.severity:?}" }
                span { class: "flex-1", "{finding.message}" }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Show finding {finding.id}",
                    onclick: { let id = finding.id.clone(); move |_| on_show.call(id.clone()) }, "Show" }
            }
        }
    }
}

const METHODS: [(PlateMethod, &str); 4] = [
    (PlateMethod::PcbFr4, "PCB FR-4"),
    (PlateMethod::Printed, "3D printed"),
    (PlateMethod::Cnc, "CNC machined"),
    (PlateMethod::CutSheet, "Cut sheet"),
];

pub(crate) fn method_options() -> Vec<(PlateMethod, String)> {
    METHODS
        .iter()
        .map(|(method, label)| (method.clone(), (*label).to_owned()))
        .collect()
}

/// Compact case choices for the guided setup flow; the full editor stays in Case settings.
#[component]
pub fn MechanicalSetupBasics(settings: MechanicalSettingsProps) -> Element {
    let Some(config) = settings
        .configuration
        .clone()
        .or_else(|| settings.preview.clone())
    else {
        return rsx! { p { role: "status", "Case settings are loading." } };
    };
    let identity = use_memo(use_reactive((&settings.identity,), |(identity,)| identity));
    let send = Sender {
        identity: identity.into(),
        sequence: settings.request_sequence,
        on_request: settings.on_request,
    };
    let reason = (!settings.editable).then(|| "Read-only right now.".to_owned());
    rsx! {
        section { class: "grid gap-4 border-0 bg-transparent p-0", aria_label: "Basic case setup",
            SelectField::<MechanicalMount> { label: "Mounting style", aria_label: "Case mounting style", value: config.mount.clone(), disabled_reason: reason.clone(),
                options: mount_options(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetMount(v)) }
            ToggleField { label: "Include a battery", aria_label: "Include wireless battery", checked: config.battery.is_some(),
                description: if settings.transport == HardwareTransport::Wireless { "For the wireless split." } else { "Choose wireless transport to add a battery." },
                disabled_reason: reason.clone().or_else(|| (settings.transport != HardwareTransport::Wireless).then(|| "Choose wireless transport to add a battery.".to_owned())),
                on_commit: move |v| send.send(MechanicalSettingsPatch::SetBatteryEnabled(v)) }
            SelectField::<PlateMethod> { label: "Case process", aria_label: "Case fabrication process", value: config.method.clone(), disabled_reason: reason,
                options: method_options(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetMethod(v)) }
        }
    }
}

fn mount_label(kind: &MountKind) -> &'static str {
    if *kind == MountKind::Boss {
        "Boss"
    } else {
        "Hole"
    }
}

fn mount_options() -> Vec<(MechanicalMount, String)> {
    vec![
        (MechanicalMount::Tray, "Tray mount".into()),
        (MechanicalMount::Rigid, "Rigid mount".into()),
        (MechanicalMount::Gasket, "Gasket mount".into()),
    ]
}

#[component]
pub fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
    let identity = use_memo(use_reactive((&props.identity,), |(identity,)| identity));
    let send = Sender {
        identity: identity.into(),
        sequence: props.request_sequence,
        on_request: props.on_request,
    };
    let editable = props.editable;
    let disabled = props.disabled_reason.clone().filter(|_| !editable);
    let failure = props
        .feedback
        .iter()
        .rev()
        .find(|entry| entry.state == MechanicalSettingsFeedbackState::Failed)
        .and_then(|entry| entry.message.clone());
    let failed_init = props.feedback.iter().any(|entry| {
        entry.field_id == "InitializeClosures"
            && entry.state == MechanicalSettingsFeedbackState::Failed
    });
    let banner = rsx! {
        if let Some(reason) = disabled { div { role: "status", Callout { "{reason}" } } }
        if let Some(message) = failure { Alert { variant: AlertVariant::Destructive, "{message}" } }
    };

    let Some(config) = props
        .configuration
        .clone()
        .filter(|_| props.mismatch.is_none())
    else {
        if let Some(board) = props.mismatch.as_ref() {
            return rsx! { section { aria_label: "Mechanical stack settings", div { role: "status", Callout { variant: CalloutVariant::Warning, "Mechanical settings are unavailable while {board} is configured. Switch to that board to edit its stack." } } } };
        }
        return rsx! {
            section { class: "grid gap-3", aria_label: "Mechanical stack settings",
                h2 { "Case construction" }
                {banner}
                if let Some(preview) = props.preview.clone() {
                    StackupPreview { configuration: preview.clone() }
                    p { class: "text-muted-foreground text-sm", "Suggested stack from this board and its switch profiles. Configure to save these settings." }
                }
                p { class: "text-muted-foreground text-sm", "Configure a mechanical stack from the current board and represented switch fit profiles." }
                Button { variant: ButtonVariant::Secondary, disabled: !editable, onclick: move |_| send.send(MechanicalSettingsPatch::Enable), "Configure mechanical stack" }
            }
        };
    };

    // A selected layer or gasket support narrows the inspector to its own measurements.
    if let Some(layer) = props
        .layers
        .iter()
        .find(|layer| layer.id == props.selected_layer)
        .filter(|layer| contextual_layer_fields(&layer.id, &config).is_some())
    {
        return rsx! { ContextualLayerInspector { configuration: config.clone(), layer: layer.clone(), editable, banner, findings: props.findings.clone(), send, on_select_layer: props.on_select_layer, on_show_finding: props.on_show_finding } };
    }
    if let Some(id) = gasket_support_id(&props.selected_layer)
        && let Some(support) = props
            .gasket_supports
            .iter()
            .find(|support| support.id == id)
    {
        return rsx! { GasketSupportInspector { support: support.clone(), supports: props.gasket_supports.clone(), editable, banner, findings: props.findings.clone(), send, on_select_layer: props.on_select_layer, on_show_finding: props.on_show_finding } };
    }

    let counterpart = props.pair_counterpart.clone();
    rsx! {
        section { class: "grid gap-3 [&_h3]:m-0 [&_p]:m-0", aria_label: "Mechanical stack settings",
            h2 { "Case construction" }
            StackupPreview { configuration: config.clone() }
            {banner}
            if failed_init {
                Button { variant: ButtonVariant::Secondary, disabled: !editable, onclick: move |_| send.send(MechanicalSettingsPatch::InitializeClosures), "Retry mounting locations" }
            }
            InspectorSection { title: "Linked halves", default_open: false,
                if let Some(features) = props.pair_features.clone().filter(|features| !features.is_empty()) {
                    PairRowControls { features, editable, send }
                } else if let Some(options) = props.pair_link_options.clone().filter(|options| !options.is_empty()) {
                    PairLinkControls { options, editable, port: props.pair_projection_port.clone(), snapshot: props.identity.snapshot_token.0, send }
                } else {
                    p { class: "text-muted-foreground text-sm", "This half has no mirror pair or link candidates." }
                }
            }
            InspectorSection { title: "Construction",
                Fields { editable,
                    SelectField::<PlateMethod> { label: "Plate method", value: config.method.clone(), options: method_options(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetMethod(v)) }
                    SelectField::<MechanicalMount> { label: "Mount style", value: config.mount.clone(), options: mount_options(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetMount(v)) }
                    SelectField::<MechanicalBottomStyle> { label: "Bottom construction", value: config.bottom_style.clone().unwrap_or(MechanicalBottomStyle::Shell), disabled: config.mount == MechanicalMount::Gasket,
                        options: vec![(MechanicalBottomStyle::Shell, "Tray shell".into()), (MechanicalBottomStyle::Sheet, "Flat sheet".into())],
                        on_commit: move |v| send.send(MechanicalSettingsPatch::SetBottomStyle(v)) }
                    if config.bottom_style == Some(MechanicalBottomStyle::Sheet) {
                        ToggleField { label: "Add middle frame", checked: config.middle_frame.unwrap_or(false), on_commit: move |v| send.send(MechanicalSettingsPatch::SetMiddleFrame(v)) }
                    }
                    ToggleField { label: "Integrate plate frame into case", checked: config.integrated_plate_frame,
                        disabled_reason: (config.mount == MechanicalMount::Gasket).then(|| "Not available for a gasket mount.".to_owned()),
                        on_commit: move |v| send.send(MechanicalSettingsPatch::SetIntegratedPlateFrame(v)) }
                }
            }
            InspectorSection { title: "Switch fit profiles", default_open: false,
                ProfileSection { profiles: props.profiles.clone(), targets: props.profile_targets.clone(), extract: props.extract_profile.clone(), plate_thickness: config.plate_thickness, plate_to_pcb: config.plate_to_pcb, editable, send }
            }
            InspectorSection { title: "Manufacturing overrides", default_open: false,
                ManufacturingSection { targets: props.process_targets.clone(), stabilizers: props.stabilizer_fits.clone(), editable, send }
            }
            InspectorSection { title: "Dimensions",
                Fields { editable,
                    for (label, field, value) in [
                        ("Plate thickness", MechanicalDimension::PlateThickness, config.plate_thickness),
                        ("Plate foam", MechanicalDimension::PlateFoamThickness, config.plate_foam_thickness),
                        ("PCB thickness", MechanicalDimension::PcbThickness, config.pcb_thickness),
                        ("Bottom foam", MechanicalDimension::BottomFoamThickness, config.bottom_foam_thickness),
                        ("Bottom thickness", MechanicalDimension::BottomThickness, config.bottom_thickness),
                        ("Wall thickness", MechanicalDimension::WallThickness, config.wall_thickness),
                        ("Clearance", MechanicalDimension::Clearance, config.clearance),
                        ("Opening allowance", MechanicalDimension::OpeningAllowance, config.opening_allowance.unwrap_or(0.0)),
                    ] {
                        {dimension(label, field, value, move |value| send.send(MechanicalSettingsPatch::SetDimension { field, value }))}
                    }
                }
            }
            InspectorSection { title: "Resolved stack", badge: props.layers.len().to_string(),
                ResolvedStack { layers: props.layers.clone(), selected: props.selected_layer.clone(), on_select_layer: props.on_select_layer }
            }
            InspectorSection { title: "Mechanical diagnostics", default_open: false, badge: props.findings.len().to_string(),
                if props.findings.is_empty() { Callout { "No current mechanical findings." } }
                {finding_list(&props.findings, props.on_show_finding, false)}
            }
            InspectorSection { title: "Battery", default_open: false,
                Fields { editable,
                    ToggleField { label: "Include a battery envelope", checked: config.battery.is_some(), on_commit: move |v| send.send(MechanicalSettingsPatch::SetBatteryEnabled(v)) }
                    if let Some(battery) = config.battery.clone() {
                        for (label, field, value) in [
                            ("Width", MechanicalDimension::BatteryWidth, battery.size.x),
                            ("Depth", MechanicalDimension::BatteryDepth, battery.size.y),
                            ("Height", MechanicalDimension::BatteryHeight, battery.size.z),
                            ("Cable width", MechanicalDimension::BatteryCableWidth, battery.cable_width.unwrap_or(2.0)),
                            ("Position X", MechanicalDimension::BatteryPositionX, battery.at.x),
                            ("Position Y", MechanicalDimension::BatteryPositionY, battery.at.y),
                            ("Cable exit X", MechanicalDimension::BatteryCableExitX, battery.cable_exit.x),
                            ("Cable exit Y", MechanicalDimension::BatteryCableExitY, battery.cable_exit.y),
                        ] {
                            {dimension(label, field, value, move |value| send.send(MechanicalSettingsPatch::SetDimension { field, value }))}
                        }
                    }
                }
            }
            InspectorSection { title: "Case openings", default_open: false, badge: config.openings.as_ref().map_or(0, Vec::len).to_string(),
                Fields { editable,
                    Button { variant: ButtonVariant::Secondary, onclick: move |_| send.send(MechanicalSettingsPatch::AddOpening), "Add volume" }
                    for (index, opening) in config.openings.clone().unwrap_or_default().into_iter().enumerate() {
                        fieldset { key: "{opening.id}", class: "grid gap-2 rounded-md border border-border p-2.5",
                            legend { "Volume {index + 1}" }
                            for (label, field, value) in [("Bottom Z", MechanicalDimension::OpeningBottomZ, opening.z), ("Height", MechanicalDimension::OpeningHeight, opening.height)] {
                                {dimension(label, field, value, { let id = opening.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetOpeningDimension { opening_id: id.clone(), vertex_id: None, field, value }) })}
                            }
                            for (point, vertex) in opening.vertices.iter().cloned().enumerate() {
                                div { key: "{vertex.id}", class: "grid gap-1",
                                    span { "Vertex {point + 1}" }
                                    for (label, field, value) in [("X", MechanicalDimension::OpeningPointX, vertex.point.x), ("Y", MechanicalDimension::OpeningPointY, vertex.point.y)] {
                                        {dimension(label, field, value, { let (id, vertex_id) = (opening.id.clone(), vertex.id.clone()); move |value| send.send(MechanicalSettingsPatch::SetOpeningDimension { opening_id: id.clone(), vertex_id: Some(vertex_id.clone()), field, value }) })}
                                    }
                                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: opening.vertices.len() <= 3, aria_label: "Remove access opening {index + 1} vertex {point + 1}",
                                        onclick: { let (id, vertex_id) = (opening.id.clone(), vertex.id.clone()); move |_| send.send(MechanicalSettingsPatch::RemoveOpeningVertex { opening_id: id.clone(), vertex_id: vertex_id.clone() }) }, "Remove vertex" }
                                }
                            }
                            if opening.vertices.len() < 3 { Alert { variant: AlertVariant::Destructive, "An opening footprint needs at least three vertices." } }
                            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Add vertex to volume {index + 1}",
                                onclick: { let id = opening.id.clone(); move |_| send.send(MechanicalSettingsPatch::AddOpeningVertex { opening_id: id.clone() }) }, "Add vertex" }
                            Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove volume {index + 1}",
                                onclick: { let id = opening.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveOpening { opening_id: id.clone() }) }, "Remove" }
                        }
                    }
                }
            }
            if config.mount != MechanicalMount::Gasket {
                MountSection { collection: MechanicalMountCollection::Suspension, label: "Suspension mounts", mounts: config.mounts.clone(), allow_add: true, editable, send }
            }
            MountSection { collection: MechanicalMountCollection::Closure, label: "Closure screws", mounts: config.closure_mounts.clone().unwrap_or_default(), allow_add: config.internal_gasket.is_none(), editable, send }
            InspectorSection { title: "Suggested mount locations", default_open: false, badge: props.suggested_mounts.len().to_string(),
                Fields { editable,
                    if props.suggested_mounts.is_empty() {
                        p { class: "text-muted-foreground text-sm", "Resolve the current geometry to see clearance-tested mounting locations." }
                    } else {
                        for (index, mount) in props.suggested_mounts.iter().enumerate() { span { key: "{mount.id}", "{index + 1}: X {mount.at.x:.1}, Y {mount.at.y:.1} mm" } }
                        if config.internal_gasket.is_some() {
                            Button { variant: ButtonVariant::Secondary, onclick: { let mounts = props.suggested_mounts.to_vec(); move |_| send.send(MechanicalSettingsPatch::AdoptClosurePositions(mounts.clone())) }, "Adopt closure positions" }
                        } else {
                            Button { variant: ButtonVariant::Secondary, onclick: { let mounts = props.suggested_mounts.to_vec(); move |_| send.send(MechanicalSettingsPatch::SetMountCollection { collection: MechanicalMountCollection::Suspension, mounts: mounts.clone() }) }, "Adopt suggested mounts" }
                            Button { variant: ButtonVariant::Secondary, onclick: { let mounts = props.suggested_mounts.to_vec(); move |_| send.send(MechanicalSettingsPatch::AddSuggestedClosureMounts(mounts.clone())) }, "Add suggested closure screws" }
                        }
                    }
                }
            }
            if config.mount == MechanicalMount::Gasket && config.internal_gasket.is_some() {
                InspectorSection { title: "Gasket supports", default_open: false, badge: props.gasket_supports.len().to_string(),
                    Fields { editable,
                        Button { variant: ButtonVariant::Secondary, onclick: move |_| send.send(MechanicalSettingsPatch::ResetGasketPlacement), "Reset gasket placement" }
                        p { class: "text-muted-foreground text-sm", "Reset releases manually positioned supports. Closure positions stay fixed." }
                        for (index, support) in props.gasket_supports.iter().enumerate() {
                            Button { key: "{support.id}", variant: ButtonVariant::Ghost, class: "justify-start",
                                onclick: { let layer = format!("gasket:{}:lower", support.id); move |_| props.on_select_layer.call(layer.clone()) }, "Gasket {index + 1}" }
                        }
                    }
                }
            }
            if let Some(internal) = config.internal_gasket.as_ref() {
                ClosureHardwareSection { hardware: internal.hardware.clone(), editable, send }
            }
            HardwareAndFitsSection {
                configuration: config.clone(), fit_parts: props.fit_parts.clone(), fit_parts_resolved: props.fit_parts_resolved,
                hardware_mounts: props.hardware_mounts.clone(), editable, send,
            }
            Button { variant: ButtonVariant::Secondary, disabled: !editable, onclick: move |_| send.send(MechanicalSettingsPatch::Disable),
                if counterpart.is_some() { "Unlink and disable this half" } else { "Disable mechanical stack" } }
            if let Some(counterpart) = counterpart {
                p { class: "text-muted-foreground text-sm", "This half is linked with {counterpart}. Disabling removes that link and disables only this half's mechanical stack; the other half keeps its geometry and settings." }
                PairMemberRemoval { counterpart, editable, send }
            }
        }
    }
}

#[component]
fn MountSection(
    collection: MechanicalMountCollection,
    label: &'static str,
    mounts: Vec<Mount>,
    allow_add: bool,
    editable: bool,
    send: Sender,
) -> Element {
    rsx! {
        InspectorSection { title: label, default_open: false, badge: mounts.len().to_string(),
            Fields { editable,
                if mounts.is_empty() { Callout { "No {label.to_lowercase()} configured." } }
                for (index, mount) in mounts.into_iter().enumerate() {
                    fieldset { key: "{mount.id}", class: "grid gap-2 rounded-md border border-border p-2.5",
                        legend { "{mount_label(&mount.kind)} {index + 1}" }
                        SelectField::<MountKind> { label: "Mount type", value: mount.kind.clone(),
                            options: vec![(MountKind::Hole, "Hole".into()), (MountKind::Boss, "Boss".into())],
                            on_commit: { let id = mount.id.clone(); move |kind| send.send(MechanicalSettingsPatch::SetMountKind { collection, mount_id: id.clone(), kind }) } }
                        for (label, field, value, show) in [
                            ("Position X", MechanicalDimension::MountPositionX, mount.at.x, true),
                            ("Position Y", MechanicalDimension::MountPositionY, mount.at.y, true),
                            ("Hole diameter", MechanicalDimension::MountHoleDiameter, mount.hole_diameter, true),
                            ("Boss diameter", MechanicalDimension::MountBossDiameter, mount.boss_diameter.unwrap_or(5.0), true),
                            ("Boss height", MechanicalDimension::MountBossHeight, mount.height.unwrap_or(5.0), mount.kind == MountKind::Boss),
                        ] {
                            if show { {dimension(label, field, value, { let id = mount.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetMountDimension { collection, mount_id: id.clone(), field, value }) })} }
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, aria_label: "Remove mount {index + 1}",
                            onclick: { let id = mount.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveMount { collection, mount_id: id.clone() }) }, "Remove" }
                    }
                }
                if allow_add { Button { variant: ButtonVariant::Secondary, onclick: move |_| send.send(MechanicalSettingsPatch::AddMount { collection }), "Add {label.trim_end_matches('s').to_lowercase()}" } }
            }
        }
    }
}
