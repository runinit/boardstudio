// Staged beneath production construction.rs by prepare.py; never in the shipped crate.
use super::*;
use cadrum::bounded_experiment as diagnostic;
use i_overlay::core::{fill_rule::FillRule, overlay_rule::OverlayRule};
use i_overlay::float::single::SingleFloatOverlay;
use serde_json::{json, Value};

fn current(ir: &PreparedCase) -> Result<Vec<Solid>, String> {
    let mut result = Vec::new();
    for region in &ir.regions {
        let mut solids = build_region_upstream(&ir.body, region, BuildMode {
            mount_holes: true, gasket_profile: true, boss_unions: true,
            ..BuildMode::default()
        })?;
        apply_openings(&ir.body, region, &mut solids)?;
        result.extend(solids);
    }
    Ok(result)
}

fn path(points: &[Vec2]) -> Vec<[f64; 2]> {
    points.iter().map(|p| [p.x, p.y]).collect()
}

// Preserve every polygon pocket and analytic circular hole. Only the construction
// changes: material profiles at each depth are extruded, then joined per region.
fn profiles(ir: &PreparedCase) -> Result<Vec<Solid>, String> {
    if ir.body.kind != CaseKind::Plate || ir.body.gasket.is_some() {
        return Err("Profile experiment only supports prepared plate-kind bottoms".into());
    }
    let z0 = ir.body.z.unwrap_or(0.);
    let z1 = z0 + ir.body.thickness;
    let openings = ir.body.openings.as_deref().unwrap_or_default();
    let mut heights = vec![z0, z1];
    for opening in openings {
        heights.extend([opening.z, opening.z + opening.height].map(|z| z.clamp(z0, z1)));
    }
    heights.sort_by(f64::total_cmp);
    heights.dedup();
    let mut result = Vec::new();
    for region in &ir.regions {
        if !region.cavities.is_empty() || !region.gaskets.is_empty()
            || region.mounts.iter().any(|m| m.kind != MountKind::Hole)
        {
            return Err("Unsupported feature in profile experiment".into());
        }
        let mut bands = Vec::new();
        let authored: Vec<_> = region.outer.iter().chain(region.holes.iter().flatten())
            .chain(openings.iter().flat_map(|o| &o.points)).collect();
        for height in heights.windows(2) {
            let middle = height[0] + (height[1] - height[0]) / 2.;
            let mut loops = vec![path(&region.outer)];
            loops.extend(region.holes.iter().map(|hole| path(hole)));
            let mut shapes = vec![loops];
            for opening in openings {
                if middle > opening.z && middle < opening.z + opening.height {
                    shapes = shapes.overlay(&vec![path(&opening.points)], OverlayRule::Difference, FillRule::EvenOdd);
                }
            }
            for shape in shapes {
                let mut edges = Vec::new();
                for contour in shape {
                    // Overlay quantizes coordinates internally. Restore unchanged
                    // authored vertices so adjacent height bands share exact edges.
                    let points: Vec<_> = contour.into_iter().map(|p| {
                        authored.iter().find(|a| (a.x-p[0]).abs()<1e-6 && (a.y-p[1]).abs()<1e-6)
                            .map(|a| Vec2 { x:a.x,y:a.y }).unwrap_or(Vec2 { x:p[0],y:p[1] })
                    }).collect();
                    edges.extend(polygon_edges(&points, height[0])?);
                }
                bands.push(Solid::extrude(&edges, DVec3::Z * (height[1] - height[0])).map_err(cadrum_error)?);
            }
        }
        let expression = bands.iter().map(Boolean::from).reduce(|a, b| a + b)
            .ok_or("Profile experiment produced no bands")?;
        let mut solids = expression.build_vec().map_err(cadrum_error)?;
        let cutters = region.mounts.iter().map(|mount| {
            make_cylinder(&mount.at, z0, mount.hole_diameter, ir.body.thickness)
        }).collect::<Result<Vec<_>, _>>()?;
        subtract_many(&mut solids, &cutters)?;
        result.extend(solids);
    }
    Ok(result)
}

fn construct(ir: &PreparedCase, variant: &str) -> Result<Vec<Solid>, String> {
    if variant == "profiles" { profiles(ir) } else { current(ir) }
}

fn configure(variant: &str, timed: bool) -> Result<(), String> {
    let (edges, ids, split) = match variant {
        "control" => (false, false, false),
        "no-edges" => (true, false, false),
        "no-ids" => (false, true, false),
        "lean" | "profiles" => (true, true, false),
        "split-timing" => (false, false, true),
        _ => return Err("Unknown experiment variant".into()),
    };
    diagnostic::configure(edges, ids, timed, split);
    Ok(())
}

fn bounds(solids: &[Solid]) -> [[f64; 3]; 2] {
    let b = solids.iter().fold([DVec3::splat(f64::INFINITY), DVec3::splat(f64::NEG_INFINITY)], |a,s| {
        let b = s.bounding_box();
        [a[0].min(b[0]), a[1].max(b[1])]
    });
    [b[0].to_array(), b[1].to_array()]
}

