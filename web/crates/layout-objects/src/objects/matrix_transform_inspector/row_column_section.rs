//! Row and column sections of the transform Inspector: shift, bend, stagger, splay.
use super::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct OriginModeTransformFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub custom: bool,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
pub(super) fn OriginModeTransformField(props: OriginModeTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_custom = props.custom;
    let observed_custom = use_signal(|| current_custom);
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_custom;
    use_effect(use_reactive((&current_custom,), move |(custom,)| {
        if observed_effect() != custom {
            observed_effect.set(custom);
            submitted_effect.set(None);
        }
    }));
    let error = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::SplayOriginMode
                && submitted() == Some(item.request_id)
                && item.message.is_some()
        })
        .and_then(|item| item.message.clone());
    let request = props.request_sequence;
    let on_edit = props.on_edit;
    let owner = props.owner.clone();
    let custom = props.custom;
    let token = props.snapshot_token;
    let revision = props.revision;
    let affect = props.splay_affect;
    rsx! {
        {
            let control_label = "Origin".to_owned();
            let control_aria_label = "Splay origin".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let error: Option<String> = error;
            let selected_origin = if custom { "custom" } else { "base" }.to_owned();
            let control_choices = [
                ("base".to_owned(), "Column base".to_owned(), false),
                ("custom".to_owned(), "Custom point".to_owned(), false),
            ];
            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        let next = choice == "custom";
        if next == custom {
            return;
        }
        let mut request = request;
        let Some(id) = request().checked_add(1) else {
            return;
        };
        request.set(id);
        submitted.set(Some(id));
        on_edit.call(MatrixTransformRequest {
            owner: owner.clone(),
            request_id: id,
            snapshot_token: token,
            revision,
            field: MatrixTransformField::SplayOriginMode,
            baseline: MatrixTransformValue::OriginMode(custom),
            value: MatrixTransformValue::OriginMode(next),
            splay_affect: affect(),
            draft: None,
            failure: None,
            submitted_text: None,
            one_shot: false,
        });
    });
    rsx! {
                TransformChoiceRow {
                    id: control_id,
                    label: control_label,
                    aria_label: control_aria_label,
                    choices: control_choices.into(),
                    selected: selected_origin,
                    disabled: !props.editable,
                    error,
                    on_value_change,
                }
            }
        }
    }
}
