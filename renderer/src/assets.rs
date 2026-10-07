use crate::{MAX_MODEL_BYTES, MAX_MODEL_POSITION_FLOATS, ModelMesh};
use three_d::{Mat4, Vec3, Vec4};
use three_d_asset::{
    Geometry, InnerSpace, Matrix, Positions, Scene, SquareMatrix, TriMesh, io::deserialize,
};

pub fn decode_stl(bytes: &[u8]) -> Result<ModelMesh, String> {
    check_size(bytes)?;
    let scene: Scene = deserialize("model.stl", bytes.to_vec())
        .map_err(|error| format!("STL preview failed: {error}"))?;
    let mut mesh = ModelMesh::default();
    append_scene_nodes(&scene, Mat4::identity(), &mut mesh)?;
    mesh.finish("STL")
}

pub(crate) fn check_size(bytes: &[u8]) -> Result<(), String> {
    if bytes.is_empty() || bytes.len() > MAX_MODEL_BYTES {
        return Err("Model must be between 1 byte and 32 MiB".to_owned());
    }
    Ok(())
}

pub(crate) fn append_geometry(
    triangles: &TriMesh,
    transform: Mat4,
    color: [f32; 3],
    vertex_colors: Option<&[[f32; 3]]>,
    result: &mut ModelMesh,
) -> Result<(), String> {
    let positions = match &triangles.positions {
        Positions::F32(values) => values
            .iter()
            .map(|point| [point.x, point.y, point.z])
            .collect::<Vec<_>>(),
        Positions::F64(values) => values
            .iter()
            .map(|point| [point.x as f32, point.y as f32, point.z as f32])
            .collect::<Vec<_>>(),
    };
    let normals = triangles.normals.as_deref();
    let colors = triangles.colors.as_deref();
    let indices = triangles.indices.to_u32();
    let count = indices.as_ref().map_or(positions.len(), Vec::len);
    if !count.is_multiple_of(3) {
        return Err("Model has incomplete triangles".to_owned());
    }

    let triangle_count = count / 3;
    let color_count =
        vertex_colors.map_or(colors.map_or(0, |items| items.len()), |items| items.len());
    let needs_colors = vertex_colors.is_some() || colors.is_some() || result.colors.is_some();
    if result.positions.len() + count * 3 > MAX_MODEL_POSITION_FLOATS {
        return Err("Model exceeds preview triangle limits".to_owned());
    }
    if needs_colors && result.colors.is_none() {
        result.colors = Some([0.72, 0.75, 0.78].repeat(result.positions.len() / 3));
    }

    for triangle in 0..triangle_count {
        let ids = [0, 1, 2].map(|corner| {
            indices
                .as_ref()
                .map_or((triangle * 3 + corner) as u32, |values| {
                    values[triangle * 3 + corner]
                }) as usize
        });
        if ids.iter().any(|index| *index >= positions.len()) {
            return Err("Model contains an invalid triangle index".to_owned());
        }
        let a = positions[ids[0]];
        let b = positions[ids[1]];
        let c = positions[ids[2]];
        let face_normal = normal_from_points(a, b, c);
        for (corner, index) in ids.into_iter().enumerate() {
            let point = transform
                * Vec4::new(
                    positions[index][0],
                    positions[index][1],
                    positions[index][2],
                    1.0,
                );
            let point = [point.x, point.y, point.z];
            let normal = normals
                .filter(|values| values.len() == positions.len())
                .map(|values| {
                    transform_normal(
                        transform,
                        [values[index].x, values[index].y, values[index].z],
                    )
                })
                .unwrap_or(face_normal);
            result.positions.extend_from_slice(&point);
            result.normals.extend_from_slice(&normal);
            if let Some(output) = &mut result.colors {
                let vertex_color = vertex_colors
                    .filter(|values| values.len() == positions.len())
                    .map(|values| values[index])
                    .or_else(|| {
                        colors
                            .filter(|values| values.len() == positions.len())
                            .map(|values| {
                                [
                                    f32::from(values[index].r) / 255.0,
                                    f32::from(values[index].g) / 255.0,
                                    f32::from(values[index].b) / 255.0,
                                ]
                            })
                    })
                    .or_else(|| (color_count == positions.len()).then_some(color))
                    .unwrap_or(color);
                output.extend_from_slice(&vertex_color);
            }
            let _ = corner;
        }
    }
    Ok(())
}

fn append_scene_nodes(
    scene: &Scene,
    transform: Mat4,
    result: &mut ModelMesh,
) -> Result<(), String> {
    for node in &scene.children {
        append_scene_node(scene, node, transform, result)?;
    }
    Ok(())
}

fn append_scene_node(
    scene: &Scene,
    node: &three_d_asset::Node,
    transform: Mat4,
    result: &mut ModelMesh,
) -> Result<(), String> {
    let combined = transform * node.transformation;
    if let Some(Geometry::Triangles(triangles)) = &node.geometry {
        let material = node
            .material_index
            .and_then(|index| scene.materials.get(index));
        let color = material.map_or([0.72, 0.75, 0.78], |material| {
            [
                f32::from(material.albedo.r) / 255.0,
                f32::from(material.albedo.g) / 255.0,
                f32::from(material.albedo.b) / 255.0,
            ]
        });
        append_geometry(triangles, combined, color, None, result)?;
    }
    for child in &node.children {
        append_scene_node(scene, child, combined, result)?;
    }
    Ok(())
}

fn normal_from_points(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = Vec3::new(b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let ac = Vec3::new(c[0] - a[0], c[1] - a[1], c[2] - a[2]);
    let normal = ab.cross(ac);
    let length = normal.magnitude();
    if length > 0.0 {
        [normal.x / length, normal.y / length, normal.z / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

fn transform_normal(transform: Mat4, normal: [f32; 3]) -> [f32; 3] {
    let matrix = transform.invert().map(|matrix| matrix.transpose());
    let value = matrix.map_or(Vec3::new(normal[0], normal[1], normal[2]), |matrix| {
        (matrix * Vec4::new(normal[0], normal[1], normal[2], 0.0)).truncate()
    });
    let length = value.magnitude();
    if length > 0.0 {
        [value.x / length, value.y / length, value.z / length]
    } else {
        [0.0, 0.0, 1.0]
    }
}

impl ModelMesh {
    pub(crate) fn finish(mut self, format: &str) -> Result<Self, String> {
        if self.positions.is_empty() || !self.positions.len().is_multiple_of(9) {
            return Err(format!("{format} model has no finite triangles"));
        }
        if self
            .positions
            .iter()
            .chain(self.normals.iter())
            .chain(self.colors.iter().flatten())
            .any(|value| !value.is_finite())
        {
            return Err(format!("{format} model has no finite triangles"));
        }
        if self
            .colors
            .as_ref()
            .is_some_and(|colors| colors.len() != self.positions.len())
        {
            self.colors = None;
        }
        Ok(self)
    }
}
