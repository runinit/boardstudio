//! Private page viewer shared by workflow-owned, immutable scene projections.
//!
//! Case is the first consumer. Its wrapper owns the Case-to-renderer projection;
//! this module owns only renderer controls, transient view state and host lifetime.
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViewerIdentity {
    pub(crate) scope: Scope,
    pub(crate) snapshot_token: SnapshotToken,
    pub(crate) viewer_instance: u64,
    pub(crate) projection_generation: u64,
    pub(crate) renderer_sequence: u64,
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
    handles: Vec<ViewerHandle>,
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
    model_rows: Option<ModelDeliveryRows>,
    selected_layer: String,
    display: CaseDisplay,
    resolved_theme: String,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let theme = resolved_theme;
    let owner = match use_hook(ViewerOwner::new) {
        Ok(owner) => owner,
        Err(error) => {
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
    let Some(source) = scene
        .map(ViewerSource::Cad)
        .or_else(|| preview.clone().map(ViewerSource::Native))
    else {
        return rsx! { p { role: "alert", "3D preview source is unavailable." } };
    };
    let inputs = ProjectionInputs {
        source: source.pointer(),
        preview: preview
            .as_ref()
            .map_or(0, |preview| Rc::as_ptr(preview) as usize),
        models: model_rows
            .as_ref()
            .map(|rows| {
                rows.delivered
                    .iter()
                    .map(|model| (model.id.clone(), Rc::as_ptr(&model.mesh) as usize))
                    .collect()
            })
            .unwrap_or_default(),
        theme: theme.clone(),
    };
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
            preview.as_deref(),
            model_rows.as_ref(),
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
        Rc::new(move || source.is_current(&runtime))
    });
    rsx! {
        SharedViewer {
            projection,
            selected_layer,
            display,
            theme,
            owner,
            current_source,
            on_signal,
            on_display_change,
        }
    }
}

#[derive(Clone)]
enum ViewerSource {
    Cad(Rc<CadScene>),
    Native(Rc<NativePreviewSnapshot>),
}

impl ViewerSource {
    fn same(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Cad(left), Self::Cad(right)) => Rc::ptr_eq(left, right),
            (Self::Native(left), Self::Native(right)) => Rc::ptr_eq(left, right),
            _ => false,
        }
    }

    fn pointer(&self) -> usize {
        match self {
            Self::Cad(scene) => Rc::as_ptr(scene) as usize,
            Self::Native(preview) => Rc::as_ptr(preview) as usize,
        }
    }

    fn identity(
        &self,
        owner: &ViewerOwner,
        inputs: &ProjectionInputs,
    ) -> Result<ViewerIdentity, String> {
        let (scope, token) = match self {
            Self::Cad(scene) => (&scene.scope, scene.token),
            Self::Native(preview) => (&preview.owner.scope, preview.owner.snapshot_token),
        };
        owner.advance_source(scope.clone(), token, inputs)
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
                    && runtime
                        .cad_scene()
                        .as_ref()
                        .is_some_and(|current| Rc::ptr_eq(current, scene))
            }
            Self::Native(preview) => runtime.native_case_preview().is_some_and(|current| {
                current.owner == preview.owner && Rc::ptr_eq(&current.lease, &preview.lease)
            }),
        }
    }
}

fn project_source(
    source: &ViewerSource,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
) -> Result<RendererSceneProjection, String> {
    match source {
        ViewerSource::Cad(scene) => {
            project_case_scene(scene.clone(), identity, theme, preview, model_rows)
        }
        ViewerSource::Native(preview) => {
            project_native_preview(preview, identity, theme, model_rows)
        }
    }
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
        handles: Vec::new(),
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
        handles: Vec::new(),
    })
}

