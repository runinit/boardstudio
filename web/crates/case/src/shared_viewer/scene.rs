use super::*;

pub(super) fn project_source(
    source: &ViewerSource,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    match source {
        ViewerSource::Cad(scene) => project_case_scene(
            scene.clone(),
            identity,
            theme,
            preview,
            model_rows,
            keycaps_preview,
        ),
        ViewerSource::Native(preview) => {
            project_native_preview(preview, identity, theme, model_rows, keycaps_preview)
        }
        ViewerSource::Layout(preview) => {
            project_layout_preview(preview, identity, theme, model_rows, keycaps_preview)
        }
        ViewerSource::Parts(preview) => project_parts_preview(preview, identity, theme, model_rows),
    }
}

pub(super) fn preview_matches_source(
    preview: &crate::case_preview::CasePreviewOwnerIdentity,
    source_scope: &Scope,
    source_token: SnapshotToken,
) -> bool {
    preview.scope == *source_scope && preview.snapshot_token == source_token
}

pub(super) fn project_case_scene(
    scene: Rc<CadScene>,
    identity: ViewerIdentity,
    theme: &str,
    preview: Option<&NativePreviewSnapshot>,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    let document = captured_case_document(&scene.snapshot, &scene.scope)
        .map_err(|error| format!("Case scene projection failed: {error:?}"))?;
    let board = document
        .boards
        .iter()
        .find(|board| board.id == scene.scope.board_id)
        .ok_or_else(|| "Case board is unavailable in the selected scene".to_owned())?;
    let stack = scene
        .mechanical
        .as_ref()
        .map(|assembly| assembly.stack.as_slice())
        .unwrap_or_default();
    let physical_preview = preview.filter(|preview| {
        preview.owner.scope == scene.scope && preview.owner.snapshot_token == scene.token
    });
    let thickness = physical_preview
        .map(|preview| preview.preview.thickness)
        .unwrap_or(board.thickness);
    let contours = physical_preview
        .map(|preview| preview.contours.as_slice())
        .unwrap_or(scene.contours.as_slice());
    let surfaces = physical_preview
        .map(|preview| preview.preview.surfaces.as_slice())
        .unwrap_or_default();
    let holes = physical_preview
        .map(|preview| preview.preview.holes.as_slice())
        .unwrap_or_default();
    let models = physical_preview
        .map(|preview| preview.preview.models.as_slice())
        .unwrap_or_default();
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": thickness,
            "contours": contours,
            "surfaces": surfaces,
            "holes": holes,
            "models": models
        },
        "reference": physical_preview.and_then(|preview| preview.board_reference.as_ref()).map(|reference| {
            serde_json::json!({ "pose": &reference.pose, "elevation": reference.elevation })
        }),
        "models": [],
        "mechanicalStack": stack
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let loaded_models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &loaded_models).map_err(js_error)?;
    let bodies = Array::new();
    for body in &scene.result.bodies {
        let value = Object::new();
        let mesh = Object::new();
        Reflect::set(
            &mesh,
            &"positions".into(),
            &Float32Array::from(body.positions.as_slice()),
        )
        .map_err(js_error)?;
        Reflect::set(
            &mesh,
            &"normals".into(),
            &Float32Array::from(body.normals.as_slice()),
        )
        .map_err(js_error)?;
        Reflect::set(&value, &"id".into(), &body.id.clone().into()).map_err(js_error)?;
        Reflect::set(&value, &"name".into(), &body.name.clone().into()).map_err(js_error)?;
        Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
        bodies.push(&value);
    }
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    for body in &scene.result.bodies {
        if !layers.iter().any(|(id, _)| id == &body.id) {
            layers.push((body.id.clone(), body.name.clone()));
        }
    }
    append_keycap_bodies(
        &bodies,
        &mut layers,
        keycaps_preview,
        &identity,
        scene.snapshot.document.revision,
    )?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
}

