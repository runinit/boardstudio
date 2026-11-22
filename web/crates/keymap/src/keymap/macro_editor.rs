//! Private read-only projection and typed view for saved Keymap macros.
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_model::model::{KeymapMacro, MacroChange, MacroStep, ProjectDoc};
use boardstudio_web_ui_shared::inspector_fields::{NumericField, SelectField, TextField};
use boardstudio_web_ui_shared::ui::alert::{Alert, AlertVariant};
use boardstudio_web_ui_shared::ui::button::{Button, ButtonVariant};
use boardstudio_web_ui_shared::ui::callout::{Callout, CalloutVariant};
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
        MacroEditStatus::Failed(message) => Some(message.clone()),
    });

    let add_context = MacroRequestContext::from(&props);
    let macro_count = macros.len();
    rsx! {
        section { class: "grid gap-3 min-w-0 pt-3 border-t border-border",
            h3 { "Macros" }
            if let Some(message) = feedback_text {
                Alert { variant: AlertVariant::Destructive, "{message}" }
            }
            if macros.is_empty() {
                Callout { variant: CalloutVariant::Info, "Build a sequence of key taps, presses, releases and delays, then assign it to a key." }
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
            Button {
                variant: ButtonVariant::Secondary,
                class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch",
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
        fieldset { class: "grid gap-2 min-w-0 m-0 p-2.5 border border-border rounded-[4px] bg-card [&_legend]:px-1 [&_legend]:text-foreground [&_legend]:[overflow-wrap:anywhere]",
            legend { "{value.name}" }
            TextDraft {
                label: "Name", aria_label: format!("Macro name {}", value.name),

                accepted: value.name.clone(), max_length: Some(32), identity: format!("{:?}:{}:{}:{}:name", props.scope, props.scope_generation, props.editor_instance_id, value.id),
                enabled: props.enabled, feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::Name), on_commit: EventHandler::new(name_change),
            }
            NumberDraft {
                label: "Tap duration (ms)", aria_label: format!("{} tapMs", value.name),

                accepted: value.tap_ms, identity: format!("{:?}:{}:{}:{}:tap", props.scope, props.scope_generation, props.editor_instance_id, value.id),
                enabled: props.enabled, feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::TapMs), on_commit: EventHandler::new(tap_change),
            }
            NumberDraft {
                label: "Between actions (ms)", aria_label: format!("{} waitMs", value.name),

                accepted: value.wait_ms, identity: format!("{:?}:{}:{}:{}:wait", props.scope, props.scope_generation, props.editor_instance_id, value.id),
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
            Button {
                variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !props.enabled || !can_add_step,
                onclick: move |_| {
                    if !can_add_step { return; }
                    request(&add_context, MacroEditTarget::AddStep, Some(add_macro_id.clone()), Some(add_sequence.clone()),
                        MacroEditChange::Change(MacroChange::AddStep { value: tap_a() }));
                }, "Add step"
            }
            Button {
                variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !props.enabled,
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
    let accepted_step = accepted.clone();
    let kind = step_kind(accepted);
    let field_value = accepted;
    let value = match field_value {
        MacroStep::Wait { ms } => *ms,
        _ => 0,
    };
    let keycode = match field_value {
        MacroStep::Tap { binding }
        | MacroStep::Press { binding }
        | MacroStep::Release { binding } => match binding {
            boardstudio_model::model::KeyBinding::KeyPress { keycode } => keycode.clone(),
            _ => String::new(),
        },
        MacroStep::Wait { .. } => String::new(),
    };
    let identity = format!(
        "{:?}:{}:{}:{}:{}",
        props.scope, props.scope_generation, props.editor_instance_id, props.macro_id, index
    );
    let select_id = format!("macro-step-kind-{}", dioxus::core::current_scope_id().0);
    rsx! {
        div { class: "grid gap-2 min-w-0 p-2 border border-border rounded-[4px] bg-accent",
            SelectField {
                id: select_id, label: "Step {index + 1}",
                aria_label: crate::macro_accessible_names::step_kind_control_name(&props.macro_name, index),
                value: kind, disabled: !props.enabled,
                options: ["tap", "press", "release", "wait"].map(|option| (option.into(), option.into())).to_vec(),
                on_select: {
                    let context = request_context.clone();
                    let macro_id = macro_id.clone();
                    let sequence = sequence.clone();
                    move |value: String| {
                        let next = match value.as_str() {
                            "wait" => MacroStep::Wait { ms: 100 },
                            "press" => press_a(), "release" => release_a(), _ => tap_a(),
                        };
                        request(&context, MacroEditTarget::StepKind { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value: next }));
                    }
                },
            }
            if let MacroStep::Wait { .. } = accepted {
                NumberDraft {
                    label: "Delay (ms)", aria_label: crate::macro_accessible_names::step_value_control_name(true).to_owned(),

                    accepted: value, identity: format!("{identity}:delay"), enabled: props.enabled,
                    feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::StepDelay { index }),
                    on_commit: { let context = request_context.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); EventHandler::new(move |ms| {
                        request(&context, MacroEditTarget::StepDelay { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value: MacroStep::Wait { ms } }));
                    }) },
                }
            } else {
                TextDraft {
                    label: "Keycode", aria_label: crate::macro_accessible_names::step_value_control_name(false).to_owned(),

                    accepted: keycode, identity: format!("{identity}:keycode"), enabled: props.enabled,
                    feedback: super::macro_controller::field_feedback(&props.scope, &macro_id, MacroEditTarget::StepKeycode { index }),
                    trim_on_commit: true,
                    // React skips an unchanged valid keycode, but blurring an
                    // unsupported legacy binding normalizes it to a key press.
                    commit_if_unchanged: step_keycode_requires_normalization(&accepted_step),
                    on_commit: { let context = request_context.clone(); let accepted = accepted_step.clone(); let macro_id = macro_id.clone(); let sequence = sequence.clone(); EventHandler::new(move |keycode: String| {
                        let value = match accepted {
                            MacroStep::Tap { .. } => MacroStep::Tap { binding: boardstudio_model::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Press { .. } => MacroStep::Press { binding: boardstudio_model::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Release { .. } => MacroStep::Release { binding: boardstudio_model::model::KeyBinding::KeyPress { keycode } },
                            MacroStep::Wait { .. } => return,
                        };
                        request(&context, MacroEditTarget::StepKeycode { index }, Some(macro_id.clone()), Some(sequence.clone()),
                            MacroEditChange::Change(MacroChange::Step { index, value }));
                    }) },
                }
            }
            Button {
                variant: ButtonVariant::Secondary, class: "max-[960px]:min-h-control-touch [@media(pointer:coarse)]:min-h-control-touch", disabled: !props.enabled || props.sequence.len() <= 1,
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
    #[props(default)]
    feedback: Option<MacroEditFeedback>,
    on_commit: EventHandler<String>,
}

#[component]
fn TextDraft(props: TextDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.clone());
    let mut dirty = use_signal(|| false);
    let accepted = props.accepted.clone();
    use_accepted_macro_draft(&props.identity, accepted.clone(), draft, dirty);
    let failure = if dirty() {
        None
    } else {
        props
            .feedback
            .as_ref()
            .map(|feedback| match &feedback.status {
                MacroEditStatus::Failed(message) => message.clone(),
            })
    };
    let on_commit = props.on_commit;
    let trim = props.trim_on_commit;
    let commit_if_unchanged = props.commit_if_unchanged;
    let id = format!("macro-text-{}", dioxus::core::current_scope_id().0);
    rsx! {
        TextField {
            id, label: props.label, aria_label: Some(props.aria_label), value: draft(),
            disabled: !props.enabled, error: failure,
            maxlength: props.max_length.map(|max| max.to_string()),
            oninput: move |event: FormEvent| { draft.set(event.value()); dirty.set(true); },
            onblur: move |_| {
                let raw = draft();
                let value = if trim { raw.trim().to_owned() } else { raw };
                if dirty() || commit_if_unchanged { dirty.set(false); draft.set(accepted.clone()); on_commit.call(value); }
            },
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
    #[props(default)]
    feedback: Option<MacroEditFeedback>,
    on_commit: EventHandler<u32>,
}

#[component]
fn NumberDraft(props: NumberDraftProps) -> Element {
    let mut draft = use_signal(|| props.accepted.to_string());
    let mut dirty = use_signal(|| false);
    let accepted = props.accepted.to_string();
    use_accepted_macro_draft(&props.identity, accepted.clone(), draft, dirty);
    let failure = if dirty() {
        None
    } else {
        props
            .feedback
            .as_ref()
            .map(|feedback| match &feedback.status {
                MacroEditStatus::Failed(message) => message.clone(),
            })
    };
    let on_commit = props.on_commit;
    let id = format!("macro-number-{}", dioxus::core::current_scope_id().0);
    let error_id = format!("{id}-error");
    let message = failure;
    rsx! {
        NumericField {
            id, label: props.label, aria_label: props.aria_label, min: "0", max: "10000", step: None::<String>,
            class: "gap-1.5", error: message, message_id: error_id,
            value: "{draft}", disabled: !props.enabled,
            oninput: move |event: FormEvent| { draft.set(event.value()); dirty.set(true); },
            onblur: move |_| {
                let raw = draft();
                let parsed = if raw.is_empty() { Some(0) } else { raw.parse::<u32>().ok() };
                if let Some(value) = parsed && dirty() { dirty.set(false); draft.set(accepted.clone()); on_commit.call(value); }
            },
        }
    }
}

fn use_accepted_macro_draft(
    identity: &str,
    accepted: String,
    mut draft: Signal<String>,
    mut dirty: Signal<bool>,
) {
    let mut owner = use_signal(|| identity.to_owned());
    let identity = identity.to_owned();
    use_effect(use_reactive(
        (&identity, &accepted),
        move |(identity, accepted)| {
            if *owner.peek() != identity {
                owner.set(identity);
                dirty.set(false);
            }
            if !*dirty.peek() && *draft.peek() != accepted {
                draft.set(accepted);
            }
        },
    ));
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
            binding: boardstudio_model::model::KeyBinding::KeyPress { .. }
        } | MacroStep::Press {
            binding: boardstudio_model::model::KeyBinding::KeyPress { .. }
        } | MacroStep::Release {
            binding: boardstudio_model::model::KeyBinding::KeyPress { .. }
        }
    )
}

fn tap_a() -> MacroStep {
    MacroStep::Tap {
        binding: boardstudio_model::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}
fn press_a() -> MacroStep {
    MacroStep::Press {
        binding: boardstudio_model::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}
fn release_a() -> MacroStep {
    MacroStep::Release {
        binding: boardstudio_model::model::KeyBinding::KeyPress {
            keycode: "A".into(),
        },
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod mounted_accessible_name_tests {
    use super::*;
    use boardstudio_application::{Scope, SessionEpoch, SnapshotToken};
    use boardstudio_model::model::{KeyBinding, KeymapConfiguration, KeymapLayer, KeymapMacro};
    use boardstudio_web_ui_shared::test_mount::MountedFixture;
    use boardstudio_web_ui_shared::test_role::by_role_name;
    use boardstudio_web_ui_shared::test_wait::wait_until;
    use std::cell::RefCell;
    use wasm_bindgen::JsCast;
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
            style { {include_str!("../../../../assets/workbench-base.css")} }
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
        let (root, _, _mounted) = mount();
        settle().await;

        let step = by_role_name(&root, "button", "Macro 1 step 1").unwrap();
        let editor = step.closest("section").unwrap().unwrap();
        let editor_classes = editor.get_attribute("class").unwrap_or_default();
        for expected in [
            "grid",
            "gap-3",
            "min-w-0",
            "pt-3",
            "border-t",
            "border-border",
        ] {
            assert!(
                editor_classes
                    .split_whitespace()
                    .any(|class| class == expected),
                "macro editor: missing {expected}"
            );
        }
        let card = step.closest("fieldset").unwrap().unwrap();
        let classes = card.get_attribute("class").unwrap_or_default();
        for expected in [
            "grid",
            "gap-2",
            "min-w-0",
            "m-0",
            "p-2.5",
            "border",
            "border-border",
            "rounded-[4px]",
            "bg-card",
        ] {
            assert!(classes.split_whitespace().any(|class| class == expected));
        }
        let step_field = step.closest("[data-slot=field]").unwrap().unwrap();
        let step_row = step_field.parent_element().unwrap();
        let classes = step_row.get_attribute("class").unwrap_or_default();
        for expected in [
            "grid",
            "gap-2",
            "min-w-0",
            "p-2",
            "border",
            "border-border",
            "rounded-[4px]",
            "bg-accent",
        ] {
            assert!(classes.split_whitespace().any(|class| class == expected));
        }
        assert!(by_role_name(&root, "button", "Macro 1 step 2").is_some());
        assert!(by_role_name(&root, "textbox", "Keycode").is_some());
        assert!(by_role_name(&root, "spinbutton", "Delay (ms)").is_some());

        root.remove();
    }

    #[wasm_bindgen_test]
    async fn unchanged_valid_step_keycode_blur_does_not_submit_a_macro_edit() {
        let (root, requests, _mounted) = mount();
        settle().await;

        let input: HtmlInputElement = by_role_name(&root, "textbox", "Keycode")
            .unwrap()
            .dyn_into()
            .unwrap();
        input.focus().unwrap();
        input.blur().unwrap();
        settle().await;

        assert!(requests.borrow().is_empty());
        root.remove();
    }

    #[wasm_bindgen_test]
    async fn step_kind_select_supports_typeahead_selection_and_escape() {
        let (root, requests, _mounted) = mount();
        settle().await;
        let trigger = by_role_name(&root, "button", "Macro 1 step 1").unwrap();
        let key = |target: &web_sys::HtmlElement, value: &str| {
            let event = web_sys::KeyboardEventInit::new();
            event.set_key(value);
            event.set_bubbles(true);
            event.set_cancelable(true);
            target
                .dispatch_event(
                    &web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &event)
                        .unwrap(),
                )
                .unwrap();
        };
        trigger.focus().unwrap();
        key(&trigger, "Enter");
        wait_until("step kind list opens", || {
            by_role_name(&root, "listbox", "Macro 1 step 1").is_some()
        })
        .await;
        let list = by_role_name(&root, "listbox", "Macro 1 step 1").unwrap();
        key(&list, "w");
        wait_until("wait is focused by typeahead", || {
            by_role_name(&root, "option", "wait")
                .is_some_and(|option| option.matches(":focus").unwrap())
        })
        .await;
        key(&by_role_name(&root, "option", "wait").unwrap(), "Enter");
        wait_until("step kind edit is emitted", || {
            !requests.borrow().is_empty()
        })
        .await;
        assert_eq!(
            requests.borrow()[0].target,
            MacroEditTarget::StepKind { index: 0 }
        );
        wait_until("select closes and returns focus", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("false")
                && trigger.matches(":focus").unwrap()
        })
        .await;
        key(&trigger, "Enter");
        wait_until("step kind list reopens", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("true")
        })
        .await;
        key(
            &by_role_name(&root, "listbox", "Macro 1 step 1").unwrap(),
            "Escape",
        );
        wait_until("escape closes and returns focus", || {
            trigger.get_attribute("aria-expanded").as_deref() == Some("false")
                && trigger.matches(":focus").unwrap()
        })
        .await;
        root.remove();
    }

    fn mount() -> (
        DomElement,
        Rc<RefCell<Vec<MacroEditRequest>>>,
        MountedFixture,
    ) {
        let document = web_sys::window().unwrap().document().unwrap();
        let root = document.create_element("div").unwrap();
        root.set_id("keymap-macro-label-test-root");
        document.body().unwrap().append_child(&root).unwrap();
        let requests = Rc::new(RefCell::new(Vec::new()));
        let virtual_dom = VirtualDom::new(fixture);
        virtual_dom.provide_root_context(Probe {
            requests: requests.clone(),
        });
        let mounted = MountedFixture::new(root.clone(), virtual_dom);
        (root, requests, mounted)
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(40).await;
    }
}
