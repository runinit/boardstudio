//! Private form for the real selected-matrix name, size, and pitch fields.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::DiodeDirection;
use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MatrixNameTarget {
    Matrix,
    Layout { id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixEditField {
    Name,
    Rows,
    Columns,
    PitchX,
    PitchY,
    SwitchDefinition,
    ApplyPreset,
    DiodeDirection,
    EdgeGapX,
    EdgeGapY,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixPreset {
    MxSolder,
    MxHotswap,
    ChocSolder,
    ChocHotswap,
    MxRgb,
    ChocRgb,
    MxHotswapRgb,
    ChocHotswapRgb,
}

impl MatrixPreset {
    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.as_str() == value)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::MxSolder => "mx-solder",
            Self::MxHotswap => "mx-hotswap",
            Self::ChocSolder => "choc-solder",
            Self::ChocHotswap => "choc-hotswap",
            Self::MxRgb => "mx-rgb",
            Self::ChocRgb => "choc-rgb",
            Self::MxHotswapRgb => "mx-hotswap-rgb",
            Self::ChocHotswapRgb => "choc-hotswap-rgb",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::MxSolder => "MX Solder",
            Self::MxHotswap => "MX Hotswap",
            Self::ChocSolder => "Choc V1 Solder",
            Self::ChocHotswap => "Choc V1 Hotswap",
            Self::MxRgb => "MX RGB",
            Self::ChocRgb => "Choc V1 RGB",
            Self::MxHotswapRgb => "MX Hotswap RGB",
            Self::ChocHotswapRgb => "Choc V1 Hotswap RGB",
        }
    }

    const ALL: [Self; 8] = [
        Self::MxSolder,
        Self::MxHotswap,
        Self::ChocSolder,
        Self::ChocHotswap,
        Self::MxRgb,
        Self::ChocRgb,
        Self::MxHotswapRgb,
        Self::ChocHotswapRgb,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchOrientation {
    South,
    North,
}

impl SwitchOrientation {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::South => "south",
            Self::North => "north",
        }
    }
}