pub(super) fn project_native_preview(
    preview: &NativePreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
    {
        return Err("Native PCB preview does not match the active Case viewer owner".into());
    }
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": preview.preview.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "reference": preview.board_reference.as_ref().map(|reference| {
            serde_json::json!({ "pose": &reference.pose, "elevation": reference.elevation })
        }),
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    let bodies = Array::new();
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    append_keycap_bodies(
        &bodies,
        &mut layers,
        keycaps_preview,
        &identity,
        preview.owner.accepted_revision,
    )?;
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
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
            let (color, label) = if let Some(id) = body.id.strip_prefix("keycap-legend:") {
                let spec = keycaps
                    .specs
                    .iter()
                    .find(|spec| spec.id == id)
                    .ok_or_else(|| {
                        "Keycap legend preview does not match its accepted specification".to_owned()
                    })?;
                (
                    spec.legend_color.as_str(),
                    format!("{} legend", spec.reference),
                )
            } else if let Some(id) = body.id.strip_prefix("keycap:") {
                let spec = keycaps
                    .specs
                    .iter()
                    .find(|spec| spec.id == id)
                    .ok_or_else(|| {
                        "Keycap preview does not match its accepted specification".to_owned()
                    })?;
                (spec.color.as_str(), spec.reference.clone())
            } else {
                return Err("Keycap preview returned an unknown body identity".into());
            };
            let value = Object::new();
            let mesh = Object::new();
            Reflect::set(
                &mesh,
                &"positions".into(),
                &Float32Array::from(body.positions.as_slice()),
            )
            .map_err(js_error)?;
            Reflect::set(
                &mesh,
                &"normals".into(),
                &Float32Array::from(body.normals.as_slice()),
            )
            .map_err(js_error)?;
            Reflect::set(&value, &"id".into(), &body.id.clone().into()).map_err(js_error)?;
            Reflect::set(&value, &"name".into(), &body.name.clone().into()).map_err(js_error)?;
            Reflect::set(&value, &"color".into(), &color.into()).map_err(js_error)?;
            Reflect::set(&value, &"mesh".into(), &mesh).map_err(js_error)?;
            bodies.push(&value);
            layers.push((body.id.clone(), label));
        }
    }
    Ok(())
}

pub(super) fn project_layout_preview(
    preview: &LayoutPreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
    keycaps_preview: Option<&crate::runtime::KeycapsCadPreview>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
        || !preview.lease.matches(&preview.owner)
    {
        return Err("Canonical Layout preview does not match the active viewer owner".into());
    }
    let reference = preview
        .board_reference
        .as_ref()
        .filter(|reference| reference.enabled)
        .map(|reference| {
            serde_json::json!({
                "pose": &reference.pose,
                "elevation": reference.elevation,
            })
        });
    let module_bodies = layout_module_bodies(
        &preview.module_scenes,
        &preview.document,
        &identity.scope.board_id,
        preview.preview.thickness,
    );
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "pcb",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": preview.preview.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "reference": reference,
        "moduleBodies": &module_bodies,
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    let bodies = Array::new();
    let mut layers = vec![("pcb".to_owned(), "PCB".to_owned())];
    for body in module_bodies {
        layers.push((
            body["id"].as_str().unwrap_or_default().to_owned(),
            body["name"].as_str().unwrap_or_default().to_owned(),
        ));
    }
    let module_model_bodies = layout_module_model_bodies(model_rows, &preview.document);
    for body in module_model_bodies {
        let id = body["id"].as_str().unwrap_or_default().to_owned();
        let name = body["name"].as_str().unwrap_or_default().to_owned();
        let value = js_sys::JSON::parse(&body.to_string()).map_err(js_error)?;
        bodies.push(&value);
        layers.push((id, name));
    }
    append_keycap_bodies(
        &bodies,
        &mut layers,
        keycaps_preview,
        &identity,
        preview.owner.accepted_revision,
    )?;
    Reflect::set(&input, &"bodies".into(), &bodies).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers,
    })
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

/// Apply Core's column-major affine transform exactly once before generic mesh
/// bodies enter the renderer. This mirrors the existing React AssemblyPreview
/// transform while keeping Core authoritative for module pose and model offsets.
fn transformed_module_points(source_positions: &[f32], matrix: &[f64; 16]) -> Vec<f32> {
    let mut positions = vec![0.0; source_positions.len()];
    for index in (0..source_positions.len()).step_by(3) {
        let x = source_positions[index];
        let y = source_positions[index + 1];
        let z = source_positions[index + 2];
        positions[index] = (matrix[0] as f32) * x
            + (matrix[4] as f32) * y
            + (matrix[8] as f32) * z
            + matrix[12] as f32;
        positions[index + 1] = (matrix[1] as f32) * x
            + (matrix[5] as f32) * y
            + (matrix[9] as f32) * z
            + matrix[13] as f32;
        positions[index + 2] = (matrix[2] as f32) * x
            + (matrix[6] as f32) * y
            + (matrix[10] as f32) * z
            + matrix[14] as f32;
    }
    positions
}

