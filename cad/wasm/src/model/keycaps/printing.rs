//! One connected material mesh per printed keycap; display socket fans are excluded.
use super::{empty_mesh, mesh_template::add_triangle, mesh_template::KeycapMeshTemplate, MeshData};
use manifold_rust::{
    manifold::Manifold,
    types::{BooleanEngine, Error, MeshGL64},
};

pub(super) fn mesh(template: &KeycapMeshTemplate) -> Result<MeshData, String> {
    let mut result = import(&template.shell)?;
    for operand in std::iter::once(&template.stem).chain(template.legend.iter()) {
        result = result.union_with_engine(&import(operand)?, BooleanEngine::Robust);
        if result.status() != Error::NoError || result.num_tri() == 0 {
            return Err(format!("Mesh union failed: {:?}", result.status()));
        }
    }
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
