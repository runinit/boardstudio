//! Packaged renderer initialization and lifetime of a canvas attachment.
use js_sys::{Array, Float32Array, Function, Object, Promise, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Event, EventTarget, HtmlCanvasElement, ResizeObserver, Window};

#[derive(Debug, PartialEq, Eq)]
pub enum RendererStatus {
    ContextLost,
    ContextRestored,
    Error(String),
}

type Status = Rc<dyn Fn(RendererStatus)>;
type Callback = Closure<dyn FnMut(Event)>;
type Observer = (ResizeObserver, Closure<dyn FnMut()>);
type Frame = Closure<dyn FnMut()>;

pub struct RendererPageHost(Rc<Canvas>);

struct Canvas {
    element: HtmlCanvasElement,
    window: Window,
    constructor: Function,
    renderer: RefCell<JsValue>,
    status: Status,
    // The first entry is the last accepted scene. Keep the packets for context restoration.
    packets: RefCell<[Option<JsValue>; 3]>,
    sequence: Cell<u64>,
    disposed: Cell<bool>,
    lost: Cell<bool>,
    submissions: Cell<u32>,
    frame: Cell<Option<i32>>,
    frame_callback: RefCell<Option<Frame>>,
    observer: RefCell<Option<Observer>>,
    listeners: RefCell<Vec<Listener>>,
    dpr: RefCell<Option<Listener>>,
}

const PACKETS: [&str; 3] = ["setScene", "setState", "setHandles"];