pub(super) fn transformed_module_mesh(
    source_positions: &[f32],
    source_normals: &[f32],
    matrix: &[f64; 16],
) -> (Vec<f32>, Vec<f32>) {
    let squared_lengths = [0, 4, 8].map(|index| {
        (matrix[index].powi(2) + matrix[index + 1].powi(2) + matrix[index + 2].powi(2)) as f32
    });
    let mut positions = vec![0.0; source_positions.len()];
    let mut normals = vec![0.0; source_normals.len()];
    for index in (0..source_positions.len()).step_by(3) {
        let x = source_positions[index];
        let y = source_positions[index + 1];
        let z = source_positions[index + 2];
        positions[index] = (matrix[0] as f32) * x
            + (matrix[4] as f32) * y
            + (matrix[8] as f32) * z
            + matrix[12] as f32;
        positions[index + 1] = (matrix[1] as f32) * x
            + (matrix[5] as f32) * y
            + (matrix[9] as f32) * z
            + matrix[13] as f32;
        positions[index + 2] = (matrix[2] as f32) * x
            + (matrix[6] as f32) * y
            + (matrix[10] as f32) * z
            + matrix[14] as f32;

        let nx = source_normals[index] / squared_lengths[0].max(f32::MIN_POSITIVE);
        let ny = source_normals[index + 1] / squared_lengths[1].max(f32::MIN_POSITIVE);
        let nz = source_normals[index + 2] / squared_lengths[2].max(f32::MIN_POSITIVE);
        let a = matrix[0] as f32 * nx + matrix[4] as f32 * ny + matrix[8] as f32 * nz;
        let b = matrix[1] as f32 * nx + matrix[5] as f32 * ny + matrix[9] as f32 * nz;
        let c = matrix[2] as f32 * nx + matrix[6] as f32 * ny + matrix[10] as f32 * nz;
        let length = (a * a + b * b + c * c).sqrt();
        let length = if length > 0.0 { length } else { 1.0 };
        normals[index] = a / length;
        normals[index + 1] = b / length;
        normals[index + 2] = c / length;
    }
    (positions, normals)
}

