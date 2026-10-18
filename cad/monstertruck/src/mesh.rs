use monstertruck_meshing::rexport_polymesh::PolygonMesh;
use monstertruck_modeling::{InnerSpace, Point3};

/// Triangle-soup render buffers in millimeters, matching the CAD worker protocol.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshBuffers {
    pub positions: Vec<f32>,
    pub normals: Vec<f32>,
}

/// Checks edge incidence using the caller's coordinate identity policy.
pub(crate) fn closed_oriented_edges<K: Copy + Ord + std::hash::Hash>(
    triangles: &[[Point3; 3]],
    point_key: impl Fn(Point3) -> Option<K>,
) -> bool {
    let mut counts = std::collections::HashMap::new();
    for triangle in triangles {
        for i in 0..3 {
            let (Some(a), Some(b)) = (point_key(triangle[i]), point_key(triangle[(i + 1) % 3]))
            else {
                return false;
            };
            if a == b {
                return false;
            }
            let (key, direction) = if a < b { ((a, b), 1i64) } else { ((b, a), -1) };
            let (count, orientation) = counts.entry(key).or_insert((0, 0));
            *count += 1;
            *orientation += direction;
        }
    }
    !counts.is_empty()
        && counts
            .values()
            .all(|&(count, orientation)| count == 2 && orientation == 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_incidence_requires_a_closed_consistently_oriented_surface() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let triangles = [[a, c, b], [a, b, d], [a, d, c], [b, c, d]];
        let key = |p: Point3| Some([p.x.to_bits(), p.y.to_bits(), p.z.to_bits()]);
        assert!(closed_oriented_edges(&triangles, key));
        assert!(!closed_oriented_edges(&triangles[..3], key));
        let mut reversed = triangles;
        reversed[0].swap(0, 1);
        assert!(!closed_oriented_edges(&reversed, key));
        let mut nonmanifold = triangles.to_vec();
        nonmanifold.push(triangles[0]);
        assert!(!closed_oriented_edges(&nonmanifold, key));
        assert!(!closed_oriented_edges(&[[a, a, b]], key));
        assert!(!closed_oriented_edges(&[], key));
    }
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