struct Listener(EventTarget, &'static str, Callback);

impl Drop for Listener {
    fn drop(&mut self) {
        let _ = self
            .0
            .remove_event_listener_with_callback(self.1, self.2.as_ref().unchecked_ref());
    }
}

impl RendererPageHost {
    /// Attach synchronously after loading the module, so the caller can use its latest scene.
    pub fn mount(
        module: &JsValue,
        element: HtmlCanvasElement,
        input: JsValue,
        sequence: u64,
        status: Status,
    ) -> Result<Self, String> {
        let constructor = Reflect::get(module, &"Renderer".into())
            .and_then(JsCast::dyn_into::<Function>)
            .map_err(js_error)?;
        let window = web_sys::window().ok_or("Renderer window unavailable")?;
        let renderer = Reflect::construct(&constructor, &Array::of1(&element)).map_err(js_error)?;
        let host = Self(Rc::new(Canvas {
            window,
            element,
            constructor,
            renderer: RefCell::new(renderer),
            status,
            packets: RefCell::new([None, None, None]),
            sequence: Cell::new(0),
            disposed: Cell::new(false),
            lost: Cell::new(false),
            submissions: Cell::new(0),
            frame: Cell::new(None),
            frame_callback: RefCell::new(None),
            observer: RefCell::new(None),
            listeners: RefCell::new(Vec::new()),
            dpr: RefCell::new(None),
        }));
        host.0.install().map_err(js_error)?;
        if !host.submit_scene(input, sequence)? {
            return Err("The renderer rejected its initial scene".into());
        }
        host.0.resize().map_err(js_error)?;
        host.fit()?;
        host.0.mark("active").map_err(js_error)?;
        Ok(host)
    }

    pub fn submit_scene(&self, input: JsValue, sequence: u64) -> Result<bool, String> {
        self.0.ensure_active()?;
        if sequence <= self.0.sequence.get() {
            return Err("Renderer scene sequence must increase".into());
        }
        self.0.sequence.set(sequence);
        let packet = stamp_scene(&input, sequence).map_err(js_error)?;
        let accepted = self
            .call("setScene", std::slice::from_ref(&packet))?
            .as_bool()
            .ok_or("Renderer setScene returned no scene acceptance result")?;
        if accepted {
            self.0.save(0, packet.into())?;
        }
        Ok(accepted)
    }

    pub fn set_display_state(&self, state: JsValue) -> Result<(), String> {
        self.call("setState", std::slice::from_ref(&state))?;
        self.0.save(1, state)
    }

    pub fn set_handles(&self, handles: JsValue) -> Result<(), String> {
        self.call("setHandles", std::slice::from_ref(&handles))?;
        self.0.save(2, handles)
    }

    pub fn orbit(&self, x: f64, y: f64) -> Result<(), String> {
        self.update("orbit", &[x.into(), y.into()]).map(|_| ())
    }

    pub fn zoom(&self, factor: f64) -> Result<(), String> {
        self.update("zoom", &[factor.into()]).map(|_| ())
    }

    pub fn view(&self, preset: &str) -> Result<(), String> {
        self.update("view", &[preset.into()]).map(|_| ())
    }

    pub fn fit(&self) -> Result<(), String> {
        self.update("fit", &[]).map(|_| ())
    }

    pub fn focus_objects(&self, ids: &[String]) -> Result<bool, String> {
        let ids: Array = ids.iter().map(|id| JsValue::from_str(id)).collect();
        self.update("focusObjects", &[ids.into()])?
            .as_bool()
            .ok_or_else(|| "Renderer returned an invalid focus result".into())
    }

    pub fn pick_at_client(&self, x: f64, y: f64) -> Result<Option<String>, String> {
        let coords = self.coordinates(x, y)?;
        let hit = self.call("pick", &coords)?;
        if hit.is_null() || hit.is_undefined() {
            Ok(None)
        } else {
            hit.as_string()
                .map(Some)
                .ok_or_else(|| "Invalid renderer pick result".into())
        }
    }

    pub fn point_on_plane_at_client(
        &self,
        x: f64,
        y: f64,
        z: f32,
    ) -> Result<Option<[f32; 3]>, String> {
        let [x, y] = self.coordinates(x, y)?;
        let hit = self
            .call("pointOnPlane", &[x, y, z.into()])?
            .dyn_into::<Float32Array>()
            .map_err(|_| "Invalid renderer world point")?;
        match hit.length() {
            0 => Ok(None),
            3 => {
                let p = [hit.get_index(0), hit.get_index(1), hit.get_index(2)];
                p.iter()
                    .all(|v| v.is_finite())
                    .then_some(Some(p))
                    .ok_or_else(|| "Non-finite renderer world point".into())
            }
            _ => Err("Invalid renderer world point length".into()),
        }
    }

    fn coordinates(&self, x: f64, y: f64) -> Result<[JsValue; 2], String> {
        let canvas = &self.0.element;
        let rect = canvas.get_bounding_client_rect();
        let x = (x - rect.left()) * f64::from(canvas.width()) / rect.width();
        let y = (y - rect.top()) * f64::from(canvas.height()) / rect.height();
        (x.is_finite() && y.is_finite())
            .then(|| [x.into(), y.into()])
            .ok_or_else(|| "Renderer pointer coordinates are invalid".into())
    }

    fn call(&self, method: &str, args: &[JsValue]) -> Result<JsValue, String> {
        self.0.ensure_active()?;
        self.0.call(method, args).map_err(js_error)
    }

    fn update(&self, method: &str, args: &[JsValue]) -> Result<JsValue, String> {
        let value = self.call(method, args)?;
        self.0.schedule().map_err(js_error)?;
        Ok(value)
    }

    pub fn dispose(&self) -> Result<(), String> {
        let c = &self.0;
        if c.disposed.replace(true) {
            return Ok(());
        }
        if let Some((observer, _)) = c.observer.borrow_mut().take() {
            observer.disconnect();
        }
        c.listeners.borrow_mut().clear();
        c.dpr.borrow_mut().take();
        let errors: Vec<_> = [
            c.cancel_frame(),
            release_renderer(&c.renderer.borrow()),
            release_context(&c.element),
        ]
        .into_iter()
        .filter_map(Result::err)
        .map(js_error)
        .collect();
        c.frame_callback.borrow_mut().take();
        c.mark(if errors.is_empty() {
            "disposed"
        } else {
            "dispose-failed"
        })
        .map_err(js_error)?;
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors.join("; "))
        }
    }
}

impl RendererPageHost {
    /// Test seam for the page suite's renderer tests: the DPR media query listener,
    /// so tests can replace it without reaching into the private host tuple field.
    #[cfg(feature = "test-support")]
    pub fn dpr_media_query(&self) -> Option<web_sys::MediaQueryList> {
        self.0
            .dpr
            .borrow()
            .as_ref()
            .and_then(|entry| entry.0.clone().dyn_into().ok())
    }
}

impl Drop for RendererPageHost {
    fn drop(&mut self) {
        if let Err(error) = self.dispose() {
            (self.0.status)(RendererStatus::Error(error));
        }
    }
}

impl Canvas {
    fn active(&self) -> bool {
        !self.disposed.get() && !self.lost.get()
    }

    fn ensure_active(&self) -> Result<(), String> {
        self.active()
            .then_some(())
            .ok_or_else(|| "3D view is unavailable".into())
    }

    fn call(&self, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
        call(&self.renderer.borrow(), name, args)
    }

    fn save(&self, slot: usize, value: JsValue) -> Result<(), String> {
        self.packets.borrow_mut()[slot] = Some(value);
        self.schedule().map_err(js_error)
    }

