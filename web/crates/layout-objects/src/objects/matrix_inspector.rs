//! Private form for the real selected-matrix name, size, and pitch fields.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::DiodeDirection;
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertDescription, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
use boardstudio_web_ui_shared::ui::collapsible::{
    Collapsible, CollapsibleContent, CollapsibleTrigger,
};
use boardstudio_web_ui_shared::ui::{
    combobox::{Combobox, ComboboxOption},
    field::{Field, FieldError, FieldLabel},
    select::{Select, SelectOption},
};
use boardstudio_web_ui_shared::ui::{
    input::InputType,
    input_group::{
        InputGroup, InputGroupAddon, InputGroupAddonAlign, InputGroupInput, InputGroupText,
    },
};

use boardstudio_web_ui_shared::ui::icons::ChevronDown;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;

pub use super::matrix_inspector_logic::{
    MatrixEditField, MatrixEditValue, MatrixNameTarget, MatrixPreset, SwitchOrientation,
};

impl MatrixPreset {
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
}

impl MatrixEditField {
    /// Every editable field, in the controller's draft-signal order.
    pub(crate) const ALL: [Self; 9] = [
        Self::Name,
        Self::Rows,
        Self::Columns,
        Self::PitchX,
        Self::PitchY,
        Self::SwitchDefinition,
        Self::DiodeDirection,
        Self::EdgeGapX,
        Self::EdgeGapY,
    ];

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatrixEditFeedback {
    pub owner: MatrixInspectorOwner,
    pub request_id: u64,
    pub field: MatrixEditField,
    pub message: String,
}

#[derive(Clone, Copy)]
pub struct MatrixFieldView {
    pub draft: Signal<String>,
    pub failure: Signal<Option<String>>,
}

impl PartialEq for MatrixFieldView {
    fn eq(&self, other: &Self) -> bool {
        self.draft == other.draft && self.failure == other.failure
    }
}

#[derive(Props, Clone, PartialEq)]
pub struct MatrixInspectorProps {
    pub projection: MatrixInspectorProjection,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub busy: bool,
    pub inspector_mounted: Signal<bool>,
    pub fields: Vec<(MatrixEditField, MatrixFieldView)>,
    pub pending_fields: Vec<MatrixEditField>,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
    pub on_apply_preset: EventHandler<MatrixPresetRequest>,
    pub on_delete: EventHandler<MatrixDeleteRequest>,
    pub on_unlink: EventHandler<MatrixUnlinkRequest>,
    pub on_duplicate: EventHandler<MatrixDuplicateRequest>,
}

#[component]
pub fn MatrixInspector(props: MatrixInspectorProps) -> Element {
    let mut assembly_open = use_signal(|| false);
    let mut spacing_open = use_signal(|| false);
    let mut actions_open = use_signal(|| false);

    let mut inspector_mounted = props.inspector_mounted;
    use_hook({
        let mut inspector_mounted = inspector_mounted;
        move || {
            if !*inspector_mounted.peek() {
                inspector_mounted.set(true);
            }
        }
    });
    use_drop(move || {
        if *inspector_mounted.peek() {
            inspector_mounted.set(false);
        }
    });
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
    let selected_preset = preset_draft().as_str().to_owned();
    let selected_orientation = orientation_draft().as_str().to_owned();
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
    let mut unlink_armed = use_signal(|| false);
    let unlink_layout_id = projection
        .layout_relation
        .as_ref()
        .and_then(|relation| relation.unlink_layout_id.clone());
    use_effect(use_reactive((&unlink_layout_id,), move |_| {
        unlink_armed.set(false);
    }));
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
    let field_view = |field| {
        props
            .fields
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map(|(_, view)| *view)
            .expect("Matrix Inspector fields are wired by the controller")
    };
    let field_pending = |field| props.pending_fields.contains(&field);
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
        section { class: "grid gap-3 min-w-0 px-4 pt-3 pb-4 text-foreground", aria_label: "Matrix inspector",
        onkeydown: move |event: KeyboardEvent| {
            if event.key().to_string() == " " {
                event.stop_propagation();
                if let Some(raw) = event.data().try_as_web_event() {
                    raw.stop_propagation();
                }
            }
        },
            header { class: "flex items-baseline justify-between gap-2 border-b border-border pb-2.5 [&_h2]:m-0 [&_h2]:text-[16px]! [&_h2]:font-[650] [&_span]:text-xs [&_span]:text-muted-foreground",
                h2 { "Matrix" }
                span { "{projection.matrix_label}" }
            }
            div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                {
                    rsx! {
                        MatrixFieldEditor {
                            key: "{name_key}",
                            owner: projection.owner.clone(),
                            snapshot_token: projection.snapshot_token,
                            revision: projection.revision,
                            field: MatrixEditField::Name,
                            label: projection.name_label,
                            value: projection.name_value.clone(),
                            baseline: projection.name_baseline.clone(),
                            kind: MatrixFieldKind::Name,
                            request_sequence: props.request_sequence,
                            editable: props.editable,
                            feedback: props.feedback.clone(),
                            view: field_view(MatrixEditField::Name),
                            pending: field_pending(MatrixEditField::Name),
                            on_edit: props.on_edit,
                        }
                    }
                }
                {
                    rsx! {
                        MatrixFieldEditor {
                            key: "{rows_key}",
                            owner: projection.owner.clone(),
                            snapshot_token: projection.snapshot_token,
                            revision: projection.revision,
                            field: MatrixEditField::Rows,
                            label: "Rows",
                            value: projection.rows.to_string(),
                            baseline: MatrixEditValue::Rows(projection.rows),
                            kind: MatrixFieldKind::PositiveInteger,
                            request_sequence: props.request_sequence,
                            editable: props.editable,
                            feedback: props.feedback.clone(),
                            view: field_view(MatrixEditField::Rows),
                            pending: field_pending(MatrixEditField::Rows),
                            on_edit: props.on_edit,
                        }
                    }
                }
                {
                    rsx! {
                        MatrixFieldEditor {
                            key: "{columns_key}",
                            owner: projection.owner.clone(),
                            snapshot_token: projection.snapshot_token,
                            revision: projection.revision,
                            field: MatrixEditField::Columns,
                            label: "Columns",
                            value: projection.columns.to_string(),
                            baseline: MatrixEditValue::Columns(projection.columns),
                            kind: MatrixFieldKind::PositiveInteger,
                            request_sequence: props.request_sequence,
                            editable: props.editable,
                            feedback: props.feedback.clone(),
                            view: field_view(MatrixEditField::Columns),
                            pending: field_pending(MatrixEditField::Columns),
                            on_edit: props.on_edit,
                        }
                    }
                }
                {
                    rsx! {
                        MatrixFieldEditor {
                            key: "{pitch_x_key}",
                            owner: projection.owner.clone(),
                            snapshot_token: projection.snapshot_token,
                            revision: projection.revision,
                            field: MatrixEditField::PitchX,
                            label: "Pitch X",
                            value: projection.pitch_x.to_string(),
                            baseline: MatrixEditValue::PitchX(projection.pitch_x),
                            kind: MatrixFieldKind::PositiveNumber,
                            request_sequence: props.request_sequence,
                            editable: props.editable,
                            feedback: props.feedback.clone(),
                            view: field_view(MatrixEditField::PitchX),
                            pending: field_pending(MatrixEditField::PitchX),
                            on_edit: props.on_edit,
                        }
                    }
                }
                {
                    rsx! {
                        MatrixFieldEditor {
                            key: "{pitch_y_key}",
                            owner: projection.owner.clone(),
                            snapshot_token: projection.snapshot_token,
                            revision: projection.revision,
                            field: MatrixEditField::PitchY,
                            label: "Pitch Y",
                            value: projection.pitch_y.to_string(),
                            baseline: MatrixEditValue::PitchY(projection.pitch_y),
                            kind: MatrixFieldKind::PositiveNumber,
                            request_sequence: props.request_sequence,
                            editable: props.editable,
                            feedback: props.feedback.clone(),
                            view: field_view(MatrixEditField::PitchY),
                            pending: field_pending(MatrixEditField::PitchY),
                            on_edit: props.on_edit,
                        }
                    }
                }
            }
            if let Some(relation) = projection.layout_relation.as_ref() {
                div { class: "mb-2 grid gap-2 border-b border-border pb-3 [&_p]:m-0 [&_p]:text-xs [&_p]:text-muted-foreground [&_button]:justify-self-start",
                    strong {
                        if let Some(partner_name) = relation.partner_name.as_ref() {
                            "Linked to {partner_name}"
                        } else {
                            "Independent layout"
                        }
                    }
                    if relation.partner_name.is_some() {
                        p {
                            "Key assemblies, diodes and components mirror across both halves. Replace a component on one half to keep it local."
                        }
                    } else {
                        p { "Geometry and components can be edited independently." }
                    }
                    if let Some(request) = unlink_request.clone() {
                        if unlink_armed() {
                            div { class: "grid gap-2",
                                p { class: "text-sm text-muted-foreground",
                                    "Unlinking ends the mirrored correspondence. Both halves stay independent and keep their poses in one Undo entry."
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    disabled: !props.editable || props.busy,
                                    onclick: move |_| props.on_unlink.call(request.clone()),
                                    "Confirm unlink"
                                }
                                Button {
                                    variant: ButtonVariant::Secondary,
                                    onclick: move |_| unlink_armed.set(false),
                                    "Cancel"
                                }
                            }
                        } else {
                            Button {
                                variant: ButtonVariant::Secondary,
                                disabled: !props.editable || props.busy,
                                onclick: move |_| unlink_armed.set(true),
                                "Unlink halves"
                            }
                        }
                    }
                }
            }
            Collapsible {
                class: "grid min-w-0 gap-2.5",
                keep_mounted: true,
                open: Some(assembly_open()),
                on_open_change: move |next| assembly_open.set(next),
                CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold",
                    span { "Key assembly" }
                    small { "{definition_label}" }
                 ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                CollapsibleContent {
                    class: "data-[open=true]:overflow-visible",
                    hidden: !assembly_open(),
                    inert: (!assembly_open()).then_some(""),
                    div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                        {
                            let control_label = "Assembly preset".to_owned();
                            let control_aria_label = "Apply matrix preset".to_owned();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                            for preset in MatrixPreset::ALL {
                                control_choices
                                    .push((
                                        preset.as_str().to_string(),
                                        preset.label().to_string(),
                                        false,
                                    ));
                            }
                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        if let Some(preset) = MatrixPreset::parse(&choice) {
            preset_draft.set(preset);
        }
    });
    rsx! {
                                InspectorChoiceRow {
                                    id: control_id,
                                    label: control_label,
                                    aria_label: control_aria_label,
                                    field_class: "grid min-w-0 content-start gap-[5px] text-xs text-muted-foreground".to_owned(),
                                    choices: control_choices,
                                    selected: selected_preset,
                                    disabled: !props.editable || props.busy,
                                    on_value_change,
                                }
                            }
                        }
                        {
                            let control_label = "Switch orientation".to_owned();
                            let control_aria_label = "Switch orientation".to_owned();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let control_choices = [
                                ("south".to_owned(), "South-facing LED".to_owned(), false),
                                ("north".to_owned(), "North-facing LED".to_owned(), false),
                            ];
                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        orientation_draft.set(if choice == "north" {
            SwitchOrientation::North
        } else {
            SwitchOrientation::South
        });
    });
    rsx! {
                                InspectorChoiceRow {
                                    id: control_id,
                                    label: control_label,
                                    aria_label: control_aria_label,
                                    field_class: "grid min-w-0 content-start gap-[5px] text-xs text-muted-foreground".to_owned(),
                                    choices: control_choices.into(),
                                    selected: selected_orientation,
                                    disabled: !props.editable || props.busy,
                                    on_value_change,
                                }
                            }
                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: !props.editable || props.busy,
                            onclick: move |_| apply_preset(),

