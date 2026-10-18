use super::*;

#[test]
fn generated_analysis_keeps_body_geometry_and_face_tags_without_certification() {
    let ir = holed_plate();
    let display = preview_body_data(&ir, "", |_| Ok(())).unwrap();
    let analysis =
        generated_body_analysis_data(&ir, GeneratedBodyAnalysisOptions::default()).unwrap();
    assert_eq!(analysis.deflection_mm, 0.1);
    assert_eq!(analysis.analysis_error_bound_mm, None);
    assert_eq!(analysis.mesh.positions, display.positions);
    assert_eq!(analysis.positions_mm_f64.len(), display.positions.len());
    assert_eq!(
        analysis.triangle_face_occurrence_indices.len(),
        display.positions.len() / 9
    );
    let tags = &analysis.triangle_face_occurrence_indices;
    for index in 0..analysis.source_faces.len() {
        assert!(
            tags.contains(&index),
            "every generated face must have triangles"
        );
    }
    assert!(tags.iter().all(|tag| *tag < analysis.source_faces.len()));
    let refined = generated_body_analysis_data(
        &ir,
        GeneratedBodyAnalysisOptions {
            target_deflection_mm: Some(0.01),
        },
    )
    .unwrap();
    assert_eq!(refined.deflection_mm, 0.01);
    assert_eq!(refined.analysis_error_bound_mm, None);
    for target in [0.0, -0.01, f64::NAN, f64::INFINITY] {
        assert!(generated_body_analysis_data(
            &ir,
            GeneratedBodyAnalysisOptions {
                target_deflection_mm: Some(target)
            }
        )
        .is_err());
    }
}

fn step_products(text: &str) -> Vec<(String, String)> {
    use step_p21::ast::{EntityInstance, Exchange, Parameter};
    let parsed: Exchange = text.parse().expect("independent Part21 syntax parser");
    parsed
        .data
        .iter()
        .flat_map(|section| &section.entities)
        .filter_map(|entity| {
            let EntityInstance::Simple { record, .. } = entity else {
                return None;
            };
            if record.name != "PRODUCT" {
                return None;
            }
            let Parameter::List(args) = &record.parameter else {
                panic!("product parameters");
            };
            let (Parameter::String(id), Parameter::String(name)) = (&args[0], &args[1]) else {
                panic!("product identity");
            };
            Some((id.clone(), name.clone()))
        })
        .collect()
}

fn holed_plate() -> PreparedCaseIR {
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("../../../../test/fixtures/holed-plate.json")).unwrap();
    let body = &fixture["bodies"][0];
    assert_eq!(body["body"]["kind"], "plate");
    assert_eq!(body["body"]["clearance"], 0);
    let contours = body["contours"].as_array().unwrap();
    let outer = contours.iter().find(|c| c["hole"] == false).unwrap()["points"].clone();
    let holes: Vec<_> = contours
        .iter()
        .filter(|c| c["hole"] == true)
        .map(|c| c["points"].clone())
        .collect();
    serde_json::from_value(serde_json::json!({"revision":body["revision"],"body":body["body"],"regions":[{"outer":outer,"holes":holes,"cavities":[],"gaskets":[],"mounts":[]}]})).unwrap()
}

fn mesh_volume(mesh: &MeshData) -> f64 {
    mesh.positions
        .as_chunks::<9>()
        .0
        .iter()
        .map(|t| {
            let t: Vec<_> = t.iter().map(|&x| f64::from(x)).collect();
            (t[0] * (t[4] * t[8] - t[5] * t[7])
                + t[1] * (t[5] * t[6] - t[3] * t[8])
                + t[2] * (t[3] * t[7] - t[4] * t[6]))
                / 6.
        })
        .sum()
}

#[test]
fn generated_case_facade_preserves_holed_plate_analytic_material_and_step() {
    let result = build_case_data(holed_plate())
        .expect("existing prepared plate builds through exact facade");
    assert_eq!(result.revision, 1);
    assert!((mesh_volume(&result.mesh) - 14400.).abs() < 1e-5);
    assert_eq!(result.mesh.positions.len(), result.mesh.normals.len());
    let bounds: [[_; 3]; 2] = [
        std::array::from_fn(|axis| {
            result
                .mesh
                .positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[axis])
                .fold(f32::INFINITY, f32::min)
        }),
        std::array::from_fn(|axis| {
            result
                .mesh
                .positions
                .as_chunks::<3>()
                .0
                .iter()
                .map(|p| p[axis])
                .fold(f32::NEG_INFINITY, f32::max)
        }),
    ];
    assert_eq!(bounds, [[0., 0., 0.], [120., 80., 2.]]);
    let step = String::from_utf8(result.step).unwrap();
    assert_eq!(
        step_products(&step),
        [("holed-plate".into(), "holed-plate".into())]
    );
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 1);
    assert!(step.contains("MILLI"));
}

