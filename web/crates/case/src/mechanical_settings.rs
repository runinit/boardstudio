//! Case mechanical inspector: declarative field sections over the accepted configuration.
//!
//! `use_mechanical_settings` projects the accepted snapshot into `MechanicalSettingsProps` and
//! commits each request synchronously through `Runtime::commit`; `MechanicalSettings` renders
//! the sections directly from those props. The 3D viewer reads the same props for its handles.
pub use crate::mechanical_rules::MechanicalSettingsPatchExt;
use crate::mechanical_rules::dimension_range;
use crate::pair_controls::MechanicalPairControls;
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
    EditOperation, Finding, GasketPlacement, MechanicalBottomStyle, MechanicalConfiguration,
    MechanicalGasketAnchor, MechanicalMount, Mount, MountKind, PartDefinition, PlateMethod,
    Severity,
};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TextCommitField, ToggleField,
};
use boardstudio_web_ui_shared::ui::{
    badge::{Badge, BadgeVariant},
    button::{Button, ButtonSize, ButtonVariant},
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
    pub patch: MechanicalSettingsPatch,
    /// Unused since reviewed pair links were removed; the shared viewer (Track 6A) still sets it.
    pub reviewed_fingerprint: Option<String>,
}

/// One gasket support the shared viewer selects and unlinks.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalGasketSupportRow {
    pub id: String,
    pub region_id: String,
    pub outline_key: String,
    pub anchor: f64,
    pub pair_id: Option<String>,
    pub length: f64,
    pub width: f64,
    pub unlinked: bool,
    pub is_previous: bool,
}

impl MechanicalGasketSupportRow {
    pub fn saved_anchor(&self) -> MechanicalGasketAnchor {
        MechanicalGasketAnchor {
            id: self.id.clone(),
            region_id: self.region_id.clone(),
            outline_key: self.outline_key.clone(),
            anchor: self.anchor,
            length: Some(self.length),
            width: Some(self.width),
            placement: Some(GasketPlacement::User),
            unlinked: self.unlinked,
        }
    }
}

// `Pending` stays because the (Track 6A) viewer matches on it; commits are synchronous.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MechanicalSettingsFeedbackState {
    Pending,
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

/// The mount lists the viewer's handles drag.
#[derive(Clone, Debug, PartialEq)]
pub struct MechanicalSettingsValues {
    pub mount: MechanicalMount,
    pub suspension_mounts: Vec<Mount>,
    pub closure_mounts: Option<Vec<Mount>>,
}

#[derive(Props, Clone, PartialEq)]
pub struct MechanicalSettingsProps {
    pub identity: MechanicalSettingsIdentity,
    pub request_sequence: Signal<u64>,
    pub configuration: Option<Rc<MechanicalConfiguration>>,
    pub values: Option<MechanicalSettingsValues>,
    pub suggested_mounts: Rc<[Mount]>,
    pub gasket_supports: Rc<[MechanicalGasketSupportRow]>,
    pub findings: Rc<[Finding]>,
    pub pair_features: Option<Rc<[crate::mechanical_pair::MechanicalPairFeatureRow]>>,
    pub editable: bool,
    pub disabled_reason: Option<String>,
    pub feedback: Rc<[MechanicalSettingsFeedback]>,
    pub on_request: EventHandler<MechanicalSettingsRequest>,
    #[props(default)]
    pub on_show_finding: EventHandler<String>,
}

#[derive(Clone, PartialEq)]
pub struct MechanicalSettingsState {
    pub props: Option<MechanicalSettingsProps>,
    /// The Case preview may generate: nothing it depends on is still being prepared or edited.
    pub generation_ready: bool,
}

