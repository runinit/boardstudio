//! Case scene admission, selection, display preferences, and edit gestures.

mod gesture;
use gesture::{CaseDrag, handle_case_gesture, retire_drag};

use crate::case_bodies::CaseBodyEdit;
use crate::case_preview::NativePreviewSnapshot;
use crate::mechanical_settings::{
    self, MechanicalSettingsPatchExt, is_gasket_layer, submit_settings_patch,
};
use crate::runtime::{CadScene, Runtime};
use crate::shared_viewer::{
    CaseDisplay, CaseViewerSource, ScopedDisplayChange, ScopedViewerSignal, ViewerFocusRequest,
    ViewerHandle, ViewerIdentity, ViewerSignalKind, Viewport3D, ViewportSource,
};
use crate::{InstanceSelection, ResolvedAppearance, selection::SelectionAdapter};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_ui_shared::ui::{
    alert::{Alert, AlertDescription, AlertVariant},
    button::{Button, ButtonVariant},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    item::{Item, ItemSize},
};
use dioxus::prelude::*;
use mechanical_settings::MechanicalSettingsProps as Settings;
use std::{cell::RefCell, collections::BTreeMap, rc::Rc};

#[derive(Clone, PartialEq)]
pub struct BodySelection {
    pub scope: Scope,
    pub body_id: String,
}

#[derive(Clone, PartialEq)]
pub struct LayerSelection {
    scope: Scope,
    id: String,
    focus: Option<(SnapshotToken, u64)>,
}

#[derive(Clone, Copy)]
pub struct CaseSelection {
    pub body: Signal<Option<BodySelection>>,
    pub layer: Signal<Option<LayerSelection>>,
    pub display: Signal<BTreeMap<String, CaseDisplay>>,
    pub body_edit_portal: CaseBodyEditPortal,
}

#[derive(Clone, Copy)]
pub struct CaseBodyEditPortal {
    pub dispatch: Signal<Option<CaseBodyEditDispatch>>,
    pub editable: Signal<bool>,
}

pub type CaseBodyEditDispatch = Rc<dyn Fn(CaseBodyEdit, Scope, SnapshotToken, u64)>;

impl CaseSelection {
    pub fn layer_id(self, scope: &Scope) -> String {
        self.layer
            .read()
            .as_ref()
            .filter(|selection| &selection.scope == scope)
            .map(|selection| selection.id.clone())
            .unwrap_or_default()
    }

    pub fn select_layer(mut self, scope: Scope, id: String) {
        self.layer.set(Some(LayerSelection {
            scope,
            id,
            focus: None,
        }));
    }

    pub fn focus_layer(mut self, scope: Scope, token: SnapshotToken, revision: u64, id: String) {
        self.layer.set(Some(LayerSelection {
            scope,
            id,
            focus: Some((token, revision)),
        }));
    }

    pub fn clear_layer_for_scope(mut self, scope: &Scope) {
        if self
            .layer
            .read()
            .as_ref()
            .is_some_and(|selection| &selection.scope == scope)
        {
            self.layer.set(None);
        }
    }

    pub fn display_value(self, scope: &Scope) -> CaseDisplay {
        let key = crate::case_display::display_storage_key(scope);
        self.display
            .read()
            .get(&key)
            .cloned()
            .unwrap_or_else(|| crate::case_display::load_display(&key))
    }

    pub fn save_display(mut self, scope: &Scope, display: CaseDisplay) {
        let key = crate::case_display::display_storage_key(scope);
        self.display.write().insert(key.clone(), display.clone());
        crate::case_display::store_display(&key, &display);
    }
}

#[derive(Clone, PartialEq)]
struct CaseKeycapsSource {
    scope: Scope,
    token: SnapshotToken,
    revision: u64,
    prepared: Option<boardstudio_model::model::PreparedCaseAssemblyIR>,
}

