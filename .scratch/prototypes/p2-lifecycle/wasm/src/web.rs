use crate::{
    gesture::{GestureCoordinator, GestureEffect, Point},
    p1_host,
    renderer_host::RendererHost,
};
use boardstudio_core::model::{
    CoreReply, CoreRequest, EditCommand, EditOperation, EditPhase, Position, ProjectDoc,
    SceneDelta, Vec2,
};
use boardstudio_p1_core::{Action, Identity, ResultPayload};
use dioxus::prelude::*;
use dioxus_web::WebEventExt;
use gloo_timers::future::TimeoutFuture;
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::Arc,
};
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Element as DomElement, FocusOptions, HtmlCanvasElement, SvgElement};

const FIXTURE: &str = include_str!("../../fixtures/reviung41.json");
const TARGET: &str = "matrix/main-right-keys/r0c0";
const _: Asset = asset!("/assets/worker/worker-entry.js");
const _: Asset = asset!("/assets/p1-worker/p1_core.js");
const _: Asset = asset!("/assets/p1-worker/p1_core_bg.wasm");
const _: Asset = asset!("/assets/renderer/boardstudio_renderer_wasm.js");
const _: Asset = asset!("/assets/renderer/boardstudio_renderer_wasm_bg.wasm");

struct WorkerOwner {
    client: RefCell<Option<Rc<p1_host::Client>>>,
    operation: Cell<u32>,
    generation: Cell<u32>,
    closed: Cell<bool>,
}

impl PartialEq for WorkerOwner {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl WorkerOwner {
    fn new() -> Self {
        Self {
            client: RefCell::new(None),
            operation: Cell::new(1),
            generation: Cell::new(0),
            closed: Cell::new(false),
        }
    }

    async fn open_fixture(&self) -> Result<(ProjectDoc, SceneDelta), String> {
        if self.closed.get() {
            return Err("root worker owner is closed".to_owned());
        }
        self.ensure_worker().await?;
        let document: ProjectDoc =
            serde_json::from_str(FIXTURE).map_err(|error| error.to_string())?;
        let reply = self
            .request(CoreRequest::Open {
                id: "p2-open-fixture".into(),
                document,
            })
            .await?;
        if self.closed.get() {
            return Err("root worker owner closed before fixture publication".to_owned());
        }
        scene_reply(reply)
    }

    async fn snapshot(&self) -> Result<(ProjectDoc, SceneDelta), String> {
        let reply = self
            .request(CoreRequest::Snapshot {
                id: self.next_request_id("snapshot"),
            })
            .await?;
        scene_reply(reply)
    }

    async fn request(&self, request: CoreRequest) -> Result<CoreReply, String> {
        self.ensure_worker().await?;
        let operation = self.operation.get();
        self.operation.set(operation.wrapping_add(1).max(1));
        let identity = Identity {
            epoch: 1,
            operation,
        };
        let client = self
            .client
            .borrow()
            .as_ref()
            .cloned()
            .ok_or_else(|| "core worker is unavailable".to_owned())?;
        client
            .send(identity, Action::Core(Box::new(request)), None)
            .map_err(|error| format!("core worker send failed: {error}"))?;
        let (reply, _) = client.wait(identity).await?;
        if self.closed.get() {
            return Err("root worker owner closed before reply publication".to_owned());
        }
        match reply.payload {
            ResultPayload::Core(reply) => Ok(*reply),
            ResultPayload::Ack => {
                Err("core worker returned an acknowledgement for a domain request".into())
            }
        }
    }

    async fn ensure_worker(&self) -> Result<(), String> {
        if self.closed.get() {
            return Err("root worker owner is closed".to_owned());
        }
        if self.client.borrow().is_none() {
            let prefix = base_prefix()?;
            let url = format!("{prefix}assets/worker/worker-entry.js");
            let client = p1_host::Client::new(&url, 1)
                .map_err(|error| format!("core worker create failed: {error:?}"))?;
            if self.closed.get() {
                client.close();
                return Err("root worker owner closed during initialization".to_owned());
            }
            *self.client.borrow_mut() = Some(Rc::new(client));
        }
        for _ in 0..300 {
            if self.closed.get() {
                return Err("root worker owner closed during readiness wait".to_owned());
            }
            if self
                .client
                .borrow()
                .as_ref()
                .is_some_and(|client| client.ready())
            {
                return Ok(());
            }
            TimeoutFuture::new(10).await;
        }
        Err("core worker readiness deadline exceeded".into())
    }

