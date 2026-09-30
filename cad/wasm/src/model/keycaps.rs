//! Independent Cadrum/OCCT keycap construction. Presets arrive from the Rust core.
use super::*;
use serde::Serialize;
use std::{cell::RefCell, collections::VecDeque};
mod lettering;

#[derive(Clone, Deserialize, Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct Spec {
    id: String,
    reference: String,
    profile: String,
    mount: String,
    row: u8,
    size: Point,
    top_size: Point,
    height: f64,
    tilt: f64,
    dish_depth: f64,
    spherical: bool,
    wall_thickness: f64,
    pose: Pose,
    side: String,
    z: f64,
    travel: f64,
    legend: String,
    color: String,
    legend_color: String,
}
#[derive(Clone, Deserialize, Serialize, Debug)]
struct Point {
    x: f64,
    y: f64,
}
#[derive(Clone, Deserialize, Serialize, Debug)]
struct Pose {
    at: Point,
    rotation: f64,
}
#[derive(Deserialize)]
struct Request {
    revision: u64,
    specs: Vec<Spec>,
    #[serde(default)]
    export: bool,
}
struct Template {
    cap: Vec<Solid>,
    legends: Vec<Solid>,
    cap_mesh: MeshData,
    legend_mesh: Option<MeshData>,
}
thread_local! { static CACHE: RefCell<VecDeque<(String,Template)>> = const { RefCell::new(VecDeque::new()) }; }

