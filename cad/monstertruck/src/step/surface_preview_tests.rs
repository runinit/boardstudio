//! Complete source surfaces are not automatically eligible solid boundaries.
use super::*;
use std::collections::{BTreeMap, BTreeSet};

const BOXES: &[u8] = include_bytes!("../../tests/fixtures/touching-unit-boxes.step");
const PANASONIC: &[u8] = include_bytes!(
    "../../../../ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step"
);

// Walk original records, independently of compressed-shell incidence or meshing.
fn census(bytes: &[u8], shell: u64) -> BTreeMap<u64, Vec<(u64, bool)>> {
    let original = parser::parse(std::str::from_utf8(bytes).unwrap()).unwrap();
    let records = source_records(&original);
    let params = |id| {
        let [record] = records[&id].as_slice() else {
            panic!("simple fixture record")
        };
        let Parameter::List(params) = &record.parameter else {
            panic!("record parameters")
        };
        params
    };
    let list = |p: &Parameter| {
        let Parameter::List(items) = p else {
            panic!("reference list")
        };
        items
            .iter()
            .map(|p| reference(p).unwrap())
            .collect::<Vec<_>>()
    };
    let flag = |p: &Parameter| match p {
        Parameter::Enumeration(value) if value == "T" => true,
        Parameter::Enumeration(value) if value == "F" => false,
        _ => panic!("source orientation"),
    };
    let faces = list(&params(shell)[1]);
    assert_eq!(faces.len(), faces.iter().collect::<BTreeSet<_>>().len());
    let mut uses = BTreeMap::<_, Vec<_>>::new();
    for face in faces {
        for bound in list(&params(face)[1]) {
            let bound = params(bound);
            let edges = list(&params(reference(&bound[1]).unwrap())[1]);
            let mut vertices = Vec::new();
            for oriented in edges {
                let oriented = params(oriented);
                let edge = reference(&oriented[3]).unwrap();
                let curve = params(edge);
                let pair = [reference(&curve[1]).unwrap(), reference(&curve[2]).unwrap()];
                vertices.push(if flag(&oriented[4]) {
                    pair
                } else {
                    [pair[1], pair[0]]
                });
                uses.entry(edge)
                    .or_default()
                    .push((face, flag(&oriented[4]) == flag(&bound[2])));
            }
            assert!(!vertices.is_empty());
            assert!(
                vertices
                    .iter()
                    .zip(vertices.iter().cycle().skip(1))
                    .all(|(a, b)| a[1] == b[0])
            );
        }
    }
    assert_eq!(
        uses.len(),
        records
            .values()
            .filter(|r| r.iter().any(|r| r.name == "EDGE_CURVE"))
            .count()
    );
    uses
}

fn replace_once(source: &[u8], before: &str, after: &str) -> Vec<u8> {
    let text = std::str::from_utf8(source).unwrap();
    assert_eq!(text.matches(before).count(), 1);
    text.replacen(before, after, 1).into_bytes()
}

#[test]
fn incomplete_unresolved_repeated_and_unbalanced_surfaces_still_refuse() {
    for (before, after) in [
        ("(#1000,#1001,#1002", "(#1001,#1002"),
        ("(#1300,#1301,#1302,#1303)", "(#1301,#1302,#1303)"),
        (
            "#1300=ORIENTED_EDGE('',*,*,#200,.T.);",
            "#1300=ORIENTED_EDGE('',*,*,#9999,.T.);",
        ),
        ("(#1000,#1001,#1002", "(#1000,#1000,#1001,#1002"),
        (
            "#200=EDGE_CURVE('',#101,#102,#300,.T.);",
            "#200=EDGE_CURVE('',#101,#103,#300,.T.);",
        ),
        (
            "#1101=FACE_OUTER_BOUND('',#1201,.T.);",
            "#1101=FACE_OUTER_BOUND('',#1201,.F.);",
        ),
    ] {
        assert!(
            import_step(replace_once(BOXES, before, after)).is_err(),
            "must refuse {before}"
        );
    }
}

