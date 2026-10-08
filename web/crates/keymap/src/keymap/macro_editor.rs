//! Private read-only projection and typed view for saved Keymap macros.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{KeymapMacro, MacroChange, MacroStep, ProjectDoc};
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub struct MacroReadSource {
    document: Arc<ProjectDoc>,
    pub token: SnapshotToken,
    pub revision: u64,
}

impl MacroReadSource {
    pub fn new(document: Arc<ProjectDoc>, token: SnapshotToken, revision: u64) -> Self {
        Self {
            document,
            token,
            revision,
        }
    }

    pub fn macros(&self) -> &[KeymapMacro] {
        self.document
            .keymap
            .as_ref()
            .map_or(&[], |keymap| keymap.macros.as_slice())
    }
}

impl PartialEq for MacroReadSource {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.document, &other.document)
            && self.token == other.token
            && self.revision == other.revision
    }
}

#[derive(Clone, Debug)]
pub struct MacroStepSequence {
    pub macro_id: Rc<str>,
    pub steps: Rc<[MacroStep]>,
}

impl PartialEq for MacroStepSequence {
    fn eq(&self, other: &Self) -> bool {
        self.macro_id == other.macro_id && Rc::ptr_eq(&self.steps, &other.steps)
    }
}

pub use crate::macro_edit::{MacroEditChange, MacroEditRequest, MacroEditTarget};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MacroEditStatus {
    Pending,
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MacroEditFeedback {
    pub scope: Scope,
    pub scope_generation: u64,
    pub admission_token: SnapshotToken,
    pub admission_revision: u64,
    pub editor_instance_id: u64,
    pub request_id: u64,
    pub macro_id: Option<String>,
    pub target: MacroEditTarget,
    pub status: MacroEditStatus,
}

#[derive(Props, Clone, PartialEq)]
pub struct MacroEditorProps {
    pub scope: Scope,
    pub scope_generation: u64,
    pub source: MacroReadSource,
    pub sequences: Rc<[MacroStepSequence]>,
    pub editor_instance_id: u64,
    pub request_sequence: Signal<u64>,
    pub enabled: bool,
    pub feedback: Option<MacroEditFeedback>,
    pub on_change: EventHandler<MacroEditRequest>,
}

fn sequence_for<'a>(
    sequences: &'a [MacroStepSequence],
    macro_id: &str,
) -> Option<&'a Rc<[MacroStep]>> {
    sequences
        .iter()
        .find(|sequence| sequence.macro_id.as_ref() == macro_id)
        .map(|sequence| &sequence.steps)
}

#[derive(Clone)]
struct MacroRequestContext {
    scope: Scope,
    scope_generation: u64,
    admission_token: SnapshotToken,
    admission_revision: u64,
    editor_instance_id: u64,
    request_sequence: Signal<u64>,
    enabled: bool,
    on_change: EventHandler<MacroEditRequest>,
}

impl From<&MacroEditorProps> for MacroRequestContext {
    fn from(props: &MacroEditorProps) -> Self {
        Self {
            scope: props.scope.clone(),
            scope_generation: props.scope_generation,
            admission_token: props.source.token,
            admission_revision: props.source.revision,
            editor_instance_id: props.editor_instance_id,
            request_sequence: props.request_sequence,
            enabled: props.enabled,
            on_change: props.on_change,
        }
    }
}

fn request(
    context: &MacroRequestContext,
    target: MacroEditTarget,
    macro_id: Option<String>,
    step_sequence: Option<Rc<[MacroStep]>>,
    change: MacroEditChange,
) {
    if !context.enabled {
        return;
    }
    let mut sequence = context.request_sequence;
    let Some(request_id) = sequence().checked_add(1) else {
        return;
    };
    sequence.set(request_id);
    context.on_change.call(MacroEditRequest {
        scope: context.scope.clone(),
        scope_generation: context.scope_generation,
        admission_token: context.admission_token,
        admission_revision: context.admission_revision,
        editor_instance_id: context.editor_instance_id,
        request_id,
        macro_id,
        target,
        step_sequence,
        change,
    });
}

