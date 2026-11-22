//! Synchronous runtime.commit controllers for Keymap layers, bindings, encoders, and macros.
use super::super::pcb_wiring::PcbWiringResolution;
use super::view::KeymapView;
use crate::binding::{BindingLayerChoice, BindingMacroChoice, BindingTarget};
use crate::encoder::{
    EncoderBindingRow, EncoderEditorProjection, EncoderInputProjection, encoder_choices,
};
use crate::layer_edit::{KeymapLayerFeedback, KeymapLayerOperation, layer_resolver};
use crate::macro_edit::{MacroEditAction, macro_resolver};
use crate::runtime::Runtime;
use boardstudio_application::{AcceptedSnapshot, EditResolver, Resolution, Scope, SnapshotToken};
use boardstudio_model::model::{EditCommand, EditOperation, KeyBinding, KeymapChange, KeymapMacro};
use dioxus::prelude::*;
use std::rc::Rc;

#[derive(Clone)]
pub struct LayerSource {
    pub scope: Scope,
    pub token: SnapshotToken,
    pub revision: u64,
}

pub struct LayerActions {
    pub enabled: bool,
    pub feedback: Option<KeymapLayerFeedback>,
    pub on_operation: EventHandler<KeymapLayerOperation>,
}

pub fn use_layer_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    mut active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    _scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> LayerActions {
    let mut feedback = use_signal(|| None::<KeymapLayerFeedback>);
    let enabled = workspace() == "Keymap" && admission_current() && source.is_some();
    let on_operation = EventHandler::new(move |op: KeymapLayerOperation| {
        let Some(src) = source.as_ref() else {
            return;
        };
        let Some(snap) = runtime.model().accepted else {
            return;
        };
        let seed = runtime.operation().0;
        let resolver = layer_resolver(src.scope.clone(), op, seed);
        let added = match resolver.resolve(&snap) {
            Resolution::Submit(EditCommand {
                operation:
                    EditOperation::EditKeymap {
                        change: KeymapChange::AddLayer { id, .. },
                    },
                ..
            }) => Some(id),
            _ => None,
        };
        match runtime.commit("keymap-layer", resolver) {
            Ok(_) => {
                if let Some(id) = added {
                    active_layer.set(id);
                }
                feedback.set(None);
            }
            Err(msg) => feedback.set(Some(KeymapLayerFeedback::Failed(msg))),
        }
    });
    LayerActions {
        enabled,
        feedback: feedback(),
        on_operation,
    }
}

