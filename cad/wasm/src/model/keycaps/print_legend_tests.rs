//! Print regressions for legends: the union must stay one closed body that keeps the roof, its
//! thickness and the legend relief, and must still refuse genuinely separate material.
use super::{
    mesh_template::build_keycap_mesh_template,
    printing,
    test_support::{contains, cross, dot},
    MeshData,
};
use boardstudio_contracts::KeycapSpec;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

/// DCS row 3: untilted, 0.6 mm cylindrical dish.
const DCS_ROW_3: (&str, f64, f64, f64, bool) = ("dcs", 8.5, 0.0, 0.6, false);
/// SA row 5: tall, tilted 8 degrees, 0.7 mm spherical dish.
const SA_ROW_5: (&str, f64, f64, f64, bool) = ("sa", 14.0, 8.0, 0.7, true);

fn cap(roof: (&str, f64, f64, f64, bool), mount: &str, legend: &str) -> Value {
    let (profile, height, tilt, dish_depth, spherical) = roof;
    json!({
        "id": "cap", "reference": "SW1", "profile": profile, "mount": mount, "row": 3,
        "size": {"x": 18.2, "y": 18.2}, "topSize": {"x": 13.2, "y": 13.2}, "height": height,
        "tilt": tilt, "dishDepth": dish_depth, "spherical": spherical, "wallThickness": 1.2,
        "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front", "z": 0, "travel": 4,
        "legend": legend, "color": "#eeeeee", "legendColor": "#222222"
    })
}

fn print_body(spec: &Value) -> Result<MeshData, String> {
    let request = json!({"revision": 3, "printable": true, "specs": [spec]});
    let mut bodies = super::build(serde_json::from_value(request).unwrap())?
        .bodies
        .unwrap();
    assert_eq!(bodies.len(), 1, "cap, stem and legend are one print body");
    assert_eq!(bodies[0].id, "keycap:cap");
    Ok(bodies.remove(0).mesh)
}

fn triangles(mesh: &MeshData) -> Vec<[[f64; 3]; 3]> {
    mesh.positions
        .as_chunks::<9>()
        .0
        .iter()
        .map(|t| [0, 1, 2].map(|i| [0, 1, 2].map(|a| f64::from(t[3 * i + a]))))
        .collect()
}

/// Closed, consistently oriented, one connected surface of positive volume; returns the volume.
fn closed_positive_volume(mesh: &MeshData) -> Result<f64, String> {
    macro_rules! ensure {
        ($condition:expr, $message:expr) => {
            // Written positively so a NaN comparison fails the check instead of passing it.
            if $condition {
            } else {
                return Err($message.to_string());
            }
        };
    }
    let key = |p: [f64; 3]| p.map(|v| (v as f32).to_bits());
    let mut directed = BTreeSet::new();
    let mut links: BTreeMap<[u32; 3], Vec<[u32; 3]>> = BTreeMap::new();
    let mut volume = 0.0;
    for (index, [a, b, c]) in triangles(mesh).into_iter().enumerate() {
        let normal = [0, 1, 2].map(|k| f64::from(mesh.normals[9 * index + k]));
        let face = cross(
            [0, 1, 2].map(|k| b[k] - a[k]),
            [0, 1, 2].map(|k| c[k] - a[k]),
        );
        let length = dot(face, face).sqrt();
        ensure!(length > 0.0, "zero-area triangle");
        // Normals come from the f64 union and positions are stored as f32, so a thin sliver may
        // differ by a fraction of a degree; a flipped or wrong face would not.
        ensure!(
            dot(normal, face) / length > 0.99,
            "normal disagrees with winding"
        );
        volume += dot(a, cross(b, c)) / 6.0;
        let keys = [a, b, c].map(key);
        for edge in 0..3 {
            let (from, to) = (keys[edge], keys[(edge + 1) % 3]);
            ensure!(directed.insert((from, to)), "duplicate directed edge");
            links.entry(from).or_default().push(to);
            links.entry(to).or_default().push(from);
        }
    }
    for &(from, to) in &directed {
        ensure!(directed.contains(&(to, from)), "open or inverted edge");
    }
    let mut seen = BTreeSet::new();
    let mut pending = vec![*links.keys().next().unwrap()];
    while let Some(vertex) = pending.pop() {
        if seen.insert(vertex) {
            pending.extend(links[&vertex].iter().copied());
        }
    }
    ensure!(seen.len() == links.len(), "disconnected material");
    ensure!(volume > 0.0, format!("enclosed volume {volume}"));
    Ok(volume)
}

