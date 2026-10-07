//! Native replacements for source-asset STEP import, bounds, and datum checks.
use super::read_step_model_data;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap()
}

fn json(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}

fn bytes(path: &str) -> Vec<u8> {
    fs::read(root().join(path)).unwrap()
}

fn sha256(data: &[u8]) -> String {
    format!("{:x}", Sha256::digest(data))
}

fn positions_bounds(positions: &[f32]) -> ([f64; 3], [f64; 3]) {
    assert!(!positions.is_empty());
    assert!(positions.iter().all(|v| v.is_finite()));
    let mut min = [f64::INFINITY; 3];
    let mut max = [f64::NEG_INFINITY; 3];
    for point in positions.chunks_exact(3) {
        for axis in 0..3 {
            min[axis] = min[axis].min(f64::from(point[axis]));
            max[axis] = max[axis].max(f64::from(point[axis]));
        }
    }
    (min, max)
}

fn assert_contains(
    outer_min: [f64; 3],
    outer_max: [f64; 3],
    inner_min: [f64; 3],
    inner_max: [f64; 3],
    tolerance: f64,
    label: &str,
) {
    for axis in 0..3 {
        assert!(
            outer_min[axis] <= inner_min[axis] + tolerance,
            "{label} min axis {axis}: {} <= {}",
            outer_min[axis],
            inner_min[axis]
        );
        assert!(
            outer_max[axis] >= inner_max[axis] - tolerance,
            "{label} max axis {axis}: {} >= {}",
            outer_max[axis],
            inner_max[axis]
        );
    }
}

fn triple(value: &Value) -> [f64; 3] {
    [
        value[0].as_f64().unwrap(),
        value[1].as_f64().unwrap(),
        value[2].as_f64().unwrap(),
    ]
}

