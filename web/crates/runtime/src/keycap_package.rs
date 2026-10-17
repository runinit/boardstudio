//! Per-keycap STL files and their source identities for the existing ZIP exporter.
use crate::mechanical_package::{
    mechanical_filename_component, mechanical_stl, push_mechanical_file,
};
use boardstudio_web_host::cad_jobs::{CadMesh, CadResult};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn files(result: CadResult) -> Result<Vec<(String, Vec<u8>)>, String> {
    if result.bodies.is_empty() || !result.step.is_empty() {
        return Err("Keycap printing requires fused mesh bodies without STEP data.".into());
    }
    let mut files = Vec::new();
    let mut paths = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut manifest = Vec::new();
    for (index, body) in result.bodies.into_iter().enumerate() {
        if !body.id.starts_with("keycap:")
            || body.id == "keycap:"
            || !ids.insert(body.id.clone())
            || body.name.trim().is_empty()
            || body.positions.is_empty()
            || body
                .positions
                .iter()
                .chain(&body.normals)
                .any(|x| !x.is_finite())
        {
            return Err("Keycap print bodies are missing, duplicated or malformed.".into());
        }
        let path = format!(
            "keycaps/{:04}-{}.stl",
            index + 1,
            mechanical_filename_component(&body.name)
        );
        manifest.push(serde_json::json!({"id":body.id,"name":body.name,"file":path}));
        let mesh = CadMesh {
            positions: body.positions,
            normals: body.normals,
        };
        verify_print_mesh(&mesh)?;
        push_mechanical_file(&mut files, &mut paths, path, mechanical_stl(&mesh)?)?;
    }
    push_mechanical_file(
        &mut files,
        &mut paths,
        "keycaps.json".into(),
        serde_json::to_vec_pretty(
            &serde_json::json!({"revision":result.revision,"units":"mm","keycaps":manifest}),
        )
        .map_err(|error| error.to_string())?,
    )?;
    Ok(files)
}

