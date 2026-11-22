use super::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum MatrixFieldKind {
    Name,
    PositiveInteger,
    PositiveNumber,
    NonnegativeNumber,
    Choice,
}

#[derive(Props, Clone, PartialEq)]
pub(super) struct MatrixFieldEditorProps {
    pub owner: MatrixInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub field: MatrixEditField,
    pub label: &'static str,
    pub value: String,
    pub baseline: MatrixEditValue,
    pub kind: MatrixFieldKind,
    #[props(default)]
    pub choices: Vec<(String, String)>,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixEditFeedback>,
    pub on_edit: EventHandler<MatrixEditRequest>,
}

#[component]
pub(super) fn MatrixFieldEditor(props: MatrixFieldEditorProps) -> Element {
    use boardstudio_web_ui_shared::inspector_fields::{
        NumericField, SelectField, TextCommitField, format_numeric,
    };

    let mut submitted = use_signal(|| None::<u64>);
    let mut sequence = props.request_sequence;
    let baseline = props.baseline.clone();
    let field = props.field;
    let owner = props.owner.clone();
    let snapshot_token = props.snapshot_token;
    let revision = props.revision;
    let on_edit = props.on_edit;
    let editable = props.editable;
    let send = move |value: MatrixEditValue| {
        if !editable || value == baseline {
            return;
        }
        let Some(request_id) = sequence().checked_add(1) else {
            return;
        };
        sequence.set(request_id);
        submitted.set(Some(request_id));
        on_edit.call(MatrixEditRequest {
            owner: owner.clone(),
            request_id,
            snapshot_token,
            revision,
            field,
            baseline: baseline.clone(),
            value,
        });
    };
    let error = submitted().and_then(|request_id| {
        props.feedback.iter().find(|feedback| {
            feedback.owner == props.owner
                && feedback.field == field
                && feedback.request_id == request_id
        })
    });
    let label = if field == MatrixEditField::SwitchDefinition {
        "Matrix part definition"
    } else {
        props.label
    };
    let disabled_reason = (!editable).then(|| "Matrix editing is unavailable".to_owned());
    let value = props.value;
    rsx! {
        div { class: "grid min-w-0 gap-1",
            match props.kind {
                MatrixFieldKind::Name => rsx! {
                    TextCommitField {
                        label, value, disabled_reason,
                        on_commit: move |name| { let mut send = send.clone(); send(MatrixEditValue::Name(Some(name))); },
                    }
                },
                MatrixFieldKind::Choice => rsx! {
                    SelectField::<String> {
                        label, value, options: props.choices, disabled: !editable,
                        on_commit: move |choice: String| {
                            let value = match field {
                                MatrixEditField::SwitchDefinition => MatrixEditValue::SwitchDefinition(choice),
                                MatrixEditField::DiodeDirection if choice == "row2col" => MatrixEditValue::DiodeDirection(DiodeDirection::Row2col),
                                MatrixEditField::DiodeDirection if choice == "col2row" => MatrixEditValue::DiodeDirection(DiodeDirection::Col2row),
                                _ => return,
                            };
                            let mut send = send.clone();
                            send(value);
                        },
                    }
                },
                kind => {
                    let whole = kind == MatrixFieldKind::PositiveInteger;
                    let precision = if whole { 0 } else { 3 };
                    // Use the displayed precision as the input baseline. The edit still
                    // carries the original accepted value for conflict detection.
                    let value = value.parse::<f64>().unwrap_or_default();
                    let displayed = format_numeric(value, precision).parse::<f64>().unwrap_or(value);
                    rsx! {
                        NumericField {
                            label, value: displayed, precision, disabled_reason,
                            unit: (!whole).then(|| "mm".to_owned()),
                            min: Some(if whole { 1.0 } else { 0.0 }),
                            step: whole.then_some(1.0),
                            max: whole.then_some(f64::from(u32::MAX)),
                            on_commit: move |number: f64| {
                                let value = match field {
                                    MatrixEditField::Rows if number.fract() == 0.0 => MatrixEditValue::Rows(number as u32),
                                    MatrixEditField::Columns if number.fract() == 0.0 => MatrixEditValue::Columns(number as u32),
                                    MatrixEditField::PitchX if number > 0.0 => MatrixEditValue::PitchX(number),
                                    MatrixEditField::PitchY if number > 0.0 => MatrixEditValue::PitchY(number),
                                    MatrixEditField::EdgeGapX => MatrixEditValue::EdgeGapX(number),
                                    MatrixEditField::EdgeGapY => MatrixEditValue::EdgeGapY(number),
                                    _ => return,
                                };
                                let mut send = send.clone();
                                send(value);
                            },
                        }
                    }
                }
            }
            if let Some(error) = error {
                FieldError { "{error.message}" }
            }
        }
    }
}