#[test]
fn pinned_vik_step_meshes_preserve_source_bounds_and_hashes() {
    let ledger = json("catalogue/modules/asset-ledger.json");
    let tight = json("cad/test/fixtures/vik-model-tight-bounds.json");
    for asset in ledger["models"].as_array().unwrap().iter().filter(|m| {
        let p = m["path"].as_str().unwrap_or("").to_ascii_lowercase();
        p.ends_with(".step") || p.ends_with(".stp")
    }) {
        let file = asset["bundledFile"].as_str().unwrap();
        let input = fs::read(root().join(file)).unwrap();
        let model = read_step_model_data(input.clone()).unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(
            model.mesh.positions.len() >= 9,
            "{file}: nonempty triangle mesh"
        );
        assert_eq!(
            model.mesh.positions.len(),
            model.mesh.normals.len(),
            "{file}: positions/normals align"
        );
        assert!(
            model.mesh.normals.iter().all(|n| n.is_finite()),
            "{file}: finite normals"
        );
        let (mesh_min, mesh_max) = positions_bounds(&model.mesh.positions);
        assert_contains(model.min, model.max, mesh_min, mesh_max, 0.03, file);
        let b = &asset["nativeBoundsMm"];
        let source_min = [
            b["XMin"].as_f64().unwrap(),
            b["YMin"].as_f64().unwrap(),
            b["ZMin"].as_f64().unwrap(),
        ];
        let source_max = [
            b["XMax"].as_f64().unwrap(),
            b["YMax"].as_f64().unwrap(),
            b["ZMax"].as_f64().unwrap(),
        ];
        assert_contains(source_min, source_max, mesh_min, mesh_max, 0.03, file);
        let measurement = tight["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["assetId"] == asset["assetId"])
            .unwrap_or_else(|| panic!("no independent tight bound for {}", asset["assetId"]));
        assert_eq!(
            sha256(&input),
            measurement["sha256"].as_str().unwrap(),
            "{file}: pinned source hash"
        );
        let tight_min = triple(&measurement["min"]);
        let tight_max = triple(&measurement["max"]);
        for axis in 0..3 {
            assert!(
                (mesh_min[axis] - tight_min[axis]).abs() <= 0.10001,
                "{file} tight min axis {axis}: {} vs {}",
                mesh_min[axis],
                tight_min[axis]
            );
            assert!(
                (mesh_max[axis] - tight_max[axis]).abs() <= 0.10001,
                "{file} tight max axis {axis}: {} vs {}",
                mesh_max[axis],
                tight_max[axis]
            );
        }
    }
}

#[test]
fn pinned_vik_binary_stls_preserve_source_bounds() {
    let ledger = json("catalogue/modules/asset-ledger.json");
    for asset in ledger["models"].as_array().unwrap().iter().filter(|m| {
        m["path"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase()
            .ends_with(".stl")
    }) {
        let file = asset["bundledFile"].as_str().unwrap();
        let data = fs::read(root().join(file)).unwrap();
        assert!(data.len() >= 84, "{file}: binary STL header");
        let triangles = u32::from_le_bytes(data[80..84].try_into().unwrap()) as usize;
        assert!(triangles > 0, "{file}: STL has triangles");
        assert_eq!(
            data.len(),
            84 + triangles * 50,
            "{file}: complete binary STL"
        );
        let (mut min, mut max) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for face in 0..triangles {
            let start = 84 + face * 50 + 12;
            for vertex in 0..3 {
                for axis in 0..3 {
                    let offset = start + vertex * 12 + axis * 4;
                    let value = f32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
                    assert!(value.is_finite(), "{file}: finite coordinate face {face}");
                    min[axis] = min[axis].min(f64::from(value));
                    max[axis] = max[axis].max(f64::from(value));
                }
            }
        }
        let b = &asset["nativeBoundsMm"];
        assert_contains(
            [
                b["XMin"].as_f64().unwrap(),
                b["YMin"].as_f64().unwrap(),
                b["ZMin"].as_f64().unwrap(),
            ],
            [
                b["XMax"].as_f64().unwrap(),
                b["YMax"].as_f64().unwrap(),
                b["ZMax"].as_f64().unwrap(),
            ],
            min,
            max,
            0.03,
            file,
        );
    }
}

#[test]
fn vik_display_transform_aligns_board_and_asymmetric_h1_h2_rims() {
    let catalogue = json("catalogue/modules/imported-modules.json");
    let definition = catalogue["modules"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| {
            e["row"] == "vik-display-adapter" && e["definition"]["variant"] == "pcb/1.47inch/pcb"
        })
        .unwrap()["definition"]
        .clone();
    let ledger = json("catalogue/modules/asset-ledger.json");
    let asset = ledger["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["path"] == "pcb/1.47inch/1.47inch.step")
        .unwrap();
    let transform = &asset["boardToModelTransform"];
    assert_eq!(
        transform["rotation"],
        serde_json::json!({"x":0,"y":0,"z":0})
    );
    assert_eq!(transform["scale"], serde_json::json!({"x":1,"y":1,"z":1}));
    assert!(transform["evidence"]["asymmetricFeatures"]
        .as_str()
        .unwrap()
        .contains("H1/H2"));
    let model = read_step_model_data(bytes(asset["bundledFile"].as_str().unwrap())).unwrap();
    let offset = [
        transform["offset"]["x"].as_f64().unwrap(),
        transform["offset"]["y"].as_f64().unwrap(),
        transform["offset"]["z"].as_f64().unwrap(),
    ];
    let board_points = definition["board"]["contours"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|c| c["points"].as_array().unwrap())
        .collect::<Vec<_>>();
    let board_min_x = board_points
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    let board_max_x = board_points
        .iter()
        .map(|p| p["x"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    let board_min_y = board_points
        .iter()
        .map(|p| p["y"].as_f64().unwrap())
        .fold(f64::INFINITY, f64::min);
    let board_max_y = board_points
        .iter()
        .map(|p| p["y"].as_f64().unwrap())
        .fold(f64::NEG_INFINITY, f64::max);
    let source = &asset["boardNativeBoundsMm"];
    assert_eq!(source["XMin"].as_f64().unwrap() + offset[0], board_min_x);
    assert_eq!(source["XMax"].as_f64().unwrap() + offset[0], board_max_x);
    assert_eq!(source["YMin"].as_f64().unwrap() + offset[1], board_min_y);
    assert_eq!(source["YMax"].as_f64().unwrap() + offset[1], board_max_y);
    assert_eq!(source["ZMin"].as_f64().unwrap() + offset[2], -0.8);
    assert_eq!(source["ZMax"].as_f64().unwrap() + offset[2], 0.8);
    for mount in definition["mounts"].as_array().unwrap() {
        let mx = mount["at"]["x"].as_f64().unwrap();
        let my = mount["at"]["y"].as_f64().unwrap();
        let radius = mount["diameter"].as_f64().unwrap() / 2.0;
        let count = model
            .mesh
            .positions
            .chunks_exact(3)
            .filter(|p| {
                let x = f64::from(p[0]) + offset[0];
                let y = f64::from(p[1]) + offset[1];
                let z = f64::from(p[2]) + offset[2];
                ((x - mx).hypot(y - my) - radius).abs() < 0.035 && (z.abs() - 0.8).abs() < 0.035
            })
            .count();
        assert!(
            count > 20,
            "{} source mount maps onto STEP rim ({count} vertices)",
            mount["sourceId"]
        );
    }
}

const THQ_MODELS: [(&str, [f64; 3], [f64; 3]); 3] = [
    (
        "THQWGD001-rotation.stp",
        [-9.216318757, -8.812124802, -3.5],
        [9.166318857, 9.183731752, 20.372216785],
    ),
    (
        "THQWGD001C-2pin.stp",
        [-9.064170501, -8.952765450, -3.461776741],
        [9.554079635, 9.042790450, 20.066001209],
    ),
    (
        "THQWGD001C-4pin.stp",
        [-9.064170501, -8.952765450, -3.461776741],
        [10.3549875, 9.042790450, 20.066001209],
    ),
];

fn thq_path(filename: &str) -> String {
    format!("ergogen/library/vendor/thqwgd001/3d_models/{filename}")
}

#[test]
fn thq_source_assemblies_keep_tight_bounds_and_meshes() {
    for (filename, expected_min, expected_max) in THQ_MODELS {
        let model = read_step_model_data(bytes(&thq_path(filename)))
            .unwrap_or_else(|e| panic!("{filename}: {e}"));
        let (min, max) = positions_bounds(&model.mesh.positions);
        for axis in 0..3 {
            assert!(
                (min[axis] - expected_min[axis]).abs() < 0.03,
                "{filename} mesh min axis {axis}: {}",
                min[axis]
            );
            assert!(
                (max[axis] - expected_max[axis]).abs() < 0.03,
                "{filename} mesh max axis {axis}: {}",
                max[axis]
            );
            assert!(
                model.min[axis] <= min[axis] + 1e-6 && model.max[axis] >= max[axis] - 1e-6,
                "{filename} imported bounds contain mesh"
            );
        }
        assert!(model.mesh.positions.len() > 10_000);
        assert_eq!(model.mesh.positions.len(), model.mesh.normals.len());
        assert!(model.min[2] < -3.0, "THQ leads remain below the PCB plane");
    }
}

#[test]
fn thq_pins_align_with_leads_and_front_back_transform_oracle() {
    let catalogue = json("catalogue/parts/imported-parts.json");
    let oracle = json("cad/test/fixtures/thqwgd001-transform-oracle.json");
    let configurations: [(&str, &str, &[&str]); 3] = [
        (
            "thqwgd001:rotation-reversible",
            "THQWGD001-rotation.stp",
            &["A", "B", "C"],
        ),
        (
            "thqwgd001:c-2pin-reversible",
            "THQWGD001C-2pin.stp",
            &["A", "B", "C", "1", "2"],
        ),
        (
            "thqwgd001:c-4pin-reversible",
            "THQWGD001C-4pin.stp",
            &["A", "B", "C", "1", "2"],
        ),
    ];
    let pose = &oracle["pose"];
    let yaw = pose["yawDegrees"].as_f64().unwrap().to_radians();
    let (sn, cs) = yaw.sin_cos();
    let tx = pose["at"][0].as_f64().unwrap();
    let ty = pose["at"][1].as_f64().unwrap();
    let board_z = pose["boardThickness"].as_f64().unwrap();
    for (index, (definition_id, filename, contacts)) in configurations.iter().enumerate() {
        let entry = catalogue["parts"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["definition"]["id"] == *definition_id)
            .unwrap();
        let pads = entry["definition"]["pads"].as_array().unwrap();
        let model = read_step_model_data(bytes(&thq_path(filename))).unwrap();
        let lead_xy = model
            .mesh
            .positions
            .chunks_exact(3)
            .filter(|p| p[2] < -3.0)
            .map(|p| [f64::from(p[0]), f64::from(p[1])])
            .collect::<Vec<_>>();
        let holes = pads
            .iter()
            .filter(|p| {
                p["drill"].is_number()
                    && contacts.contains(&p["number"].as_str().unwrap_or(""))
                    && (!["A", "B", "C"].contains(&p["number"].as_str().unwrap_or(""))
                        || p["at"]["x"].as_f64().unwrap() > -6.0)
            })
            .collect::<Vec<_>>();
        assert!(!holes.is_empty());
        for hole in holes {
            let x = hole["at"]["x"].as_f64().unwrap();
            let y = hole["at"]["y"].as_f64().unwrap();
            let nearest = lead_xy
                .iter()
                .map(|p| (p[0] - x).hypot(p[1] - y))
                .fold(f64::INFINITY, f64::min);
            assert!(
                nearest < 0.19,
                "{filename} lead {nearest:.3} mm from footprint hole {}",
                hole["id"]
            );
        }
        let source = &oracle["models"][index];
        assert_eq!(
            sha256(&bytes(&thq_path(filename))),
            source["modelSource"]["sha256"].as_str().unwrap(),
            "{filename} pinned source hash"
        );
        for side in ["front", "back"] {
            let mut min = [f64::INFINITY; 3];
            let mut max = [f64::NEG_INFINITY; 3];
            for p in model.mesh.positions.chunks_exact(3) {
                let x = f64::from(p[0]) * if side == "back" { -1.0 } else { 1.0 };
                let y = f64::from(p[1]);
                let z = f64::from(p[2]);
                let point = [
                    tx + x * cs - y * sn,
                    ty + x * sn + y * cs,
                    if side == "front" { board_z + z } else { -z },
                ];
                for axis in 0..3 {
                    min[axis] = min[axis].min(point[axis]);
                    max[axis] = max[axis].max(point[axis]);
                }
            }
            let expected = &source["placements"][side]["exactTransformedSolidBounds"];
            let expected_min = triple(&expected["min"]);
            let expected_max = triple(&expected["max"]);
            for axis in 0..3 {
                assert!(
                    (min[axis] - expected_min[axis]).abs() < 0.03,
                    "{filename} {side} min axis {axis}: {}",
                    min[axis]
                );
                assert!(
                    (max[axis] - expected_max[axis]).abs() < 0.03,
                    "{filename} {side} max axis {axis}: {}",
                    max[axis]
                );
            }
        }
    }
}

#[test]
fn step_expectations_agree_with_fixture_manifests() {
    let expectations = json("cad/test/fixtures/step-expectations.json");
    assert_eq!(expectations["units"], "millimetres");
    assert!(expectations["cases"].as_array().unwrap().len() >= 25);
    let baselines = expectations["manifestBaselines"].as_object().unwrap();
    let mut checked = 0;
    for name in ["manifest.json", "followup-manifest.json"] {
        let manifest = json(&format!("cad/test/fixtures/{name}"));
        for fixture in manifest["fixtures"].as_array().unwrap() {
            let id = fixture["id"].as_str().unwrap();
            let baseline = baselines
                .get(id)
                .unwrap_or_else(|| panic!("{id}: no baseline expectation"));
            assert_eq!(baseline["solids"], fixture["expectedSolids"], "{id} solids");
            if let Some(volume) = fixture["expectedVolume"].as_f64() {
                assert!(
                    (baseline["volume"].as_f64().unwrap() - volume).abs() < 0.1,
                    "{id} volume"
                );
            }
            let (min, max) = (triple(&baseline["min"]), triple(&baseline["max"]));
            let (want_min, want_max) = (
                triple(&fixture["expectedBounds"][0]),
                triple(&fixture["expectedBounds"][1]),
            );
            for axis in 0..3 {
                assert!(
                    (min[axis] - want_min[axis]).abs() < 0.01,
                    "{id} min axis {axis}"
                );
                assert!(
                    (max[axis] - want_max[axis]).abs() < 0.01,
                    "{id} max axis {axis}"
                );
            }
            checked += 1;
        }
    }
    assert_eq!(
        checked,
        baselines.len(),
        "every baseline belongs to a manifest fixture"
    );
}
