mod collision;
mod generated;
mod keycaps;
mod metrics;
#[cfg(all(test, not(target_arch = "wasm32")))]
mod source_validation_tests;
mod vendor_step;

pub use collision::query_collision_mesh_pair;
pub use generated::{
    analyze_generated_body, build_assembly, build_case, export_cached_assembly, preview_body,
};
pub use keycaps::build_keycaps;
pub use vendor_step::read_step_model;

use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use metrics::Stage;
use serde::Deserialize;
use wasm_bindgen::prelude::*;

pub(crate) struct MeshData {
    pub(crate) positions: Vec<f32>,
    pub(crate) normals: Vec<f32>,
}

impl Clone for MeshData {
    fn clone(&self) -> Self {
        let _stage = Stage::new("cacheMeshCopy");
        metrics::count(
            "copiedMeshBytes",
            (self.positions.len() + self.normals.len()) * 4,
        );
        Self {
            positions: self.positions.clone(),
            normals: self.normals.clone(),
        }
    }
}

pub(crate) struct BodyMeshData {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) mesh: MeshData,
}

pub(crate) struct CaseResultData {
    pub(crate) revision: u64,
    pub(crate) step: Vec<u8>,
    pub(crate) mesh: MeshData,
    pub(crate) bodies: Option<Vec<BodyMeshData>>,
}

fn deserialize<T: for<'de> Deserialize<'de>>(input: JsValue) -> Result<T, JsValue> {
    serde_wasm_bindgen::from_value(input)
        .map_err(|error| js_error(format!("Invalid CAD input: {error}")))
}

fn js_error(message: impl ToString) -> JsValue {
    JsValue::from_str(&message.to_string())
}

fn set(target: &Object, key: &str, value: &JsValue) -> Result<(), JsValue> {
    Reflect::set(target, &JsValue::from_str(key), value)?;
    Ok(())
}

fn mesh_to_js(mesh: &MeshData) -> Result<JsValue, JsValue> {
    let _stage = Stage::new("wasmMeshCopy");
    metrics::count(
        "copiedMeshBytes",
        (mesh.positions.len() + mesh.normals.len()) * 4,
    );
    let value = Object::new();
    set(
        &value,
        "positions",
        &Float32Array::from(mesh.positions.as_slice()),
    )?;
    set(
        &value,
        "normals",
        &Float32Array::from(mesh.normals.as_slice()),
    )?;
    Ok(value.into())
}

fn case_result_to_js(result: CaseResultData) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(
        &value,
        "revision",
        &JsValue::from_f64(result.revision as f64),
    )?;
    {
        let _stage = Stage::new("wasmStepCopy");
        set(&value, "step", &Uint8Array::from(result.step.as_slice()))?;
    }
    set(&value, "mesh", &mesh_to_js(&result.mesh)?)?;

    if let Some(body_data) = result.bodies {
        let bodies = js_sys::Array::new();
        for body in body_data {
            let _stage = Stage::new("wasmMeshCopy");
            metrics::count(
                "copiedMeshBytes",
                (body.mesh.positions.len() + body.mesh.normals.len()) * 4,
            );
            let value = Object::new();
            set(&value, "id", &JsValue::from_str(&body.id))?;
            set(&value, "name", &JsValue::from_str(&body.name))?;
            set(
                &value,
                "positions",
                &Float32Array::from(body.mesh.positions.as_slice()),
            )?;
            set(
                &value,
                "normals",
                &Float32Array::from(body.mesh.normals.as_slice()),
            )?;
            bodies.push(&value);
        }
        set(&value, "bodies", &bodies)?;
    }

    Ok(value.into())
}
