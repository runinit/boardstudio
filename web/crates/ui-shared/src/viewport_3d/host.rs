//! Shared 3D viewport for immutable scene projections from each workspace.

mod canvas;
mod controls;
mod interaction;
mod scene;

use canvas::*;
use controls::ViewerOverlayConfig;
use scene::*;

use super::assembly_projection::{assembly_layers_with_stack, physical_component_layers};
use super::camera::ProjectionInputs;
use crate::case_display::CaseDisplay;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_host::renderer_host::{RendererPageHost, js_error};
use boardstudio_web_runtime::case_preview::NativePreviewSnapshot;
use boardstudio_web_runtime::layout_viewer_source::LayoutPreviewSnapshot;
use boardstudio_web_runtime::model_delivery::ModelDeliveryRows;
use boardstudio_web_runtime::parts_preview::PartsPreviewSnapshot;
use boardstudio_web_runtime::runtime::{CadScene, KeycapsCadPreview, Runtime};
use boardstudio_web_ui_model::Workspace;
use boardstudio_web_ui_model::state::WorkspaceState;
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, Float32Array, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, PointerEvent};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerIdentity {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub renderer_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ViewerFocusRequest {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub revision: u64,
    pub navigation_id: u64,
    pub target_ids: Vec<String>,
}

/// Geometry-only edit handle projected into the renderer. Workspaces keep
/// their own typed handle targets and map them onto `edit_group`, the
/// edit-mode toggle index that reveals the handle.
#[derive(Clone, Debug, PartialEq)]
pub struct ViewportHandle {
    pub id: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub tangent_x: f32,
    pub tangent_y: f32,
    pub normal_x: f32,
    pub normal_y: f32,
    pub length: f32,
    pub invalid: bool,
    pub edit_group: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandleGesturePhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewerSignalKind {
    Failed(String),
    Picked(String),
    LayerSelected(String),
    HandleGesture {
        phase: HandleGesturePhase,
        handle_id: String,
        point: Option<[f32; 3]>,
    },
}

#[derive(Clone, Debug)]
pub struct ScopedViewerEvent<T, D = ()> {
    pub identity: ViewerIdentity,
    pub kind: T,
    pub display: D,
    owner: Weak<ViewerOwner>,
}

// Preserve the payload field names exposed to consumers of the component.
pub type ScopedViewerSignal = ScopedViewerEvent<ViewerSignalKind>;
pub type ScopedDisplayChange = ScopedViewerEvent<(), CaseDisplay>;

impl<T, D> ScopedViewerEvent<T, D> {
    pub fn is_current(&self) -> bool {
        self.owner
            .upgrade()
            .is_some_and(|owner| owner.is_current(&self.identity))
    }
}

#[derive(Default)]
struct ViewerOwner {
    active: Cell<bool>,
    sequence: Cell<u64>,
    identity: RefCell<Option<ViewerIdentity>>,
}

impl PartialEq for ViewerOwner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl ViewerOwner {
    fn is_current(&self, identity: &ViewerIdentity) -> bool {
        self.active.get() && self.identity.borrow().as_ref() == Some(identity)
    }

    fn emit<T: 'static, D: 'static>(
        self: &Rc<Self>,
        handler: EventHandler<ScopedViewerEvent<T, D>>,
        identity: &ViewerIdentity,
        kind: T,
        display: D,
    ) {
        if self.is_current(identity) {
            handler.call(ScopedViewerEvent {
                identity: identity.clone(),
                kind,
                display,
                owner: Rc::downgrade(self),
            });
        }
    }

    fn next(
        &self,
        (scope, token, revision): (&Scope, SnapshotToken, u64),
    ) -> Result<ViewerIdentity, String> {
        let mut current = self.identity.borrow_mut();
        let sequence = self.sequence.get() + 1;
        if sequence > 9_007_199_254_740_991 {
            return Err("Renderer scene sequence exhausted".into());
        }
        self.sequence.set(sequence);
        let identity = ViewerIdentity {
            scope: scope.clone(),
            snapshot_token: token,
            revision,
            renderer_sequence: sequence,
        };
        *current = Some(identity.clone());
        Ok(identity)
    }
}

struct ProjectionCache {
    // Retain the sources whose addresses form the cache key.
    _source: ViewportSource,
    _preview: Option<Rc<NativePreviewSnapshot>>,
    _models: Option<ModelDeliveryRows>,
    inputs: ProjectionInputs,
    projection: Rc<RendererSceneProjection>,
}

struct RendererSceneProjection {
    identity: ViewerIdentity,
    input: JsValue,
    layers: Vec<(String, String)>,
}

#[derive(Clone)]
struct SourceGuard(Rc<dyn Fn() -> bool>);

impl PartialEq for SourceGuard {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl PartialEq for RendererSceneProjection {
    fn eq(&self, other: &Self) -> bool {
        self.identity == other.identity
    }
}

#[component]
pub fn Viewport3D(
    source: ViewportSource,
    keycaps_preview: Option<KeycapsCadPreview>,
    model_rows: Option<ModelDeliveryRows>,
    #[props(default = "pcb".to_owned())] selected_layer: String,
    display: CaseDisplay,
    resolved_theme: String,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
    #[props(default)] focus_request: Option<ViewerFocusRequest>,
) -> Element {
    let runtime = use_context::<Rc<Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let workspace = use_context::<WorkspaceState>().0;
    let theme = resolved_theme;
    let owner = use_hook(|| Rc::new(ViewerOwner::default()));
    let projection_cache = use_hook(|| Rc::new(RefCell::new(None::<ProjectionCache>)));
    let (case, parts_toolbar) = match &source {
        ViewportSource::Case(case) => (Some(case), None),
        ViewportSource::Parts(_, toolbar) => (None, toolbar.clone()),
        _ => (None, None),
    };
    let Some(view) = source.view(model_rows.as_ref(), workspace) else {
        return rsx! { p { role: "alert", "3D preview source is no longer current." } };
    };
    let matching_preview = view.preview;
    let matching_model_rows = view.model_rows;
    let inputs = ProjectionInputs {
        source: view.pointer,
        preview: matching_preview.map_or(0, |preview| Rc::as_ptr(preview) as usize),
        keycaps: keycaps_preview.as_ref().map(|preview| preview.generation),
        models: matching_model_rows
            .map(|rows| {
                rows.delivered
                    .iter()
                    .map(|model| (model.id.clone(), Rc::as_ptr(&model.mesh) as usize))
                    .collect()
            })
            .unwrap_or_default(),
        theme: theme.clone(),
    };
    let handle_projection = case
        .and_then(|case| case.handle_source.clone())
        .filter(|accepted| {
            // A disposable gesture preview inherits handles only from the current accepted scene.
            view.cad.is_some_and(|displayed| {
                runtime
                    .cad_scene()
                    .is_some_and(|current| Rc::ptr_eq(&current, accepted))
                    && (Rc::ptr_eq(displayed, accepted)
                        || runtime
                            .case_gesture_preview_scene(accepted)
                            .is_some_and(|preview| Rc::ptr_eq(displayed, &preview)))
            })
        })
        .map(|source| CaseHandleProjection {
            source,
            inputs: inputs.clone(),
        });
    let cached = projection_cache
        .borrow()
        .as_ref()
        .filter(|cache| {
            cache.inputs == inputs
                && owner.identity.borrow().as_ref() == Some(&cache.projection.identity)
        })
        .map(|cache| cache.projection.clone());
    let projection = if let Some(cached) = cached {
        cached
    } else {
        let projected = owner.next(view.identity).and_then(|identity| {
            project_source(
                &source,
                identity,
                &theme,
                matching_preview.map(Rc::as_ref),
                matching_model_rows,
                keycaps_preview.as_ref(),
            )
        });
        let projection = match projected {
            Ok(projection) => Rc::new(projection),
            Err(error) => {
                projection_cache.borrow_mut().take();
                return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
            }
        };
        *projection_cache.borrow_mut() = Some(ProjectionCache {
            _source: source.clone(),
            _preview: matching_preview.cloned(),
            _models: matching_model_rows.cloned(),
            inputs,
            projection: projection.clone(),
        });
        projection
    };
    let current_source = SourceGuard({
        let runtime = runtime.clone();
        let source = source.clone();
        let scope = view.identity.0.clone();
        Rc::new(move || {
            runtime.scope().as_ref() == Some(&scope)
                && source.is_current(&runtime)
                && keycaps_preview.as_ref().is_none_or(|preview| {
                    runtime.scope().as_ref() == Some(&preview.scope)
                        && runtime.model().accepted.as_ref().is_some_and(|accepted| {
                            accepted.token == preview.token
                                && accepted.document.revision == preview.revision
                        })
                })
        })
    });
    let generated_layers = projection
        .layers
        .iter()
        .filter(|(id, _)| id != "pcb")
        .cloned();
    let stack = view.cad.and_then(|scene| scene.mechanical.as_ref());
    let assembly_layers = assembly_layers_with_stack(
        generated_layers,
        stack
            .into_iter()
            .flat_map(|assembly| assembly.stack.iter().map(|layer| layer.id.clone())),
    );
    let selectable_layers = {
        let mut layers = vec!["pcb".to_owned()];
        if let Some(scene) = view.cad.map(Rc::as_ref) {
            if let Some(assembly) = &scene.mechanical {
                layers.extend(assembly.stack.iter().map(|layer| layer.id.clone()));
                if !assembly.gasket_supports.is_empty() {
                    layers.push("gaskets".to_owned());
                }
            }
            layers.extend(
                scene
                    .result
                    .bodies
                    .iter()
                    .filter(|body| body.id.starts_with("gasket:"))
                    .map(|body| body.id.clone()),
            );
        }
        layers
    };
    let component_layers = physical_component_layers(view.models, matching_model_rows);
    let module_model_errors = matching_model_rows
        .into_iter()
        .flat_map(|rows| rows.failures.iter())
        .filter(|failure| failure.reference.starts_with("module-model/"))
        .map(|failure| format!("{}: {}", failure.reference, failure.reason))
        .collect::<Vec<_>>();
    let base_message = case.and_then(|case| case.gesture_message.clone());
    let gesture_message = if module_model_errors.is_empty() {
        base_message
    } else {
        Some(match base_message {
            Some(existing) if !existing.is_empty() => {
                format!(
                    "{existing} · Mounted-module model: {}",
                    module_model_errors.join("; ")
                )
            }
            _ => format!("Mounted-module model: {}", module_model_errors.join("; ")),
        })
    };
    let overlay = ViewerOverlayConfig {
        layers: projection.layers.clone(),
        assembly_layers,
        component_layers,
        selectable_layers,
        selected_layer,
        selected_reference: case.and_then(|case| case.selected_reference.clone()),
        display,
        toolbar_extra: case.and_then(|case| case.toolbar_extra.clone()),
        edit_labels: case.map_or_else(Vec::new, |case| case.edit_labels.clone()),
        toolbar_leading: parts_toolbar,
        handles: case.map(|case| case.handles.clone()).unwrap_or_default(),
        gesture_message,
    };
    rsx! { MountedCanvas {
            projection,
            overlay,
            canvas_context: view.context,
            theme,
            owner,
            current_source,
            on_signal,
            on_display_change,
            handle_projection,
            edit_mode: case.and_then(|case| case.edit_mode),
            handle_preview: case.and_then(|case| case.handle_preview.clone()),
            focus_request,
    } }
}

#[derive(Clone, PartialEq)]
pub struct CaseViewerSource {
    pub scene: Option<Rc<CadScene>>,
    pub preview: Option<Rc<NativePreviewSnapshot>>,
    /// Workspace-owned toolbar content (Case mechanical settings actions)
    /// rendered beside the generic edit-mode toggle.
    pub toolbar_extra: Option<Element>,
    /// Edit-mode labels indexed by `ViewportHandle::edit_group`.
    pub edit_labels: Vec<&'static str>,
    /// Case owns the edit-mode signal so its `toolbar_extra` content can
    /// react to the generic toggle the viewport renders.
    pub edit_mode: Option<Signal<Option<usize>>>,
    pub handle_source: Option<Rc<CadScene>>,
    pub handles: Vec<ViewportHandle>,
    pub handle_preview: Option<Vec<ViewportHandle>>,
    pub gesture_message: Option<String>,
    pub selected_reference: Option<String>,
}

#[derive(Clone, PartialEq)]
pub enum ViewportSource {
    Case(CaseViewerSource),
    Layout(Rc<LayoutPreviewSnapshot>),
    Parts(Rc<PartsPreviewSnapshot>, Option<Element>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewerCanvasContext {
    Case,
    Parts,
    Layout,
    Keymap,
    Keycaps,
}

impl ViewerCanvasContext {
    fn accessible_name(self) -> &'static str {
        match self {
            Self::Case => {
                "Interactive 3D Case preview. Click a visible body to select its current mapped item; use the controls to navigate and change display."
            }
            Self::Parts => {
                "Interactive 3D Parts sample preview. Navigate the isolated read-only sample; it cannot edit the active project."
            }
            Self::Layout => {
                "Interactive 3D Layout PCB assembly. Click a visible component to select its current Layout part; use the controls to navigate and change display."
            }
            Self::Keymap => {
                "Interactive 3D Keymap board preview. Click a visible current-board component or key to select it."
            }
            Self::Keycaps => {
                "Interactive 3D Keycaps board preview. Click a visible current-board component or key to select it."
            }
        }
    }

    pub(crate) fn fit_label(self) -> &'static str {
        match self {
            Self::Parts => "Fit sample",
            Self::Case => "Fit case",
            Self::Layout => "Fit layout",
            Self::Keymap => "Fit Keymap",
            Self::Keycaps => "Fit Keycaps",
        }
    }

    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Parts => "Parts sample",
            Self::Case => "Case",
            Self::Layout => "Layout",
            Self::Keymap => "Keymap",
            Self::Keycaps => "Keycaps",
        }
    }
}

struct SourceView<'a> {
    identity: (&'a Scope, SnapshotToken, u64),
    pointer: usize,
    context: ViewerCanvasContext,
    cad: Option<&'a Rc<CadScene>>,
    preview: Option<&'a Rc<NativePreviewSnapshot>>,
    model_rows: Option<&'a ModelDeliveryRows>,
    models: &'a [boardstudio_model::model::PcbModel],
}

impl ViewportSource {
    fn view<'a>(
        &'a self,
        model_rows: Option<&'a ModelDeliveryRows>,
        workspace: Signal<Workspace>,
    ) -> Option<SourceView<'a>> {
        match self {
            Self::Case(case) => {
                let (identity, pointer, cad) = match (&case.scene, &case.preview) {
                    (Some(scene), _) => (
                        (&scene.scope, scene.token, scene.snapshot.document.revision),
                        Rc::as_ptr(scene) as usize,
                        Some(scene),
                    ),
                    (None, Some(preview)) => (
                        (
                            &preview.owner.scope,
                            preview.owner.snapshot_token,
                            preview.accepted_document.revision,
                        ),
                        Rc::as_ptr(preview) as usize,
                        None,
                    ),
                    (None, None) => return None,
                };
                let preview = case.preview.as_ref().filter(|preview| {
                    preview.owner.scope == *identity.0 && preview.owner.snapshot_token == identity.1
                });
                Some(SourceView {
                    identity,
                    pointer,
                    context: ViewerCanvasContext::Case,
                    cad,
                    preview,
                    model_rows: preview.and(model_rows),
                    models: preview.map_or(&[], |preview| preview.preview.models.as_slice()),
                })
            }
            Self::Layout(preview) => {
                if !preview.lease.matches(&preview.owner) {
                    return None;
                }
                Some(SourceView {
                    identity: (
                        &preview.owner.scope,
                        preview.owner.snapshot_token,
                        preview.document.revision,
                    ),
                    pointer: Rc::as_ptr(preview) as usize,
                    context: match workspace() {
                        Workspace::Keymap => ViewerCanvasContext::Keymap,
                        Workspace::Keycaps => ViewerCanvasContext::Keycaps,
                        _ => ViewerCanvasContext::Layout,
                    },
                    cad: None,
                    preview: None,
                    model_rows,
                    models: &preview.preview.models,
                })
            }
            Self::Parts(preview, _) => {
                if !preview.lease.matches(&preview.owner) {
                    return None;
                }
                Some(SourceView {
                    identity: (
                        &preview.owner.scope,
                        preview.owner.snapshot_token,
                        preview.sample_document.revision,
                    ),
                    pointer: Rc::as_ptr(preview) as usize,
                    context: ViewerCanvasContext::Parts,
                    cad: None,
                    preview: None,
                    model_rows: preview.model_rows.as_ref(),
                    models: &preview.preview.models,
                })
            }
        }
    }

    fn is_current(&self, runtime: &Runtime) -> bool {
        match self {
            Self::Case(case) => {
                if let Some(scene) = &case.scene {
                    runtime.scope().as_ref() == Some(&scene.scope)
                        && runtime
                            .model()
                            .accepted
                            .as_ref()
                            .is_some_and(|snapshot| snapshot.token == scene.token)
                        && (runtime
                            .cad_scene()
                            .as_ref()
                            .is_some_and(|current| Rc::ptr_eq(current, scene))
                            || runtime
                                .case_gesture_preview_scene(scene)
                                .as_ref()
                                .is_some_and(|current| Rc::ptr_eq(current, scene)))
                } else if let Some(preview) = &case.preview {
                    runtime.native_case_preview().is_some_and(|current| {
                        current.owner == preview.owner && Rc::ptr_eq(&current.lease, &preview.lease)
                    })
                } else {
                    false
                }
            }
            Self::Layout(preview) => runtime.layout_preview().is_some_and(|current| {
                current.owner == preview.owner
                    && Rc::ptr_eq(&current.lease, &preview.lease)
                    && preview.lease.matches(&preview.owner)
            }),
            Self::Parts(preview, _) => {
                preview.lease.matches(&preview.owner)
                    && runtime.parts_preview_snapshot_is_current(preview)
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum AssemblyView {
    Assembled,
    Exploded,
    Section,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub(crate) enum RenderMode {
    Shaded,
    Wireframe,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct TransientView {
    pub(crate) assembly: AssemblyView,
    pub(crate) mode: RenderMode,
    pub(crate) explode_amount: f32,
    pub(crate) section_plane: &'static str,
    pub(crate) section_position: f32,
    pub(crate) show_section_plane: bool,
    pub(crate) show_hidden: bool,
}

impl Default for TransientView {
    fn default() -> Self {
        Self {
            assembly: AssemblyView::Assembled,
            mode: RenderMode::Hybrid,
            explode_amount: 1.0,
            section_plane: "YZ",
            section_position: 0.0,
            show_section_plane: true,
            show_hidden: false,
        }
    }
}

#[derive(Clone)]
struct CaseHandleProjection {
    source: Rc<CadScene>,
    inputs: ProjectionInputs,
}

impl PartialEq for CaseHandleProjection {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.source, &other.source) && self.inputs == other.inputs
    }
}

impl CaseHandleProjection {
    fn continues_preview(&self, next: &Self) -> bool {
        Rc::ptr_eq(&self.source, &next.source) && self.inputs.continues_drag_in(&next.inputs)
    }
}

fn display_state(
    display: &CaseDisplay,
    selected_layer: &str,
    theme: &str,
    transient: TransientView,
) -> Result<JsValue, String> {
    let value = serde_json::json!({
        "hidden": &display.hidden,
        "selectedLayer": selected_layer,
        "view": Into::<&'static str>::into(transient.assembly),
        "mode": Into::<&'static str>::into(transient.mode),
        "theme": theme,
        "explodeAmount": transient.explode_amount,
        "sectionPlane": transient.section_plane,
        "sectionPosition": transient.section_position,
        "showSectionPlane": transient.show_section_plane,
        "showHidden": transient.show_hidden,
        "colors": &display.colors,
    });
    js_sys::JSON::parse(&value.to_string()).map_err(js_error)
}

fn handles_value(handles: &[ViewportHandle]) -> Result<JsValue, String> {
    let number = |value: f32| {
        if value.is_finite() {
            value.into()
        } else {
            JsValue::NULL
        }
    };
    let point = |x, y| js_obj([("x", number(x)), ("y", number(y))]);
    let values = Array::new();
    for handle in handles {
        values.push(&js_obj([
            ("id", handle.id.as_str().into()),
            ("at", point(handle.x, handle.y)?),
            ("tangent", point(handle.tangent_x, handle.tangent_y)?),
            ("normal", point(handle.normal_x, handle.normal_y)?),
            ("length", number(handle.length)),
            ("z", number(handle.z)),
            ("invalid", handle.invalid.into()),
        ])?);
    }
    Ok(values.into())
}