fn assert_closed_positive_body(mesh: &MeshData, label: &str) -> f64 {
    closed_positive_volume(mesh).unwrap_or_else(|error| panic!("{label}: {error}"))
}

/// Roof height at `(x, y)` from the authored tilt and the sagitta of the dish.
fn surface(spec: &Value, x: f64, y: f64) -> f64 {
    let number = |value: &Value| value.as_f64().unwrap();
    let depth = number(&spec["dishDepth"]);
    let half = number(&spec["topSize"]["y"]) / 2.0;
    let radius = (half * half + depth * depth) / (2.0 * depth);
    let distance = if spec["spherical"] == true {
        x.hypot(y)
    } else {
        y.abs()
    };
    let sag = (depth - (radius - (radius * radius - distance * distance).max(0.0).sqrt()))
        .clamp(0.0, depth);
    number(&spec["height"]) + y * number(&spec["tilt"]).to_radians().tan() - sag
}

/// The roof keeps its 1.2 mm thickness over a cavity, and the stem stays solid beside its blind
/// socket. Guards the print union against collapsing or filling material while it is repaired.
fn assert_roof_thickness_and_cavity(spec: &Value, mesh: &MeshData, label: &str) {
    let roof = surface(spec, 5.0, -4.0);
    for (point, material, what) in [
        ([5.0, -4.0, roof - 0.6], true, "roof"),
        ([5.0, -4.0, roof - 1.5], false, "cavity under the roof"),
        ([5.0, -4.0, roof + 0.5], false, "air above the dish"),
        ([0.0, 8.5, 0.2], true, "side wall"),
        ([0.0, 6.0, 1.0], false, "cavity beside the stem"),
        ([2.5, 2.5, 1.0], true, "stem body"),
        ([0.137, 0.113, 1.0], false, "blind socket"),
    ] {
        assert_eq!(
            contains(mesh, point),
            material,
            "{label}: {what} at {point:?}"
        );
    }
}

/// DCS row 3 with a legend left a collinear sliver in the union that collapsed at f32 precision.
/// The simplified union must still be the same cap: roof, thickness, cavity and legend relief.
#[test]
fn dcs_row_three_legend_print_keeps_roof_cavity_and_relief() {
    for mount in ["mx", "alps"] {
        let label = format!("dcs row 3 {mount} 'A'");
        let bare_spec = cap(DCS_ROW_3, mount, "");
        let bare = assert_closed_positive_body(&print_body(&bare_spec).unwrap(), "bare");
        let spec = cap(DCS_ROW_3, mount, "A");
        let mesh = print_body(&spec).unwrap_or_else(|error| panic!("{label}: {error}"));
        let volume = assert_closed_positive_body(&mesh, &label);
        // An "A" has roughly 8 mm² of top face at 0.03 mm relief: a fraction of a cubic millimetre.
        assert!(
            volume > bare + 0.05 && volume < bare + 0.6,
            "{label}: legend adds {}",
            volume - bare
        );
        assert_roof_thickness_and_cavity(&spec, &mesh, &label);
    }
}