fn use_case_keycaps(
    runtime: Rc<Runtime>,
    scope: &Scope,
    token: SnapshotToken,
    revision: u64,
) -> (Option<crate::runtime::KeycapsCadPreview>, Element) {
    let _ = use_context::<Signal<u64>>()();
    let source = runtime
        .model()
        .accepted
        .filter(|accepted| {
            runtime.scope().as_ref() == Some(scope)
                && accepted.token == token
                && accepted.document.revision == revision
                && accepted.document.keycaps.is_some()
        })
        .map(|_| CaseKeycapsSource {
            scope: scope.clone(),
            token,
            revision,
            prepared: runtime
                .cad_scene()
                .filter(|scene| {
                    scene.exact
                        && scene.scope == *scope
                        && scene.token == token
                        && scene.snapshot.document.revision == revision
                        && scene.prepared.revision == revision
                })
                .map(|scene| scene.prepared.clone()),
        });
    let mut retry = use_signal(|| 0_u64);
    let resolved = use_memo(use_reactive((&source, &retry()), {
        let runtime = runtime.clone();
        move |(source, _retry)| {
            source.as_ref().map(|source| {
                runtime.resolve_case_keycaps_preview(
                    source.scope.clone(),
                    source.token,
                    source.revision,
                )
            })
        }
    }));
    let resolved = resolved.read();
    let (findings, input, resolve_error) = match resolved.as_ref() {
        Some(Ok(fit)) => {
            let blocked = fit.specs.is_empty()
                || fit
                    .findings
                    .iter()
                    .any(|finding| finding.severity == boardstudio_model::model::Severity::Error);
            let input = if blocked {
                None
            } else {
                source
                    .as_ref()
                    .map(|source| crate::runtime::KeycapsPreviewInput {
                        scope: source.scope.clone(),
                        token: source.token,
                        revision: source.revision,
                        specs: fit.specs.clone(),
                    })
            };
            (fit.findings.as_slice(), input, None)
        }
        Some(Err(error)) => (&[][..], None, Some(error.clone())),
        None => (&[][..], None, None),
    };
    let keycaps =
        boardstudio_web_ui_shared::viewport_3d::sources::use_keycaps_cad_preview(runtime, input);
    let keycaps_retry = keycaps.retry;
    let error = resolve_error.or(keycaps.error);
    let feedback = rsx! {
        if source.is_some() && keycaps.pending { p { role: "status", "Generating keycap CAD…" } }
        if let Some(error) = error {
            Alert { variant: AlertVariant::Destructive,
                AlertDescription { "Keycap preview failed: {error}" }
                Button { variant: ButtonVariant::Secondary, button_type: "button",
                    class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                    onclick: move |_| { retry += 1; keycaps_retry.call(()); }, "Retry keycaps" }
            }
        }
        if !findings.is_empty() {
            Collapsible { class: "flex-none max-h-[min(25vh,180px)] overflow-auto overscroll-contain", keep_mounted: true, default_open: true,
                CollapsibleTrigger { class: "w-full p-3 bg-muted text-foreground", aria_label: "Keycap clearance · {findings.len()} findings", "Keycap clearance · {findings.len()} findings" }
                CollapsibleContent { class: "data-[open=false]:hidden",
                    for finding in findings { Item { size: ItemSize::Sm, key: "{finding.id}", p { "{finding.message}" } } }
                }
            }
        }
    };
    (keycaps.preview, feedback)
}
#[derive(Clone, PartialEq)]
pub(crate) enum CaseViewportSource {
    Scene {
        scene: Rc<CadScene>,
        pcb_preview: Option<Rc<NativePreviewSnapshot>>,
    },
    Preview(Rc<NativePreviewSnapshot>),
}

impl CaseViewportSource {
    pub(crate) fn scope(&self) -> &Scope {
        match self {
            Self::Scene { scene, .. } => &scene.scope,
            Self::Preview(preview) => &preview.owner.scope,
        }
    }
}