    fn next_request_id(&self, prefix: &str) -> String {
        let operation = self.operation.get();
        self.operation.set(operation.wrapping_add(1).max(1));
        format!("p2-{prefix}-{operation}")
    }

    fn mount_generation(&self) -> u32 {
        let generation = self.generation.get().wrapping_add(1).max(1);
        self.generation.set(generation);
        generation
    }

    fn close(&self) {
        self.closed.set(true);
        self.generation
            .set(self.generation.get().wrapping_add(1).max(1));
        if let Some(client) = self.client.borrow_mut().take() {
            client.close();
        }
    }
}

pub fn app() -> Element {
    let worker = use_hook(|| Rc::new(WorkerOwner::new()));
    let mut document = use_signal(|| None::<Arc<ProjectDoc>>);
    let mut scene = use_signal(|| None::<Arc<SceneDelta>>);
    let mut status = use_signal(|| "Starting public CoreEngine worker".to_owned());
    let mut panel_mounted = use_signal(|| true);
    let close_worker = worker.clone();
    use_drop(move || close_worker.close());

    use_hook(|| {
        let worker = worker.clone();
        spawn_local(async move {
            match worker.open_fixture().await {
                Ok((opened, next_scene)) => {
                    if worker.closed.get() {
                        return;
                    }
                    document.set(Some(Arc::new(opened)));
                    scene.set(Some(Arc::new(next_scene)));
                    status.set(
                        "Copied Reviung 41 fixture opened in the P1 CoreEngine worker".to_owned(),
                    );
                }
                Err(error) => status.set(format!("Core worker failed: {error}")),
            }
        });
    });

    let undo_worker = worker.clone();
    let redo_worker = worker.clone();
    rsx! {
        style { {STYLE} }
        main {
            class: "p2-shell",
            h1 { "BoardStudio P2 lifecycle probe" }
            p { class: "summary", "A copied Reviung 41 fixture, public CoreEngine worker, Rust gesture coordinator, and the existing generated renderer." }
            div { class: "toolbar",
                button {
                    onclick: move |_| {
                        let next = !panel_mounted();
                        panel_mounted.set(next);
                    },
                    "data-testid": "toggle-editor-panel",
                    if panel_mounted() { "Unmount editor panel" } else { "Mount editor panel" }
                }
                button {
                    disabled: document.read().is_none(),
                    onclick: move |_| send_history(undo_worker.clone(), document, scene, status, false),
                    "Undo"
                }
                button {
                    disabled: document.read().is_none(),
                    onclick: move |_| send_history(redo_worker.clone(), document, scene, status, true),
                    "Redo"
                }
            }
            p { class: "status", role: "status", "aria-live": "polite", "{status}" }
            if panel_mounted() {
                EditorPanel { worker: worker.clone(), document, scene, status }
            } else {
                div { class: "panel-unmounted", "Editor panel is unmounted; the root-owned CoreEngine worker remains active." }
            }
        }
    }
}

#[component]
fn EditorPanel(
    worker: Rc<WorkerOwner>,
    document: Signal<Option<Arc<ProjectDoc>>>,
    scene: Signal<Option<Arc<SceneDelta>>>,
    status: Signal<String>,
) -> Element {
    let gesture = use_hook(|| Rc::new(RefCell::new(GestureCoordinator::default())));
    let preview_scene = use_signal(|| None::<Arc<SceneDelta>>);
    let preview_sequence = use_hook(|| Rc::new(Cell::new(0_u32)));
    let resources = use_hook(|| Rc::new(RefCell::new(PanelResources::default())));
    let generation = use_hook(|| worker.mount_generation());
    let cleanup_worker = worker.clone();
    let cleanup_gesture = gesture.clone();
    let cleanup_resources = resources.clone();
    let mut cleanup_status = status;
    use_drop(move || {
        cleanup_worker
            .generation
            .set(cleanup_worker.generation.get().wrapping_add(1).max(1));
        if let Some(svg) = cleanup_resources.borrow().svg.as_ref() {
            for effect in cleanup_gesture.borrow_mut().escape() {
                if let GestureEffect::Release { pointer_id } = effect
                    && let Err(error) = release_if_captured(svg, pointer_id)
                {
                    cleanup_status.set(format!("Pointer cleanup failed: {error}"));
                }
            }
        }
        if let Some(renderer) = cleanup_resources.borrow_mut().renderer.take()
            && let Err(error) = renderer.dispose()
        {
            cleanup_status.set(format!("Renderer disposal failed: {error}"));
        }
        cleanup_resources.borrow_mut().mounted = false;
    });

    use_hook(|| {
        let worker = worker.clone();
        let mut document = document;
        let mut scene = scene;
        let mut status = status;
        spawn_local(async move {
            match worker.snapshot().await {
                Ok((current, current_scene)) if worker.generation.get() == generation => {
                    document.set(Some(Arc::new(current)));
                    scene.set(Some(Arc::new(current_scene)));
                }
                Ok(_) => {}
                Err(error) if worker.generation.get() == generation => {
                    status.set(format!("Snapshot failed: {error}"))
                }
                Err(_) => {}
            }
        });
    });

    let effect_resources = resources.clone();
    let preview_scene_effect = preview_scene;
    let mut effect_status = status;
    use_effect(move || {
        let current_scene = preview_scene_effect
            .read()
            .as_ref()
            .cloned()
            .or_else(|| scene.read().as_ref().cloned());
        if let Some(current_scene) = current_scene
            && let Some(renderer) = effect_resources.borrow().renderer.as_ref()
        {
            let thickness = document
                .read()
                .as_ref()
                .and_then(|doc| doc.boards.first())
                .map_or(1.6, |board| board.thickness);
            if let Err(error) = renderer.update_scene(&current_scene, thickness) {
                effect_status.set(format!("Renderer update failed: {error}"));
            }
        }
    });

    let Some(document_value) = document.read().as_ref().cloned() else {
        return rsx! { section { class: "editor-panel", p { "Loading copied project through CoreEngine worker…" } } };
    };
    let scene_value = scene.read().as_ref().cloned();
    let (min_x, min_y, max_x, max_y) = part_bounds(&document_value.parts);
    let scale = (920.0 / (max_x - min_x).max(1.0))
        .min(540.0 / (max_y - min_y).max(1.0))
        .clamp(0.8, 3.4);
    let key_width = (15.0 * scale).clamp(20.0, 50.0);
    let key_height = (14.0 * scale).clamp(20.0, 48.0);
    let svg_holder = resources.clone();
    let canvas_holder = resources.clone();
    let canvas_scene = scene_value.clone();
    let canvas_doc = document_value.clone();
    let panel_status = status;

    let pointer_down = {
        let gesture = gesture.clone();
        let resources = resources.clone();
        let project = document_value.clone();
        let worker = worker.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            if pointer.button() != 0
                || pointer.ctrl_key()
                || pointer.meta_key()
                || pointer.shift_key()
            {
                return;
            }
            let Some(id) = pointer
                .target()
                .and_then(|target| target.dyn_into::<DomElement>().ok())
                .and_then(|target| target.closest("[data-part-id]").ok().flatten())
                .and_then(|target| target.get_attribute("data-part-id"))
            else {
                return;
            };
            if project
                .parts
                .iter()
                .find(|part| part.id == id)
                .is_some_and(|part| part.locked.unwrap_or(false))
            {
                return;
            }
            let Some(svg) = resources.borrow().svg.clone() else {
                status.set("Gesture start failed: SVG surface is not mounted".to_owned());
                return;
            };
            let Some(start) =
                client_to_viewbox(&svg, pointer.client_x() as f64, pointer.client_y() as f64)
            else {
                status.set("Gesture start failed: SVG has no measurable view box".to_owned());
                return;
            };
            let Some(part) = project.parts.iter().find(|part| part.id == id) else {
                return;
            };
            let origin = Point {
                x: part.pose.at.x,
                y: part.pose.at.y,
            };
            let transaction_id = worker.next_request_id("drag-tx");
            gesture.borrow_mut().pointer_down_with_base(
                pointer.pointer_id(),
                &id,
                start,
                project.revision,
                transaction_id,
                origin,
            );
            if !gesture.borrow().is_active_for(pointer.pointer_id()) {
                return;
            }
            pointer.prevent_default();
            if let Err(error) = svg.set_pointer_capture(pointer.pointer_id()) {
                gesture.borrow_mut().pointer_cancel(pointer.pointer_id());
                status.set(format!("Pointer capture failed: {error:?}"));
                return;
            }
            let focus_options = FocusOptions::new();
            focus_options.set_prevent_scroll(true);
            if let Err(error) = svg.focus_with_options(&focus_options) {
                let release_error = svg.release_pointer_capture(pointer.pointer_id()).err();
                gesture.borrow_mut().pointer_cancel(pointer.pointer_id());
                status.set(format!(
                    "SVG focus failed: {error:?}; capture release: {release_error:?}"
                ));
            }
        }
    };

