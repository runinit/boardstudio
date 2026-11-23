//! Shared 3D viewport for immutable scene projections from each workspace.

use boardstudio_web_ui_model::Workspace;
mod canvas;
mod case;
pub use case::{
    BodySelection, CaseBodyEditDispatch, CaseBodyEditPortal, CaseSelection, LayerSelection,
};
pub(crate) use case::{CaseViewport, CaseViewportSource};
mod interaction;
mod scene;

use canvas::*;
use scene::*;

use super::case_assembly_layers::{
    CaseAssemblyLayers, assembly_layers_with_stack, physical_component_layers,
};
use super::layout_viewer_source::LayoutPreviewSnapshot;
use super::mechanical_settings::MechanicalSettingsPatchExt;
use super::model_delivery::ModelDeliveryRows;
use crate::case_preview::NativePreviewSnapshot;
use crate::{runtime::CadScene, viewport_camera::ProjectionInputs};
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web_host::cad_jobs::captured_case_document;
use boardstudio_web_host::renderer_host::{RendererPageHost, js_error};
use boardstudio_web_ui_shared::ui::{
    button::{Button as UiButton, ButtonSize as UiButtonSize, ButtonVariant as UiButtonVariant},
    checkbox::{Checkbox as UiCheckbox, CheckboxState},
    collapsible::{Collapsible, CollapsibleContent, CollapsibleTrigger},
    field::Field,
    input::InputType,
    input_group::{InputGroup, InputGroupInput},
    label::Label,
    select::{Select, SelectOption},
    slider::Slider,
    toggle_group::{ToggleGroup, ToggleItem},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, Float32Array, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{HtmlCanvasElement, PointerEvent};

pub use super::case_display::CaseDisplay;
use super::case_display::preference_ids;

/// Reusable canvas header and view switch for the Keymap and Keycaps consumers.
/// The page owner supplies guarded actions and the accepted shared view state.
#[derive(Props, Clone, PartialEq)]
pub struct DesignViewToolbarProps {
    #[props(default)]
    pub contextual_tools: Option<Element>,
    pub workspace: Workspace,
    pub detail: String,
    pub assembly_3d: bool,
    pub footprints_visible: bool,
    pub on_view_mode: EventHandler<bool>,
    pub on_toggle_footprints: EventHandler<()>,
}

#[component]
pub fn DesignViewToolbar(props: DesignViewToolbarProps) -> Element {
    let label = props.workspace.as_str();
    let detail = props.detail;
    let assembly_3d = props.assembly_3d;
    let footprints_visible = props.footprints_visible;
    rsx! {
        div {
            class: "absolute top-4 left-1/2 z-[1] flex min-h-10 w-fit max-w-[calc(100%-32px)] -translate-x-1/2 items-center gap-2 rounded-2xl border border-border-strong bg-card px-2.5 py-1.5",
            role: "group",
            aria_label: "{label} commands",
            if let Some(tools) = props.contextual_tools { {tools} } else {
            div { class: "flex items-center gap-3 p-0 [&_strong]:text-xs [&_strong]:text-foreground [&_span]:p-0 [&_span]:text-xs [&_span]:text-muted-foreground",
                svg {
                    class: "size-[17px] flex-none max-[760px]:w-3.5",
                    view_box: "0 0 20 20",
                    fill: "none",
                    stroke: "currentColor",
                    stroke_width: "1.5",
                    stroke_linecap: "round",
                    stroke_linejoin: "round",
                    "aria-hidden": "true",
                    if props.workspace == Workspace::Keymap { path { d: "M3 5h14v10H3zM6 8h2m2 0h2m2 0h1M6 11h2m2 0h2M6 14h8" } }
                    else { path { d: "m3 15 2-10h10l2 10zM5 5l2 4h6l2-4M7 9l-1 6m7-6 1 6" } }
                }
                strong { "{label}" }
                span { "{detail}" }
            }
            }
        }
        div { class: "absolute bottom-3 left-3 z-[11] flex gap-0.5 rounded-lg border border-border bg-card p-[3px] max-[760px]:left-2",
            ToggleGroup {
                aria_label: "View mode",
                pressed: Some(if assembly_3d { [1].into_iter().collect() } else { [0].into_iter().collect() }),
                on_pressed_change: move |next: std::collections::HashSet<usize>| {
                    if next.contains(&1) {
                        props.on_view_mode.call(true);
                    } else if next.contains(&0) {
                        props.on_view_mode.call(false);
                    }
                },
                ToggleItem { index: 0usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "2D" }
                ToggleItem { index: 1usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "3D assembly" }
            }
            if !assembly_3d {
                ToggleGroup {
                    aria_label: "Footprints",
                    pressed: Some(if footprints_visible { [0].into_iter().collect() } else { Default::default() }),
                    on_pressed_change: move |next: std::collections::HashSet<usize>| {
                        if next.contains(&0) != footprints_visible {
                            props.on_toggle_footprints.call(());
                        }
                    },
                    ToggleItem { index: 0usize, class: "max-[960px]:min-h-11 [@media(pointer:coarse)]:min-h-11", "Footprints" }
                }
            }
        }
    }
}

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
pub struct ScopedViewerSignal {
    pub identity: ViewerIdentity,
    pub kind: ViewerSignalKind,
    owner: Weak<ViewerOwner>,
}

impl ScopedViewerSignal {
    /// Check the live viewer owner, not merely the identity copied into this event.
    pub fn is_current(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            owner.active.get() && owner.identity.borrow().as_ref() == Some(&self.identity)
        })
    }
}