#[test]
fn preview_reuses_geometry_across_revisions_and_body_identity() {
    let first = holed_plate();
    let mut renamed = first.clone();
    renamed.revision = 77;
    renamed.body.id = "renamed".into();
    renamed.body.name = "Renamed body".into();
    let mut phases = Vec::new();
    let a = preview_body_data(&first, "same-scope", |phase| {
        phases.push(phase);
        Ok(())
    })
    .unwrap();
    let b = preview_body_data(&renamed, "same-scope", |phase| {
        phases.push(phase);
        Ok(())
    })
    .unwrap();
    assert!(
        std::rc::Rc::ptr_eq(&a, &b),
        "unchanged exact geometry reuses its retained mesh"
    );
    assert_eq!(phases, ["building", "tessellating"]);
}

#[test]
fn generated_assembly_keeps_body_order_names_revision_and_combined_material() {
    let mut ir: PreparedCaseAssemblyIR = serde_json::from_str(include_str!(
        "../../../../monstertruck/tests/fixtures/layered/gasketed-pair.prepared.json"
    ))
    .unwrap();
    ir.revision = 42;
    for body in &mut ir.bodies {
        body.revision = 42;
    }
    ir.bodies[0].body.id = "bottom".into();
    ir.bodies[0].body.name = "Bottom's tray".into();
    ir.bodies[1].body.id = "top".into();
    ir.bodies[1].body.name = "Top lid".into();
    let result = build_assembly_data(ir).unwrap();
    assert_eq!(result.revision, 42);
    assert!((mesh_volume(&result.mesh) - 18244.).abs() < 1e-5);
    let bodies = result.bodies.unwrap();
    assert_eq!(
        bodies
            .iter()
            .map(|b| (b.id.as_str(), b.name.as_str()))
            .collect::<Vec<_>>(),
        [("bottom", "Bottom's tray"), ("top", "Top lid")]
    );
    for (body, expected_z) in bodies.iter().zip([[0., 8.], [12., 20.]]) {
        assert!((mesh_volume(&body.mesh) - 9122.).abs() < 1e-5);
        let low = body
            .mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[2])
            .fold(f32::INFINITY, f32::min);
        let high = body
            .mesh
            .positions
            .as_chunks::<3>()
            .0
            .iter()
            .map(|p| p[2])
            .fold(f32::NEG_INFINITY, f32::max);
        assert_eq!([low, high], expected_z);
    }
    assert_eq!(
        result.mesh.positions.len(),
        bodies.iter().map(|b| b.mesh.positions.len()).sum::<usize>()
    );
    let step = String::from_utf8(result.step).unwrap();
    assert_eq!(
        step_products(&step),
        [
            ("bottom".into(), "Bottom's tray".into()),
            ("top".into(), "Top lid".into())
        ]
    );
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 2);
}

#[test]
fn step_only_export_reuses_exact_model_without_building_an_unused_mesh() {
    let body = holed_plate();
    let ir = PreparedCaseAssemblyIR {
        revision: body.revision,
        bodies: vec![body.clone()],
    };
    let key = "step-only-new-scope".to_string();
    let (revision, bytes) = step_only_data(&ir, std::slice::from_ref(&key)).unwrap();
    assert_eq!(revision, 1);
    assert!(!bytes.is_empty());
    let before = cache::body(&body, &key, false, &mut |_| Ok(())).unwrap();
    let mut phases = Vec::new();
    preview_body_data(&body, &key, |phase| {
        phases.push(phase);
        Ok(())
    })
    .unwrap();
    assert_eq!(
        phases,
        ["tessellating"],
        "STEP-only must retain exact geometry without meshing it"
    );
    let after = cache::body(&body, &key, false, &mut |_| Ok(())).unwrap();
    assert!(std::rc::Rc::ptr_eq(&before.model, &after.model));
}

#[test]
fn caller_key_cannot_override_geometry_or_scope_identity() {
    let first = holed_plate();
    let mut changed = first.clone();
    changed.body.thickness = 3.;
    let a = preview_body_data(&first, "untrusted-key", |_| Ok(())).unwrap();
    let b = preview_body_data(&changed, "untrusted-key", |_| Ok(())).unwrap();
    assert!(!std::rc::Rc::ptr_eq(&a, &b));
    assert!((mesh_volume(&b) - 21600.).abs() < 1e-5);
    let c = preview_body_data(&first, "other-scope", |_| Ok(())).unwrap();
    assert!(!std::rc::Rc::ptr_eq(&a, &c));
}