    fn mark(&self, state: &str) -> Result<(), JsValue> {
        self.element.set_attribute("data-renderer-state", state)
    }

    fn weak_callback(self: &Rc<Self>, run: impl Fn(&Self) + 'static) -> Frame {
        let weak = Rc::downgrade(self);
        Frame::new(move || weak.upgrade().map_or((), |canvas| run(&canvas)))
    }

    fn install(self: &Rc<Self>) -> Result<(), JsValue> {
        *self.frame_callback.borrow_mut() = Some(self.weak_callback(|canvas| {
            canvas.frame.set(None);
            if canvas.active()
                && let Err(error) = canvas.draw()
            {
                canvas.lost.set(true);
                let _ = canvas.mark("frame-failed");
                (canvas.status)(RendererStatus::Error(format!(
                    "Renderer submission failed: {}",
                    js_error(error)
                )));
            }
        }));
        let callback = self.weak_callback(|canvas| canvas.report(canvas.resize()));
        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref())?;
        observer.observe(&self.element);
        *self.observer.borrow_mut() = Some((observer, callback));
        for (target, event) in [
            (self.window.as_ref(), "resize"),
            (self.element.as_ref(), "webglcontextlost"),
            (self.element.as_ref(), "webglcontextrestored"),
        ] {
            self.listeners
                .borrow_mut()
                .push(self.listen(target, event)?);
        }
        self.watch_dpr()
    }

    fn listen(
        self: &Rc<Self>,
        target: &EventTarget,
        name: &'static str,
    ) -> Result<Listener, JsValue> {
        let weak = Rc::downgrade(self);
        let callback = Callback::new(move |event: Event| {
            if let Some(c) = weak.upgrade() {
                c.report(match name {
                    "webglcontextlost" => {
                        event.prevent_default();
                        c.lost.set(true);
                        (c.status)(RendererStatus::ContextLost);
                        c.cancel_frame().and(c.mark("context-lost"))
                    }
                    "webglcontextrestored" => c.restore(),
                    "change" => {
                        c.report(c.resize());
                        c.watch_dpr()
                    }
                    _ => c.resize(),
                });
            }
        });
        target.add_event_listener_with_callback(name, callback.as_ref().unchecked_ref())?;
        Ok(Listener(target.clone(), name, callback))
    }

    fn report(&self, result: Result<(), JsValue>) {
        if let Err(error) = result {
            (self.status)(RendererStatus::Error(js_error(error)));
        }
    }

    fn watch_dpr(self: &Rc<Self>) -> Result<(), JsValue> {
        let query = format!("(resolution: {}dppx)", self.window.device_pixel_ratio());
        let query = self
            .window
            .match_media(&query)?
            .ok_or("DPR query unavailable")?;
        let listener = self.listen(query.as_ref(), "change")?;
        self.dpr.borrow_mut().replace(listener);
        Ok(())
    }

    fn draw(&self) -> Result<(), JsValue> {
        self.call("render", &[])?;
        let count = self.submissions.get().saturating_add(1);
        self.submissions.set(count);
        self.element
            .set_attribute("data-render-submissions", &count.to_string())
    }

    fn schedule(&self) -> Result<(), JsValue> {
        if self.active()
            && self.frame.get().is_none()
            && let Some(callback) = self.frame_callback.borrow().as_ref()
        {
            self.frame
                .set(Some(self.window.request_animation_frame(
                    callback.as_ref().unchecked_ref(),
                )?));
        }
        Ok(())
    }

    fn cancel_frame(&self) -> Result<(), JsValue> {
        self.frame
            .take()
            .map_or(Ok(()), |id| self.window.cancel_animation_frame(id))
    }

    fn resize(&self) -> Result<(), JsValue> {
        if !self.active() {
            return Ok(());
        }
        let canvas = &self.element;
        let dpr = self.window.device_pixel_ratio().max(1.0);
        let scale = |length: i32| (f64::from(length) * dpr).round().max(1.0) as u32;
        let (width, height) = (scale(canvas.client_width()), scale(canvas.client_height()));
        if (canvas.width(), canvas.height()) != (width, height) {
            canvas.set_width(width);
            canvas.set_height(height);
            self.call("resize", &[width.into(), height.into()])?;
            let count = canvas
                .get_attribute("data-resize-count")
                .and_then(|value| value.parse::<u32>().ok())
                .unwrap_or(0)
                .saturating_add(1);
            canvas.set_attribute("data-resize-count", &count.to_string())?;
            self.schedule()?;
        }
        canvas.set_attribute("data-dpr", &dpr.to_string())
    }

    fn replay(&self, canvas: &HtmlCanvasElement, next: &JsValue) -> Result<(), JsValue> {
        for (name, packet) in PACKETS.into_iter().zip(self.packets.borrow().iter()) {
            if let Some(packet) = packet {
                let answer = call(next, name, std::slice::from_ref(packet))?;
                if name == "setScene" && answer.as_bool() != Some(true) {
                    return Err("Renderer rejected its restored scene".into());
                }
            }
        }
        call(
            next,
            "resize",
            &[canvas.width().into(), canvas.height().into()],
        )?;
        call(next, "fit", &[])?;
        Ok(())
    }

    fn restore(&self) -> Result<(), JsValue> {
        if self.disposed.get() || !self.lost.get() {
            return Ok(());
        }
        let canvas = &self.element;
        let next = Reflect::construct(&self.constructor, &Array::of1(canvas))?;
        if let Err(error) = self.replay(canvas, &next) {
            let _ = release_renderer(&next);
            return Err(error);
        }
        self.report(release_renderer(&self.renderer.replace(next)));
        self.lost.set(false);
        self.resize()?;
        self.schedule()?;
        self.mark("active")?;
        (self.status)(RendererStatus::ContextRestored);
        Ok(())
    }
}