fn verify_print_mesh(mesh: &CadMesh) -> Result<(), String> {
    let bad = || "Keycap STL requires one closed, outward-oriented material mesh.".to_owned();
    if !mesh.positions.len().is_multiple_of(9) || mesh.positions.len() != mesh.normals.len() {
        return Err(bad());
    }
    let origin = [mesh.positions[0], mesh.positions[1], mesh.positions[2]].map(f64::from);
    let mut edges = BTreeMap::new();
    let mut vertex_stars = BTreeMap::new();
    let key = |point: [f32; 3]| point.map(|x| if x == 0.0 { 0 } else { x.to_bits() });
    let mut volume = 0.0;
    for (index, triangle) in mesh.positions.as_chunks::<9>().0.iter().enumerate() {
        let points = [0, 1, 2].map(|i| [triangle[3 * i], triangle[3 * i + 1], triangle[3 * i + 2]]);
        let keys = points.map(key);
        for edge in 0..3 {
            let a = keys[edge];
            let b = keys[(edge + 1) % 3];
            if a == b || edges.insert((a, b), (index, edge)).is_some() {
                return Err(bad());
            }
            vertex_stars.entry(a).or_insert((b, 0usize)).1 += 1;
        }
        let [a, b, c] = points.map(|p| {
            [
                f64::from(p[0]) - origin[0],
                f64::from(p[1]) - origin[1],
                f64::from(p[2]) - origin[2],
            ]
        });
        volume += a[0] * (b[1] * c[2] - b[2] * c[1])
            + a[1] * (b[2] * c[0] - b[0] * c[2])
            + a[2] * (b[0] * c[1] - b[1] * c[0]);
    }
    if !volume.is_finite() || volume <= 0.0 {
        return Err(bad());
    }
    let count = mesh.positions.len() / 9;
    let mut neighbours = vec![[0; 3]; count];
    for (&(a, b), &(triangle, edge)) in &edges {
        let &(other, _) = edges.get(&(b, a)).ok_or_else(bad)?;
        neighbours[triangle][edge] = other;
    }
    // A closed edge graph can still have two surface fans pinched at one vertex.
    for (&vertex, &(first, degree)) in &vertex_stars {
        let mut next = first;
        for step in 0..degree {
            let &(triangle, edge) = edges.get(&(vertex, next)).ok_or_else(bad)?;
            let base = triangle * 9 + ((edge + 2) % 3) * 3;
            next = key([
                mesh.positions[base],
                mesh.positions[base + 1],
                mesh.positions[base + 2],
            ]);
            if next == first && step + 1 != degree {
                return Err(bad());
            }
        }
        if next != first {
            return Err(bad());
        }
    }
    let mut seen = vec![false; count];
    let mut pending = vec![0];
    while let Some(triangle) = pending.pop() {
        if !seen[triangle] {
            seen[triangle] = true;
            pending.extend(neighbours[triangle]);
        }
    }
    if seen.iter().any(|seen| !seen) {
        return Err(bad());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use boardstudio_web_host::cad_jobs::CadBodyMesh;

    #[test]
    fn printable_package_retains_ids_units_and_binary_stl_triangles() {
        let result = CadResult {
            revision: 7,
            bodies: vec![CadBodyMesh {
                id: "keycap:key-1".into(),
                name: "SW/1".into(),
                positions: vec![
                    0., 0., 0., 0., 1., 0., 1., 0., 0., 0., 0., 0., 1., 0., 0., 0., 0., 1., 0., 0.,
                    0., 0., 0., 1., 0., 1., 0., 1., 0., 0., 0., 1., 0., 0., 0., 1.,
                ],
                normals: vec![
                    0., 0., -1., 0., 0., -1., 0., 0., -1., 0., -1., 0., 0., -1., 0., 0., -1., 0.,
                    -1., 0., 0., -1., 0., 0., -1., 0., 0., 0.57735026, 0.57735026, 0.57735026,
                    0.57735026, 0.57735026, 0.57735026, 0.57735026, 0.57735026, 0.57735026,
                ],
            }],
            ..CadResult::default()
        };
        let mut open = result.clone();
        open.bodies[0].positions.truncate(9);
        open.bodies[0].normals.truncate(9);
        assert!(
            files(open).is_err(),
            "open preview triangles must not become printable STL"
        );
        let mut disconnected = result.clone();
        let positions = disconnected.bodies[0]
            .positions
            .iter()
            .enumerate()
            .map(|(i, x)| if i % 3 == 0 { x + 10.0 } else { *x })
            .collect::<Vec<_>>();
        disconnected.bodies[0].positions.extend(positions);
        let normals = disconnected.bodies[0].normals.clone();
        disconnected.bodies[0].normals.extend(normals);
        assert!(
            files(disconnected).is_err(),
            "separate closed display components are not one print body"
        );
        let output = files(result).unwrap();
        assert_eq!(output.len(), 2);
        assert_eq!(output[0].0, "keycaps/0001-SW-1.stl");
        let bytes = &output[0].1;
        assert_eq!(
            bytes.len(),
            284,
            "84-byte header plus four 50-byte triangles"
        );
        assert_eq!(u32::from_le_bytes(bytes[80..84].try_into().unwrap()), 4);
        assert_eq!(f32::from_le_bytes(bytes[96..100].try_into().unwrap()), 0.0);
        assert_eq!(f32::from_le_bytes(bytes[112..116].try_into().unwrap()), 1.0);
        let manifest: serde_json::Value = serde_json::from_slice(&output[1].1).unwrap();
        assert_eq!(manifest["revision"], 7);
        assert_eq!(manifest["units"], "mm");
        assert_eq!(manifest["keycaps"][0]["id"], "keycap:key-1");
        assert_eq!(manifest["keycaps"][0]["file"], output[0].0);
        assert!(files(CadResult::default()).is_err());
    }
}
