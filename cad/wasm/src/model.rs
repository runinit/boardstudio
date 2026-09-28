mod construction;
mod metrics;
pub use construction::{build_assembly, build_case, export_cached_assembly, preview_body};
use metrics::Stage;

use cadrum::{Boolean, DVec3, Edge, Mesh, Solid, Tessellation};
use js_sys::{Float32Array, Object, Reflect, Uint8Array};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

const MAX_STEP_BYTES: usize = 32 * 1024 * 1024;
const MESH_OPTIONS: Tessellation = Tessellation {
    deflection_linear: 0.1,
    deflection_angular: 0.5,
    relative_linear: false,
};

#[derive(Deserialize, Debug)]
struct PreparedCase {
    revision: u64,
    body: CaseBody,
    regions: Vec<PreparedRegion>,
}

#[derive(Deserialize, Debug)]
struct PreparedAssembly {
    revision: u64,
    bodies: Vec<PreparedCase>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CaseBody {
    #[serde(default)]
    features: Vec<CaseFeature>,
    id: String,
    name: String,
    kind: CaseKind,
    thickness: f64,
    #[serde(default)]
    z: Option<f64>,
    #[serde(default)]
    wall_height: Option<f64>,
    #[serde(default)]
    openings: Option<Vec<CaseOpening>>,
    #[serde(default)]
    gasket: Option<Gasket>,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum CaseFeature {
    SupportPrism {
        id: String,
        points: Vec<Vec2>,
        z: f64,
        height: f64,
    },
    RoundSeat {
        id: String,
        at: Vec2,
        z: f64,
        height: f64,
        diameter: f64,
    },
    ConicalSeat {
        id: String,
        at: Vec2,
        z: f64,
        height: f64,
        diameter: f64,
        #[serde(rename = "endDiameter")]
        end_diameter: f64,
    },
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum CaseKind {
    Plate,
    Tray,
    Lid,
}

#[derive(Deserialize, Debug)]
struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Deserialize, Debug)]
struct PreparedRegion {
    outer: Vec<Vec2>,
    #[serde(default)]
    holes: Vec<Vec<Vec2>>,
    #[serde(default)]
    cavities: Vec<Vec<Vec2>>,
    #[serde(default)]
    gaskets: Vec<PreparedGasket>,
    #[serde(default)]
    mounts: Vec<Mount>,
}

#[derive(Deserialize, Debug)]
struct PreparedGasket {
    outer: Vec<Vec2>,
    #[serde(default)]
    holes: Vec<Vec<Vec2>>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct CaseOpening {
    points: Vec<Vec2>,
    z: f64,
    height: f64,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Mount {
    at: Vec2,
    kind: MountKind,
    hole_diameter: f64,
    #[serde(default)]
    boss_diameter: Option<f64>,
    #[serde(default)]
    height: Option<f64>,
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
enum MountKind {
    Hole,
    Boss,
}

#[derive(Deserialize, Debug)]
struct Gasket {
    depth: f64,
}

struct MeshData {
    positions: Vec<f32>,
    normals: Vec<f32>,
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

struct BodyMeshData {
    id: String,
    name: String,
    mesh: MeshData,
}

struct CaseResultData {
    revision: u64,
    step: Vec<u8>,
    mesh: MeshData,
    bodies: Option<Vec<BodyMeshData>>,
}

struct StepModelData {
    mesh: MeshData,
    min: [f64; 3],
    max: [f64; 3],
    #[cfg(test)]
    solid_count: usize,
}

#[wasm_bindgen]
pub fn read_step_model(bytes: Uint8Array) -> Result<JsValue, JsValue> {
    let bytes = {
        let _stage = Stage::new("wasmStepImportCopy");
        bytes.to_vec()
    };
    let result = read_step_model_data(bytes).map_err(js_error)?;
    step_model_to_js(result)
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

fn step_model_to_js(model: StepModelData) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(&value, "mesh", &mesh_to_js(&model.mesh)?)?;
    let bounds = Object::new();
    set(
        &bounds,
        "min",
        &serde_wasm_bindgen::to_value(&model.min).map_err(js_error)?,
    )?;
    set(
        &bounds,
        "max",
        &serde_wasm_bindgen::to_value(&model.max).map_err(js_error)?,
    )?;
    set(&value, "bounds", &bounds)?;
    Ok(value.into())
}

fn export_case(
    solids: Vec<Solid>,
    revision: u64,
    bodies: Option<Vec<BodyMeshData>>,
) -> Result<CaseResultData, String> {
    if solids.is_empty() {
        return Err("OpenCascade returned an empty case solid".into());
    }
    let mut step = Vec::new();
    {
        let _stage = Stage::new("stepSerialization");
        Solid::write_step(&solids, &mut step)
            .map_err(|error| format!("OpenCascade STEP export failed: {error}"))?;
    }
    let mesh = if let Some(bodies) = &bodies {
        let _stage = Stage::new("combinedMeshCopy");
        metrics::count(
            "copiedMeshBytes",
            bodies
                .iter()
                .map(|body| (body.mesh.positions.len() + body.mesh.normals.len()) * 4)
                .sum(),
        );
        MeshData {
            positions: bodies
                .iter()
                .flat_map(|body| body.mesh.positions.iter().copied())
                .collect(),
            normals: bodies
                .iter()
                .flat_map(|body| body.mesh.normals.iter().copied())
                .collect(),
        }
    } else {
        mesh_data(&solids)?
    };
    Ok(CaseResultData {
        revision,
        step,
        mesh,
        bodies,
    })
}

fn read_step_model_data(bytes: Vec<u8>) -> Result<StepModelData, String> {
    if bytes.is_empty() || bytes.len() > MAX_STEP_BYTES {
        return Err("STEP import failed: invalid file size".into());
    }
    let mut reader = std::io::Cursor::new(bytes);
    let solids = {
        let _stage = Stage::new("stepImport");
        Solid::read_step(&mut reader).map_err(|error| format!("STEP import failed: {error}"))?
    };
    if solids.is_empty() {
        return Err("STEP import failed: empty shape".into());
    }

    let mut min = DVec3::splat(f64::INFINITY);
    let mut max = DVec3::splat(f64::NEG_INFINITY);
    for solid in &solids {
        let [solid_min, solid_max] = solid.bounding_box();
        min = min.min(solid_min);
        max = max.max(solid_max);
    }
    if !min.is_finite() || !max.is_finite() || min.cmpgt(max).any() {
        return Err("STEP import failed: invalid bounds".into());
    }

    Ok(StepModelData {
        mesh: mesh_data(&solids).map_err(|error| format!("STEP import failed: {error}"))?,
        min: min.to_array(),
        max: max.to_array(),
        #[cfg(test)]
        solid_count: solids.len(),
    })
}

fn mesh_data(solids: &[Solid]) -> Result<MeshData, String> {
    let mesh = {
        let _stage = Stage::new("tessellation");
        metrics::count("tessellations", 1);
        Solid::mesh(solids, MESH_OPTIONS).map_err(cadrum_error)?
    };
    mesh_to_data(&mesh)
}

fn mesh_to_data(mesh: &Mesh) -> Result<MeshData, String> {
    let _stage = Stage::new("meshExpansion");
    metrics::count("tessellatedTriangles", mesh.indices.len() / 3);
    if mesh.indices.is_empty()
        || mesh.indices.len() % 3 != 0
        || mesh.normals.len() != mesh.vertices.len()
    {
        return Err("OpenCascade returned an invalid case mesh".into());
    }
    let mut positions = Vec::with_capacity(mesh.indices.len() * 3);
    let mut normals = Vec::with_capacity(mesh.indices.len() * 3);
    for &index in &mesh.indices {
        let position = mesh
            .vertices
            .get(index)
            .ok_or_else(|| "Cadrum mesh index is out of bounds".to_string())?;
        let normal = mesh
            .normals
            .get(index)
            .ok_or_else(|| "Cadrum mesh normal index is out of bounds".to_string())?;
        for value in position.to_array() {
            if !value.is_finite() || (value as f32).is_infinite() {
                return Err("OpenCascade returned a non-finite case mesh".into());
            }
            positions.push(value as f32);
        }
        for value in normal.to_array() {
            if !value.is_finite() || (value as f32).is_infinite() {
                return Err("OpenCascade returned a non-finite case mesh".into());
            }
            normals.push(value as f32);
        }
    }
    if positions.is_empty() || positions.len() != normals.len() {
        return Err("OpenCascade returned an empty case mesh".into());
    }
    Ok(MeshData { positions, normals })
}

fn cadrum_error(error: impl ToString) -> String {
    format!("OpenCascade CAD operation failed: {}", error.to_string())
}
