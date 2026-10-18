use super::*;
use boardstudio_contracts::{PreparedCaseAssemblyIR, PreparedCaseIR};

const MAX_REVISION: u64 = 9_007_199_254_740_991;

#[wasm_bindgen]
pub fn build_case(input: JsValue) -> Result<JsValue, JsValue> {
    let ir: PreparedCaseIR = deserialize(input)?;
    case_result_to_js(build_case_data(ir).map_err(js_error)?)
}

fn build_case_data(ir: PreparedCaseIR) -> Result<CaseResultData, String> {
    let entry = cache::body(&ir, "", true, &mut |_| Ok(()))?;
    let _stage = Stage::new("stepSerialization");
    let step = boardstudio_cad::export_step_bodies(&[boardstudio_cad::StepBody {
        id: &ir.body.id,
        name: &ir.body.name,
        model: &entry.model,
    }])?;
    let mesh = entry
        .mesh
        .as_ref()
        .ok_or("Exact case has no mesh")?
        .as_ref()
        .clone();
    Ok(CaseResultData {
        revision: ir.revision,
        step,
        mesh,
        bodies: None,
    })
}

#[path = "generated/cache.rs"]
mod cache;

#[wasm_bindgen]
pub fn preview_body(
    input: JsValue,
    key: String,
    progress: js_sys::Function,
) -> Result<JsValue, JsValue> {
    let ir: PreparedCaseIR = deserialize(input)?;
    let mesh = preview_body_data(&ir, &key, |phase| {
        progress
            .call1(&JsValue::NULL, &JsValue::from_str(phase))
            .map(|_| ())
            .map_err(|_| "Progress callback failed".to_string())
    })
    .map_err(js_error)?;
    mesh_to_js(&mesh)
}

