fn call(receiver: &JsValue, name: &str, args: &[JsValue]) -> Result<JsValue, JsValue> {
    Reflect::get(receiver, &name.into())?
        .dyn_into::<Function>()?
        .apply(receiver, &args.iter().collect::<Array>())
}

use boardstudio_web_host::renderer_host::RendererPageHost;
use gloo_timers::future::TimeoutFuture;
use js_sys::{Array, Function, Object, Reflect};
use std::rc::Rc;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlCanvasElement};

wasm_bindgen_test_configure!(run_in_browser);

struct Fixture {
    canvas: HtmlCanvasElement,
    module: JsValue,
    log: Array,
}

impl Fixture {
    fn new(fail: &str) -> Self {
        let document = web_sys::window().unwrap().document().unwrap();
        let canvas: HtmlCanvasElement = document.create_element("canvas").unwrap().unchecked_into();
        canvas
            .style()
            .set_css_text("width:80px;height:60px;display:block");
        document.body().unwrap().append_child(&canvas).unwrap();
        let log = Array::new();
        let module = Function::new_with_args(
            "log, fail",
            "
            const methods = ['setScene', 'setState', 'setHandles', 'resize', 'fit',
                'render', 'dispose', 'free', 'orbit', 'zoom', 'view', 'pick',
                'pointOnPlane', 'focusObjects'];
            const control = { result: true, next: 0 };
            return {
                control,
                Renderer: function () {
                    const id = ++control.next;
                    for (const name of methods) {
                        this[name] = (...args) => {
                            log.push([id, name, ...args]);
                            if (name === fail) throw new Error(name + ' failed');
                            if (name === 'setScene') return control.result;
                            if (name === 'pointOnPlane') return new Float32Array([1, 2, 3]);
                            if (name === 'pick') return 'body';
                            if (name === 'focusObjects') return true;
                        };
                    }
                }
            };
        ",
        )
        .call2(&JsValue::NULL, &log, &fail.into())
        .unwrap();
        Self {
            canvas,
            module,
            log,
        }
    }

    fn mount(&self) -> Result<RendererPageHost, String> {
        RendererPageHost::mount(
            &self.module,
            self.canvas.clone(),
            Object::new().into(),
            1,
            Rc::new(|_| {}),
        )
    }

    fn calls(&self, instance: u32, method: &str) -> Vec<Array> {
        self.log
            .iter()
            .map(|entry| entry.unchecked_into::<Array>())
            .filter(|entry| {
                entry.get(0).as_f64() == Some(f64::from(instance))
                    && entry.get(1).as_string().as_deref() == Some(method)
            })
            .collect()
    }

    fn event(&self, name: &str) -> Event {
        let init = web_sys::EventInit::new();
        init.set_cancelable(true);
        let event = Event::new_with_event_init_dict(name, &init).unwrap();
        self.canvas.dispatch_event(&event).unwrap();
        event
    }

    fn state(&self) -> Option<String> {
        self.canvas.get_attribute("data-renderer-state")
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.canvas.remove();
    }
}

async fn settle() {
    TimeoutFuture::new(100).await;
}

fn mounted(fail: &str) -> (Fixture, RendererPageHost) {
    let fixture = Fixture::new(fail);
    let host = fixture.mount().unwrap();
    (fixture, host)
}

#[wasm_bindgen_test]
async fn scene_sequences_are_monotonic_and_only_accepted_updates_draw() {
    let (f, host) = mounted("");
    settle().await;
    let renders = f.calls(1, "render").len();
    let scene: JsValue = Object::new().into();
    let control = Reflect::get(&f.module, &"control".into()).unwrap();
    for (sequence, result) in [(2, false.into()), (3, JsValue::UNDEFINED)] {
        Reflect::set(&control, &"result".into(), &result).unwrap();
        assert_ne!(host.submit_scene(scene.clone(), sequence), Ok(true));
        assert!(host.submit_scene(scene.clone(), sequence).is_err());
    }
    settle().await;
    assert_eq!(f.calls(1, "render").len(), renders);
    Reflect::set(&control, &"result".into(), &true.into()).unwrap();
    assert_eq!(host.submit_scene(scene, 4), Ok(true));
    settle().await;
    assert_eq!(f.calls(1, "render").len(), renders + 1);
    assert_eq!(f.calls(1, "setScene").len(), 4);
}

