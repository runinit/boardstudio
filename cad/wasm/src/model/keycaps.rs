//! Mesh-only keycap display construction. Presets arrive from the Rust core.
use super::*;

mod adapter;
mod lettering;
mod mesh_template;
mod printing;
#[cfg(test)]
mod print_coverage;
mod request;

use request::Request;

fn empty_mesh() -> MeshData {
    MeshData {
        positions: Vec::new(),
        normals: Vec::new(),
    }
}

include!("keycaps/validation.rs");

#[wasm_bindgen]
pub fn build_keycaps(input: JsValue) -> Result<JsValue, JsValue> {
    let request: Request = deserialize(input)?;
    case_result_to_js(build(request).map_err(js_error)?)
}

fn build(request: Request) -> Result<CaseResultData, String> {
    request.validate()?;
    let bodies = if request.printable {
        adapter::build_printable(&request.specs)?
    } else {
        adapter::build(&request.specs)?
    };
    Ok(CaseResultData {
        revision: request.revision,
        step: Vec::new(),
        mesh: empty_mesh(),
        bodies: Some(bodies),
    })
}

#[cfg(test)]
mod print_tests {
    use super::*;
    use std::collections::BTreeMap;

    fn print_input(mount: &str) -> serde_json::Value {
        serde_json::json!({
            "revision": 9, "printable": true,
            "specs": [{"id":"print-cap", "reference":"SW1", "profile":"g20",
                "mount":mount, "row":3, "size":{"x":18.2,"y":18.2},
                "topSize":{"x":13.2,"y":13.2}, "height":6.5, "tilt":0,
                "dishDepth":0, "spherical":false, "wallThickness":1.2,
                "pose":{"at":{"x":0,"y":0},"rotation":0}, "side":"front",
                "z":0, "travel":4, "legend":"", "color":"#eee", "legendColor":"#222"}]
        })
    }

    #[test]
    fn printable_tilted_legend_retains_closed_material() {
        let mut input = print_input("alps");
        input["specs"][0]["height"] = 14.into();
        input["specs"][0]["tilt"] = 8.into();
        input["specs"][0]["legend"] = "O".into();
        let result = build(serde_json::from_value(input).unwrap())
            .expect("tilted legend must form a closed print operand");
        assert_eq!(result.bodies.unwrap().len(), 1, "legend is merged into cap");
    }

    #[test]
    fn printable_dcs_row_three_dished_legend_prints() {
        // Product preset: DCS row 3 is untilted with a 0.6 mm cylindrical dish.
        let mut input = print_input("mx");
        input["specs"][0]["profile"] = "dcs".into();
        input["specs"][0]["height"] = 8.5.into();
        input["specs"][0]["dishDepth"] = 0.6.into();
        input["specs"][0]["legend"] = "A".into();
        build(serde_json::from_value(input).unwrap()).expect("dished legend must print");
    }

    #[test]
    fn printable_keycaps_are_single_closed_bodies_with_independent_socket_volumes() {
        for (mount, expected_volume) in [
            ("mx", 824.019207008451),
            ("alps", 817.503207008451),
            ("choc-v1", 833.697207008451),
            ("choc-v2", 836.018207008451),
        ] {
            let input = print_input(mount);
            let result = build(serde_json::from_value(input).unwrap()).expect("printable flat cap");
            assert_eq!(result.revision, 9);
            assert!(result.step.is_empty(), "printing never creates keycap STEP");
            let bodies = result.bodies.unwrap();
            assert_eq!(bodies.len(), 1);
            assert_eq!(bodies[0].id, "keycap:print-cap");
            let mesh = &bodies[0].mesh;
            let mut edges = BTreeMap::new();
            let mut volume = 0.0_f64;
            for triangle in mesh.positions.as_chunks::<9>().0 {
                let points =
                    [0, 1, 2].map(|i| [triangle[3 * i], triangle[3 * i + 1], triangle[3 * i + 2]]);
                for i in 0..3 {
                    let key = |p: [f32; 3]| p.map(|v| if v == 0.0 { 0 } else { v.to_bits() });
                    *edges
                        .entry((key(points[i]), key(points[(i + 1) % 3])))
                        .or_insert(0usize) += 1;
                }
                let [a, b, c] = points.map(|p| p.map(f64::from));
                volume += (a[0] * (b[1] * c[2] - b[2] * c[1])
                    + a[1] * (b[2] * c[0] - b[0] * c[2])
                    + a[2] * (b[0] * c[1] - b[1] * c[0]))
                    / 6.0;
            }
            for (&(a, b), &count) in &edges {
                assert_eq!(count, 1, "{mount}: duplicate directed edge");
                assert_eq!(
                    edges.get(&(b, a)),
                    Some(&1),
                    "{mount}: open or inconsistent edge"
                );
            }
            // Frozen analytic rounded-loft volume plus stem material minus roof overlap.
            assert!(
                (volume - expected_volume).abs() < 0.05,
                "{mount}: volume {volume}, expected {expected_volume}"
            );
        }
    }
}
