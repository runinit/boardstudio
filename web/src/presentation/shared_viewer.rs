//! Private page viewer shared by workflow-owned, immutable scene projections.
//!
//! Case is the first consumer. Its wrapper owns the Case-to-renderer projection;
//! this module owns only renderer controls, transient view state and host lifetime.
use crate::renderer_host_page::{RendererModelSource, RendererPageHost};
use crate::{cad_jobs::captured_case_document, runtime::CadScene};
use boardstudio_application::{Scope, SnapshotToken};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use js_sys::{Array, Float32Array, Object, Reflect};
use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::spawn_local;
use web_sys::{HtmlCanvasElement, PointerEvent};

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct CaseDisplay {
    pub hidden: Vec<String>,
    pub colors: BTreeMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ViewerIdentity {
    pub scope: Scope,
    pub snapshot_token: SnapshotToken,
    pub viewer_instance: u64,
    pub projection_generation: u64,
    pub renderer_sequence: u64,
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
pub(crate) struct ScopedDisplayChange {
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
    last_projection: Cell<usize>,
    last_theme: RefCell<String>,
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
            last_projection: Cell::new(0),
            last_theme: RefCell::new(String::new()),
            projection_generation: Cell::new(0),
            renderer_sequence: Cell::new(0),
            viewer_instance: next_viewer_instance()?,
        }))
    }

    fn advance(&self, scene: &Rc<CadScene>, theme: &str) -> Result<ViewerIdentity, String> {
        let projection_ptr = Rc::as_ptr(scene) as usize;
        let changed =
            self.last_projection.get() != projection_ptr || *self.last_theme.borrow() != theme;
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
            self.last_projection.set(projection_ptr);
            *self.last_theme.borrow_mut() = theme.to_owned();
        }
        let identity = ViewerIdentity {
            scope: scene.scope.clone(),
            snapshot_token: scene.token,
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
    models: Vec<RendererModelSource>,
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
    scene: Rc<CadScene>,
    selected_layer: String,
    display: CaseDisplay,
    on_signal: EventHandler<ScopedViewerSignal>,
    on_display_change: EventHandler<ScopedDisplayChange>,
) -> Element {
    let runtime = use_context::<Rc<crate::runtime::Runtime>>();
    let _ = use_context::<Signal<u64>>()();
    let theme_state = use_context::<super::ThemeState>().0;
    let preference = theme_state();
    let theme = effective_theme(preference);
    let owner = match use_hook(ViewerOwner::new) {
        Ok(owner) => owner,
        Err(error) => {
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
    let projection_cache = use_hook(|| {
        Rc::new(RefCell::new(
            None::<(Rc<CadScene>, String, Rc<RendererSceneProjection>)>,
        ))
    });
    let identity = match owner.advance(&scene, &theme) {
        Ok(identity) => identity,
        Err(error) => {
            return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
        }
    };
    let projection = if let Some((cached_scene, cached_theme, projection)) =
        projection_cache.borrow().as_ref()
        && Rc::ptr_eq(cached_scene, &scene)
        && cached_theme == &theme
    {
        projection.clone()
    } else {
        match project_case_scene(scene.clone(), identity.clone(), &theme) {
            Ok(projection) => {
                let projection = Rc::new(projection);
                *projection_cache.borrow_mut() =
                    Some((scene.clone(), theme.clone(), projection.clone()));
                projection
            }
            Err(error) => {
                return rsx! { p { role: "alert", "3D preview unavailable: {error}" } };
            }
        }
    };
    let current_source = SourceGuard({
        let runtime = runtime.clone();
        let scene = scene.clone();
        Rc::new(move || {
            runtime.scope().as_ref() == Some(&scene.scope)
                && runtime
                    .model()
                    .accepted
                    .as_ref()
                    .is_some_and(|snapshot| snapshot.token == scene.token)
                && runtime
                    .cad_scene()
                    .as_ref()
                    .is_some_and(|current| Rc::ptr_eq(current, &scene))
        })
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

fn project_case_scene(
    scene: Rc<CadScene>,
    identity: ViewerIdentity,
    theme: &str,
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
            "contours": &scene.contours,
            "surfaces": [],
            "holes": [],
            "models": []
        },
        "models": [],
        "mechanicalStack": stack
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
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
    let mut layers = vec![("PCB".to_owned(), "PCB".to_owned())];
    for body in &scene.result.bodies {
        if !layers.iter().any(|(id, _)| id == &body.id) {
            layers.push((body.id.clone(), body.name.clone()));
        }
    }
    Ok(RendererSceneProjection {
        identity,
        input,
        models: Vec::new(),
        layers,
        handles: Vec::new(),
    })
}

fn effective_theme(preference: &'static str) -> String {
    if preference == "dark" {
        return "dark".to_owned();
    }
    if preference == "light" {
        return "light".to_owned();
    }
    web_sys::window()
        .and_then(|window| window.document())
        .and_then(|document| document.document_element())
        .and_then(|element| element.get_attribute("data-theme"))
        .filter(|value| value == "dark")
        .unwrap_or_else(|| "light".to_owned())
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
        pointer_id: i32,
        last_x: f64,
        last_y: f64,
        start_x: f64,
        start_y: f64,
        moved: bool,
        picked: Option<String>,
    },
    Handle {
        pointer_id: i32,
        z: f32,
        id: String,
    },
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
    let mounted = use_signal(|| false);
    let mut status = use_signal(|| "Starting 3D preview…".to_owned());
    let mut transient = use_signal(TransientView::default);
    let pointer = use_hook(|| Rc::new(RefCell::new(None::<PointerOwner>)));
    let applied_sequence = use_hook(|| Rc::new(Cell::new(0_u64)));

    use_drop({
        let host = host.clone();
        let owner = owner.clone();
        let on_signal = on_signal;
        move || {
            if let Some(host) = host.borrow_mut().take() {
                if let Err(error) = host.dispose()
                    && let Some(identity) = owner.identity.borrow().clone()
                {
                    on_signal.call(ScopedViewerSignal {
                        identity,
                        kind: ViewerSignalKind::Failed(error),
                        owner: Rc::downgrade(&owner),
                    });
                }
            }
            owner.active.set(false);
        }
    });

    use_effect(use_reactive((&projection, &mounted()), {
        let host = host.clone();
        let applied_sequence = applied_sequence.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, mounted)| {
            if !mounted || !owner.active.get() {
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
                    host.update_scene_checked(projection.input.clone(), &projection.models)
                });
            match result {
                Ok(accepted) => {
                    if accepted {
                        applied_sequence.set(identity.renderer_sequence);
                    }
                    status.set(if accepted {
                        "3D preview updated.".to_owned()
                    } else {
                        "A stale 3D scene update was ignored.".to_owned()
                    });
                    emit_signal(&owner, on_signal, ViewerSignalKind::SceneAccepted(accepted));
                }
                Err(error) => {
                    status.set(error.clone());
                    emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                }
            }
            let _ = (&runtime, &current_source);
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
            let on_signal = on_signal;
            let mut status = status;
            move |(projection, selected_layer, display, theme, transient, mounted)| {
                if !mounted || !owner.active.get() {
                    return;
                }
                if owner.identity.borrow().as_ref() != Some(&projection.identity) {
                    return;
                }
                let result =
                    display_state(display, selected_layer, theme, transient).and_then(|state| {
                        host.borrow()
                            .as_ref()
                            .ok_or_else(|| "3D renderer is not mounted".to_owned())
                            .and_then(|host| host.set_display_state(state))
                    });
                if let Err(error) = result {
                    status.set(error.clone());
                    emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                }
            }
        },
    ));

    use_effect(use_reactive((&projection, &mounted()), {
        let host = host.clone();
        let owner = owner.clone();
        let on_signal = on_signal;
        let mut status = status;
        move |(projection, mounted)| {
            if !mounted || !owner.active.get() {
                return;
            }
            if owner.identity.borrow().as_ref() != Some(&projection.identity) {
                return;
            }
            let result = handles_value(&projection.handles).and_then(|handles| {
                host.borrow()
                    .as_ref()
                    .ok_or_else(|| "3D renderer is not mounted".to_owned())
                    .and_then(|host| host.set_handles(handles).map(|_| ()))
            });
            if let Err(error) = result {
                status.set(error.clone());
                emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
            }
        }
    }));

    let mount_host = {
        let host = host.clone();
        let canvas_state = canvas.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let on_signal = on_signal;
        let mut status = status;
        let mut mounted = mounted;
        let applied_sequence = applied_sequence.clone();
        move |event: MountedEvent| {
            let Some(element) = event
                .data()
                .try_as_web_event()
                .and_then(|value| value.dyn_into::<HtmlCanvasElement>().ok())
            else {
                status.set("3D canvas could not be attached.".to_owned());
                emit_signal(
                    &owner,
                    on_signal,
                    ViewerSignalKind::Failed("3D canvas could not be attached.".to_owned()),
                );
                return;
            };
            *canvas_state.borrow_mut() = Some(element.clone());
            let host = host.clone();
            let owner = owner.clone();
            let runtime = runtime.clone();
            let on_signal = on_signal;
            let mut status = status;
            let mut mounted = mounted;
            let identity = projection.identity.clone();
            let input = projection.input.clone();
            let models = projection.models.clone();
            let mount_scope = identity.scope.clone();
            let source_is_current = Rc::new(move || {
                owner.active.get() && runtime.scope().as_ref() == Some(&mount_scope)
            });
            let current: Rc<dyn Fn() -> bool> = source_is_current;
            emit_signal(
                &owner,
                on_signal,
                ViewerSignalKind::Lifecycle(ViewerLifecycle::Starting),
            );
            status.set("Starting the 3D renderer…".to_owned());
            spawn_local(async move {
                let report_owner = owner.clone();
                let report_runtime = runtime.clone();
                let report_status = status;
                let status_callback = Rc::new(move |message: String| {
                    let Some(identity) = report_owner.identity.borrow().clone() else {
                        return;
                    };
                    if !report_owner.active.get()
                        || report_runtime.scope().as_ref() != Some(&identity.scope)
                    {
                        return;
                    }
                    let mut report_status = report_status;
                    report_status.set(message.clone());
                    let kind = if message.to_ascii_lowercase().contains("context lost") {
                        ViewerSignalKind::Lifecycle(ViewerLifecycle::ContextLost)
                    } else if message.contains("initialized") {
                        ViewerSignalKind::Lifecycle(ViewerLifecycle::Ready)
                    } else if message.to_ascii_lowercase().contains("failed") {
                        ViewerSignalKind::Failed(message)
                    } else {
                        return;
                    };
                    on_signal.call(ScopedViewerSignal {
                        identity,
                        kind,
                        owner: Rc::downgrade(&report_owner),
                    });
                });
                match RendererPageHost::mount(element, input, &models, status_callback, current)
                    .await
                {
                    Ok(renderer)
                        if owner.active.get()
                            && runtime.scope().as_ref() == Some(&identity.scope) =>
                    {
                        *host.borrow_mut() = Some(renderer);
                        applied_sequence.set(identity.renderer_sequence);
                        mounted.set(true);
                        status.set("3D preview ready.".to_owned());
                        if owner.identity.borrow().as_ref() == Some(&identity) {
                            emit_signal(&owner, on_signal, ViewerSignalKind::SceneAccepted(true));
                        }
                    }
                    Ok(renderer) => {
                        let _ = renderer.dispose();
                    }
                    Err(error)
                        if owner.active.get()
                            && runtime.scope().as_ref() == Some(&identity.scope) =>
                    {
                        status.set(format!(
                            "3D preview unavailable: {error}. Case forms and the 2D route remain available."
                        ));
                        emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                        emit_signal(
                            &owner,
                            on_signal,
                            ViewerSignalKind::Lifecycle(ViewerLifecycle::Failed),
                        );
                    }
                    Err(_) => {}
                }
            });
        }
    };

    let on_pointer_down = {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        let handles = projection.handles.clone();
        let mut status = status;
        move |event: PointerEvent| {
            if !(current_source.0)() || !owner_is_current(&owner, &projection.identity) {
                return;
            }
            event.prevent_default();
            let (x, y) = pointer_point(&event);
            let Some(host) = host.borrow().as_ref() else {
                return;
            };
            let picked = match host.pick_at_client(x, y) {
                Ok(id) => id.filter(|id| !id.is_empty()),
                Err(error) => {
                    status.set(error.clone());
                    emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                    None
                }
            };
            if let Some(handle) = picked
                .as_ref()
                .and_then(|id| handles.iter().find(|handle| &handle.id == id))
            {
                let point = match host.point_on_plane_at_client(x, y, handle.z) {
                    Ok(point) => point,
                    Err(error) => {
                        status.set(error.clone());
                        emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                        return;
                    }
                };
                emit_signal(&owner, on_signal, ViewerSignalKind::WorldPoint(point));
                emit_signal(
                    &owner,
                    on_signal,
                    ViewerSignalKind::HandleGesture {
                        phase: HandleGesturePhase::Start,
                        handle_id: handle.id.clone(),
                        point,
                    },
                );
                *pointer.borrow_mut() = Some(PointerOwner::Handle {
                    pointer_id: event.pointer_id(),
                    z: handle.z,
                    id: handle.id.clone(),
                });
            } else {
                *pointer.borrow_mut() = Some(PointerOwner::Orbit {
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

    let on_pointer_move = {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let mut status = status;
        move |event: PointerEvent| {
            if !(current_source.0)() || !owner_is_current(&owner, &projection.identity) {
                pointer.borrow_mut().take();
                return;
            }
            let Some(host) = host.borrow().as_ref() else {
                return;
            };
            let (x, y) = pointer_point(&event);
            let active = pointer.borrow_mut().take();
            match active {
                Some(PointerOwner::Orbit {
                    pointer_id,
                    last_x,
                    last_y,
                    start_x,
                    start_y,
                    mut moved,
                    picked,
                }) if pointer_id == event.pointer_id() => {
                    let delta_x = x - last_x;
                    let delta_y = y - last_y;
                    moved |= (x - start_x).abs() + (y - start_y).abs() > 4.0;
                    if let Err(error) = host.orbit(delta_x, delta_y) {
                        status.set(error.clone());
                        emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                    }
                    *pointer.borrow_mut() = Some(PointerOwner::Orbit {
                        pointer_id,
                        last_x: x,
                        last_y: y,
                        start_x,
                        start_y,
                        moved,
                        picked,
                    });
                }
                Some(PointerOwner::Handle { pointer_id, z, id })
                    if pointer_id == event.pointer_id() =>
                {
                    let point = match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            status.set(error.clone());
                            emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                            *pointer.borrow_mut() =
                                Some(PointerOwner::Handle { pointer_id, z, id });
                            return;
                        }
                    };
                    emit_signal(&owner, on_signal, ViewerSignalKind::WorldPoint(point));
                    emit_signal(
                        &owner,
                        on_signal,
                        ViewerSignalKind::HandleGesture {
                            phase: HandleGesturePhase::Move,
                            handle_id: id.clone(),
                            point,
                        },
                    );
                    *pointer.borrow_mut() = Some(PointerOwner::Handle { pointer_id, z, id });
                }
                other => *pointer.borrow_mut() = other,
            }
        }
    };

    let on_pointer_up = {
        let host = host.clone();
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: PointerEvent| {
            let active = pointer.borrow_mut().take();
            if !(current_source.0)() || !owner_is_current(&owner, &projection.identity) {
                return;
            }
            if let Some(canvas) = canvas.borrow().as_ref() {
                let _ = canvas.release_pointer_capture(event.pointer_id());
            }
            match active {
                Some(PointerOwner::Orbit {
                    pointer_id,
                    moved,
                    picked,
                    ..
                }) if pointer_id == event.pointer_id() => {
                    if !moved
                        && runtime.scope().as_ref() == Some(&projection.identity.scope)
                        && let Some(id) = picked
                    {
                        emit_signal(&owner, on_signal, ViewerSignalKind::Picked(id));
                    }
                }
                Some(PointerOwner::Handle { pointer_id, z, id })
                    if pointer_id == event.pointer_id() =>
                {
                    if let Some(host) = host.borrow().as_ref() {
                        let (x, y) = pointer_point(&event);
                        let point = match host.point_on_plane_at_client(x, y, z) {
                            Ok(point) => point,
                            Err(error) => {
                                emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                                emit_signal(
                                    &owner,
                                    on_signal,
                                    ViewerSignalKind::HandleGesture {
                                        phase: HandleGesturePhase::Cancel,
                                        handle_id: id,
                                        point: None,
                                    },
                                );
                                return;
                            }
                        };
                        emit_signal(&owner, on_signal, ViewerSignalKind::WorldPoint(point));
                        emit_signal(
                            &owner,
                            on_signal,
                            ViewerSignalKind::HandleGesture {
                                phase: HandleGesturePhase::End,
                                handle_id: id,
                                point,
                            },
                        );
                    }
                }
                other => *pointer.borrow_mut() = other,
            }
        }
    };

    let on_pointer_cancel = {
        let host = host.clone();
        let owner = owner.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        let pointer = pointer.clone();
        let canvas = canvas.clone();
        move |event: PointerEvent| {
            let active = pointer.borrow_mut().take();
            if !(current_source.0)() || !owner_is_current(&owner, &projection.identity) {
                return;
            }
            if let Some(canvas) = canvas.borrow().as_ref() {
                let _ = canvas.release_pointer_capture(event.pointer_id());
            }
            if let Some(PointerOwner::Handle { pointer_id, z, id }) = active
                && pointer_id == event.pointer_id()
            {
                let point = if let Some(host) = host.borrow().as_ref() {
                    let (x, y) = pointer_point(&event);
                    match host.point_on_plane_at_client(x, y, z) {
                        Ok(point) => point,
                        Err(error) => {
                            emit_signal(&owner, on_signal, ViewerSignalKind::Failed(error));
                            None
                        }
                    }
                } else {
                    None
                };
                emit_signal(&owner, on_signal, ViewerSignalKind::WorldPoint(point));
                emit_signal(
                    &owner,
                    on_signal,
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
        let owner = owner.clone();
        let runtime = runtime.clone();
        let current_source = current_source.clone();
        let on_signal = on_signal;
        move |id: String| {
            if (current_source.0)()
                && runtime.scope().as_ref() == Some(&projection.identity.scope)
                && owner_is_current(&owner, &projection.identity)
            {
                emit_signal(&owner, on_signal, ViewerSignalKind::LayerSelected(id));
            }
        }
    };

    let change_display = {
        let owner = owner.clone();
        let runtime = runtime.clone();
        move |next: CaseDisplay| {
            let identity = projection.identity.clone();
            if !owner_is_current(&owner, &identity)
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
            div { role: "group", "aria-label": "Case camera",
                button { onclick: move |_| run_host(&fit, |host| host.fit(), &mut status), "Fit case" }
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
                    let mut transient = transient;
                    let label = mode.renderer_value();
                    button {
                        "aria-pressed": transient().mode == mode,
                        onclick: move |_| transient.with_mut(|view| view.mode = mode),
                        "{label}"
                    }
                }
            }
            div { role: "group", "aria-label": "Case assembly view",
                for view in [AssemblyView::Assembled, AssemblyView::Exploded, AssemblyView::Section] {
                    let mut transient = transient;
                    let label = view.renderer_value();
                    button {
                        "aria-pressed": transient().assembly == view,
                        onclick: move |_| transient.with_mut(|state| state.assembly = view),
                        "{label}"
                    }
                }
            }
            if transient().assembly == AssemblyView::Exploded {
                label { "Explode amount"
                    input {
                        r#type: "range", min: "0", max: "2", step: "0.05",
                        value: "{transient().explode_amount}",
                        oninput: move |event: FormEvent| {
                            if let Ok(value) = event.value().parse::<f32>() {
                                transient.with_mut(|state| state.explode_amount = value.clamp(0.0, 2.0));
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
                        r#type: "range", min: "-100", max: "100", step: "0.5",
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
            canvas {
                style: "width:100%;height:320px;display:block",
                tabindex: "0", role: "img",
                "aria-label": "Interactive 3D Case preview. Click a visible body to select its current mapped item; use the controls to navigate and change display.",
                onmounted: mount_host,
                onpointerdown: on_pointer_down,
                onpointermove: on_pointer_move,
                onpointerup: on_pointer_up,
                onpointercancel: on_pointer_cancel,
            }
            p { role: if status().to_ascii_lowercase().contains("unavailable") || status().contains("failed") { "alert" } else { "status" }, "aria-live": "polite", "{status()}" }
            details {
                summary { "Case layers and colors" }
                div { role: "group", "aria-label": "Case layers",
                for (id, label) in &projection.layers {
                    let id = id.clone();
                    let label = label.clone();
                    let ids = preference_ids(&id);
                    let visible = !ids.iter().all(|entry| display.hidden.contains(entry));
                    let color = display.colors.get(&id).cloned().unwrap_or_else(|| "#b0b5bd".to_owned());
                    let has_color = display.colors.contains_key(&id);
                    let toggle_next = display.clone();
                    let color_next = display.clone();
                    let reset_next = display.clone();
                    let toggle_display = change_display.clone();
                    let select_layer = select_layer.clone();
                    let set_color = change_display.clone();
                    let reset_color = change_display.clone();
                    rsx! {
                        div { class: "m1-viewer-layer", key: "{id}",
                            button {
                                "aria-pressed": selected_layer == id,
                                onclick: move |_| select_layer(id.clone()),
                                "Select {label}"
                            }
                            button {
                                "aria-pressed": visible,
                                "aria-label": "Toggle visibility for {label}",
                                onclick: move |_| {
                                    let mut next_display = toggle_next;
                                    let aliases = preference_ids(&id);
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
                                        let mut next_display = color_next;
                                        next_display.colors.insert(id.clone(), event.value());
                                        set_color(next_display.clone());
                                    }
                                }
                            }
                            button {
                                disabled: !has_color,
                                onclick: move |_| {
                                    let mut next_display = reset_next;
                                    next_display.colors.remove(&id);
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

fn owner_is_current(owner: &ViewerOwner, identity: &ViewerIdentity) -> bool {
    owner.active.get() && owner.identity.borrow().as_ref() == Some(identity)
}

fn emit_signal(
    owner: &Rc<ViewerOwner>,
    on_signal: EventHandler<ScopedViewerSignal>,
    kind: ViewerSignalKind,
) {
    let Some(identity) = owner.identity.borrow().clone() else {
        return;
    };
    if !owner_is_current(owner, &identity) {
        return;
    }
    on_signal.call(ScopedViewerSignal {
        identity,
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

fn preference_ids(id: &str) -> Vec<String> {
    if id == "gaskets" {
        vec!["Gaskets".to_owned()]
    } else if id == "pcb" {
        ["PCB", "Models", "Keycaps", "Copper", "Mask", "Silkscreen"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    } else if id.starts_with("gasket:") {
        let stem = id
            .strip_suffix(":upper")
            .or_else(|| id.strip_suffix(":lower"))
            .unwrap_or(id);
        vec![format!("{stem}:lower"), format!("{stem}:upper")]
    } else {
        vec![id.to_owned()]
    }
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

    #[test]
    fn viewer_events_reject_changed_owner_identity_and_unmount() {
        let identity = identity();
        let owner = Rc::new(ViewerOwner {
            active: Cell::new(true),
            identity: RefCell::new(Some(identity.clone())),
            last_projection: Cell::new(1),
            last_theme: RefCell::new("light".to_owned()),
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

        let mut changed = identity;
        changed.renderer_sequence += 1;
        *owner.identity.borrow_mut() = Some(changed);
        assert!(!signal.is_current());

        owner.active.set(false);
        assert!(!signal.is_current());
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
