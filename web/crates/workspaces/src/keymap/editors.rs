//! Declarative Keymap binding, encoder, and macro editors.
use crate::binding::{
    Behavior, BindingLayerChoice, BindingMacroChoice, BindingTarget, HOLD_MODIFIERS,
    KEYCODE_CHOICES, choose_behavior, with_keycode, with_layer_id,
};
use crate::encoder::EncoderEditorProjection;
use crate::macro_edit::MacroEditAction;
use boardstudio_model::model::{EncoderDirection, KeyBinding, KeymapMacro, MacroChange, MacroStep};
use boardstudio_web_ui_shared::inspector_fields::{
    InspectorSection, NumericField, SelectField, TextCommitField,
};
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertVariant},
    button::{Button, ButtonSize, ButtonVariant},
};
use dioxus::prelude::*;
use std::rc::Rc;

fn keycode_options() -> Vec<(String, String)> {
    KEYCODE_CHOICES
        .iter()
        .map(|(v, l)| ((*v).to_owned(), (*l).to_owned()))
        .collect()
}

#[component]
pub fn BindingEditor(
    label: String,
    active_layer_id: String,
    target: BindingTarget,
    value: KeyBinding,
    layers: Rc<[BindingLayerChoice]>,
    macros: Rc<[BindingMacroChoice]>,
    enabled: bool,
    on_commit: EventHandler<(String, BindingTarget, KeyBinding)>,
) -> Element {
    let behavior = Behavior::from_binding(&value);
    let behavior_opts: Vec<_> = Behavior::ALL
        .iter()
        .map(|(b, l)| (b.value().to_owned(), (*l).to_owned()))
        .collect();
    let layer_opts: Vec<_> = layers
        .iter()
        .map(|l| (l.id.clone(), l.name.clone()))
        .collect();
    let macro_opts: Vec<_> = macros
        .iter()
        .map(|m| (m.id.clone(), m.name.clone()))
        .collect();
    let mod_opts: Vec<_> = HOLD_MODIFIERS
        .iter()
        .map(|m| ((*m).to_owned(), (*m).to_owned()))
        .collect();
    let emit = {
        let (lid, tgt) = (active_layer_id.clone(), target.clone());
        move |next: KeyBinding| on_commit.call((lid.clone(), tgt.clone(), next))
    };
    let (e_beh, e_kc, e_hold, e_lay, e_mac) =
        (emit.clone(), emit.clone(), emit.clone(), emit.clone(), emit);
    let (v_kc, v_hold, v_lay) = (value.clone(), value.clone(), value.clone());
    rsx! {
        div { class: "grid min-w-0 gap-2",
            SelectField::<String> {
                label: format!("Behavior ({label})"), aria_label: format!("Behavior for {label}"),
                value: behavior.value().to_owned(), options: behavior_opts, disabled: !enabled,
                on_commit: move |v: String| if let Some(b) = Behavior::from_value(&v) { e_beh(choose_behavior(b, &layers, &macros)); },
            }
            if let Some((kc_label, kc_val)) = match &value {
                KeyBinding::KeyPress { keycode } | KeyBinding::StickyKey { keycode } => Some(("Keycode", keycode.clone())),
                KeyBinding::ModTap { tap, .. } | KeyBinding::LayerTap { tap, .. } => Some(("Tap keycode", tap.clone())),
                _ => None,
            } {
                SelectField::<String> {
                    label: kc_label.to_owned(), aria_label: format!("{kc_label} for {label}"),
                    value: kc_val, options: keycode_options(), disabled: !enabled,
                    on_commit: move |v: String| if let Some(next) = with_keycode(&v_kc, v) { e_kc(next); },
                }
            }
            if let KeyBinding::ModTap { hold, tap } = &v_hold {
                {
                    let tap = tap.clone();
                    rsx! {
                        SelectField::<String> {
                            label: "Hold modifier".to_owned(), aria_label: format!("Hold modifier for {label}"),
                            value: hold.clone(), options: mod_opts, disabled: !enabled,
                            on_commit: move |v: String| e_hold(KeyBinding::ModTap { hold: v, tap: tap.clone() }),
                        }
                    }
                }
            }
            if let Some(lid) = match &v_lay {
                KeyBinding::LayerTap { layer_id, .. }
                | KeyBinding::MomentaryLayer { layer_id }
                | KeyBinding::ToggleLayer { layer_id }
                | KeyBinding::ToLayer { layer_id }
                | KeyBinding::StickyLayer { layer_id } => Some(layer_id.clone()),
                _ => None,
            } {
                SelectField::<String> {
                    label: "Layer".to_owned(), aria_label: format!("Layer for {label}"),
                    value: lid, options: layer_opts, disabled: !enabled,
                    on_commit: move |v: String| if let Some(next) = with_layer_id(&v_lay, v) { e_lay(next); },
                }
            }
            if let KeyBinding::Macro { macro_id } = &value {
                SelectField::<String> {
                    label: "Macro".to_owned(), aria_label: format!("Macro for {label}"),
                    value: macro_id.clone(), options: macro_opts, disabled: !enabled,
                    placeholder: "Select macro",
                    on_commit: move |v: String| e_mac(KeyBinding::Macro { macro_id: v }),
                }
            }
        }
    }
}