fn loaded_model_inputs(rows: Option<&ModelDeliveryRows>) -> Array {
    let loaded = Array::new();
    if let Some(rows) = rows {
        for model in &rows.delivered {
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

#[derive(Clone, Debug)]
struct ViewerHandle {
    id: String,
    x: f32,
    y: f32,
    z: f32,
    tangent_x: f32,
    tangent_y: f32,
    normal_x: f32,
    normal_y: f32,
    length: f32,
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
    selected_layer: String,
    display: CaseDisplay,
    theme: String,
    owner: Rc<ViewerOwner>,
    current_source: SourceGuard,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
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

    use_effect(use_reactive((&projection,), {
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let owner = owner.clone();
        move |(_projection,)| {
            cancel_superseded_pointer(&pointer, &canvas, &owner);
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

    use_effect(use_reactive((&projection, &mounted()), {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, mounted)| {
            if !mounted
                || !owner.active.get()
                || !(current_source.0)()
                || owner.identity.borrow().as_ref() != Some(&projection.identity)
            {
                return;
            }
            let result = handles_value(&projection.handles).and_then(|handles| {
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
    }));

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

    let on_pointer_down = {
        let projection = projection.clone();
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let handles = projection.handles.clone();
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
    let fit = host.clone();
    let top = host.clone();
    let bottom = host.clone();
    let iso = host.clone();
    let left = host.clone();
    let right = host.clone();
    let zoom_in = host.clone();
    let zoom_out = host.clone();

    rsx! {
        div { class: "m1-case-view m1-shared-viewer",
            div { class: "m1-case-view-toolbar",
                button { onclick: move |_| run_host(&fit, |host| host.fit(), &mut status), "Fit case" }
                details { class: "m1-case-view-settings",
                    summary { "View controls" }
                    div { class: "m1-case-view-settings-body",
            div { role: "group", "aria-label": "Case camera",
                button { onclick: move |_| run_host(&top, |host| host.view("top"), &mut status), "Top view" }
                button { onclick: move |_| run_host(&bottom, |host| host.view("bottom"), &mut status), "Bottom view" }
                button { onclick: move |_| run_host(&iso, |host| host.view("isometric"), &mut status), "Isometric view" }
                button { onclick: move |_| run_host(&left, |host| host.orbit(-50.0, 0.0), &mut status), "Rotate left" }
                button { onclick: move |_| run_host(&right, |host| host.orbit(50.0, 0.0), &mut status), "Rotate right" }
                button { onclick: move |_| run_host(&zoom_in, |host| host.zoom(0.85), &mut status), "Zoom in" }
                button { onclick: move |_| run_host(&zoom_out, |host| host.zoom(1.15), &mut status), "Zoom out" }
            }
            div { role: "group", "aria-label": "Case display mode",
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
            div { role: "group", "aria-label": "Case assembly view",
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
            canvas {
                style: "width:100%;height:100%;display:block",
                tabindex: "0", role: "img",
                "aria-label": "Interactive 3D Case preview. Click a visible body to select its current mapped item; use the controls to navigate and change display.",
                onmounted: mount_host,
                onpointerdown: on_pointer_down,
                onpointermove: on_pointer_move,
                onpointerup: on_pointer_up,
                onpointercancel: on_pointer_cancel,
                onlostpointercapture: on_pointer_cancel,
                onwheel: on_wheel,
            }
            p { role: if status().to_ascii_lowercase().contains("unavailable") || status().contains("failed") { "alert" } else { "status" }, "aria-live": "polite", "{status()}" }
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

fn owner_is_current(owner: &ViewerOwner, identity: &ViewerIdentity) -> bool {
    owner.active.get() && owner.identity.borrow().as_ref() == Some(identity)
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
) {
    let stale_pointer = {
        let mut active = pointer.borrow_mut();
        let current_identity = owner.identity.borrow();
        if active.as_ref().is_some_and(|active| {
            current_identity
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
            viewer_instance: 2,
            projection_generation: 4,
            renderer_sequence: 6,
        }
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

    #[test]
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

    #[test]
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

    #[test]
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
}
