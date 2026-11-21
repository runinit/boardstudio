use boardstudio_cad::import_step;

const BRANCHING: &str = include_str!("fixtures/mapped-over-budget.step");

fn replace_once(source: &str, before: &str, after: &str) -> String {
    assert_eq!(source.matches(before).count(), 1);
    source.replacen(before, after, 1)
}

#[test]
fn branching_mapped_occurrences_refuse_before_geometry() {
    // Eleven binary mapping levels expand this small source to 2,048 occurrences.
    let error = import_step(include_bytes!("fixtures/mapped-over-budget.step").to_vec())
        .err()
        .expect("over-budget mapped occurrences must refuse");
    assert!(
        error.contains("STEP occurrence limit exceeded (1024 occurrences)"),
        "{error}"
    );
}

#[test]
fn occurrence_budget_is_shared_across_roots() {
    // Each root expands to exactly 1,024, but together they exceed admission.
    let source = replace_once(
        BRANCHING,
        "#104=SHAPE_REPRESENTATION('',(#205,#206),#103);",
        "#104=SHAPE_REPRESENTATION('',(#205),#103);\n\
         #900=SHAPE_REPRESENTATION('',(#206),#103);",
    );
    let error = import_step(source.into_bytes()).err().unwrap();
    assert!(
        error.contains("STEP occurrence limit exceeded (1024 occurrences)"),
        "{error}"
    );
}

#[test]
fn traversal_budget_is_shared_across_roots() {
    let source = replace_once(
        BRANCHING,
        "#105=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
        "#105=SHAPE_REPRESENTATION('',(#200),#103);\n\
         #900=SHAPE_REPRESENTATION('',(#205,#206),#103);\n\
         #901=SHAPE_REPRESENTATION('',(#205,#206),#103);\n\
         #902=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
    );
    let error = import_step(source.into_bytes()).err().unwrap();
    assert!(
        error.contains("STEP traversal limit exceeded (16384 steps)"),
        "{error}"
    );
}

#[test]
fn occurrence_budget_is_shared_across_assembly_paths() {
    // Append the committed binary mapping graph with IDs outside the assembly fixture.
    let (_, mapping) = BRANCHING.split_once("#200=").unwrap();
    let mapping = format!("#200={mapping}");
    let mut renamed = String::new();
    for part in mapping.split_inclusive('#') {
        // STEP references and declarations have the same numeric spelling.
        let digits = part.chars().take_while(char::is_ascii_digit).count();
        let id = part[..digits].parse::<u64>().ok();
        if let Some(id) = id.filter(|id| *id >= 200) {
            renamed.push_str(&(id + 1000).to_string());
            renamed.push_str(&part[digits..]);
        } else {
            renamed.push_str(part);
        }
    }
    let assembly = replace_once(
        include_str!("fixtures/two-occurrences-mm.step"),
        "#104=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
        "#104=SHAPE_REPRESENTATION('',(#1205),#103);\n\
         #105=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
    );
    let source = replace_once(&assembly, "ENDSEC;END-ISO-10303-21;", &renamed);
    let error = import_step(source.into_bytes()).err().unwrap();
    assert!(
        error.contains("STEP occurrence limit exceeded (1024 occurrences)"),
        "{error}"
    );
}

#[test]
fn admitted_binary_mapping_keeps_boundary_count_source_placement_and_mesh_coverage() {
    let source = replace_once(
        BRANCHING,
        "#104=SHAPE_REPRESENTATION('',(#205,#206),#103);",
        "#104=SHAPE_REPRESENTATION('',(#205),#103);",
    )
    .into_bytes();
    let model = import_step(source.clone()).unwrap();
    assert_eq!(model.original_bytes(), source);
    assert_eq!(model.solid_count(), 1);
    assert_eq!(model.occurrence_count(), 1024);
    // Eleven mappings each translate by 100 mm, without changing source geometry.
    assert_eq!(model.bounds(), [[1100., 0., 0.], [1102., 3., 4.]]);
    assert_eq!(model.source_faces().len(), 4);
    assert!(model.source_faces().iter().all(|face| face.solid_id == 101));
    assert_eq!(model.face_occurrences().len(), 4 * 1024);
    assert!(
        model
            .face_occurrences()
            .iter()
            .all(|face| face.representation_id == 105)
    );
    assert_eq!(model.mesh().positions.len(), 4 * 1024 * 9);
    let covered: std::collections::HashSet<_> = model.triangle_faces().iter().copied().collect();
    assert_eq!(covered.len(), model.face_occurrences().len());
    for face in 0..model.face_occurrences().len() {
        assert!(covered.contains(&face));
    }
    assert_eq!(model.accuracy().analysis_error_bound_mm, None);
}

#[test]
fn branching_placement_only_traversal_refuses_without_emitting_occurrences() {
    let source = include_str!("fixtures/mapped-over-budget.step").replace(
        "#105=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
        "#105=SHAPE_REPRESENTATION('',(#600,#601),#103);\n\
         #600=MAPPED_ITEM('',#602,#203);#601=MAPPED_ITEM('',#602,#203);\n\
         #602=REPRESENTATION_MAP(#200,#603);\n\
         #603=SHAPE_REPRESENTATION('',(#604,#605),#103);\n\
         #604=MAPPED_ITEM('',#606,#203);#605=MAPPED_ITEM('',#606,#203);\n\
         #606=REPRESENTATION_MAP(#200,#607);\n\
         #607=SHAPE_REPRESENTATION('',(#608,#609),#103);\n\
         #608=MAPPED_ITEM('',#610,#203);#609=MAPPED_ITEM('',#610,#203);\n\
         #610=REPRESENTATION_MAP(#200,#611);\n\
         #611=SHAPE_REPRESENTATION('',(#200),#103);\n\
         #900=ADVANCED_BREP_SHAPE_REPRESENTATION('',(#101),#103);",
    );
    let error = import_step(source.into_bytes())
        .err()
        .expect("placement-only branching must consume the traversal budget");
    assert!(
        error.contains("STEP traversal limit exceeded (16384 steps)"),
        "{error}"
    );
}