#[component]
pub fn EncoderEditor(
    projection: Rc<EncoderEditorProjection>,
    enabled: bool,
    on_commit: EventHandler<(String, BindingTarget, KeyBinding)>,
) -> Element {
    if projection.rows.is_empty() {
        return rsx! { p { class: "text-sm text-muted-foreground", "No rotary encoders on this board." } };
    }
    rsx! {
        div { class: "grid min-w-0 gap-3",
            for row in projection.rows.iter() {
                InspectorSection { key: "{row.id}", title: row.label.to_string(), default_open: true,
                    BindingEditor {
                        label: format!("{} clockwise", row.label),
                        active_layer_id: projection.effective_layer_id.clone(),
                        target: BindingTarget::EncoderRotation { encoder_id: row.id.to_string(), direction: EncoderDirection::Clockwise },
                        value: row.clockwise.clone(),
                        layers: projection.layers.clone(),
                        macros: projection.macros.clone(),
                        enabled, on_commit,
                    }
                    BindingEditor {
                        label: format!("{} counterclockwise", row.label),
                        active_layer_id: projection.effective_layer_id.clone(),
                        target: BindingTarget::EncoderRotation { encoder_id: row.id.to_string(), direction: EncoderDirection::Counterclockwise },
                        value: row.counterclockwise.clone(),
                        layers: projection.layers.clone(),
                        macros: projection.macros.clone(),
                        enabled, on_commit,
                    }
                    if let (Some(pk), Some(pb)) = (&row.push_key_id, &row.push_binding) {
                        BindingEditor {
                            label: format!("{} push", row.label),
                            active_layer_id: projection.effective_layer_id.clone(),
                            target: BindingTarget::EncoderPush { encoder_id: row.id.to_string(), key_id: pk.to_string() },
                            value: pb.clone(),
                            layers: projection.layers.clone(),
                            macros: projection.macros.clone(),
                            enabled, on_commit,
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn MacroEditor(
    macros: Rc<[KeymapMacro]>,
    enabled: bool,
    error: Option<String>,
    on_action: EventHandler<MacroEditAction>,
) -> Element {
    let step_kinds = vec![
        ("tap".to_owned(), "Tap".to_owned()),
        ("press".to_owned(), "Press".to_owned()),
        ("release".to_owned(), "Release".to_owned()),
        ("wait".to_owned(), "Wait (ms)".to_owned()),
    ];
    rsx! {
        div { class: "grid min-w-0 gap-3",
            Button {
                variant: ButtonVariant::Secondary, disabled: !enabled || macros.len() >= 128,
                onclick: move |_| on_action.call(MacroEditAction::Add {
                    tap_ms: 40, wait_ms: 40,
                    steps: vec![MacroStep::Tap { binding: KeyBinding::KeyPress { keycode: "A".into() } }],
                }),
                "Add macro"
            }
            if let Some(msg) = error { Alert { variant: AlertVariant::Destructive, "{msg}" } }
            for item in macros.iter() {
                {
                    let (id, can_remove_step) = (item.id.clone(), item.steps.len() > 1);
                    let (id_n, id_t, id_w, id_add, id_rm) = (id.clone(), id.clone(), id.clone(), id.clone(), id.clone());
                    rsx! {
                        InspectorSection { key: "{id}", title: item.name.clone(), default_open: true,
                            TextCommitField {
                                label: "Name".to_owned(), value: item.name.clone(),
                                on_commit: move |v: String| on_action.call(MacroEditAction::Change { macro_id: id_n.clone(), change: MacroChange::Name { value: v } }),
                            }
                            div { class: "grid grid-cols-2 gap-2",
                                NumericField { label: "Tap".to_owned(), unit: "ms", value: item.tap_ms as f64, min: Some(0.0), max: Some(5000.0), step: Some(10.0), precision: 0,
                                    on_commit: move |v: f64| on_action.call(MacroEditAction::Change { macro_id: id_t.clone(), change: MacroChange::TapMs { value: v.round() as u32 } }),
                                }
                                NumericField { label: "Wait".to_owned(), unit: "ms", value: item.wait_ms as f64, min: Some(0.0), max: Some(5000.0), step: Some(10.0), precision: 0,
                                    on_commit: move |v: f64| on_action.call(MacroEditAction::Change { macro_id: id_w.clone(), change: MacroChange::WaitMs { value: v.round() as u32 } }),
                                }
                            }
                            for (idx, step) in item.steps.iter().enumerate() {
                                {
                                    let (id_k, id_v, id_d) = (id.clone(), id.clone(), id.clone());
                                    let kind = match step {
                                        MacroStep::Tap { .. } => "tap",
                                        MacroStep::Press { .. } => "press",
                                        MacroStep::Release { .. } => "release",
                                        MacroStep::Wait { .. } => "wait",
                                    };
                                    let step_clone = step.clone();
                                    rsx! {
                                        div { key: "{id}-step-{idx}", class: "grid gap-1.5 rounded border border-border p-2",
                                            SelectField::<String> {
                                                label: format!("Step {}", idx + 1), value: kind.to_owned(), options: step_kinds.clone(), disabled: !enabled,
                                                on_commit: move |k: String| {
                                                    let next = match k.as_str() {
                                                        "press" => MacroStep::Press { binding: KeyBinding::KeyPress { keycode: "A".into() } },
                                                        "release" => MacroStep::Release { binding: KeyBinding::KeyPress { keycode: "A".into() } },
                                                        "wait" => MacroStep::Wait { ms: 100 },
                                                        _ => MacroStep::Tap { binding: KeyBinding::KeyPress { keycode: "A".into() } },
                                                    };
                                                    on_action.call(MacroEditAction::Change { macro_id: id_k.clone(), change: MacroChange::Step { index: idx, value: next } });
                                                },
                                            }
                                            if let MacroStep::Wait { ms } = step {
                                                NumericField { label: "Delay".to_owned(), unit: "ms", value: *ms as f64, min: Some(0.0), max: Some(10000.0), step: Some(10.0), precision: 0,
                                                    on_commit: move |v: f64| on_action.call(MacroEditAction::Change { macro_id: id_v.clone(), change: MacroChange::Step { index: idx, value: MacroStep::Wait { ms: v.round() as u32 } } }),
                                                }
                                            } else {
                                                SelectField::<String> {
                                                    label: "Keycode".to_owned(),
                                                    value: match step {
                                                        MacroStep::Tap { binding: KeyBinding::KeyPress { keycode } }
                                                        | MacroStep::Press { binding: KeyBinding::KeyPress { keycode } }
                                                        | MacroStep::Release { binding: KeyBinding::KeyPress { keycode } } => keycode.clone(),
                                                        _ => "A".into(),
                                                    },
                                                    options: keycode_options(), disabled: !enabled,
                                                    on_commit: move |kc: String| {
                                                        let b = KeyBinding::KeyPress { keycode: kc };
                                                        let next = match step_clone {
                                                            MacroStep::Press { .. } => MacroStep::Press { binding: b },
                                                            MacroStep::Release { .. } => MacroStep::Release { binding: b },
                                                            _ => MacroStep::Tap { binding: b },
                                                        };
                                                        on_action.call(MacroEditAction::Change { macro_id: id_v.clone(), change: MacroChange::Step { index: idx, value: next } });
                                                    },
                                                }
                                            }
                                            if can_remove_step {
                                                Button { variant: ButtonVariant::Ghost, size: ButtonSize::Sm, disabled: !enabled,
                                                    onclick: move |_| on_action.call(MacroEditAction::Change { macro_id: id_d.clone(), change: MacroChange::RemoveStep { index: idx } }),
                                                    "Remove step"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            div { class: "flex gap-2",
                                Button { variant: ButtonVariant::Secondary, size: ButtonSize::Sm, disabled: !enabled || item.steps.len() >= 128,
                                    onclick: move |_| on_action.call(MacroEditAction::Change { macro_id: id_add.clone(), change: MacroChange::AddStep { value: MacroStep::Tap { binding: KeyBinding::KeyPress { keycode: "A".into() } } } }),
                                    "Add step"
                                }
                                Button { variant: ButtonVariant::Destructive, size: ButtonSize::Sm, disabled: !enabled,
                                    onclick: move |_| on_action.call(MacroEditAction::Remove { macro_id: id_rm.clone() }),
                                    "Remove macro"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
