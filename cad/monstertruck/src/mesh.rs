use monstertruck_meshing::rexport_polymesh::PolygonMesh;
use monstertruck_modeling::{InnerSpace, Point3};

/// Triangle-soup render buffers in millimeters, matching the CAD worker protocol.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshBuffers {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

pub(crate) fn mesh_buffers(
    meshes: impl IntoIterator<Item = PolygonMesh>,
) -> Result<MeshBuffers, String> {
    let mut buffers = MeshBuffers::default();
    for mesh in meshes {
        for triangle in mesh.faces().triangle_iter() {
            let points = triangle.map(|vertex| mesh.positions()[vertex.pos]);
            push_triangle(&mut buffers, points)?;
        }
    }
    if buffers.positions.is_empty() {
        return Err("Monstertruck produced an empty mesh".into());
    }
    Ok(buffers)
}

pub(crate) fn push_triangle(buffers: &mut MeshBuffers, points: [Point3; 3]) -> Result<(), String> {
    let normal = (points[1] - points[0]).cross(points[2] - points[0]);
    let length = normal.magnitude();
    if !length.is_finite() || length == 0.0 {
        return Err("Monstertruck produced a degenerate mesh triangle".into());
    }
    let rendered = points.map(|point| {
        Point3::new(
            f64::from(point.x as f32),
            f64::from(point.y as f32),
            f64::from(point.z as f32),
        )
    });
    if rendered
        .iter()
        .any(|point| !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite())
    {
        return Err("Monstertruck mesh contains a coordinate outside the render range".into());
    }
    let rendered_normal = (rendered[1] - rendered[0]).cross(rendered[2] - rendered[0]);
    if rendered_normal.dot(normal) <= 0.0 {
        return Err(
            "Monstertruck mesh triangle loses its orientation at f32 render precision".into(),
        );
    }
    let normal = normal / length;
    for point in points {
        push_point(&mut buffers.positions, point)?;
        push_point(
            &mut buffers.normals,
            Point3::new(normal.x, normal.y, normal.z),
        )?;
    }
    Ok(())
}

fn push_point(output: &mut Vec<f32>, point: Point3) -> Result<(), String> {
    for coordinate in [point.x, point.y, point.z] {
        let coordinate = coordinate as f32;
        if !coordinate.is_finite() {
            return Err("Monstertruck mesh contains a coordinate outside the render range".into());
        }
        output.push(coordinate);
    }
    Ok(())
}
