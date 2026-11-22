//! Model delivery through the renderer module shared with viewport mounts.
use boardstudio_web_host::renderer_host::{js_error, renderer_module};
use js_sys::{Float32Array, Function, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

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
    let array = |attribute: &str| Reflect::get(&mesh, &attribute.into()).map_err(js_error);
    let floats = |value: JsValue| {
        value
            .dyn_into::<Float32Array>()
            .map(|array| array.to_vec())
            .map_err(js_error)
    };
    let colors = array("colors")?;
    let colors = (!colors.is_null() && !colors.is_undefined())
        .then(|| floats(colors))
        .transpose()?;
    Ok(crate::model_delivery::MeshArrays {
        positions: floats(array("positions")?)?,
        normals: floats(array("normals")?)?,
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