#[component]
pub(crate) fn CaseViewport(
    source: CaseViewportSource,
    model_rows: Option<crate::model_delivery::ModelDeliveryRows>,
    mechanical_settings: Option<crate::MechanicalSettingsProps>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let scope = source.scope();
    let (token, revision, scene, preview) = match &source {
        CaseViewportSource::Scene { scene, pcb_preview } => (
            scene.token,
            scene.snapshot.document.revision,
            Some(scene.clone()),
            pcb_preview.clone(),
        ),
        CaseViewportSource::Preview(preview) => (
            preview.owner.snapshot_token,
            preview.accepted_document.revision,
            None,
            Some(preview.clone()),
        ),
    };
    let (keycaps_preview, keycaps_feedback) =
        use_case_keycaps(runtime.clone(), scope, token, revision);
    let instance_selection = use_context::<InstanceSelection>();
    let live_preview = (use_context::<crate::CaseGenerationState>().live_preview)();
    let selection_adapter = use_context::<SelectionAdapter>();
    let mut selection = use_context::<CaseSelection>();
    let theme = use_context::<ResolvedAppearance>().0;
    let selected_reference = {
        let selected = selection_adapter
            .selected_context
            .read()
            .clone()
            .filter(|selected| selected.scope == *scope);
        crate::case_selection::selected_part_summary(&runtime.model(), selected.as_ref())
            .map(|summary| summary.title)
    };
    let display = selection.display_value(scope);
    let selected_layer = scene.as_ref().map_or_else(
        || "pcb".to_owned(),
        |scene| {
            let layer = selection.layer_id(scope);
            if is_layer(scene, &layer) {
                return layer;
            }
            selection
                .body
                .read()
                .as_ref()
                .filter(|body| body.scope == *scope && is_layer(scene, &body.body_id))
                .map(|body| body.body_id.clone())
                .unwrap_or_default()
        },
    );
    let focus_request = scene.as_ref().and_then(|scene| {
        let layer = selection.layer.read();
        let id = layer
            .as_ref()
            .filter(|selected| {
                selected.focus == Some((scene.token, scene.snapshot.document.revision))
                    && selected.scope == scene.scope
                    && is_layer(scene, &selected.id)
            })
            .map(|selected| selected.id.clone())?;
        Some(ViewerFocusRequest {
            scope: scene.scope.clone(),
            snapshot_token: scene.token,
            revision: scene.snapshot.document.revision,
            navigation_id: 0,
            target_ids: vec![id],
        })
    });
    let handles = scene.as_ref().map_or_else(Vec::new, |scene| {
        let settings = mechanical_settings.as_ref().filter(|settings| {
            settings.editable
                && settings.identity.scope == scene.scope
                && settings.identity.snapshot_token == scene.token
                && settings.identity.revision == scene.snapshot.document.revision
        });
        crate::case_handles::project_handles(
            scene,
            settings.is_some(),
            settings.and_then(|settings| settings.configuration.as_deref()),
            (selection.body_edit_portal.editable)(),
        )
    });
    let gesture = use_hook(|| Rc::new(RefCell::new(CaseDrag::default())));
    let mut handle_preview = use_signal(|| None::<Vec<ViewerHandle>>);
    let edit_mode = use_signal(|| None::<usize>);
    let mut gesture_field = use_signal(|| None::<String>);
    let mut gesture_message = use_signal(|| None::<String>);
    use_effect(use_reactive(
        (scope, &token, &revision, &scene.as_ref().map(Rc::as_ptr)),
        {
            let gesture = gesture.clone();
            let runtime = runtime.clone();
            move |_| {
                retire_drag(&runtime, &gesture);
                handle_preview.set(None);
                gesture_field.set(None);
                gesture_message.set(None);
            }
        },
    ));
    use_effect(use_reactive((&live_preview,), {
        let gesture = gesture.clone();
        let runtime = runtime.clone();
        move |(live_preview,)| {
            if !live_preview && let Some(owner) = gesture.borrow_mut().preview.take() {
                runtime.cancel_case_gesture_preview(&owner);
            }
        }
    }));
    use_drop({
        let runtime = runtime.clone();
        let gesture = gesture.clone();
        move || {
            retire_drag(&runtime, &gesture);
        }
    });
    let current = {
        let scene = scene.clone();
        let preview = preview.clone();
        let runtime = runtime.clone();
        Rc::new(move |identity: &ViewerIdentity| match scene.as_ref() {
            Some(scene) => {
                let model = runtime.model();
                instance_selection.is_current(&model)
                    && runtime.scope().as_ref() == Some(&identity.scope)
                    && identity.scope == scene.scope
                    && identity.snapshot_token == scene.token
                    && identity.revision == scene.snapshot.document.revision
                    && model.accepted.as_ref().is_some_and(|accepted| {
                        accepted.token == identity.snapshot_token
                            && accepted.document.revision == identity.revision
                    })
                    && runtime
                        .cad_scene()
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(current, scene))
            }
            None => preview.as_ref().is_some_and(|preview| {
                identity.scope == preview.owner.scope
                    && identity.snapshot_token == preview.owner.snapshot_token
                    && runtime.native_case_preview().is_some_and(|current| {
                        current.owner == preview.owner && Rc::ptr_eq(&current.lease, &preview.lease)
                    })
            }),
        })
    };
    let on_signal =
        {
            let runtime = runtime.clone();
            let scene = scene.clone();
            let preview = preview.clone();
            let handles = handles.clone();
            let settings = mechanical_settings.clone();
            let current = current.clone();
            move |event: ScopedViewerSignal| {
                if !event.is_current() || !current(&event.identity) {
                    return;
                }
                if let ViewerSignalKind::Picked(id) = &event.kind
                    && let Some(preview) = preview.as_ref()
                    && select_native_preview_model(
                        &runtime,
                        instance_selection,
                        &selection_adapter,
                        preview,
                        &event.identity,
                        id,
                    )
                {
                    return;
                }
                if let ViewerSignalKind::Failed(message) = event.kind {
                    runtime.report(message);
                    return;
                }
                let Some(scene) = scene.as_ref() else { return };
                match event.kind {
                    ViewerSignalKind::Picked(id) | ViewerSignalKind::LayerSelected(id)
                        if is_layer(scene, &id) =>
                    {
                        selection.select_layer(scene.scope.clone(), id.clone());
                        let authored = captured_case_document(&scene.snapshot, &scene.scope)
                            .is_ok_and(|document| {
                                document.case_bodies.iter().any(|body| {
                                    body.id == id && body.board_id == scene.scope.board_id
                                })
                            });
                        selection.body.set(authored.then(|| BodySelection {
                            scope: scene.scope.clone(),
                            body_id: id,
                        }));
                    }
                    ViewerSignalKind::HandleGesture {
                        phase,
                        handle_id,
                        point,
                    } => handle_case_gesture(
                        scene,
                        &event.identity,
                        settings.as_ref(),
                        &handles,
                        &gesture,
                        &mut handle_preview,
                        &mut gesture_field,
                        &mut gesture_message,
                        selection,
                        &runtime,
                        live_preview,
                        phase,
                        &handle_id,
                        point,
                    ),
                    _ => {}
                }
            }
        };
    let on_display_change = move |event: ScopedDisplayChange| {
        if event.is_current() && current(&event.identity) {
            selection.save_display(&event.identity.scope, event.display);
        }
    };
    let displayed_scene = scene.as_ref().map(|scene| {
        runtime
            .case_gesture_preview_scene(scene)
            .unwrap_or_else(|| scene.clone())
    });
    let toolbar_extra = case_edit_extra(
        &selected_layer,
        mechanical_settings.as_ref(),
        edit_mode(),
        runtime.clone(),
        scene.clone(),
    );
    let gesture_message = runtime.case_gesture_preview_message().or_else(|| {
        let Some(field_id) = gesture_field() else {
            return gesture_message();
        };
        let feedback = mechanical_settings
            .as_ref()?
            .feedback
            .iter()
            .rev()
            .find(|entry| entry.field_id == field_id)?;
        match feedback.state {
            mechanical_settings::MechanicalSettingsFeedbackState::Landed => None,
            mechanical_settings::MechanicalSettingsFeedbackState::Failed => Some(
                feedback
                    .message
                    .clone()
                    .unwrap_or_else(|| "Position could not be saved.".into()),
            ),
        }
    });
    rsx! {
        Viewport3D {
            source: ViewportSource::Case(CaseViewerSource {
                scene: displayed_scene,
                preview: preview.clone(),
                toolbar_extra,
                edit_labels: vec!["Edit gaskets", "Edit mounts"],
                edit_mode: Some(edit_mode),
                handle_source: scene.clone(),
                handles: handles.iter().map(crate::case_handles::viewport_handle).collect(),
                handle_preview: handle_preview()
                    .map(|preview| preview.iter().map(crate::case_handles::viewport_handle).collect()),
                gesture_message,
                selected_reference,
            }),
            keycaps_preview,
            model_rows: model_rows.clone(),
            selected_layer,
            display,
            resolved_theme: theme,
            on_signal,
            on_display_change,
            focus_request,
        }
        {keycaps_feedback}
        if let Some(scene) = scene {
            crate::case_collision::CaseCollisionChecks { scene, preview, model_rows }
        }
    }
}