#[test]
fn six_uses_are_not_admitted_merely_because_the_count_is_even() {
    let text = std::str::from_utf8(BOXES).unwrap();
    let mut extra = String::new();
    for id in 1000..1006 {
        let record = text
            .lines()
            .find(|line| line.starts_with(&format!("#{id}=")))
            .unwrap();
        extra.push_str(&record.replacen(&format!("#{id}="), &format!("#{}=", id + 2000), 1));
        extra.push('\n');
    }
    let source = replace_once(
        BOXES,
        "#1011));",
        "#1011,#3000,#3001,#3002,#3003,#3004,#3005));",
    );
    let source = replace_once(
        &source,
        "ENDSEC;END-ISO-10303-21;",
        &(extra + "ENDSEC;END-ISO-10303-21;"),
    );
    assert_eq!(census(&source, 2000)[&205].len(), 6);
    assert!(import_step(source).is_err());
}

#[test]
fn repeated_placements_classify_the_definition_shell_not_aggregated_uses() {
    let source = replace_once(
        BOXES,
        "ENDSEC;END-ISO-10303-21;",
        "#2101=DIRECTION('',(0.,0.,1.));\n#2102=DIRECTION('',(1.,0.,0.));\n#2105=DIRECTION('',(0.,1.,0.));\n#2100=AXIS2_PLACEMENT_3D('',#1,#2101,#2102);\n#2103=REPRESENTATION_MAP(#2100,#2004);\n#2106=CARTESIAN_POINT('',(20.,0.,0.));\n#2109=CARTESIAN_POINT('',(40.,0.,0.));\n#2104=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',#2102,#2105,#2106,1.,#2101);\n#2108=CARTESIAN_TRANSFORMATION_OPERATOR_3D('','',#2102,#2105,#2109,1.,#2101);\n#2107=MAPPED_ITEM('',#2103,#2104);\n#2110=MAPPED_ITEM('',#2103,#2108);\n#2111=SHAPE_REPRESENTATION('',(#2107,#2110),#2003);\nENDSEC;END-ISO-10303-21;",
    );
    let model = import_step(source.clone()).unwrap();
    assert_eq!(model.original_bytes(), source);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 2);
    assert_eq!(model.source_faces().len(), 12);
    assert_eq!(model.face_occurrences().len(), 24);
    assert_eq!(model.bounds(), [[20., 0., 0.], [42., 2., 1.]]);
    assert!(!model.accuracy().mesh_closed_oriented);
    assert_eq!(
        model
            .triangle_faces()
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .len(),
        24
    );
    for occurrence in 0..2 {
        assert_eq!(
            model
                .face_occurrences()
                .iter()
                .filter(|face| face.occurrence == occurrence)
                .count(),
            12
        );
    }
}

fn assert_complete(model: &ImportedStep, bytes: &[u8], count: usize) {
    assert_eq!(model.original_bytes(), bytes);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1);
    assert_eq!(model.source_faces().len(), count);
    assert_eq!(model.face_occurrences().len(), count);
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
    assert_eq!(model.accuracy().max_repair_displacement_mm, None);
    assert!(!model.accuracy().mesh_closed_oriented);
    assert_eq!(model.line_presentation().occurrences().len(), 0);
    let mut areas = vec![0.; count];
    for (triangle, &tag) in model
        .preview_positions_f64()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(model.triangle_faces())
    {
        let [a, b, c] = triangle.map(Point3::from);
        let area = (b - a).cross(c - a).magnitude() / 2.;
        assert!(area.is_finite() && area > 0.);
        areas[tag] += area;
    }
    assert!(areas.iter().all(|area| *area > 0.));
    assert_eq!(
        model.preview_positions_f64().len(),
        model.triangle_faces().len() * 3
    );
    assert_eq!(
        model.mesh().positions.len(),
        model.preview_positions_f64().len() * 3
    );
    for (&source, &render) in model
        .preview_positions_f64()
        .iter()
        .flatten()
        .zip(&model.mesh().positions)
    {
        assert!(source.is_finite());
        assert_eq!(source as f32, render);
    }
}

