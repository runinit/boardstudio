//! Candidate production adoption of the accepted P2 renderer attachment lifetime.
//! The caller supplies neutral provider-derived scenes; the host owns DOM/GPU cleanup.
//! Production acceptance still requires the M1 renderer and exact-case browser gates.
use js_sys::{Array, Function, Promise, Reflect};
use std::{
    cell::{Cell, RefCell},
    rc::{Rc, Weak},
};
use wasm_bindgen::{JsCast, JsValue, closure::Closure};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Event, HtmlCanvasElement, MediaQueryList, ResizeObserver, Window};

pub struct RendererHost {
    inner: Rc<RendererInner>,
}

type EventCallback = Closure<dyn FnMut(Event)>;
type ObserverCallback = Closure<dyn FnMut(Array, ResizeObserver)>;
type FrameCallback = Closure<dyn FnMut(f64)>;

struct RendererInner {
    renderer: JsValue,
    canvas: HtmlCanvasElement,
    status: Rc<dyn Fn(String)>,
    disposed: Cell<bool>,
    context_lost: Cell<bool>,
    frame_id: Cell<i32>,
    render_submissions: Cell<u32>,
    observer: RefCell<Option<ResizeObserver>>,
    observer_callback: RefCell<Option<ObserverCallback>>,
    resize_callback: RefCell<Option<EventCallback>>,
    dpr_query: RefCell<Option<MediaQueryList>>,
    dpr_callback: RefCell<Option<EventCallback>>,
    context_callback: RefCell<Option<EventCallback>>,
    frame_callback: RefCell<Option<FrameCallback>>,
}