#[test]
fn failed_preview_progress_does_not_publish_and_the_same_input_can_retry() {
    let body = holed_plate();
    let error = preview_body_data(&body, "retry-scope", |phase| {
        if phase == "tessellating" {
            Err("callback stopped this preview".into())
        } else {
            Ok(())
        }
    })
    .err()
    .unwrap();
    assert_eq!(error, "callback stopped this preview");
    let mut phases = Vec::new();
    let mesh = preview_body_data(&body, "retry-scope", |phase| {
        phases.push(phase);
        Ok(())
    })
    .unwrap();
    assert_eq!(phases, ["building", "tessellating"]);
    assert!((mesh_volume(&mesh) - 14400.).abs() < 1e-5);
}

#[test]
fn cached_export_attaches_current_body_identity_without_copying_exact_geometry() {
    let body = holed_plate();
    let key = "identity-export".to_string();
    preview_body_data(&body, &key, |_| Ok(())).unwrap();
    let before = cache::body(&body, &key, false, &mut |_| Ok(())).unwrap();
    let mut renamed = body.clone();
    renamed.revision = 99;
    renamed.body.id = "new-id".into();
    renamed.body.name = "New body's name".into();
    let ir = PreparedCaseAssemblyIR {
        revision: 99,
        bodies: vec![renamed.clone()],
    };
    let result = full_assembly_data(&ir, std::slice::from_ref(&key)).unwrap();
    assert_eq!(result.revision, 99);
    let bodies = result.bodies.unwrap();
    assert_eq!(
        (bodies[0].id.as_str(), bodies[0].name.as_str()),
        ("new-id", "New body's name")
    );
    let step = String::from_utf8(result.step).unwrap();
    assert_eq!(
        step_products(&step),
        [("new-id".into(), "New body's name".into())]
    );
    let after = cache::body(&renamed, &key, false, &mut |_| Ok(())).unwrap();
    assert!(std::rc::Rc::ptr_eq(&before.model, &after.model));
}

#[test]
fn stale_assembly_and_unsafe_revision_fail_even_with_warm_cache() {
    let body = holed_plate();
    let key = "revision-scope".to_string();
    preview_body_data(&body, &key, |_| Ok(())).unwrap();
    let mut assembly = PreparedCaseAssemblyIR {
        revision: 2,
        bodies: vec![body.clone()],
    };
    assert!(full_assembly_data(&assembly, std::slice::from_ref(&key))
        .err()
        .unwrap()
        .contains("stale body revision"));
    assert!(step_only_data(&assembly, std::slice::from_ref(&key))
        .err()
        .unwrap()
        .contains("stale body revision"));
    assembly.bodies[0].revision = 2;
    assert_eq!(
        full_assembly_data(&assembly, std::slice::from_ref(&key))
            .unwrap()
            .revision,
        2
    );
    let mut unsafe_body = body;
    unsafe_body.revision = 9_007_199_254_740_992;
    let error = preview_body_data(&unsafe_body, &key, |_| {
        panic!("invalid revision cannot start construction")
    })
    .err()
    .unwrap();
    assert_eq!(error, "Invalid CAD revision");
    assert!(full_assembly_data(&assembly, &[])
        .err()
        .unwrap()
        .contains("Invalid CAD cache keys"));
    let empty = PreparedCaseAssemblyIR {
        revision: 1,
        bodies: vec![],
    };
    assert!(step_only_data(&empty, &[]).is_err());
}

fn plain_plate() -> PreparedCaseIR {
    let mut body = holed_plate();
    body.regions[0].holes.clear();
    body
}

#[test]
fn body_cache_keeps_sixty_four_recent_entries_and_releases_the_evicted_model() {
    let body = plain_plate();
    let entry = cache::body(&body, "lru0", false, &mut |_| Ok(())).unwrap();
    let weak = std::rc::Rc::downgrade(&entry.model);
    drop(entry);
    for i in 1..64 {
        cache::body(&body, &format!("lru{i}"), false, &mut |_| Ok(())).unwrap();
    }
    assert!(
        weak.upgrade().is_some(),
        "the entry limit must permit64 inexpensive bodies"
    );
    cache::body(&body, "lru64", false, &mut |_| Ok(())).unwrap();
    assert!(
        weak.upgrade().is_none(),
        "the65th entry releases the least recently used model"
    );
}

