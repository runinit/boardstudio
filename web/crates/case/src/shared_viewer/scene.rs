use super::*;
use crate::viewport_geometry::{
    board_packet, keycap_style, layout_module_bodies, reference_packet, transformed_module_mesh,
    transformed_module_points,
};

pub(super) fn project_source(
    source: &ViewportSource,
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
    let mut layout_document = None;
    let mut reference = None;
    let mut selected_layer = "";
    let (thickness, contours, pcb) = match source {
        ViewportSource::Case(case) => {
            if let Some(scene) = &case.scene {
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
                    bodies.push(&body_value(body, None)?);
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
            } else if let Some(preview) = &case.preview {
                reference = preview.board_reference.as_ref();
                (
                    preview.preview.thickness,
                    &preview.contours,
                    Some(&preview.preview),
                )
            } else {
                return Err("3D preview source is unavailable.".into());
            }
        }
        ViewportSource::Layout(preview) => {
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
            layout_document = Some(preview.document.as_ref());
            (
                preview.preview.thickness,
                &preview.contours,
                Some(&preview.preview),
            )
        }
        ViewportSource::Parts(preview, _) => {
            let board = preview
                .sample_document
                .boards
                .iter()
                .find(|board| board.id == "sample-board")
                .ok_or("Parts sample board is unavailable")?;
            (board.thickness, &preview.contours, Some(&preview.preview))
        }
    };
    let models = model_packets(&bodies, &mut layers, model_rows, layout_document)?;
    if !matches!(source, ViewportSource::Parts(..)) {
        append_keycap_bodies(&bodies, &mut layers, keycaps_preview, &identity)?;
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
    set_js_fields(
        &input,
        [("bodies", bodies.into()), ("models", models.into())],
    )?;
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

fn set_js_fields(
    object: &JsValue,
    fields: impl IntoIterator<Item = (&'static str, JsValue)>,
) -> Result<(), String> {
    for (key, value) in fields {
        Reflect::set(object, &key.into(), &value).map_err(js_error)?;
    }
    Ok(())
}

pub(super) fn js_obj(
    fields: impl IntoIterator<Item = (&'static str, JsValue)>,
) -> Result<JsValue, String> {
    let object = Object::new();
    set_js_fields(&object, fields)?;
    Ok(object.into())
}

fn mesh_packet(
    positions: &[f32],
    normals: &[f32],
    colors: Option<&[f32]>,
) -> Result<JsValue, String> {
    js_obj(
        [
            ("positions", Some(positions)),
            ("normals", Some(normals)),
            ("colors", colors),
        ]
        .into_iter()
        .filter_map(|(key, values)| Some((key, Float32Array::from(values?).into()))),
    )
}

fn body_value(
    body: &boardstudio_web_host::cad_jobs::CadBodyMesh,
    color: Option<&str>,
) -> Result<JsValue, String> {
    js_obj(
        [
            ("id", body.id.as_str().into()),
            ("name", body.name.as_str().into()),
            ("mesh", mesh_packet(&body.positions, &body.normals, None)?),
        ]
        .into_iter()
        .chain(color.map(|color| ("color", color.into()))),
    )
}

fn append_keycap_bodies(
    bodies: &Array,
    layers: &mut Vec<(String, String)>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
    identity: &ViewerIdentity,
) -> Result<(), String> {
    if let Some(keycaps) = keycaps_preview {
        if keycaps.scope != identity.scope
            || keycaps.token != identity.snapshot_token
            || keycaps.revision != identity.revision
        {
            return Err("Keycap preview does not match the current viewer source".into());
        }
        for body in &keycaps.bodies {
            let (color, label) = keycap_style(&body.id, &keycaps.specs)
                .ok_or("Keycap preview body does not match its accepted specification")?;
            bodies.push(&body_value(body, Some(color))?);
            layers.push((body.id.clone(), label));
        }
    }
    Ok(())
}

fn model_packets(
    bodies: &Array,
    layers: &mut Vec<(String, String)>,
    rows: Option<&ModelDeliveryRows>,
    layout_document: Option<&boardstudio_model::model::ProjectDoc>,
) -> Result<Array, String> {
    let models = Array::new();
    for model in rows.into_iter().flat_map(|rows| &rows.delivered) {
        let mesh = &model.mesh;
        let lines = mesh
            .vendor_step
            .as_ref()
            .map(|step| step.line_render_positions.as_slice())
            .filter(|lines| !lines.is_empty());
        let (target, mesh, lines, name) = if let Some(matrix) = model.matrix {
            let Some((document, (module_id, _))) = layout_document.zip(
                model
                    .id
                    .strip_prefix("module-model/")
                    .and_then(|id| id.rsplit_once('/')),
            ) else {
                continue;
            };
            let name = document
                .modules
                .iter()
                .find(|module| module.id == module_id)
                .and_then(|module| {
                    document
                        .module_definitions
                        .iter()
                        .find(|def| def.id == module.definition_id)
                })
                .map_or("Mounted module", |def| def.name.as_str());
            let name = format!("{name} · 3D model");
            layers.push((model.id.clone(), name.clone()));
            let (positions, normals) =
                transformed_module_mesh(&mesh.positions, &mesh.normals, &matrix);
            (
                bodies,
                mesh_packet(&positions, &normals, mesh.colors.as_deref())?,
                lines.map(|lines| {
                    Float32Array::from(transformed_module_points(lines, &matrix).as_slice())
                }),
                Some(name),
            )
        } else {
            (
                &models,
                mesh_packet(&mesh.positions, &mesh.normals, mesh.colors.as_deref())?,
                lines.map(Float32Array::from),
                None,
            )
        };
        target.push(&js_obj(
            [("id", model.id.as_str().into()), ("mesh", mesh)]
                .into_iter()
                .chain(lines.map(|lines| ("lines", lines.into())))
                .chain(
                    name.as_deref()
                        .into_iter()
                        .flat_map(|name| [("name", name.into()), ("color", "#8f9b9e".into())]),
                ),
        )?);
    }
    Ok(models)
}