pub(super) fn layout_module_bodies(
    modules: &[boardstudio_model::model::ResolvedModule],
    document: &boardstudio_model::model::ProjectDoc,
    board_id: &str,
    pcb_top_z: f64,
) -> Vec<serde_json::Value> {
    modules
        .iter()
        .flat_map(|module| {
            let Some(instance) = document
                .modules
                .iter()
                .find(|instance| instance.id == module.id && instance.host_board_id == board_id)
            else {
                return Vec::new();
            };
            let name = document
                .module_definitions
                .iter()
                .find(|definition| definition.id == instance.definition_id)
                .map(|definition| definition.name.as_str())
                .unwrap_or("Mounted module");
            let contour =
                |points: &[_], hole: bool| serde_json::json!({ "points": points, "hole": hole });
            let mut bodies = module
                .board
                .iter()
                .enumerate()
                .map(|(index, solid)| {
                    let mut contours = vec![contour(&solid.points, false)];
                    contours.extend(
                        module
                            .board_holes
                            .iter()
                            .map(|hole| contour(&hole.points, true)),
                    );
                    serde_json::json!({
                        "id": format!("module:{}:pcb:{index}", module.id),
                        "name": format!("{name} · PCB"),
                        "z": solid.z + pcb_top_z,
                        "thickness": solid.height,
                        "contours": contours,
                    })
                })
                .collect::<Vec<_>>();
            bodies.extend(module.volumes.iter().enumerate().map(|(index, volume)| {
                serde_json::json!({
                    "id": format!("module:{}:volume:{index}", module.id),
                    "name": format!("{name} · Assembly volume"),
                    "z": volume.geometry.z + pcb_top_z,
                    "thickness": volume.geometry.height,
                    "contours": [contour(&volume.geometry.points, false)],
                })
            }));
            if instance.attachment == boardstudio_model::model::ModuleAttachment::Board {
                for support in &module.mount_supports {
                    let ring = |diameter: f64, clockwise: bool| {
                        (0..48)
                            .map(|index| {
                                let direction = if clockwise { -1.0 } else { 1.0 };
                                let angle = std::f64::consts::TAU * index as f64 / 48.0;
                                boardstudio_model::model::Vec2 {
                                    x: support.at.x + diameter / 2.0 * (direction * angle).cos(),
                                    y: support.at.y + diameter / 2.0 * (direction * angle).sin(),
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                    bodies.push(serde_json::json!({
                        "id": format!("module:{}:standoff:{}", module.id, support.mount_id),
                        "name": format!("{} · PCB standoff", module.id),
                        "z": support.z + pcb_top_z,
                        "thickness": support.height,
                        "contours": [
                            contour(&ring(support.outer_diameter, false), false),
                            contour(&ring(support.hole_diameter, true), true),
                        ],
                    }));
                }
            }
            bodies
        })
        .collect()
}

fn project_parts_preview(
    preview: &crate::parts_preview::PartsPreviewSnapshot,
    identity: ViewerIdentity,
    theme: &str,
    model_rows: Option<&ModelDeliveryRows>,
) -> Result<RendererSceneProjection, String> {
    if preview.owner.scope != identity.scope
        || preview.owner.snapshot_token != identity.snapshot_token
        || !preview.lease.matches(&preview.owner)
    {
        return Err("Parts sample does not match the active viewer owner".into());
    }
    let board = preview
        .sample_document
        .boards
        .iter()
        .find(|board| board.id == "sample-board")
        .ok_or_else(|| "Parts sample board is unavailable".to_owned())?;
    let packet = serde_json::json!({
        "revision": identity.renderer_sequence,
        "kind": "assembly",
        "theme": theme,
        "view": "assembled",
        "keepCamera": true,
        "selectedLayer": "",
        "hidden": [],
        "board": {
            "revision": identity.renderer_sequence,
            "thickness": board.thickness,
            "contours": &preview.contours,
            "surfaces": &preview.preview.surfaces,
            "holes": &preview.preview.holes,
            "models": &preview.preview.models
        },
        "models": [],
        "mechanicalStack": []
    });
    let input = js_sys::JSON::parse(&packet.to_string()).map_err(js_error)?;
    let models = loaded_model_inputs(model_rows);
    Reflect::set(&input, &"models".into(), &models).map_err(js_error)?;
    Ok(RendererSceneProjection {
        identity,
        input,
        layers: vec![("pcb".to_owned(), "PCB".to_owned())],
    })
}

pub(super) fn loaded_model_inputs(rows: Option<&ModelDeliveryRows>) -> Array {
    let loaded = Array::new();
    if let Some(rows) = rows {
        for model in &rows.delivered {
            if model.matrix.is_some() {
                continue;
            }
            let value = Object::new();
            let mesh = Object::new();
            let _ = Reflect::set(&value, &"id".into(), &model.id.clone().into());
            let _ = Reflect::set(
                &mesh,
                &"positions".into(),
                &Float32Array::from(model.mesh.positions.as_ref()),
            );
            let _ = Reflect::set(
                &mesh,
                &"normals".into(),
                &Float32Array::from(model.mesh.normals.as_ref()),
            );
            if let Some(colors) = &model.mesh.colors {
                let _ = Reflect::set(
                    &mesh,
                    &"colors".into(),
                    &Float32Array::from(colors.as_ref()),
                );
            }
            let _ = Reflect::set(&value, &"mesh".into(), &mesh);
            if let Some(vendor_step) = &model.mesh.vendor_step
                && !vendor_step.line_render_positions.is_empty()
            {
                let _ = Reflect::set(
                    &value,
                    &"lines".into(),
                    &Float32Array::from(vendor_step.line_render_positions.as_slice()),
                );
            }
            loaded.push(&value);
        }
    }
    loaded
}

pub(super) fn js_error(value: JsValue) -> String {
    value
        .as_string()
        .or_else(|| {
            Reflect::get(&value, &"message".into())
                .ok()
                .and_then(|message| message.as_string())
        })
        .unwrap_or_else(|| "Browser renderer operation failed".to_owned())
}
