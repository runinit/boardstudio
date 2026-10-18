//! One connected material mesh per printed keycap; display socket fans are excluded.
use super::{
    empty_mesh, lettering, mesh_template::add_triangle, mesh_template::KeycapMeshTemplate, MeshData,
};
use boardstudio_contracts::KeycapSpec;
use manifold_rust::{
    linalg::Vec2,
    manifold::Manifold,
    types::{BooleanEngine, Error, MeshGL64, Polygons},
};

/// Edges and collinear vertices within 10 nm are merged. This is far below print resolution and
/// f32 render precision, and removes zero-area triangles where a thin legend grazes a dished roof.
const SIMPLIFY_TOLERANCE_MM: f64 = 1e-5;

const LEGEND_EDGE_MM: f64 = 0.4;

pub(super) fn mesh(template: &KeycapMeshTemplate, spec: &KeycapSpec) -> Result<MeshData, String> {
    let mut result = import(&template.shell)?;
    let operands = [Some(import(&template.stem)?), legend_solid(spec)?];
    for operand in operands.iter().flatten() {
        result = result.union_with_engine(operand, BooleanEngine::Robust);
        if result.status() != Error::NoError || result.num_tri() == 0 {
            return Err(format!("Mesh union failed: {:?}", result.status()));
        }
    }
    result = result.simplify(SIMPLIFY_TOLERANCE_MM);
    if result.decompose().len() != 1 {
        return Err("Printable keycap contains disconnected material".into());
    }
    let indexed = result.get_mesh_gl64(-1);
    let mut output = empty_mesh();
    for triangle in indexed.tri_verts.as_chunks::<3>().0 {
        let points = [0, 1, 2].map(|i| {
            let base = triangle[i] as usize * indexed.num_prop as usize;
            [
                indexed.vert_properties[base],
                indexed.vert_properties[base + 1],
                indexed.vert_properties[base + 2],
            ]
        });
        add_triangle(&mut output, points)?;
    }
    Ok(output)
}

/// The legend as an extruded outline whose top and bottom follow the roof. The display legend's
/// triangulated caps leave T-junctions at collinear outline points, so it is not a closed operand.
fn legend_solid(spec: &KeycapSpec) -> Result<Option<Manifold>, String> {
    let Some(contours) = lettering::print_contours(spec)? else {
        return Ok(None);
    };
    let polygons: Polygons = contours
        .iter()
        .map(|contour| contour.iter().map(|p| Vec2::new(p[0], p[1])).collect())
        .collect();
    let prism = Manifold::extrude(&polygons, 1.0, 0, 0.0, Vec2::new(1.0, 1.0));
    if prism.status() != Error::NoError || prism.num_tri() == 0 {
        return Err(format!("Invalid legend outline: {:?}", prism.status()));
    }
    // Chords of the outline's coarse caps would stand up to a few tenths of a millimetre above a
    // dished roof; edges of at most 0.4 mm keep the 0.03 mm relief within the roof's own chord error.
    let solid = prism.refine_to_length(LEGEND_EDGE_MM).warp(|vertex| {
        let roof = lettering::roof_z(spec, vertex.x, vertex.y);
        vertex.z = if vertex.z > 0.5 {
            roof + lettering::RAISE_MM
        } else {
            roof - lettering::EMBED_MM
        };
    });
    if solid.status() != Error::NoError || solid.num_tri() == 0 {
        return Err(format!("Invalid legend solid: {:?}", solid.status()));
    }
    Ok(Some(solid))
}

fn import(mesh: &MeshData) -> Result<Manifold, String> {
    if mesh.positions.is_empty()
        || !mesh.positions.len().is_multiple_of(9)
        || mesh.positions.iter().any(|value| !value.is_finite())
    {
        return Err("Print operand must contain finite complete triangles".into());
    }
    let indexed = MeshGL64 {
        num_prop: 3,
        vert_properties: mesh.positions.iter().copied().map(f64::from).collect(),
        tri_verts: (0..mesh.positions.len() as u64 / 3).collect(),
        ..MeshGL64::default()
    };
    let solid = Manifold::from_mesh_gl64_robust(&indexed);
    if solid.status() != Error::NoError || solid.num_tri() == 0 {
        return Err(format!("Invalid print operand: {:?}", solid.status()));
    }
    Ok(solid)
}
