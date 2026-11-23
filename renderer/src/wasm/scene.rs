use super::text::text_mesh;
use super::*;

pub(super) struct BuiltScene {
    pub(super) objects: Vec<SceneObject>,
    pub(super) bounds: Option<(Vec3, f32)>,
    pub(super) revision: u64,
    pub(super) keep_camera: bool,
}

pub(super) fn build_scene(input: SceneInput) -> Result<BuiltScene, JsValue> {
    let mut output = BuiltScene {
        objects: Vec::new(),
        bounds: None,
        revision: input.revision,
        keep_camera: input.keep_camera,
    };
    if input.kind == "case" {
        if let Some(mesh) = input.case_mesh {
            push_mesh(
                &mut output,
                "case",
                mesh,
                Mat4::identity(),
                [0.72, 0.70, 0.65, 1.0],
                0.74,
                0.04,
            )?;
        }
        for (index, component) in input.component_previews.into_iter().enumerate() {
            let back = component.side == "back";
            let pose = Mat4::from_translation(Vec3::new(
                component.pose.at.x,
                component.pose.at.y,
                if back { 0.0 } else { input.board_thickness },
            )) * Mat4::from_angle_z(three_d::Rad(component.pose.rotation.to_radians()));
            let model = component_model_transform(
                [
                    component.model.offset.x,
                    component.model.offset.y,
                    component.model.offset.z,
                ],
                [
                    component.model.rotation.x,
                    component.model.rotation.y,
                    component.model.rotation.z,
                ],
                [
                    component.model.scale.x,
                    component.model.scale.y,
                    component.model.scale.z,
                ],
                back,
            );
            push_mesh(
                &mut output,
                if component.reference.is_empty() {
                    &component.id
                } else {
                    &component.reference
                },
                component.mesh,
                pose * model,
                if index % 2 == 0 {
                    [0.62, 0.67, 0.74, 1.0]
                } else {
                    [0.73, 0.72, 0.68, 1.0]
                },
                0.62,
                0.08,
            )?;
        }
    } else if input.kind == "bodyPatch" {
        for body in input.bodies {
            let color = body
                .color
                .as_deref()
                .and_then(hex_color)
                .unwrap_or(body_base_color(&input.theme, &body.id));
            push_mesh(
                &mut output,
                &body.id,
                body.mesh,
                Mat4::identity(),
                color,
                0.68,
                0.03,
            )?;
            if let Some(lines) = body.lines {
                let object = output.objects.last_mut().unwrap();
                push_transformed_lines(&mut object.lines, lines, Mat4::identity())?;
            }
        }
    } else if let Some(board) = input.board {
        let hidden = |id: &str| input.hidden.iter().any(|entry| entry == id);
        let contours = board
            .contours
            .iter()
            .map(|contour| {
                (
                    contour
                        .points
                        .iter()
                        .map(|point| [point.x, point.y])
                        .collect::<Vec<_>>(),
                    contour.hole,
                )
            })
            .collect::<Vec<_>>();
        let holes = board
            .holes
            .iter()
            .map(|hole| {
                hole.iter()
                    .map(|point| [point.x, point.y])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut contour_refs = contours
            .iter()
            .map(|(points, hole)| BoardContour {
                points,
                hole: *hole,
            })
            .collect::<Vec<_>>();
        contour_refs.extend(
            holes
                .iter()
                .map(|points| BoardContour { points, hole: true }),
        );
        let board_area = contours
            .iter()
            .map(|(points, hole)| polygon_area(points) * if *hole { -1.0 } else { 1.0 })
            .sum::<f32>()
            - holes.iter().map(|hole| polygon_area(hole)).sum::<f32>();
        let board_area = board_area.max(1.0);
        let explode_z = if input.view == "exploded" {
            explode_offset(stack_index(&input.mechanical_stack, "pcb"))
        } else {
            0.0
        };
        let pcb_transform = Mat4::from_translation(Vec3::new(0.0, 0.0, explode_z))
            * physical_pcb_transform(
                input.reference.as_ref().map(|reference| {
                    (
                        [reference.pose.at.x, reference.pose.at.y],
                        reference.pose.rotation,
                        reference.elevation,
                    )
                }),
                board.thickness,
                !input.mechanical_stack.is_empty(),
            );
        if !hidden("PCB") {
            let mut board_mesh = board_mesh(&contour_refs, board.thickness)
                .map_err(|error| JsValue::from_str(&error))?;
            board_mesh.colors = Some(
                board_mesh
                    .normals
                    .iter()
                    .map(|normal| {
                        if normal[2].abs() > 0.5 {
                            [0.12, 0.36, 0.22]
                        } else {
                            [0.49, 0.39, 0.24]
                        }
                    })
                    .collect(),
            );
            translate_mesh(&mut board_mesh, pcb_transform);
            push_data(
                &mut output,
                "pcb",
                board_mesh,
                [1.0, 1.0, 1.0, 1.0],
                0.82,
                0.0,
            );
        }
        let clipper = SurfaceClipper::new(&contour_refs, &holes);
        for surface in &board.surfaces {
            let kind = if surface.layer.ends_with(".Cu") {
                "Copper"
            } else if surface.layer.ends_with(".Mask") {
                "Mask"
            } else if surface.layer.ends_with(".SilkS") {
                "Silkscreen"
            } else {
                continue;
            };
            if hidden("PCB") || hidden(kind) {
                continue;
            }
            let back = surface.layer.starts_with("B.");
            let points = surface
                .points
                .iter()
                .map(|point| [point.x, point.y])
                .collect::<Vec<_>>();
            // Large filled copper polygons are normally covered by solder mask.
            // Keep them under the green board color so they do not paint a whole
            // copper zone gold over traces, pads, and silkscreen.
            let covered_copper =
                kind == "Copper" && surface.filled && polygon_area(&points) >= board_area * 0.08;
            let layer_offset = match kind {
                "Copper" if covered_copper => 0.02,
                "Copper" => 0.04,
                "Mask" => 0.06,
                _ => 0.08,
            };
            let z = if back {
                -layer_offset
            } else {
                board.thickness + layer_offset
            };
            let normal = if back {
                [0.0, 0.0, -1.0]
            } else {
                [0.0, 0.0, 1.0]
            };
            let geometry = if !surface.text.is_empty() {
                text_mesh(
                    &surface.text,
                    surface
                        .points
                        .first()
                        .map_or([0.0, 0.0], |point| [point.x, point.y]),
                    surface.text_size.max(0.1),
                    surface.rotation,
                    back,
                    z,
                )
            } else if surface.filled {
                surface_mesh(&points, z, normal)
            } else {
                stroke_mesh(&points, surface.width, z, normal)
            }
            .map_err(|error| JsValue::from_str(&error))?;
            let color = if covered_copper {
                [0.12, 0.36, 0.22, 1.0]
            } else {
                match kind {
                    "Silkscreen" => [0.93, 0.92, 0.85, 1.0],
                    "Mask" | "Copper" => [0.76, 0.58, 0.22, 1.0],
                    _ => [0.10, 0.30, 0.21, 1.0],
                }
            };
            let mut geometry = geometry;
            clipper
                .clip(&mut geometry)
                .map_err(|error| JsValue::from_str(&error))?;
            translate_mesh(&mut geometry, pcb_transform);
            push_data(
                &mut output,
                "pcb",
                geometry,
                color,
                if kind == "Copper" || kind == "Mask" {
                    0.35
                } else {
                    0.8
                },
                if kind == "Copper" || kind == "Mask" {
                    0.65
                } else {
                    0.0
                },
            );
            output.objects.last_mut().unwrap().groups =
                vec!["PCB".into(), kind.into(), surface.layer.clone()];
        }
        if !hidden("PCB") {
            for (points, _) in contours.iter().filter(|(_, hole)| !hole) {
                if points.len() < 3 {
                    continue;
                }
                let mut boundary = points.clone();
                boundary.push(points[0]);
                let mut outline =
                    stroke_mesh(&boundary, 0.3, board.thickness + 0.1, [0.0, 0.0, 1.0])
                        .map_err(|error| JsValue::from_str(&error))?;
                translate_mesh(&mut outline, pcb_transform);
                push_data(
                    &mut output,
                    "pcb-selection",
                    outline,
                    [0.97, 0.69, 0.20, 1.0],
                    0.45,
                    0.15,
                );
            }
        }
        for loaded in input.models {
            if hidden("Models") || hidden(&loaded.id) {
                continue;
            }
            if let Some(model) = board.models.iter().find(|model| model.id == loaded.id) {
                if model.path.to_ascii_lowercase().contains("keycap") && hidden("Keycaps") {
                    continue;
                }
                let back = model.side == "back";
                let model_transform = pcb_model_transform(
                    [
                        model.pose.at.x,
                        model.pose.at.y,
                        if back { 0.0 } else { board.thickness },
                    ],
                    model.pose.rotation,
                    back,
                    [model.offset.x, model.offset.y, model.offset.z],
                    [model.rotation.x, model.rotation.y, model.rotation.z],
                    [model.scale.x, model.scale.y, model.scale.z],
                );
                let color = if input.selected_layer == model.reference {
                    [0.84, 0.63, 0.23, 1.0]
                } else {
                    [0.70, 0.73, 0.77, 1.0]
                };
                let row_transform = pcb_transform * model_transform;
                push_mesh(
                    &mut output,
                    &model.reference,
                    loaded.mesh,
                    row_transform,
                    color,
                    0.55,
                    0.12,
                )?;
                let object = output.objects.last_mut().unwrap();
                if let Some(lines) = loaded.lines {
                    push_transformed_lines(&mut object.lines, lines, row_transform)?;
                }
                object.groups = vec!["Models".into(), loaded.id.clone()];
                if model.path.to_ascii_lowercase().contains("keycap") {
                    object.groups.push("Keycaps".into());
                }
            }
        }
        for body in input.module_bodies {
            if hidden(&body.id) {
                continue;
            }
            let mut mesh = module_body_mesh(&body).map_err(|error| JsValue::from_str(&error))?;
            if mesh.positions.is_empty() {
                continue;
            }
            let transform = pcb_transform * Mat4::from_translation(Vec3::new(0.0, 0.0, body.z));
            transform_mesh(&mut mesh, transform);
            let color = if input.selected_layer == body.id {
                [0.84, 0.63, 0.23, 1.0]
            } else {
                [0.28, 0.49, 0.40, 1.0]
            };
            push_data(&mut output, &body.id, mesh, color, 0.72, 0.0);
        }
        for body in input.bodies {
            if hidden(&body.id) || (body.id.starts_with("keycap") && hidden("Keycaps")) {
                continue;
            }
            let layer_index = stack_index(&input.mechanical_stack, &body.id);
            let offset = if input.view == "exploded" {
                explode_offset(layer_index)
            } else {
                0.0
            };
            let fallback = if input.selected_layer == body.id {
                [0.84, 0.63, 0.23, 1.0]
            } else {
                body_base_color(&input.theme, &body.id)
            };
            let color = body
                .color
                .as_deref()
                .and_then(hex_color)
                .unwrap_or(fallback);
            let body_transform = Mat4::from_translation(Vec3::new(0.0, 0.0, offset));
            push_mesh(
                &mut output,
                &body.id,
                body.mesh,
                body_transform,
                color,
                0.68,
                0.03,
            )?;
            if let Some(lines) = body.lines {
                let object = output.objects.last_mut().unwrap();
                push_transformed_lines(&mut object.lines, lines, body_transform)?;
            }
        }
        if let Some(battery) = input.battery
            && !hidden("battery")
        {
            let offset = if input.view == "exploded" {
                explode_offset(stack_index(&input.mechanical_stack, "battery"))
            } else {
                0.0
            };
            let center_z = input
                .mechanical_stack
                .iter()
                .find(|layer| layer.id == "battery")
                .map_or(battery.size.z / 2.0, |layer| {
                    layer.z + layer.thickness / 2.0
                })
                + offset;
            let mut box_geometry = box_data([battery.size.x, battery.size.y, battery.size.z]);
            translate_mesh(
                &mut box_geometry,
                Mat4::from_translation(Vec3::new(battery.at.x, battery.at.y, center_z)),
            );
            push_data(
                &mut output,
                "battery",
                box_geometry,
                [0.76, 0.43, 0.24, 1.0],
                0.7,
                0.02,
            );
            let cable = stroke_mesh(
                &[
                    [battery.at.x, battery.at.y],
                    [battery.cable_exit.x, battery.cable_exit.y],
                ],
                0.45,
                center_z,
                [0.0, 0.0, 1.0],
            )
            .map_err(|error| JsValue::from_str(&error))?;
            push_data(
                &mut output,
                "battery",
                cable,
                [0.81, 0.25, 0.22, 1.0],
                0.72,
                0.0,
            );
        }
    }
    for object in &mut output.objects {
        if object.id.starts_with("keycap:") || object.id.starts_with("keycap-legend:") {
            object.groups.extend(["Keycaps".into(), "Models".into()]);
        }
        if (object.id == "pcb" || object.id == "pcb-selection")
            && !object.groups.iter().any(|g| g == "PCB")
        {
            object.groups.push("PCB".into());
        }
        let layer = if object.groups.iter().any(|g| g == "Models") || object.id.starts_with("pcb") {
            "pcb"
        } else {
            &object.id
        };
        object.explode = explode_offset(stack_index(&input.mechanical_stack, layer));
    }
    batch_surfaces(&mut output.objects);
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for object in &output.objects {
        for point in object.positions.iter().chain(&object.lines) {
            min.x = min.x.min(point.x);
            min.y = min.y.min(point.y);
            min.z = min.z.min(point.z);
            max.x = max.x.max(point.x);
            max.y = max.y.max(point.y);
            max.z = max.z.max(point.z);
        }
    }
    if min.x.is_finite() {
        let center = (min + max) / 2.0;
        let radius = (max - min).magnitude() / 2.0;
        output.bounds = Some((center, radius));
    }
    Ok(output)
}

fn module_body_mesh(body: &ModuleBodyInput) -> Result<MeshData, String> {
    if body.id.is_empty()
        || !body.z.is_finite()
        || !body.thickness.is_finite()
        || body.thickness <= 0.0
    {
        return Err("Module body identity and thickness must be valid".into());
    }
    let contours = body
        .contours
        .iter()
        .map(|contour| {
            (
                contour
                    .points
                    .iter()
                    .map(|point| [point.x, point.y])
                    .collect::<Vec<_>>(),
                contour.hole,
            )
        })
        .collect::<Vec<_>>();
    let contour_refs = contours
        .iter()
        .map(|(points, hole)| BoardContour {
            points,
            hole: *hole,
        })
        .collect::<Vec<_>>();
    board_mesh(&contour_refs, body.thickness)
}

pub(super) fn push_mesh(
    output: &mut BuiltScene,
    id: &str,
    mesh: MeshInput,
    transform: Mat4,
    color: [f32; 4],
    roughness: f32,
    metallic: f32,
) -> Result<(), JsValue> {
    let mut data = MeshData::default();
    if !mesh.positions.len().is_multiple_of(3) {
        return Err(JsValue::from_str("Mesh position buffer is incomplete"));
    }
    for chunk in mesh.positions.as_chunks::<3>().0.iter() {
        let transformed = transform * Vec4::new(chunk[0], chunk[1], chunk[2], 1.0);
        data.positions
            .push([transformed.x, transformed.y, transformed.z]);
    }
    if mesh.normals.len() == mesh.positions.len() {
        let normal_matrix = transform
            .invert()
            .unwrap_or_else(Mat4::identity)
            .transpose();
        for normal in mesh.normals.as_chunks::<3>().0.iter() {
            let value = normal_matrix * Vec4::new(normal[0], normal[1], normal[2], 0.0);
            let value = value.truncate().normalize();
            data.normals.push([value.x, value.y, value.z]);
        }
    } else {
        for tri in data.positions.as_chunks::<3>().0.iter() {
            let normal = (Vec3::new(
                tri[1][0] - tri[0][0],
                tri[1][1] - tri[0][1],
                tri[1][2] - tri[0][2],
            )
            .cross(Vec3::new(
                tri[2][0] - tri[0][0],
                tri[2][1] - tri[0][1],
                tri[2][2] - tri[0][2],
            )))
            .normalize();
            data.normals
                .extend_from_slice(&[[normal.x, normal.y, normal.z]; 3]);
        }
    }
    if !mesh.positions.len().is_multiple_of(9) {
        return Err(JsValue::from_str(
            "Mesh position buffer must contain complete triangles",
        ));
    }
    data.indices = (0..data.positions.len() as u32).collect();
    if let Some(colors) = mesh.colors {
        if colors.len() != data.positions.len() * 3 {
            return Err(JsValue::from_str(
                "Mesh color buffer does not match positions",
            ));
        }
        data.colors = Some(
            colors
                .as_chunks::<3>()
                .0
                .iter()
                .map(|value| [value[0], value[1], value[2]])
                .collect(),
        );
    }
    push_data(output, id, data, color, roughness, metallic);
    Ok(())
}

pub(super) fn push_data(
    output: &mut BuiltScene,
    id: &str,
    data: MeshData,
    color: [f32; 4],
    roughness: f32,
    metallic: f32,
) {
    output.objects.push(SceneObject {
        id: id.to_owned(),
        groups: vec![id.to_owned()],
        explode: 0.,
        edges: None,
        lines: Vec::new(),
        positions: data
            .positions
            .iter()
            .map(|p| Vec3::new(p[0], p[1], p[2]))
            .collect(),
        normals: data
            .normals
            .iter()
            .map(|n| Vec3::new(n[0], n[1], n[2]))
            .collect(),
        colors: data.colors.as_ref().map(|items| {
            items
                .iter()
                .map(|c| {
                    Srgba::new(
                        (c[0].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[1].clamp(0.0, 1.0) * 255.0) as u8,
                        (c[2].clamp(0.0, 1.0) * 255.0) as u8,
                        255,
                    )
                })
                .collect()
        }),
        indices: data.indices,
        color: Srgba::new(
            (color[0].clamp(0.0, 1.0) * 255.0) as u8,
            (color[1].clamp(0.0, 1.0) * 255.0) as u8,
            (color[2].clamp(0.0, 1.0) * 255.0) as u8,
            (color[3].clamp(0.0, 1.0) * 255.0) as u8,
        ),
        roughness,
        metallic,
    });
}

pub(super) fn push_transformed_lines(
    target: &mut Vec<Vec3>,
    lines: Vec<f32>,
    transform: Mat4,
) -> Result<(), JsValue> {
    if !lines.len().is_multiple_of(6) || lines.iter().any(|value| !value.is_finite()) {
        return Err(JsValue::from_str(
            "Source line buffer must contain finite 3D endpoint pairs",
        ));
    }
    for point in lines.as_chunks::<3>().0.iter() {
        let transformed = transform * Vec4::new(point[0], point[1], point[2], 1.0);
        target.push(Vec3::new(transformed.x, transformed.y, transformed.z));
    }
    Ok(())
}

/// Default body color shared by the body-patch and case-body render paths.
fn body_base_color(theme: &str, id: &str) -> [f32; 4] {
    if id.to_ascii_lowercase().contains("foam") || id.to_ascii_lowercase().contains("gasket") {
        [0.20, 0.18, 0.24, 1.0]
    } else if id.to_ascii_lowercase().contains("plate") {
        [0.34, 0.57, 0.44, 1.0]
    } else if theme == "dark" {
        [0.62, 0.66, 0.72, 1.0]
    } else {
        [0.68, 0.71, 0.75, 1.0]
    }
}

fn translate_mesh(mesh: &mut MeshData, transform: Mat4) {
    for point in &mut mesh.positions {
        let output = transform * Vec4::new(point[0], point[1], point[2], 1.0);
        *point = [output.x, output.y, output.z];
    }
}

fn transform_mesh(mesh: &mut MeshData, transform: Mat4) {
    translate_mesh(mesh, transform);
    let normal_matrix = transform
        .invert()
        .unwrap_or_else(Mat4::identity)
        .transpose();
    for normal in &mut mesh.normals {
        let transformed = normal_matrix * Vec4::new(normal[0], normal[1], normal[2], 0.0);
        let transformed = transformed.truncate().normalize();
        *normal = [transformed.x, transformed.y, transformed.z];
    }
}

fn box_data(size: [f32; 3]) -> MeshData {
    let vertices = box_corners(size);
    let faces = [
        [0, 1, 2, 3],
        [4, 7, 6, 5],
        [0, 4, 5, 1],
        [1, 5, 6, 2],
        [2, 6, 7, 3],
        [4, 0, 3, 7],
    ];
    let mut data = MeshData::default();
    for face in faces {
        for triangle in [[face[0], face[1], face[2]], [face[0], face[2], face[3]]] {
            let start = data.positions.len() as u32;
            for index in triangle {
                data.positions.push(vertices[index]);
            }
            let a = Vec3::new(
                data.positions[start as usize][0],
                data.positions[start as usize][1],
                data.positions[start as usize][2],
            );
            let b = Vec3::new(
                data.positions[start as usize + 1][0],
                data.positions[start as usize + 1][1],
                data.positions[start as usize + 1][2],
            );
            let c = Vec3::new(
                data.positions[start as usize + 2][0],
                data.positions[start as usize + 2][1],
                data.positions[start as usize + 2][2],
            );
            let n = (b - a).cross(c - a).normalize();
            data.normals.extend_from_slice(&[[n.x, n.y, n.z]; 3]);
            data.indices
                .extend_from_slice(&[start, start + 1, start + 2]);
        }
    }
    data
}