impl RendererHost {
    pub async fn mount(
        canvas: HtmlCanvasElement,
        input: JsValue,
        status: Rc<dyn Fn(String)>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Self, String> {
        ensure_current(&is_current)?;
        let module_url = resource_url("assets/renderer/boardstudio_renderer_wasm.js")?;
        let import = Function::new_with_args("url", "return import(url)");
        let module = import
            .call1(&JsValue::NULL, &JsValue::from_str(&module_url))
            .map_err(js_error)?
            .dyn_into::<Promise>()
            .map_err(js_error)?;
        let module = JsFuture::from(module).await.map_err(js_error)?;
        ensure_current(&is_current)?;
        let init = Reflect::get(&module, &JsValue::from_str("default"))
            .map_err(js_error)?
            .dyn_into::<Function>()
            .map_err(js_error)?;
        let ready = init.call0(&JsValue::NULL).map_err(js_error)?;
        JsFuture::from(ready.dyn_into::<Promise>().map_err(js_error)?)
            .await
            .map_err(js_error)?;
        ensure_current(&is_current)?;

        let constructor = Reflect::get(&module, &JsValue::from_str("Renderer"))
            .map_err(js_error)?
            .dyn_into::<Function>()
            .map_err(js_error)?;
        ensure_current(&is_current)?;
        let arguments = Array::new();
        arguments.push(&canvas);
        let renderer = Reflect::construct(&constructor, &arguments).map_err(js_error)?;
        Self::attach(renderer, canvas, input, status, is_current)
    }

    /// Take ownership of an already constructed renderer object. Everything
    /// fallible after this point is covered by the cleanup guard below.
    fn attach(
        renderer: JsValue,
        canvas: HtmlCanvasElement,
        input: JsValue,
        status: Rc<dyn Fn(String)>,
        is_current: Rc<dyn Fn() -> bool>,
    ) -> Result<Self, String> {
        // From the first allocation onward all fallible initialization is owned
        // by this guard; any failure runs every available cleanup step.
        let inner = Rc::new(RendererInner {
            renderer,
            canvas,
            status,
            disposed: Cell::new(false),
            context_lost: Cell::new(false),
            frame_id: Cell::new(0),
            render_submissions: Cell::new(0),
            observer: RefCell::new(None),
            observer_callback: RefCell::new(None),
            resize_callback: RefCell::new(None),
            dpr_query: RefCell::new(None),
            dpr_callback: RefCell::new(None),
            context_callback: RefCell::new(None),
            frame_callback: RefCell::new(None),
        });
        let initialized = (|| -> Result<(), String> {
            ensure_current(&is_current)?;
            call_method(&inner.renderer, "setScene", &[input]).map_err(js_error)?;
            call_method(&inner.renderer, "fit", &[]).map_err(js_error)?;
            install_lifecycle(&inner).map_err(js_error)?;
            resize(&inner).map_err(js_error)?;
            schedule_frame(&inner).map_err(js_error)?;
            ensure_current(&is_current)?;
            inner
                .canvas
                .set_attribute("data-renderer-state", "active")
                .map_err(js_error)?;
            (inner.status)("Renderer initialized; first render submission queued".to_owned());
            Ok(())
        })();
        if let Err(error) = initialized {
            return match dispose(&inner) {
                Ok(()) => Err(error),
                Err(cleanup) => Err(format!("{error}; renderer cleanup also failed: {cleanup}")),
            };
        }
        Ok(Self { inner })
    }

    pub fn update_scene(&self, input: JsValue) -> Result<(), String> {
        if self.inner.disposed.get() || self.inner.context_lost.get() {
            return Err("Renderer is no longer active".to_owned());
        }
        call_method(&self.inner.renderer, "setScene", &[input]).map_err(js_error)?;
        schedule_frame(&self.inner).map_err(js_error)
    }

    pub fn orbit(&self, delta_x: f64, delta_y: f64) -> Result<(), String> {
        self.camera("orbit", &[delta_x.into(), delta_y.into()])
    }
    pub fn zoom(&self, factor: f64) -> Result<(), String> {
        self.camera("zoom", &[factor.into()])
    }
    pub fn view(&self, preset: &str) -> Result<(), String> {
        self.camera("view", &[preset.into()])
    }
    pub fn fit(&self) -> Result<(), String> {
        self.camera("fit", &[])
    }
    fn camera(&self, method: &str, arguments: &[JsValue]) -> Result<(), String> {
        if self.inner.disposed.get() || self.inner.context_lost.get() {
            return Err("3D view is unavailable; reopen the preview.".into());
        }
        call_method(&self.inner.renderer, method, arguments).map_err(js_error)?;
        schedule_frame(&self.inner).map_err(js_error)
    }

    pub fn dispose(&self) -> Result<(), String> {
        dispose(&self.inner)
    }
}

impl Drop for RendererHost {
    fn drop(&mut self) {
        if let Err(error) = dispose(&self.inner) {
            (self.inner.status)(format!("Renderer disposal failed: {error}"));
        }
    }
}

fn ensure_current(is_current: &Rc<dyn Fn() -> bool>) -> Result<(), String> {
    if is_current() {
        Ok(())
    } else {
        Err("Renderer mount scope was cancelled".to_owned())
    }
}

fn install_lifecycle(inner: &Rc<RendererInner>) -> Result<(), JsValue> {
    let weak = Rc::downgrade(inner);
    let observer_callback = Closure::<dyn FnMut(Array, ResizeObserver)>::new(move |_, _| {
        if let Some(inner) = weak.upgrade()
            && let Err(error) = resize(&inner)
        {
            (inner.status)(format!("Resize failed: {}", js_error(error)));
        }
    });
    let observer = ResizeObserver::new(observer_callback.as_ref().unchecked_ref())?;
    observer.observe(&inner.canvas);
    *inner.observer.borrow_mut() = Some(observer);
    *inner.observer_callback.borrow_mut() = Some(observer_callback);

    let weak = Rc::downgrade(inner);
    let resize_callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(inner) = weak.upgrade()
            && let Err(error) = resize(&inner)
        {
            (inner.status)(format!("Resize failed: {}", js_error(error)));
        }
    });
    window()?
        .add_event_listener_with_callback("resize", resize_callback.as_ref().unchecked_ref())?;
    *inner.resize_callback.borrow_mut() = Some(resize_callback);

    let weak = Rc::downgrade(inner);
    let dpr_callback = Closure::<dyn FnMut(Event)>::new(move |_| {
        if let Some(inner) = weak.upgrade() {
            if let Err(error) = resize(&inner) {
                (inner.status)(format!("DPR resize failed: {}", js_error(error)));
            }
            if let Err(error) = arm_dpr_listener(&inner) {
                (inner.status)(format!("DPR listener re-arm failed: {}", js_error(error)));
            }
        }
    });
    *inner.dpr_callback.borrow_mut() = Some(dpr_callback);
    arm_dpr_listener(inner)?;

    let weak = Rc::downgrade(inner);
    let context_callback = Closure::<dyn FnMut(Event)>::new(move |event: Event| {
        event.prevent_default();
        if let Some(inner) = weak.upgrade() {
            inner.context_lost.set(true);
            if let Err(error) = inner
                .canvas
                .set_attribute("data-renderer-state", "context-lost")
            {
                (inner.status)(format!(
                    "Context-loss state update failed: {}",
                    js_error(error)
                ));
            }
            if let Err(error) = cancel_frame(&inner) {
                (inner.status)(format!("Context loss cleanup failed: {}", js_error(error)));
            }
            (inner.status)("WebGL context lost; renderer stopped and reported failure".to_owned());
        }
    });
    inner.canvas.add_event_listener_with_callback(
        "webglcontextlost",
        context_callback.as_ref().unchecked_ref(),
    )?;
    *inner.context_callback.borrow_mut() = Some(context_callback);

    let weak: Weak<RendererInner> = Rc::downgrade(inner);
    let frame_callback = Closure::<dyn FnMut(f64)>::new(move |_| {
        if let Some(inner) = weak.upgrade() {
            // RAF is one-shot. Clear its identity before rendering so a resize
            // or scene update may queue the next dirty frame.
            inner.frame_id.set(0);
            if inner.disposed.get() || inner.context_lost.get() {
                return;
            }
            match call_method(&inner.renderer, "render", &[]) {
                Ok(_) => {
                    let submissions = inner.render_submissions.get().saturating_add(1);
                    inner.render_submissions.set(submissions);
                    if let Err(error) = inner
                        .canvas
                        .set_attribute("data-render-submissions", &submissions.to_string())
                    {
                        (inner.status)(format!(
                            "Render submission counter failed: {}",
                            js_error(error)
                        ));
                    }
                }
                Err(error) => {
                    inner.context_lost.set(true);
                    if let Err(state_error) = inner
                        .canvas
                        .set_attribute("data-renderer-state", "frame-failed")
                    {
                        (inner.status)(format!(
                            "Renderer state update failed: {}",
                            js_error(state_error)
                        ));
                    }
                    (inner.status)(format!("Renderer submission failed: {}", js_error(error)));
                }
            }
        }
    });
    *inner.frame_callback.borrow_mut() = Some(frame_callback);
    Ok(())
}