pub struct EncoderInputActions {
    pub projection: Memo<Option<EncoderInputProjection>>,
    pub current: Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

pub fn use_encoder_inputs(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    resolution: Signal<PcbWiringResolution>,
) -> EncoderInputActions {
    let current = use_hook(move || {
        Rc::new(move || {
            let scope = runtime.scope()?;
            let snap = runtime.model().accepted?;
            let res_guard = resolution.read();
            let (plan, settled) = match &*res_guard {
                PcbWiringResolution::Current { identity, plan }
                    if identity.token == snap.token
                        && identity.revision == snap.document.revision =>
                {
                    (Some(plan.as_ref()), true)
                }
                PcbWiringResolution::Failed { identity, .. }
                    if identity.token == snap.token
                        && identity.revision == snap.document.revision =>
                {
                    (None, true)
                }
                _ => (None, false),
            };
            Some(EncoderInputProjection {
                encoders: Rc::from(encoder_choices(&snap.document, &scope.board_id, plan)),
                electrical_plan_settled: settled,
            })
        }) as Rc<dyn Fn() -> Option<EncoderInputProjection>>
    });
    let key = source
        .as_ref()
        .map(|s| (s.scope.clone(), s.token, s.revision));
    let res_val = resolution.read().clone();
    let projection = use_memo(use_reactive((&key, &res_val), {
        let current = current.clone();
        move |_| current()
    }));
    EncoderInputActions {
        projection,
        current,
    }
}

pub struct BindingProjectionSources {
    pub source: Option<LayerSource>,
    pub view: Option<Rc<KeymapView>>,
    pub encoder_projection: Memo<Option<EncoderInputProjection>>,
    pub current_encoder_projection: Rc<dyn Fn() -> Option<EncoderInputProjection>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BindingEditorProjection {
    pub effective_layer_id: String,
    pub key_label: String,
    pub binding: KeyBinding,
    pub layers: Rc<[BindingLayerChoice]>,
    pub macros: Rc<[BindingMacroChoice]>,
}

pub struct BindingActions {
    pub enabled: bool,
    pub projection: Option<Rc<BindingEditorProjection>>,
    pub encoder_projection: Option<Rc<EncoderEditorProjection>>,
    pub error: Option<String>,
    pub on_commit: EventHandler<(String, BindingTarget, KeyBinding)>,
}

pub fn use_binding_operations(
    runtime: Rc<Runtime>,
    sources: BindingProjectionSources,
    active_layer: Signal<String>,
    workspace: Signal<&'static str>,
    _scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> BindingActions {
    let mut error = use_signal(|| None::<String>);
    let model = runtime.model();
    let enabled = workspace() == "Keymap" && admission_current() && sources.source.is_some();
    let (projection, encoder_projection) = if let (Some(src), Some(snap), Some(view)) = (
        sources.source.as_ref(),
        model.accepted.as_ref(),
        sources.view.as_ref(),
    ) {
        let map = snap
            .document
            .keymap
            .as_ref()
            .filter(|m| !m.layers.is_empty());
        let saved_layers = map.map_or(&[][..], |m| m.layers.as_slice());
        let layer_idx = saved_layers
            .iter()
            .position(|l| l.id == active_layer())
            .unwrap_or(0);
        let eff_layer = saved_layers
            .get(layer_idx)
            .map_or_else(|| "base".to_owned(), |l| l.id.clone());
        let layers: Rc<[BindingLayerChoice]> = if saved_layers.is_empty() {
            Rc::from([BindingLayerChoice {
                id: "base".into(),
                name: "Base".into(),
            }])
        } else {
            Rc::from(
                saved_layers
                    .iter()
                    .map(|l| BindingLayerChoice {
                        id: l.id.clone(),
                        name: l.name.clone(),
                    })
                    .collect::<Vec<_>>(),
            )
        };
        let macros: Rc<[BindingMacroChoice]> = Rc::from(
            map.map_or(&[][..], |m| m.macros.as_slice())
                .iter()
                .map(|m| BindingMacroChoice {
                    id: m.id.clone(),
                    name: m.name.clone(),
                })
                .collect::<Vec<_>>(),
        );
        let read_key = |key_id: &str| {
            saved_layers
                .get(layer_idx)
                .and_then(|l| l.bindings.get(key_id))
                .cloned()
                .unwrap_or(if layer_idx > 0 {
                    KeyBinding::Transparent
                } else {
                    KeyBinding::None
                })
        };
        let key_proj = model.selected_part_ids.first().and_then(|id| {
            let k = view.keys.iter().find(|k| k.id.as_ref() == id.as_str())?;
            Some(Rc::new(BindingEditorProjection {
                effective_layer_id: eff_layer.clone(),
                key_label: k.reference.to_string(),
                binding: read_key(id),
                layers: layers.clone(),
                macros: macros.clone(),
            }))
        });
        let enc_proj = sources.encoder_projection.read().as_ref().map(|ep| {
            let rows: Vec<_> = ep
                .encoders
                .iter()
                .map(|e| {
                    let sensor = saved_layers
                        .get(layer_idx)
                        .and_then(|l| l.sensors.get(e.id.as_ref()));
                    let fallback = || {
                        if layer_idx > 0 {
                            KeyBinding::Transparent
                        } else {
                            KeyBinding::None
                        }
                    };
                    EncoderBindingRow {
                        id: e.id.clone(),
                        label: e.label.clone(),
                        clockwise: sensor.map_or_else(fallback, |s| s.clockwise.clone()),
                        counterclockwise: sensor
                            .map_or_else(fallback, |s| s.counterclockwise.clone()),
                        push_key_id: e.push_key_id.clone(),
                        push_binding: e.push_key_id.as_deref().map(read_key),
                    }
                })
                .collect();
            Rc::new(EncoderEditorProjection {
                effective_layer_id: eff_layer,
                rows: Rc::from(rows),
                layers,
                macros,
            })
        });
        let _ = src;
        (key_proj, enc_proj)
    } else {
        (None, None)
    };
    let on_commit = EventHandler::new(
        move |(layer_id, target, binding): (String, BindingTarget, KeyBinding)| {
            let Some(src) = sources.source.as_ref() else {
                return;
            };
            let board_id = src.scope.board_id.clone();
            let change = match target {
                BindingTarget::Key { key_id } | BindingTarget::EncoderPush { key_id, .. } => {
                    KeymapChange::Binding {
                        layer_id,
                        key_id,
                        binding,
                    }
                }
                BindingTarget::EncoderRotation {
                    encoder_id,
                    direction,
                } => KeymapChange::Encoder {
                    layer_id,
                    encoder_id,
                    direction,
                    binding,
                },
            };
            let resolver =
                EditResolver::new("keymap-binding", move |accepted: &AcceptedSnapshot| {
                    if !accepted.document.boards.iter().any(|b| b.id == board_id) {
                        return Resolution::Retire("This board no longer exists.".into());
                    }
                    Resolution::submit(
                        vec![board_id.clone()],
                        EditOperation::EditKeymap {
                            change: change.clone(),
                        },
                    )
                });
            error.set(runtime.commit("keymap-binding", resolver).err());
        },
    );
    BindingActions {
        enabled,
        projection,
        encoder_projection,
        error: error(),
        on_commit,
    }
}

pub struct MacroActions {
    pub enabled: bool,
    pub macros: Rc<[KeymapMacro]>,
    pub error: Option<String>,
    pub on_action: EventHandler<MacroEditAction>,
}

pub fn use_macro_operations(
    runtime: Rc<Runtime>,
    source: Option<LayerSource>,
    workspace: Signal<&'static str>,
    _scope_generation: Signal<u64>,
    admission_current: Rc<dyn Fn() -> bool>,
) -> MacroActions {
    let mut error = use_signal(|| None::<String>);
    let enabled = workspace() == "Keymap" && admission_current() && source.is_some();
    let macros: Rc<[KeymapMacro]> = runtime
        .model()
        .accepted
        .as_ref()
        .and_then(|s| s.document.keymap.as_ref())
        .map_or_else(|| Rc::from([]), |m| Rc::from(m.macros.clone()));
    let on_action = EventHandler::new(move |action: MacroEditAction| {
        let Some(src) = source.as_ref() else {
            return;
        };
        let seed = runtime.operation().0;
        error.set(
            runtime
                .commit(
                    "keymap-macro",
                    macro_resolver(src.scope.clone(), action, seed),
                )
                .err(),
        );
    });
    MacroActions {
        enabled,
        macros,
        error: error(),
        on_action,
    }
}