/// Text the product produces from bindings that the display legend's triangulated caps could not
/// close (`T`, `1`, `4`, labels with straight strokes), or that printed disconnected (backtick and
/// arrows). Each must now print as one closed body that gains legend material on both roofs.
#[test]
fn ordinary_legend_text_prints_as_one_closed_body() {
    let legends = [
        "A",
        "O",
        "T",
        "1",
        "4",
        "Q",
        "Enter",
        "Backspace",
        "Shift",
        "F12",
        "\u{2190}",
        "\u{2192}",
        "`",
    ];
    // Collect every refusal so one run names each failing legend and its reason. The cap bodies
    // are independent, so every (roof, legend) pair prints on its own thread.
    let roofs = [DCS_ROW_3, SA_ROW_5];
    let bare = roofs.map(|roof| {
        assert_closed_positive_body(&print_body(&cap(roof, "mx", "")).unwrap(), "bare")
    });
    let work: Vec<_> = roofs
        .iter()
        .enumerate()
        .flat_map(|(roof_index, roof)| {
            legends
                .iter()
                .map(move |legend| (roof_index, *roof, *legend))
        })
        .collect();
    let mut refused: Vec<(usize, String)> = std::thread::scope(|scope| {
        let handles: Vec<_> = work
            .iter()
            .enumerate()
            .map(|(index, (roof_index, roof, legend))| {
                scope.spawn(move || {
                    let label = format!("{} '{legend}'", roof.0);
                    match print_body(&cap(*roof, "mx", legend))
                        .and_then(|mesh| closed_positive_volume(&mesh))
                    {
                        Err(error) => Some((index, format!("{label}: {error}"))),
                        Ok(volume) if volume <= bare[*roof_index] + 0.005 => {
                            Some((index, format!("{label}: legend material missing")))
                        }
                        Ok(_) => None,
                    }
                })
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|handle| handle.join().unwrap())
            .collect()
    });
    refused.sort_by_key(|(index, _)| *index);
    assert!(
        refused.is_empty(),
        "legends refused to print:\n{}",
        refused
            .into_iter()
            .map(|(_, message)| message)
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// A long stroke spans most of the dish. Its relief must follow the roof (about 0.03 mm above the
/// analytic surface, never below the 0.02 mm the roof mesh's own chords leave) instead of
/// standing on the chord of a coarse triangulation, which rises tenths of a millimetre mid-span.
#[test]
fn long_stroke_relief_follows_the_dish() {
    let spec = cap(SA_ROW_5, "mx", "-");
    let mesh = print_body(&spec).unwrap();
    assert_closed_positive_body(&mesh, "-");
    // The dash is centred on the cap, so its centre line runs along y = 0.
    for x in [-5.0, -3.0, -1.0, 0.0, 1.0, 3.0, 5.0] {
        let z = surface(&spec, x, 0.0);
        assert!(
            contains(&mesh, [x, 0.0, z + 0.02]),
            "'-' lost relief at x={x}"
        );
        assert!(
            !contains(&mesh, [x, 0.0, z + 0.06]),
            "'-' stands proud at x={x}"
        );
    }
}

/// Repair must not hide invalid input: material that is not joined to the cap, or an operand that
/// is not a closed solid, still refuses, and the public operation returns no fallback bodies.
#[test]
fn genuinely_separate_or_open_print_operands_still_refuse() {
    let spec = cap(DCS_ROW_3, "mx", "A");
    let typed: KeycapSpec = serde_json::from_value(spec.clone()).unwrap();
    let mut floating = build_keycap_mesh_template(&typed).unwrap();
    for x in floating.stem.positions.iter_mut().step_by(3) {
        *x += 40.0;
    }
    let error = printing::mesh(&floating, &typed)
        .err()
        .expect("floating stem must refuse");
    assert!(error.contains("disconnected material"), "{error}");

    let mut open = build_keycap_mesh_template(&typed).unwrap();
    open.stem.positions.truncate(open.stem.positions.len() - 9);
    open.stem.normals.truncate(open.stem.positions.len());
    assert!(
        printing::mesh(&open, &typed).is_err(),
        "open stem operand must not print"
    );

    let mut thin = spec;
    thin["wallThickness"] = 0.1.into();
    let request = json!({"revision": 4, "printable": true, "specs": [thin]});
    assert!(super::build(serde_json::from_value(request).unwrap()).is_err());
}
