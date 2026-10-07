use super::*;
use serde_json::json;

#[test]
fn wall_support_is_fused_and_insert_seat_retains_roof() {
    let ir: PreparedCase = serde_json::from_value(json!({
        "revision":1,
        "body":{"id":"tray","name":"Tray","kind":"tray","thickness":2,
            "z":0,"wallHeight":8,"features":[
                {"kind":"support-prism","id":"tower","points":[
                    {"x":1,"y":5},{"x":6,"y":5},{"x":6,"y":15},{"x":1,"y":15}
                ],"z":2,"height":4},
                {"kind":"round-seat","id":"seat","at":{"x":4,"y":10},
                    "z":2,"height":3,"diameter":2}
            ]},
        "regions":[{"outer":[{"x":0,"y":0},{"x":30,"y":0},{"x":30,"y":20},{"x":0,"y":20}],
            "cavities":[[{"x":2,"y":2},{"x":28,"y":2},{"x":28,"y":18},{"x":2,"y":18}]]}]
    }))
    .unwrap();
    let solids = build_body(&ir).unwrap();
    assert_eq!(solids.len(), 1, "tower must join the wall and floor");
    // Floor 1200 + wall 1472 + tower's cavity material 160 - blind bore 3π.
    assert!((solids[0].volume() - (2832. - 3. * std::f64::consts::PI)).abs() < 1e-5);
    let roof_probe = Solid::cube(DVec3::new(3.9, 9.9, 5.2), DVec3::new(4.1, 10.1, 5.8));
    let remaining = (Boolean::from(&solids[0]) * &roof_probe)
        .build_vec()
        .unwrap();
    assert!((remaining.iter().map(Solid::volume).sum::<f64>() - 0.024).abs() < 1e-7);
}

#[test]
fn resolved_internal_case_is_connected_and_preserves_wall_material() {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let request = |input: serde_json::Value| {
        let mut driver = Command::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../target/debug/examples/prepare_case"
        ))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
        writeln!(driver.stdin.take().unwrap(), "{input}").unwrap();
        let output = driver.wait_with_output().unwrap();
        assert!(output.status.success());
        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap()
    };
    for fixture in [
        "rectangle",
        "countersunk",
        "downward-boss",
        "rotated-concave",
        "split",
    ] {
        let path = format!(
            "{}/../test/fixtures/internal-gasket-v1/{fixture}.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let source: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        let resolved = request(source);
        assert_eq!(
            resolved["assembly"]["generationBlocked"], false,
            "{fixture}: {resolved}"
        );
        let prepared = request(
            json!({"kind":"prepare-case","id":"internal-native","ir":resolved["assembly"]["case"]}),
        );
        let assembly: PreparedAssembly = serde_json::from_value(prepared["ir"].clone()).unwrap();
        let plate = build_body(
            assembly
                .bodies
                .iter()
                .find(|b| b.body.id == "plate")
                .unwrap(),
        )
        .unwrap();
        let pcb = request(
            json!({"kind":"prepare-case","id":"pcb-native","ir":{"revision":0,"bodies":[resolved["assembly"]["pcbReference"]]}}),
        );
        let pcb: PreparedAssembly = serde_json::from_value(pcb["ir"].clone()).unwrap();
        let pcb = build_body(&pcb.bodies[0]).unwrap();
        for body in &assembly.bodies {
            let solids = build_body(body).unwrap();
            assert_eq!(
                solids.len(),
                body.regions.len(),
                "{fixture}: {} must be connected per region",
                body.body.id
            );
            if body.body.id == "bottom" || body.body.id == "retainer" {
                let exported = export_case(solids.clone(), 0, None).unwrap();
                let imported = Solid::read_step(&mut std::io::Cursor::new(exported.step)).unwrap();
                assert_eq!(imported.len(), solids.len());
                assert!(
                    (imported.iter().map(Solid::volume).sum::<f64>()
                        - solids.iter().map(Solid::volume).sum::<f64>())
                    .abs()
                        < 1e-5
                );
                for rigid in &solids {
                    for moving in plate.iter().chain(pcb.iter()) {
                        for delta in [-0.1, 0.1] {
                            let shifted = moving.clone().translate(DVec3::Z * delta);
                            let rigid = rigid.clone();
                            let contact = (Boolean::from(&shifted) * &rigid).build_vec().unwrap();
                            assert!(
                                contact.iter().map(Solid::volume).sum::<f64>() < 1e-7,
                                "{fixture}: rigid contact at {delta}"
                            );
                        }
                    }
                }
                for feature in &body.body.features {
                    if let CaseFeature::SupportPrism {
                        id,
                        points,
                        z,
                        height,
                    } = feature
                    {
                        if id.starts_with("tower:") {
                            let tower = make_prism(points, *z, *height).unwrap();
                            let remainder = solids
                                .iter()
                                .fold(Boolean::from(&tower), |shape, solid| shape - solid)
                                .build_vec()
                                .unwrap();
                            assert!(
                                remainder.iter().map(Solid::volume).sum::<f64>() < 1e-7,
                                "{fixture}: {id} has missing material"
                            );
                        }
                    }
                }
            }
        }
    }
}

#[test]
fn countersunk_seat_removes_a_conical_frustum_not_a_counterbore() {
    let ir: PreparedCase=serde_json::from_value(json!({"revision":0,
        "body":{"id":"seat","name":"Seat","kind":"plate","thickness":3,"features":[
            {"kind":"conical-seat","id":"head","at":{"x":5,"y":5},"z":0,"height":1,"diameter":4,"endDiameter":2}]},
        "regions":[{"outer":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]}]
    })).unwrap();
    let solids = build_body(&ir).unwrap();
    assert!((solids[0].volume() - (300. - 7. * std::f64::consts::PI / 3.)).abs() < 1e-6);
}

#[test]
fn prepared_support_cannot_protrude_or_be_silently_ignored() {
    for x in [-1., 11.] {
        let ir: PreparedCase=serde_json::from_value(json!({"revision":0,
            "body":{"id":"tray","name":"Tray","kind":"plate","thickness":2,"features":[
                {"kind":"support-prism","id":"outside","points":[
                    {"x":x,"y":2},{"x":x+3.,"y":2},{"x":x+3.,"y":4},{"x":x,"y":4}],"z":1,"height":2}]},
            "regions":[{"outer":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}]}]
        })).unwrap();
        let error = build_body(&ir).expect_err("outside support must be rejected");
        assert!(error.contains("outside"), "{error}");
    }
}
