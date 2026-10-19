//! Page renderer lifetime and model decoding through the packaged renderer module.
pub use boardstudio_web_host::renderer_host::RendererPageHost;
use js_sys::{Float32Array, Function, Promise, Reflect, Uint8Array};
use std::cell::RefCell;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

thread_local! {
    /// Cache the imported and initialized renderer module namespace. Model
    /// decoding and Renderer construction therefore use the same packaged
    /// WASM module and singleton decoder implementation.
    static RENDERER_MODULE: RefCell<Option<Promise>> = const { RefCell::new(None) };
}

async fn renderer_module() -> Result<JsValue, String> {
    let promise = if let Some(promise) = RENDERER_MODULE.with(|module| module.borrow().clone()) {
        promise
    } else {
        let module_url =
            crate::runtime::resource_url("assets/renderer/boardstudio_renderer_wasm.js")?;
        let import_and_initialize = Function::new_with_args(
            "url",
            "return import(url).then(async module => { await module.default(); return module; })",
        );
        let promise = import_and_initialize
            .call1(&JsValue::NULL, &JsValue::from_str(&module_url))
            .map_err(js_error)?
            .dyn_into::<Promise>()
            .map_err(js_error)?;
        RENDERER_MODULE.with(|module| *module.borrow_mut() = Some(promise.clone()));
        promise
    };
    JsFuture::from(promise).await.map_err(js_error)
}

async fn decode_model(
    name: &str,
    bytes: &[u8],
) -> Result<crate::model_delivery::MeshArrays, String> {
    let module = renderer_module().await?;
    let decoder = Reflect::get(&module, &JsValue::from_str(name))
        .map_err(js_error)?
        .dyn_into::<Function>()
        .map_err(js_error)?;
    let mesh = decoder
        .call1(&module, &Uint8Array::from(bytes))
        .map_err(js_error)?;
    let positions = Reflect::get(&mesh, &JsValue::from_str("positions"))
        .map_err(js_error)?
        .dyn_into::<Float32Array>()
        .map_err(js_error)?
        .to_vec();
    let normals = Reflect::get(&mesh, &JsValue::from_str("normals"))
        .map_err(js_error)?
        .dyn_into::<Float32Array>()
        .map_err(js_error)?
        .to_vec();
    let colors = Reflect::get(&mesh, &JsValue::from_str("colors")).map_err(js_error)?;
    let colors = if colors.is_null() || colors.is_undefined() {
        None
    } else {
        Some(
            colors
                .dyn_into::<Float32Array>()
                .map_err(js_error)?
                .to_vec(),
        )
    };
    Ok(crate::model_delivery::MeshArrays {
        positions,
        normals,
        colors,
        step_bounds: None,
        vendor_step: None,
    })
}

pub async fn decode_stl(bytes: Vec<u8>) -> Result<crate::model_delivery::MeshArrays, String> {
    decode_model("decodeStl", &bytes).await
}

pub async fn decode_wrl(bytes: Vec<u8>) -> Result<crate::model_delivery::MeshArrays, String> {
    decode_model("decodeWrl", &bytes).await
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