#[test]
fn touching_boxes_preserve_twelve_analytic_faces_without_solid_eligibility() {
    let uses = census(BOXES, 2000);
    assert_eq!(uses.values().filter(|u| u.len() == 2).count(), 22);
    assert_eq!(
        uses[&205].iter().map(|u| u.0).collect::<Vec<_>>(),
        [1001, 1003, 1006, 1008]
    );
    assert_eq!(uses[&205].iter().filter(|u| u.1).count(), 2);
    let model = import_step(BOXES.to_vec()).expect("complete touching-box surfaces must preview");
    assert_complete(&model, BOXES, 12);
    assert_eq!(model.bounds(), [[0., 0., 0.], [2., 2., 1.]]);
    assert_eq!(
        model
            .source_faces()
            .iter()
            .map(|face| face.face_id)
            .collect::<BTreeSet<_>>(),
        (1000..1012).collect()
    );
    let mut areas = [0.; 12];
    for (triangle, &tag) in model
        .preview_positions_f64()
        .as_chunks::<3>()
        .0
        .iter()
        .zip(model.triangle_faces())
    {
        let [a, b, c] = triangle.map(Point3::from);
        let source = &model.source_faces()[model.face_occurrences()[tag].source_face];
        areas[(source.face_id - 1000) as usize] += (b - a).cross(c - a).magnitude() / 2.;
    }
    assert!(
        areas.into_iter().all(|area| (area - 1.).abs() < 1e-12),
        "each independently authored unit-box face has area one"
    );
}

#[test]
fn original_panasonic_preserves_every_face_and_boundary_line() {
    assert_eq!(PANASONIC.len(), 123401);
    let uses = census(PANASONIC, 16);
    assert_eq!(uses.values().filter(|u| u.len() == 2).count(), 425);
    let four: BTreeMap<_, _> = uses
        .iter()
        .filter(|(_, u)| u.len() == 4)
        .map(|(&id, u)| (id, u.iter().map(|u| u.0).collect::<BTreeSet<_>>()))
        .collect();
    assert_eq!(
        four,
        BTreeMap::from([
            (452, BTreeSet::from([443, 459, 1098, 1953])),
            (464, BTreeSet::from([459, 474, 1295, 1967])),
            (1050, BTreeSet::from([1046, 1650, 1664, 1782])),
            (1348, BTreeSet::from([1341, 1450, 1460, 2136]))
        ])
    );
    assert!(
        uses.values()
            .filter(|u| u.len() == 4)
            .all(|u| u.iter().filter(|u| u.1).count() == 2)
    );
    let model =
        import_step(PANASONIC.to_vec()).expect("complete original Panasonic surfaces must preview");
    assert_complete(&model, PANASONIC, 155);
    let original = parser::parse(std::str::from_utf8(PANASONIC).unwrap()).unwrap();
    let records = source_records(&original);
    let retained = source_records(&model._original_source);
    assert_eq!(
        model.solids[&15].source[&10].source().shells()[0]
            .shell()
            .edges
            .len(),
        429
    );
    let expected: BTreeSet<_> = records
        .iter()
        .filter(|(_, records)| records.iter().any(|record| record.name == "ADVANCED_FACE"))
        .map(|(&id, _)| id)
        .collect();
    assert_eq!(
        model
            .source_faces()
            .iter()
            .map(|face| face.face_id)
            .collect::<BTreeSet<_>>(),
        expected
    );
    let mut lines = 0;
    for (&id, records) in &records {
        if records.iter().any(|record| record.name == "LINE") {
            assert_eq!(
                format!("{:?}", retained[&id]),
                format!("{records:?}"),
                "original LINE #{id} geometry/frame"
            );
            lines += 1;
        }
    }
    assert_eq!(
        lines, 353,
        "boundary LINE definitions are not display segments"
    );
    for (actual, expected) in model
        .bounds()
        .into_iter()
        .flatten()
        .zip([-3.2, -1.75, -0.5, 3.2, 2.75, 1.65])
    {
        assert!(
            (actual - expected).abs() < 1e-6,
            "source bounds {actual} vs {expected}"
        );
    }
}