fn select_native_preview_model(
    runtime: &Rc<Runtime>,
    instance_selection: InstanceSelection,
    adapter: &SelectionAdapter,
    preview: &Rc<crate::case_preview::NativePreviewSnapshot>,
    identity: &ViewerIdentity,
    model_reference: &str,
) -> bool {
    let owner = &preview.owner;
    let model = runtime.model();
    if !instance_selection.is_current(&model)
        || runtime.native_case_preview().is_none_or(|current| {
            current.owner != *owner || !Rc::ptr_eq(&current.lease, &preview.lease)
        })
    {
        return false;
    }
    let Some(part_id) = crate::case_preview::native_preview_pick_part_id(
        preview,
        &identity.scope,
        identity.snapshot_token,
        identity.revision,
        model_reference,
    ) else {
        return false;
    };
    let Some(context) = crate::objects::context_for_part(&model, &part_id) else {
        return false;
    };
    crate::selection::submit_context(
        runtime,
        adapter,
        crate::selection::ContextRequest {
            scope: owner.scope.clone(),
            context,
            mode: boardstudio_application::SelectionMode::Replace,
        },
    );
    true
}

fn is_layer(scene: &CadScene, id: &str) -> bool {
    id == "pcb"
        || scene.result.bodies.iter().any(|body| body.id == id)
        || scene.mechanical.as_ref().is_some_and(|assembly| {
            let supports = &assembly.gasket_supports;
            assembly.stack.iter().any(|layer| layer.id == id)
                || is_gasket_layer(id, supports, |support| &support.id)
        })
}