    let pointer_move = {
        let gesture = gesture.clone();
        let resources = resources.clone();
        let worker = worker.clone();
        let sequence = preview_sequence.clone();
        let mut preview_scene = preview_scene;
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(svg) = resources.borrow().svg.clone() else {
                return;
            };
            let Some(at) =
                client_to_viewbox(&svg, pointer.client_x() as f64, pointer.client_y() as f64)
            else {
                return;
            };
            let effects = gesture.borrow_mut().pointer_move(pointer.pointer_id(), at);
            for effect in effects {
                if let GestureEffect::Preview {
                    id,
                    at,
                    base_revision,
                    transaction_id,
                    origin,
                } = effect
                {
                    let world = Point {
                        x: origin.x + at.x / scale,
                        y: origin.y - at.y / scale,
                    };
                    let request = preview_request(
                        worker.next_request_id("preview"),
                        transaction_id,
                        base_revision,
                        id,
                        world,
                    );
                    let worker = worker.clone();
                    let sequence = sequence.clone();
                    let current = sequence.get().wrapping_add(1).max(1);
                    sequence.set(current);
                    spawn_local(async move {
                        match worker.request(request).await.and_then(preview_reply) {
                            Ok(next_scene)
                                if worker.generation.get() == generation
                                    && sequence.get() == current =>
                            {
                                preview_scene.set(Some(Arc::new(next_scene)));
                            }
                            Err(error)
                                if worker.generation.get() == generation
                                    && sequence.get() == current =>
                            {
                                status.set(format!("CoreEngine preview failed: {error}"));
                                preview_scene.set(None);
                            }
                            _ => {}
                        }
                    });
                }
            }
        }
    };

    let pointer_up = {
        let gesture = gesture.clone();
        let resources = resources.clone();
        let worker = worker.clone();
        let mut document = document;
        let mut scene = scene;
        let mut preview_scene = preview_scene;
        let sequence = preview_sequence.clone();
        let mut status = status;
        move |event: dioxus::prelude::PointerEvent| {
            let Some(pointer) = event.data().try_as_web_event() else {
                return;
            };
            let Some(svg) = resources.borrow().svg.clone() else {
                status.set("Pointer up ignored: SVG surface was unmounted".to_owned());
                return;
            };
            let Some(at) =
                client_to_viewbox(&svg, pointer.client_x() as f64, pointer.client_y() as f64)
            else {
                status.set("Pointer up ignored: SVG has no measurable view box".to_owned());
                return;
            };
            let effects = gesture.borrow_mut().pointer_up(pointer.pointer_id(), at);
            if !effects
                .iter()
                .any(|effect| matches!(effect, GestureEffect::Commit { .. }))
            {
                sequence.set(sequence.get().wrapping_add(1).max(1));
                preview_scene.set(None);
            }
            for effect in effects {
                match effect {
                    GestureEffect::Commit {
                        id,
                        at,
                        base_revision,
                        transaction_id,
                        origin,
                    } => {
                        sequence.set(sequence.get().wrapping_add(1).max(1));
                        preview_scene.set(None);
                        if let Some(current) = document.read().as_ref().cloned() {
                            if current.revision != base_revision {
                                status.set("Drag canceled because the document revision changed during capture".to_owned());
                                if let Err(error) = release_if_captured(&svg, pointer.pointer_id())
                                {
                                    status.set(error);
                                }
                                continue;
                            }
                            let world_delta = Point {
                                x: at.x / scale,
                                y: -at.y / scale,
                            };
                            let position = Position {
                                id: id.clone(),
                                at: Vec2 {
                                    x: origin.x + world_delta.x,
                                    y: origin.y + world_delta.y,
                                },
                            };
                            let request_id = worker.next_request_id("drag");
                            let request = CoreRequest::Edit {
                                id: request_id,
                                command: EditCommand {
                                    base_revision,
                                    transaction_id,
                                    phase: EditPhase::Commit,
                                    target_ids: vec![id],
                                    operation: EditOperation::MoveParts {
                                        positions: vec![position],
                                    },
                                },
                            };
                            let worker = worker.clone();
                            spawn_local(async move {
                                match worker.request(request).await.and_then(scene_reply) {
                                    Ok((next_document, next_scene))
                                        if worker.generation.get() == generation =>
                                    {
                                        document.set(Some(Arc::new(next_document)));
                                        scene.set(Some(Arc::new(next_scene)));
                                        status.set(
                                            "Drag committed by public CoreEngine worker".to_owned(),
                                        );
                                    }
                                    Ok(_) => {}
                                    Err(error) if worker.generation.get() == generation => {
                                        status.set(format!("Drag commit failed: {error}"))
                                    }
                                    Err(_) => {}
                                }
                            });
                        }
                    }
                    GestureEffect::Release { pointer_id } => {
                        if let Err(error) = release_if_captured(&svg, pointer_id) {
                            status.set(error);
                        }
                    }
                    _ => {}
                }
            }
        }
    };

    let cancel_gesture = {
        let gesture = gesture.clone();
        let resources = resources.clone();
        let worker = worker.clone();
        let sequence = preview_sequence.clone();
        let mut preview_scene = preview_scene;
        let mut status = status;
        move || {
            let effects = gesture.borrow_mut().escape();
            sequence.set(sequence.get().wrapping_add(1).max(1));
            for effect in effects {
                match effect {
                    GestureEffect::Cancel {
                        id,
                        base_revision,
                        transaction_id,
                        origin,
                    } => {
                        let request = preview_request(
                            worker.next_request_id("cancel-preview"),
                            transaction_id,
                            base_revision,
                            id,
                            origin,
                        );
                        let worker = worker.clone();
                        let mut status = status;
                        spawn_local(async move {
                            if let Err(error) =
                                worker.request(request).await.and_then(preview_reply)
                                && worker.generation.get() == generation
                            {
                                status.set(format!("Cancel preview restoration failed: {error}"));
                            }
                        });
                    }
                    GestureEffect::Release { pointer_id } => {
                        if let Some(svg) = resources.borrow().svg.as_ref()
                            && let Err(error) = release_if_captured(svg, pointer_id)
                        {
                            status.set(error);
                        }
                    }
                    _ => {}
                }
            }
            preview_scene.set(None);
            status.set("Gesture canceled; CoreEngine document unchanged".to_owned());
        }
    };

    let key_down = {
        let mut cancel = cancel_gesture.clone();
        let worker = worker.clone();
        let mut document = document;
        let mut scene = scene;
        let mut status = status;
        move |event: dioxus::prelude::KeyboardEvent| {
            let Some(keyboard) = event.data().try_as_web_event() else {
                return;
            };
            if keyboard.key() == "Escape" {
                keyboard.prevent_default();
                cancel();
                return;
            }
            let delta = match keyboard.key().as_str() {
                "ArrowLeft" => Some((-1.0, 0.0)),
                "ArrowRight" => Some((1.0, 0.0)),
                "ArrowUp" => Some((0.0, 1.0)),
                "ArrowDown" => Some((0.0, -1.0)),
                _ => None,
            };
            let Some((dx, dy)) = delta else {
                return;
            };
            keyboard.prevent_default();
            let Some(current) = document.read().as_ref().cloned() else {
                return;
            };
            let Some(part) = current.parts.iter().find(|part| part.id == TARGET) else {
                return;
            };
            let transaction_id = worker.next_request_id("keyboard");
            let request = CoreRequest::Edit {
                id: transaction_id.clone(),
                command: EditCommand {
                    base_revision: current.revision,
                    transaction_id,
                    phase: EditPhase::Commit,
                    target_ids: vec![TARGET.to_owned()],
                    operation: EditOperation::MoveParts {
                        positions: vec![Position {
                            id: TARGET.to_owned(),
                            at: Vec2 {
                                x: part.pose.at.x + dx,
                                y: part.pose.at.y + dy,
                            },
                        }],
                    },
                },
            };
            let worker = worker.clone();
            spawn_local(async move {
                match worker.request(request).await.and_then(scene_reply) {
                    Ok((next_document, next_scene)) if worker.generation.get() == generation => {
                        document.set(Some(Arc::new(next_document)));
                        scene.set(Some(Arc::new(next_scene)));
                        status.set("Keyboard move committed by CoreEngine".to_owned());
                    }
                    Ok(_) => {}
                    Err(error) if worker.generation.get() == generation => {
                        status.set(format!("Keyboard edit failed: {error}"))
                    }
                    Err(_) => {}
                }
            });
        }
    };

    let mut svg_status = panel_status;
    let mount_svg = move |event: MountedEvent| {
        let Some(element) = event.data().try_as_web_event() else {
            svg_status
                .set("SVG mount failed: Dioxus did not expose the mounted element".to_owned());
            return;
        };
        match element.dyn_into::<SvgElement>() {
            Ok(svg) => {
                let mut resources = svg_holder.borrow_mut();
                resources.svg = Some(svg);
                resources.mounted = true;
            }
            Err(error) => svg_status.set(format!(
                "SVG mount failed: wrong mounted element: {error:?}"
            )),
        }
    };
    let mount_worker = worker.clone();
    let mut canvas_status = panel_status;
    let mount_canvas = move |event: MountedEvent| {
        let Some(element) = event.data().try_as_web_event() else {
            canvas_status
                .set("Canvas mount failed: Dioxus did not expose the mounted element".to_owned());
            return;
        };
        let Ok(canvas) = element.dyn_into::<HtmlCanvasElement>() else {
            canvas_status.set("Canvas mount failed: mounted node is not an HTML canvas".to_owned());
            return;
        };
        let resources = canvas_holder.clone();
        let scene = canvas_scene.clone();
        let doc = canvas_doc.clone();
        let mut status = canvas_status;
        let worker = mount_worker.clone();
        let generation = generation;
        let current_resources = resources.clone();
        let is_current: Rc<dyn Fn() -> bool> = Rc::new(move || {
            worker.generation.get() == generation && current_resources.borrow().mounted
        });
        spawn_local(async move {
            let Some(scene) = scene else {
                status.set("Renderer mount failed: core scene unavailable".to_owned());
                return;
            };
            let thickness = doc.boards.first().map_or(1.6, |board| board.thickness);
            let status_for_callback = Rc::new(RefCell::new(status));
            let status_fn: Rc<dyn Fn(String)> = Rc::new(move |message| {
                status_for_callback.borrow_mut().set(message);
            });
            match RendererHost::mount(canvas, &scene, thickness, status_fn, is_current.clone())
                .await
            {
                Ok(renderer) => {
                    if is_current() {
                        resources.borrow_mut().renderer = Some(renderer);
                    } else {
                        if let Err(error) = renderer.dispose() {
                            web_sys::console::error_1(
                                &format!("Stale renderer disposal failed: {error}").into(),
                            );
                        }
                    }
                }
                Err(error) if is_current() => {
                    status.set(format!("Renderer initialization failed: {error}"))
                }
                Err(_) => {}
            }
        });
    };

    let pointer_cancel = make_pointer_cancel_handler(
        gesture.clone(),
        preview_sequence.clone(),
        preview_scene,
        worker.clone(),
        resources.clone(),
        panel_status,
        "Pointer cancellation restored the captured origin",
    );
    let lost_capture = make_pointer_cancel_handler(
        gesture.clone(),
        preview_sequence.clone(),
        preview_scene,
        worker.clone(),
        resources.clone(),
        panel_status,
        "Gesture canceled after pointer capture was lost",
    );

    let display_scene = preview_scene.read().as_ref().cloned();
    rsx! {
        section { class: "editor-panel", "data-panel-generation": "{generation}",
            div { class: "editor-grid",
                div { class: "gesture-panel",
                    h2 { "Keyboard layout" }
                    p { "Drag a key, press Escape to cancel, or use arrow keys after focusing the canvas." }
                    svg {
                        class: "layout-canvas",
                        view_box: "0 0 1000 650",
                        tabindex: "0",
                        role: "application",
                        "aria-label": "Keyboard layout editor",
                        onmounted: mount_svg,
                        onpointerdown: pointer_down,
                        onpointermove: pointer_move,
                        onpointerup: pointer_up,
                        onpointercancel: pointer_cancel,
                        onlostpointercapture: lost_capture,
                        onkeydown: key_down,
                        for part in document_value.parts.iter() {
                            {
                                let pose = display_scene.as_ref().and_then(|scene| scene.transforms.iter().find(|transform| transform.id == part.id)).map(|transform| transform.pose).unwrap_or(part.pose);
                                let x = 50.0 + (pose.at.x - min_x) * scale;
                                let y = 50.0 + (max_y - pose.at.y) * scale;
                                let selected = part.id == TARGET;
                                let label = format!("{}{}", part.reference, if selected { ", selected drag target" } else { "" });
                                let rotation = -pose.rotation;
                                rsx! {
                                    g {
                                        key: "{part.id}",
                                        "data-part-id": "{part.id}",
                                        tabindex: "-1",
                                        role: "button",
                                        "aria-label": "{label}",
                                        transform: "translate({x} {y}) rotate({rotation})",
                                        rect { x: "{-key_width / 2.0}", y: "{-key_height / 2.0}", width: "{key_width}", height: "{key_height}", rx: "5", class: if selected { "part selected" } else { "part" } }
                                        text { x: "0", y: "4", text_anchor: "middle", class: "part-label", "{part.reference}" }
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "renderer-panel",
                    h2 { "Existing 3D renderer" }
                    p { "WebGL frame, device-pixel resize, remount, and explicit context-loss failure." }
                    canvas { class: "renderer-canvas", "aria-label": "Rendered keyboard board preview", onmounted: mount_canvas }
                    p { class: "renderer-status", "aria-live": "polite", "{status}" }
                }
            }
        }
    }
}

#[derive(Default)]
struct PanelResources {
    mounted: bool,
    svg: Option<SvgElement>,
    renderer: Option<RendererHost>,
}

const STYLE: &str = r#"
:root { color-scheme: light; font: 15px/1.45 system-ui, sans-serif; color: #1f2937; background: #f4f6fa; }
* { box-sizing: border-box; }
body { margin: 0; }
.p2-shell { max-width: 1440px; margin: 0 auto; padding: 24px; }
h1 { margin: 0 0 4px; font-size: 26px; }
.summary { margin: 0 0 18px; color: #5b6472; }
.toolbar { display: flex; gap: 8px; margin-bottom: 10px; }
button { border: 1px solid #bbc4d2; border-radius: 6px; background: white; padding: 8px 12px; font: inherit; cursor: pointer; }
button:focus-visible, .layout-canvas:focus-visible { outline: 3px solid #4779e5; outline-offset: 2px; }
.status { min-height: 24px; color: #31445f; }
.editor-panel { border: 1px solid #d6dce5; border-radius: 10px; background: #fff; padding: 14px; }
.editor-grid { display: grid; grid-template-columns: minmax(0, 1.15fr) minmax(320px, .85fr); gap: 18px; }
.gesture-panel, .renderer-panel { min-width: 0; }
.gesture-panel h2, .renderer-panel h2 { margin: 0 0 4px; font-size: 18px; }
.gesture-panel p, .renderer-panel p { margin: 0 0 10px; font-size: 13px; color: #697386; }
.layout-canvas { display: block; width: 100%; aspect-ratio: 1000 / 650; border: 1px solid #e0e5ed; border-radius: 8px; background: #fbfcfe; touch-action: none; }
.part { fill: #f4f6fb; stroke: #68768c; stroke-width: 1.4; }
.part.selected { fill: #ffd788; stroke: #be6e12; stroke-width: 2.3; }
.part-label { pointer-events: none; font-size: 10px; fill: #354257; }
.renderer-canvas { display: block; width: 100%; height: 320px; border-radius: 8px; background: #151b25; }
.renderer-status { min-height: 36px; margin-top: 8px !important; }
.panel-unmounted { border: 1px dashed #9aa5b4; padding: 24px; border-radius: 10px; background: white; }
@media (max-width: 860px) { .editor-grid { grid-template-columns: 1fr; } .renderer-canvas { height: 260px; } .p2-shell { padding: 14px; } }
"#;

fn part_bounds(parts: &[boardstudio_core::model::Part]) -> (f64, f64, f64, f64) {
    let mut bounds = parts.iter().fold(
        (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        ),
        |(min_x, min_y, max_x, max_y), part| {
            (
                min_x.min(part.pose.at.x),
                min_y.min(part.pose.at.y),
                max_x.max(part.pose.at.x),
                max_y.max(part.pose.at.y),
            )
        },
    );
    if !bounds.0.is_finite() {
        bounds = (0.0, 0.0, 1.0, 1.0);
    }
    bounds.0 -= 8.0;
    bounds.1 -= 8.0;
    bounds.2 += 8.0;
    bounds.3 += 8.0;
    bounds
}

fn scene_reply(reply: CoreReply) -> Result<(ProjectDoc, SceneDelta), String> {
    match reply {
        CoreReply::Scene {
            document, scene, ..
        } => Ok((*document, scene)),
        CoreReply::Error { message, .. } => Err(message),
        _ => Err("CoreEngine returned a non-scene reply".to_owned()),
    }
}

fn preview_reply(reply: CoreReply) -> Result<SceneDelta, String> {
    match reply {
        CoreReply::Preview { scene, .. } => Ok(scene),
        CoreReply::Error { message, .. } => Err(message),
        _ => Err("CoreEngine returned a non-preview reply".to_owned()),
    }
}

fn preview_request(
    request_id: String,
    transaction_id: String,
    base_revision: u64,
    id: String,
    at: Point,
) -> CoreRequest {
    CoreRequest::Edit {
        id: request_id,
        command: EditCommand {
            base_revision,
            transaction_id,
            phase: EditPhase::Preview,
            target_ids: vec![id.clone()],
            operation: EditOperation::MoveParts {
                positions: vec![Position {
                    id,
                    at: Vec2 { x: at.x, y: at.y },
                }],
            },
        },
    }
}

fn client_to_viewbox(svg: &SvgElement, x: f64, y: f64) -> Option<Point> {
    let width = svg.client_width() as f64;
    let height = svg.client_height() as f64;
    if width <= 0.0 || height <= 0.0 {
        return None;
    }
    let rect = svg.get_bounding_client_rect();
    let local_x = x - rect.left() - svg.client_left() as f64;
    let local_y = y - rect.top() - svg.client_top() as f64;
    Some(Point {
        x: local_x * 1000.0 / width,
        y: local_y * 650.0 / height,
    })
}

fn release_if_captured(svg: &SvgElement, pointer_id: i32) -> Result<(), String> {
    if !svg.has_pointer_capture(pointer_id) {
        return Ok(());
    }
    svg.release_pointer_capture(pointer_id)
        .map_err(|error| format!("Pointer capture release failed: {error:?}"))
}

fn send_history(
    worker: Rc<WorkerOwner>,
    mut document: Signal<Option<Arc<ProjectDoc>>>,
    mut scene: Signal<Option<Arc<SceneDelta>>>,
    mut status: Signal<String>,
    redo: bool,
) {
    if document.read().is_none() {
        return;
    }
    let generation = worker.generation.get();
    let request_id = worker.next_request_id(if redo { "redo" } else { "undo" });
    let request = if redo {
        CoreRequest::Redo { id: request_id }
    } else {
        CoreRequest::Undo { id: request_id }
    };
    spawn_local(async move {
        match worker.request(request).await.and_then(scene_reply) {
            Ok((next_document, next_scene)) if worker.generation.get() == generation => {
                document.set(Some(Arc::new(next_document)));
                scene.set(Some(Arc::new(next_scene)));
                status.set(
                    if redo {
                        "One-step Redo completed"
                    } else {
                        "One-step Undo completed"
                    }
                    .to_owned(),
                );
            }
            Ok(_) => {}
            Err(error) if worker.generation.get() == generation => {
                status.set(format!("History request failed: {error}"))
            }
            Err(_) => {}
        }
    });
}

fn make_pointer_cancel_handler(
    gesture: Rc<RefCell<GestureCoordinator>>,
    sequence: Rc<Cell<u32>>,
    mut preview_scene: Signal<Option<Arc<SceneDelta>>>,
    worker: Rc<WorkerOwner>,
    resources: Rc<RefCell<PanelResources>>,
    mut status: Signal<String>,
    message: &'static str,
) -> impl FnMut(dioxus::prelude::PointerEvent) + 'static {
    move |event| {
        let Some(pointer) = event.data().try_as_web_event() else {
            return;
        };
        let effects = gesture.borrow_mut().pointer_cancel(pointer.pointer_id());
        if effects.is_empty() {
            return;
        }
        let generation = worker.generation.get();
        sequence.set(sequence.get().wrapping_add(1).max(1));
        preview_scene.set(None);
        for effect in effects {
            match effect {
                GestureEffect::Cancel {
                    id,
                    base_revision,
                    transaction_id,
                    origin,
                } => {
                    let request = preview_request(
                        worker.next_request_id("pointer-cancel"),
                        transaction_id,
                        base_revision,
                        id,
                        origin,
                    );
                    let worker = worker.clone();
                    let mut status = status;
                    spawn_local(async move {
                        if let Err(error) = worker.request(request).await.and_then(preview_reply)
                            && worker.generation.get() == generation
                        {
                            status.set(format!("Cancel restoration failed: {error}"));
                        }
                    });
                }
                GestureEffect::Release { pointer_id } => {
                    if let Some(svg) = resources.borrow().svg.as_ref()
                        && let Err(error) = release_if_captured(svg, pointer_id)
                    {
                        status.set(error);
                    }
                }
                _ => {}
            }
        }
        status.set(message.to_owned());
    }
}

fn base_prefix() -> Result<&'static str, String> {
    let path = web_sys::window()
        .ok_or_else(|| "window unavailable".to_owned())?
        .location()
        .pathname()
        .map_err(|error| format!("location unavailable: {error:?}"))?;
    Ok(
        if path.starts_with("/boardstudio/") || path == "/boardstudio" {
            "/boardstudio/"
        } else {
            "/"
        },
    )
}