fn resize(inner: &RendererInner) -> Result<(), JsValue> {
    if inner.disposed.get() || inner.context_lost.get() {
        return Ok(());
    }
    let scale = window()?.device_pixel_ratio().max(1.0);
    let width = ((inner.canvas.client_width() as f64) * scale)
        .round()
        .max(1.0) as u32;
    let height = ((inner.canvas.client_height() as f64) * scale)
        .round()
        .max(1.0) as u32;
    if inner.canvas.width() != width || inner.canvas.height() != height {
        inner.canvas.set_width(width);
        inner.canvas.set_height(height);
        call_method(&inner.renderer, "resize", &[width.into(), height.into()])?;
        let count = inner
            .canvas
            .get_attribute("data-resize-count")
            .and_then(|value| value.parse::<u32>().ok())
            .unwrap_or(0)
            .saturating_add(1);
        inner
            .canvas
            .set_attribute("data-resize-count", &count.to_string())?;
        schedule_frame(inner)?;
    }
    inner.canvas.set_attribute("data-dpr", &scale.to_string())?;
    Ok(())
}

fn arm_dpr_listener(inner: &RendererInner) -> Result<(), JsValue> {
    if inner.disposed.get() {
        return Ok(());
    }
    let callback = inner.dpr_callback.borrow();
    let callback = callback
        .as_ref()
        .ok_or_else(|| JsValue::from_str("DPR callback unavailable"))?
        .as_ref()
        .unchecked_ref::<Function>()
        .clone();
    let scale = window()?.device_pixel_ratio().max(1.0);
    let query = window()?
        .match_media(&format!("(resolution: {scale}dppx)"))?
        .ok_or_else(|| JsValue::from_str("resolution media query unavailable"))?;
    query.add_event_listener_with_callback("change", &callback)?;
    if let Some(previous) = inner.dpr_query.borrow_mut().replace(query) {
        previous.remove_event_listener_with_callback("change", &callback)?;
    }
    Ok(())
}