                            "Update assembly preset"

                        }
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: props.busy,
                            onclick: move |_| props.on_duplicate.call(duplicate_base.clone()),

                            "Duplicate design as variant"

                        }
                        {
                            rsx! {
                                {
                                    rsx! {
                                        MatrixFieldEditor {
                                            key: "{switch_key}",
                                            owner: projection.owner.clone(),
                                            snapshot_token: projection.snapshot_token,
                                            revision: projection.revision,
                                            field: MatrixEditField::SwitchDefinition,
                                            label: "Switch footprint",
                                            value: projection.definition_id.clone(),
                                            baseline: MatrixEditValue::SwitchDefinition(projection.definition_id.clone()),
                                            kind: MatrixFieldKind::Choice,
                                            choices: projection.switch_choices.clone(),
                                            request_sequence: props.request_sequence,
                                            editable: props.editable,
                                            feedback: props.feedback.clone(),
                                            view: field_view(MatrixEditField::SwitchDefinition),
                                            pending: field_pending(MatrixEditField::SwitchDefinition),
                                            on_edit: props.on_edit,
                                        }
                                    }
                                }
                            }
                        }
                        {
                            rsx! {
                                {
                                    rsx! {
                                        MatrixFieldEditor {
                                            key: "{diode_key}",
                                            owner: projection.owner.clone(),
                                            snapshot_token: projection.snapshot_token,
                                            revision: projection.revision,
                                            field: MatrixEditField::DiodeDirection,
                                            label: "Diode direction",
                                            value: diode_direction_value(projection.diode_direction),
                                            baseline: MatrixEditValue::DiodeDirection(projection.diode_direction),
                                            kind: MatrixFieldKind::Choice,
                                            choices: vec![
                                                ("row2col".to_owned(), "Rows to columns".to_owned()),
                                                ("col2row".to_owned(), "Columns to rows".to_owned()),
                                            ],
                                            request_sequence: props.request_sequence,
                                            editable: props.editable,
                                            feedback: props.feedback.clone(),
                                            view: field_view(MatrixEditField::DiodeDirection),
                                            pending: field_pending(MatrixEditField::DiodeDirection),
                                            on_edit: props.on_edit,
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(feedback) = preset_feedback {
                            Alert { variant: AlertVariant::Destructive,AlertDescription {  "{feedback.message}"  } }
                        }
                    }
                }
            }
            Collapsible {
                class: "grid min-w-0 gap-2.5",
                keep_mounted: true,
                open: Some(spacing_open()),
                on_open_change: move |next| spacing_open.set(next),
                CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold",
                    span { "Keycap spacing" }
                    small { "Preview only" }
                 ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                CollapsibleContent {
                    hidden: !spacing_open(),
                    inert: (!spacing_open()).then_some(""),
                    div { class: "grid min-w-0 grid-cols-1 gap-x-2.5 gap-y-3",
                        {
                            rsx! {
                                {
                                    rsx! {
                                        MatrixFieldEditor {
                                            key: "{edge_gap_x_key}",
                                            owner: projection.owner.clone(),
                                            snapshot_token: projection.snapshot_token,
                                            revision: projection.revision,
                                            field: MatrixEditField::EdgeGapX,
                                            label: "Edge gap X",
                                            value: projection.edge_gap_x.to_string(),
                                            baseline: MatrixEditValue::EdgeGapX(projection.edge_gap_x),
                                            kind: MatrixFieldKind::NonnegativeNumber,
                                            choices: Vec::new(),
                                            request_sequence: props.request_sequence,
                                            editable: props.editable,
                                            feedback: props.feedback.clone(),
                                            view: field_view(MatrixEditField::EdgeGapX),
                                            pending: field_pending(MatrixEditField::EdgeGapX),
                                            on_edit: props.on_edit,
                                        }
                                    }
                                }
                            }
                        }
                        {
                            rsx! {
                                {
                                    rsx! {
                                        MatrixFieldEditor {
                                            key: "{edge_gap_y_key}",
                                            owner: projection.owner.clone(),
                                            snapshot_token: projection.snapshot_token,
                                            revision: projection.revision,
                                            field: MatrixEditField::EdgeGapY,
                                            label: "Edge gap Y",
                                            value: projection.edge_gap_y.to_string(),
                                            baseline: MatrixEditValue::EdgeGapY(projection.edge_gap_y),
                                            kind: MatrixFieldKind::NonnegativeNumber,
                                            choices: Vec::new(),
                                            request_sequence: props.request_sequence,
                                            editable: props.editable,
                                            feedback: props.feedback.clone(),
                                            view: field_view(MatrixEditField::EdgeGapY),
                                            pending: field_pending(MatrixEditField::EdgeGapY),
                                            on_edit: props.on_edit,
                                        }
                                    }
                                }
                            }
                        }
                    }
                    p { class: "m-0 text-muted-foreground text-sm",
                        "Keycap preview {(projection.pitch_x - projection.edge_gap_x).max(0.0):.1} × {(projection.pitch_y - projection.edge_gap_y).max(0.0):.1} mm"
                    }
                }
            }
            Collapsible {
                class: "grid min-w-0 gap-2.5",
                keep_mounted: true,
                open: Some(actions_open()),
                on_open_change: move |next| actions_open.set(next),
                CollapsibleTrigger { class: "group min-h-control-touch w-full justify-start px-3 py-2 text-left text-sm font-semibold", "Matrix actions"  ChevronDown { class: "ml-auto size-4 shrink-0 transition-transform group-data-[state=open]:rotate-180" } }
                CollapsibleContent {
                    hidden: !actions_open(),
                    inert: (!actions_open()).then_some(""),
                    Button {
                        variant: ButtonVariant::Secondary,
                        class: "m-0 text-muted-foreground text-sm",
                        disabled: !props.editable || props.busy,
                        onclick: move |_| props.on_delete.call(delete_request.clone()),

                        "Delete matrix"

                    }
                }
            }
            if props.busy {
                div { role: "status",  Callout { class: "m-0 text-muted-foreground text-sm", variant: CalloutVariant::Info,  "Saving matrix change…"  } }
            }
        }
    }
}

mod field_editor;
mod fields;

use field_editor::{MatrixFieldEditor, MatrixFieldKind};
use fields::InspectorChoiceRow;

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
