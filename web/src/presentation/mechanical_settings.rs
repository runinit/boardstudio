//! Private read-only presentation and field intents for the Case mechanical stack.
//! Root owns accepted-state admission and commits every intent through Runtime.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{
    MechanicalBottomStyle, MechanicalMount, MechanicalSwitchFamily, PlateMethod, Severity, Vec2,
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use std::rc::Rc;
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsIdentity {
    pub(crate) editor_instance_id: u64,
    /// Advances whenever the mounted Case presentation is superseded, including away-and-back
    /// navigation that returns to an equal Scope.
    pub(crate) scope_generation: u64,
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) active_board_id: String,
    /// The accepted configuration's board, or the active board for Configure.
    pub(crate) configuration_board_id: String,
}

/// Narrow accepted values used by this control group; it is not an editable
/// configuration copy and deliberately excludes every field this ticket leaves alone.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsValues {
    pub(crate) board_id: String,
    pub(crate) method: PlateMethod,
    pub(crate) mount: MechanicalMount,
    pub(crate) bottom_style: MechanicalBottomStyle,
    pub(crate) middle_frame: bool,
    pub(crate) integrated_plate_frame: bool,
    pub(crate) plate_thickness: f64,
    pub(crate) plate_foam_thickness: f64,
    pub(crate) pcb_thickness: f64,
    pub(crate) bottom_foam_thickness: f64,
    pub(crate) bottom_thickness: f64,
    pub(crate) wall_thickness: f64,
    pub(crate) clearance: f64,
    pub(crate) opening_allowance: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalProfileChoice {
    pub(crate) definition_id: String,
    pub(crate) name: String,
    pub(crate) source: Option<String>,
    pub(crate) family: Option<MechanicalSwitchFamily>,
    pub(crate) plate_to_pcb: Option<f64>,
    pub(crate) supported_thickness: Option<Vec2>,
    /// True only for a placed switch definition selected by the accepted parent projection.
    pub(crate) switch_family_selectable: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalLayerRow {
    pub(crate) id: String,
    pub(crate) label: String,
    pub(crate) z: f64,
    pub(crate) thickness: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalFindingRow {
    pub(crate) id: String,
    pub(crate) severity: Severity,
    pub(crate) message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalBoardMismatch {
    pub(crate) board_id: String,
    pub(crate) board_name: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalDimension {
    PlateThickness,
    PlateFoamThickness,
    PcbThickness,
    BottomFoamThickness,
    BottomThickness,
    WallThickness,
    Clearance,
    OpeningAllowance,
}

impl MechanicalDimension {
    fn field_id(self) -> &'static str {
        match self {
            Self::PlateThickness => "plate-thickness",
            Self::PlateFoamThickness => "plate-foam-thickness",
            Self::PcbThickness => "pcb-thickness",
            Self::BottomFoamThickness => "bottom-foam-thickness",
            Self::BottomThickness => "bottom-thickness",
            Self::WallThickness => "wall-thickness",
            Self::Clearance => "clearance",
            Self::OpeningAllowance => "opening-allowance",
        }
    }

    fn rule(self) -> NumberRule {
        match self {
            Self::OpeningAllowance => NumberRule::Bounded(-1, 1),
            _ => NumberRule::Nonnegative,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum MechanicalSettingsPatch {
    Enable,
    Disable,
    SetMethod(PlateMethod),
    SetMount(MechanicalMount),
    SetBottomStyle(MechanicalBottomStyle),
    SetMiddleFrame(bool),
    SetIntegratedPlateFrame(bool),
    SetDimension {
        field: MechanicalDimension,
        value: f64,
    },
    SetSwitchFamily {
        definition_id: String,
        family: MechanicalSwitchFamily,
    },
}

impl MechanicalSettingsPatch {
    fn field_id(&self) -> String {
        match self {
            Self::Enable => "configure".to_owned(),
            Self::Disable => "disable".to_owned(),
            Self::SetMethod(_) => "method".to_owned(),
            Self::SetMount(_) => "mount".to_owned(),
            Self::SetBottomStyle(_) => "bottom-style".to_owned(),
            Self::SetMiddleFrame(_) => "middle-frame".to_owned(),
            Self::SetIntegratedPlateFrame(_) => "integrated-plate-frame".to_owned(),
            Self::SetDimension { field, .. } => field.field_id().to_owned(),
            Self::SetSwitchFamily { definition_id, .. } => {
                format!("switch-family:{definition_id}")
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsRequest {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_id: u64,
    pub(crate) field_id: String,
    pub(crate) patch: MechanicalSettingsPatch,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MechanicalSettingsFeedbackState {
    Pending,
    Saved,
    Failed,
}

/// Root echoes this full request identity and the field owner in all states.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MechanicalSettingsFeedback {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) request_id: u64,
    pub(crate) field_id: String,
    pub(crate) state: MechanicalSettingsFeedbackState,
    pub(crate) message: Option<String>,
}

#[derive(Props, Clone, PartialEq)]
pub(crate) struct MechanicalSettingsProps {
    pub(crate) identity: MechanicalSettingsIdentity,
    pub(crate) values: Option<MechanicalSettingsValues>,
    pub(crate) profiles: Rc<[MechanicalProfileChoice]>,
    pub(crate) layers: Rc<[MechanicalLayerRow]>,
    pub(crate) findings: Rc<[MechanicalFindingRow]>,
    pub(crate) selected_layer: String,
    pub(crate) mismatch: Option<MechanicalBoardMismatch>,
    pub(crate) editable: bool,
    pub(crate) disabled_reason: Option<String>,
    /// Bounded per-request feedback keeps a rejected raced submit from replacing the
    /// currently admitted operation's Pending/Saved response.
    pub(crate) feedback: Rc<[MechanicalSettingsFeedback]>,
    pub(crate) on_request: EventHandler<MechanicalSettingsRequest>,
    pub(crate) on_select_layer: EventHandler<String>,
    pub(crate) on_show_finding: EventHandler<String>,
    pub(crate) on_show_configured_board: EventHandler<String>,
}

#[component]
pub(crate) fn MechanicalSettings(props: MechanicalSettingsProps) -> Element {
    let mut request_sequence = use_signal(|| 0_u64);
    let identity = &props.identity;
    let configuration_matches = props
        .values
        .as_ref()
        .is_some_and(|values| values.board_id == identity.active_board_id);
    let mismatch = props.mismatch.as_ref();
    let owner_key = format!(
        "{}:{}:{}:{}:{}:{:?}",
        identity.editor_instance_id,
        identity.scope_generation,
        identity.scope.session_epoch.0,
        identity.scope.document_id,
        identity.scope.board_id,
        identity.scope.instance_id,
    );
    let current_feedback = props
        .feedback
        .iter()
        .filter(|feedback| {
            feedback.identity.editor_instance_id == identity.editor_instance_id
                && feedback.identity.scope_generation == identity.scope_generation
                && feedback.identity.scope == identity.scope
                && feedback.identity.active_board_id == identity.active_board_id
                && feedback.identity.configuration_board_id == identity.configuration_board_id
        })
        .max_by_key(|feedback| feedback.request_id);

    rsx! {
        section { class: "m1-mechanical-settings", aria_label: "Mechanical stack settings",
            h2 { "Case construction" }
            if let Some(mismatch) = mismatch {
                div { role: "status", class: "m1-mechanical-mismatch",
                    p { "The mechanical stack belongs to {mismatch.board_name}. This board remains in authored Case mode." }
                    button {
                        r#type: "button",
                        onclick: {
                            let on_show = props.on_show_configured_board;
                            let id = mismatch.board_id.clone();
                            move |_| on_show.call(id.clone())
                        },
                        "Show configured board"
                    }
                }
            } else if let Some(values) = props.values.as_ref().filter(|_| configuration_matches) {
                if let Some(feedback) = current_feedback {
                    match feedback.state {
                        MechanicalSettingsFeedbackState::Pending => p { role: "status", "Saving mechanical settings…" },
                        MechanicalSettingsFeedbackState::Saved => p { role: "status", "Mechanical settings saved." },
                        MechanicalSettingsFeedbackState::Failed => if let Some(message) = feedback.message.as_deref() { p { role: "alert", "{message}" } },
                    }
                }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() {
                        p { role: "status", "{reason}" }
                    }
                }
                p { class: "m1-mechanical-help", "Authored Case bodies remain saved while the generated stack is active. Disable the stack to return to authored bodies." }
                ConstructionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                ProfileGuidance {
                    identity: props.identity.clone(),
                    profiles: props.profiles.clone(),
                    plate_thickness: values.plate_thickness,
                    editable: props.editable,
                    request_sequence,
                    on_request: props.on_request,
                }
                DimensionControls {
                    identity: props.identity.clone(),
                    values: values.clone(),
                    editable: props.editable,
                    disabled_reason: props.disabled_reason.clone(),
                    feedback: props.feedback.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    owner_key: owner_key.clone(),
                }
                ResolvedStack {
                    layers: props.layers.clone(),
                    selected_layer: props.selected_layer.clone(),
                    on_select_layer: props.on_select_layer,
                }
                Diagnostics {
                    findings: props.findings.clone(),
                    on_show_finding: props.on_show_finding,
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-disable",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Disable)
                    },
                    "Disable mechanical stack"
                }
            } else if props.values.is_some() {
                p { role: "status", "The configured stack belongs to another board. Switch to that board to edit it." }
            } else {
                if let Some(feedback) = current_feedback {
                    if feedback.state == MechanicalSettingsFeedbackState::Pending {
                        p { role: "status", "Saving mechanical settings…" }
                    } else if feedback.state == MechanicalSettingsFeedbackState::Failed {
                        if let Some(message) = feedback.message.as_deref() { p { role: "alert", "{message}" } }
                    }
                }
                p { class: "m1-mechanical-help", "Configure a mechanical stack from the current board and represented switch fit profiles." }
                if !props.editable {
                    if let Some(reason) = props.disabled_reason.as_deref() { p { role: "status", "{reason}" } }
                }
                button {
                    r#type: "button",
                    class: "m1-mechanical-configure",
                    disabled: !props.editable,
                    onclick: {
                        let identity = props.identity.clone();
                        let mut sequence = request_sequence;
                        let on_request = props.on_request;
                        move |_| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::Enable)
                    },
                    "Configure mechanical stack"
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NumberRule {
    Nonnegative,
    Bounded(i8, i8),
}

impl NumberRule {
    fn error(self) -> &'static str {
        match self {
            Self::Nonnegative => "Enter a finite value of zero or greater.",
            Self::Bounded(min, max) => {
                if min == -1 && max == 1 {
                    "Enter a finite value from -1 to 1 mm."
                } else {
                    "Enter a finite value within the supported range."
                }
            }
        }
    }

    fn valid(self, value: f64) -> bool {
        value.is_finite()
            && match self {
                Self::Nonnegative => value >= 0.0,
                Self::Bounded(min, max) => value >= f64::from(min) && value <= f64::from(max),
            }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ConstructionControlsProps {
    identity: MechanicalSettingsIdentity,
    values: MechanicalSettingsValues,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn ConstructionControls(props: ConstructionControlsProps) -> Element {
    let values = &props.values;
    rsx! {
        fieldset { class: "m1-mechanical-group", disabled: !props.editable,
            legend { "Construction" }
            label { class: "m1-mechanical-field",
                span { "Plate method" }
                select {
                    value: "{method_id(&values.method)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(method) = parse_method(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMethod(method));
                        }
                    },
                    option { value: "pcb-fr4", "PCB FR-4" }
                    option { value: "printed", "3D printed" }
                    option { value: "cnc", "CNC machined" }
                    option { value: "cut-sheet", "Cut sheet" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Mount style" }
                select {
                    value: "{mount_id(&values.mount)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(mount) = parse_mount(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMount(mount));
                        }
                    },
                    option { value: "tray", "Tray" }
                    option { value: "rigid", "Rigid mount" }
                    option { value: "gasket", "Gasket mount" }
                }
            }
            label { class: "m1-mechanical-field",
                span { "Bottom construction" }
                select {
                    disabled: values.mount == MechanicalMount::Gasket,
                    value: "{bottom_style_id(&values.bottom_style)}",
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| if let Some(style) = parse_bottom_style(&event.value()) {
                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetBottomStyle(style));
                        }
                    },
                    option { value: "shell", "Tray shell" }
                    option { value: "sheet", "Flat sheet" }
                }
            }
            if values.bottom_style == MechanicalBottomStyle::Sheet {
                label { class: "m1-mechanical-check",
                    input {
                        r#type: "checkbox",
                        checked: values.middle_frame,
                        onchange: {
                            let identity = props.identity.clone();
                            let mut sequence = props.request_sequence;
                            let on_request = props.on_request;
                            move |event: FormEvent| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetMiddleFrame(event.checked()))
                        },
                    }
                    span { "Add middle frame" }
                }
            }
            label { class: "m1-mechanical-check",
                input {
                    r#type: "checkbox",
                    disabled: values.mount == MechanicalMount::Gasket,
                    checked: values.integrated_plate_frame,
                    onchange: {
                        let identity = props.identity.clone();
                        let mut sequence = props.request_sequence;
                        let on_request = props.on_request;
                        move |event: FormEvent| send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetIntegratedPlateFrame(event.checked()))
                    },
                }
                span { "Integrate plate frame into case" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ProfileGuidanceProps {
    identity: MechanicalSettingsIdentity,
    profiles: Rc<[MechanicalProfileChoice]>,
    plate_thickness: f64,
    editable: bool,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
}

#[component]
fn ProfileGuidance(props: ProfileGuidanceProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Switch fit profiles",
            h3 { "Switch fit profiles" }
            if props.profiles.is_empty() {
                p { class: "m1-mechanical-help", "No switch fit profile is available. Select or add a supported switch family in Parts to resolve the plate gap and supported thickness." }
            }
            for profile in props.profiles.iter() {
                div { class: "m1-mechanical-profile", key: "{profile.definition_id}",
                    strong { "{profile.name}" }
                    if let Some(source) = profile.source.as_deref() { p { "Profile source: {source}." } }
                    if profile.switch_family_selectable {
                        label { class: "m1-mechanical-field",
                            span { "Switch fit family" }
                            select {
                                disabled: !props.editable,
                                value: "{profile.family.map(family_id).unwrap_or("")}",
                                onchange: {
                                    let definition_id = profile.definition_id.clone();
                                    let identity = props.identity.clone();
                                    let mut sequence = props.request_sequence;
                                    let on_request = props.on_request;
                                    move |event: FormEvent| {
                                        if let Some(family) = parse_family(&event.value()) {
                                            send_request(&mut sequence, &identity, on_request, MechanicalSettingsPatch::SetSwitchFamily { definition_id: definition_id.clone(), family });
                                        }
                                    }
                                },
                                option { value: "", "Choose switch family…" }
                                option { value: "mx", "MX" }
                                option { value: "choc-v1", "Choc v1" }
                                option { value: "choc-v2", "Choc v2" }
                            }
                        }
                    } else if let Some(family) = profile.family {
                        p { "Switch fit: {family_label(family)}" }
                    }
                    if let Some(gap) = profile.plate_to_pcb {
                        p { "Plate underside to PCB top: {gap:.2} mm · derived from switch fit." }
                    }
                    if let Some(range) = profile.supported_thickness {
                        p { "Supported plate thickness: {range.x:.2}–{range.y:.2} mm" }
                    } else {
                        p { role: "status", "Supplier review needed for the supported thickness range." }
                    }
                }
            }
            if props.profiles.iter().any(|profile| profile.family.is_none()) {
                p { role: "status", "Choose a supported switch family to resolve the plate gap." }
            }
            p { "Current plate thickness: {props.plate_thickness:.2} mm." }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DimensionControlsProps {
    identity: MechanicalSettingsIdentity,
    values: MechanicalSettingsValues,
    editable: bool,
    disabled_reason: Option<String>,
    feedback: Rc<[MechanicalSettingsFeedback]>,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    owner_key: String,
}

#[component]
fn DimensionControls(props: DimensionControlsProps) -> Element {
    let fields = [
        (
            MechanicalDimension::PlateThickness,
            "Plate thickness",
            props.values.plate_thickness,
        ),
        (
            MechanicalDimension::PlateFoamThickness,
            "Plate foam",
            props.values.plate_foam_thickness,
        ),
        (
            MechanicalDimension::PcbThickness,
            "PCB thickness",
            props.values.pcb_thickness,
        ),
        (
            MechanicalDimension::BottomFoamThickness,
            "Bottom foam",
            props.values.bottom_foam_thickness,
        ),
        (
            MechanicalDimension::BottomThickness,
            "Bottom thickness",
            props.values.bottom_thickness,
        ),
        (
            MechanicalDimension::WallThickness,
            "Wall thickness",
            props.values.wall_thickness,
        ),
        (
            MechanicalDimension::Clearance,
            "Clearance",
            props.values.clearance,
        ),
        (
            MechanicalDimension::OpeningAllowance,
            "Radial opening allowance",
            props.values.opening_allowance,
        ),
    ];
    let mut request_sequence = props.request_sequence;
    rsx! {
        fieldset { class: "m1-mechanical-group", disabled: !props.editable,
            legend { "Dimensions and clearances · mm" }
            for (field, label, value) in fields {
                DimensionField {
                    key: "{props.owner_key}:{field.field_id()}",
                    identity: props.identity.clone(),
                    request_sequence,
                    on_request: props.on_request,
                    feedback: props.feedback.clone(),
                    field,
                    label,
                    value,
                    editable: props.editable,
                }
            }
            if let Some(reason) = props.disabled_reason.as_deref() {
                p { role: "status", "{reason}" }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DimensionFieldProps {
    identity: MechanicalSettingsIdentity,
    request_sequence: Signal<u64>,
    on_request: EventHandler<MechanicalSettingsRequest>,
    feedback: Option<MechanicalSettingsFeedback>,
    field: MechanicalDimension,
    label: &'static str,
    value: f64,
    editable: bool,
}

#[component]
fn DimensionField(props: DimensionFieldProps) -> Element {
    let mut draft = use_signal(|| props.value.to_string());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitted = use_signal(|| None::<MechanicalSettingsRequest>);
    let mut field_status = use_signal(|| None::<String>());
    let mut previous_accepted = use_signal(|| props.value);

    let saved_value = props.value;
    let feedback_entries = props.feedback.clone();
    let mut draft_for_ack = draft;
    let mut dirty_for_ack = dirty;
    let mut error_for_ack = error;
    let mut submitted_for_ack = submitted;
    let mut status_for_ack = field_status;
    let submitted_copy = submitted;
    use_effect(use_reactive(
        (&feedback_entries, &saved_value),
        move |(feedback_entries, accepted)| {
            if *previous_accepted.read() != accepted {
                previous_accepted.set(accepted);
                draft_for_ack.set(accepted.to_string());
                dirty_for_ack.set(false);
                error_for_ack.set(None);
                status_for_ack.set(None);
                submitted_for_ack.set(None);
                return;
            }
            let Some(request) = submitted_copy.read().clone() else {
                return;
            };
            let Some(feedback) = feedback_entries.iter().find(|feedback| {
                feedback.identity == request.identity
                    && feedback.request_id == request.request_id
                    && feedback.field_id == request.field_id
            }) else {
                return;
            };
            match feedback.state {
                MechanicalSettingsFeedbackState::Pending => {
                    status_for_ack.set(Some("Saving…".to_owned()))
                }
                MechanicalSettingsFeedbackState::Saved => {
                    draft_for_ack.set(accepted.to_string());
                    dirty_for_ack.set(false);
                    error_for_ack.set(None);
                    status_for_ack.set(Some("Saved".to_owned()));
                    submitted_for_ack.set(None);
                }
                MechanicalSettingsFeedbackState::Failed => {
                    error_for_ack.set(Some(feedback.message.clone().unwrap_or_else(|| {
                        "This setting was not saved. Review the value and retry.".to_owned()
                    })));
                    status_for_ack.set(None);
                    submitted_for_ack.set(None);
                }
            }
        },
    ));

    let commit: Rc<dyn Fn()> = Rc::new({
        let identity = props.identity.clone();
        let field = props.field;
        let accepted = props.value;
        let mut sequence = props.request_sequence;
        let on_request = props.on_request;
        move || {
            let text = draft();
            if submitted().is_some() {
                return;
            }
            let Ok(value) = text.trim().parse::<f64>() else {
                error.set(Some(field.rule().error().to_owned()));
                return;
            };
            if !field.rule().valid(value) {
                error.set(Some(field.rule().error().to_owned()));
                return;
            }
            error.set(None);
            if value == accepted {
                draft.set(accepted.to_string());
                dirty.set(false);
                field_status.set(None);
                submitted.set(None);
                return;
            }
            let patch = MechanicalSettingsPatch::SetDimension { field, value };
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Mechanical request identity is exhausted; reopen the Case inspector."
                        .to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MechanicalSettingsRequest {
                identity: identity.clone(),
                request_id,
                field_id: patch.field_id(),
                patch,
            };
            submitted.set(Some(request.clone()));
            field_status.set(Some("Saving…".to_owned()));
            on_request.call(request);
        }
    });
    let error_text = error();
    let status_text = field_status();
    rsx! {
        label { class: "m1-mechanical-dimension",
            span { "{props.label}" }
            span { class: "m1-mechanical-number",
                input {
                    r#type: "number",
                    step: "0.1",
                    min: match props.field.rule() { NumberRule::Nonnegative => "0", NumberRule::Bounded(_, _) => "-1" },
                    max: if props.field == MechanicalDimension::OpeningAllowance { "1" },
                    value: "{draft}",
                    disabled: !props.editable,
                    "aria-invalid": error_text.is_some(),
                    oninput: move |event: FormEvent| {
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        field_status.set(None);
                        submitted.set(None);
                    },
                    onblur: {
                        let commit = commit.clone();
                        move |_| commit()
                    },
                    onkeydown: {
                        let accepted = props.value;
                        move |event: KeyboardEvent| {
                            let key = event.data().key().to_string();
                            if key == "Enter" {
                                event.prevent_default();
                                if let Some(input) = event
                                    .data()
                                    .try_as_web_event()
                                    .and_then(|event| event.current_target())
                                    .and_then(|target| target.dyn_into::<HtmlInputElement>().ok())
                                {
                                    let _ = input.blur();
                                }
                            } else if key == "Escape" {
                                event.prevent_default();
                                draft.set(accepted.to_string());
                                dirty.set(false);
                                error.set(None);
                                submitted.set(None);
                                field_status.set(None);
                            }
                        }
                    }
                }
                small { "mm" }
            }
            if let Some(error) = error_text { small { role: "alert", "{error}" } }
            if let Some(status) = status_text { small { role: "status", "{status}" } }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct ResolvedStackProps {
    layers: Rc<[MechanicalLayerRow]>,
    selected_layer: String,
    on_select_layer: EventHandler<String>,
}

#[component]
fn ResolvedStack(props: ResolvedStackProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Resolved mechanical stack",
            h3 { "Resolved stack · {props.layers.len()} layers" }
            if props.layers.is_empty() {
                p { role: "status", "The stack appears after the current revision resolves." }
            } else {
                div { class: "m1-mechanical-stack",
                    for row in props.layers.iter() {
                        let id = row.id.clone();
                        let label = row.label.clone();
                        let selected = props.selected_layer == id;
                        let callback = props.on_select_layer;
                        button {
                            key: "{id}",
                            r#type: "button",
                            class: if selected { "m1-mechanical-layer is-selected" } else { "m1-mechanical-layer" },
                            aria_pressed: selected,
                            onclick: move |_| callback.call(if selected { String::new() } else { id.clone() }),
                            strong { "{label}" }
                            span { "{row.thickness:.2} mm" }
                            small { "Z {row.z:.2}" }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DiagnosticsProps {
    findings: Rc<[MechanicalFindingRow]>,
    on_show_finding: EventHandler<String>,
}

#[component]
fn Diagnostics(props: DiagnosticsProps) -> Element {
    rsx! {
        section { class: "m1-mechanical-group", aria_label: "Mechanical diagnostics",
            h3 { "Mechanical diagnostics · {props.findings.len()}" }
            if props.findings.is_empty() {
                p { "No current mechanical findings." }
            }
            ul {
                for row in props.findings.iter() {
                    let id = row.id.clone();
                    let severity = severity_label(&row.severity);
                    let callback = props.on_show_finding;
                    li { key: "{id}",
                        span { "{severity}: {row.message}" }
                        button {
                            r#type: "button",
                            onclick: move |_| callback.call(id.clone()),
                            "Show"
                        }
                    }
                }
            }
        }
    }
}

fn send_request(
    sequence: &mut Signal<u64>,
    identity: &MechanicalSettingsIdentity,
    on_request: EventHandler<MechanicalSettingsRequest>,
    patch: MechanicalSettingsPatch,
) {
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    on_request.call(MechanicalSettingsRequest {
        identity: identity.clone(),
        request_id,
        field_id: patch.field_id(),
        patch,
    });
}

fn method_id(value: &PlateMethod) -> &'static str {
    match value {
        PlateMethod::PcbFr4 => "pcb-fr4",
        PlateMethod::Printed => "printed",
        PlateMethod::Cnc => "cnc",
        PlateMethod::CutSheet => "cut-sheet",
    }
}

fn parse_method(value: &str) -> Option<PlateMethod> {
    Some(match value {
        "pcb-fr4" => PlateMethod::PcbFr4,
        "printed" => PlateMethod::Printed,
        "cnc" => PlateMethod::Cnc,
        "cut-sheet" => PlateMethod::CutSheet,
        _ => return None,
    })
}

fn mount_id(value: &MechanicalMount) -> &'static str {
    match value {
        MechanicalMount::Tray => "tray",
        MechanicalMount::Rigid => "rigid",
        MechanicalMount::Gasket => "gasket",
    }
}

fn parse_mount(value: &str) -> Option<MechanicalMount> {
    Some(match value {
        "tray" => MechanicalMount::Tray,
        "rigid" => MechanicalMount::Rigid,
        "gasket" => MechanicalMount::Gasket,
        _ => return None,
    })
}

fn bottom_style_id(value: &MechanicalBottomStyle) -> &'static str {
    match value {
        MechanicalBottomStyle::Shell => "shell",
        MechanicalBottomStyle::Sheet => "sheet",
    }
}

fn parse_bottom_style(value: &str) -> Option<MechanicalBottomStyle> {
    Some(match value {
        "shell" => MechanicalBottomStyle::Shell,
        "sheet" => MechanicalBottomStyle::Sheet,
        _ => return None,
    })
}

fn family_id(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "mx",
        MechanicalSwitchFamily::ChocV1 => "choc-v1",
        MechanicalSwitchFamily::ChocV2 => "choc-v2",
    }
}

fn family_label(value: MechanicalSwitchFamily) -> &'static str {
    match value {
        MechanicalSwitchFamily::Mx => "MX",
        MechanicalSwitchFamily::ChocV1 => "Choc v1",
        MechanicalSwitchFamily::ChocV2 => "Choc v2",
    }
}

fn parse_family(value: &str) -> Option<MechanicalSwitchFamily> {
    Some(match value {
        "mx" => MechanicalSwitchFamily::Mx,
        "choc-v1" => MechanicalSwitchFamily::ChocV1,
        "choc-v2" => MechanicalSwitchFamily::ChocV2,
        _ => return None,
    })
}

fn severity_label(value: &Severity) -> &'static str {
    match value {
        Severity::Error => "Error",
        Severity::Warning => "Warning",
        Severity::Info => "Info",
    }
}