fn schedule_frame(inner: &RendererInner) -> Result<(), JsValue> {
    if inner.disposed.get() || inner.context_lost.get() || inner.frame_id.get() != 0 {
        return Ok(());
    }
    let callback = inner.frame_callback.borrow();
    let callback = callback
        .as_ref()
        .ok_or_else(|| JsValue::from_str("frame callback unavailable"))?;
    let id = window()?.request_animation_frame(callback.as_ref().unchecked_ref())?;
    inner.frame_id.set(id);
    Ok(())
}

fn cancel_frame(inner: &RendererInner) -> Result<(), JsValue> {
    let id = inner.frame_id.replace(0);
    if id != 0 {
        window()?.cancel_animation_frame(id)?;
    }
    Ok(())
}

fn dispose(inner: &RendererInner) -> Result<(), String> {
    if inner.disposed.replace(true) {
        return Ok(());
    }
    let mut failures = Vec::new();
    if let Some(observer) = inner.observer.borrow_mut().take() {
        observer.disconnect();
    }
    if let Some(callback) = inner.resize_callback.borrow_mut().take() {
        match window().and_then(|window| {
            window.remove_event_listener_with_callback("resize", callback.as_ref().unchecked_ref())
        }) {
            Ok(()) => {}
            Err(error) => failures.push(format!("remove resize listener: {}", js_error(error))),
        }
    }
    if let Some(query) = inner.dpr_query.borrow_mut().take() {
        let callback = inner.dpr_callback.borrow();
        if let Some(callback) = callback.as_ref() {
            match query
                .remove_event_listener_with_callback("change", callback.as_ref().unchecked_ref())
            {
                Ok(()) => {}
                Err(error) => failures.push(format!("remove DPR listener: {}", js_error(error))),
            }
        }
    }
    if let Some(callback) = inner.context_callback.borrow_mut().take() {
        match inner.canvas.remove_event_listener_with_callback(
            "webglcontextlost",
            callback.as_ref().unchecked_ref(),
        ) {
            Ok(()) => {}
            Err(error) => failures.push(format!("remove context listener: {}", js_error(error))),
        }
    }
    if let Err(error) = cancel_frame(inner) {
        failures.push(format!("cancel animation frame: {}", js_error(error)));
    }
    if let Err(error) = call_method(&inner.renderer, "dispose", &[]) {
        failures.push(format!("renderer dispose: {}", js_error(error)));
    }
    if let Err(error) = call_method(&inner.renderer, "free", &[]) {
        failures.push(format!("renderer free: {}", js_error(error)));
    }
    if let Err(error) = release_graphics_context(&inner.canvas) {
        failures.push(format!("release WebGL context: {}", js_error(error)));
    }
    inner.observer_callback.borrow_mut().take();
    inner.dpr_callback.borrow_mut().take();
    inner.frame_callback.borrow_mut().take();
    let state = if failures.is_empty() {
        "disposed"
    } else {
        "dispose-failed"
    };
    if let Err(error) = inner.canvas.set_attribute("data-renderer-state", state) {
        failures.push(format!("set renderer state: {}", js_error(error)));
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

fn release_graphics_context(canvas: &HtmlCanvasElement) -> Result<(), JsValue> {
    let Some(context) = canvas.get_context("webgl2")? else {
        return Ok(());
    };
    if call_method(&context, "isContextLost", &[])?.as_bool() == Some(true) {
        return Ok(());
    }
    let extension = call_method(
        &context,
        "getExtension",
        &[JsValue::from_str("WEBGL_lose_context")],
    )?;
    if extension.is_null() || extension.is_undefined() {
        return Err(JsValue::from_str("WEBGL_lose_context is unavailable"));
    }
    // The canvas is leaving this mount permanently. This releases the context's
    // internal VAO too; three-d 0.19 does not expose that object for deletion.
    call_method(&extension, "loseContext", &[])?;
    Ok(())
}

fn call_method(receiver: &JsValue, name: &str, arguments: &[JsValue]) -> Result<JsValue, JsValue> {
    let function = Reflect::get(receiver, &JsValue::from_str(name))?.dyn_into::<Function>()?;
    let args = Array::new();
    for argument in arguments {
        args.push(argument);
    }
    function.apply(receiver, &args)
}

fn resource_url(path: &str) -> Result<String, String> {
    let location = window().map_err(js_error)?.location();
    let path_name = location.pathname().map_err(js_error)?;
    let origin = location.origin().map_err(js_error)?;
    let prefix = if path_name.starts_with("/boardstudio/") || path_name == "/boardstudio" {
        "/boardstudio/"
    } else {
        "/"
    };
    // Dynamic Function imports can retain an earlier document's referrer base
    // after navigation. Pin both origin and prefix to the current host page.
    Ok(format!("{origin}{prefix}{path}"))
}

fn window() -> Result<Window, JsValue> {
    web_sys::window().ok_or_else(|| JsValue::from_str("window unavailable"))
}

fn js_error(error: JsValue) -> String {
    error
        .as_string()
        .or_else(|| {
            Reflect::get(&error, &JsValue::from_str("message"))
                .ok()
                .and_then(|value| value.as_string())
        })
        .or_else(|| {
            js_sys::JSON::stringify(&error)
                .ok()
                .and_then(|value| value.as_string())
        })
        .unwrap_or_else(|| "unknown browser error".to_owned())
}

#[cfg(test)]
mod lifecycle_tests {
    //! Deterministic lifecycle checks against a scripted renderer object. They need
    //! a browser DOM (canvas, window, ResizeObserver) but no real WebGL context:
    //! teardown tolerates a canvas without one, and `loseContext` is only reached
    //! when the browser happens to provide a context.
    use super::*;
    use wasm_bindgen_test::wasm_bindgen_test;

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn scripted_renderer(fail: &str) -> JsValue {
        Function::new_with_args(
            "fail",
            "const calls = { setScene: 0, fit: 0, resize: 0, render: 0, dispose: 0, free: 0, zoom: 0, args: {} };
             const hook = (name) => function () {
               calls[name] += 1;
               (calls.args[name] ||= []).push(Array.from(arguments));
               if (fail === name) { throw new Error(name + ' failed'); }
             };
             return { calls, setScene: hook('setScene'), fit: hook('fit'), resize: hook('resize'),
                      render: hook('render'), dispose: hook('dispose'), free: hook('free'),
                      zoom: hook('zoom') };",
        )
        .call1(&JsValue::NULL, &JsValue::from_str(fail))
        .unwrap()
    }

    fn calls(renderer: &JsValue, name: &str) -> u32 {
        let calls = Reflect::get(renderer, &JsValue::from_str("calls")).unwrap();
        Reflect::get(&calls, &JsValue::from_str(name))
            .unwrap()
            .as_f64()
            .unwrap() as u32
    }

    fn resize_args(renderer: &JsValue, index: u32) -> (u32, u32) {
        let calls = Reflect::get(renderer, &JsValue::from_str("calls")).unwrap();
        let args = Reflect::get(&calls, &JsValue::from_str("args")).unwrap();
        let resizes = Reflect::get(&args, &JsValue::from_str("resize"))
            .unwrap()
            .dyn_into::<Array>()
            .unwrap();
        let call = resizes.get(index).dyn_into::<Array>().unwrap();
        (
            call.get(0).as_f64().unwrap() as u32,
            call.get(1).as_f64().unwrap() as u32,
        )
    }

    fn controlled_dpr(value: f64) -> JsValue {
        Function::new_with_args(
            "value",
            "const descriptor = Object.getOwnPropertyDescriptor(window, 'devicePixelRatio');
             Object.defineProperty(window, 'devicePixelRatio', { configurable: true, value });
             return descriptor || null;",
        )
        .call1(&JsValue::NULL, &JsValue::from_f64(value))
        .unwrap()
    }

    fn restore_dpr(descriptor: &JsValue) {
        Function::new_with_args(
            "descriptor",
            "if (descriptor) { Object.defineProperty(window, 'devicePixelRatio', descriptor); }
             else { delete window.devicePixelRatio; }",
        )
        .call1(&JsValue::NULL, descriptor)
        .unwrap();
    }

    /// Count live (target, type, callback) registrations for the listener types the
    /// host installs, plus ResizeObserver observe/disconnect balance.
    struct ListenerSpy;

    impl ListenerSpy {
        fn install() -> Self {
            Function::new_no_args(
                "if (window.__rendererSpy) { return; }
                 const spy = { entries: [], observers: 0, queries: [], original: {} };
                 const watched = ['resize', 'change', 'webglcontextlost'];
                 const proto = EventTarget.prototype;
                 spy.original.add = proto.addEventListener;
                 spy.original.remove = proto.removeEventListener;
                 proto.addEventListener = function (type, callback, options) {
                   if (type === 'change' && typeof this.media === 'string') { spy.queries.push(this); }
                   if (watched.includes(type) && !spy.entries.some((e) => e.t === this && e.type === type && e.cb === callback)) {
                     spy.entries.push({ t: this, type, cb: callback });
                   }
                   return spy.original.add.call(this, type, callback, options);
                 };
                 proto.removeEventListener = function (type, callback, options) {
                   spy.entries = spy.entries.filter((e) => !(e.t === this && e.type === type && e.cb === callback));
                   return spy.original.remove.call(this, type, callback, options);
                 };
                 spy.original.observe = ResizeObserver.prototype.observe;
                 spy.original.disconnect = ResizeObserver.prototype.disconnect;
                 ResizeObserver.prototype.observe = function (...args) { spy.observers += 1; return spy.original.observe.apply(this, args); };
                 ResizeObserver.prototype.disconnect = function (...args) { spy.observers -= 1; return spy.original.disconnect.apply(this, args); };
                 window.__rendererSpy = spy;",
            )
            .call0(&JsValue::NULL)
            .unwrap();
            Self
        }

        fn live_listeners(&self) -> u32 {
            Function::new_no_args("return window.__rendererSpy.entries.length")
                .call0(&JsValue::NULL)
                .unwrap()
                .as_f64()
                .unwrap() as u32
        }

        fn query_count(&self) -> u32 {
            Function::new_no_args("return window.__rendererSpy.queries.length")
                .call0(&JsValue::NULL)
                .unwrap()
                .as_f64()
                .unwrap() as u32
        }

        fn query(&self, index: u32) -> MediaQueryList {
            Function::new_with_args("index", "return window.__rendererSpy.queries[index]")
                .call1(&JsValue::NULL, &JsValue::from_f64(index as f64))
                .unwrap()
                .dyn_into::<MediaQueryList>()
                .unwrap()
        }

        fn has_live_change_listener(&self, query: &MediaQueryList) -> bool {
            Function::new_with_args(
                "query",
                "return window.__rendererSpy.entries.some((entry) => entry.t === query && entry.type === 'change')",
            )
            .call1(&JsValue::NULL, query.as_ref())
            .unwrap()
            .as_bool()
            .unwrap()
        }

        fn live_observers(&self) -> i32 {
            Function::new_no_args("return window.__rendererSpy.observers")
                .call0(&JsValue::NULL)
                .unwrap()
                .as_f64()
                .unwrap() as i32
        }
    }

    impl Drop for ListenerSpy {
        fn drop(&mut self) {
            let _ = Function::new_no_args(
                "const spy = window.__rendererSpy; if (!spy) { return; }
                 EventTarget.prototype.addEventListener = spy.original.add;
                 EventTarget.prototype.removeEventListener = spy.original.remove;
                 ResizeObserver.prototype.observe = spy.original.observe;
                 ResizeObserver.prototype.disconnect = spy.original.disconnect;
                 delete window.__rendererSpy;",
            )
            .call0(&JsValue::NULL);
        }
    }

    fn make_canvas() -> HtmlCanvasElement {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas = document
            .create_element("canvas")
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        canvas
            .set_attribute("style", "width:200px;height:100px")
            .unwrap();
        document.body().unwrap().append_child(&canvas).unwrap();
        canvas
    }

    fn status_log() -> (Rc<dyn Fn(String)>, Rc<RefCell<Vec<String>>>) {
        let log = Rc::new(RefCell::new(Vec::new()));
        let sink = log.clone();
        (Rc::new(move |message| sink.borrow_mut().push(message)), log)
    }

    fn attach(
        renderer: &JsValue,
        canvas: &HtmlCanvasElement,
        status: Rc<dyn Fn(String)>,
    ) -> Result<RendererHost, String> {
        RendererHost::attach(
            renderer.clone(),
            canvas.clone(),
            JsValue::from_str("scene"),
            status,
            Rc::new(|| true),
        )
    }

    async fn settle() {
        gloo_timers::future::TimeoutFuture::new(120).await;
    }

    fn state(canvas: &HtmlCanvasElement) -> Option<String> {
        canvas.get_attribute("data-renderer-state")
    }

    #[wasm_bindgen_test]
    async fn resize_and_dpr_changes_update_backing_size_and_rearm_listener() {
        let spy = ListenerSpy::install();
        let baseline = spy.live_listeners();
        let descriptor = controlled_dpr(1.0);
        let renderer = scripted_renderer("");
        let canvas = make_canvas();
        let (status, _) = status_log();
        let host = attach(&renderer, &canvas, status).expect("scripted renderer attaches");
        assert_eq!((canvas.width(), canvas.height()), (200, 100));
        assert_eq!(resize_args(&renderer, 0), (200, 100));
        assert_eq!(canvas.get_attribute("data-dpr").as_deref(), Some("1"));
        assert_eq!(spy.query_count(), 1);
        let old_query = spy.query(0);
        assert!(spy.has_live_change_listener(&old_query));

        canvas
            .set_attribute("style", "width:240px;height:120px")
            .unwrap();
        web_sys::window()
            .unwrap()
            .dispatch_event(&web_sys::Event::new("resize").unwrap())
            .unwrap();
        assert_eq!((canvas.width(), canvas.height()), (240, 120));
        assert_eq!(resize_args(&renderer, 1), (240, 120));

        let _ = controlled_dpr(2.0);
        old_query
            .dispatch_event(&web_sys::Event::new("change").unwrap())
            .unwrap();
        assert_eq!((canvas.width(), canvas.height()), (480, 240));
        assert_eq!(resize_args(&renderer, 2), (480, 240));
        assert_eq!(canvas.get_attribute("data-dpr").as_deref(), Some("2"));
        assert_eq!(
            spy.query_count(),
            2,
            "DPR listener is re-armed for the new scale"
        );
        assert!(
            !spy.has_live_change_listener(&old_query),
            "old media query listener is removed"
        );
        let new_query = spy.query(1);
        assert!(spy.has_live_change_listener(&new_query));

        drop(host);
        assert_eq!(calls(&renderer, "dispose"), 1);
        assert_eq!(calls(&renderer, "free"), 1);
        assert_eq!(
            spy.live_listeners(),
            baseline,
            "resize/DPR listeners are disposed"
        );
        assert_eq!(spy.live_observers(), 0);
        restore_dpr(&descriptor);
        canvas.remove();
    }

    #[wasm_bindgen_test]
    async fn context_loss_settles_callers_stops_rendering_and_unmount_releases_everything() {
        let spy = ListenerSpy::install();
        let baseline = spy.live_listeners();
        let renderer = scripted_renderer("");
        let canvas = make_canvas();
        let (status, log) = status_log();
        let host = attach(&renderer, &canvas, status).expect("scripted renderer attaches");
        settle().await;
        assert_eq!(state(&canvas).as_deref(), Some("active"));
        assert!(calls(&renderer, "render") >= 1, "first frame is submitted");
        assert!(
            spy.live_listeners() > baseline,
            "lifecycle listeners are installed"
        );

        let init = web_sys::EventInit::new();
        init.set_cancelable(true);
        let lost = web_sys::Event::new_with_event_init_dict("webglcontextlost", &init).unwrap();
        canvas.dispatch_event(&lost).unwrap();
        assert!(
            lost.default_prevented(),
            "loss must be reported, not auto-restored"
        );
        assert_eq!(state(&canvas).as_deref(), Some("context-lost"));
        assert!(
            log.borrow()
                .iter()
                .any(|message| message.contains("context lost"))
        );

        // Callers settle with an error instead of waiting on a dead renderer.
        for result in [
            host.zoom(1.2),
            host.fit(),
            host.orbit(1.0, 1.0),
            host.view("top"),
        ] {
            assert!(result.unwrap_err().contains("unavailable"));
        }
        let renders = calls(&renderer, "render");
        let zooms = calls(&renderer, "zoom");
        web_sys::window()
            .unwrap()
            .dispatch_event(&web_sys::Event::new("resize").unwrap())
            .unwrap();
        settle().await;
        assert_eq!(
            calls(&renderer, "render"),
            renders,
            "no frame after context loss"
        );
        assert_eq!(
            calls(&renderer, "zoom"),
            zooms,
            "no camera call reaches a lost renderer"
        );

        drop(host);
        assert_eq!(calls(&renderer, "dispose"), 1);
        assert_eq!(calls(&renderer, "free"), 1);
        assert_eq!(state(&canvas).as_deref(), Some("disposed"));
        assert_eq!(spy.live_listeners(), baseline, "all listeners released");
        assert_eq!(spy.live_observers(), 0, "resize observer disconnected");
        // A late loss event after unmount must not resurrect state.
        canvas
            .dispatch_event(&web_sys::Event::new("webglcontextlost").unwrap())
            .unwrap();
        assert_eq!(state(&canvas).as_deref(), Some("disposed"));
        canvas.remove();
    }

    #[wasm_bindgen_test]
    async fn render_submission_error_marks_the_host_failed_and_settles_callers() {
        let renderer = scripted_renderer("render");
        let canvas = make_canvas();
        let (status, log) = status_log();
        let host = attach(&renderer, &canvas, status).expect("attach succeeds before first frame");
        settle().await;
        assert_eq!(state(&canvas).as_deref(), Some("frame-failed"));
        assert!(
            log.borrow()
                .iter()
                .any(|message| message.contains("Renderer submission failed"))
        );
        assert!(host.zoom(1.1).unwrap_err().contains("unavailable"));
        let renders = calls(&renderer, "render");
        settle().await;
        assert_eq!(
            calls(&renderer, "render"),
            renders,
            "failed host does not retry frames"
        );
        host.dispose().unwrap();
        assert_eq!(state(&canvas).as_deref(), Some("disposed"));
        canvas.remove();
    }

    #[wasm_bindgen_test]
    async fn dispose_is_idempotent_and_drop_after_dispose_does_not_free_twice() {
        let spy = ListenerSpy::install();
        let baseline = spy.live_listeners();
        let renderer = scripted_renderer("");
        let canvas = make_canvas();
        let (status, _) = status_log();
        let host = attach(&renderer, &canvas, status).unwrap();
        settle().await;
        host.dispose().unwrap();
        host.dispose().unwrap();
        drop(host);
        assert_eq!(calls(&renderer, "dispose"), 1);
        assert_eq!(calls(&renderer, "free"), 1);
        assert_eq!(spy.live_listeners(), baseline);
        canvas.remove();
    }

    #[wasm_bindgen_test]
    async fn repeated_mount_and_unmount_does_not_accumulate_listeners_or_observers() {
        let spy = ListenerSpy::install();
        let baseline = spy.live_listeners();
        for cycle in 0..8 {
            let renderer = scripted_renderer("");
            let canvas = make_canvas();
            let (status, _) = status_log();
            let host = attach(&renderer, &canvas, status).unwrap();
            if cycle % 2 == 0 {
                settle().await;
            }
            drop(host);
            assert_eq!(calls(&renderer, "dispose"), 1, "cycle {cycle}");
            assert_eq!(calls(&renderer, "free"), 1, "cycle {cycle}");
            assert_eq!(
                spy.live_listeners(),
                baseline,
                "listeners leaked in cycle {cycle}"
            );
            assert_eq!(spy.live_observers(), 0, "observer leaked in cycle {cycle}");
            canvas.remove();
        }
    }

    #[wasm_bindgen_test]
    async fn cancelled_or_failed_initialization_releases_partial_resources() {
        let spy = ListenerSpy::install();
        let baseline = spy.live_listeners();

        // Mount scope cancelled after the lifecycle was installed.
        let renderer = scripted_renderer("");
        let canvas = make_canvas();
        let (status, _) = status_log();
        let checks = Rc::new(Cell::new(0u32));
        let counter = checks.clone();
        let result = RendererHost::attach(
            renderer.clone(),
            canvas.clone(),
            JsValue::from_str("scene"),
            status,
            Rc::new(move || {
                counter.set(counter.get() + 1);
                counter.get() < 2
            }),
        );
        assert!(result.err().unwrap().contains("cancelled"));
        assert_eq!(calls(&renderer, "dispose"), 1);
        assert_eq!(calls(&renderer, "free"), 1);
        assert_eq!(state(&canvas).as_deref(), Some("disposed"));
        assert_eq!(spy.live_listeners(), baseline);
        canvas.remove();

        // Renderer rejects the scene.
        let renderer = scripted_renderer("setScene");
        let canvas = make_canvas();
        let (status, _) = status_log();
        let result = attach(&renderer, &canvas, status);
        assert!(result.err().unwrap().contains("setScene failed"));
        assert_eq!(calls(&renderer, "dispose"), 1);
        assert_eq!(calls(&renderer, "free"), 1);
        assert_eq!(spy.live_listeners(), baseline);
        canvas.remove();
    }
}
