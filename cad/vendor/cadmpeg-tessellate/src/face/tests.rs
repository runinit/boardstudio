// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeSet;

use cadmpeg_core::decode::WorkBudget;
use cadmpeg_ir::index::ModelIndex;
use cadmpeg_ir::math::{Point3, Vector3};

use super::finish;
use crate::refine::Mesh;
use crate::surface::Carrier;

#[test]
fn surface_normals_follow_the_winding_without_losing_their_direction() {
    let ir = cadmpeg_ir::examples::unit_cube().unwrap();
    let index = ModelIndex::new_model_only(&ir);
    let work = WorkBudget::new(1000);
    let carrier = Carrier::new(&index, &work, &ir.model.faces[0].surface).unwrap();
    let mesh = Mesh {
        uv: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        points: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
        ],
        normals: vec![Some(Vector3::new(0.6, 0.0, -0.8)); 3],
        scales: vec![[1.0, 1.0]; 3],
        triangles: vec![Some([0, 1, 2])],
        fixed: BTreeSet::new(),
    };
    let result = finish(&carrier, mesh, 1.0, Vec::new(), 0.0);
    for normal in result.normals {
        assert!((normal.x + 0.6).abs() < 1e-12);
        assert!((normal.z - 0.8).abs() < 1e-12);
    }
}

#[test]
fn finish_drops_sub_ulp_pole_slivers_and_keeps_valid_triangles() {
    let ir = cadmpeg_ir::examples::unit_cube().unwrap();
    let index = ModelIndex::new_model_only(&ir);
    let work = WorkBudget::new(1000);
    let carrier = Carrier::new(&index, &work, &ir.model.faces[0].surface).unwrap();
    let mesh = Mesh {
        uv: vec![[0.0, 0.0]; 6],
        points: vec![
            Point3::new(0.0, 0.0, 0.0),
            Point3::new(1.0, 0.0, 0.0),
            Point3::new(0.0, 1.0, 0.0),
            // 1-ULP sphere-pole triangle observed on SW_Cherry_MX_PCB.stp (#9831)
            Point3::new(-4.507086614173, -6.707086614173, 0.542519685039),
            Point3::new(-4.507086614173, -6.707086614173, 0.5425196850390002),
            Point3::new(-4.622095445900555, -6.688837550823168, 0.45120256544297105),
        ],
        normals: vec![None; 6],
        scales: vec![[1.0, 1.0]; 6],
        triangles: vec![Some([0, 1, 2]), Some([3, 4, 5])],
        fixed: BTreeSet::new(),
    };
    let result = finish(&carrier, mesh, 1.0, Vec::new(), 0.0);
    assert_eq!(result.triangles.len(), 1);
    assert_eq!(result.triangles[0], [0, 1, 2]);
}

#[test]
fn finish_retains_thin_real_slivers_above_roundoff_floor() {
    let ir = cadmpeg_ir::examples::unit_cube().unwrap();
    let index = ModelIndex::new_model_only(&ir);
    let work = WorkBudget::new(1000);
    let carrier = Carrier::new(&index, &work, &ir.model.faces[0].surface).unwrap();
    let mesh = Mesh {
        uv: vec![[0.0, 0.0]; 3],
        points: vec![
            // Real circular-segment sliver (altitude ~1.59e-8 mm at scale ~3.18 mm)
            Point3::new(3.18045526059587, -0.000318045527119739, 0.0),
            Point3::new(3.18045526059587, 0.000318045527119739, 0.0),
            Point3::new(3.18045527649815, 0.0, 0.0),
        ],
        normals: vec![None; 3],
        scales: vec![[1.0, 1.0]; 3],
        triangles: vec![Some([0, 1, 2])],
        fixed: BTreeSet::new(),
    };
    let result = finish(&carrier, mesh, 1.0, Vec::new(), 0.0);
    assert_eq!(result.triangles.len(), 1);
}

#[test]
fn finish_retains_long_edge_cone_ruling_sliver() {
    let ir = cadmpeg_ir::examples::unit_cube().unwrap();
    let index = ModelIndex::new_model_only(&ir);
    let work = WorkBudget::new(1000);
    let carrier = Carrier::new(&index, &work, &ir.model.faces[0].surface).unwrap();
    let mesh = Mesh {
        uv: vec![[0.0, 0.0]; 3],
        points: vec![
            // Real cone-ruling sliver from occt-cone-apex.step @ 0.01 mm (altitude/scale ~1.89e-13)
            Point3::new(3.0, -2.0, 12.0),
            Point3::new(4.26374272336019, -2.026736070268468, 10.419968113303575),
            Point3::new(3.2639664419414243, -2.005584542810659, 11.669968113303263),
        ],
        normals: vec![None; 3],
        scales: vec![[1.0, 1.0]; 3],
        triangles: vec![Some([0, 1, 2])],
        fixed: BTreeSet::new(),
    };
    let result = finish(&carrier, mesh, 1.0, Vec::new(), 0.0);
    assert_eq!(result.triangles.len(), 1);
}
