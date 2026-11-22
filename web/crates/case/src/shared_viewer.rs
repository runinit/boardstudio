//! Shared 3D viewport for immutable scene projections from each workspace.

mod canvas;
mod case;
pub(crate) use case::CaseViewport;
pub use case::{
    BodySelection, CaseBodyEditDispatch, CaseBodyEditPortal, CaseSelection, LayerSelection,
};
mod interaction;
mod mount;
mod scene;

use canvas::*;
use scene::*;

// Layout and Parts use this name for the same component.
pub use Viewport3D as CaseSharedViewer;

use super::case_assembly_layers::{
    CaseAssemblyLayers, assembly_layers_with_stack, physical_component_layers,
};
use super::layout_viewer_source::LayoutPreviewSnapshot;
use super::mechanical_settings::MechanicalSettingsPatchExt;
use super::model_delivery::ModelDeliveryRows;
use crate::case_model_lifecycle::ProjectionInputs;
use crate::case_preview::NativePreviewSnapshot;
use crate::runtime::CadScene;
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
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlCanvasElement, PointerEvent};

#[cfg(any(test, feature = "test-support"))]
#[derive(Clone)]
pub struct SharedViewerProjectionProbe(pub Rc<dyn Fn(ViewerIdentity, JsValue)>);

#[cfg(any(test, feature = "test-support"))]
thread_local! {
    static NEXT_RENDERER_MOUNT_ERROR: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Inject one host initialization error through SharedViewer's real mount-error path.
/// RendererHost's scripted lifecycle tests separately cover initialization cleanup.
#[cfg(any(test, feature = "test-support"))]
pub struct RendererMountErrorGuard;

#[cfg(any(test, feature = "test-support"))]
impl Drop for RendererMountErrorGuard {
    fn drop(&mut self) {
        NEXT_RENDERER_MOUNT_ERROR.with(|error| error.borrow_mut().take());
    }
}

#[cfg(any(test, feature = "test-support"))]
pub fn fail_next_renderer_mount_for_test(message: impl Into<String>) -> RendererMountErrorGuard {
    NEXT_RENDERER_MOUNT_ERROR.with(|error| {
        assert!(error.borrow_mut().replace(message.into()).is_none());
    });
    RendererMountErrorGuard
}

#[cfg(any(test, feature = "test-support"))]
fn take_renderer_mount_error_for_test() -> Option<String> {
    NEXT_RENDERER_MOUNT_ERROR.with(|error| error.borrow_mut().take())
}

pub use super::case_display::CaseDisplay;
use super::case_display::preference_ids;
use super::case_display::{ComponentModelSource, component_models_for_source};

/// Reusable canvas header and view switch for the Keymap and Keycaps consumers.
/// The page owner supplies guarded actions and the accepted shared view state.
#[derive(Props, Clone, PartialEq)]
pub struct DesignViewToolbarProps {
    #[props(default)]
    pub contextual_tools: Option<Element>,
    pub label: String,
    pub detail: String,
    pub assembly_3d: bool,
    pub footprints_visible: bool,
    pub on_view_mode: EventHandler<bool>,
    pub on_toggle_footprints: EventHandler<()>,
}

#[component]
pub fn DesignViewToolbar(props: DesignViewToolbarProps) -> Element {
    let label = props.label;
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
                    if label == "Keymap" { path { d: "M3 5h14v10H3zM6 8h2m2 0h2m2 0h1M6 11h2m2 0h2M6 14h8" } }
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
    pub viewer_instance: u64,
    pub projection_generation: u64,
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
pub enum ViewerLifecycle {
    Starting,
    Ready,
    Failed,
    ContextLost,
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
    SceneAccepted(bool),
    Lifecycle(ViewerLifecycle),
    Failed(String),
    Picked(String),
    LayerSelected(String),
    WorldPoint(Option<[f32; 3]>),
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

struct ViewerOwner {
    active: Cell<bool>,
    identity: RefCell<Option<ViewerIdentity>>,
    last_inputs: RefCell<Option<ProjectionInputs>>,
    projection_generation: Cell<u64>,
    viewer_instance: u64,
}

impl PartialEq for ViewerOwner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for ViewerOwner {}

impl ViewerOwner {
    fn new() -> Result<Rc<Self>, String> {
        Ok(Rc::new(Self {
            active: Cell::new(true),
            identity: RefCell::new(None),
            last_inputs: RefCell::new(None),
            projection_generation: Cell::new(0),
            viewer_instance: next_viewer_instance()?,
        }))
    }

    fn advance_source(
        &self,
        scope: Scope,
        token: SnapshotToken,
        revision: u64,
        inputs: &ProjectionInputs,
    ) -> Result<ViewerIdentity, String> {
        let changed = self.last_inputs.borrow().as_ref() != Some(inputs);
        if changed {
            let generation = self
                .projection_generation
                .get()
                .checked_add(1)
                .filter(|generation| *generation <= 9_007_199_254_740_991)
                .ok_or_else(|| "Renderer scene sequence exhausted".to_owned())?;
            self.projection_generation.set(generation);
            *self.last_inputs.borrow_mut() = Some(inputs.clone());
        }
        let identity = ViewerIdentity {
            scope,
            snapshot_token: token,
            revision,
            viewer_instance: self.viewer_instance,
            projection_generation: self.projection_generation.get(),
            renderer_sequence: self.projection_generation.get(),
        };
        *self.identity.borrow_mut() = Some(identity.clone());
        Ok(identity)
    }
}

thread_local! {
    static NEXT_VIEWER_INSTANCE: Cell<u64> = const { Cell::new(1) };
}

fn next_viewer_instance() -> Result<u64, String> {
    NEXT_VIEWER_INSTANCE.with(|next| {
        let id = next.get();
        let next_id = id
            .checked_add(1)
            .ok_or_else(|| "Viewer instance identity exhausted".to_owned())?;
        next.set(next_id);
        Ok(id)
    })
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
    let owner = match use_hook(ViewerOwner::new) {
        Ok(owner) => owner,
        Err(error) => {
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
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
        ViewerSource::Layout(preview) if preview.lease.matches(&preview.owner) => {
            model_rows.as_ref()
        }
        ViewerSource::Parts(preview) if preview.lease.matches(&preview.owner) => {
            preview.model_rows.as_ref()
        }
        _ => None,
    };
    let preview_key = (
        matching_preview.map_or(0, |preview| Rc::as_ptr(preview) as usize),
        keycaps_preview.as_ref().map(|preview| preview.generation),
    );
    let preview_identity = use_hook(|| Rc::new(RefCell::new((None, 0_usize))));
    let preview_generation = {
        let mut identity = preview_identity.borrow_mut();
        if identity.0 != Some(preview_key) {
            let Some(next) = identity.1.checked_add(1) else {
                return rsx! { p { role: "alert", "3D preview identity exhausted" } };
            };
            *identity = (Some(preview_key), next);
        }
        identity.1
    };
    let inputs = ProjectionInputs {
        source: source.pointer(),
        preview: preview_generation,
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
    let identity = match source.identity(&owner, &inputs) {
        Ok(identity) => identity,
        Err(error) => {
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
    let projection_cache = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(
                ViewerSource,
                Option<Rc<NativePreviewSnapshot>>,
                ProjectionInputs,
                Rc<RendererSceneProjection>,
            )>,
        ))
    });
    let projection = if let Some((cached_source, _, cached_inputs, projection)) =
        projection_cache.borrow().as_ref()
        && cached_source.same(&source)
        && cached_inputs == &inputs
    {
        projection.clone()
    } else {
        match project_source(
            &source,
            identity.clone(),
            &theme,
            matching_preview.map(Rc::as_ref),
            matching_model_rows,
            keycaps_preview.as_ref(),
        ) {
            Ok(projection) => {
                let projection = Rc::new(projection);
                *projection_cache.borrow_mut() = Some((
                    source.clone(),
                    preview.clone(),
                    inputs.clone(),
                    projection.clone(),
                ));
                projection
            }
            Err(error) => {
                return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
            }
        }
    };
    #[cfg(any(test, feature = "test-support"))]
    if let Some(probe) = try_consume_context::<SharedViewerProjectionProbe>()
        && source.is_current(&runtime)
    {
        (probe.0)(projection.identity.clone(), projection.input.clone());
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
    let configured_stack_ids = match &source {
        ViewerSource::Cad(scene)
            if scene.scope == identity.scope && scene.token == identity.snapshot_token =>
        {
            scene
                .mechanical
                .as_ref()
                .map(|mechanical| {
                    mechanical
                        .stack
                        .iter()
                        .map(|layer| layer.id.clone())
                        .collect()
                })
                .unwrap_or_default()
        }
        ViewerSource::Cad(_)
        | ViewerSource::Native(_)
        | ViewerSource::Layout(_)
        | ViewerSource::Parts(_) => Vec::new(),
    };
    let assembly_layers = assembly_layers_with_stack(generated_layers, configured_stack_ids);
    let (component_model_source, layout_models, parts_models) = match &source {
        ViewerSource::Layout(preview) => (
            ComponentModelSource::Layout,
            preview
                .lease
                .matches(&preview.owner)
                .then_some(preview.preview.models.as_slice()),
            None,
        ),
        ViewerSource::Parts(preview) => (
            ComponentModelSource::Parts,
            None,
            preview
                .lease
                .matches(&preview.owner)
                .then_some(preview.preview.models.as_slice()),
        ),
        ViewerSource::Cad(_) | ViewerSource::Native(_) => {
            (ComponentModelSource::Physical, None, None)
        }
    };
    let physical_models = matching_preview.map(|preview| preview.preview.models.as_slice());
    let component_models = component_models_for_source(
        component_model_source,
        layout_models,
        physical_models,
        parts_models,
    );
    let component_layers = component_models.map_or_else(Vec::new, |models| {
        physical_component_layers(models, matching_model_rows)
    });
    let current_workspace = use_context::<super::WorkspaceState>().0;
    let canvas_context = match &source {
        ViewerSource::Layout(_) => match current_workspace() {
            "Keymap" => ViewerCanvasContext::Keymap,
            "Keycaps" => ViewerCanvasContext::Keycaps,
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
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cad(left), Self::Cad(right)) => Rc::ptr_eq(left, right),
            (Self::Native(left), Self::Native(right)) => Rc::ptr_eq(left, right),
            (Self::Layout(left), Self::Layout(right)) => Rc::ptr_eq(left, right),
            (Self::Parts(left), Self::Parts(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }

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
                preview.owner.accepted_revision,
            ),
            Self::Layout(preview) => (
                &preview.owner.scope,
                preview.owner.snapshot_token,
                preview.owner.accepted_revision,
            ),
            Self::Parts(preview) => (
                &preview.owner.scope,
                preview.owner.snapshot_token,
                preview.owner.accepted_revision,
            ),
        }
    }

    fn identity(
        &self,
        owner: &ViewerOwner,
        inputs: &ProjectionInputs,
    ) -> Result<ViewerIdentity, String> {
        let (scope, token, revision) = self.identity_parts();
        owner.advance_source(scope.clone(), token, revision, inputs)
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
        collection: super::mechanical_settings::MechanicalMountCollection,
        mount_id: String,
    },
    AuthoredMount {
        body_id: String,
        mount_id: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssemblyView {
    Assembled,
    Exploded,
    Section,
}

impl AssemblyView {
    fn renderer_value(self) -> &'static str {
        match self {
            Self::Assembled => "assembled",
            Self::Exploded => "exploded",
            Self::Section => "section",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RenderMode {
    Shaded,
    Wireframe,
    Hybrid,
}

impl RenderMode {
    fn renderer_value(self) -> &'static str {
        match self {
            Self::Shaded => "shaded",
            Self::Wireframe => "wireframe",
            Self::Hybrid => "hybrid",
        }
    }
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
        Rc::ptr_eq(&self.source, &next.source)
            && self.inputs.source != next.inputs.source
            && self.inputs.preview == next.inputs.preview
            && self.inputs.models == next.inputs.models
            && self.inputs.theme == next.inputs.theme
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

#[cfg(all(test, target_arch = "wasm32"))]
type ArmHandleCapture = Rc<dyn Fn(String)>;

#[cfg(all(test, target_arch = "wasm32"))]
#[derive(Clone, Default)]
pub struct SharedViewerEscapeProbe(Rc<RefCell<Option<ArmHandleCapture>>>);

#[cfg(all(test, target_arch = "wasm32"))]
impl SharedViewerEscapeProbe {
    pub fn arm_handle_capture(&self, handle_id: impl Into<String>) {
        let arm = self
            .0
            .borrow()
            .clone()
            .expect("mounted SharedViewer publishes its Escape test probe");
        arm(handle_id.into());
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
            && identity.viewer_instance == current.viewer_instance
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

#[cfg(all(any(test, feature = "test-support"), target_arch = "wasm32"))]
pub fn emit_handle_gesture_for_test(
    on_signal: EventHandler<ScopedViewerSignal>,
    identity: &ViewerIdentity,
    phase: HandleGesturePhase,
    handle_id: String,
    point: Option<[f32; 3]>,
) {
    let owner = ViewerOwner::new().expect("test viewer owner identity is available");
    *owner.identity.borrow_mut() = Some(identity.clone());
    emit_signal(
        &owner,
        on_signal,
        identity,
        ViewerSignalKind::HandleGesture {
            phase,
            handle_id,
            point,
        },
    );
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
        "view": transient.assembly.renderer_value(),
        "mode": transient.mode.renderer_value(),
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
                || matches!(
                    &handle.target,
                    ViewerHandleTarget::Mount { .. } | ViewerHandleTarget::AuthoredMount { .. }
                ) && edit_mounts
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

#[cfg(all(test, target_arch = "wasm32"))]
#[path = "shared_viewer_020_tests.rs"]
mod rust_ui_020_tests;

#[cfg(all(test, target_arch = "wasm32"))]
mod projection_tests;