impl MatrixEditField {
    fn key(self) -> &'static str {
        match self {
            Self::Name => "name",
            Self::Rows => "rows",
            Self::Columns => "columns",
            Self::PitchX => "pitch-x",
            Self::PitchY => "pitch-y",
            Self::SwitchDefinition => "switch-definition",
            Self::ApplyPreset => "apply-preset",
            Self::DiodeDirection => "diode-direction",
            Self::EdgeGapX => "edge-gap-x",
            Self::EdgeGapY => "edge-gap-y",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum MatrixEditValue {
    Name(Option<String>),
    Rows(u32),
    Columns(u32),
    PitchX(f64),
    PitchY(f64),
    SwitchDefinition(String),
    DiodeDirection(DiodeDirection),
    EdgeGapX(f64),
    EdgeGapY(f64),
}

/// Draft owner excludes accepted token/revision so unrelated accepted edits do not erase text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixInspectorOwner {
    pub editor_instance_id: u64,
    pub context_generation: u64,
    pub scope_generation: u64,
    pub scope: Scope,
    pub matrix_id: String,
    pub name_target: MatrixNameTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixInspectorProjection {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub matrix_label: String,
    pub name_label: &'static str,
    pub name_value: String,
    pub name_baseline: MatrixEditValue,
    pub rows: u32,
    pub columns: u32,
    pub pitch_x: f64,
    pub pitch_y: f64,
    pub definition_id: String,
    pub switch_choices: Vec<(String, String)>,
    pub diode_direction: DiodeDirection,
    pub edge_gap_x: f64,
    pub edge_gap_y: f64,
    pub preset: Option<MatrixPreset>,
    pub orientation: Option<SwitchOrientation>,
    pub baseline_variant: Option<String>,
    pub layout_relation: Option<MatrixLayoutRelation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixLayoutRelation {
    pub partner_name: Option<String>,
    pub unlink_layout_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixEditRequest {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixEditField,
    pub baseline: MatrixEditValue,
    pub value: MatrixEditValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixPresetRequest {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub baseline_variant: Option<String>,
    pub preset: MatrixPreset,
    pub orientation: SwitchOrientation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixDeleteRequest {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixUnlinkRequest {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub layout_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatrixDuplicateRequest {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub preset: MatrixPreset,
    pub orientation: SwitchOrientation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatrixEditState {
    Pending,
    Saved,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixEditFeedback {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub field: MatrixEditField,
    pub state: MatrixEditState,
    pub message: Option<String>,
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixInspectorProps {
    pub projection: MatrixInspectorProjection,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
    pub on_apply_preset: EventHandler<MatrixPresetRequest>,
    pub on_delete: EventHandler<MatrixDeleteRequest>,
    pub on_unlink: EventHandler<MatrixUnlinkRequest>,
    pub on_duplicate: EventHandler<MatrixDuplicateRequest>,
}

#[component]
pub fn MatrixInspector(props: MatrixInspectorProps) -> Element {
    let projection = &props.projection;
    let definition_label = projection
        .switch_choices
        .iter()
        .find(|(id, _)| id == &projection.definition_id)
        .map(|(_, label)| label.clone())
        .unwrap_or_else(|| projection.definition_id.clone());
    let mut preset_draft = use_signal(|| projection.preset.unwrap_or(MatrixPreset::MxSolder));
    let mut orientation_draft =
        use_signal(|| projection.orientation.unwrap_or(SwitchOrientation::South));
    let accepted_owner = projection.owner.clone();
    let accepted_preset = projection.preset;
    let accepted_orientation = projection.orientation;
    let mut preset_draft_for_effect = preset_draft;
    let mut orientation_draft_for_effect = orientation_draft;
    use_effect(use_reactive(
        (&accepted_owner, &accepted_preset, &accepted_orientation),
        move |(_, preset, orientation)| {
            preset_draft_for_effect.set(preset.unwrap_or(MatrixPreset::MxSolder));
            orientation_draft_for_effect.set(orientation.unwrap_or(SwitchOrientation::South));
        },
    ));
    let mut apply_sequence = props.request_sequence;
    let apply_owner = projection.owner.clone();
    let apply_token = projection.snapshot_token;
    let apply_revision = projection.revision;
    let apply_baseline_variant = projection.baseline_variant.clone();
    let on_apply = props.on_apply_preset;
    let preset_request_id = use_signal(|| None::<u64>);
    let mut submitted_preset_id = preset_request_id;
    let editable = props.editable;
    let busy = props.busy;
    let mut apply_preset = move || {
        if !editable || busy {
            return;
        }
        let Some(request_id) = apply_sequence().checked_add(1) else {
            return;
        };
        apply_sequence.set(request_id);
        submitted_preset_id.set(Some(request_id));
        on_apply.call(MatrixPresetRequest {
            owner: apply_owner.clone(),
            request_id,
            snapshot_token: apply_token,
            revision: apply_revision,
            baseline_variant: apply_baseline_variant.clone(),
            preset: preset_draft(),
            orientation: orientation_draft(),
        });
    };
    let delete_request = MatrixDeleteRequest {
        owner: projection.owner.clone(),
        snapshot_token: projection.snapshot_token,
        revision: projection.revision,
    };
    let duplicate_base = MatrixDuplicateRequest {
        owner: projection.owner.clone(),
        snapshot_token: projection.snapshot_token,
        revision: projection.revision,
        preset: preset_draft(),
        orientation: orientation_draft(),
    };
    let unlink_request = projection
        .layout_relation
        .as_ref()
        .and_then(|relation| relation.unlink_layout_id.as_ref())
        .map(|layout_id| MatrixUnlinkRequest {
            owner: projection.owner.clone(),
            snapshot_token: projection.snapshot_token,
            revision: projection.revision,
            layout_id: layout_id.clone(),
        });
    let preset_feedback = props.feedback.iter().find(|feedback| {
        feedback.owner == projection.owner
            && feedback.field == MatrixEditField::ApplyPreset
            && preset_request_id() == Some(feedback.request_id)
    });
    let name_feedback = props.feedback.clone();
    let rows_feedback = props.feedback.clone();
    let columns_feedback = props.feedback.clone();
    let pitch_x_feedback = props.feedback.clone();
    let pitch_y_feedback = props.feedback.clone();
    let switch_feedback = props.feedback.clone();
    let diode_feedback = props.feedback.clone();
    let edge_gap_x_feedback = props.feedback.clone();
    let edge_gap_y_feedback = props.feedback.clone();
    let name_key = owner_key(&projection.owner, MatrixEditField::Name);
    let rows_key = owner_key(&projection.owner, MatrixEditField::Rows);
    let columns_key = owner_key(&projection.owner, MatrixEditField::Columns);
    let pitch_x_key = owner_key(&projection.owner, MatrixEditField::PitchX);
    let pitch_y_key = owner_key(&projection.owner, MatrixEditField::PitchY);
    let switch_key = owner_key(&projection.owner, MatrixEditField::SwitchDefinition);
    let diode_key = owner_key(&projection.owner, MatrixEditField::DiodeDirection);
    let edge_gap_x_key = owner_key(&projection.owner, MatrixEditField::EdgeGapX);
    let edge_gap_y_key = owner_key(&projection.owner, MatrixEditField::EdgeGapY);
    // Each field needs its own template root: nested component keys do not
    // create an identity boundary in Dioxus static templates.
    rsx! {
        section { class: "m1-matrix-inspector", aria_label: "Matrix inspector",
            header { class: "m1-matrix-inspector-heading",
                h2 { "Matrix" }
                span { "{projection.matrix_label}" }
            }
            div { class: "m1-matrix-inspector-fields",
                {rsx! {
                MatrixFieldEditor {
                    key: "{name_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Name,
                    label: projection.name_label, value: projection.name_value.clone(),
                    baseline: projection.name_baseline.clone(), kind: MatrixFieldKind::Name,
                    request_sequence: props.request_sequence, editable: props.editable, 
                    feedback: name_feedback, on_edit: props.on_edit,
                }
                }}
                {rsx! {
                MatrixFieldEditor {
                    key: "{rows_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Rows,
                    label: "Rows", value: projection.rows.to_string(),
                    baseline: MatrixEditValue::Rows(projection.rows), kind: MatrixFieldKind::PositiveInteger,
                    request_sequence: props.request_sequence, editable: props.editable, 
                    feedback: rows_feedback, on_edit: props.on_edit,
                }
                }}
                {rsx! {
                MatrixFieldEditor {
                    key: "{columns_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::Columns,
                    label: "Columns", value: projection.columns.to_string(),
                    baseline: MatrixEditValue::Columns(projection.columns), kind: MatrixFieldKind::PositiveInteger,
                    request_sequence: props.request_sequence, editable: props.editable, 
                    feedback: columns_feedback, on_edit: props.on_edit,
                }
                }}
                {rsx! {
                MatrixFieldEditor {
                    key: "{pitch_x_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::PitchX,
                    label: "Pitch X", value: projection.pitch_x.to_string(),
                    baseline: MatrixEditValue::PitchX(projection.pitch_x), kind: MatrixFieldKind::PositiveNumber,
                    request_sequence: props.request_sequence, editable: props.editable, 
                    feedback: pitch_x_feedback, on_edit: props.on_edit,
                }
                }}
                {rsx! {
                MatrixFieldEditor {
                    key: "{pitch_y_key}",
                    owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                    revision: projection.revision, field: MatrixEditField::PitchY,
                    label: "Pitch Y", value: projection.pitch_y.to_string(),
                    baseline: MatrixEditValue::PitchY(projection.pitch_y), kind: MatrixFieldKind::PositiveNumber,
                    request_sequence: props.request_sequence, editable: props.editable, 
                    feedback: pitch_y_feedback, on_edit: props.on_edit,
                }
                }}
            }
            if let Some(relation) = projection.layout_relation.as_ref() {
                div { class: "m1-layout-link",
                    strong {
                        if let Some(partner_name) = relation.partner_name.as_ref() {
                            "Linked to {partner_name}"
                        } else {
                            "Independent layout"
                        }
                    }
                    if relation.partner_name.is_some() {
                        p { "Key assemblies, diodes and components mirror across both halves. Replace a component on one half to keep it local." }
                    } else {
                        p { "Geometry and components can be edited independently." }
                    }
                    if let Some(request) = unlink_request.clone() {
                        button {
                            disabled: !props.editable || props.busy,
                            onclick: move |_| props.on_unlink.call(request.clone()),
                            "Unlink halves"
                        }
                    }
                }
            }
            details { class: "m1-matrix-inspector-section",
                summary { span { "Key assembly" } small { "{definition_label}" } }
                div { class: "m1-matrix-inspector-fields",
                label { class: "m1-matrix-field",
                    span { "Assembly preset" }
                    select {
                        aria_label: "Apply matrix preset",
                        value: "{preset_draft().as_str()}",
                        disabled: !props.editable || props.busy,
                        onchange: move |event: FormEvent| {
                            if let Some(preset) = MatrixPreset::parse(&event.value()) {
                                preset_draft.set(preset);
                            }
                        },
                        for preset in MatrixPreset::ALL {
                            option { value: "{preset.as_str()}", selected: preset_draft() == preset, "{preset.label()}" }
                        }
                    }
                }
                label { class: "m1-matrix-field",
                    span { "Switch orientation" }
                    select {
                        aria_label: "Switch orientation",
                        value: "{orientation_draft().as_str()}",
                        disabled: !props.editable || props.busy,
                        onchange: move |event: FormEvent| {
                            orientation_draft.set(if event.value() == "north" { SwitchOrientation::North } else { SwitchOrientation::South });
                        },
                        option { value: "south", selected: orientation_draft() == SwitchOrientation::South, "South-facing LED" }
                        option { value: "north", selected: orientation_draft() == SwitchOrientation::North, "North-facing LED" }
                    }
                }
                button {
                    disabled: !props.editable || props.busy,
                    onclick: move |_| apply_preset(),
                    "Update assembly preset"
                }
                button {
                    disabled: props.busy,
                    onclick: move |_| props.on_duplicate.call(duplicate_base.clone()),
                    "Duplicate design as variant"
                }
                {rsx! {
                    MatrixFieldEditor {
                        key: "{switch_key}", owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                        revision: projection.revision, field: MatrixEditField::SwitchDefinition,
                        label: "Switch footprint", value: projection.definition_id.clone(),
                        baseline: MatrixEditValue::SwitchDefinition(projection.definition_id.clone()), kind: MatrixFieldKind::Choice,
                        choices: projection.switch_choices.clone(),
                        request_sequence: props.request_sequence, editable: props.editable, 
                        feedback: switch_feedback, on_edit: props.on_edit,
                    }
                }}
                {rsx! {
                    MatrixFieldEditor {
                        key: "{diode_key}", owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                        revision: projection.revision, field: MatrixEditField::DiodeDirection,
                        label: "Diode direction", value: diode_direction_value(projection.diode_direction),
                        baseline: MatrixEditValue::DiodeDirection(projection.diode_direction), kind: MatrixFieldKind::Choice,
                        choices: vec![("row2col".to_owned(), "Rows to columns".to_owned()), ("col2row".to_owned(), "Columns to rows".to_owned())],
                        request_sequence: props.request_sequence, editable: props.editable, 
                        feedback: diode_feedback, on_edit: props.on_edit,
                    }
                }}
                if let Some(feedback) = preset_feedback {
                    if feedback.state == MatrixEditState::Failed {
                        p { role: "alert", "{feedback.message.as_deref().unwrap_or(\"The matrix preset was not saved.\")}" }
                    } else if feedback.state == MatrixEditState::Saved {
                        p { role: "status", "Preset updated" }
                    }
                }
                }
            }
            details { class: "m1-matrix-inspector-section",
                summary { span { "Keycap spacing" } small { "Preview only" } }
                div { class: "m1-matrix-inspector-fields",
                {rsx! {
                    MatrixFieldEditor {
                        key: "{edge_gap_x_key}", owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                        revision: projection.revision, field: MatrixEditField::EdgeGapX,
                        label: "Edge gap X", value: projection.edge_gap_x.to_string(),
                        baseline: MatrixEditValue::EdgeGapX(projection.edge_gap_x), kind: MatrixFieldKind::NonnegativeNumber,
                        choices: Vec::new(), request_sequence: props.request_sequence,
                        editable: props.editable,  feedback: edge_gap_x_feedback, on_edit: props.on_edit,
                    }
                }}
                {rsx! {
                    MatrixFieldEditor {
                        key: "{edge_gap_y_key}", owner: projection.owner.clone(), snapshot_token: projection.snapshot_token,
                        revision: projection.revision, field: MatrixEditField::EdgeGapY,
                        label: "Edge gap Y", value: projection.edge_gap_y.to_string(),
                        baseline: MatrixEditValue::EdgeGapY(projection.edge_gap_y), kind: MatrixFieldKind::NonnegativeNumber,
                        choices: Vec::new(), request_sequence: props.request_sequence,
                        editable: props.editable,  feedback: edge_gap_y_feedback, on_edit: props.on_edit,
                    }
                }}
            }
                p { class: "m1-matrix-edit-status", "Keycap preview {(projection.pitch_x - projection.edge_gap_x).max(0.0):.1} × {(projection.pitch_y - projection.edge_gap_y).max(0.0):.1} mm" }
            }
            details { class: "m1-matrix-inspector-section",
                summary { "Matrix actions" }
                button {
                    class: "m1-matrix-edit-status",
                    disabled: !props.editable || props.busy,
                    onclick: move |_| props.on_delete.call(delete_request.clone()),
                    "Delete matrix"
                }
            }
            if props.busy {
                p { class: "m1-matrix-edit-status", role: "status", "Saving matrix change…" }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MatrixFieldKind {
    Name,
    PositiveInteger,
    PositiveNumber,
    NonnegativeNumber,
    Choice,
}

#[derive(Props, Clone, PartialEq)]
struct MatrixFieldEditorProps {
    owner: MatrixInspectorOwner,
    snapshot_token: SnapshotToken,
    revision: u64,
    field: MatrixEditField,
    label: &'static str,
    value: String,
    baseline: MatrixEditValue,
    kind: MatrixFieldKind,
    #[props(default)]
    choices: Vec<(String, String)>,
    request_sequence: Signal<u64>,
    editable: bool,
    feedback: Vec<MatrixEditFeedback>,
    on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
fn MatrixFieldEditor(props: MatrixFieldEditorProps) -> Element {
    let mut draft = use_signal(|| props.value.clone());
    let mut draft_baseline = use_signal(|| props.baseline.clone());
    let mut dirty = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut submitted_request_id = use_signal(|| None::<u64>);
    let mut status = use_signal(|| None::<String>);
    let accepted_value = props.value.clone();
    let accepted_baseline = props.baseline.clone();
    let owner = props.owner.clone();
    let field = props.field;
    let feedback = props.feedback.clone();
    let editable = props.editable;
    let mut draft_for_effect = draft;
    let mut baseline_for_effect = draft_baseline;
    let mut dirty_for_effect = dirty;
    let mut error_for_effect = error;
    let mut submitted_for_effect = submitted_request_id;
    let mut status_for_effect = status;
    let submitted_id = submitted_request_id();
    let matching_feedback = feedback.into_iter().find(|feedback| {
        feedback.owner == owner
            && feedback.field == field
            && submitted_id == Some(feedback.request_id)
    });
    use_effect(use_reactive(
        (&accepted_value, &accepted_baseline, &matching_feedback),
        move |(value, baseline, feedback)| {
            match feedback.as_ref().map(|feedback| feedback.state) {
                Some(MatrixEditState::Pending) => {
                    status_for_effect.set(Some("Saving…".to_owned()));
                    return;
                }
                Some(MatrixEditState::Saved) => {
                    draft_for_effect.set(value.clone());
                    baseline_for_effect.set(baseline.clone());
                    dirty_for_effect.set(false);
                    error_for_effect.set(None);
                    submitted_for_effect.set(None);
                    status_for_effect.set(Some("Saved".to_owned()));
                    return;
                }
                Some(MatrixEditState::Failed) => {
                    submitted_for_effect.set(None);
                    status_for_effect.set(None);
                    error_for_effect.set(Some(
                        feedback
                            .as_ref()
                            .and_then(|item| item.message.clone())
                            .unwrap_or_else(|| {
                                "This matrix change was not saved. Review the value and retry."
                                    .to_owned()
                            }),
                    ));
                }
                None => {}
            }

            // A dirty draft keeps the user's value: the latest committed value wins.
            if baseline_for_effect() != baseline && !dirty_for_effect() {
                draft_for_effect.set(value.clone());
                baseline_for_effect.set(baseline.clone());
                error_for_effect.set(None);
                status_for_effect.set(None);
                submitted_for_effect.set(None);
            }
        },
    ));

    let commit = {
        let owner = props.owner.clone();
        let field = props.field;
        let kind = props.kind;
        let sequence = props.request_sequence;
        let snapshot_token = props.snapshot_token;
        let revision = props.revision;
        let on_edit = props.on_edit;
        let accepted_display = props.value.clone();
        let accepted_baseline_now = props.baseline.clone();
        move |replacement_text: Option<String>| {
            if !editable || submitted_request_id().is_some() {
                return;
            }
            let text = match replacement_text {
                Some(text) => text,
                None => draft(),
            };
            let value = match kind {
                MatrixFieldKind::Name => {
                    let name = text.trim();
                    if name.is_empty() {
                        error.set(Some("Enter a name.".to_owned()));
                        return;
                    }
                    MatrixEditValue::Name(Some(name.to_owned()))
                }
                MatrixFieldKind::PositiveInteger => match text.trim().parse::<u32>() {
                    Ok(value) if value > 0 => match field {
                        MatrixEditField::Rows => MatrixEditValue::Rows(value),
                        MatrixEditField::Columns => MatrixEditValue::Columns(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a positive whole number.".to_owned()));
                        return;
                    }
                },
                MatrixFieldKind::PositiveNumber => match text.trim().parse::<f64>() {
                    Ok(value) if value.is_finite() && value > 0.0 => match field {
                        MatrixEditField::PitchX => MatrixEditValue::PitchX(value),
                        MatrixEditField::PitchY => MatrixEditValue::PitchY(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some("Enter a finite number greater than zero.".to_owned()));
                        return;
                    }
                },
                MatrixFieldKind::NonnegativeNumber => match text.trim().parse::<f64>() {
                    Ok(value) if value.is_finite() && value >= 0.0 => match field {
                        MatrixEditField::EdgeGapX => MatrixEditValue::EdgeGapX(value),
                        MatrixEditField::EdgeGapY => MatrixEditValue::EdgeGapY(value),
                        _ => return,
                    },
                    _ => {
                        error.set(Some(
                            "Enter a finite number greater than or equal to zero.".to_owned(),
                        ));
                        return;
                    }
                },
                MatrixFieldKind::Choice => match field {
                    MatrixEditField::SwitchDefinition => {
                        MatrixEditValue::SwitchDefinition(text.clone())
                    }
                    MatrixEditField::DiodeDirection => match text.as_str() {
                        "row2col" => MatrixEditValue::DiodeDirection(DiodeDirection::Row2col),
                        "col2row" => MatrixEditValue::DiodeDirection(DiodeDirection::Col2row),
                        _ => return,
                    },
                    _ => return,
                },
            };
            let baseline = draft_baseline();
            let accepted_now = accepted_baseline_now.clone();
            let displayed_name_is_unchanged = field == MatrixEditField::Name
                && matches!(&value, MatrixEditValue::Name(Some(name)) if name == accepted_display.trim());
            if value == accepted_now || displayed_name_is_unchanged {
                draft.set(accepted_display.clone());
                dirty.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
                return;
            }
            let mut sequence = sequence;
            let Some(request_id) = sequence().checked_add(1) else {
                error.set(Some(
                    "Matrix request identity is exhausted; reopen the inspector.".to_owned(),
                ));
                return;
            };
            sequence.set(request_id);
            let request = MatrixEditRequest {
                owner: owner.clone(),
                request_id,
                snapshot_token,
                revision,
                field,
                baseline,
                value,
            };
            submitted_request_id.set(Some(request_id));
            dirty.set(true);
            error.set(None);
            status.set(Some("Saving…".to_owned()));
            on_edit.call(request);
        }
    };

    let on_keydown = {
        let mut commit = commit.clone();
        let accepted_baseline = props.baseline.clone();
        move |event: KeyboardEvent| match event.data().key() {
            Key::Enter => {
                event.prevent_default();
                commit(None);
            }
            Key::Escape => {
                event.prevent_default();
                draft.set(props.value.clone());
                draft_baseline.set(accepted_baseline.clone());
                dirty.set(false);
                error.set(None);
                status.set(None);
                submitted_request_id.set(None);
            }
            _ => {}
        }
    };
    let feedback_error = props
        .feedback
        .iter()
        .find(|feedback| {
            feedback.owner == props.owner
                && feedback.field == props.field
                && submitted_request_id() == Some(feedback.request_id)
                && feedback.state == MatrixEditState::Failed
        })
        .and_then(|feedback| feedback.message.clone());
    let error_text = error().or(feedback_error);
    let current_status = status();
    let input_value = draft();
    let current_baseline = props.baseline.clone();
    let input_type = if props.kind == MatrixFieldKind::Name {
        "text"
    } else {
        "number"
    };
    let step = match props.kind {
        MatrixFieldKind::Name => "any",
        MatrixFieldKind::PositiveInteger => "1",
        MatrixFieldKind::PositiveNumber | MatrixFieldKind::NonnegativeNumber => "any",
        MatrixFieldKind::Choice => "any",
    };
    let min = match props.kind {
        MatrixFieldKind::Name => None,
        MatrixFieldKind::PositiveInteger => Some("1"),
        MatrixFieldKind::PositiveNumber => Some("0"),
        MatrixFieldKind::NonnegativeNumber => Some("0"),
        MatrixFieldKind::Choice => None,
    };
    let mut commit_choice = commit.clone();
    let choices = props.choices.clone();
    let is_select = props.kind == MatrixFieldKind::Choice;
    rsx! {
        label { class: if error_text.is_some() { "m1-matrix-field has-error" } else { "m1-matrix-field" },
            span { "{props.label}" }
            span { class: "m1-matrix-field-input",
                if is_select {
                    select {
                        value: "{input_value}",
                        disabled: !props.editable,
                        "aria-label": if props.field == MatrixEditField::SwitchDefinition { "Matrix part definition" } else { props.label },
                        "aria-invalid": error_text.is_some(),
                        onchange: move |event: FormEvent| {
                            let value = event.value();
                            draft.set(value.clone());
                            dirty.set(true);
                            error.set(None);
                            status.set(Some("Saving…".to_owned()));
                            commit_choice(Some(value));
                        },
                        for (value, label) in choices {
                            option { value: "{value}", selected: input_value == value, "{label}" }
                        }
                    }
                } else {
                input {
                    r#type: input_type,
                    step: step,
                    min: min,
                    value: "{input_value}",
                    readonly: !props.editable,
                    "aria-label": if props.field == MatrixEditField::SwitchDefinition { "Matrix part definition" } else { props.label },
                    "aria-invalid": error_text.is_some(),
                    oninput: move |event: FormEvent| {
                        if !dirty() {
                            draft_baseline.set(current_baseline.clone());
                        }
                        draft.set(event.value());
                        dirty.set(true);
                        error.set(None);
                        status.set(None);
                        submitted_request_id.set(None);
                    },
                    onblur: {
                        let mut commit = commit.clone();
                        move |_| commit(None)
                    },
                    onkeydown: on_keydown,
                }
                }
                if matches!(props.kind, MatrixFieldKind::PositiveNumber | MatrixFieldKind::NonnegativeNumber) { small { "mm" } }
            }
            if let Some(message) = error_text.as_deref() {
                small { role: "alert", "{message}" }
            }
            if let Some(message) = current_status.as_deref() {
                small { role: "status", "{message}" }
            }
        }
    }
}

fn owner_key(owner: &MatrixInspectorOwner, field: MatrixEditField) -> String {
    fn encode(value: &str) -> String {
        use std::fmt::Write as _;
        let mut result = String::with_capacity(value.len() * 2);
        for byte in value.as_bytes() {
            let _ = write!(result, "{byte:02x}");
        }
        result
    }
    let instance = owner
        .scope
        .instance_id
        .as_deref()
        .map(encode)
        .unwrap_or_default();
    let target = match &owner.name_target {
        MatrixNameTarget::Matrix => "matrix".to_owned(),
        MatrixNameTarget::Layout { id } => format!("layout-{}", encode(id)),
    };
    format!(
        "{}-{}-{}-{}-{}-{}-{}-{}-{}",
        owner.editor_instance_id,
        owner.context_generation,
        owner.scope_generation,
        owner.scope.session_epoch.0,
        encode(&owner.scope.document_id),
        encode(&owner.scope.board_id),
        instance,
        encode(&owner.matrix_id),
        format_args!("{target}-{}", field.key()),
    )
}

fn diode_direction_value(direction: DiodeDirection) -> String {
    match direction {
        DiodeDirection::Row2col => "row2col".to_owned(),
        DiodeDirection::Col2row => "col2row".to_owned(),
    }
}