/// Keep this alive for the whole Editor so request ids and feedback survive a hidden panel.
pub fn use_mechanical_settings(
    runtime: Rc<Runtime>,
    workspace: Signal<&'static str>,
    instance_selection: InstanceSelection,
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
        });
    let key = current
        .as_ref()
        .map(|(accepted, scope)| (scope.clone(), accepted.token, accepted.document.revision));
    let configuration = use_memo(use_reactive((&key,), {
        let runtime = runtime.clone();
        move |(key,)| {
            let (scope, ..) = key?;
            let accepted = runtime.model().accepted?;
            let document = captured_case_document(&accepted, &scope).ok()?;
            document.mechanical.map(Rc::new)
        }
    }))();

    let saved = matches!(
        model.lifecycle,
        Lifecycle::Ready | Lifecycle::Applying | Lifecycle::Saving
    ) && matches!(
        model.durability,
        Durability::Saved { .. } | Durability::Saving { .. }
    );
    let in_case = workspace() == "Case" && instance_selection.is_current(&model);
    let editable = in_case && saved && model.display_preview.is_none() && model.gesture.is_none();

    let scene = runtime.cad_scene();
    let assembly = current.as_ref().and_then(|(_, scope)| {
        scene
            .as_ref()
            .filter(|scene| scene.exact && scene.scope == *scope)
            .and_then(|scene| scene.mechanical.clone())
    });
    let gasket_supports: Vec<_> = assembly
        .iter()
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
            is_previous: false,
        })
        .collect();
    let suggested: Rc<[Mount]> = assembly
        .as_ref()
        .map(|assembly| assembly.suggested_mounts.clone())
        .unwrap_or_default()
        .into();
    let needs_closures = configuration.as_ref().is_some_and(|config| {
        config.closure_mounts.is_none()
            && config.mount != MechanicalMount::Gasket
            && !suggested.is_empty()
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
    // accepted snapshot is editable.
    let init_key = key
        .clone()
        .filter(|_| editable && needs_closures && hole().is_some());
    use_effect(use_reactive((&init_key,), move |(init,)| {
        let Some((scope, token, revision)) = init else {
            return;
        };
        let id = request_sequence.peek().saturating_add(1);
        request_sequence.set(id);
        on_request.call(MechanicalSettingsRequest {
            identity: MechanicalSettingsIdentity {
                scope,
                snapshot_token: token,
                revision,
            },
            request_id: id,
            field_id: "InitializeClosures".into(),
            patch: MechanicalSettingsPatch::InitializeClosures,
            reviewed_fingerprint: None,
        });
    }));

    let generation_ready = current.as_ref().is_some_and(|(accepted, scope)| {
        let resolved = boardstudio_application::resolved_document(&accepted.document);
        in_case
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
            && model.durability
                == (Durability::Saved {
                    revision: accepted.document.revision,
                })
            && model.display_preview.is_none()
            && model.gesture.is_none()
            && !needs_closures
    });

    let props = current
        .filter(|_| workspace() == "Case")
        .map(|(accepted, scope)| {
            let values = configuration
                .as_ref()
                .map(|config| MechanicalSettingsValues {
                    mount: config.mount.clone(),
                    suspension_mounts: config.mounts.clone(),
                    closure_mounts: config.closure_mounts.clone(),
                });
            let pair_features = scope.instance_id.as_deref().and_then(|instance_id| {
                crate::mechanical_pair::pair_projection(
                    &accepted.document,
                    &accepted.scene.board_contours,
                    instance_id,
                )
            });
            let mismatch = configuration
                .as_ref()
                .filter(|config| config.board_id != scope.board_id)
                .map(|config| format!("These settings belong to board {}.", config.board_id));
            MechanicalSettingsProps {
                identity: MechanicalSettingsIdentity {
                    scope,
                    snapshot_token: accepted.token,
                    revision: accepted.document.revision,
                },
                request_sequence,
                configuration,
                values,
                suggested_mounts: suggested,
                gasket_supports: gasket_supports.into(),
                findings: assembly
                    .map(|assembly| assembly.diagnostics.clone())
                    .unwrap_or_default()
                    .into(),
                pair_features: pair_features.map(|projection| projection.features.into()),
                editable: editable && mismatch.is_none(),
                disabled_reason: (!editable)
                    .then(|| {
                        "Mechanical settings are read-only while an edit or preview is in progress."
                            .to_owned()
                    })
                    .or(mismatch),
                feedback: feedback(),
                on_request,
                on_show_finding: EventHandler::default(),
            }
        });
    MechanicalSettingsState {
        props,
        generation_ready,
    }
}

/// Plan the edit against the accepted snapshot and commit it synchronously.
fn commit_patch(
    runtime: &Rc<Runtime>,
    request: &MechanicalSettingsRequest,
    hole: Option<Rc<PartDefinition>>,
) -> Result<(), String> {
    let identity = &request.identity;
    let accepted = runtime.model().accepted;
    if !accepted.as_ref().is_some_and(|a| {
        a.token == identity.snapshot_token && a.document.revision == identity.revision
    }) {
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
        change: request.patch.clone(),
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

/// Sends one patch with a fresh request id.
#[derive(Clone, Copy, PartialEq)]
struct Sender {
    identity: ReadSignal<MechanicalSettingsIdentity>,
    sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

impl Sender {
    fn send(mut self, patch: MechanicalSettingsPatch) {
        let id = self.sequence.peek().saturating_add(1);
        self.sequence.set(id);
        self.on_request.call(MechanicalSettingsRequest {
            identity: self.identity.peek().clone(),
            request_id: id,
            field_id: patch.field_id(),
            patch,
            reviewed_fingerprint: None,
        });
    }
}

#[component]
pub fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
    let identity = use_memo(use_reactive((&props.identity,), |(identity,)| identity));
    let send = Sender {
        identity: identity.into(),
        sequence: props.request_sequence,
        on_request: props.on_request,
    };
    let off = props.disabled_reason.clone().filter(|_| !props.editable);
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|entry| entry.state == MechanicalSettingsFeedbackState::Failed)
        .and_then(|entry| entry.message.clone());
    let Some(config) = props.configuration.clone() else {
        return rsx! {
            section { aria_label: "Mechanical settings",
                p { "The case has no mechanical configuration yet." }
                Button { disabled: !props.editable, onclick: move |_| send.send(MechanicalSettingsPatch::Enable), "Configure mechanical case" }
                if let Some(reason) = off { p { "{reason}" } }
            }
        };
    };
    let dim = move |label: &str, field: MechanicalDimension, value: f64| {
        dimension(
            label,
            field,
            value,
            off_reason(&props.editable),
            move |value| {
                send.send(MechanicalSettingsPatch::SetDimension { field, value });
            },
        )
    };
    let reason = off_reason(&props.editable);
    rsx! {
        section { aria_label: "Mechanical settings",
            if let Some(reason) = off { p { "{reason}" } }
            if let Some(error) = error { p { role: "alert", "{error}" } }
            InspectorSection { title: "Construction",
                SelectField::<PlateMethod> { label: "Plate method", value: config.method.clone(), disabled_reason: reason.clone(),
                    options: vec![(PlateMethod::PcbFr4, "PCB (FR4)".into()), (PlateMethod::Printed, "Printed".into()), (PlateMethod::Cnc, "CNC".into()), (PlateMethod::CutSheet, "Cut sheet".into())],
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetMethod(v)) }
                SelectField::<MechanicalMount> { label: "Mounting style", value: config.mount.clone(), disabled_reason: reason.clone(),
                    options: vec![(MechanicalMount::Tray, "Tray".into()), (MechanicalMount::Rigid, "Rigid".into()), (MechanicalMount::Gasket, "Gasket".into())],
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetMount(v)) }
                SelectField::<MechanicalBottomStyle> { label: "Bottom style", value: config.bottom_style.clone().unwrap_or(MechanicalBottomStyle::Shell), disabled_reason: reason.clone(),
                    options: vec![(MechanicalBottomStyle::Shell, "Shell".into()), (MechanicalBottomStyle::Sheet, "Sheet".into())],
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetBottomStyle(v)) }
                ToggleField { label: "Middle frame", checked: config.middle_frame.unwrap_or(false), disabled_reason: reason.clone(),
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetMiddleFrame(v)) }
                ToggleField { label: "Integrated plate frame", checked: config.integrated_plate_frame, disabled_reason: reason.clone(),
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetIntegratedPlateFrame(v)) }
                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !props.editable, onclick: move |_| send.send(MechanicalSettingsPatch::Disable), "Remove mechanical configuration" }
            }
            InspectorSection { title: "Dimensions",
                {dim("Plate thickness", MechanicalDimension::PlateThickness, config.plate_thickness)}
                {dim("Plate foam", MechanicalDimension::PlateFoamThickness, config.plate_foam_thickness)}
                {dim("PCB thickness", MechanicalDimension::PcbThickness, config.pcb_thickness)}
                {dim("Bottom foam", MechanicalDimension::BottomFoamThickness, config.bottom_foam_thickness)}
                {dim("Bottom thickness", MechanicalDimension::BottomThickness, config.bottom_thickness)}
                {dim("Wall thickness", MechanicalDimension::WallThickness, config.wall_thickness)}
            }
            InspectorSection { title: "Clearances",
                {dim("Clearance", MechanicalDimension::Clearance, config.clearance)}
                {dim("Opening allowance", MechanicalDimension::OpeningAllowance, config.opening_allowance.unwrap_or(0.0))}
            }
            MountSection { collection: MechanicalMountCollection::Suspension, mounts: config.mounts.clone(), reason: reason.clone(), send }
            if let Some(closures) = config.closure_mounts.clone() {
                MountSection { collection: MechanicalMountCollection::Closure, mounts: closures, reason: reason.clone(), send }
            } else if !props.suggested_mounts.is_empty() {
                Button { disabled: !props.editable, onclick: {
                    let suggested = props.suggested_mounts.to_vec();
                    move |_| send.send(MechanicalSettingsPatch::AddSuggestedClosureMounts(suggested.clone()))
                }, "Add suggested closure mounts" }
            }
            InspectorSection { title: "Hardware", badge: config.hardware.as_ref().map(|h| h.len().to_string()),
                for item in config.hardware.clone().unwrap_or_default() {
                    div { key: "{item.id}",
                        TextCommitField { label: "Designation", value: item.designation.clone(), disabled_reason: reason.clone(),
                            on_commit: { let id = item.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetHardwareText { hardware_id: id.clone(), field: boardstudio_core_mechanical::mechanical::change::MechanicalHardwareTextField::Designation, value }) } }
                        {dimension("Length", MechanicalDimension::HardwareLength, item.length, reason.clone(), { let id = item.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetHardwareDimension { hardware_id: id.clone(), field: MechanicalDimension::HardwareLength, value }) })}
                        {dimension("Quantity", MechanicalDimension::HardwareQuantity, f64::from(item.quantity), reason.clone(), { let id = item.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetHardwareDimension { hardware_id: id.clone(), field: MechanicalDimension::HardwareQuantity, value }) })}
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !props.editable,
                            onclick: { let id = item.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveHardware { hardware_id: id.clone() }) }, "Remove {item.designation}" }
                    }
                }
                if let Some(first) = config.mounts.first().or_else(|| config.closure_mounts.as_ref().and_then(|m| m.first())) {
                    Button { disabled: !props.editable, onclick: {
                        let (part_id, feature_id) = (if config.mount == MechanicalMount::Rigid { "plate" } else { "bottom" }.to_owned(), first.id.clone());
                        move |_| send.send(MechanicalSettingsPatch::AddHardware { part_id: part_id.clone(), feature_id: feature_id.clone() })
                    }, "Add hardware" }
                }
            }
            InspectorSection { title: "Openings", badge: config.openings.as_ref().map(|o| o.len().to_string()),
                for opening in config.openings.clone().unwrap_or_default() {
                    div { key: "{opening.id}",
                        {dimension("Bottom Z", MechanicalDimension::OpeningBottomZ, opening.z, reason.clone(), { let id = opening.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetOpeningDimension { opening_id: id.clone(), vertex_id: None, field: MechanicalDimension::OpeningBottomZ, value }) })}
                        {dimension("Height", MechanicalDimension::OpeningHeight, opening.height, reason.clone(), { let id = opening.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetOpeningDimension { opening_id: id.clone(), vertex_id: None, field: MechanicalDimension::OpeningHeight, value }) })}
                        for vertex in opening.vertices.clone() {
                            for (axis, field, value) in [("X", MechanicalDimension::OpeningPointX, vertex.point.x), ("Y", MechanicalDimension::OpeningPointY, vertex.point.y)] {
                                {dimension(&format!("Point {axis}"), field, value, reason.clone(), { let (id, vertex_id) = (opening.id.clone(), vertex.id.clone()); move |value| send.send(MechanicalSettingsPatch::SetOpeningDimension { opening_id: id.clone(), vertex_id: Some(vertex_id.clone()), field, value }) })}
                            }
                        }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !props.editable,
                            onclick: { let id = opening.id.clone(); move |_| send.send(MechanicalSettingsPatch::AddOpeningVertex { opening_id: id.clone() }) }, "Add vertex" }
                        Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !props.editable,
                            onclick: { let id = opening.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveOpening { opening_id: id.clone() }) }, "Remove opening" }
                    }
                }
                Button { disabled: !props.editable, onclick: move |_| send.send(MechanicalSettingsPatch::AddOpening), "Add opening" }
            }
            InspectorSection { title: "Battery",
                ToggleField { label: "Battery compartment", checked: config.battery.is_some(), disabled_reason: reason.clone(),
                    on_commit: move |v| send.send(MechanicalSettingsPatch::SetBatteryEnabled(v)) }
                if let Some(battery) = config.battery.clone() {
                    {dim("Battery width", MechanicalDimension::BatteryWidth, battery.size.x)}
                    {dim("Battery depth", MechanicalDimension::BatteryDepth, battery.size.y)}
                    {dim("Battery height", MechanicalDimension::BatteryHeight, battery.size.z)}
                    {dim("Position X", MechanicalDimension::BatteryPositionX, battery.at.x)}
                    {dim("Position Y", MechanicalDimension::BatteryPositionY, battery.at.y)}
                    {dim("Cable exit X", MechanicalDimension::BatteryCableExitX, battery.cable_exit.x)}
                    {dim("Cable exit Y", MechanicalDimension::BatteryCableExitY, battery.cable_exit.y)}
                }
            }
            if !props.findings.is_empty() {
                InspectorSection { title: "Findings", badge: props.findings.len().to_string(),
                    for finding in props.findings.iter().cloned() {
                        Button { key: "{finding.id}", variant: ButtonVariant::Ghost, class: "justify-start text-left",
                            onclick: { let id = finding.id.clone(); move |_| props.on_show_finding.call(id.clone()) },
                            Badge { variant: if finding.severity == Severity::Error { BadgeVariant::Destructive } else { BadgeVariant::Secondary }, "{finding.severity:?}" }
                            " {finding.message}"
                        }
                    }
                }
            }
            if let Some(features) = props.pair_features.clone() {
                MechanicalPairControls { features, instance_id: props.identity.scope.instance_id.clone().unwrap_or_default(), editable: props.editable, send_patch: EventHandler::new(move |patch| send.send(patch)) }
            }
        }
    }
}