fn preview_body_data(
    ir: &PreparedCaseIR,
    key: &str,
    mut progress: impl FnMut(&'static str) -> Result<(), String>,
) -> Result<std::rc::Rc<MeshData>, String> {
    let entry = cache::body(ir, key, true, &mut progress)?;
    entry
        .mesh
        .as_ref()
        .cloned()
        .ok_or_else(|| "Exact preview has no mesh".into())
}

#[wasm_bindgen]
pub fn build_assembly(input: JsValue) -> Result<JsValue, JsValue> {
    let ir: PreparedCaseAssemblyIR = deserialize(input)?;
    case_result_to_js(build_assembly_data(ir).map_err(js_error)?)
}

fn build_assembly_data(ir: PreparedCaseAssemblyIR) -> Result<CaseResultData, String> {
    let keys = vec![String::new(); ir.bodies.len()];
    full_assembly_data(&ir, &keys)
}

fn assembly_entries(
    ir: &PreparedCaseAssemblyIR,
    keys: &[String],
    want_mesh: bool,
) -> Result<Vec<std::rc::Rc<cache::CachedBody>>, String> {
    if ir.bodies.is_empty() {
        return Err("Case assembly requires at least one body".into());
    }
    if ir.revision > MAX_REVISION {
        return Err("Invalid CAD revision".into());
    }
    if keys.len() != ir.bodies.len() {
        return Err("Invalid CAD cache keys".into());
    }
    if ir.bodies.iter().any(|body| body.revision != ir.revision) {
        return Err("Case assembly contains a stale body revision".into());
    }
    ir.bodies
        .iter()
        .zip(keys)
        .map(|(body, key)| cache::body(body, key, want_mesh, &mut |_| Ok(())))
        .collect()
}

fn assembly_step(
    ir: &PreparedCaseAssemblyIR,
    entries: &[std::rc::Rc<cache::CachedBody>],
) -> Result<Vec<u8>, String> {
    let bodies = ir
        .bodies
        .iter()
        .zip(entries)
        .map(|(body, entry)| boardstudio_cad::StepBody {
            id: &body.body.id,
            name: &body.body.name,
            model: &entry.model,
        })
        .collect::<Vec<_>>();
    let _stage = Stage::new("stepSerialization");
    boardstudio_cad::export_step_bodies(&bodies)
}

fn full_assembly_data(
    ir: &PreparedCaseAssemblyIR,
    keys: &[String],
) -> Result<CaseResultData, String> {
    let entries = assembly_entries(ir, keys, true)?;
    let step = assembly_step(ir, &entries)?;
    let bodies = ir
        .bodies
        .iter()
        .zip(&entries)
        .map(|(body, entry)| {
            let mesh = entry
                .mesh
                .as_ref()
                .ok_or("Exact assembly body has no mesh")?
                .as_ref()
                .clone();
            Ok(BodyMeshData {
                id: body.body.id.clone(),
                name: body.body.name.clone(),
                mesh,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    let _stage = Stage::new("combinedMeshCopy");
    let mut mesh = MeshData {
        positions: Vec::new(),
        normals: Vec::new(),
    };
    for body in &bodies {
        metrics::count(
            "copiedMeshBytes",
            (body.mesh.positions.len() + body.mesh.normals.len()) * 4,
        );
        mesh.positions.extend_from_slice(&body.mesh.positions);
        mesh.normals.extend_from_slice(&body.mesh.normals);
    }
    Ok(CaseResultData {
        revision: ir.revision,
        step,
        mesh,
        bodies: Some(bodies),
    })
}

fn step_only_data(ir: &PreparedCaseAssemblyIR, keys: &[String]) -> Result<(u64, Vec<u8>), String> {
    let entries = assembly_entries(ir, keys, false)?;
    Ok((ir.revision, assembly_step(ir, &entries)?))
}

#[wasm_bindgen]
pub fn export_cached_assembly(input: JsValue, keys: JsValue) -> Result<JsValue, JsValue> {
    let step_only = Reflect::get(&input, &JsValue::from_str("stepOnly"))?
        .as_bool()
        .unwrap_or(false);
    let ir: PreparedCaseAssemblyIR = deserialize(input)?;
    let keys: Vec<String> = deserialize(keys)?;
    if !step_only {
        return case_result_to_js(full_assembly_data(&ir, &keys).map_err(js_error)?);
    }
    let (revision, step) = step_only_data(&ir, &keys).map_err(js_error)?;
    let result = Object::new();
    set(&result, "revision", &JsValue::from_f64(revision as f64))?;
    let _stage = Stage::new("wasmStepCopy");
    set(&result, "step", &Uint8Array::from(step.as_slice()))?;
    Ok(result.into())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct GeneratedBodyAnalysisOptions {
    source_sha256: String,
    #[serde(default)]
    target_deflection_mm: Option<f64>,
}

pub(crate) fn build_generated_body_analysis_asset(
    ir: &PreparedCaseIR,
    key: &str,
    source_sha256: String,
    target_deflection_mm: Option<f64>,
) -> Result<super::vendor_step::ReadStepAsset, String> {
    let entry = cache::body(ir, key, false, &mut |_| Ok(()))?;
    let analysis =
        boardstudio_cad::mesh_profile_for_analysis(&entry.model, target_deflection_mm)?;
    super::vendor_step::ReadStepAsset::from_generated_analysis(
        source_sha256,
        target_deflection_mm,
        analysis,
    )
}

#[wasm_bindgen]
pub fn build_generated_body_analysis(
    input: JsValue,
    key: String,
    options: JsValue,
) -> Result<JsValue, JsValue> {
    let ir: PreparedCaseIR = deserialize(input)?;
    let options: GeneratedBodyAnalysisOptions = serde_wasm_bindgen::from_value(options)
        .map_err(|error| js_error(format!("Invalid generated analysis options: {error}")))?;
    build_generated_body_analysis_asset(
        &ir,
        &key,
        options.source_sha256,
        options.target_deflection_mm,
    )
    .map_err(js_error)?
    .to_js_value()
}

#[cfg(test)]
#[path = "generated/tests.rs"]
mod tests;