#[derive(Clone, Debug)]
pub struct ScopedDisplayChange {
    pub identity: ViewerIdentity,
    pub display: CaseDisplay,
    owner: Weak<ViewerOwner>,
}

impl ScopedDisplayChange {
    pub fn is_current(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            owner.active.get() && owner.identity.borrow().as_ref() == Some(&self.identity)
        })
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
    _source: ViewerSource,
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
    scene: Option<Rc<CadScene>>,
    preview: Option<Rc<NativePreviewSnapshot>>,
    layout_preview: Option<Rc<LayoutPreviewSnapshot>>,
    keycaps_preview: Option<crate::runtime::KeycapsCadPreview>,
    parts_preview: Option<Rc<crate::parts_preview::PartsPreviewSnapshot>>,
    model_rows: Option<ModelDeliveryRows>,
    selected_layer: String,
    selected_reference: Option<String>,
    display: CaseDisplay,
    resolved_theme: String,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
    mechanical_settings: Option<super::MechanicalSettingsProps>,
    #[props(default)] inline_case_controls: bool,
    #[props(default)] toolbar_leading: Option<Element>,
    #[props(default)] handle_source: Option<Rc<CadScene>>,
    #[props(default)] handles: Vec<ViewerHandle>,
    #[props(default)] handle_preview: Option<Vec<ViewerHandle>>,
    #[props(default)] gesture_message: Option<String>,
    #[props(default)] focus_request: Option<ViewerFocusRequest>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let theme = resolved_theme;
    let mut selectable_layers = vec!["pcb".to_owned()];
    if let Some(scene) = scene.as_ref() {
        if let Some(mechanical) = scene.mechanical.as_ref() {
            selectable_layers.extend(mechanical.stack.iter().map(|layer| layer.id.clone()));
            if !mechanical.gasket_supports.is_empty() {
                selectable_layers.push("gaskets".to_owned());
            }
        }
        selectable_layers.extend(
            scene
                .result
                .bodies
                .iter()
                .filter(|body| body.id.starts_with("gasket:"))
                .map(|body| body.id.clone()),
        );
    }
    let owner = use_hook(|| Rc::new(ViewerOwner::default()));
    let projection_cache = use_hook(|| Rc::new(RefCell::new(None::<ProjectionCache>)));
    let Some(source) = scene
        .map(ViewerSource::Cad)
        .or_else(|| preview.clone().map(ViewerSource::Native))
        .or_else(|| layout_preview.clone().map(ViewerSource::Layout))
        .or_else(|| parts_preview.clone().map(ViewerSource::Parts))
    else {
        return rsx! { p { role: "alert", "3D preview source is unavailable." } };
    };
    if match &source {
        ViewerSource::Layout(preview) => !preview.lease.matches(&preview.owner),
        ViewerSource::Parts(preview) => !preview.lease.matches(&preview.owner),
        _ => false,
    } {
        return rsx! { p { role: "alert", "3D preview source is no longer current." } };
    }
    let (source_scope, source_token, _) = source.identity_parts();
    let matching_preview = preview.as_ref().filter(|preview| {
        preview.owner.scope == *source_scope && preview.owner.snapshot_token == source_token
    });
    let matching_model_rows = match &source {
        ViewerSource::Cad(_) | ViewerSource::Native(_) => matching_preview.and(model_rows.as_ref()),
        ViewerSource::Layout(_) => model_rows.as_ref(),
        ViewerSource::Parts(preview) => preview.model_rows.as_ref(),
    };
    let inputs = ProjectionInputs {
        source: source.pointer(),
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
    // Case supplies the accepted scene independently of its disposable display
    // scene. Only a currently admitted preview may carry that gesture source.
    let handle_projection = handle_source
        .filter(|accepted| {
            runtime
                .cad_scene()
                .as_ref()
                .is_some_and(|current| Rc::ptr_eq(current, accepted))
                && matches!(&source, ViewerSource::Cad(displayed)
                    if Rc::ptr_eq(displayed, accepted)
                        || runtime.case_gesture_preview_scene(accepted).as_ref()
                            .is_some_and(|preview| Rc::ptr_eq(displayed, preview)))
        })
        .map(|source| CaseHandleProjection {
            source,
            inputs: inputs.clone(),
        });
    let projection = match projection_cache.borrow().as_ref().filter(|cache| {
        cache.inputs == inputs
            && owner.identity.borrow().as_ref() == Some(&cache.projection.identity)
    }) {
        Some(cache) => Ok(cache.projection.clone()),
        None => owner.next(source.identity_parts()).and_then(|identity| {
            project_source(
                &source,
                identity,
                &theme,
                matching_preview.map(Rc::as_ref),
                matching_model_rows,
                keycaps_preview.as_ref(),
            )
            .map(Rc::new)
        }),
    };
    let projection = match projection {
        Ok(projection) => projection,
        Err(error) => {
            projection_cache.borrow_mut().take();
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
    if projection_cache
        .borrow()
        .as_ref()
        .is_none_or(|cache| cache.projection.identity != projection.identity)
    {
        *projection_cache.borrow_mut() = Some(ProjectionCache {
            _source: source.clone(),
            _preview: matching_preview.cloned(),
            _models: matching_model_rows.cloned(),
            inputs,
            projection: projection.clone(),
        });
    }
    let current_source = SourceGuard({
        let runtime = runtime.clone();
        let source = source.clone();
        let keycaps_preview = keycaps_preview.clone();
        Rc::new(move || {
            runtime.scope().as_ref() == Some(source.identity_parts().0)
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
    let stack = match &source {
        ViewerSource::Cad(scene) => scene
            .mechanical
            .as_ref()
            .map(|assembly| assembly.stack.as_slice())
            .unwrap_or_default(),
        _ => &[],
    };
    let assembly_layers =
        assembly_layers_with_stack(generated_layers, stack.iter().map(|layer| layer.id.clone()));
    let models = match &source {
        ViewerSource::Layout(preview) => preview.preview.models.as_slice(),
        ViewerSource::Parts(preview) => preview.preview.models.as_slice(),
        _ => matching_preview
            .map(|preview| preview.preview.models.as_slice())
            .unwrap_or_default(),
    };
    let component_layers = physical_component_layers(models, matching_model_rows);
    let current_workspace = use_context::<super::WorkspaceState>().0;
    let canvas_context = match &source {
        ViewerSource::Layout(_) => match current_workspace() {
            Workspace::Keymap => ViewerCanvasContext::Keymap,
            Workspace::Keycaps => ViewerCanvasContext::Keycaps,
            _ => ViewerCanvasContext::Layout,
        },
        ViewerSource::Cad(_) | ViewerSource::Native(_) => ViewerCanvasContext::Case,
        ViewerSource::Parts(_) => ViewerCanvasContext::Parts,
    };
    let module_model_errors = matching_model_rows
        .into_iter()
        .flat_map(|rows| rows.failures.iter())
        .filter(|failure| failure.reference.starts_with("module-model/"))
        .map(|failure| format!("{}: {}", failure.reference, failure.reason))
        .collect::<Vec<_>>();
    let gesture_message = if module_model_errors.is_empty() {
        gesture_message
    } else {
        Some(match gesture_message {
            Some(existing) if !existing.is_empty() => {
                format!(
                    "{existing} · Mounted-module model: {}",
                    module_model_errors.join("; ")
                )
            }
            _ => format!("Mounted-module model: {}", module_model_errors.join("; ")),
        })
    };
    rsx! { MountedCanvas {
            projection,
            assembly_layers,
            component_layers,
            selectable_layers,
            canvas_context,
            selected_layer,
            selected_reference,
            display,
            theme,
            owner,
            current_source,
            on_signal,
            on_display_change,
            mechanical_settings,
            inline_case_controls,
            toolbar_leading,
            handle_projection,
            handles,
            handle_preview,
            gesture_message,
            focus_request,
    } }
}

#[derive(Clone)]
enum ViewerSource {
    Cad(Rc<CadScene>),
    Native(Rc<NativePreviewSnapshot>),
    Layout(Rc<LayoutPreviewSnapshot>),
    Parts(Rc<crate::parts_preview::PartsPreviewSnapshot>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ViewerCanvasContext {
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

    fn fit_label(self) -> &'static str {
        match self {
            Self::Parts => "Fit sample",
            Self::Case => "Fit case",
            Self::Layout => "Fit layout",
            Self::Keymap => "Fit Keymap",
            Self::Keycaps => "Fit Keycaps",
        }
    }

    fn camera_label(self) -> &'static str {
        match self {
            Self::Parts => "Parts sample camera",
            Self::Case => "Case camera",
            Self::Layout => "Layout camera",
            Self::Keymap => "Keymap camera",
            Self::Keycaps => "Keycaps camera",
        }
    }

    fn display_label(self) -> &'static str {
        match self {
            Self::Parts => "Parts sample display mode",
            Self::Case => "Case display mode",
            Self::Layout => "Layout display mode",
            Self::Keymap => "Keymap display mode",
            Self::Keycaps => "Keycaps display mode",
        }
    }

    fn assembly_label(self) -> &'static str {
        match self {
            Self::Parts => "Parts sample view",
            Self::Case => "Case assembly view",
            Self::Layout => "Layout assembly view",
            Self::Keymap => "Keymap assembly view",
            Self::Keycaps => "Keycaps assembly view",
        }
    }
}

impl ViewerSource {
    fn pointer(&self) -> usize {
        match self {
            Self::Cad(scene) => Rc::as_ptr(scene) as usize,
            Self::Native(preview) => Rc::as_ptr(preview) as usize,
            Self::Layout(preview) => Rc::as_ptr(preview) as usize,
            Self::Parts(preview) => Rc::as_ptr(preview) as usize,
        }
    }

    fn identity_parts(&self) -> (&Scope, SnapshotToken, u64) {
        match self {
            Self::Cad(scene) => (&scene.scope, scene.token, scene.snapshot.document.revision),
            Self::Native(preview) => (
                &preview.owner.scope,
                preview.owner.snapshot_token,
                preview.accepted_document.revision,
            ),
            Self::Layout(preview) => (
                &preview.owner.scope,
                preview.owner.snapshot_token,
                preview.document.revision,
            ),
            Self::Parts(preview) => (
                &preview.owner.scope,
                preview.owner.snapshot_token,
                preview.sample_document.revision,
            ),
        }
    }

    fn is_current(&self, runtime: &crate::runtime::Runtime) -> bool {
        match self {
            Self::Cad(scene) => {
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
            }
            Self::Native(preview) => runtime.native_case_preview().is_some_and(|current| {
                current.owner == preview.owner && Rc::ptr_eq(&current.lease, &preview.lease)
            }),
            Self::Layout(preview) => runtime.layout_preview().is_some_and(|current| {
                current.owner == preview.owner
                    && Rc::ptr_eq(&current.lease, &preview.lease)
                    && preview.lease.matches(&preview.owner)
            }),
            Self::Parts(preview) => {
                preview.lease.matches(&preview.owner)
                    && runtime.parts_preview_snapshot_is_current(preview)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ViewerHandle {
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
    pub target: ViewerHandleTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ViewerHandleTarget {
    Gasket {
        support_id: String,
    },
    Mount {
        owner: CaseMountTarget,
        mount_id: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub enum CaseMountTarget {
    Mechanical(super::mechanical_settings::MechanicalMountCollection),
    Authored { body_id: String },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum AssemblyView {
    Assembled,
    Exploded,
    Section,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
enum RenderMode {
    Shaded,
    Wireframe,
    Hybrid,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TransientView {
    assembly: AssemblyView,
    mode: RenderMode,
    explode_amount: f32,
    section_plane: &'static str,
    section_position: f32,
    show_section_plane: bool,
    show_hidden: bool,
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

#[derive(Clone)]
enum PointerOwner {
    Orbit {
        identity: ViewerIdentity,
        pointer_id: i32,
        last: (f64, f64),
        start: (f64, f64),
        moved: bool,
        picked: Option<String>,
    },
    Handle {
        identity: ViewerIdentity,
        projection: Option<CaseHandleProjection>,
        pointer_id: i32,
        z: f32,
        id: String,
    },
}

impl PointerOwner {
    fn identity(&self) -> &ViewerIdentity {
        match self {
            Self::Orbit { identity, .. } | Self::Handle { identity, .. } => identity,
        }
    }

    fn pointer_id(&self) -> i32 {
        match self {
            Self::Orbit { pointer_id, .. } | Self::Handle { pointer_id, .. } => *pointer_id,
        }
    }
}

fn viewer_control_icon(label: &str) -> Element {
    let path = match label {
        "Fit" => "M7 2H2v5M13 2h5v5M2 13v5h5M18 13v5h-5M6 6h8v8H6z",
        "Top" => "M3 5h14v11H3zM3 5l7-3 7 3M7 8h6",
        "Bottom" => "M3 4h14v11H3zM3 15l7 3 7-3M7 12h6",
        "Shaded" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8",
        "Wireframe" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M10 2v8M3 14l7-4 7 4",
        "Hybrid" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M13 8v8",
        "Assembled" => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8M3 10l7 4 7-4",
        "Exploded" => "m3 5 7-3 7 3-7 3ZM3 10l7 3 7-3M3 15l7 3 7-3",
        "Section" => "M3 3h14v14H3zM10 1v18M12 5l3 3M12 10l3 3",
        "Edit gaskets" => "M5 3h10l3 4v6l-3 4H5l-3-4V7ZM6 7h8v6H6z",
        "Edit mounts" => {
            "M10 2v4M10 14v4M2 10h4M14 10h4M15 10a5 5 0 1 1-10 0 5 5 0 0 1 10 0ZM12 10a2 2 0 1 1-4 0 2 2 0 0 1 4 0Z"
        }
        "View controls" => "M2 5h16M2 10h16M2 15h16M6 3v4M14 8v4M8 13v4",
        _ => "m10 2 7 4v8l-7 4-7-4V6ZM3 6l7 4 7-4M10 10v8",
    };
    rsx! {
        svg { view_box: "0 0 20 20", "aria-hidden": "true", path { d: path } }
        span { class: "sr-only", "{label}" }
    }
}

fn owner_is_current(owner: &ViewerOwner, identity: &ViewerIdentity) -> bool {
    owner.active.get() && owner.identity.borrow().as_ref() == Some(identity)
}

fn focus_request_matches(request: &ViewerFocusRequest, identity: &ViewerIdentity) -> bool {
    request.scope == identity.scope
        && request.snapshot_token == identity.snapshot_token
        && request.revision == identity.revision
        && !request.target_ids.is_empty()
}

fn take_pointer_for_event(
    pointer: &Rc<RefCell<Option<PointerOwner>>>,
    pointer_id: i32,
    identity: &ViewerIdentity,
) -> Option<PointerOwner> {
    let mut active = pointer.borrow_mut();
    if active
        .as_ref()
        .is_some_and(|active| active.pointer_id() == pointer_id && active.identity() == identity)
    {
        active.take()
    } else {
        None
    }
}

fn cancel_superseded_pointer(
    pointer: &Rc<RefCell<Option<PointerOwner>>>,
    canvas: &Rc<RefCell<Option<HtmlCanvasElement>>>,
    owner: &Rc<ViewerOwner>,
    handle_projection: Option<&CaseHandleProjection>,
    source_current: bool,
) {
    let stale_pointer = {
        let mut active = pointer.borrow_mut();
        let current_identity = owner.identity.borrow();
        if owner.active.get()
            && source_current
            && let Some(current) = current_identity.as_ref()
            && let Some(PointerOwner::Handle {
                identity,
                projection: Some(previous),
                ..
            }) = active.as_mut()
            && let Some(next) = handle_projection
            && identity.scope == current.scope
            && identity.snapshot_token == current.snapshot_token
            && identity.revision == current.revision
            && previous.continues_preview(next)
        {
            // Transfer this gesture to the new render; event admission remains
            // full-identity checked, so callbacks from the old render stay stale.
            *identity = current.clone();
            *previous = next.clone();
        }
        if active.as_ref().is_some_and(|active| {
            !owner.active.get()
                || (matches!(active, PointerOwner::Handle { .. }) && !source_current)
                || current_identity
                    .as_ref()
                    .is_none_or(|identity| active.identity() != identity)
        }) {
            active.take().map(|active| active.pointer_id())
        } else {
            None
        }
    };
    if let Some(pointer_id) = stale_pointer {
        release_pointer_capture(canvas, pointer_id);
    }
}

fn release_capture(
    pointer: &RefCell<Option<PointerOwner>>,
    canvas: &Rc<RefCell<Option<HtmlCanvasElement>>>,
) -> Option<PointerOwner> {
    let captured = pointer.borrow_mut().take();
    if let Some(captured) = &captured {
        release_pointer_capture(canvas, captured.pointer_id());
    }
    captured
}

fn release_pointer_capture(canvas: &Rc<RefCell<Option<HtmlCanvasElement>>>, pointer_id: i32) {
    if let Some(canvas) = canvas.borrow().as_ref() {
        let _ = canvas.release_pointer_capture(pointer_id);
    }
}

fn emit_signal(
    owner: &Rc<ViewerOwner>,
    on_signal: EventHandler<ScopedViewerSignal>,
    identity: &ViewerIdentity,
    kind: ViewerSignalKind,
) {
    if !owner_is_current(owner, identity) {
        return;
    }
    on_signal.call(ScopedViewerSignal {
        identity: identity.clone(),
        kind,
        owner: Rc::downgrade(owner),
    });
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
        "view": transient.assembly,
        "mode": transient.mode,
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

fn active_case_handles(
    handles: &[ViewerHandle],
    preview: Option<&[ViewerHandle]>,
    edit_gaskets: bool,
    edit_mounts: bool,
) -> Vec<ViewerHandle> {
    if !edit_gaskets && !edit_mounts {
        return Vec::new();
    }
    preview
        .unwrap_or(handles)
        .iter()
        .filter(|handle| {
            matches!(&handle.target, ViewerHandleTarget::Gasket { .. }) && edit_gaskets
                || matches!(&handle.target, ViewerHandleTarget::Mount { .. }) && edit_mounts
        })
        .cloned()
        .collect()
}

fn handles_value(handles: &[ViewerHandle]) -> Result<JsValue, String> {
    let handles = handles
        .iter()
        .map(|handle| {
            serde_json::json!({
                "id": &handle.id,
                "at": { "x": handle.x, "y": handle.y },
                "tangent": { "x": handle.tangent_x, "y": handle.tangent_y },
                "normal": { "x": handle.normal_x, "y": handle.normal_y },
                "length": handle.length,
                "z": handle.z,
                "invalid": handle.invalid,
            })
        })
        .collect::<Vec<_>>();
    js_sys::JSON::parse(&serde_json::Value::Array(handles).to_string()).map_err(js_error)
}

fn pointer_point(event: &PointerEvent) -> (f64, f64) {
    (f64::from(event.client_x()), f64::from(event.client_y()))
}

fn run_host(
    host: &Rc<RefCell<Option<Rc<RendererPageHost>>>>,
    operation: impl FnOnce(&RendererPageHost) -> Result<(), String>,
    status: &mut Signal<String>,
) {
    if let Some(host) = host.borrow().as_ref()
        && let Err(error) = operation(host)
    {
        status.set(error);
    }
}