/// Case's `toolbar_extra` for `<Viewport3D />`: the unlink action for the
/// selected gasket support, shown while the gasket edit mode is active.
fn case_edit_extra(
    selected_layer: &str,
    settings: Option<&Settings>,
    edit_mode: Option<usize>,
    runtime: Rc<Runtime>,
    scene: Option<Rc<CadScene>>,
) -> Option<Element> {
    if edit_mode != Some(0) {
        return None;
    }
    let selected_support = settings.and_then(|settings| {
        let id = mechanical_settings::gasket_support_id(selected_layer)?;
        settings
            .gasket_supports
            .iter()
            .find(|support| !support.is_previous && support.id == id)
    });
    let unlink = selected_support.zip(settings).map(|(support, settings)| {
        let settings = settings.clone();
        let anchors = settings
            .gasket_supports
            .iter()
            .filter(|candidate| {
                candidate.id == support.id
                    || candidate.pair_id.as_deref() == Some(support.id.as_str())
            })
            .map(mechanical_settings::MechanicalGasketSupportRow::saved_anchor)
            .collect();
        let patch = mechanical_settings::MechanicalSettingsPatch::SetGasketSupportUnlinked {
            support_id: support.id.clone(),
            pair_id: support.pair_id.clone(),
            anchors,
        };
        let field_id = patch.field_id();
        let feedback = settings
            .feedback
            .iter()
            .rev()
            .find(|feedback| feedback.field_id == field_id)
            .filter(|feedback| {
                feedback.state == mechanical_settings::MechanicalSettingsFeedbackState::Failed
            })
            .map(|feedback| {
                feedback
                    .message
                    .clone()
                    .unwrap_or_else(|| "The support could not be unlinked.".into())
            });
        let unlinked = support.unlinked;
        let disabled = !settings.editable || unlinked;
        let on_click = move |_| {
            let current = runtime.scope().as_ref() == Some(&settings.identity.scope)
                && runtime.model().accepted.as_ref().is_some_and(|accepted| {
                    accepted.token == settings.identity.snapshot_token
                        && accepted.document.revision == settings.identity.revision
                })
                && scene.as_ref().is_some_and(|scene| {
                    runtime
                        .cad_scene()
                        .is_some_and(|current| Rc::ptr_eq(&current, scene))
                });
            if current {
                submit_settings_patch(&settings, patch.clone());
            }
        };
        rsx! {
            Button { variant: ButtonVariant::Secondary, button_type: "button",
                class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11",
                disabled, onclick: on_click, "Unlink selected support"
            }
            if let Some(message) = feedback { p { role: "alert", "{message}" } }
            if unlinked { p { "This gasket is unlinked from its pair." } }
        }
    });
    Some(match unlink {
        Some(unlink) => unlink,
        None => rsx! { p { "Select a gasket in Objects to unlink its mirrored support pair." } },
    })
}