fn run(input: &str, variant: &str, timed: bool) -> Result<Value, String> {
    let ir: PreparedCase = serde_json::from_str(input).map_err(cadrum_error)?;
    configure(variant, timed)?;
    let start = diagnostic::now();
    let solids = construct(&ir, variant)?;
    let built = diagnostic::now();
    let mesh = mesh_data(&solids)?;
    let meshed = diagnostic::now();
    let names = ["intersection", "buildWithFiller", "booleanPerform", "cellSelection",
        "cleanup", "history", "copy", "surfaceMeshing", "meshExtractionAndNormals", "edgeSampling"];
    let stages: serde_json::Map<String,Value> = names.into_iter().zip(diagnostic::snapshot())
        .map(|(name,(ms,calls))| (name.into(),json!({"durationMs":ms,"calls":calls}))).collect();
    // These checks are outside the timed construction/mesh interval for every variant.
    Ok(json!({"variant": variant, "constructionMs": built-start, "meshMs": meshed-built,
        "generationMs": meshed-start, "stages": stages, "solids":solids.len(),
        "volume":solids.iter().map(Solid::volume).sum::<f64>(), "bounds":bounds(&solids),
        "triangles":mesh.positions.len()/9, "meshBytes":(mesh.positions.len()+mesh.normals.len())*4}))
}

#[wasm_bindgen]
pub fn gasket_experiment(input: &str, variant: &str, timed: bool) -> Result<String, JsValue> {
    run(input,variant,timed).map(|v| v.to_string()).map_err(js_error)
}

#[wasm_bindgen]
pub fn gasket_experiment_mesh(input: &str, variant: &str) -> Result<String, JsValue> {
    let ir: PreparedCase = serde_json::from_str(input).map_err(js_error)?;
    configure(variant, false).map_err(js_error)?;
    let solids = construct(&ir,variant).map_err(js_error)?;
    let mesh = mesh_data(&solids).map_err(js_error)?;
    Ok(json!({"positions":mesh.positions,"normals":mesh.normals}).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Bounded geometry comparison; run separately from timing"]
    fn bounded_gasket_equivalence() {
        let input = std::fs::read_to_string(std::env::var("CAD_EXPERIMENT_INPUT").unwrap()).unwrap();
        let ir: PreparedCase = serde_json::from_str(&input).unwrap();
        let output = std::path::PathBuf::from(std::env::var("CAD_EXPERIMENT_OUTPUT").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        configure("control",false).unwrap();
        let reference = current(&ir).unwrap();
        let reference_mesh = mesh_data(&reference).unwrap();
        let volume = |solids: &[Solid]| solids.iter().map(Solid::volume).sum::<f64>();
        let mut results = Vec::new();
        for variant in ["control", "no-edges", "no-ids", "lean", "profiles", "split-timing"] {
            configure(variant,true).unwrap();
            let candidate = construct(&ir,variant).unwrap();
            let mesh = mesh_data(&candidate).unwrap();
            assert_eq!(candidate.len(),reference.len(),"{variant}");
            assert!((volume(&candidate)-volume(&reference)).abs()<0.01,"{variant} volume");
            for (a,b) in bounds(&candidate).into_iter().flatten().zip(bounds(&reference).into_iter().flatten()) {
                assert!((a-b).abs()<0.01,"{variant} bounds");
            }
            if ["control","no-edges","no-ids","lean"].contains(&variant) {
                assert_eq!(mesh.positions,reference_mesh.positions,"{variant} positions");
                assert_eq!(mesh.normals,reference_mesh.normals,"{variant} normals");
            }
            let mut step = Vec::new();
            Solid::write_step(candidate.iter(),&mut step).unwrap();
            std::fs::write(output.join(format!("{variant}.step")),&step).unwrap();
            let mut differences = Vec::new();
            for (a,b) in reference.iter().zip(&candidate) {
                for (left,right) in [(a,b),(b,a)] {
                    let diff = (Boolean::from(&left.clone())-Boolean::from(&right.clone())).build_vec().unwrap();
                    let difference: f64 = diff.iter().map(Solid::volume).sum();
                    if difference.abs() >= 0.01 {
                        std::fs::write(output.join(format!("{variant}-failure.json")),json!({
                            "difference":difference,"referenceVolume":a.volume(),"candidateVolume":b.volume(),
                            "referenceBounds":bounds(std::slice::from_ref(a)),"candidateBounds":bounds(std::slice::from_ref(b))
                        }).to_string()).unwrap();
                    }
                    assert!(difference.abs()<0.01,"{variant} material difference: {difference}; volumes {}/{}; bounds {:?}/{:?}",a.volume(),b.volume(),bounds(std::slice::from_ref(a)),bounds(std::slice::from_ref(b)));
                    differences.push(difference);
                }
            }
            let imported = Solid::read_step(&mut std::io::Cursor::new(step)).unwrap();
            assert_eq!(imported.len(),candidate.len());
            assert!((volume(&imported)-volume(&reference)).abs()<0.01);
            let result = run(&input,variant,true).unwrap();
            results.push(json!({"variant":variant,"materialDifferences":differences,
                "stepRoundtripVolume":volume(&imported),"metrics":result}));
            println!("{variant}: equivalent, STEP roundtrip passed");
        }
        std::fs::write(output.join("geometry.json"),serde_json::to_string_pretty(&results).unwrap()).unwrap();
    }
}