#[wasm_bindgen]
pub fn build_keycaps(input: JsValue) -> Result<JsValue, JsValue> {
    let request: Request = deserialize(input)?;
    case_result_to_js(build(request).map_err(js_error)?)
}
fn validate(s: &Spec) -> Result<(), String> {
    let numbers = [
        s.size.x,
        s.size.y,
        s.top_size.x,
        s.top_size.y,
        s.height,
        s.tilt,
        s.dish_depth,
        s.wall_thickness,
        s.pose.at.x,
        s.pose.at.y,
        s.pose.rotation,
        s.z,
        s.travel,
    ];
    if numbers.iter().any(|n| !n.is_finite())
        || !(12.0..=150.0).contains(&s.size.x)
        || !(12.0..=150.0).contains(&s.size.y)
        || !(3.0..=20.0).contains(&s.height)
        || s.tilt.abs() > 12.0
        || !(0.0..=0.8).contains(&s.dish_depth)
        || !(0.8..=2.0).contains(&s.wall_thickness)
        || s.top_size.x < 5.0
        || s.top_size.y < 5.0
        || s.top_size.x > s.size.x - 2.0
        || s.top_size.y > s.size.y - 2.0
        || !(1..=5).contains(&s.row)
        || !["mx", "choc-v1", "choc-v2", "alps"].contains(&s.mount.as_str())
        || !["front", "back"].contains(&s.side.as_str())
        || s.legend.chars().count() > 12
        || s.legend.chars().any(|c| c.is_control())
    {
        return Err("Invalid keycap CAD parameters".into());
    }
    Ok(())
}
fn wire(width: f64, depth: f64, z: f64, tilt: f64) -> Result<Vec<Edge>, String> {
    let r = 0.8_f64.min(width / 5.0).min(depth / 5.0);
    let mut points = Vec::new();
    for (x, y, start) in [
        (width / 2.0 - r, depth / 2.0 - r, 0.0_f64),
        (-width / 2.0 + r, depth / 2.0 - r, 90.0),
        (-width / 2.0 + r, -depth / 2.0 + r, 180.0),
        (width / 2.0 - r, -depth / 2.0 + r, 270.0),
    ] {
        for i in 0..=4 {
            let angle = (start + f64::from(i) * 22.5).to_radians();
            let px = x + r * angle.cos();
            let py = y + r * angle.sin();
            points.push(DVec3::new(px, py, z + py * tilt.to_radians().tan()));
        }
    }
    Edge::polygon(&points).map_err(cadrum_error)
}
fn loft(bottom: (f64, f64, f64), top: (f64, f64, f64), tilt: f64) -> Result<Solid, String> {
    let a = wire(bottom.0, bottom.1, bottom.2, 0.0)?;
    let b = wire(top.0, top.1, top.2, tilt)?;
    Solid::loft([a.iter(), b.iter()], true).map_err(cadrum_error)
}
fn template(s: &Spec) -> Result<Template, String> {
    validate(s)?;
    let wall = s.wall_thickness;
    let outer = loft(
        (s.size.x, s.size.y, 0.0),
        (s.top_size.x, s.top_size.y, s.height),
        s.tilt,
    )?;
    // Roof clearance is measured from the deepest point in the dish.
    let roof = s.height - s.dish_depth - wall;
    let cavity = loft(
        (s.size.x - 2.0 * wall, s.size.y - 2.0 * wall, -0.1),
        (s.top_size.x - 2.0 * wall, s.top_size.y - 2.0 * wall, roof),
        s.tilt,
    )?;
    let mut cap = (outer - cavity).build().map_err(cadrum_error)?;
    if s.dish_depth > 0.0 {
        let half = s.top_size.y / 2.0;
        let radius = (half * half + s.dish_depth * s.dish_depth) / (2.0 * s.dish_depth);
        let dish = if s.spherical {
            Solid::sphere(radius)
        } else {
            Solid::cylinder(radius, DVec3::new(s.size.x * 2.0, 0.0, 0.0))
                .translate(DVec3::new(-s.size.x, 0.0, 0.0))
        };
        let dish = dish
            .translate(DVec3::new(0.0, 0.0, s.height + radius - s.dish_depth))
            .rotate(
                DVec3::new(0.0, 0.0, s.height),
                DVec3::X,
                s.tilt.to_radians(),
            );
        cap = (cap - dish).build().map_err(cadrum_error)?;
    }
    let socket_depth = if s.mount.starts_with("choc-") {
        2.3
    } else {
        3.6
    };
    if roof < socket_depth + 0.2 {
        return Err("Keycap roof is too low for the socket; reduce wall thickness".into());
    }
    let mut stems = Vec::new();
    if s.mount == "choc-v1" {
        for x in [-2.85, 2.85] {
            let stem = Solid::cube(
                DVec3::new(x - 2.1, -2.1, 0.0),
                DVec3::new(x + 2.1, 2.1, s.height - wall + 0.1),
            );
            let socket = Solid::cube(
                DVec3::new(x - 0.65, -1.65, -0.1),
                DVec3::new(x + 0.65, 1.65, socket_depth),
            );
            stems.push((stem - socket).build().map_err(cadrum_error)?);
        }
    } else {
        let stem = Solid::cube(
            DVec3::new(-3.0, -3.0, 0.0),
            DVec3::new(3.0, 3.0, s.height - wall + 0.1),
        );
        let socket = if s.mount == "alps" {
            Solid::cube(
                DVec3::new(-2.3, -1.2, -0.1),
                DVec3::new(2.3, 1.2, socket_depth),
            )
        } else {
            let a = Solid::cube(
                DVec3::new(-2.1, -0.65, -0.1),
                DVec3::new(2.1, 0.65, socket_depth),
            );
            let b = Solid::cube(
                DVec3::new(-0.65, -2.1, -0.1),
                DVec3::new(0.65, 2.1, socket_depth),
            );
            (a + b).build().map_err(cadrum_error)?
        };
        stems.push((stem - socket).build().map_err(cadrum_error)?);
    }
    for stem in stems {
        cap = (cap + stem).build().map_err(cadrum_error)?;
    }
    let glyphs = lettering::solids(
        &s.legend,
        s.top_size.x - 2.0,
        s.top_size.y - 2.0,
        s.height - s.dish_depth - 0.35,
        s.dish_depth + 0.7,
    )?;
    let mut legends = Vec::new();
    if !glyphs.is_empty() {
        let mut cuts = Vec::new();
        for glyph in glyphs {
            // Transform the letter prism with the top plane, then clip to the dish.
            let glyph = glyph.rotate(
                DVec3::new(0.0, 0.0, s.height),
                DVec3::X,
                s.tilt.to_radians(),
            );
            let inlay = (cap.clone() * glyph.clone())
                .build_vec()
                .map_err(cadrum_error)?;
            if inlay.is_empty() {
                return Err("Legend does not intersect the keycap roof".into());
            }
            legends.extend(inlay);
            cuts.push(glyph);
        }
        let cut = cuts
            .into_iter()
            .map(Boolean::from)
            .reduce(|a, b| a + b)
            .unwrap();
        cap = (cap - cut).build().map_err(cadrum_error)?;
    }
    let cap = vec![cap];
    Ok(Template {
        cap_mesh: mesh_data(&cap)?,
        legend_mesh: if legends.is_empty() {
            None
        } else {
            Some(mesh_data(&legends)?)
        },
        cap,
        legends,
    })
}
fn placed_mesh(mesh: &MeshData, s: &Spec) -> MeshData {
    let a = s.pose.rotation.to_radians();
    let (sn, cs) = a.sin_cos();
    let sign = if s.side == "back" { -1.0 } else { 1.0 };
    let convert = |input: &[f32], normal: bool| {
        input
            .chunks_exact(3)
            .flat_map(|p| {
                let x = f64::from(p[0]);
                let y = f64::from(p[1]) * sign;
                let z = f64::from(p[2]) * sign;
                [
                    (x * cs - y * sn + if normal { 0.0 } else { s.pose.at.x }) as f32,
                    (x * sn + y * cs + if normal { 0.0 } else { s.pose.at.y }) as f32,
                    (z + if normal { 0.0 } else { s.z }) as f32,
                ]
            })
            .collect()
    };
    MeshData {
        positions: convert(&mesh.positions, false),
        normals: convert(&mesh.normals, true),
    }
}
fn placed_solid(solid: Solid, s: &Spec) -> Solid {
    let solid = if s.side == "back" {
        solid.rotate(DVec3::ZERO, DVec3::X, std::f64::consts::PI)
    } else {
        solid
    };
    solid
        .rotate(DVec3::ZERO, DVec3::Z, s.pose.rotation.to_radians())
        .translate(DVec3::new(s.pose.at.x, s.pose.at.y, s.z))
}
fn build(request: Request) -> Result<CaseResultData, String> {
    if request.specs.len() > 4096 {
        return Err("At most 4096 keycaps can be built per request".into());
    }
    let mut meshes = Vec::new();
    let mut solids = Vec::new();
    for s in &request.specs {
        validate(s)?;
        let key = format!(
            "{:?}",
            (
                &s.profile,
                &s.mount,
                s.row,
                &s.size,
                &s.top_size,
                s.height,
                s.tilt,
                s.dish_depth,
                s.spherical,
                s.wall_thickness,
                &s.legend
            )
        );
        CACHE.with(|cache| -> Result<(), String> {
            let mut cache = cache.borrow_mut();
            let template = if let Some(index) = cache.iter().position(|(k, _)| k == &key) {
                cache.remove(index).unwrap().1
            } else {
                template(s)?
            };
            meshes.push(BodyMeshData {
                id: format!("keycap:{}", s.id),
                name: s.reference.clone(),
                mesh: placed_mesh(&template.cap_mesh, s),
            });
            if let Some(mesh) = &template.legend_mesh {
                meshes.push(BodyMeshData {
                    id: format!("keycap-legend:{}", s.id),
                    name: format!("{} legend", s.reference),
                    mesh: placed_mesh(mesh, s),
                });
            }
            if request.export {
                solids.extend(
                    template
                        .cap
                        .iter()
                        .chain(&template.legends)
                        .cloned()
                        .map(|solid| placed_solid(solid, s)),
                );
            }
            cache.push_back((key, template));
            while cache.len() > 64
                || cache
                    .iter()
                    .map(|(_, t)| {
                        t.cap_mesh.positions.len()
                            + t.cap_mesh.normals.len()
                            + t.legend_mesh
                                .as_ref()
                                .map(|m| m.positions.len() + m.normals.len())
                                .unwrap_or(0)
                    })
                    .sum::<usize>()
                    * 4
                    > 8 * 1024 * 1024
            {
                cache.pop_front();
            }
            Ok(())
        })?;
    }
    if request.export && !solids.is_empty() {
        export_case(solids, request.revision, Some(meshes))
    } else {
        Ok(CaseResultData {
            revision: request.revision,
            step: vec![],
            mesh: MeshData {
                positions: vec![],
                normals: vec![],
            },
            bodies: Some(meshes),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn spec() -> Spec {
        serde_json::from_value(serde_json::json!({"id":"key","reference":"SW1","profile":"dsa","mount":"mx","row":3,"size":{"x":18.2,"y":18.2},"topSize":{"x":13.2,"y":13.2},"height":7.5,"tilt":0.0,"dishDepth":0.65,"spherical":true,"wallThickness":1.2,"pose":{"at":{"x":0,"y":0},"rotation":0},"side":"front","z":10,"travel":4,"legend":"O","color":"#eeeeee","legendColor":"#222222"})).unwrap()
    }
    #[test]
    fn hollow_socket_and_letter_are_exact_solids() {
        let s = spec();
        let t = template(&s).unwrap();
        assert!(!t.legends.is_empty());
        assert!(!t.cap[0].contains(DVec3::new(0.0, 0.0, 1.0))); // MX socket
        assert!(!t.cap[0].contains(DVec3::new(5.0, 0.0, 1.0))); // hollow underside
        assert!(t.cap[0].volume() > 0.0);
        assert!(!t.cap_mesh.positions.is_empty());
        let mut step = Vec::new();
        Solid::write_step(&t.cap, &mut step).unwrap();
        let roundtrip = Solid::read_step(&mut std::io::Cursor::new(step)).unwrap();
        assert!((roundtrip[0].volume() - t.cap[0].volume()).abs() < 0.01);
    }
    #[test]
    fn all_profiles_mounts_tilt_and_back_placement() {
        for (mount, height) in [
            ("mx", 10.0),
            ("choc-v1", 4.2),
            ("choc-v2", 4.2),
            ("alps", 10.0),
        ] {
            let mut s = spec();
            s.mount = mount.into();
            s.height = height;
            if height < 5.0 {
                s.dish_depth = 0.4;
            }
            s.legend = "A".into();
            s.tilt = 6.0;
            let t = template(&s).unwrap();
            s.side = "back".into();
            s.z = -8.0;
            let mesh = placed_mesh(&t.cap_mesh, &s);
            assert!(mesh.positions.chunks(3).all(|p| p[2] <= -8.0 + 1e-5));
        }
    }
    #[test]
    fn rejects_invalid_and_unsupported_glyphs() {
        let mut s = spec();
        s.size.x = f64::NAN;
        assert!(template(&s).is_err());
        let mut s = spec();
        s.legend = "🦀".into();
        assert!(template(&s).err().unwrap().contains("glyph"));
    }
}