fn release_renderer(renderer: &JsValue) -> Result<(), JsValue> {
    call(renderer, "dispose", &[])
        .and(call(renderer, "free", &[]))
        .map(|_| ())
}

fn release_context(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    if let Some(context) = canvas.get_context("webgl2")?
        && call(context.as_ref(), "isContextLost", &[])?.as_bool() != Some(true)
    {
        let ext = call(
            context.as_ref(),
            "getExtension",
            &["WEBGL_lose_context".into()],
        )?;
        if !ext.is_null() && !ext.is_undefined() {
            call(&ext, "loseContext", &[])?;
        }
    }
    Ok(())
}

fn call(receiver: &JsValue, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    Reflect::get(receiver, &name.into())?
        .dyn_into::<Function>()?
        .apply(receiver, &args.iter().collect::<Array>())
}

fn stamp_scene(input: &JsValue, sequence: u64) -> Result<Object, JsValue> {
    let object = input
        .dyn_ref::<Object>()
        .ok_or("Renderer scene packet must be an object")?;
    let copy = Object::assign(&Object::new(), object);
    Reflect::set(&copy, &"revision".into(), &(sequence as f64).into())?;
    let board = Reflect::get(&copy, &"board".into())?;
    if !board.is_null() && !board.is_undefined() {
        let board = Object::assign(&Object::new(), board.unchecked_ref());
        Reflect::set(&board, &"revision".into(), &(sequence as f64).into())?;
        Reflect::set(&copy, &"board".into(), &board)?;
    }
    Ok(copy)
}

thread_local! {
    static RENDERER_MODULE: RefCell<Option<Promise>> = const { RefCell::new(None) };
}

pub async fn renderer_module() -> Result<JsValue, String> {
    let promise = RENDERER_MODULE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if let Some(promise) = slot.as_ref() {
            return Ok(promise.clone());
        }
        let url = resource_url("assets/renderer/boardstudio_renderer_wasm.js")?;
        let load = Function::new_with_args(
            "url",
            "return import(url).then(async module => { await module.default(); return module; })",
        );
        let promise = load
            .call1(&JsValue::NULL, &url.into())
            .and_then(JsCast::dyn_into::<Promise>)
            .map_err(js_error)?;
        *slot = Some(promise.clone());
        Ok::<_, String>(promise)
    })?;
    let result = JsFuture::from(promise.clone()).await.map_err(js_error);
    if result.is_err() {
        RENDERER_MODULE.with(|slot| {
            let mut slot = slot.borrow_mut();
            if slot
                .as_ref()
                .is_some_and(|cached| Object::is(cached.as_ref(), promise.as_ref()))
            {
                slot.take();
            }
        });
    }
    result
}

fn resource_url(path: &str) -> Result<String, String> {
    let location = web_sys::window()
        .ok_or("Renderer window unavailable")?
        .location();
    let path_name = location.pathname().map_err(js_error)?;
    let origin = location.origin().map_err(js_error)?;
    let base = if path_name.starts_with("/boardstudio/") || path_name == "/boardstudio" {
        "/boardstudio"
    } else {
        ""
    };
    Ok(format!("{origin}{base}/{path}"))
}

pub use crate::browser::js_error;
