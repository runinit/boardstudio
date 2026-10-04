//! Private page viewer shared by workflow-owned, immutable scene projections.
//!
//! Case is the first consumer. Its wrapper owns the Case-to-renderer projection;
//! this module owns only renderer controls, transient view state and host lifetime.
use super::case_assembly_layers::{
    CaseAssemblyLayers, assembly_layers_with_stack, physical_component_layers,
};
use super::layout_viewer_source::LayoutPreviewSnapshot;
use super::model_delivery::ModelDeliveryRows;
use crate::case_model_lifecycle::ProjectionInputs;
use crate::case_preview::NativePreviewSnapshot;
use crate::renderer_host_page::RendererPageHost;
use crate::runtime::CadScene;
use boardstudio_application::{Scope, SnapshotToken};
use boardstudio_web::cad_jobs::captured_case_document;
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

pub(crate) use super::case_display::CaseDisplay;
use super::case_display::preference_ids;
use super::case_display::{ComponentModelSource, component_models_for_source};

/// Reusable canvas header and view switch for the Keymap and Keycaps consumers.
/// The page owner supplies guarded actions and the accepted shared view state.
#[derive(Props, Clone, PartialEq)]
pub(crate) struct DesignViewToolbarProps {
    pub(crate) label: String,
    pub(crate) detail: String,
    pub(crate) assembly_3d: bool,
    pub(crate) footprints_visible: bool,
    pub(crate) on_view_mode: EventHandler<bool>,
    pub(crate) on_toggle_footprints: EventHandler<()>,
}

