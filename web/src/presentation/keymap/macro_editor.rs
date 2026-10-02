//! Private read-only projection and typed view for saved Keymap macros.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_core::model::{KeymapMacro, MacroChange, MacroStep, ProjectDoc};
use dioxus::prelude::*;
use std::rc::Rc;
use std::sync::Arc;

#[derive(Clone)]
pub(in crate::presentation) struct MacroReadSource {
    document: Arc<ProjectDoc>,
    pub(in crate::presentation) token: SnapshotToken,
    pub(in crate::presentation) revision: u64,
}

impl MacroReadSource {
    pub(in crate::presentation) fn new(
        document: Arc<ProjectDoc>,
        token: SnapshotToken,
        revision: u64,
    ) -> Self {
        Self {
            document,
            token,
            revision,
        }
    }

    pub(in crate::presentation) fn macros(&self) -> &[KeymapMacro] {
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
pub(in crate::presentation) struct MacroStepSequence {
    pub(in crate::presentation) macro_id: Rc<str>,
    pub(in crate::presentation) steps: Rc<[MacroStep]>,
}

impl PartialEq for MacroStepSequence {
    fn eq(&self, other: &Self) -> bool {
        self.macro_id == other.macro_id && Rc::ptr_eq(&self.steps, &other.steps)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum MacroEditTarget {
    AddMacro,
    RemoveMacro,
    Name,
    TapMs,
    WaitMs,
    AddStep,
    RemoveStep { index: usize },
    StepKind { index: usize },
    StepDelay { index: usize },
    StepKeycode { index: usize },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::presentation) struct MacroEditRequest {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) scope_generation: u64,
    pub(in crate::presentation) admission_token: SnapshotToken,
    pub(in crate::presentation) admission_revision: u64,
    pub(in crate::presentation) editor_instance_id: u64,
    pub(in crate::presentation) request_id: u64,
    pub(in crate::presentation) macro_id: Option<String>,
    pub(in crate::presentation) target: MacroEditTarget,
    pub(in crate::presentation) step_sequence: Option<Rc<[MacroStep]>>,
    pub(in crate::presentation) change: MacroEditChange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum MacroEditChange {
    Add {
        name: String,
        tap_ms: u32,
        wait_ms: u32,
        steps: Vec<MacroStep>,
    },
    Remove,
    Change(MacroChange),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::presentation) enum MacroEditStatus {
    Pending,
    Saved,
    Failed(String),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::presentation) struct MacroEditFeedback {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) scope_generation: u64,
    pub(in crate::presentation) admission_token: SnapshotToken,
    pub(in crate::presentation) admission_revision: u64,
    pub(in crate::presentation) editor_instance_id: u64,
    pub(in crate::presentation) request_id: u64,
    pub(in crate::presentation) macro_id: Option<String>,
    pub(in crate::presentation) target: MacroEditTarget,
    pub(in crate::presentation) status: MacroEditStatus,
}

#[derive(Props, Clone, PartialEq)]
pub(in crate::presentation) struct MacroEditorProps {
    pub(in crate::presentation) scope: Scope,
    pub(in crate::presentation) scope_generation: u64,
    pub(in crate::presentation) source: MacroReadSource,
    pub(in crate::presentation) sequences: Rc<[MacroStepSequence]>,
    pub(in crate::presentation) editor_instance_id: u64,
    pub(in crate::presentation) request_sequence: Signal<u64>,
    pub(in crate::presentation) enabled: bool,
    pub(in crate::presentation) feedback: Option<MacroEditFeedback>,
    pub(in crate::presentation) on_change: EventHandler<MacroEditRequest>,
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
pub(in crate::presentation) fn MacroEditor(props: MacroEditorProps) -> Element {
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
    let feedback_text = feedback.map(|feedback| match &feedback.status {
        MacroEditStatus::Pending => "Saving macro…".to_owned(),
        MacroEditStatus::Saved => "Macro saved".to_owned(),
        MacroEditStatus::Failed(message) => message.clone(),
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
                disabled: busy || macros.len() >= 128,
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
                accepted: value.name.clone(), max_length: Some(32), identity: format!("{:?}:{}:{}:name:{}", props.scope, props.editor_instance_id, value.id, value.name),
                enabled: props.enabled, on_commit: EventHandler::new(name_change),
            }
            NumberDraft {
                label: "Tap duration (ms)", aria_label: format!("{} tapMs", value.name),
                accepted: value.tap_ms, identity: format!("{:?}:{}:{}:tap:{}", props.scope, props.editor_instance_id, value.id, value.tap_ms),
                enabled: props.enabled, on_commit: EventHandler::new(tap_change),
            }
            NumberDraft {
                label: "Between actions (ms)", aria_label: format!("{} waitMs", value.name),
                accepted: value.wait_ms, identity: format!("{:?}:{}:{}:wait:{}", props.scope, props.editor_instance_id, value.id, value.wait_ms),
                enabled: props.enabled, on_commit: EventHandler::new(wait_change),
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
                        macro_id: macro_id.clone(), sequence: sequence.clone(),
                        on_change: props.on_change,
                    } }
                }
            }
            button {
                r#type: "button", disabled: !props.enabled || !can_add_step,
                onclick: move |_| {
                    if !can_add_step { return; }
                    request(&add_context, MacroEditTarget::AddStep, Some(add_macro_id.clone()), Some(add_sequence.clone()),
                        MacroEditChange::Change(MacroChange::AddStep { value: tap_a() }));
                }, "Add step"
            }
            button {
                r#type: "button", disabled: !props.enabled,
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
                    aria_label: format!("{} step {}", props.macro_id, index + 1),
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
                    label: "Delay (ms)", aria_label: format!("{} step {} delay", props.macro_id, index + 1),
                    accepted: value, identity: format!("{}:delay:{}", identity, value), enabled: props.enabled,
                    on_commit: { let context = request_context.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); EventHandler::new(move |ms| {
                        request(&context, MacroEditTarget::StepDelay { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value: MacroStep::Wait { ms } }));
                    }) },
                }
            } else {
                TextDraft {
                    label: "Keycode", aria_label: format!("{} step {} keycode", props.macro_id, index + 1),
                    accepted: keycode, identity: format!("{}:keycode:{}", identity, keycode_identity), enabled: props.enabled,
                    trim_on_commit: true,
                    commit_if_unchanged: true,
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
                r#type: "button", disabled: !props.enabled || props.sequence.len() <= 1,
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
    enabled: bool,
    #[props(default = false)]
    trim_on_commit: bool,
    #[props(default = false)]
    commit_if_unchanged: bool,
    #[props(default)]
    max_length: Option<u32>,
    on_commit: EventHandler<String>,
}

#[component]
fn TextDraft(props: TextDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.clone());
    let accepted_for_effect = props.accepted.clone();
    use_effect(use_reactive((&props.identity, &props.accepted), {
        let mut draft = draft;
        move |_| draft.set(accepted_for_effect.clone())
    }));
    let accepted = props.accepted.clone();
    let on_commit = props.on_commit;
    let trim = props.trim_on_commit;
    let commit_if_unchanged = props.commit_if_unchanged;
    rsx! {
        label { "{props.label}"
            input {
                aria_label: "{props.aria_label}", maxlength: props.max_length, value: "{draft}", disabled: !props.enabled,
                oninput: move |event| draft.set(event.value()),
                onblur: move |_| {
                    let raw = draft();
                    let value = if trim { raw.trim().to_owned() } else { raw };
                    if value != accepted || commit_if_unchanged { on_commit.call(value); }
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
    enabled: bool,
    on_commit: EventHandler<u32>,
}

#[component]
fn NumberDraft(props: NumberDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.to_string());
    let accepted_for_effect = props.accepted;
    use_effect(use_reactive((&props.identity, &props.accepted), {
        let mut draft = draft;
        move |_| draft.set(accepted_for_effect.to_string())
    }));
    let accepted = props.accepted;
    let on_commit = props.on_commit;
    rsx! {
        label { "{props.label}"
            input {
                aria_label: "{props.aria_label}", r#type: "number", min: "0", max: "10000",
                value: "{draft}", disabled: !props.enabled,
                oninput: move |event| draft.set(event.value()),
                onblur: move |_| {
                    let raw = draft();
                    let parsed = if raw.is_empty() { Some(0) } else { raw.parse::<u32>().ok() };
                    if let Some(value) = parsed && value != accepted { on_commit.call(value); }
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
