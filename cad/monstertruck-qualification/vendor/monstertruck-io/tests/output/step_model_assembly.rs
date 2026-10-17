//! Public StepModel/StepDesign assembly regression for every indexed source path.

use monstertruck_assembly::assy::{Assembly, EdgeEntity, NodeEntity};
use monstertruck_io::step::{
    common::PartAttributes,
    load::Table,
    save::{CompleteStepDisplay, StepDesign, StepHeaderDescriptor, StepModel},
};
use monstertruck_modeling::*;
use std::collections::{HashMap, HashSet};

type WriterShape<'a> = StepModel<'a, Point3, Curve, Surface>;

fn cylinder(height: f64, radius: f64) -> Solid {
    let vertex = builder::vertex(Point3::new(0.0, -height / 2.0, radius));
    let circle = builder::revolve(
        &vertex,
        Point3::origin(),
        Vector3::unit_y(),
        builder::SweepAngle::Closed,
        2,
    );
    let disk = builder::try_attach_plane(&[circle]).unwrap();
    builder::extrude(&disk, Vector3::new(0.0, height, 0.0))
}

fn attrs(id: &str) -> PartAttributes {
    PartAttributes {
        id: id.to_owned(),
        name: format!("{id} name"),
        description: format!("{id} test node"),
        shape_representation: None,
    }
}

#[test]
fn assembly_indexes_all_step_model_variants_and_preserves_shape_grouping() {
    let source = cylinder(2.0, 0.75);
    let solid = source.compress();
    let shell = solid.boundaries[0].clone();
    let trimmed = source.compress_with_parameter_curves(0.01);
    let trimmed_shell = trimmed.boundaries[0].clone();

    // One assembly forces each StepModel variant to compute both its actual
    // assigned entity index and its matching StepLength after prior shapes.
    let shapes = [
        ("solid", StepModel::from(&solid), true),
        ("shell", StepModel::from(&shell), false),
        ("trimmed-solid", StepModel::from(&trimmed), true),
        ("trimmed-shell", StepModel::from(&trimmed_shell), false),
        ("curve3d-solid", StepModel::from_curve3d_only_solid(&solid), true),
        ("curve3d-shell", StepModel::from_curve3d_only_shell(&shell), false),
    ];

    let expected: HashMap<&str, bool> = shapes
        .iter()
        .map(|(id, _, is_solid)| (*id, *is_solid))
        .collect();

    let mut assembly: Assembly<Vec<WriterShape<'_>>, PartAttributes, Matrix4, PartAttributes> =
        Assembly::new();
    let mut previous = None;
    for (index, (id, shape, _is_solid)) in shapes.into_iter().enumerate() {
        let node = assembly.create_node(NodeEntity {
            shape: vec![shape],
            attrs: attrs(id),
        });
        if let Some(parent) = previous {
            assembly.create_edge(
                parent,
                node,
                EdgeEntity {
                    matrix: Matrix4::from_translation(Vector3::new(index as f64, 0.0, 0.0)),
                    attrs: attrs(&format!("edge-{index}")),
                },
            );
        }
        previous = Some(node);
    }

    let design = StepDesign::<WriterShape<'_>, _, Matrix4>::new(assembly);
    let step = CompleteStepDisplay::new(design, StepHeaderDescriptor::default()).to_string();
    let table = Table::from_step(&step).expect("public StepDesign output must parse");
    table.entity_report.require_empty().expect("all emitted entities should load");
    assert!(table.pcurve.len() > 0, "trimmed StepModel paths should retain pcurves");
    assert!(table.surface_curve.len() > 0, "ordinary StepModel paths should retain surface associations");
    assert_eq!(table.manifold_solid_brep.len(), 3);
    assert_eq!(table.shell_based_surface_model.len(), 3);

    let parsed = table.step_assy().expect("written product graph must load");
    assert_eq!(parsed.len(), 6);
    assert_eq!(
        parsed.all_nodes().map(|node| node.edges().count()).sum::<usize>(),
        5,
        "assembly edges must survive after shape entity counts are advanced",
    );

    let mut actual_ids = HashSet::new();
    for node in parsed.all_nodes() {
        let id = node.attributes().id.as_str();
        actual_ids.insert(id);
        let expected_solid = expected[id];
        let solid_roots = node.shape().iter().filter(|root| table.manifold_solid_brep.contains_key(root)).count();
        let shell_roots = node.shape().iter().filter(|root| table.shell_based_surface_model.contains_key(root)).count();
        assert_eq!(solid_roots, usize::from(expected_solid), "wrong solid root grouping for {id}");
        assert_eq!(shell_roots, usize::from(!expected_solid), "wrong shell root grouping for {id}");
    }
    assert_eq!(actual_ids, expected.keys().copied().collect());
}

#[test]
fn curve3d_only_step_models_omit_surface_and_pcurve_associations() {
    let source = cylinder(2.0, 0.75);
    let solid = source.compress();
    let shell = solid.boundaries[0].clone();

    let solid_step = CompleteStepDisplay::new(
        StepDesign::from_model(StepModel::from_curve3d_only_solid(&solid)),
        StepHeaderDescriptor::default(),
    )
    .to_string();
    let shell_step = CompleteStepDisplay::new(
        StepDesign::from_model(StepModel::from_curve3d_only_shell(&shell)),
        StepHeaderDescriptor::default(),
    )
    .to_string();

    for (label, step) in [("solid", solid_step), ("shell", shell_step)] {
        let table = Table::from_step(&step).expect("curve3d-only StepDesign should parse");
        table.entity_report.require_empty().expect("all curve3d entities should load");
        assert!(!table.edge_curve.is_empty(), "{label} should emit 3D edge curves");
        assert!(table.surface_curve.is_empty(), "{label} should not attach curves to surfaces");
        assert!(table.pcurve.is_empty(), "{label} should not emit pcurve associations");
    }
}