#[component]
pub(crate) fn DesignViewToolbar(props: DesignViewToolbarProps) -> Element {
    let label = props.label;
    let detail = props.detail;
    let assembly_3d = props.assembly_3d;
    let footprints_visible = props.footprints_visible;
    rsx! {
        div {
            class: "m1-canvas-toolbar m1-design-canvas-toolbar",
            role: "toolbar",
            aria_label: "{label} commands",
            div { class: "m1-canvas-context",
                svg {
                    class: "m1-tab-icon",
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
        div { class: "m1-design-view-group", role: "group", aria_label: "Design view",
            button {
                r#type: "button",
                aria_pressed: "{!assembly_3d}",
                onclick: move |_| props.on_view_mode.call(false),
                "2D"
            }
            button {
                r#type: "button",
                aria_pressed: "{assembly_3d}",
                onclick: move |_| props.on_view_mode.call(true),
                "3D assembly"
            }
            if !assembly_3d {
                button {
                    r#type: "button",
                    aria_pressed: "{footprints_visible}",
                    onclick: move |_| props.on_toggle_footprints.call(()),
                    "Footprints"
                }
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViewerIdentity {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) viewer_instance: u64,
    pub(crate) projection_generation: u64,
    pub(crate) renderer_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViewerFocusRequest {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) revision: u64,
    pub(crate) navigation_id: u64,
    pub(crate) target_ids: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ViewerLifecycle {
    Starting,
    Ready,
    Failed,
    ContextLost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HandleGesturePhase {
    Start,
    Move,
    End,
    Cancel,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ViewerSignalKind {
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
pub(crate) struct ScopedViewerSignal {
    pub(crate) identity: ViewerIdentity,
    pub(crate) kind: ViewerSignalKind,
    owner: Weak<ViewerOwner>,
}

impl ScopedViewerSignal {
    /// Check the live viewer owner, not merely the identity copied into this event.
    pub(crate) fn is_current(&self) -> bool {
        self.owner.upgrade().is_some_and(|owner| {
            owner.active.get() && owner.identity.borrow().as_ref() == Some(&self.identity)
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct ScopedDisplayChange {
    pub(crate) identity: ViewerIdentity,
    pub(crate) display: CaseDisplay,
    owner: Weak<ViewerOwner>,
}

impl ScopedDisplayChange {
    pub(crate) fn is_current(&self) -> bool {
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
    renderer_sequence: Cell<u64>,
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
            renderer_sequence: Cell::new(0),
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
                .ok_or_else(|| "Viewer projection identity exhausted".to_owned())?;
            let sequence = self
                .renderer_sequence
                .get()
                .checked_add(1)
                .filter(|sequence| *sequence <= 9_007_199_254_740_991)
                .ok_or_else(|| "Renderer scene sequence exhausted".to_owned())?;
            self.projection_generation.set(generation);
            self.renderer_sequence.set(sequence);
            *self.last_inputs.borrow_mut() = Some(inputs.clone());
        }
        let identity = ViewerIdentity {
            scope,
            snapshot_token: token,
            revision,
            viewer_instance: self.viewer_instance,
            projection_generation: self.projection_generation.get(),
            renderer_sequence: self.renderer_sequence.get(),
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
pub(crate) fn CaseSharedViewer(
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
    let (source_scope, source_token, _) = source.identity_parts();
    let matching_preview = preview
        .as_ref()
        .filter(|preview| preview_matches_source(&preview.owner, source_scope, source_token));
    let matching_model_rows = match &source {
        ViewerSource::Cad(_) | ViewerSource::Native(_) => matching_preview.and(model_rows.as_ref()),
        ViewerSource::Layout(source_preview)
            if layout_preview.as_ref().is_some_and(|input_preview| {
                Rc::ptr_eq(input_preview, source_preview)
                    && input_preview.lease.matches(&input_preview.owner)
            }) =>
        {
            model_rows.as_ref()
        }
        ViewerSource::Layout(_) => None,
        ViewerSource::Parts(preview) if preview.lease.matches(&preview.owner) => {
            preview.model_rows.as_ref()
        }
        ViewerSource::Parts(_) => None,
    };
    let inputs = ProjectionInputs {
        source: source.pointer(),
        preview: keycaps_preview.as_ref().map_or_else(
            || matching_preview.map_or(0, |preview| Rc::as_ptr(preview) as usize),
            |preview| preview.generation as usize,
        ),
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
    let current_source = SourceGuard({
        let runtime = runtime.clone();
        let source = source.clone();
        let keycaps_preview = keycaps_preview.clone();
        Rc::new(move || {
            source.is_current(&runtime)
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
    rsx! {
        SharedViewer {
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
            handle_projection,
            handles,
            handle_preview,
            gesture_message,
            focus_request,
        }
    }
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

fn project_source(
    source: &ViewerSource,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    match source {
        ViewerSource::Cad(scene) => {
            project_case_scene(scene.clone(), identity, theme, preview, model_rows)
        }
        ViewerSource::Native(preview) => {
            project_native_preview(preview, identity, theme, model_rows)
        }
        ViewerSource::Layout(preview) => {
            project_layout_preview(preview, identity, theme, model_rows, keycaps_preview)
        }
        ViewerSource::Parts(preview) => project_parts_preview(preview, identity, theme, model_rows),
    }
}

fn preview_matches_source(
    preview: &crate::case_preview::CasePreviewOwnerIdentity,
    source_scope: &Scope,
    source_token: SnapshotToken,
) -> bool {
    preview.scope == *source_scope && preview.snapshot_token == source_token
}

fn project_case_scene(
    scene: Rc<CadScene>,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
) -> Result<RendererSceneProjection, String> {
    let document = captured_case_document(&scene.snapshot, &scene.scope)
        .map_err(|error| format!("Case scene projection failed: {error:?}"))?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scene.scope.board_id)
        .ok_or_else(|| "Case board is unavailable in the selected scene".to_owned())?;
    let stack = scene
        .mechanical
        .as_ref()
        .map(|assembly| assembly.stack.as_slice())
        .unwrap_or_default();
    let physical_preview = preview.filter(|preview| {
        preview.owner.scope == scene.scope && preview.owner.snapshot_token == scene.token
    });
    let thickness = physical_preview
        .map(|preview| preview.preview.thickness)
        .unwrap_or(board.thickness);
    let contours = physical_preview
        .map(|preview| preview.contours.as_slice())
        .unwrap_or(scene.contours.as_slice());
    let surfaces = physical_preview
        .map(|preview| preview.preview.surfaces.as_slice())
        .unwrap_or_default();
    let holes = physical_preview
        .map(|preview| preview.preview.holes.as_slice())
        .unwrap_or_default();
    let models = physical_preview
        .map(|preview| preview.preview.models.as_slice())
        .unwrap_or_default();
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": thickness,
            "contours": contours,
            "surfaces": surfaces,
            "holes": holes,
            "models": models
        },
        "models": [],
        "mechanicalStack": stack
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let loaded_models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &loaded_models).map_err(js_error)?;
    let bodies = Array::new();
    for body in &scene.result.bodies {
        let value = Object::new();
        let mesh = Object::new();
        Reflect::set(
            &mesh,
            &"positions".into(),
            &Float32Array::from(body.positions.as_slice()),
        )
        .map_err(js_error)?;
        Reflect::set(
            &mesh,
            &"normals".into(),
            &Float32Array::from(body.normals.as_slice()),
        )
        .map_err(js_error)?;
        Reflect::set(&value, &"id".into(), &body.id.clone().into()).map_err(js_error)?;
        Reflect::set(&value, &"name".into(), &body.name.clone().into()).map_err(js_error)?;
        Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
        bodies.push(&value);
    }
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    for body in &scene.result.bodies {
        if !layers.iter().any(|(id, _)| id == &body.id) {
            layers.push((body.id.clone(), body.name.clone()));
        }
    }
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
}

fn project_native_preview(
    preview: &NativePreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
    {
        return Err("Native PCB preview does not match the active Case viewer owner".into());
    }
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": preview.preview.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers: vec![("pcb".to_owned(), "PCB".to_owned())],
    })
}

fn project_layout_preview(
    preview: &LayoutPreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
        || !preview.lease.matches(&preview.owner)
    {
        return Err("Canonical Layout preview does not match the active viewer owner".into());
    }
    let reference = preview
        .board_reference
        .as_ref()
        .filter(|reference| reference.enabled)
        .map(|reference| {
            serde_json::json!({
                "pose": &reference.pose,
                "elevation": reference.elevation,
            })
        });
    let module_bodies = layout_module_bodies(
        &preview.module_scenes,
        &preview.document,
        &identity.scope.board_id,
        preview.preview.thickness,
    );
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "pcb",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": preview.preview.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "reference": reference,
        "moduleBodies": &module_bodies,
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    let bodies = Array::new();
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    for body in module_bodies {
        layers.push((
            body["id"].as_str().unwrap_or_default().to_owned(),
            body["name"].as_str().unwrap_or_default().to_owned(),
        ));
    }
    let module_model_bodies = layout_module_model_bodies(model_rows, &preview.document);
    for body in module_model_bodies {
        let id = body["id"].as_str().unwrap_or_default().to_owned();
        let name = body["name"].as_str().unwrap_or_default().to_owned();
        let value = js_sys::JSON::parse(&body.to_string()).map_err(js_error)?;
        bodies.push(&value);
        layers.push((id, name));
    }
    if let Some(keycaps) = keycaps_preview {
        if keycaps.scope != identity.scope
            || keycaps.token != identity.snapshot_token
            || keycaps.revision != preview.owner.accepted_revision
        {
            return Err("Keycap preview does not match the current canonical board".into());
        }
        for body in &keycaps.bodies {
            let (color, label) = if let Some(id) = body.id.strip_prefix("keycap-legend:") {
                let spec = keycaps
                    .specs
                    .iter()
                    .find(|spec| spec.id == id)
                    .ok_or_else(|| {
                        "Keycap legend preview does not match its accepted specification".to_owned()
                    })?;
                (
                    spec.legend_color.as_str(),
                    format!("{} legend", spec.reference),
                )
            } else if let Some(id) = body.id.strip_prefix("keycap:") {
                let spec = keycaps
                    .specs
                    .iter()
                    .find(|spec| spec.id == id)
                    .ok_or_else(|| {
                        "Keycap preview does not match its accepted specification".to_owned()
                    })?;
                (spec.color.as_str(), spec.reference.clone())
            } else {
                return Err("Keycap preview returned an unknown body identity".into());
            };
            let value = Object::new();
            let mesh = Object::new();
            Reflect::set(
                &mesh,
                &"positions".into(),
                &Float32Array::from(body.positions.as_slice()),
            )
            .map_err(js_error)?;
            Reflect::set(
                &mesh,
                &"normals".into(),
                &Float32Array::from(body.normals.as_slice()),
            )
            .map_err(js_error)?;
            Reflect::set(&value, &"id".into(), &body.id.clone().into()).map_err(js_error)?;
            Reflect::set(&value, &"name".into(), &body.name.clone().into()).map_err(js_error)?;
            Reflect::set(&value, &"color".into(), &color.into()).map_err(js_error)?;
            Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
            bodies.push(&value);
            layers.push((body.id.clone(), label));
        }
    }
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
}

fn layout_module_model_bodies(
    rows: Option<&ModelDeliveryRows>,
    document: &boardstudio_core::model::ProjectDoc,
) -> Vec<serde_json::Value> {
    rows.into_iter()
        .flat_map(|rows| rows.delivered.iter())
        .filter_map(|model| {
            let matrix = model.matrix?;
            let (positions, normals) = transformed_module_mesh(
                model.mesh.positions.as_ref(),
                model.mesh.normals.as_ref(),
                &matrix,
            );
            let module_id = model.id.strip_prefix("module-model/")?.rsplit_once('/')?.0;
            let name = document
                .modules
                .iter()
                .find(|module| module.id == module_id)
                .and_then(|module| {
                    document
                        .module_definitions
                        .iter()
                        .find(|definition| definition.id == module.definition_id)
                })
                .map(|definition| format!("{} · 3D model", definition.name))
                .unwrap_or_else(|| "Mounted module · 3D model".to_owned());
            Some(serde_json::json!({
                "id": model.id,
                "name": name,
                "color": "#8f9b9e",
                "mesh": {
                    "positions": positions,
                    "normals": normals,
                    "colors": model.mesh.colors.as_ref().map(|colors| colors.as_ref()),
                },
            }))
        })
        .collect()
}

/// Apply Core's column-major affine transform exactly once before generic mesh
/// bodies enter the renderer. This mirrors the existing React AssemblyPreview
/// transform while keeping Core authoritative for module pose and model offsets.
fn transformed_module_mesh(
    source_positions: &[f32],
    source_normals: &[f32],
    matrix: &[f64; 16],
) -> (Vec<f32>, Vec<f32>) {
    let squared_lengths = [0, 4, 8].map(|index| {
        (matrix[index].powi(2) + matrix[index + 1].powi(2) + matrix[index + 2].powi(2)) as f32
    });
    let mut positions = vec![0.0; source_positions.len()];
    let mut normals = vec![0.0; source_normals.len()];
    for index in (0..source_positions.len()).step_by(3) {
        let x = source_positions[index];
        let y = source_positions[index + 1];
        let z = source_positions[index + 2];
        positions[index] = (matrix[0] as f32) * x
            + (matrix[4] as f32) * y
            + (matrix[8] as f32) * z
            + matrix[12] as f32;
        positions[index + 1] = (matrix[1] as f32) * x
            + (matrix[5] as f32) * y
            + (matrix[9] as f32) * z
            + matrix[13] as f32;
        positions[index + 2] = (matrix[2] as f32) * x
            + (matrix[6] as f32) * y
            + (matrix[10] as f32) * z
            + matrix[14] as f32;

        let nx = source_normals[index] / squared_lengths[0].max(f32::MIN_POSITIVE);
        let ny = source_normals[index + 1] / squared_lengths[1].max(f32::MIN_POSITIVE);
        let nz = source_normals[index + 2] / squared_lengths[2].max(f32::MIN_POSITIVE);
        let a = matrix[0] as f32 * nx + matrix[4] as f32 * ny + matrix[8] as f32 * nz;
        let b = matrix[1] as f32 * nx + matrix[5] as f32 * ny + matrix[9] as f32 * nz;
        let c = matrix[2] as f32 * nx + matrix[6] as f32 * ny + matrix[10] as f32 * nz;
        let length = (a * a + b * b + c * c).sqrt();
        let length = if length > 0.0 { length } else { 1.0 };
        normals[index] = a / length;
        normals[index + 1] = b / length;
        normals[index + 2] = c / length;
    }
    (positions, normals)
}

fn layout_module_bodies(
    modules: &[boardstudio_core::model::ResolvedModule],
    document: &boardstudio_core::model::ProjectDoc,
    board_id: &str,
    pcb_top_z: f64,
) -> Vec<serde_json::Value> {
    modules
        .iter()
        .flat_map(|module| {
            let Some(instance) = document
                .modules
                .iter()
                .find(|instance| instance.id == module.id && instance.host_board_id == board_id)
            else {
                return Vec::new();
            };
            let name = document
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)
                .map(|definition| definition.name.as_str())
                .unwrap_or("Mounted module");
            let contour =
                |points: &[_], hole: bool| serde_json::json!({ "points": points, "hole": hole });
            let mut bodies = module
                .board
                .iter()
                .enumerate()
                .map(|(index, solid)| {
                    let mut contours = vec![contour(&solid.points, false)];
                    contours.extend(
                        module
                            .board_holes
                            .iter()
                            .map(|hole| contour(&hole.points, true)),
                    );
                    serde_json::json!({
                        "id": format!("module:{}:pcb:{index}", module.id),
                        "name": format!("{name} · PCB"),
                        "z": solid.z + pcb_top_z,
                        "thickness": solid.height,
                        "contours": contours,
                    })
                })
                .collect::<Vec<_>>();
            bodies.extend(module.volumes.iter().enumerate().map(|(index, volume)| {
                serde_json::json!({
                    "id": format!("module:{}:volume:{index}", module.id),
                    "name": format!("{name} · Assembly volume"),
                    "z": volume.geometry.z + pcb_top_z,
                    "thickness": volume.geometry.height,
                    "contours": [contour(&volume.geometry.points, false)],
                })
            }));
            if instance.attachment == boardstudio_core::model::ModuleAttachment::Board {
                for support in &module.mount_supports {
                    let ring = |diameter: f64, clockwise: bool| {
                        (0..48)
                            .map(|index| {
                                let direction = if clockwise { -1.0 } else { 1.0 };
                                let angle = std::f64::consts::TAU * index as f64 / 48.0;
                                boardstudio_core::model::Vec2 {
                                    x: support.at.x + diameter / 2.0 * (direction * angle).cos(),
                                    y: support.at.y + diameter / 2.0 * (direction * angle).sin(),
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                    bodies.push(serde_json::json!({
                        "id": format!("module:{}:standoff:{}", module.id, support.mount_id),
                        "name": format!("{} · PCB standoff", module.id),
                        "z": support.z + pcb_top_z,
                        "thickness": support.height,
                        "contours": [
                            contour(&ring(support.outer_diameter, false), false),
                            contour(&ring(support.hole_diameter, true), true),
                        ],
                    }));
                }
            }
            bodies
        })
        .collect()
}

fn project_parts_preview(
    preview: &crate::parts_preview::PartsPreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
        || !preview.lease.matches(&preview.owner)
    {
        return Err("Parts sample does not match the active viewer owner".into());
    }
    let board = preview
        .sample_document
        .boards
        .iter()
        .find(|board| board.id == "sample-board")
        .ok_or_else(|| "Parts sample board is unavailable".to_owned())?;
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": board.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers: vec![("pcb".to_owned(), "PCB".to_owned())],
    })
}

fn loaded_model_inputs(rows: Option<&ModelDeliveryRows>) -> Array {
    let loaded = Array::new();
    if let Some(rows) = rows {
        for model in &rows.delivered {
            if model.matrix.is_some() {
                continue;
            }
            let value = Object::new();
            let mesh = Object::new();
            let _ = Reflect::set(&value, &"id".into(), &model.id.clone().into());
            let _ = Reflect::set(
                &mesh,
                &"positions".into(),
                &Float32Array::from(model.mesh.positions.as_ref()),
            );
            let _ = Reflect::set(
                &mesh,
                &"normals".into(),
                &Float32Array::from(model.mesh.normals.as_ref()),
            );
            if let Some(colors) = &model.mesh.colors {
                let _ = Reflect::set(
                    &mesh,
                    &"colors".into(),
                    &Float32Array::from(colors.as_ref()),
                );
            }
            let _ = Reflect::set(&value, &"mesh".into(), &mesh);
            loaded.push(&value);
        }
    }
    loaded
}

fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| "Browser renderer operation failed".to_owned())
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ViewerHandle {
    pub(super) id: String,
    pub(super) x: f32,
    pub(super) y: f32,
    pub(super) z: f32,
    pub(super) tangent_x: f32,
    pub(super) tangent_y: f32,
    pub(super) normal_x: f32,
    pub(super) normal_y: f32,
    pub(super) length: f32,
    pub(super) invalid: bool,
    pub(super) target: ViewerHandleTarget,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum ViewerHandleTarget {
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
        last_x: f64,
        last_y: f64,
        start_x: f64,
        start_y: f64,
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

#[component]
fn SharedViewer(
    projection: Rc<RendererSceneProjection>,
    assembly_layers: Vec<super::case_assembly_layers::CaseAssemblyLayer>,
    component_layers: Vec<super::case_assembly_layers::CaseComponentLayer>,
    selectable_layers: Vec<String>,
    canvas_context: ViewerCanvasContext,
    selected_layer: String,
    selected_reference: Option<String>,
    display: CaseDisplay,
    theme: String,
    owner: Rc<ViewerOwner>,
    current_source: SourceGuard,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
    mechanical_settings: Option<super::MechanicalSettingsProps>,
    inline_case_controls: bool,
    handle_projection: Option<CaseHandleProjection>,
    handles: Vec<ViewerHandle>,
    handle_preview: Option<Vec<ViewerHandle>>,
    gesture_message: Option<String>,
    focus_request: Option<ViewerFocusRequest>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let mut editing_gaskets = use_signal(|| false);
    let mut editing_mounts = use_signal(|| false);
    let host = use_hook(|| Rc::new(RefCell::new(None::<RendererPageHost>)));
    let canvas = use_hook(|| Rc::new(RefCell::new(None::<HtmlCanvasElement>)));
    let live_projection = use_hook(|| Rc::new(RefCell::new(projection.clone())));
    *live_projection.borrow_mut() = projection.clone();
    let live_source = use_hook(|| Rc::new(RefCell::new(current_source.clone())));
    *live_source.borrow_mut() = current_source.clone();
    let mounted = use_signal(|| false);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let mut transient = use_signal(TransientView::default);
    let pointer = use_hook(|| Rc::new(RefCell::new(None::<PointerOwner>)));
    let applied_sequence = use_hook(|| Rc::new(Cell::new(0_u64)));
    let applied_identity = use_hook(|| Rc::new(RefCell::new(None::<ViewerIdentity>)));

    use_effect(use_reactive((&projection, &handle_projection), {
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let owner = owner.clone();
        let live_source = live_source.clone();
        move |(_projection, handle_projection)| {
            cancel_superseded_pointer(
                &pointer,
                &canvas,
                &owner,
                handle_projection.as_ref(),
                (live_source.borrow().0)(),
            );
        }
    }));

    use_drop({
        let host = host.clone();
        let owner = owner.clone();
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move || {
            owner.active.set(false);
            if let Some(pointer_id) = pointer
                .borrow_mut()
                .take()
                .map(|active| active.pointer_id())
            {
                release_pointer_capture(&canvas, pointer_id);
            }
            if let Some(host) = host.borrow_mut().take() {
                let _ = host.dispose();
            }
        }
    });

    use_effect(use_reactive((&projection, &mounted()), {
        let host = host.clone();
        let applied_sequence = applied_sequence.clone();
        let applied_identity = applied_identity.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, mounted)| {
            if !mounted || !owner.active.get() || !(current_source.0)() {
                return;
            }
            let identity = projection.identity.clone();
            if owner.identity.borrow().as_ref() != Some(&identity) {
                return;
            }
            if applied_sequence.get() >= identity.renderer_sequence {
                return;
            }
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| {
                    host.submit_scene(projection.input.clone(), identity.renderer_sequence)
                });
            match result {
                Ok(accepted) => {
                    applied_sequence.set(identity.renderer_sequence);
                    if accepted {
                        *applied_identity.borrow_mut() = Some(identity.clone());
                    }
                    if owner_is_current(&owner, &identity) && (current_source.0)() {
                        status.set(if accepted {
                            "3D preview updated.".to_owned()
                        } else {
                            "A stale 3D scene update was ignored.".to_owned()
                        });
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::SceneAccepted(accepted),
                        );
                    }
                }
                Err(error) => {
                    if owner_is_current(&owner, &identity) && (current_source.0)() {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::Failed(error),
                        );
                    }
                }
            }
        }
    }));

    use_effect(use_reactive(
        (
            &projection,
            &selected_layer,
            &display,
            &theme,
            &transient(),
            &mounted(),
        ),
        {
            let host = host.clone();
            let owner = owner.clone();
            let current_source = current_source.clone();
            let on_signal = on_signal;
            let mut status = status;
            move |(projection, selected_layer, display, theme, transient, mounted)| {
                if !mounted
                    || !owner.active.get()
                    || owner.identity.borrow().as_ref() != Some(&projection.identity)
                {
                    return;
                }
                let result =
                    display_state(&display, &selected_layer, &theme, transient).and_then(|state| {
                        host.borrow()
                            .as_ref()
                            .ok_or_else(|| "3D renderer is not mounted".to_owned())
                            .and_then(|host| host.set_display_state(state))
                    });
                if let Err(error) = result
                    && owner_is_current(&owner, &projection.identity)
                    && (current_source.0)()
                {
                    status.set(error.clone());
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        },
    ));

    use_effect(use_reactive((&projection, &focus_request, &mounted()), {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, request, mounted)| {
            if !mounted
                || !owner.active.get()
                || owner.identity.borrow().as_ref() != Some(&projection.identity)
                || !(current_source.0)()
            {
                return;
            }
            let target_ids = request
                .filter(|request| focus_request_matches(request, &projection.identity))
                .map_or_else(Vec::new, |request| request.target_ids.clone());
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| host.focus_objects(&target_ids));
            match result {
                Ok(true) => {}
                Ok(false) => {
                    status.set("Finding geometry is not available in this 3D scene.".into())
                }
                Err(error) => {
                    status.set(error.clone());
                    if owner_is_current(&owner, &projection.identity) && (current_source.0)() {
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                    }
                }
            }
        }
    }));

    use_effect(use_reactive(
        (
            &projection,
            &mounted(),
            &editing_gaskets(),
            &editing_mounts(),
            &handles,
            &handle_preview,
        ),
        {
            let host = host.clone();
            let owner = owner.clone();
            let current_source = current_source.clone();
            let on_signal = on_signal;
            let mut status = status;
            move |(projection, mounted, edit_gaskets, edit_mounts, handles, preview)| {
                if !mounted
                    || !owner.active.get()
                    || !(current_source.0)()
                    || owner.identity.borrow().as_ref() != Some(&projection.identity)
                {
                    return;
                }
                let active =
                    active_case_handles(&handles, preview.as_deref(), edit_gaskets, edit_mounts);
                let result = handles_value(&active).and_then(|handles| {
                    host.borrow()
                        .as_ref()
                        .ok_or_else(|| "3D renderer is not mounted".to_owned())
                        .and_then(|host| host.set_handles(handles).map(|_| ()))
                });
                if let Err(error) = result
                    && owner_is_current(&owner, &projection.identity)
                    && (current_source.0)()
                {
                    status.set(error.clone());
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        },
    ));

    let mount_host = {
        let projection = projection.clone();
        let host = host.clone();
        let canvas_state = canvas.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let live_projection = live_projection.clone();
        let live_source = live_source.clone();
        let applied_identity = applied_identity.clone();
        let on_signal = on_signal;
        let mut status = status;
        let applied_sequence = applied_sequence.clone();
        move |event: MountedEvent| {
            let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|value| value.dyn_into::<HtmlCanvasElement>().ok())
            else {
                let identity = projection.identity.clone();
                if owner_is_current(&owner, &identity) && (live_source.borrow().0)() {
                    status.set("3D canvas could not be attached.".to_owned());
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::Failed("3D canvas could not be attached.".to_owned()),
                    );
                }
                return;
            };
            *canvas_state.borrow_mut() = Some(element.clone());
            let host = host.clone();
            let owner = owner.clone();
            let runtime = runtime.clone();
            let live_projection = live_projection.clone();
            let live_source = live_source.clone();
            let applied_identity = applied_identity.clone();
            let on_signal = on_signal;
            let mut status = status;
            let mut mounted = mounted;
            let initial_projection = projection.clone();
            let applied_sequence = applied_sequence.clone();
            if owner_is_current(&owner, &initial_projection.identity) && (live_source.borrow().0)()
            {
                emit_signal(
                    &owner,
                    on_signal,
                    &initial_projection.identity,
                    ViewerSignalKind::Lifecycle(ViewerLifecycle::Starting),
                );
                status.set("Starting the 3D renderer…".to_owned());
            }
            spawn_local(async move {
                let mut request = initial_projection;
                loop {
                    if !owner.active.get()
                        || runtime.scope().as_ref() != Some(&request.identity.scope)
                    {
                        break;
                    }
                    if !owner_is_current(&owner, &request.identity) {
                        request = live_projection.borrow().clone();
                        continue;
                    }
                    let request_identity = request.identity.clone();
                    let request_owner = owner.clone();
                    let request_runtime = runtime.clone();
                    let guard_identity = request_identity.clone();
                    let is_current = Rc::new(move || {
                        owner_is_current(&request_owner, &guard_identity)
                            && request_runtime.scope().as_ref() == Some(&guard_identity.scope)
                    });
                    let mount_guard: Rc<dyn Fn() -> bool> = is_current.clone();
                    let report_owner = owner.clone();
                    let report_runtime = runtime.clone();
                    let report_source = live_source.clone();
                    let report_applied = applied_identity.clone();
                    let report_request_identity = request.identity.clone();
                    let report_status = status;
                    let status_callback =
                        Rc::new(move |message: String| {
                            let identity = report_applied
                                .borrow()
                                .clone()
                                .unwrap_or_else(|| report_request_identity.clone());
                            if !owner_is_current(&report_owner, &identity)
                                || report_runtime.scope().as_ref() != Some(&identity.scope)
                                || !(report_source.borrow().0)()
                                || !report_runtime.model().accepted.as_ref().is_some_and(
                                    |snapshot| snapshot.token == identity.snapshot_token,
                                )
                            {
                                return;
                            }
                            let kind = if message.to_ascii_lowercase().contains("context lost") {
                                ViewerSignalKind::Lifecycle(ViewerLifecycle::ContextLost)
                            } else if message.contains("initialized") {
                                ViewerSignalKind::Lifecycle(ViewerLifecycle::Ready)
                            } else if message.to_ascii_lowercase().contains("failed") {
                                ViewerSignalKind::Failed(message.clone())
                            } else {
                                return;
                            };
                            let mut report_status = report_status;
                            report_status.set(message);
                            on_signal.call(ScopedViewerSignal {
                                identity,
                                kind,
                                owner: Rc::downgrade(&report_owner),
                            });
                        });
                    match RendererPageHost::mount(
                        element.clone(),
                        request.input.clone(),
                        request.identity.renderer_sequence,
                        status_callback,
                        mount_guard,
                    )
                    .await
                    {
                        Ok((renderer, accepted)) if is_current() => {
                            *host.borrow_mut() = Some(renderer);
                            applied_sequence.set(request.identity.renderer_sequence);
                            if accepted {
                                *applied_identity.borrow_mut() = Some(request.identity.clone());
                            }
                            mounted.set(true);
                            if owner_is_current(&owner, &request.identity)
                                && (live_source.borrow().0)()
                            {
                                status.set(if accepted {
                                    "3D preview ready.".to_owned()
                                } else {
                                    "A stale 3D scene update was ignored.".to_owned()
                                });
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::SceneAccepted(accepted),
                                );
                            }
                            break;
                        }
                        Ok((renderer, _)) => {
                            let _ = renderer.dispose();
                            if !owner.active.get()
                                || runtime.scope().as_ref() != Some(&request.identity.scope)
                            {
                                break;
                            }
                            request = live_projection.borrow().clone();
                        }
                        Err(_error) if !is_current() => {
                            if !owner.active.get()
                                || runtime.scope().as_ref() != Some(&request.identity.scope)
                            {
                                break;
                            }
                            request = live_projection.borrow().clone();
                            if request.identity == request_identity {
                                break;
                            }
                        }
                        Err(error) => {
                            if owner_is_current(&owner, &request.identity)
                                && (live_source.borrow().0)()
                                && runtime.model().accepted.as_ref().is_some_and(|snapshot| {
                                    snapshot.token == request.identity.snapshot_token
                                })
                            {
                                status.set(format!(
                                    "3D preview unavailable: {error}. Case forms and the 2D route remain available."
                                ));
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::Failed(error),
                                );
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &request.identity,
                                    ViewerSignalKind::Lifecycle(ViewerLifecycle::Failed),
                                );
                            }
                            break;
                        }
                    }
                }
            });
        }
    };

    let active_handles = active_case_handles(
        &handles,
        handle_preview.as_deref(),
        editing_gaskets(),
        editing_mounts(),
    );
    let on_pointer_down = {
        let handle_projection = handle_projection.clone();
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let handles = active_handles.clone();
        let applied_identity = applied_identity.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) || pointer.borrow().is_some() {
                return;
            }
            event.prevent_default();
            let (x, y) = pointer_point(&event);
            let host_guard = host.borrow();
            let Some(host) = host_guard.as_ref() else {
                return;
            };
            let picked = if (current_source.0)()
                && applied_identity.borrow().as_ref() == Some(&projection.identity)
            {
                match host.pick_at_client(x, y) {
                    Ok(id) => id.filter(|id| !id.is_empty()),
                    Err(error) => {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                        None
                    }
                }
            } else {
                None
            };
            if let Some(handle) = picked
                .as_ref()
                .and_then(|id| handles.iter().find(|handle| &handle.id == id))
            {
                let point = match host.point_on_plane_at_client(x, y, handle.z) {
                    Ok(point) => point,
                    Err(error) => {
                        status.set(error.clone());
                        emit_signal(
                            &owner,
                            on_signal,
                            &projection.identity,
                            ViewerSignalKind::Failed(error),
                        );
                        return;
                    }
                };
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::WorldPoint(point),
                );
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::HandleGesture {
                        phase: HandleGesturePhase::Start,
                        handle_id: handle.id.clone(),
                        point,
                    },
                );
                *pointer.borrow_mut() = Some(PointerOwner::Handle {
                    projection: handle_projection.clone(),
                    identity: projection.identity.clone(),
                    pointer_id: event.pointer_id(),
                    z: handle.z,
                    id: handle.id.clone(),
                });
            } else {
                *pointer.borrow_mut() = Some(PointerOwner::Orbit {
                    identity: projection.identity.clone(),
                    pointer_id: event.pointer_id(),
                    last_x: x,
                    last_y: y,
                    start_x: x,
                    start_y: y,
                    moved: false,
                    picked,
                });
            }
            if let Some(canvas) = canvas.borrow().as_ref()
                && let Err(error) = canvas.set_pointer_capture(event.pointer_id())
            {
                status.set(js_error(error));
            }
        }
    };

    let on_wheel = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |event: WheelEvent| {
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let Some(wheel) = event.data().try_as_web_event() else {
                return;
            };
            wheel.prevent_default();
            let delta = wheel.delta_y();
            let factor = (delta.signum() * (delta.abs() * 0.001).min(1.0)).exp();
            let result = host
                .borrow()
                .as_ref()
                .ok_or_else(|| "3D renderer is not mounted".to_owned())
                .and_then(|host| host.zoom(factor));
            if let Err(error) = result {
                status.set(error.clone());
                if (current_source.0)() {
                    emit_signal(
                        &owner,
                        on_signal,
                        &projection.identity,
                        ViewerSignalKind::Failed(error),
                    );
                }
            }
        }
    };

    let on_pointer_move = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let applied_identity = applied_identity.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let host_guard = host.borrow();
            let Some(host) = host_guard.as_ref() else {
                return;
            };
            let (x, y) = pointer_point(&event);
            let Some(active) =
                take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity)
            else {
                return;
            };
            let identity = active.identity().clone();
            match active {
                PointerOwner::Orbit {
                    identity: pointer_identity,
                    pointer_id,
                    last_x,
                    last_y,
                    start_x,
                    start_y,
                    mut moved,
                    picked,
                } if pointer_id == event.pointer_id() => {
                    let delta_x = x - last_x;
                    let delta_y = y - last_y;
                    moved |= (x - start_x).abs() + (y - start_y).abs() > 4.0;
                    if let Err(error) = host.orbit(delta_x, delta_y) {
                        status.set(error.clone());
                        if (current_source.0)() {
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                        }
                    }
                    *pointer.borrow_mut() = Some(PointerOwner::Orbit {
                        identity: pointer_identity,
                        pointer_id,
                        last_x: x,
                        last_y: y,
                        start_x,
                        start_y,
                        moved,
                        picked,
                    });
                }
                PointerOwner::Handle {
                    identity: pointer_identity,
                    projection: handle_projection,
                    pointer_id,
                    z,
                    id,
                } if pointer_id == event.pointer_id() => {
                    if !(current_source.0)()
                        || applied_identity.borrow().as_ref() != Some(&identity)
                    {
                        release_pointer_capture(&canvas, pointer_id);
                        return;
                    }
                    let point = match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            status.set(error.clone());
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                            *pointer.borrow_mut() = Some(PointerOwner::Handle {
                                projection: handle_projection,
                                identity: pointer_identity,
                                pointer_id,
                                z,
                                id,
                            });
                            return;
                        }
                    };
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::WorldPoint(point),
                    );
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::HandleGesture {
                            phase: HandleGesturePhase::Move,
                            handle_id: id.clone(),
                            point,
                        },
                    );
                    *pointer.borrow_mut() = Some(PointerOwner::Handle {
                        projection: handle_projection,
                        identity: pointer_identity,
                        pointer_id,
                        z,
                        id,
                    });
                }
                _ => {}
            }
        }
    };

    let on_pointer_up = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let applied_identity = applied_identity.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let active = take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity);
            let Some(active) = active else {
                return;
            };
            release_pointer_capture(&canvas, event.pointer_id());
            let identity = active.identity().clone();
            if identity != projection.identity {
                return;
            }
            match active {
                PointerOwner::Orbit {
                    pointer_id,
                    moved,
                    picked,
                    ..
                } if pointer_id == event.pointer_id() => {
                    if !moved
                        && (current_source.0)()
                        && applied_identity.borrow().as_ref() == Some(&identity)
                        && runtime.scope().as_ref() == Some(&projection.identity.scope)
                        && let Some(id) = picked
                    {
                        if let Some(host) = host.borrow().as_ref() {
                            let _ = host.focus_objects(&[]);
                        }
                        emit_signal(&owner, on_signal, &identity, ViewerSignalKind::Picked(id));
                    }
                }
                PointerOwner::Handle {
                    pointer_id, z, id, ..
                } if pointer_id == event.pointer_id() => {
                    if !(current_source.0)()
                        || applied_identity.borrow().as_ref() != Some(&identity)
                    {
                        return;
                    }
                    if let Some(host) = host.borrow().as_ref() {
                        let (x, y) = pointer_point(&event);
                        let point = match host.point_on_plane_at_client(x, y, z) {
                            Ok(point) => point,
                            Err(error) => {
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &identity,
                                    ViewerSignalKind::Failed(error),
                                );
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    &identity,
                                    ViewerSignalKind::HandleGesture {
                                        phase: HandleGesturePhase::Cancel,
                                        handle_id: id,
                                        point: None,
                                    },
                                );
                                return;
                            }
                        };
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::WorldPoint(point),
                        );
                        emit_signal(
                            &owner,
                            on_signal,
                            &identity,
                            ViewerSignalKind::HandleGesture {
                                phase: HandleGesturePhase::End,
                                handle_id: id,
                                point,
                            },
                        );
                    }
                }
                _ => {}
            }
        }
    };

    let on_pointer_cancel = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let applied_identity = applied_identity.clone();
        move |event: dioxus::prelude::PointerEvent| {
            let Some(event) = event.data().try_as_web_event() else {
                return;
            };
            if !owner_is_current(&owner, &projection.identity) {
                return;
            }
            let active = take_pointer_for_event(&pointer, event.pointer_id(), &projection.identity);
            let Some(active) = active else {
                return;
            };
            release_pointer_capture(&canvas, event.pointer_id());
            let identity = active.identity().clone();
            if identity != projection.identity
                || !(current_source.0)()
                || applied_identity.borrow().as_ref() != Some(&identity)
            {
                return;
            }
            if let PointerOwner::Handle {
                pointer_id, z, id, ..
            } = active
                && pointer_id == event.pointer_id()
            {
                let point = if let Some(host) = host.borrow().as_ref() {
                    let (x, y) = pointer_point(&event);
                    match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            emit_signal(
                                &owner,
                                on_signal,
                                &identity,
                                ViewerSignalKind::Failed(error),
                            );
                            None
                        }
                    }
                } else {
                    None
                };
                emit_signal(
                    &owner,
                    on_signal,
                    &identity,
                    ViewerSignalKind::WorldPoint(point),
                );
                emit_signal(
                    &owner,
                    on_signal,
                    &identity,
                    ViewerSignalKind::HandleGesture {
                        phase: HandleGesturePhase::Cancel,
                        handle_id: id,
                        point,
                    },
                );
            }
        }
    };

    let on_key_down = {
        let projection = projection.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: KeyboardEvent| {
            let Some(keyboard) = event.data().try_as_web_event() else {
                return;
            };
            if keyboard.key() != "Escape"
                || !owner_is_current(&owner, &projection.identity)
                || !(current_source.0)()
            {
                return;
            }
            let active = pointer.borrow_mut().take();
            match active {
                Some(PointerOwner::Handle {
                    identity,
                    pointer_id,
                    id,
                    ..
                }) if identity == projection.identity => {
                    release_pointer_capture(&canvas, pointer_id);
                    keyboard.prevent_default();
                    emit_signal(
                        &owner,
                        on_signal,
                        &identity,
                        ViewerSignalKind::HandleGesture {
                            phase: HandleGesturePhase::Cancel,
                            handle_id: id,
                            point: None,
                        },
                    );
                }
                Some(active) => *pointer.borrow_mut() = Some(active),
                None => {}
            }
        }
    };

    let select_layer = {
        let projection = projection.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        move |id: String| {
            if (current_source.0)()
                && runtime.scope().as_ref() == Some(&projection.identity.scope)
                && owner_is_current(&owner, &projection.identity)
            {
                emit_signal(
                    &owner,
                    on_signal,
                    &projection.identity,
                    ViewerSignalKind::LayerSelected(id),
                );
            }
        }
    };

    let change_display = {
        let projection = projection.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        move |next: CaseDisplay| {
            let identity = projection.identity.clone();
            if !(current_source.0)()
                || !owner_is_current(&owner, &identity)
                || runtime.scope().as_ref() != Some(&identity.scope)
            {
                return;
            }
            on_display_change.call(ScopedDisplayChange {
                identity,
                display: next,
                owner: Rc::downgrade(&owner),
            });
        }
    };

    let on_pointer_cancel = EventHandler::new(on_pointer_cancel);
    let on_key_down = EventHandler::new(on_key_down);
    let fit = host.clone();
    let top = host.clone();
    let top_for_gaskets = host.clone();
    let top_for_mounts = host.clone();
    let bottom = host.clone();
    let iso = host.clone();
    let case_fit = host.clone();
    let case_top = host.clone();
    let case_bottom = host.clone();
    let case_iso = host.clone();
    let left = host.clone();
    let right = host.clone();
    let zoom_in = host.clone();
    let zoom_out = host.clone();
    let assembly_change_display = change_display.clone();
    let selected_support = mechanical_settings.as_ref().and_then(|settings| {
        settings.gasket_supports.iter().find(|support| {
            !support.is_previous
                && (selected_layer == format!("gasket:{}:lower", support.id)
                    || selected_layer == format!("gasket:{}:upper", support.id))
        })
    });
    let can_edit_gaskets = mechanical_settings.as_ref().is_some_and(|settings| {
        handles
            .iter()
            .any(|handle| matches!(&handle.target, ViewerHandleTarget::Gasket { .. }))
            && settings.identity.scope == projection.identity.scope
            && settings.identity.snapshot_token == projection.identity.snapshot_token
            && settings.editable
    });
    let can_edit_mounts = mechanical_settings.as_ref().is_some_and(|settings| {
        handles.iter().any(|handle| {
            matches!(
                &handle.target,
                ViewerHandleTarget::Mount { .. } | ViewerHandleTarget::AuthoredMount { .. }
            )
        }) && settings.identity.scope == projection.identity.scope
            && settings.identity.snapshot_token == projection.identity.snapshot_token
            && settings.editable
    }) || handles
        .iter()
        .any(|handle| matches!(&handle.target, ViewerHandleTarget::AuthoredMount { .. }));
    let unlink_request = selected_support.and_then(|support| {
        mechanical_settings.as_ref().map(|settings| {
            let anchors = settings
                .gasket_supports
                .iter()
                .filter(|candidate| {
                    candidate.id == support.id
                        || candidate.pair_id.as_deref() == Some(support.id.as_str())
                })
                .map(super::mechanical_settings::MechanicalGasketSupportRow::saved_anchor)
                .collect::<Vec<_>>();
            (
                support.id.clone(),
                support.pair_id.clone(),
                anchors,
                settings.identity.clone(),
                settings.request_sequence,
                settings.on_request,
                settings.editable,
            )
        })
    });
    let unlink_feedback = selected_support.and_then(|support| {
        mechanical_settings.as_ref().and_then(|settings| {
            let field_id = format!("gasket-support:{}:link", support.id);
            settings
                .feedback
                .iter()
                .rev()
                .find(|feedback| feedback.field_id == field_id)
                .cloned()
        })
    });
    let unlink_message = unlink_feedback
        .as_ref()
        .map(|feedback| match feedback.state {
            super::mechanical_settings::MechanicalSettingsFeedbackState::Pending => {
                (false, "Unlinking selected support…".to_owned())
            }
            super::mechanical_settings::MechanicalSettingsFeedbackState::Saved => {
                (false, "Selected support unlinked.".to_owned())
            }
            super::mechanical_settings::MechanicalSettingsFeedbackState::Failed => (
                true,
                feedback
                    .message
                    .clone()
                    .unwrap_or_else(|| "The support could not be unlinked.".to_owned()),
            ),
        });
    let unlink_source = current_source.clone();
    let unlink_owner = owner.clone();
    let unlink_projection = projection.clone();
    let unlink_runtime = runtime.clone();
    let show_assembly_controls = projection.layers.iter().any(|(id, _)| id != "pcb");
    let select_assembly_layer = select_layer.clone();

    rsx! {
        div { class: "m1-case-view m1-shared-viewer",
            div { class: "m1-case-view-toolbar",
                if inline_case_controls {
                    div { role: "group", "aria-label": "Display mode",
                        for mode in [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid] {
                            {
                                let mut transient = transient;
                                let label = match mode {
                                    RenderMode::Shaded => "Shaded",
                                    RenderMode::Wireframe => "Wireframe",
                                    RenderMode::Hybrid => "Hybrid",
                                };
                                rsx! { button {
                                    "aria-pressed": transient().mode == mode,
                                    onclick: move |_| transient.with_mut(|view| view.mode = mode),
                                    "{label}"
                                } }
                            }
                        }
                        button {
                            "aria-label": "Show hidden lines",
                            title: "Show hidden lines",
                            "aria-pressed": transient().show_hidden,
                            onclick: move |_| transient.with_mut(|view| view.show_hidden = !view.show_hidden),
                            svg { view_box: "0 0 24 24", "aria-hidden": "true",
                                path { d: "m12 2 9 5v10l-9 5-9-5V7ZM3 7l9 5 9-5M12 12v10" }
                                path { stroke_dasharray: "2 2", d: "M12 2v10M3 17l9-5 9 5" }
                            }
                        }
                    }
                if show_assembly_controls {
                    div { role: "group", "aria-label": "Mechanical assembly view",
                            for view in [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section] {
                                {
                                    let mut transient = transient;
                                    let label = match view {
                                        AssemblyView::Assembled => "Assembled",
                                        AssemblyView::Exploded => "Exploded",
                                        AssemblyView::Section => "Section",
                                    };
                                    rsx! { button {
                                        "aria-pressed": transient().assembly == view,
                                        onclick: move |_| transient.with_mut(|state| state.assembly = view),
                                        "{label}"
                                    } }
                                }
                            }
                        }
                    }
                }
                if can_edit_gaskets {
                    button {
                        r#type: "button",
                        aria_pressed: editing_gaskets(),
                        disabled: mechanical_settings.as_ref().is_none_or(|settings| !settings.editable),
                        onclick: move |_| {
                            let enable = !editing_gaskets();
                            editing_gaskets.set(enable);
                            editing_mounts.set(false);
                            if enable { run_host(&top_for_gaskets, |host| host.view("top"), &mut status); }
                        },
                        "Edit gaskets"
                    }
                }
                if can_edit_mounts {
                    button {
                        r#type: "button",
                        aria_pressed: editing_mounts(),
                        onclick: move |_| {
                            let enable = !editing_mounts();
                            editing_mounts.set(enable);
                            editing_gaskets.set(false);
                            if enable { run_host(&top_for_mounts, |host| host.view("top"), &mut status); }
                        },
                        "Edit mounts"
                    }
                }
                if editing_gaskets() {
                    if let Some((support_id, pair_id, anchors, identity, mut sequence, on_request, editable)) = unlink_request.clone() {
                        button {
                            r#type: "button",
                            disabled: !editable || selected_support.is_some_and(|support| support.unlinked),
                            onclick: move |_| {
                                if !(unlink_source.0)()
                                    || !owner_is_current(&unlink_owner, &unlink_projection.identity)
                                    || unlink_runtime.scope().as_ref() != Some(&identity.scope)
                                {
                                    return;
                                }
                                let Some(request_id) = sequence().checked_add(1) else { return };
                                sequence.set(request_id);
                                let patch = super::mechanical_settings::MechanicalSettingsPatch::SetGasketSupportUnlinked {
                                    support_id: support_id.clone(), pair_id: pair_id.clone(), anchors: anchors.clone(),
                                };
                                on_request.call(super::mechanical_settings::MechanicalSettingsRequest {
                                    identity: identity.clone(),
                                    request_id,
                                    field_id: patch.field_id(),
                                    patch,
                                });
                            },
                            "Unlink selected support"
                        }
                        if let Some((alert, message)) = unlink_message {
                            p { role: if alert { "alert" } else { "status" }, "{message}" }
                        }
                        if selected_support.is_some_and(|support| support.unlinked) {
                            p { class: "m1-case-edit-hint", "This gasket is unlinked from its pair." }
                        }
                    } else {
                        p { class: "m1-case-edit-hint", "Select a gasket in Objects to unlink its mirrored support pair." }
                    }
                }
                if !inline_case_controls {
                button { onclick: move |_| run_host(&fit, |host| host.fit(), &mut status), "{canvas_context.fit_label()}" }
                details { class: "m1-case-view-settings",
                    summary { "View controls" }
                    div { class: "m1-case-view-settings-body",
            div { role: "group", "aria-label": "{canvas_context.camera_label()}",
                button { onclick: move |_| run_host(&top, |host| host.view("top"), &mut status), "Top view" }
                button { onclick: move |_| run_host(&bottom, |host| host.view("bottom"), &mut status), "Bottom view" }
                button { onclick: move |_| run_host(&iso, |host| host.view("isometric"), &mut status), "Isometric view" }
                button { onclick: move |_| run_host(&left, |host| host.orbit(-50.0, 0.0), &mut status), "Rotate left" }
                button { onclick: move |_| run_host(&right, |host| host.orbit(50.0, 0.0), &mut status), "Rotate right" }
                button { onclick: move |_| run_host(&zoom_in, |host| host.zoom(0.85), &mut status), "Zoom in" }
                button { onclick: move |_| run_host(&zoom_out, |host| host.zoom(1.15), &mut status), "Zoom out" }
            }
            div { role: "group", "aria-label": "{canvas_context.display_label()}",
                for mode in [RenderMode::Shaded, RenderMode::Wireframe, RenderMode::Hybrid] {
                    {
                        let mut transient = transient;
                        let label = mode.renderer_value();
                        rsx! { button {
                            "aria-pressed": transient().mode == mode,
                            onclick: move |_| transient.with_mut(|view| view.mode = mode),
                            "{label}"
                        } }
                    }
                }
            }
            div { role: "group", "aria-label": "{canvas_context.assembly_label()}",
                for view in [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section] {
                    {
                        let mut transient = transient;
                        let label = view.renderer_value();
                        rsx! { button {
                            "aria-pressed": transient().assembly == view,
                            onclick: move |_| transient.with_mut(|state| state.assembly = view),
                            "{label}"
                        } }
                    }
                }
            }
            if transient().assembly == AssemblyView::Exploded {
                label { "Explode amount"
                    input {
                        r#type: "range", min: "0", max: "10", step: "0.1",
                        value: "{transient().explode_amount}",
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                transient.with_mut(|state| state.explode_amount = value.clamp(0.0, 10.0));
                            }
                        }
                    }
                }
            }
            if transient().assembly == AssemblyView::Section {
                label { "Section plane"
                    select {
                        value: "{transient().section_plane}",
                        onchange: move |event: FormEvent| {
                            transient.with_mut(|state| state.section_plane = match event.value().as_str() { "XY" => "XY", "XZ" => "XZ", _ => "YZ" });
                        },
                        option { value: "XY", "XY" }
                        option { value: "XZ", "XZ" }
                        option { value: "YZ", "YZ" }
                    }
                }
                label { "Section position"
                    input {
                        r#type: "range", min: "-100", max: "100", step: "1",
                        value: "{transient().section_position}",
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                transient.with_mut(|state| state.section_position = value.clamp(-100.0, 100.0));
                            }
                        }
                    }
                }
                label { input {
                    r#type: "checkbox", checked: transient().show_section_plane,
                    onchange: move |_| transient.with_mut(|state| state.show_section_plane = !state.show_section_plane),
                } "Show section plane" }
            }
            label { input {
                r#type: "checkbox", checked: transient().show_hidden,
                onchange: move |_| transient.with_mut(|state| state.show_hidden = !state.show_hidden),
            } "Show hidden lines" }
                    }
                }
                }
            }
            if inline_case_controls && show_assembly_controls && transient().assembly == AssemblyView::Exploded {
                label { class: "m1-case-view-adjustment", "Explode amount"
                    input {
                        r#type: "range", min: "0", max: "10", step: "0.1",
                        value: "{transient().explode_amount}",
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                transient.with_mut(|state| state.explode_amount = value.clamp(0.0, 10.0));
                            }
                        }
                    }
                }
            }
            if inline_case_controls && show_assembly_controls && transient().assembly == AssemblyView::Section {
                div { class: "m1-case-view-adjustments",
                    label { "Section plane"
                        select {
                            value: "{transient().section_plane}",
                            onchange: move |event: FormEvent| {
                                transient.with_mut(|state| state.section_plane = match event.value().as_str() { "XY" => "XY", "XZ" => "XZ", _ => "YZ" });
                            },
                            option { value: "XY", "XY" }
                            option { value: "XZ", "XZ" }
                            option { value: "YZ", "YZ" }
                        }
                    }
                    label { "Section position"
                        input {
                            r#type: "range", min: "-100", max: "100", step: "1",
                            value: "{transient().section_position}",
                            oninput: move |event: FormEvent| {
                                if let Ok(value) = event.value().parse::<f32>() {
                                    transient.with_mut(|state| state.section_position = value.clamp(-100.0, 100.0));
                                }
                            }
                        }
                    }
                    label { input {
                        r#type: "checkbox", checked: transient().show_section_plane,
                        onchange: move |_| transient.with_mut(|state| state.show_section_plane = !state.show_section_plane),
                    } "Show plane" }
                }
            }
            div { class: "m1-case-view-canvas-shell",
                if let Some(reference) = selected_reference.as_deref() {
                    div { class: "m1-case-picked-part-badge", "aria-hidden": "true", "{reference}" }
                }
                canvas {
                    style: "width:100%;height:100%;display:block",
                    tabindex: "0", role: "img",
                    "aria-label": "{canvas_context.accessible_name()}",
                    onmounted: mount_host,
                    onpointerdown: on_pointer_down,
                    onpointermove: on_pointer_move,
                    onpointerup: on_pointer_up,
                    onpointercancel: on_pointer_cancel,
                    onlostpointercapture: on_pointer_cancel,
                    onkeydown: on_key_down,
                    onwheel: on_wheel,
                }
                if inline_case_controls {
                    div { class: "m1-case-camera-controls", role: "group", "aria-label": "Assembly camera",
                        button { onclick: move |_| run_host(&case_fit, |host| host.fit(), &mut status), "Fit" }
                        button { onclick: move |_| run_host(&case_top, |host| host.view("top"), &mut status), "Top" }
                        button { onclick: move |_| run_host(&case_bottom, |host| host.view("bottom"), &mut status), "Bottom" }
                        button { onclick: move |_| run_host(&case_iso, |host| host.view("isometric"), &mut status), "Isometric" }
                    }
                }
                CaseAssemblyLayers {
                    assembly: assembly_layers,
                    components: component_layers,
                    selectable_layers: selectable_layers.clone(),
                    selected_layer: selected_layer.clone(),
                    on_select_layer: move |id| select_assembly_layer(id),
                    display: display.clone(),
                    on_display_change: assembly_change_display,
                }
                if let Some(message) = gesture_message.as_deref() {
                    p { class: "m1-case-edit-hint", role: "status", "{message}" }
                }
                p { class: "m1-case-view-status", role: if status().to_ascii_lowercase().contains("unavailable") || status().contains("failed") { "alert" } else { "status" }, "aria-live": "polite", "{status()}" }
            }
            if !inline_case_controls {
            details {
                summary { "Case layers and colors" }
                div { role: "group", "aria-label": "Case layers",
                for (id, label) in &projection.layers {
                    {
                    let id = id.clone();
                    let label = label.clone();
                    let ids = preference_ids(&id);
                    let visible = !ids.iter().all(|entry| display.hidden.contains(entry));
                    let color = display.color(&id).unwrap_or("#b0b5bd").to_owned();
                    let has_color = display.has_color(&id);
                    let toggle_next = display.clone();
                    let color_next = display.clone();
                    let reset_next = display.clone();
                    let toggle_display = change_display.clone();
                    let select_layer = select_layer.clone();
                    let set_color = change_display.clone();
                    let reset_color = change_display.clone();
                    let select_id = id.clone();
                    let toggle_id = id.clone();
                    let color_id = id.clone();
                    let reset_id = id.clone();
                    rsx! {
                        div { class: "m1-viewer-layer", key: "{id}",
                            button {
                                "aria-pressed": selected_layer == id,
                                onclick: move |_| select_layer(select_id.clone()),
                                "Select {label}"
                            }
                            button {
                                "aria-pressed": visible,
                                "aria-label": "Toggle visibility for {label}",
                                onclick: move |_| {
                                    let mut next_display = toggle_next.clone();
                                    let aliases = preference_ids(&toggle_id);
                                    let is_hidden = aliases.iter().all(|entry| next_display.hidden.contains(entry));
                                    if is_hidden {
                                        next_display.hidden.retain(|entry| !aliases.contains(entry));
                                    } else {
                                        for entry in aliases {
                                            if !next_display.hidden.contains(&entry) { next_display.hidden.push(entry); }
                                        }
                                    }
                                    toggle_display(next_display.clone());
                                },
                                if visible { "Visible" } else { "Hidden" }
                            }
                            label { "Color for {label}"
                                input {
                                    r#type: "color", value: "{color}",
                                    oninput: move |event: FormEvent| {
                                        let mut next_display = color_next.clone();
                                        next_display.set_color(&color_id, &event.value());
                                        set_color(next_display.clone());
                                    }
                                }
                            }
                            button {
                                disabled: !has_color,
                                onclick: move |_| {
                                    let mut next_display = reset_next.clone();
                                    next_display.set_color(&reset_id, "");
                                    reset_color(next_display.clone());
                                },
                                "Reset color"
                            }
                        }
                    }
                    }
                }
                }
            }
            }
        }
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
    host: &Rc<RefCell<Option<RendererPageHost>>>,
    operation: impl FnOnce(&RendererPageHost) -> Result<(), String>,
    status: &mut Signal<String>,
) {
    if let Some(host) = host.borrow().as_ref()
        && let Err(error) = operation(host)
    {
        status.set(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_application::SessionEpoch;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn identity() -> ViewerIdentity {
        ViewerIdentity {
            scope: Scope {
                session_epoch: SessionEpoch(3),
                document_id: "project-1".to_owned(),
                board_id: "board-1".to_owned(),
                instance_id: Some("case-instance-1".to_owned()),
            },
            snapshot_token: SnapshotToken(8),
            revision: 9,
            viewer_instance: 2,
            projection_generation: 4,
            renderer_sequence: 6,
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn generic_layout_module_projection_preserves_board_holes_and_omits_opening_voids() {
        use boardstudio_core::model::{
            CaseOpening, ModuleAttachment, ModuleSupportGeometry, MountedModule, ProjectDoc,
            ResolvedModule, Side, Vec2,
        };

        let mut document = ProjectDoc::empty("project-1", "Generic module");
        document.modules.push(MountedModule {
            id: "placement-generic".into(),
            definition_id: "definition-generic".into(),
            host_board_id: "board-1".into(),
            host_instance_id: None,
            host_face: Side::Front,
            facing_face: Side::Front,
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            gap: 0.0,
            attachment: ModuleAttachment::Board,
            detached: false,
            connection: None,
            service_clearance: 0.0,
            mount_supports: vec![],
        });
        let polygon = vec![
            Vec2 { x: 0.0, y: 0.0 },
            Vec2 { x: 6.0, y: 0.0 },
            Vec2 { x: 6.0, y: 2.0 },
            Vec2 { x: 2.0, y: 2.0 },
            Vec2 { x: 2.0, y: 6.0 },
            Vec2 { x: 0.0, y: 6.0 },
        ];
        let module = ResolvedModule {
            id: "placement-generic".into(),
            definition_id: "definition-generic".into(),
            at: Vec2 { x: 0.0, y: 0.0 },
            rotation: 0.0,
            midplane_z: 1.6,
            flipped: false,
            board: vec![CaseOpening {
                points: polygon.clone(),
                z: 0.0,
                height: 1.2,
            }],
            board_holes: vec![boardstudio_core::model::Contour {
                points: vec![
                    Vec2 { x: 0.5, y: 0.5 },
                    Vec2 { x: 0.5, y: 1.5 },
                    Vec2 { x: 1.5, y: 1.5 },
                    Vec2 { x: 1.5, y: 0.5 },
                ],
                hole: true,
            }],
            volumes: vec![boardstudio_core::model::ModuleVolume {
                id: "volume-1".into(),
                geometry: CaseOpening {
                    points: polygon,
                    z: 1.2,
                    height: 0.4,
                },
                purpose: "assembly".into(),
                source: "fixture".into(),
                qualified: true,
            }],
            openings: vec![boardstudio_core::model::ModuleVolume {
                id: "clearance-void".into(),
                geometry: CaseOpening {
                    points: vec![
                        Vec2 { x: 2.5, y: 2.5 },
                        Vec2 { x: 3.5, y: 2.5 },
                        Vec2 { x: 3.5, y: 3.5 },
                    ],
                    z: 1.0,
                    height: 3.0,
                },
                purpose: "connector-clearance".into(),
                source: "fixture".into(),
                qualified: true,
            }],
            mounts: vec![],
            mount_supports: vec![ModuleSupportGeometry {
                mount_id: "support-1".into(),
                at: Vec2 { x: 4.0, y: 1.0 },
                outer_diameter: 2.0,
                hole_diameter: 0.8,
                z: 0.0,
                height: 1.0,
            }],
            footprints: vec![],
            models: vec![],
            gates: vec![],
        };

        let bodies = layout_module_bodies(&[module], &document, "board-1", 1.6);
        assert_eq!(bodies.len(), 3, "PCB, assembly volume and standoff only");
        assert_eq!(bodies[0]["id"], "module:placement-generic:pcb:0");
        assert_eq!(bodies[0]["contours"].as_array().unwrap().len(), 2);
        assert_eq!(bodies[0]["contours"][1]["hole"], true);
        assert_eq!(
            bodies[0]["contours"][0]["points"].as_array().unwrap().len(),
            6
        );
        assert_eq!(bodies[1]["id"], "module:placement-generic:volume:0");
        assert_eq!(
            bodies[2]["id"],
            "module:placement-generic:standoff:support-1"
        );
        assert_eq!(bodies[2]["contours"][1]["hole"], true);
        assert!(
            bodies
                .iter()
                .all(|body| !body["id"].as_str().unwrap().contains("clearance-void"))
        );
    }

    fn imported_layout_projection(
        reference: boardstudio_core::model::BoardReference,
        model_rows: Option<ModelDeliveryRows>,
    ) -> RendererSceneProjection {
        use boardstudio_core::model::{
            Asset, Board, BoardContours, PcbPreview, ProjectDoc, Readiness, SceneDelta,
        };
        use std::{collections::BTreeMap, sync::Arc};

        let viewer = identity();
        let mut document = ProjectDoc::empty("project-1", "Routed board");
        document.revision = viewer.revision;
        document.boards.push(Board {
            id: "board-1".into(),
            name: "Board".into(),
            outline_ids: vec![],
            part_ids: vec![],
            net_ids: vec![],
            thickness: 1.6,
            traces: vec![],
            vias: vec![],
        });
        document.assets.push(Asset {
            id: "routed-source".into(),
            name: "board.kicad_pcb".into(),
            media_type: "application/vnd.kicad.pcb".into(),
            sha256: "ab".repeat(32),
            license: None,
            source: None,
        });
        document.board_references.push(reference);
        let accepted = boardstudio_application::AcceptedSnapshot {
            token: viewer.snapshot_token,
            session_epoch: viewer.scope.session_epoch,
            document: Arc::new(document),
            scene: Arc::new(SceneDelta {
                module_scenes: vec![],
                revision: viewer.revision,
                transaction_id: "test".into(),
                changed_ids: vec![],
                transforms: vec![],
                matrix_scenes: vec![],
                contours: vec![],
                board_contours: vec![BoardContours {
                    board_id: "board-1".into(),
                    contours: vec![],
                }],
                board_readiness: vec![],
                board_outline_scenes: vec![],
                finding_markers: vec![],
                findings: vec![],
                readiness: Readiness {
                    layout: false,
                    outline: false,
                    pcb: false,
                    case_ready: false,
                },
            }),
        };
        let capture = super::super::layout_viewer_source::LayoutSourceCapture::capture(
            &accepted,
            &viewer.scope,
            1,
            "layout-preview-test".into(),
            BTreeMap::new(),
        )
        .unwrap();
        let preview = capture
            .accept_preview(PcbPreview {
                revision: viewer.revision,
                thickness: 1.6,
                contours: vec![],
                surfaces: vec![],
                holes: vec![],
                models: vec![],
                diagnostics: vec![],
            })
            .unwrap();
        project_layout_preview(&preview, viewer, "light", model_rows.as_ref(), None).unwrap()
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn imported_layout_projection_preserves_nonidentity_board_reference_pose_and_elevation() {
        use boardstudio_core::model::{BoardReference, Pose2, Vec2};
        use std::collections::BTreeMap;

        let projection = imported_layout_projection(
            BoardReference {
                id: "routed-board-1".into(),
                board_id: "board-1".into(),
                asset_id: "routed-source".into(),
                enabled: true,
                pose: Pose2 {
                    at: Vec2 { x: 17.25, y: -8.5 },
                    rotation: 31.0,
                },
                elevation: 4.75,
                model_assets: BTreeMap::new(),
            },
            None,
        );
        let serialized = js_sys::JSON::stringify(&projection.input)
            .unwrap()
            .as_string()
            .unwrap();
        let packet: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            packet["reference"],
            serde_json::json!({
                "pose": {
                    "at": { "x": 17.25, "y": -8.5 },
                    "rotation": 31
                },
                "elevation": 4.75
            })
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn layout_packet_includes_module_model_mesh_at_core_stable_id_and_transform() {
        use super::super::model_delivery::{DeliveredModel, ModelDeliveryRows, ValidatedMesh};
        use boardstudio_core::model::{BoardReference, Pose2, Vec2};
        use std::collections::BTreeMap;

        let placement_id = "module-model/placement-a/0".to_owned();
        let matrix = [
            1.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 12.0, 13.0, 14.0, 1.0,
        ];
        let rows = ModelDeliveryRows {
            delivered: vec![DeliveredModel {
                id: placement_id.clone(),
                mesh: Rc::new(ValidatedMesh {
                    positions: Rc::from([1.0_f32, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]),
                    normals: Rc::from([1.0_f32, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0]),
                    colors: None,
                }),
                matrix: Some(matrix),
            }],
            pending: Vec::new(),
            failures: Vec::new(),
        };
        let projection = imported_layout_projection(
            BoardReference {
                id: "routed-board-1".into(),
                board_id: "board-1".into(),
                asset_id: "routed-source".into(),
                enabled: true,
                pose: Pose2 {
                    at: Vec2 { x: 0.0, y: 0.0 },
                    rotation: 0.0,
                },
                elevation: 0.0,
                model_assets: BTreeMap::new(),
            },
            Some(rows),
        );
        let serialized = js_sys::JSON::stringify(&projection.input)
            .unwrap()
            .as_string()
            .unwrap();
        let packet: serde_json::Value = serde_json::from_str(&serialized).unwrap();

        assert_eq!(packet["bodies"][0]["id"], placement_id);
        assert_eq!(packet["bodies"][0]["mesh"]["positions"][0], 13.0);
        assert_eq!(packet["bodies"][0]["mesh"]["positions"][1], 13.0);
        assert_eq!(packet["bodies"][0]["mesh"]["positions"][2], 14.0);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn layout_module_model_normals_follow_nonuniform_core_scale() {
        let diagonal = std::f32::consts::FRAC_1_SQRT_2;
        let matrix = [
            2.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
        ];
        let (_, normals) =
            transformed_module_mesh(&[0.0, 0.0, 0.0], &[diagonal, diagonal, 0.0], &matrix);
        assert!((normals[0] - 1.0 / 5.0_f32.sqrt()).abs() < 1e-5);
        assert!((normals[1] - 2.0 / 5.0_f32.sqrt()).abs() < 1e-5);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn finding_focus_retriggers_repeats_and_rejects_stale_project_board_token_and_revision() {
        let identity = identity();
        let request = ViewerFocusRequest {
            scope: identity.scope.clone(),
            snapshot_token: identity.snapshot_token,
            revision: identity.revision,
            navigation_id: 1,
            target_ids: vec!["plate".into()],
        };
        assert!(focus_request_matches(&request, &identity));

        let focused = super::super::keycaps_finding_marker::FocusedFinding {
            scope: identity.scope.clone(),
            token: identity.snapshot_token,
            revision: identity.revision,
            finding_id: "finding-1".into(),
            navigation_id: request.navigation_id,
        };
        let mut repeated = request.clone();
        repeated.navigation_id =
            super::super::keycaps_finding_marker::next_navigation_id(Some(&focused));
        assert_ne!(repeated, request);
        assert!(focus_request_matches(&repeated, &identity));

        let mut stale = request.clone();
        stale.scope.document_id.push_str("-other");
        assert!(!focus_request_matches(&stale, &identity));
        let mut stale = request.clone();
        stale.scope.board_id.push_str("-other");
        assert!(!focus_request_matches(&stale, &identity));
        let mut stale = request.clone();
        stale.snapshot_token = SnapshotToken(identity.snapshot_token.0 + 1);
        assert!(!focus_request_matches(&stale, &identity));
        let mut stale = request;
        stale.revision += 1;
        assert!(!focus_request_matches(&stale, &identity));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn physical_preview_rows_are_qualified_by_source_scope_and_token() {
        let viewer = identity();
        let owner = crate::case_preview::CasePreviewOwnerIdentity {
            scope: viewer.scope.clone(),
            snapshot_token: viewer.snapshot_token,
            accepted_revision: 11,
            accepted_scene_identity: 12,
            viewer_instance: 13,
            projection_generation: 14,
            batch_generation: 15,
            core_executor_epoch: 16,
            core_worker_identity: 17,
            request_token: "case-preview-1".to_owned(),
        };

        assert!(preview_matches_source(
            &owner,
            &viewer.scope,
            viewer.snapshot_token
        ));
        assert!(!preview_matches_source(
            &owner,
            &viewer.scope,
            SnapshotToken(viewer.snapshot_token.0 + 1)
        ));
        let mut other_scope = viewer.scope.clone();
        other_scope.board_id = "board-2".to_owned();
        assert!(!preview_matches_source(
            &owner,
            &other_scope,
            viewer.snapshot_token
        ));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn viewer_events_reject_changed_owner_identity_and_unmount() {
        let identity = identity();
        let owner = Rc::new(ViewerOwner {
            active: Cell::new(true),
            identity: RefCell::new(Some(identity.clone())),
            last_inputs: RefCell::new(None),
            projection_generation: Cell::new(identity.projection_generation),
            renderer_sequence: Cell::new(identity.renderer_sequence),
            viewer_instance: identity.viewer_instance,
        });
        let signal = ScopedViewerSignal {
            identity: identity.clone(),
            kind: ViewerSignalKind::Picked("body-1".to_owned()),
            owner: Rc::downgrade(&owner),
        };
        assert!(signal.is_current());

        let mut changed = identity.clone();
        changed.renderer_sequence += 1;
        *owner.identity.borrow_mut() = Some(changed);
        assert!(!signal.is_current());

        *owner.identity.borrow_mut() = Some(identity);
        assert!(signal.is_current());
        owner.active.set(false);
        assert!(!signal.is_current());
    }

    fn gesture_scene(identity: &ViewerIdentity) -> Rc<CadScene> {
        use boardstudio_core::model::{PreparedCaseAssemblyIR, ProjectDoc, Readiness, SceneDelta};
        use std::sync::Arc;
        let mut document = ProjectDoc::empty(&identity.scope.document_id, "Gesture source");
        document.revision = identity.revision;
        Rc::new(CadScene {
            scope: identity.scope.clone(),
            token: identity.snapshot_token,
            snapshot: boardstudio_application::AcceptedSnapshot {
                token: identity.snapshot_token,
                session_epoch: identity.scope.session_epoch,
                document: Arc::new(document),
                scene: Arc::new(SceneDelta {
                    module_scenes: vec![],
                    revision: identity.revision,
                    transaction_id: String::new(),
                    changed_ids: vec![],
                    transforms: vec![],
                    matrix_scenes: vec![],
                    contours: vec![],
                    board_contours: vec![],
                    board_readiness: vec![],
                    board_outline_scenes: vec![],
                    finding_markers: vec![],
                    findings: vec![],
                    readiness: Readiness {
                        layout: true,
                        outline: true,
                        pcb: true,
                        case_ready: true,
                    },
                }),
            },
            result: boardstudio_web::cad_jobs::CadResult::default(),
            prepared: PreparedCaseAssemblyIR {
                revision: identity.revision,
                bodies: vec![],
            },
            physical_fingerprint: None,
            mechanical: None,
            exact: true,
            contours: vec![],
        })
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn case_handle_survives_published_provisional_scene_until_pointer_up() {
        let source_identity = identity();
        let accepted = gesture_scene(&source_identity);
        let owner = ViewerOwner::new().unwrap();
        let mut inputs = ProjectionInputs {
            source: Rc::as_ptr(&accepted) as usize,
            preview: 0,
            models: vec![],
            theme: "light".into(),
        };
        let started = ViewerSource::Cad(accepted.clone())
            .identity(&owner, &inputs)
            .unwrap();
        let pointer = Rc::new(RefCell::new(Some(PointerOwner::Handle {
            identity: started.clone(),
            pointer_id: 9,
            z: 0.0,
            id: "mount-1".into(),
            projection: Some(CaseHandleProjection {
                source: accepted.clone(),
                inputs: inputs.clone(),
            }),
        })));
        let mut preview = crate::case_gesture_preview::CaseGesturePreviewState::default();
        let preview_owner = preview
            .begin(
                started.scope.clone(),
                started.snapshot_token,
                started.revision,
            )
            .unwrap();
        assert!(preview.publish(&preview_owner, gesture_scene(&source_identity)));
        let display_scene = preview
            .scene(
                &accepted.scope,
                accepted.token,
                accepted.snapshot.document.revision,
            )
            .unwrap();
        assert!(!Rc::ptr_eq(&accepted, &display_scene));
        inputs.source = Rc::as_ptr(&display_scene) as usize;
        let current = ViewerSource::Cad(display_scene)
            .identity(&owner, &inputs)
            .unwrap();
        assert!(current.projection_generation > started.projection_generation);
        assert!(current.renderer_sequence > started.renderer_sequence);

        let handle_projection = CaseHandleProjection {
            source: accepted.clone(),
            inputs,
        };
        cancel_superseded_pointer(
            &pointer,
            &Rc::new(RefCell::new(None)),
            &owner,
            Some(&handle_projection),
            true,
        );
        assert!(
            pointer.borrow().is_some(),
            "Publishing this gesture's preview must retain its handle capture"
        );
        assert!(
            take_pointer_for_event(&pointer, 9, &started).is_none(),
            "Old callbacks must stay stale"
        );
        let dom = VirtualDom::new(|| rsx! { div {} });
        let delivered = Rc::new(RefCell::new(Vec::new()));
        let on_signal = dom.in_scope(ScopeId::ROOT, {
            let delivered = delivered.clone();
            move || {
                EventHandler::new(move |event: ScopedViewerSignal| {
                    assert!(
                        event.is_current(),
                        "Case must receive a current owner stamp"
                    );
                    if let ViewerSignalKind::HandleGesture { phase, .. } = event.kind {
                        delivered.borrow_mut().push(phase);
                    }
                })
            }
        });
        // The same admission used by Move/Up must see the current full identity,
        // including the renderer sequence, while old emissions remain rejected.
        let moved =
            take_pointer_for_event(&pointer, 9, &current).expect("Move must still own the handle");
        let applied_identity = current.clone();
        assert_eq!(moved.identity(), &applied_identity);
        emit_signal(
            &owner,
            on_signal,
            &started,
            ViewerSignalKind::HandleGesture {
                phase: HandleGesturePhase::End,
                handle_id: "mount-1".into(),
                point: None,
            },
        );
        assert!(delivered.borrow().is_empty());
        emit_signal(
            &owner,
            on_signal,
            moved.identity(),
            ViewerSignalKind::HandleGesture {
                phase: HandleGesturePhase::Move,
                handle_id: "mount-1".into(),
                point: Some([1.0, 2.0, 0.0]),
            },
        );
        *pointer.borrow_mut() = Some(moved);
        let completed = take_pointer_for_event(&pointer, 9, &current)
            .expect("Pointer-up must still own the handle");
        emit_signal(
            &owner,
            on_signal,
            completed.identity(),
            ViewerSignalKind::HandleGesture {
                phase: HandleGesturePhase::End,
                handle_id: "mount-1".into(),
                point: Some([1.0, 2.0, 0.0]),
            },
        );
        assert_eq!(
            *delivered.borrow(),
            [HandleGesturePhase::Move, HandleGesturePhase::End]
        );
        assert!(
            preview.cancel(&preview_owner),
            "End must retire the provisional preview"
        );
        assert!(
            preview
                .scene(
                    &accepted.scope,
                    accepted.token,
                    accepted.snapshot.document.revision
                )
                .is_none()
        );
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn case_handle_preview_transfer_rejects_replaced_source_scope_revision_and_owner() {
        for change in [
            "source", "session", "document", "board", "instance", "token", "revision", "viewer",
            "unmount", "stale", "models", "theme", "preview", "missing", "orbit",
        ] {
            let accepted = gesture_scene(&identity());
            let owner = ViewerOwner::new().unwrap();
            let inputs = ProjectionInputs {
                source: Rc::as_ptr(&accepted) as usize,
                preview: 0,
                models: vec![],
                theme: "light".into(),
            };
            let started = ViewerSource::Cad(accepted.clone())
                .identity(&owner, &inputs)
                .unwrap();
            let mut next = CaseHandleProjection {
                source: accepted.clone(),
                inputs: inputs.clone(),
            };
            let display_scene = gesture_scene(&started);
            next.inputs.source = Rc::as_ptr(&display_scene) as usize;
            let mut current = ViewerSource::Cad(display_scene)
                .identity(&owner, &next.inputs)
                .unwrap();
            match change {
                "source" => next.source = gesture_scene(&started),
                "session" => current.scope.session_epoch = SessionEpoch(99),
                "document" => current.scope.document_id.push_str("-other"),
                "board" => current.scope.board_id.push_str("-other"),
                "instance" => current.scope.instance_id = None,
                "token" => current.snapshot_token = SnapshotToken(99),
                "revision" => current.revision += 1,
                "viewer" => current.viewer_instance += 1,
                "unmount" => owner.active.set(false),
                "models" => next.inputs.models.push(("model".into(), 1)),
                "theme" => next.inputs.theme = "dark".into(),
                "preview" => next.inputs.preview += 1,
                _ => {}
            }
            *owner.identity.borrow_mut() = Some(current);
            let active = if change == "orbit" {
                PointerOwner::Orbit {
                    identity: started,
                    pointer_id: 9,
                    last_x: 0.0,
                    last_y: 0.0,
                    start_x: 0.0,
                    start_y: 0.0,
                    moved: false,
                    picked: None,
                }
            } else {
                PointerOwner::Handle {
                    identity: started,
                    projection: Some(CaseHandleProjection {
                        source: accepted,
                        inputs,
                    }),
                    pointer_id: 9,
                    z: 0.0,
                    id: "mount-1".into(),
                }
            };
            let pointer = Rc::new(RefCell::new(Some(active)));
            cancel_superseded_pointer(
                &pointer,
                &Rc::new(RefCell::new(None)),
                &owner,
                (change != "missing").then_some(&next),
                change != "stale",
            );
            assert!(
                pointer.borrow().is_none(),
                "{change} must cancel the old capture"
            );
        }
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn pointer_completion_keeps_its_starting_projection_identity() {
        let started = identity();
        let mut current = started.clone();
        current.projection_generation += 1;
        current.renderer_sequence += 1;
        let pointer = PointerOwner::Orbit {
            identity: started.clone(),
            pointer_id: 9,
            last_x: 10.0,
            last_y: 20.0,
            start_x: 10.0,
            start_y: 20.0,
            moved: false,
            picked: Some("body-1".to_owned()),
        };
        assert_eq!(pointer.identity(), &started);
        assert_ne!(pointer.identity(), &current);
        let owner = ViewerOwner {
            active: Cell::new(true),
            identity: RefCell::new(Some(current)),
            last_inputs: RefCell::new(None),
            projection_generation: Cell::new(5),
            renderer_sequence: Cell::new(7),
            viewer_instance: started.viewer_instance,
        };
        assert!(!owner_is_current(&owner, pointer.identity()));
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn pointer_event_cannot_take_reused_id_from_another_projection() {
        let started = identity();
        let mut current = started.clone();
        current.projection_generation += 1;
        current.renderer_sequence += 1;
        let pointer = Rc::new(RefCell::new(Some(PointerOwner::Orbit {
            identity: current.clone(),
            pointer_id: 9,
            last_x: 10.0,
            last_y: 20.0,
            start_x: 10.0,
            start_y: 20.0,
            moved: false,
            picked: Some("body-1".to_owned()),
        })));

        assert!(take_pointer_for_event(&pointer, 9, &started).is_none());
        assert_eq!(
            pointer.borrow().as_ref().map(PointerOwner::identity),
            Some(&current)
        );
        assert!(take_pointer_for_event(&pointer, 9, &current).is_some());
        assert!(pointer.borrow().is_none());
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn display_preference_aliases_match_case_display_groups() {
        assert_eq!(
            preference_ids("pcb"),
            ["PCB", "Models", "Keycaps", "Copper", "Mask", "Silkscreen"].map(str::to_owned)
        );
        assert_eq!(preference_ids("gaskets"), ["Gaskets"]);
        assert_eq!(
            preference_ids("gasket:plate:lower"),
            ["gasket:plate:lower", "gasket:plate:upper"]
        );
        assert_eq!(preference_ids("case-body-1"), ["case-body-1"]);
    }

    #[wasm_bindgen_test::wasm_bindgen_test]
    fn canvas_accessible_names_match_the_active_workflow_context() {
        assert!(
            ViewerCanvasContext::Layout
                .accessible_name()
                .contains("Layout PCB assembly")
        );
        assert!(
            ViewerCanvasContext::Layout
                .accessible_name()
                .contains("current Layout part")
        );
        assert!(
            ViewerCanvasContext::Case
                .accessible_name()
                .contains("3D Case preview")
        );
        assert!(
            !ViewerCanvasContext::Layout
                .accessible_name()
                .contains("Case preview")
        );
        assert!(
            ViewerCanvasContext::Keymap
                .accessible_name()
                .contains("3D Keymap board preview")
        );
        assert!(
            ViewerCanvasContext::Keycaps
                .accessible_name()
                .contains("3D Keycaps board preview")
        );
    }
}