#[wasm_bindgen_test]
async fn observer_resizes_backing_buffer_and_teardown_stops_draws() {
    let (f, host) = mounted("");
    f.canvas.style().set_property("width", "125px").unwrap();
    settle().await;
    let dpr = web_sys::window().unwrap().device_pixel_ratio().max(1.0);
    assert_eq!(f.canvas.width(), (125.0 * dpr).round() as u32);
    let resize = f.calls(1, "resize");
    assert_eq!(
        resize.last().unwrap().get(2).as_f64(),
        Some(f64::from(f.canvas.width()))
    );
    assert!(host.pick_at_client(10.0, 10.0).unwrap().is_some());
    host.dispose().unwrap();
    host.dispose().unwrap();
    drop(host);
    let log = f.log.length();
    f.canvas.style().set_property("width", "140px").unwrap();
    f.event("webglcontextrestored");
    settle().await;
    assert_eq!(f.log.length(), log);
    assert_eq!(f.calls(1, "free").len(), 1);
    assert_eq!(f.state().as_deref(), Some("disposed"));
}

#[wasm_bindgen_test]
fn dpr_change_replaces_its_listener_and_resizes_the_canvas() {
    let (f, host) = mounted("");
    let key = JsValue::from_str("devicePixelRatio");
    let window = web_sys::window().unwrap();
    let window = window.unchecked_ref::<Object>();
    let previous = Object::get_own_property_descriptor(window, &key);
    let descriptor = Object::new();
    Reflect::set(&descriptor, &"configurable".into(), &true.into()).unwrap();
    Reflect::set(&descriptor, &"value".into(), &2.0.into()).unwrap();
    Object::define_property(window, &key, &descriptor);

    let previous_query = host.dpr_media_query().unwrap();
    let dispatched = previous_query.dispatch_event(&Event::new("change").unwrap());
    let changed_width = f.canvas.width();
    let current_query = host.dpr_media_query().unwrap();

    if previous.is_undefined() {
        Reflect::delete_property(window, &key).unwrap();
    } else {
        Object::define_property(window, &key, previous.unchecked_ref());
    }
    dispatched.unwrap();
    assert_eq!(changed_width, 160);
    assert!(!Object::is(previous_query.as_ref(), current_query.as_ref()));
    previous_query
        .dispatch_event(&Event::new("change").unwrap())
        .unwrap();
    assert!(Object::is(
        current_query.as_ref(),
        host.dpr_media_query().as_ref().unwrap().as_ref()
    ));
}

#[wasm_bindgen_test]
async fn lost_context_blocks_input_and_restoration_replays_latest_packets() {
    let (f, host) = mounted("");
    // Use the real WebGL context if available. The lifecycle still receives native DOM events.
    let gl = f.canvas.get_context("webgl2").unwrap();
    assert!(host.submit_scene(Object::new().into(), 5).unwrap());
    host.set_display_state(Object::new().into()).unwrap();
    host.set_handles(Array::new().into()).unwrap();
    assert!(f.event("webglcontextlost").default_prevented());
    assert_eq!(f.state().as_deref(), Some("context-lost"));
    assert!(host.zoom(1.1).is_err());
    assert!(host.pick_at_client(5.0, 5.0).is_err());
    settle().await;
    assert!(f.calls(1, "render").is_empty());
    f.event("webglcontextrestored");
    settle().await;
    assert_eq!(f.state().as_deref(), Some("active"));
    let scene = f.calls(2, "setScene")[0].get(2);
    assert_eq!(
        Reflect::get(&scene, &"revision".into()).unwrap().as_f64(),
        Some(5.0)
    );
    for method in ["setState", "setHandles", "fit", "render"] {
        assert_eq!(f.calls(2, method).len(), 1, "{method}");
    }
    assert_eq!(f.calls(1, "free").len(), 1);
    host.dispose().unwrap();
    if let Some(gl) = gl {
        let extension = call(gl.as_ref(), "getExtension", &["WEBGL_lose_context".into()]).unwrap();
        if !extension.is_null() {
            settle().await;
            assert_eq!(
                call(gl.as_ref(), "isContextLost", &[]).unwrap().as_bool(),
                Some(true)
            );
        }
    }
}

#[wasm_bindgen_test]
async fn initialization_and_frame_failure_stop_and_release_the_renderer() {
    let failed = Fixture::new("setScene");
    assert!(failed.mount().err().unwrap().contains("setScene failed"));
    assert_eq!(failed.calls(1, "free").len(), 1);
    assert_eq!(failed.state().as_deref(), Some("disposed"));

    let (frame, host) = mounted("render");
    settle().await;
    assert_eq!(frame.state().as_deref(), Some("frame-failed"));
    assert!(host.fit().is_err());
    frame.canvas.style().set_property("width", "72px").unwrap();
    settle().await;
    assert_eq!(frame.calls(1, "render").len(), 1);
    drop(host);
    assert_eq!(frame.calls(1, "free").len(), 1);
}