#[test]
fn an_assembly_retains_all_models_while_its_cache_evicts_earlier_bodies() {
    let mut ir = PreparedCaseAssemblyIR {
        revision: 1,
        bodies: Vec::new(),
    };
    let mut keys = Vec::new();
    for i in 0..65 {
        let mut body = plain_plate();
        body.body.z = Some(i as f64 * 3.);
        body.body.id = format!("body-{i}");
        body.body.name = format!("Body{i}");
        ir.bodies.push(body);
        keys.push(format!("assembly-lru-{i}"));
    }
    let result = full_assembly_data(&ir, &keys).unwrap();
    assert!((mesh_volume(&result.mesh) - 1_248_000.).abs() < 1e-5);
    let bodies = result.bodies.unwrap();
    assert_eq!(bodies.len(), 65);
    let last = bodies[64].mesh.positions.as_chunks::<3>().0;
    assert_eq!(
        last.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min),
        192.
    );
    assert_eq!(
        last.iter().map(|p| p[2]).fold(f32::NEG_INFINITY, f32::max),
        194.
    );
    let step = String::from_utf8(result.step).unwrap();
    assert!(step.contains("PRODUCT('body-0', 'Body0'"));
    assert!(step.contains("PRODUCT('body-64', 'Body64'"));
    assert_eq!(step.matches("MANIFOLD_SOLID_BREP(").count(), 65);
}

#[test]
fn the_solid_budget_refuses_to_retain_a_257_component_model() {
    let mut body = plain_plate();
    let region = body.regions.remove(0);
    for i in 0..257 {
        let mut next = region.clone();
        for p in &mut next.outer {
            p.x += i as f64 * 200.;
        }
        body.regions.push(next);
    }
    let entry = cache::body(&body, "solid-budget", false, &mut |_| Ok(())).unwrap();
    assert_eq!(entry.model.cache_complexity(), (257, 7710));
    let weak = std::rc::Rc::downgrade(&entry.model);
    drop(entry);
    assert!(
        weak.upgrade().is_none(),
        "models exceeding256 physical solids are not retained"
    );
}

#[test]
fn the_topology_budget_refuses_to_retain_an_oversized_exact_polygon() {
    let mut body = plain_plate();
    body.regions[0].outer = (0..1180)
        .map(|i| {
            let a = std::f64::consts::TAU * i as f64 / 1180.;
            boardstudio_contracts::Vec2 {
                x: 20. * a.cos(),
                y: 20. * a.sin(),
            }
        })
        .collect();
    let entry = cache::body(&body, "topology-budget", false, &mut |_| Ok(())).unwrap();
    assert_eq!(entry.model.cache_complexity(), (1, 8262));
    let weak = std::rc::Rc::downgrade(&entry.model);
    drop(entry);
    assert!(
        weak.upgrade().is_none(),
        "models exceeding8192 topology units are not retained"
    );
}

#[test]
fn geometry_key_storage_counts_against_the_64_mib_retention_limit() {
    let body = plain_plate();
    let scope = "x".repeat(64 * 1024 * 1024);
    let entry = cache::body(&body, &scope, false, &mut |_| Ok(())).unwrap();
    let weak = std::rc::Rc::downgrade(&entry.model);
    drop(entry);
    assert!(
        weak.upgrade().is_none(),
        "a key alone cannot exceed the cache memory budget"
    );
}

#[test]
fn pcb_source_preview_changes_cannot_reuse_a_polygon_only_cache_entry() {
    let input: PreparedCaseIR = serde_json::from_str(include_str!(
        "../../../../monstertruck/tests/fixtures/pcb-source/circle-tracer.json"
    ))
    .unwrap();
    let first = preview_body_data(&input, "source-pcb", |_| Ok(())).unwrap();
    // An inscribed mesh with the written 0.5 rad angular limit lies outside
    // radius r*cos(0.25). This checks source-circle material, not the diamond projection.
    let check_volume = |mesh: &MeshData, radius: f64| {
        let exact = 2.0 * std::f64::consts::PI * radius * radius;
        let lower = exact * 0.25_f64.cos().powi(2);
        let actual = mesh_volume(mesh);
        assert!(
            (lower - 1e-3..=exact + 1e-3).contains(&actual),
            "authored radius {radius}: expected volume band [{lower}, {exact}], got {actual}"
        );
    };
    check_volume(&first, 2.0);
    let mut changed = input.clone();
    let primitive = &mut changed.body.pcb_source.as_mut().unwrap().contours[0].primitives[0];
    let boardstudio_contracts::MechanicalShape::Circle { end, .. } = &mut primitive.geometry else {
        panic!("source fixture must retain its authored circle");
    };
    end.x = 11.0;
    primitive.curve_support.as_mut().unwrap().radius_mm = 1.0;
    let second = preview_body_data(&changed, "source-pcb", |_| Ok(())).unwrap();
    assert!(!std::rc::Rc::ptr_eq(&first, &second));
    check_volume(&second, 1.0);
    let mut renamed = input;
    renamed.revision = 2;
    renamed.body.id = "renamed-source-pcb".into();
    renamed.body.name = "Renamed source PCB".into();
    let reused = preview_body_data(&renamed, "source-pcb", |_| Ok(())).unwrap();
    assert!(std::rc::Rc::ptr_eq(&first, &reused));
}
