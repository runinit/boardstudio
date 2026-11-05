//! Private form for the real selected-matrix name, size, and pitch fields.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::DiodeDirection;
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
    let preset_choice = use_memo(move || Some(preset_draft().as_str().to_owned()));
    let orientation_choice = use_memo(move || Some(orientation_draft().as_str().to_owned()));
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
            header { class: "m1-matrix-inspector-heading",
                h2 { "Matrix" }
                span { "{projection.matrix_label}" }
            }
            div { class: "m1-matrix-inspector-fields",
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
                div { class: "m1-layout-link",
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
                        Button {
                            variant: ButtonVariant::Secondary,
                            disabled: !props.editable || props.busy,
                            onclick: move |_| props.on_unlink.call(request.clone()),

                            "Unlink halves"

                        }
                    }
                }
            }
            Collapsible {
                class: "m1-matrix-inspector-section",
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
                    div { class: "m1-matrix-inspector-fields",
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

                                Field { class: "m1-matrix-field",
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    Select::<String> {
                                        id: control_id,
                                        aria_label: control_aria_label,
                                        value: Some(preset_choice.into()),
                                        disabled: !props.editable || props.busy,
                                        on_value_change,
                                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                            SelectOption::<String> {
                                                index,
                                                value: value.clone(),
                                                text_value: label.clone(),
                                                disabled: *disabled,
                                                "{label}"
                                            }
                                        }
                                    }
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

                                Field { class: "m1-matrix-field",
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    Select::<String> {
                                        id: control_id,
                                        aria_label: control_aria_label,
                                        value: Some(orientation_choice.into()),
                                        disabled: !props.editable || props.busy,
                                        on_value_change,
                                        for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                            SelectOption::<String> {
                                                index,
                                                value: value.clone(),
                                                text_value: label.clone(),
                                                disabled: *disabled,
                                                "{label}"
                                            }
                                        }
                                    }
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
                class: "m1-matrix-inspector-section",
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
                    div { class: "m1-matrix-inspector-fields",
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
                class: "m1-matrix-inspector-section",
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
    view: MatrixFieldView,
    pending: bool,
    on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
fn MatrixFieldEditor(props: MatrixFieldEditorProps) -> Element {
    use_hook({
        let mut draft = props.view.draft;
        let mut failure = props.view.failure;
        let value = props.value.clone();
        let pending = props.pending;
        move || {
            if !pending && draft.peek().as_str() != value.as_str() {
                draft.set(value);
            }
            if failure.peek().is_some() {
                failure.set(None);
            }
        }
    });
    let mut draft = props.view.draft;
    let mut draft_baseline = use_signal(|| props.baseline.clone());
    let mut dirty = use_signal(|| false);
    let mut error = props.view.failure;
    let mut submitted_request_id = use_signal(|| None::<u64>);
    let mut status = use_signal(|| None::<String>);
    let accepted_value = props.value.clone();
    let accepted_baseline = props.baseline.clone();
    let pending = props.pending;
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
        (
            &accepted_value,
            &accepted_baseline,
            &matching_feedback,
            &pending,
        ),
        move |(value, baseline, feedback, pending)| {
            if pending {
                if status_for_effect.peek().as_deref() != Some("Saving…") {
                    status_for_effect.set(Some("Saving…".to_owned()));
                }
                return;
            }

            if status_for_effect.peek().as_deref() == Some("Saving…") {
                status_for_effect.set(None);
            }

            if submitted_for_effect().is_some() {
                submitted_for_effect.set(None);
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
            }

            if let Some(feedback) = feedback.as_ref() {
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
                if error_for_effect.peek().as_deref() != Some(feedback.message.as_str()) {
                    error_for_effect.set(Some(feedback.message.clone()));
                }
            }

            if draft_for_effect() == value {
                if *baseline_for_effect.peek() != baseline {
                    baseline_for_effect.set(baseline.clone());
                }
                if *dirty_for_effect.peek() {
                    dirty_for_effect.set(false);
                }
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
            }

            // A dirty draft keeps the user's value: the latest committed value wins.
            if baseline_for_effect() != baseline && !dirty_for_effect() {
                draft_for_effect.set(value.clone());
                baseline_for_effect.set(baseline.clone());
                error_for_effect.set(None);
                if status_for_effect.peek().is_some() {
                    status_for_effect.set(None);
                }
                if submitted_for_effect.peek().is_some() {
                    submitted_for_effect.set(None);
                }
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
        })
        .map(|feedback| feedback.message.clone());
    let error_text = error.read().clone().or(feedback_error);
    let error_id = format!("matrix-field-error-{}", dioxus::core::current_scope_id().0);
    let current_status = status();
    let selected_choice = use_memo(move || Some(draft()));
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
            div { class: if error_text.is_some() { "m1-matrix-field has-error" } else { "m1-matrix-field" },
                span { class: "m1-matrix-field-input",
                    if is_select {
                        {
                            let control_label = (if props.field == MatrixEditField::SwitchDefinition {
                                "Matrix part definition"
                            } else {
                                props.label
                            })
                                .to_string();
                            let control_aria_label = (if props.field == MatrixEditField::SwitchDefinition {
                                "Matrix part definition"
                            } else {
                                props.label
                            })
                                .to_string();
                            let control_id = format!(
                                "layout-choice-{}-{}",
                                dioxus::core::current_scope_id().0,
                                control_aria_label.replace(' ', "-"),
                            );

                            let mut control_choices: Vec<(String, String, bool)> = Vec::new();
                            for (value, label) in choices {
                                control_choices.push((value.to_string(), label.to_string(), false));
                            }

                            let on_value_change = EventHandler::new(move |choice: Option<String>| {
            let Some(choice) = choice else {
                return;
            };
            let value = choice;
            draft.set(value.clone());
            dirty.set(true);
            error.set(None);
            status.set(Some("Saving…".to_owned()));
            commit_choice(Some(value));
        });
        rsx! {

                                Field { invalid: error_text.is_some(),
                                    FieldLabel { html_for: control_id.clone(), "{control_label}" }
                                    if props.field == MatrixEditField::SwitchDefinition {
                                        Combobox::<String> {
                                            id: control_id.clone(),
                                            aria_label: control_aria_label.clone(),
                                            aria_invalid: error_text.is_some(),
                                            aria_describedby: error_text.as_ref().map(|_| error_id.clone()),
                                            list_aria_label: control_aria_label.clone(),
                                            value: Some(selected_choice.into()),
                                            disabled: !props.editable,
                                            on_value_change,
                                            for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                ComboboxOption::<String> {
                                                    index,
                                                    value: value.clone(),
                                                    text_value: label.clone(),
                                                    disabled: *disabled,
                                                    "{label}"
                                                }
                                            }
                                        }
                                    } else {
                                        Select::<String> {
                                            id: control_id.clone(),
                                            aria_label: control_aria_label.clone(),
                                            aria_invalid: error_text.is_some(),
                                            aria_describedby: error_text.as_ref().map(|_| error_id.clone()),
                                            value: Some(selected_choice.into()),
                                            disabled: !props.editable,
                                            on_value_change,
                                            for (index, (value, label, disabled)) in control_choices.iter().enumerate() {
                                                SelectOption::<String> {
                                                    index,
                                                    value: value.clone(),
                                                    text_value: label.clone(),
                                                    disabled: *disabled,
                                                    "{label}"
                                                }
                                            }
                                        }
                                    }
                                }

                            }
                        }
                    } else {
                        {
                            let control_input_id = format!(
                                "layout-matrix-field-{}",
                                dioxus::core::current_scope_id().0,
                            );
                            rsx! {
                                Field { invalid: error_text.is_some(),
                                    FieldLabel { html_for: control_input_id.clone(), "{props.label}" }
                                    InputGroup { class: "max-[760px]:h-control-touch",
                                        InputGroupInput { class: "max-[760px]:h-control-touch", r#type: if input_type == "number" { InputType::Number } else { InputType::Text },
    step: step,
    min: min.map(str::to_owned),
    value: "{input_value}",
    readonly: !props.editable,
    aria_label: if props.field == MatrixEditField::SwitchDefinition { "Matrix part definition" } else { props.label },
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
    id: control_input_id.clone(),
    aria_describedby: error_text.as_ref().map(|_| error_id.clone()),
    attributes: vec![dioxus::core::Attribute::new("aria-invalid", error_text.is_some(), None, false),dioxus::events::onblur({
                            let mut commit = commit.clone();
                            move |_| commit(None)
                        }),dioxus::events::onkeydown(on_keydown)], }
                                        if matches!(
                                            props.kind,
                                            MatrixFieldKind::PositiveNumber | MatrixFieldKind::NonnegativeNumber
                                        )
                                        {
                                            InputGroupAddon { align: InputGroupAddonAlign::InlineEnd,
                                                InputGroupText { "mm" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                }
                if let Some(message) = error_text.as_deref() {
                    div { id: error_id,
                        FieldError { "{message}" }
                    }
                }
                if let Some(message) = current_status.as_deref() {
                    div { role: "status",  Callout {  variant: CalloutVariant::Info,  "{message}"  } }
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
