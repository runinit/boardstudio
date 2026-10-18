//! Product-preset print coverage. Each check compares the fused print mesh with expectations derived
//! from the spec and the frozen stem constants, never with the display template that built it.
use super::{adapter, mesh_template::build_keycap_mesh_template, printing, MeshData};
use boardstudio_contracts::KeycapSpec;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn presets() -> Value {
    serde_json::from_str(include_str!("../../../tests/fixtures/keycap-product-presets.json")).unwrap()
}

/// The product spec for `profile` at `row`, using the preset table that `core` is tested against.
fn product_spec(profile: &str, row: u64, mount: &str, legend: &str) -> Value {
    let presets = presets();
    let preset = presets["presets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["profile"] == profile && p["row"] == row)
        .unwrap_or_else(|| panic!("no {profile} row {row} preset"));
    json!({
        "id": format!("{profile}-r{row}-{mount}"), "reference": "SW1", "profile": profile,
        "mount": mount, "row": row, "size": presets["size"], "topSize": presets["topSize"],
        "height": preset["height"], "tilt": preset["tilt"], "dishDepth": preset["dishDepth"],
        "spherical": preset["spherical"], "wallThickness": presets["wallThickness"],
        "pose": {"at": {"x": 0, "y": 0}, "rotation": 0}, "side": "front", "z": 0, "travel": 4,
        "legend": legend, "color": "#eeeeee", "legendColor": "#222222"
    })
}

fn print_body(spec: &Value) -> Result<MeshData, String> {
    let request = json!({"revision": 3, "printable": true, "specs": [spec]});
    let result = super::build(serde_json::from_value(request).unwrap())?;
    assert!(result.step.is_empty(), "printing never creates keycap STEP");
    let mut bodies = result.bodies.unwrap();
    assert_eq!(bodies.len(), 1, "cap, stem and legend are one print body");
    let body = bodies.remove(0);
    assert_eq!(body.id, format!("keycap:{}", spec["id"].as_str().unwrap()));
    assert_eq!(body.name, "SW1");
    Ok(body.mesh)
}

type Triangle = [[f64; 3]; 3];

fn triangles(mesh: &MeshData) -> Vec<Triangle> {
    mesh.positions
        .as_chunks::<9>()
        .0
        .iter()
        .map(|t| [0, 1, 2].map(|i| [0, 1, 2].map(|a| f64::from(t[3 * i + a]))))
        .collect()
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn volume(mesh: &MeshData) -> f64 {
    triangles(mesh).iter().map(|[a, b, c]| dot(*a, cross(*b, *c)) / 6.0).sum()
}

/// Winding-number point membership; independent of how the mesh was built.
fn contains(mesh: &MeshData, point: [f64; 3]) -> bool {
    let mut angle = 0.0;
    for [a, b, c] in triangles(mesh) {
        let [a, b, c] = [a, b, c].map(|p| [p[0] - point[0], p[1] - point[1], p[2] - point[2]]);
        let length = |p: [f64; 3]| dot(p, p).sqrt();
        let denominator = length(a) * length(b) * length(c)
            + dot(a, b) * length(c)
            + dot(b, c) * length(a)
            + dot(c, a) * length(b);
        angle += 2.0 * dot(a, cross(b, c)).atan2(denominator);
    }
    angle.abs() > std::f64::consts::TAU
}

struct Shape {
    volume: f64,
    bounds: [[f64; 3]; 2],
}

/// Closed, consistently oriented, one connected positive-volume surface with unit outward normals.
fn assert_print_shape(mesh: &MeshData, label: &str) -> Shape {
    assert_eq!(mesh.positions.len(), mesh.normals.len(), "{label}");
    let key = |p: [f64; 3]| p.map(|v| (v as f32).to_bits());
    let mut directed = BTreeMap::new();
    let mut parent = BTreeMap::new();
    fn root(parent: &mut BTreeMap<[u32; 3], [u32; 3]>, key: [u32; 3]) -> [u32; 3] {
        let up = *parent.entry(key).or_insert(key);
        if up == key {
            return key;
        }
        let top = root(parent, up);
        parent.insert(key, top);
        top
    }
    let mut bounds = [[f64::INFINITY; 3], [f64::NEG_INFINITY; 3]];
    for (index, triangle) in triangles(mesh).iter().enumerate() {
        let face = cross(
            [0, 1, 2].map(|a| triangle[1][a] - triangle[0][a]),
            [0, 1, 2].map(|a| triangle[2][a] - triangle[0][a]),
        );
        let length = dot(face, face).sqrt();
        assert!(length > 0.0, "{label}: zero-area triangle {index}");
        let normal = [0, 1, 2].map(|a| f64::from(mesh.normals[9 * index + a]));
        assert!((dot(normal, normal).sqrt() - 1.0).abs() < 1e-5, "{label}: normal length");
        assert!(dot(normal, face) / length > 0.999, "{label}: normal disagrees with winding {index}");
        for edge in 0..3 {
            let (a, b) = (key(triangle[edge]), key(triangle[(edge + 1) % 3]));
            assert_ne!(a, b, "{label}: collapsed edge");
            assert!(directed.insert((a, b), index).is_none(), "{label}: duplicate directed edge");
            let (ra, rb) = (root(&mut parent, a), root(&mut parent, b));
            parent.insert(ra, rb);
        }
        for point in triangle {
            for axis in 0..3 {
                bounds[0][axis] = bounds[0][axis].min(point[axis]);
                bounds[1][axis] = bounds[1][axis].max(point[axis]);
            }
        }
    }
    for &(a, b) in directed.keys() {
        assert!(directed.contains_key(&(b, a)), "{label}: open or inconsistently oriented edge");
    }
    let vertices = parent.keys().copied().collect::<Vec<_>>();
    let components = vertices.iter().map(|&v| root(&mut parent, v)).collect::<std::collections::BTreeSet<_>>();
    assert_eq!(components.len(), 1, "{label}: disconnected material");
    let volume = volume(mesh);
    assert!(volume > 0.0, "{label}: negative or zero enclosed volume {volume}");
    Shape { volume, bounds }
}

fn spec_f64(spec: &Value, field: &str) -> f64 {
    spec[field].as_f64().unwrap()
}

fn sag(spec: &Value, x: f64, y: f64) -> f64 {
    let depth = spec_f64(spec, "dishDepth");
    if depth == 0.0 {
        return 0.0;
    }
    let half = spec_f64(&spec["topSize"], "y") / 2.0;
    let radius = (half * half + depth * depth) / (2.0 * depth);
    let distance = if spec["spherical"] == true { x.hypot(y) } else { y.abs() };
    (depth - (radius - (radius * radius - distance * distance).max(0.0).sqrt())).clamp(0.0, depth)
}

fn surface(spec: &Value, x: f64, y: f64) -> f64 {
    let tilt = spec_f64(spec, "tilt").to_radians().tan();
    spec_f64(spec, "height") + y * tilt - sag(spec, x, y)
}

/// Rounded-rectangle loft volume of the undished, untilted shell (the display tests pin the
/// flat shell against this same figure).
fn loft(width: f64, depth: f64, top_width: f64, top_depth: f64, height: f64) -> f64 {
    let area = |t: f64| {
        let w = width + (top_width - width) * t;
        let d = depth + (top_depth - depth) * t;
        let radius = 0.8_f64.min(w / 5.0).min(d / 5.0);
        w * d - (4.0 - 8.0 * 22.5_f64.to_radians().sin()) * radius * radius
    };
    height * (area(0.0) + 4.0 * area(0.5) + area(1.0)) / 6.0
}

fn undished_shell_volume(spec: &Value) -> f64 {
    let (size, top) = (&spec["size"], &spec["topSize"]);
    let (wall, height) = (spec_f64(spec, "wallThickness"), spec_f64(spec, "height"));
    loft(spec_f64(size, "x"), spec_f64(size, "y"), spec_f64(top, "x"), spec_f64(top, "y"), height)
        - loft(
            spec_f64(size, "x") - 2.0 * wall,
            spec_f64(size, "y") - 2.0 * wall,
            spec_f64(top, "x") - 2.0 * wall,
            spec_f64(top, "y") - 2.0 * wall,
            height - wall,
        )
}

/// Blind-socket stem footprint rectangles, their socket area and depth, from the frozen mount sizes.
fn mount(mount: &str) -> (Vec<[f64; 4]>, f64, f64) {
    match mount {
        "mx" => (vec![[-3., -3., 3., 3.]], 9.23, 3.6),
        "alps" => (vec![[-3., -3., 3., 3.]], 4.6 * 2.4, 3.6),
        "choc-v1" => (vec![[-4.95, -2.1, -0.75, 2.1], [0.75, -2.1, 4.95, 2.1]], 2.0 * 1.3 * 3.3, 2.3),
        "choc-v2" => (vec![[-3., -3., 3., 3.]], 9.23, 2.3),
        other => panic!("unknown mount {other}"),
    }
}

/// Stem material minus its overlap with the roof, integrated over the stem footprint.
fn stem_net_volume(spec: &Value) -> f64 {
    let (wall, height) = (spec_f64(spec, "wallThickness"), spec_f64(spec, "height"));
    let (footprint, socket_area, socket_depth) = mount(spec["mount"].as_str().unwrap());
    let stem_top = height - wall + 0.1;
    let tilt = spec_f64(spec, "tilt").to_radians().tan();
    let mut stem = -socket_area * socket_depth;
    let mut overlap = 0.0;
    let steps = 120;
    for [x0, y0, x1, y1] in footprint {
        stem += (x1 - x0) * (y1 - y0) * stem_top;
        for i in 0..steps {
            for j in 0..steps {
                let x = x0 + (x1 - x0) * (i as f64 + 0.5) / f64::from(steps);
                let y = y0 + (y1 - y0) * (j as f64 + 0.5) / f64::from(steps);
                let underside = height - wall + y * tilt - sag(spec, x, y);
                overlap += (stem_top - underside).clamp(0.0, wall) * (x1 - x0) * (y1 - y0)
                    / f64::from(steps * steps);
            }
        }
    }
    stem - overlap
}

fn assert_independent_material(spec: &Value, mesh: &MeshData, shape: &Shape, label: &str) {
    let (size, top) = (&spec["size"], &spec["topSize"]);
    let (half_x, half_y) = (spec_f64(size, "x") / 2.0, spec_f64(size, "y") / 2.0);
    let wall = spec_f64(spec, "wallThickness");
    let expected_top = spec_f64(spec, "height")
        + spec_f64(top, "y") / 2.0 * spec_f64(spec, "tilt").to_radians().tan().abs();
    for (actual, expected, what) in [
        (shape.bounds[0][0], -half_x, "min x"),
        (shape.bounds[1][0], half_x, "max x"),
        (shape.bounds[0][1], -half_y, "min y"),
        (shape.bounds[1][1], half_y, "max y"),
        (shape.bounds[0][2], 0.0, "open underside plane"),
        (shape.bounds[1][2], expected_top, "tilted rim height"),
    ] {
        assert!((actual - expected).abs() < 1e-4, "{label}: {what} {actual}, expected {expected}");
    }
    // The printed union is the display shell plus the stem, less their overlap. The display shell is
    // closed and pinned by the display tests; the dish only removes wall-top material from it.
    let typed: KeycapSpec = serde_json::from_value(spec.clone()).unwrap();
    let shell = volume(&build_keycap_mesh_template(&typed).unwrap().shell);
    let undished = undished_shell_volume(spec);
    let removed = undished - shell;
    let dish = spec_f64(spec, "dishDepth");
    if dish == 0.0 {
        assert!(removed.abs() < 0.05, "{label}: flat shell volume {shell}, expected {undished}");
    } else {
        // The dish only moves the roof and shifts the wall tops, by at most the dish depth along the
        // top perimeter, so it changes the shell by far less than that perimeter band.
        let ceiling = dish * 2.0 * (spec_f64(top, "x") + spec_f64(top, "y")) * wall;
        assert!(removed.abs() < ceiling, "{label}: dish changed the shell by {removed} mm³, bound {ceiling}");
    }
    let expected = shell + stem_net_volume(spec);
    assert!(
        (shape.volume - expected).abs() < 0.4,
        "{label}: material volume {:.3} mm³, independent expectation {expected:.3}",
        shape.volume
    );
    let (px, py) = (5.0, -4.0);
    let roof = surface(spec, px, py);
    for (point, material, what) in [
        ([0.0, half_y - wall / 2.0, 0.2], true, "side wall"),
        ([0.0, 6.0, 1.0], false, "cavity beside the stem"),
        ([px, py, roof - wall / 2.0], true, "roof"),
        ([px, py, roof - wall - 0.3], false, "cavity under the roof"),
        ([px, py, roof + 0.5], false, "air above the dish"),
    ] {
        assert_eq!(contains(mesh, point), material, "{label}: {what} at {point:?}");
    }
    let socket_depth = mount(spec["mount"].as_str().unwrap()).2;
    let cx = if spec["mount"] == "choc-v1" { 2.85 } else { 0.0 };
    let wall_probe = if spec["mount"] == "choc-v1" { [4.5, 0.1] } else { [2.5, 2.5] };
    for (point, material, what) in [
        ([wall_probe[0], wall_probe[1], 1.0], true, "stem body"),
        ([cx + 0.137, 0.113, socket_depth - 0.05], false, "blind socket"),
        ([cx + 0.137, 0.113, socket_depth + 0.05], true, "socket floor"),
    ] {
        assert_eq!(contains(mesh, point), material, "{label}: {what} at {point:?}");
    }
}

/// Representative product-preset combinations chosen for risk: every dish type, both tilt signs,
/// the lowest and tallest profiles, every mount, and legends with a counter or several glyphs.
const CASES: &[(&str, u64, &str, &str)] = &[
    ("cherry", 1, "mx", "A"),     // cylindrical dish, tilt -6
    ("oem", 5, "alps", "O"),      // cylindrical dish, tilt +8, glyph counter
    ("dcs", 3, "mx", "A"),        // untilted dish: reproduced legend/roof graze
    ("dsa", 3, "choc-v2", "A1"),  // untilted spherical dish, two glyphs
    ("sa", 1, "mx", "O"),         // tall spherical dish, tilt -8
    ("hi-pro", 5, "alps", "A"),   // tallest profile, tilt +8
    ("g20", 3, "choc-v1", ""),    // flat roof, no legend, split Choc V1 stems
    ("choc", 3, "choc-v1", "O"),  // low profile spherical dish
    ("choc", 3, "choc-v2", ""),   // low profile with a cross stem
];

#[test]
fn product_presets_print_as_one_closed_positive_body_with_independent_material() {
    for &(profile, row, mount, legend) in CASES {
        let label = format!("{profile} row {row} {mount} '{legend}'");
        let spec = product_spec(profile, row, mount, legend);
        let mesh = print_body(&spec).unwrap_or_else(|error| panic!("{label}: {error}"));
        let shape = assert_print_shape(&mesh, &label);
        // The legend adds relief and perturbs the independent volume only by its own raised area.
        let bare = if legend.is_empty() {
            None
        } else {
            let mut bare = spec.clone();
            bare["legend"] = "".into();
            Some(print_body(&bare).unwrap_or_else(|error| panic!("{label} bare: {error}")))
        };
        let reference = bare.as_ref().unwrap_or(&mesh);
        let reference_shape = if bare.is_some() {
            assert_print_shape(reference, &label)
        } else {
            Shape { volume: shape.volume, bounds: shape.bounds }
        };
        assert_independent_material(&spec, reference, &reference_shape, &label);
        if bare.is_some() {
            assert_legend_detail(&spec, &mesh, shape.volume - reference_shape.volume, &label);
        }
    }
}

/// The raised legend must survive the union with relief on the roof: its top-face centres are
/// material, glyph counters are not, and the added volume tracks the legend's raised area.
fn assert_legend_detail(spec: &Value, mesh: &MeshData, added: f64, label: &str) {
    let template = build_keycap_mesh_template(&serde_json::from_value::<KeycapSpec>(spec.clone()).unwrap()).unwrap();
    let legend = template.legend.expect("legend mesh");
    let mut area = 0.0;
    let mut probes = Vec::new();
    for ([a, b, c], normal) in triangles(&legend).into_iter().zip(legend.normals.as_chunks::<9>().0) {
        if normal[2] < 0.9 {
            continue;
        }
        let projected = ((b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1])).abs() / 2.0;
        area += projected;
        if projected > 0.02 {
            let centre = [(a[0] + b[0] + c[0]) / 3.0, (a[1] + b[1] + c[1]) / 3.0];
            probes.push(centre);
        }
    }
    assert!(!probes.is_empty(), "{label}: legend has no top faces");
    let step = (probes.len() / 24).max(1);
    for centre in probes.iter().step_by(step) {
        let z = surface(spec, centre[0], centre[1]);
        assert!(contains(mesh, [centre[0], centre[1], z + 0.02]), "{label}: legend lost at {centre:?}");
        // The relief stays near its nominal 0.03 mm rise: the roof mesh and the legend caps each
        // deviate from the analytic dish by at most a hundredth of a millimetre.
        assert!(!contains(mesh, [centre[0], centre[1], z + 0.06]), "{label}: legend stands proud at {centre:?}");
    }
    let raised = area * 0.03;
    assert!(
        added > 0.5 * raised && added < 1.5 * raised,
        "{label}: legend adds {added:.4} mm³ against {raised:.4} mm³ at the nominal 0.03 mm relief"
    );
    if spec["legend"] == "O" {
        assert!(!contains(mesh, [0.0, 0.0, surface(spec, 0.0, 0.0) + 0.015]), "{label}: O counter filled");
    }
}

#[test]
fn print_placement_keeps_orientation_for_rotated_back_side_two_unit_cap() {
    let mut spec = product_spec("cherry", 2, "mx", "A");
    spec["size"] = json!({"x": 37.25, "y": 18.2});
    spec["topSize"] = json!({"x": 32.25, "y": 13.2});
    let local = print_body(&spec).unwrap();
    let local_shape = assert_print_shape(&local, "local 2u");
    spec["pose"] = json!({"at": {"x": 30, "y": -20}, "rotation": 90});
    spec["side"] = "back".into();
    spec["z"] = (-8.4).into();
    let placed = print_body(&spec).unwrap();
    let shape = assert_print_shape(&placed, "back-side rotated 2u");
    assert!((shape.volume - local_shape.volume).abs() < 1e-3, "placement preserves material");
    // 90 degrees about the origin swaps the extents; the back side hangs below its mounting plane.
    let expected = [[30.0 - 9.1, -20.0 - 18.625, -8.4 - local_shape.bounds[1][2]], [30.0 + 9.1, -20.0 + 18.625, -8.4]];
    for side in 0..2 {
        for axis in 0..3 {
            assert!(
                (shape.bounds[side][axis] - expected[side][axis]).abs() < 1e-3,
                "bound {side}/{axis} = {}, expected {}",
                shape.bounds[side][axis],
                expected[side][axis]
            );
        }
    }
}

#[test]
fn failed_print_refuses_instead_of_returning_display_pieces() {
    let spec = product_spec("g20", 3, "mx", "A");
    let spec_typed: KeycapSpec = serde_json::from_value(spec.clone()).unwrap();
    // Display output is several overlapping components and is never a print result.
    let display = adapter::build(std::slice::from_ref(&spec_typed)).unwrap();
    assert_eq!(display.len(), 2, "display keeps the cap and legend as separate bodies");

    // A stem that does not touch the cap is separate material: refuse, do not label it printable.
    let mut template = build_keycap_mesh_template(&spec_typed).unwrap();
    for x in template.stem.positions.iter_mut().step_by(3) {
        *x += 40.0;
    }
    let error = printing::mesh(&template, &spec_typed).err().expect("disconnected stem must refuse");
    assert!(error.contains("disconnected material"), "{error}");

    // An operand that is not a closed manifold is refused rather than repaired silently.
    let mut template = build_keycap_mesh_template(&spec_typed).unwrap();
    template.stem.positions.truncate(template.stem.positions.len() - 9);
    template.stem.normals.truncate(template.stem.positions.len());
    assert!(printing::mesh(&template, &spec_typed).is_err(), "open stem operand must not print");

    // The public operation carries the failure and offers no fallback bodies.
    let mut invalid = spec.clone();
    invalid["wallThickness"] = 0.1.into();
    let request = json!({"revision": 4, "printable": true, "specs": [invalid]});
    assert!(super::build(serde_json::from_value(request).unwrap()).is_err());
    let empty = json!({"revision": 5, "printable": true, "specs": []});
    assert!(super::build(serde_json::from_value(empty).unwrap()).is_err());
}

/// Every legend text the product derives from keymap bindings (`binding_legend`), plus the glyph
/// shapes that exposed open legend operands: straight strokes (`T`, `1`, `4`), multi-glyph runs,
/// punctuation and arrows. Each must print as one closed positive body that gains legend material.
#[test]
fn product_legend_vocabulary_prints_closed_with_retained_relief() {
    let mut legends = [
        "Space", "Enter", "Esc", "Backspace", "Tab", "Shift", "Ctrl", "Alt", "Super", "-", "=", "[", "]",
        "\\", ";", "'", ",", ".", "/", "`", "\u{2191}", "\u{2193}", "\u{2190}", "\u{2192}", "A1", "11",
        "F12", "Delete", "T", "1", "4",
    ]
    .map(String::from)
    .to_vec();
    legends.extend(('A'..='Z').step_by(5).map(String::from));
    // A tilted spherical dish and an untilted cylindrical dish: the two roofs the relief must follow.
    for (profile, row, mount) in [("sa", 5, "mx"), ("dcs", 3, "alps")] {
        let bare_spec = product_spec(profile, row, mount, "");
        let bare = assert_print_shape(&print_body(&bare_spec).unwrap(), "bare");
        for legend in &legends {
            let label = format!("{profile} row {row} '{legend}'");
            let spec = product_spec(profile, row, mount, legend);
            let mesh = print_body(&spec).unwrap_or_else(|error| panic!("{label}: {error}"));
            let shape = assert_print_shape(&mesh, &label);
            assert!(shape.volume > bare.volume + 0.005, "{label}: legend material missing");
            
        }
    }
}
