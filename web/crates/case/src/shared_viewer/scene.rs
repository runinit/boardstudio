use super::*;
use crate::viewport_geometry::{
    board_packet, keycap_style, layout_module_bodies, reference_packet, transformed_module_mesh,
    transformed_module_points,
};

pub(super) fn project_source(
    source: &ViewerSource,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    let bodies = Array::new();
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    let mut stack = &[][..];
    let mut module_bodies = Vec::new();
    let mut reference = None;
    let mut selected_layer = "";
    let (thickness, contours, pcb) = match source {
        ViewerSource::Cad(scene) => {
            let document = captured_case_document(&scene.snapshot, &scene.scope)
                .map_err(|error| format!("Case scene projection failed: {error:?}"))?;
            let board = document
                .boards
                .iter()
                .find(|board| board.id == scene.scope.board_id)
                .ok_or("Case board is unavailable in the selected scene")?;
            if let Some(assembly) = scene.mechanical.as_ref() {
                stack = &assembly.stack;
            }
            for body in &scene.result.bodies {
                bodies.push(&body_value(body)?);
                if !layers.iter().any(|(id, _)| id == &body.id) {
                    layers.push((body.id.clone(), body.name.clone()));
                }
            }
            if let Some(preview) = preview {
                reference = preview.board_reference.as_ref();
                (
                    preview.preview.thickness,
                    &preview.contours,
                    Some(&preview.preview),
                )
            } else {
                (board.thickness, &scene.contours, None)
            }
        }
        ViewerSource::Native(preview) => {
            reference = preview.board_reference.as_ref();
            (
                preview.preview.thickness,
                &preview.contours,
                Some(&preview.preview),
            )
        }
        ViewerSource::Layout(preview) => {
            reference = preview
                .board_reference
                .as_ref()
                .filter(|reference| reference.enabled);
            selected_layer = "pcb";
            module_bodies = layout_module_bodies(
                &preview.module_scenes,
                &preview.document,
                &identity.scope.board_id,
                preview.preview.thickness,
            );
            layers.extend(module_bodies.iter().map(body_layer));
            for body in layout_module_model_bodies(model_rows, &preview.document) {
                layers.push(body_layer(&body));
                bodies.push(&js_sys::JSON::parse(&body.to_string()).map_err(js_error)?);
            }
            (
                preview.preview.thickness,
                &preview.contours,
                Some(&preview.preview),
            )
        }
        ViewerSource::Parts(preview) => {
            let board = preview
                .sample_document
                .boards
                .iter()
                .find(|board| board.id == "sample-board")
                .ok_or("Parts sample board is unavailable")?;
            (board.thickness, &preview.contours, Some(&preview.preview))
        }
    };
    if !matches!(source, ViewerSource::Parts(_)) {
        append_keycap_bodies(
            &bodies,
            &mut layers,
            keycaps_preview,
            &identity,
            identity.revision,
        )?;
    }
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": selected_layer,
        "hidden": [],
        "board": board_packet(identity.renderer_sequence, thickness, contours, pcb),
        "reference": reference.map(reference_packet),
        "mechanicalStack": stack,
        "moduleBodies": module_bodies
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows)?;
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
}

fn body_layer(body: &serde_json::Value) -> (String, String) {
    let id = body["id"].as_str().unwrap_or_default();
    let name = body["name"].as_str().unwrap_or_default();
    (id.to_owned(), name.to_owned())
}

fn mesh_packet(
    positions: &[f32],
    normals: &[f32],
    colors: Option<&[f32]>,
) -> Result<Object, String> {
    let mesh = Object::new();
    for (field, values) in [
        ("positions", Some(positions)),
        ("normals", Some(normals)),
        ("colors", colors),
    ] {
        if let Some(values) = values {
            Reflect::set(&mesh, &field.into(), &Float32Array::from(values)).map_err(js_error)?;
        }
    }
    Ok(mesh)
}

fn body_value(body: &boardstudio_web_host::cad_jobs::CadBodyMesh) -> Result<JsValue, String> {
    let value = Object::new();
    let mesh = mesh_packet(&body.positions, &body.normals, None)?;
    Reflect::set(&value, &"id".into(), &body.id.as_str().into()).map_err(js_error)?;
    Reflect::set(&value, &"name".into(), &body.name.as_str().into()).map_err(js_error)?;
    Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
    Ok(value.into())
}

fn append_keycap_bodies(
    bodies: &Array,
    layers: &mut Vec<(String, String)>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
    identity: &ViewerIdentity,
    revision: u64,
) -> Result<(), String> {
    if let Some(keycaps) = keycaps_preview {
        if keycaps.scope != identity.scope
            || keycaps.token != identity.snapshot_token
            || keycaps.revision != revision
        {
            return Err("Keycap preview does not match the current viewer source".into());
        }
        for body in &keycaps.bodies {
            let (color, label) = keycap_style(&body.id, &keycaps.specs)
                .ok_or("Keycap preview body does not match its accepted specification")?;
            let value = body_value(body)?;
            Reflect::set(&value, &"color".into(), &color.into()).map_err(js_error)?;
            bodies.push(&value);
            layers.push((body.id.clone(), label));
        }
    }
    Ok(())
}

fn layout_module_model_bodies(
    rows: Option<&ModelDeliveryRows>,
    document: &boardstudio_model::model::ProjectDoc,
) -> Vec<serde_json::Value> {
    rows.into_iter()
        .flat_map(|rows| rows.delivered.iter())
        .filter_map(|model| {
            let matrix = model.matrix?;
            let (positions, normals) = transformed_module_mesh(
                model.mesh.positions.as_ref(),
                model.mesh.normals.as_ref(),
                &matrix,
            );
            let module_id = model.id.strip_prefix("module-model/")?.rsplit_once('/')?.0;
            let name = document
                .modules
                .iter()
                .find(|module| module.id == module_id)
                .and_then(|module| {
                    document
                        .module_definitions
                        .iter()
                        .find(|definition| definition.id == module.definition_id)
                })
                .map(|definition| format!("{} · 3D model", definition.name))
                .unwrap_or_else(|| "Mounted module · 3D model".to_owned());
            let lines = model
                .mesh
                .vendor_step
                .as_ref()
                .filter(|step| !step.line_render_positions.is_empty())
                .map(|step| transformed_module_points(&step.line_render_positions, &matrix));
            Some(serde_json::json!({
                "id": model.id,
                "name": name,
                "color": "#8f9b9e",
                "mesh": {
                    "positions": positions,
                    "normals": normals,
                    "colors": model.mesh.colors.as_ref().map(|colors| colors.as_ref()),
                },
                "lines": lines,
            }))
        })
        .collect()
}

fn loaded_model_inputs(rows: Option<&ModelDeliveryRows>) -> Result<Array, String> {
    let loaded = Array::new();
    for model in rows
        .into_iter()
        .flat_map(|rows| &rows.delivered)
        .filter(|model| model.matrix.is_none())
    {
        let value = Object::new();
        let mesh = mesh_packet(
            &model.mesh.positions,
            &model.mesh.normals,
            model.mesh.colors.as_deref(),
        )?;
        Reflect::set(&value, &"id".into(), &model.id.as_str().into()).map_err(js_error)?;
        Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
        if let Some(step) = &model.mesh.vendor_step
            && !step.line_render_positions.is_empty()
        {
            let lines = Float32Array::from(step.line_render_positions.as_slice());
            Reflect::set(&value, &"lines".into(), &lines).map_err(js_error)?;
        }
        loaded.push(&value);
    }
    Ok(loaded)
}