fn off_reason(editable: &bool) -> Option<String> {
    (!editable).then(|| "Read-only right now.".to_owned())
}

fn dimension(
    label: &str,
    field: MechanicalDimension,
    value: f64,
    disabled_reason: Option<String>,
    on_commit: impl FnMut(f64) + 'static,
) -> Element {
    let (min, max, step) = dimension_range(field);
    rsx! {
        NumericField {
            label: label.to_owned(), unit: "mm", value, min, max, step, disabled_reason,
            on_commit: EventHandler::new(on_commit),
        }
    }
}

#[component]
fn MountSection(
    collection: MechanicalMountCollection,
    mounts: Vec<Mount>,
    reason: Option<String>,
    send: Sender,
) -> Element {
    let title = match collection {
        MechanicalMountCollection::Suspension => "Mounts",
        MechanicalMountCollection::Closure => "Closure mounts",
    };
    rsx! {
        InspectorSection { title, badge: mounts.len().to_string(),
            for mount in mounts {
                div { key: "{mount.id}",
                    SelectField::<MountKind> { label: "Mount type", value: mount.kind.clone(), disabled_reason: reason.clone(),
                        options: vec![(MountKind::Hole, "Hole".into()), (MountKind::Boss, "Boss".into())],
                        on_commit: { let id = mount.id.clone(); move |kind| send.send(MechanicalSettingsPatch::SetMountKind { collection, mount_id: id.clone(), kind }) } }
                    for (label, field, value) in [
                        ("Position X", MechanicalDimension::MountPositionX, mount.at.x),
                        ("Position Y", MechanicalDimension::MountPositionY, mount.at.y),
                        ("Hole diameter", MechanicalDimension::MountHoleDiameter, mount.hole_diameter),
                        ("Boss diameter", MechanicalDimension::MountBossDiameter, mount.boss_diameter.unwrap_or(5.0)),
                        ("Boss height", MechanicalDimension::MountBossHeight, mount.height.unwrap_or(5.0)),
                    ] {
                        {dimension(label, field, value, reason.clone(), { let id = mount.id.clone(); move |value| send.send(MechanicalSettingsPatch::SetMountDimension { collection, mount_id: id.clone(), field, value }) })}
                    }
                    Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: reason.is_some(),
                        onclick: { let id = mount.id.clone(); move |_| send.send(MechanicalSettingsPatch::RemoveMount { collection, mount_id: id.clone() }) }, "Remove mount" }
                }
            }
            Button { disabled: reason.is_some(), onclick: move |_| send.send(MechanicalSettingsPatch::AddMount { collection }), "Add mount" }
        }
    }
}