/// Displays accepted macros and emits one typed, target-correlated intent at a time.
#[component]
pub fn MacroEditor(props: MacroEditorProps) -> Element {
    let macros = props.source.macros();
    let busy = !props.enabled;
    let editor_id = props.editor_instance_id;
    let feedback = props.feedback.as_ref().filter(|feedback| {
        feedback.scope == props.scope
            && feedback.scope_generation == props.scope_generation
            && feedback.editor_instance_id == editor_id
            && feedback.macro_id.as_ref().is_none_or(|id| {
                macros.iter().any(|item| &item.id == id)
                    || matches!(
                        feedback.target,
                        MacroEditTarget::AddMacro | MacroEditTarget::RemoveMacro
                    )
            })
    });
    let feedback_text = feedback.and_then(|feedback| match &feedback.status {
        MacroEditStatus::Pending => None,
        MacroEditStatus::Failed(message) => Some(message.clone()),
    });

    let add_context = MacroRequestContext::from(&props);
    let macro_count = macros.len();
    rsx! {
        section { class: "m1-keymap-macros",
            h3 { "Macros" }
            if let Some(message) = feedback_text {
                p { class: "m1-keymap-macro-status", role: "status", "{message}" }
            }
            if macros.is_empty() {
                p { class: "m1-keymap-empty", "Build a sequence of key taps, presses, releases and delays, then assign it to a key." }
            }
            for item in macros.iter() {
                if let Some(sequence) = sequence_for(&props.sequences, &item.id).cloned() {
                    MacroCard {
                        key: "{item.id}",
                        scope: props.scope.clone(),
                        scope_generation: props.scope_generation,
                        editor_instance_id: editor_id,
                        admission_token: props.source.token, admission_revision: props.source.revision,
                        request_sequence: props.request_sequence,
                        enabled: props.enabled,
                        source: props.source.clone(),
                        macro_id: item.id.clone(),
                        sequence,
                        on_change: props.on_change,
                    }
                }
            }
            button {
                r#type: "button",
                disabled: busy || macros.len() >= 128 || super::macro_controller::action_pending(None, MacroEditTarget::AddMacro),
                onclick: move |_| {
                    if macro_count >= 128 { return; }
                    // Entity identity is allocated by the controller; this typed request
                    // contains only the user's Add intent.
                    request(&add_context, MacroEditTarget::AddMacro, None, None, MacroEditChange::Add {
                        name: format!("Macro {}", macro_count + 1), tap_ms: 30, wait_ms: 0,
                        steps: vec![tap_a()],
                    });
                },
                "Add macro"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MacroCardProps {
    scope: Scope,
    scope_generation: u64,
    editor_instance_id: u64,
    admission_token: SnapshotToken,
    admission_revision: u64,
    request_sequence: Signal<u64>,
    enabled: bool,
    source: MacroReadSource,
    macro_id: String,
    sequence: Rc<[MacroStep]>,
    on_change: EventHandler<MacroEditRequest>,
}

#[component]
fn MacroCard(props: MacroCardProps) -> Element {
    let Some(value) = props
        .source
        .macros()
        .iter()
        .find(|item| item.id == props.macro_id)
    else {
        return rsx! {};
    };
    let macro_id = value.id.clone();
    let sequence = props.sequence.clone();
    let request_context = MacroRequestContext {
        scope: props.scope.clone(),
        scope_generation: props.scope_generation,
        admission_token: props.admission_token,
        admission_revision: props.admission_revision,
        editor_instance_id: props.editor_instance_id,
        request_sequence: props.request_sequence,
        enabled: props.enabled,
        on_change: props.on_change,
    };
    let name_change = {
        let context = request_context.clone();
        let macro_id = macro_id.clone();
        move |new_value: String| {
            request(
                &context,
                MacroEditTarget::Name,
                Some(macro_id.clone()),
                None,
                MacroEditChange::Change(MacroChange::Name { value: new_value }),
            )
        }
    };
    let tap_change = {
        let context = request_context.clone();
        let macro_id = macro_id.clone();
        move |new_value: u32| {
            request(
                &context,
                MacroEditTarget::TapMs,
                Some(macro_id.clone()),
                None,
                MacroEditChange::Change(MacroChange::TapMs { value: new_value }),
            )
        }
    };
    let wait_change = {
        let context = request_context.clone();
        let macro_id = macro_id.clone();
        move |new_value: u32| {
            request(
                &context,
                MacroEditTarget::WaitMs,
                Some(macro_id.clone()),
                None,
                MacroEditChange::Change(MacroChange::WaitMs { value: new_value }),
            )
        }
    };
    let can_add_step = value.steps.len() < 128;
    let add_context = request_context.clone();
    let remove_context = request_context.clone();
    let add_macro_id = macro_id.clone();
    let remove_macro_id = macro_id.clone();
    let add_sequence = sequence.clone();
    rsx! {
        fieldset { class: "m1-keymap-macro",
            legend { "{value.name}" }
            TextDraft {
                label: "Name", aria_label: format!("Macro name {}", value.name),
                macro_id: macro_id.clone(), target: MacroEditTarget::Name,
                accepted: value.name.clone(), max_length: Some(32), identity: format!("{:?}:{}:{}:name:{}", props.scope, props.editor_instance_id, value.id, value.name),
                enabled: props.enabled, feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::Name), on_commit: EventHandler::new(name_change),
            }
            NumberDraft {
                label: "Tap duration (ms)", aria_label: format!("{} tapMs", value.name),
                macro_id: macro_id.clone(), target: MacroEditTarget::TapMs,
                accepted: value.tap_ms, identity: format!("{:?}:{}:{}:tap:{}", props.scope, props.editor_instance_id, value.id, value.tap_ms),
                enabled: props.enabled, feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::TapMs), on_commit: EventHandler::new(tap_change),
            }
            NumberDraft {
                label: "Between actions (ms)", aria_label: format!("{} waitMs", value.name),
                macro_id: macro_id.clone(), target: MacroEditTarget::WaitMs,
                accepted: value.wait_ms, identity: format!("{:?}:{}:{}:wait:{}", props.scope, props.editor_instance_id, value.id, value.wait_ms),
                enabled: props.enabled, feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::WaitMs), on_commit: EventHandler::new(wait_change),
            }
            for (index, _) in value.steps.iter().enumerate() {
                {
                    let props = props.clone();
                    let macro_id = macro_id.clone();
                    let sequence = sequence.clone();
                    rsx! { StepEditor {
                        key: "{index}", index: index,
                        scope: props.scope.clone(), scope_generation: props.scope_generation,
                        admission_token: props.admission_token, admission_revision: props.admission_revision,
                        editor_instance_id: props.editor_instance_id,
                        request_sequence: props.request_sequence, enabled: props.enabled,
                        macro_id: macro_id.clone(), macro_name: value.name.clone(), sequence: sequence.clone(),
                        on_change: props.on_change,
                    } }
                }
            }
            button {
                r#type: "button", disabled: !props.enabled || !can_add_step || super::macro_controller::action_pending(Some(&props.macro_id), MacroEditTarget::AddStep),
                onclick: move |_| {
                    if !can_add_step { return; }
                    request(&add_context, MacroEditTarget::AddStep, Some(add_macro_id.clone()), Some(add_sequence.clone()),
                        MacroEditChange::Change(MacroChange::AddStep { value: tap_a() }));
                }, "Add step"
            }
            button {
                r#type: "button", disabled: !props.enabled || super::macro_controller::action_pending(Some(&props.macro_id), MacroEditTarget::RemoveMacro),
                onclick: move |_| request(&remove_context, MacroEditTarget::RemoveMacro,
                    Some(remove_macro_id.clone()), None, MacroEditChange::Remove),
                "Remove macro"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct StepEditorProps {
    index: usize,
    scope: Scope,
    scope_generation: u64,
    admission_token: SnapshotToken,
    admission_revision: u64,
    editor_instance_id: u64,
    request_sequence: Signal<u64>,
    enabled: bool,
    macro_id: String,
    macro_name: String,
    sequence: Rc<[MacroStep]>,
    on_change: EventHandler<MacroEditRequest>,
}

#[component]
fn StepEditor(props: StepEditorProps) -> Element {
    let index = props.index;
    let sequence = props.sequence.clone();
    let macro_id = props.macro_id.clone();
    let request_context = MacroRequestContext {
        scope: props.scope.clone(),
        scope_generation: props.scope_generation,
        admission_token: props.admission_token,
        admission_revision: props.admission_revision,
        editor_instance_id: props.editor_instance_id,
        request_sequence: props.request_sequence,
        enabled: props.enabled,
        on_change: props.on_change,
    };
    let sequence_len = sequence.len();
    let Some(accepted) = props.sequence.get(index) else {
        return rsx! {};
    };
    let displayed = super::macro_controller::draft_step(&props.scope, &macro_id, index, accepted);
    let accepted = &displayed;
    let accepted_step = accepted.clone();
    let kind = step_kind(accepted);
    let value = match accepted {
        MacroStep::Wait { ms } => *ms,
        _ => 0,
    };
    let keycode = match accepted {
        MacroStep::Tap { binding }
        | MacroStep::Press { binding }
        | MacroStep::Release { binding } => match binding {
            boardstudio_core::model::KeyBinding::KeyPress { keycode } => keycode.clone(),
            _ => String::new(),
        },
        MacroStep::Wait { .. } => String::new(),
    };
    let keycode_identity = keycode.clone();
    let stamp_identity = Rc::as_ptr(&sequence) as *const MacroStep as usize;
    let identity = format!(
        "{:?}:{}:{}:{}:{}",
        props.scope, props.editor_instance_id, props.macro_id, index, stamp_identity
    );
    rsx! {
        div { class: "m1-keymap-macro-step",
            label {
                "Step {index + 1}"
                select {
                    aria_label: crate::macro_accessible_names::step_kind_control_name(&props.macro_name, index),
                    value: "{kind}", disabled: !props.enabled,
                    onchange: { let context = request_context.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); move |event| {
                        let next = match event.value().as_str() {
                            "wait" => MacroStep::Wait { ms: 100 },
                            "press" => press_a(), "release" => release_a(), _ => tap_a(),
                        };
                        request(&context, MacroEditTarget::StepKind { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value: next }));
                    } },
                    option { value: "tap", "tap" }
                    option { value: "press", "press" }
                    option { value: "release", "release" }
                    option { value: "wait", "wait" }
                }
            }
            if let MacroStep::Wait { .. } = accepted {
                NumberDraft {
                    label: "Delay (ms)", aria_label: crate::macro_accessible_names::step_value_control_name(true).to_owned(),
                    macro_id: macro_id.clone(), target: MacroEditTarget::StepDelay { index },
                    accepted: value, identity: format!("{}:delay:{}", identity, value), enabled: props.enabled,
                    feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::StepDelay { index }),
                    on_commit: { let context = request_context.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); EventHandler::new(move |ms| {
                        request(&context, MacroEditTarget::StepDelay { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value: MacroStep::Wait { ms } }));
                    }) },
                }
            } else {
                TextDraft {
                    label: "Keycode", aria_label: crate::macro_accessible_names::step_value_control_name(false).to_owned(),
                    macro_id: macro_id.clone(), target: MacroEditTarget::StepKeycode { index },
                    accepted: keycode, identity: format!("{}:keycode:{}", identity, keycode_identity), enabled: props.enabled,
                    feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::StepKeycode { index }),
                    trim_on_commit: true,
                    // React skips an unchanged valid keycode, but blurring an
                    // unsupported legacy binding normalizes it to a key press.
                    commit_if_unchanged: step_keycode_requires_normalization(&accepted_step),
                    on_commit: { let context = request_context.clone(); let accepted = accepted_step.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); EventHandler::new(move |keycode: String| {
                        let value = match accepted {
                            MacroStep::Tap { .. } => MacroStep::Tap { binding: boardstudio_core::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Press { .. } => MacroStep::Press { binding: boardstudio_core::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Release { .. } => MacroStep::Release { binding: boardstudio_core::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Wait { .. } => return,
                        };
                        request(&context, MacroEditTarget::StepKeycode { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value }));
                    }) },
                }
            }
            button {
                r#type: "button", disabled: !props.enabled || props.sequence.len() <= 1 || super::macro_controller::action_pending(Some(&props.macro_id), MacroEditTarget::RemoveStep { index }),
                onclick: { let context = request_context.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); move |_| {
                    if sequence_len <= 1 { return; }
                    request(&context, MacroEditTarget::RemoveStep { index }, Some(macro_id.clone()), Some(sequence.clone()),
                        MacroEditChange::Change(MacroChange::RemoveStep { index }));
                } }, "Remove step {index + 1}"
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct TextDraftProps {
    label: String,
    aria_label: String,
    accepted: String,
    identity: String,
    macro_id: String,
    target: MacroEditTarget,
    enabled: bool,
    #[props(default = false)]
    trim_on_commit: bool,
    #[props(default = false)]
    commit_if_unchanged: bool,
    #[props(default)]
    max_length: Option<u32>,
    #[props(default)]
    feedback: Option<MacroEditFeedback>,
    on_commit: EventHandler<String>,
}

#[component]
fn TextDraft(props: TextDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.clone());
    let mut dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    super::macro_controller::use_bound_macro_field(&props.macro_id, props.target, draft, failure);
    let accepted_for_effect = props.accepted.clone();
    use_effect(use_reactive(
        (&props.identity, &props.accepted, &props.feedback),
        {
            let mut draft = draft;
            move |(_, _, feedback)| {
                if !dirty()
                    && !feedback
                        .as_ref()
                        .is_some_and(|entry| matches!(entry.status, MacroEditStatus::Pending))
                {
                    draft.set(accepted_for_effect.clone());
                }
            }
        },
    ));
    let on_commit = props.on_commit;
    let trim = props.trim_on_commit;
    let commit_if_unchanged = props.commit_if_unchanged;
    rsx! {
        label { "{props.label}"
            input {
                aria_label: "{props.aria_label}", maxlength: props.max_length, value: "{draft}", disabled: !props.enabled,
                oninput: move |event| { failure.set(None); draft.set(event.value()); dirty.set(true); },
                onblur: move |_| {
                    let raw = draft();
                    let value = if trim { raw.trim().to_owned() } else { raw };
                    if dirty() || commit_if_unchanged { dirty.set(false); draft.set(value.clone()); on_commit.call(value); }
                }
            }
            if !dirty() {
                if let Some(message) = failure() {
                    small { role: "alert", "{message}" }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct NumberDraftProps {
    label: String,
    aria_label: String,
    accepted: u32,
    identity: String,
    macro_id: String,
    target: MacroEditTarget,
    enabled: bool,
    #[props(default)]
    feedback: Option<MacroEditFeedback>,
    on_commit: EventHandler<u32>,
}

#[component]
fn NumberDraft(props: NumberDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.to_string());
    let mut dirty = use_signal(|| false);
    let mut failure = use_signal(|| None::<String>);
    super::macro_controller::use_bound_macro_field(&props.macro_id, props.target, draft, failure);
    let accepted_for_effect = props.accepted;
    use_effect(use_reactive(
        (&props.identity, &props.accepted, &props.feedback),
        {
            let mut draft = draft;
            move |(_, _, feedback)| {
                if !dirty()
                    && !feedback
                        .as_ref()
                        .is_some_and(|entry| matches!(entry.status, MacroEditStatus::Pending))
                {
                    draft.set(accepted_for_effect.to_string());
                }
            }
        },
    ));
    let on_commit = props.on_commit;
    rsx! {
        label { "{props.label}"
            input {
                aria_label: "{props.aria_label}", r#type: "number", min: "0", max: "10000",
                value: "{draft}", disabled: !props.enabled,
                oninput: move |event| { failure.set(None); draft.set(event.value()); dirty.set(true); },
                onblur: move |_| {
                    let raw = draft();
                    let parsed = if raw.is_empty() { Some(0) } else { raw.parse::<u32>().ok() };
                    if let Some(value) = parsed && dirty() { dirty.set(false); draft.set(value.to_string()); on_commit.call(value); }
                }
            }
            if !dirty() {
                if let Some(message) = failure() {
                    small { role: "alert", "{message}" }
                }
            }
        }
    }
}

fn step_kind(step: &MacroStep) -> &'static str {
    match step {
        MacroStep::Tap { .. } => "tap",
        MacroStep::Press { .. } => "press",
        MacroStep::Release { .. } => "release",
        MacroStep::Wait { .. } => "wait",
    }
}

fn step_keycode_requires_normalization(step: &MacroStep) -> bool {
    !matches!(
        step,
        MacroStep::Tap {
            binding: boardstudio_core::model::KeyBinding::KeyPress { .. }
        } | MacroStep::Press {
            binding: boardstudio_core::model::KeyBinding::KeyPress { .. }
        } | MacroStep::Release {
            binding: boardstudio_core::model::KeyBinding::KeyPress { .. }
        }
    )
}

fn tap_a() -> MacroStep {
    MacroStep::Tap {
        binding: boardstudio_core::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}
fn press_a() -> MacroStep {
    MacroStep::Press {
        binding: boardstudio_core::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}
fn release_a() -> MacroStep {
    MacroStep::Release {
        binding: boardstudio_core::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_accessible_name_tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_core::model::{KeyBinding, KeymapConfiguration, KeymapLayer, KeymapMacro};
    use std::cell::RefCell;
    use wasm_bindgen::{JsCast, JsValue};
    use wasm_bindgen_test::*;
    use web_sys::{Element as DomElement, HtmlInputElement};

    wasm_bindgen_test_configure!(run_in_browser);

    #[derive(Clone)]
    struct Probe {
        requests: Rc<RefCell<Vec<MacroEditRequest>>>,
    }

    #[component]
    fn fixture() -> Element {
        let probe = use_context::<Probe>();
        let steps: Rc<[MacroStep]> = Rc::from(vec![
            MacroStep::Tap {
                binding: KeyBinding::KeyPress {
                    keycode: "A".into(),
                },
            },
            MacroStep::Wait { ms: 100 },
        ]);
        let mut document = ProjectDoc::empty("doc", "Fixture");
        document.keymap = Some(KeymapConfiguration {
            layers: vec![KeymapLayer {
                id: "base".into(),
                name: "Base".into(),
                bindings: Default::default(),
                sensors: Default::default(),
            }],
            macros: vec![KeymapMacro {
                id: "keymap-macro-11-131081".into(),
                name: "Macro 1".into(),
                steps: steps.to_vec(),
                tap_ms: 30,
                wait_ms: 0,
            }],
        });
        let source = MacroReadSource::new(Arc::new(document), SnapshotToken(1), 0);
        let sequences: Rc<[MacroStepSequence]> = Rc::from(vec![MacroStepSequence {
            macro_id: Rc::from("keymap-macro-11-131081"),
            steps,
        }]);
        let scope = Scope {
            session_epoch: SessionEpoch(1),
            document_id: "doc".into(),
            board_id: "board".into(),
            instance_id: None,
        };
        let request_sequence = use_signal(|| 0);
        rsx! {
            style { {include_str!("../../../../assets/m1.css")} }
            MacroEditor {
                scope,
                scope_generation: 1,
                source,
                sequences,
                editor_instance_id: 1,
                request_sequence,
                enabled: true,
                feedback: None,
                on_change: EventHandler::new(move |request| {
                    probe.requests.borrow_mut().push(request);
                }),
            }
        }
    }

    #[wasm_bindgen_test]
    async fn mounted_macro_editor_uses_display_and_visible_field_names() {
        let (root, _) = mount();
        settle().await;

        assert!(element("select[aria-label='Macro 1 step 1']").is_ok());
        assert!(element("select[aria-label='Macro 1 step 2']").is_ok());
        assert!(element("input[aria-label='Keycode']").is_ok());
        assert!(element("input[aria-label='Delay (ms)']").is_ok());
        assert!(element("[aria-label^='keymap-macro-11-131081']").is_err());

        root.remove();
    }

    #[wasm_bindgen_test]
    async fn unchanged_valid_step_keycode_blur_does_not_submit_a_macro_edit() {
        let (root, requests) = mount();
        settle().await;

        let input: HtmlInputElement = root
            .query_selector("input[aria-label='Keycode']")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();
        input.focus().unwrap();
        input.blur().unwrap();
        settle().await;

        assert!(requests.borrow().is_empty());
        root.remove();
    }

    fn mount() -> (DomElement, Rc<RefCell<Vec<MacroEditRequest>>>) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("keymap-macro-label-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let virtual_dom = VirtualDom::new(fixture);
        virtual_dom.provide_root_context(Probe {
            requests: requests.clone(),
        });
        dioxus_web::launch::launch_virtual_dom(
            virtual_dom,
            dioxus_web::Config::new().rootnode(root.clone().into()),
        );
        (root, requests)
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }

    fn element(selector: &str) -> Result<DomElement, JsValue> {
        web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .query_selector(&format!("#keymap-macro-label-test-root {selector}"))?
            .ok_or_else(|| JsValue::from_str("expected mounted macro editor control"))
    }
}
