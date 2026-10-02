//! Case owns display persistence and domain selection; the viewer owns GPU state.
use super::shared_viewer::{CaseDisplay, CaseSharedViewer, ViewerIdentity, ViewerSignalKind};
use super::{InstanceSelection, ResolvedTheme};
use crate::runtime::{CadScene, Runtime};
use boardstudio_application::Scope;
use boardstudio_web::cad_jobs::captured_case_document;
use dioxus::prelude::*;
use std::{collections::BTreeMap, rc::Rc};

#[derive(Clone, PartialEq)]
pub(super) struct BodySelection {
    pub(super) scope: Scope,
    pub(super) body_id: String,
}

#[derive(Clone, PartialEq)]
pub(super) struct LayerSelection {
    scope: Scope,
    id: String,
}

#[derive(Clone, Copy)]
pub(super) struct CaseSelection {
    pub(super) body: Signal<Option<BodySelection>>,
    pub(super) layer: Signal<Option<LayerSelection>>,
    pub(super) display: Signal<BTreeMap<String, CaseDisplay>>,
}

impl CaseSelection {
    pub(super) fn layer_id(self, scope: &Scope) -> String {
        self.layer
            .read()
            .as_ref()
            .filter(|selection| &selection.scope == scope)
            .map(|selection| selection.id.clone())
            .unwrap_or_default()
    }

    pub(super) fn select_layer(mut self, scope: Scope, id: String) {
        self.layer.set(Some(LayerSelection { scope, id }));
    }

    pub(super) fn display_value(self, scope: &Scope) -> CaseDisplay {
        let key = display_key(scope);
        self.display
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| read_display(&key))
    }

    pub(super) fn save_display(mut self, scope: &Scope, display: CaseDisplay) {
        let key = display_key(scope);
        self.display.write().insert(key.clone(), display.clone());
        persist_display(&key, &display);
    }
}

#[component]
pub(crate) fn CaseViewer(scene: Rc<CadScene>) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let instance_selection = use_context::<InstanceSelection>();
    let selection = use_context::<CaseSelection>();
    let theme = use_context::<ResolvedTheme>().0;
    let key = display_key(&scene.scope);
    let initial_display = use_hook({
        let key = key.clone();
        move || read_display(&key)
    });
    let display = selection
        .display
        .read()
        .get(&key)
        .cloned()
        .unwrap_or(initial_display);
    // The Case editor keeps its independently selected authored body, but a
    // newer explicit tree/viewer layer choice owns the viewer highlight.
    let selected_layer = selection
        .layer
        .read()
        .as_ref()
        .filter(|selected| selected.scope == scene.scope && is_layer(&scene, &selected.id))
        .map(|selected| selected.id.clone())
        .or_else(|| {
            selection
                .body
                .read()
                .as_ref()
                .filter(|selected| {
                    selected.scope == scene.scope && is_layer(&scene, &selected.body_id)
                })
                .map(|selected| selected.body_id.clone())
        })
        .unwrap_or_default();
    let on_signal = {
        let runtime = runtime.clone();
        let scene = scene.clone();
        let mut selection = selection;
        move |event: super::shared_viewer::ScopedViewerSignal| {
            if !event.is_current()
                || !source_is_current(&runtime, instance_selection, &scene, &event.identity)
            {
                return;
            }
            match event.kind {
                ViewerSignalKind::Picked(id) | ViewerSignalKind::LayerSelected(id) => {
                    if !is_layer(&scene, &id) {
                        return;
                    }
                    selection.layer.set(Some(LayerSelection {
                        scope: scene.scope.clone(),
                        id: id.clone(),
                    }));
                    // CAD mesh IDs become authored body selections only when the
                    // current captured document proves that exact membership.
                    if let Ok(document) = captured_case_document(&scene.snapshot, &scene.scope)
                        && document
                            .case_bodies
                            .iter()
                            .any(|body| body.id == id && body.board_id == scene.scope.board_id)
                    {
                        selection.body.set(Some(BodySelection {
                            scope: scene.scope.clone(),
                            body_id: id,
                        }));
                    } else {
                        selection.body.set(None);
                    }
                }
                ViewerSignalKind::Failed(message) => runtime.report(message),
                ViewerSignalKind::SceneAccepted(_)
                | ViewerSignalKind::Lifecycle(_)
                | ViewerSignalKind::WorldPoint(_)
                | ViewerSignalKind::HandleGesture { .. } => {}
            }
        }
    };
    let on_display_change = {
        let runtime = runtime.clone();
        let expected_scene = scene.clone();
        let scope = scene.scope.clone();
        move |event: super::shared_viewer::ScopedDisplayChange| {
            if !event.is_current()
                || !source_is_current(
                    &runtime,
                    instance_selection,
                    &expected_scene,
                    &event.identity,
                )
            {
                return;
            }
            // Keep the entire preference value, including temporarily absent
            // geometry, so a regenerated scene or Undo can restore its display.
            selection.save_display(&scope, event.display.clone());
        }
    };
    rsx! {
        CaseSharedViewer {
            scene,
            selected_layer,
            display,
            resolved_theme: theme().to_owned(),
            on_signal,
            on_display_change,
        }
    }
}

fn source_is_current(
    runtime: &Runtime,
    instance: InstanceSelection,
    expected: &Rc<CadScene>,
    identity: &ViewerIdentity,
) -> bool {
    let model = runtime.model();
    instance.is_current(&model)
        && runtime.scope().as_ref() == Some(&identity.scope)
        && identity.scope == expected.scope
        && identity.snapshot_token == expected.token
        && model
            .accepted
            .as_ref()
            .is_some_and(|accepted| accepted.token == identity.snapshot_token)
        && runtime
            .cad_scene()
            .as_ref()
            .is_some_and(|current| Rc::ptr_eq(current, expected))
}

fn is_layer(scene: &CadScene, id: &str) -> bool {
    id == "pcb" || scene.result.bodies.iter().any(|body| body.id == id)
}

fn read_display(key: &str) -> CaseDisplay {
    let value = web_sys::window()
        .and_then(|window| window.local_storage().ok().flatten())
        .and_then(|storage| storage.get_item(key).ok().flatten())
        .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
    let Some(value) = value else {
        return CaseDisplay::default();
    };
    CaseDisplay {
        hidden: value
            .get("hidden")
            .and_then(|hidden| hidden.as_array())
            .map(|hidden| {
                hidden
                    .iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
        colors: value
            .get("colors")
            .and_then(|colors| colors.as_object())
            .map(|colors| {
                colors
                    .iter()
                    .filter_map(|(id, color)| {
                        color.as_str().map(|color| (id.clone(), color.to_owned()))
                    })
                    .collect()
            })
            .unwrap_or_default(),
    }
}

fn display_key(scope: &Scope) -> String {
    format!(
        "boardstudio:case-display:{}:{}",
        scope.document_id,
        scope.instance_id.as_deref().unwrap_or(&scope.board_id),
    )
}

fn persist_display(key: &str, display: &CaseDisplay) {
    if let Some(storage) =
        web_sys::window().and_then(|window| window.local_storage().ok().flatten())
    {
        let encoded = serde_json::json!({ "hidden": display.hidden, "colors": display.colors });
        let _ = storage.set_item(key, &encoded.to_string());
    }
}
