//! Matrix-level section of the transform Inspector: origin, rotation, mirror, anchor.
use super::*;

#[derive(Props, Clone, PartialEq)]
pub(super) struct MirrorTransformFieldProps {
    pub owner: MatrixTransformInspectorOwner,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub value: Option<Mirror>,
    pub mirror_y_locked: bool,
    pub request_sequence: Signal<u64>,
    pub editable: bool,
    pub feedback: Vec<MatrixTransformFeedback>,
    pub on_edit: EventHandler<MatrixTransformRequest>,
    pub splay_affect: Signal<MatrixSplayAffect>,
}

#[component]
pub(super) fn MirrorTransformField(props: MirrorTransformFieldProps) -> Element {
    let mut submitted = use_signal(|| None::<u64>);
    let current_value = props.value;
    let observed_value = use_signal(|| current_value);
    let mut submitted_effect = submitted;
    let mut observed_effect = observed_value;
    use_effect(use_reactive((&current_value,), move |(value,)| {
        if observed_effect() != value {
            observed_effect.set(value);
            submitted_effect.set(None);
        }
    }));
    let message = props
        .feedback
        .iter()
        .rev()
        .find(|item| {
            item.owner == props.owner
                && item.field == MatrixTransformField::MatrixMirror
                && submitted() == Some(item.request_id)
        })
        .filter(|item| item.message.is_some())
        .and_then(|item| item.message.clone());
    let request = props.request_sequence;
    let on_edit = props.on_edit;
    let owner = props.owner.clone();
    let value = props.value;
    let token = props.snapshot_token;
    let revision = props.revision;
    let affect = props.splay_affect;
    rsx! {
        {
            let control_label = "Mirror".to_owned();
            let control_aria_label = "Mirror matrix".to_owned();
            let control_id = format!(
                "layout-choice-{}-{}",
                dioxus::core::current_scope_id().0,
                control_aria_label.replace(' ', "-"),
            );

            let error: Option<String> = message;
            let selected_mirror = mirror_value(value).to_owned();
            let control_choices = [
                ("none".to_owned(), "None".to_owned(), false),
                ("x".to_owned(), "X axis".to_owned(), false),
                ("y".to_owned(), "Y axis".to_owned(), props.mirror_y_locked),
            ];
            let on_value_change = EventHandler::new(move |choice: Option<String>| {
        let Some(choice) = choice else {
            return;
        };
        let next = match choice.as_str() {
            "x" => Some(Mirror::X),
            "y" => Some(Mirror::Y),
            _ => None,
        };
        if next == value || (next == Some(Mirror::Y) && props.mirror_y_locked) {
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
            field: MatrixTransformField::MatrixMirror,
            baseline: MatrixTransformValue::Mirror(value),
            value: MatrixTransformValue::Mirror(next),
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
                    selected: selected_mirror,
                    disabled: !props.editable,
                    error,
                    on_value_change,
                }
            }
        }
    }
}
