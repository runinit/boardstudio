use super::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

struct CachedBody {
    key: String,
    // Final cache entries are immutable. Share ownership across region and body
    // caches; clone topology only when exporting or entering mutable CAD work.
    solids: Vec<Rc<Solid>>,
    mesh: MeshData,
    topology: usize,
}

thread_local! {
    static REGION_CACHE: RefCell<VecDeque<Rc<CachedBody>>> = const { RefCell::new(VecDeque::new()) };
    static PREVIEW_CACHE: RefCell<VecDeque<Rc<CachedBody>>> = const { RefCell::new(VecDeque::new()) };
}

fn cached_body(key: String, solids: Vec<Rc<Solid>>, mesh: MeshData) -> Rc<CachedBody> {
    let topology = solids
        .iter()
        .map(|solid| solid.iter_face().count() + solid.iter_edge().count())
        .sum();
    Rc::new(CachedBody {
        key,
        solids,
        mesh,
        topology,
    })
}

fn retained_complexity(cache: &VecDeque<Rc<CachedBody>>) -> (usize, usize) {
    cache.iter().fold((0, 0), |(solids, topology), entry| {
        (solids + entry.solids.len(), topology + entry.topology)
    })
}

const CACHE_SOLIDS: usize = 256;
const CACHE_TOPOLOGY: usize = 8192;

fn cached_preview(key: &str) -> Option<Rc<CachedBody>> {
    PREVIEW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.key == key)?;
        let entry = cache.remove(index)?;
        cache.push_back(Rc::clone(&entry));
        Some(entry)
    })
}

const STAGE_ENTRIES: usize = 16;
const STAGE_SOLIDS: usize = 32;
const STAGE_TOPOLOGY: usize = 2048;
struct StagedRegion {
    key: String,
    solids: Vec<Solid>,
    topology: usize,
}
thread_local! {
    static STAGE_CACHE: RefCell<VecDeque<StagedRegion>> = const { RefCell::new(VecDeque::new()) };
}

pub(super) fn upstream_region(
    body: &CaseBody,
    region: &PreparedRegion,
    build: impl FnOnce() -> Result<Vec<Solid>, String>,
) -> Result<Vec<Solid>, String> {
    // Restrict retention to the expensive fixed-corpus construction classes.
    // Without a downstream opening, the final region cache already owns this
    // result. Retaining an upstream deep copy cannot avoid any additional work.
    if (region.mounts.len() < 4 && region.holes.len() < 8)
        || !body
            .openings
            .iter()
            .flatten()
            .any(|opening| opening_intersects_region(body, region, opening))
    {
        return build();
    }
    let key = format!(
        "upstream:v1:cadrum-0.8.20:{:?}",
        (
            &body.kind,
            body.thickness,
            body.z,
            body.wall_height,
            &body.gasket,
            region
        )
    );
    if let Some(solids) = STAGE_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.key == key)?;
        let entry = cache.remove(index)?;
        let _stage = Stage::new("stageSolidCopy");
        // Cadrum Solid::clone deep-copies topology. Never give a cached handle
        // to Boolean evaluation, which is not guaranteed non-destructive.
        let solids = entry.solids.clone();
        cache.push_back(entry);
        Some(solids)
    }) {
        metrics::count("upstreamStageHits", 1);
        return Ok(solids);
    }
    metrics::count("upstreamStageMisses", 1);
    let solids = build()?;
    let topology = solids
        .iter()
        .map(|solid| solid.iter_face().count() + solid.iter_edge().count())
        .sum();
    if solids.len() <= STAGE_SOLIDS && topology <= STAGE_TOPOLOGY {
        let _stage = Stage::new("stageSolidCopy");
        let entry = StagedRegion {
            key,
            solids: solids.clone(),
            topology,
        };
        STAGE_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.push_back(entry);
            while cache.len() > STAGE_ENTRIES
                || cache.iter().map(|entry| entry.solids.len()).sum::<usize>() > STAGE_SOLIDS
                || cache.iter().map(|entry| entry.topology).sum::<usize>() > STAGE_TOPOLOGY
            {
                cache.pop_front();
                metrics::count("upstreamStageEvictions", 1);
            }
        });
    }
    Ok(solids)
}

/// Mesh-only generation; STEP serialization is reserved for export.
#[wasm_bindgen]
pub fn preview_body(
    input: JsValue,
    key: String,
    progress: js_sys::Function,
) -> Result<JsValue, JsValue> {
    let ir: PreparedCase = deserialize(input)?;
    if let Some(mesh) = cached_preview(&key) {
        metrics::count("bodyCacheHits", 1);
        return mesh_to_js(&mesh.mesh);
    }
    metrics::count("bodyCacheMisses", 1);
    let mut solids = Vec::new();
    let mut mesh = MeshData {
        positions: Vec::new(),
        normals: Vec::new(),
    };
    for (index, region) in ir.regions.iter().enumerate() {
        progress.call2(
            &JsValue::NULL,
            &JsValue::from_str("building"),
            &JsValue::from_f64(index as f64),
        )?;
        let entry = cached_region(&ir.body, region, || {
            progress
                .call2(
                    &JsValue::NULL,
                    &JsValue::from_str("tessellating"),
                    &JsValue::from_f64(index as f64),
                )
                .map(|_| ())
                .map_err(|_| "Progress callback failed".to_string())
        })
        .map_err(js_error)?;
        solids.extend(entry.solids.iter().cloned());
        let _stage = Stage::new("regionMeshCopy");
        metrics::count(
            "copiedMeshBytes",
            (entry.mesh.positions.len() + entry.mesh.normals.len()) * 4,
        );
        mesh.positions.extend_from_slice(&entry.mesh.positions);
        mesh.normals.extend_from_slice(&entry.mesh.normals);
    }
    let result = mesh_to_js(&mesh)?;
    PREVIEW_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.push_back(cached_body(key, solids, mesh));
        while cache.len() > 64
            || retained_complexity(&cache).0 > CACHE_SOLIDS
            || retained_complexity(&cache).1 > CACHE_TOPOLOGY
            || cache
                .iter()
                .map(|body| body.mesh.positions.len() * 8)
                .sum::<usize>()
                > 64 * 1024 * 1024
        {
            cache.pop_front();
            metrics::count("bodyCacheEvictions", 1);
        }
    });
    Ok(result)
}

fn region_key(body: &CaseBody, region: &PreparedRegion) -> String {
    format!(
        "region-mesh:v1:0.1:0.5:{:?}",
        (
            &body.kind,
            body.thickness,
            body.z,
            body.wall_height,
            &body
                .openings
                .iter()
                .flatten()
                .filter(|opening| opening_intersects_region(body, region, opening))
                .collect::<Vec<_>>(),
            &body.gasket,
            region
        )
    )
}

fn cached_region(
    body: &CaseBody,
    region: &PreparedRegion,
    tessellating: impl FnOnce() -> Result<(), String>,
) -> Result<Rc<CachedBody>, String> {
    let key = region_key(body, region);
    let cached = REGION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.key == key)?;
        let entry = cache.remove(index)?;
        cache.push_back(entry.clone());
        Some(entry)
    });
    if let Some(entry) = cached {
        metrics::count("regionCacheHits", 1);
        return Ok(entry);
    }
    metrics::count("regionCacheMisses", 1);
    let solids = build_region(body, region)?;
    tessellating()?;
    let mesh = match planar_mesh::plate_mesh(body, region) {
        Some(mesh) => mesh,
        None => mesh_data(&solids)?,
    };
    let entry = cached_body(key, solids.into_iter().map(Rc::new).collect(), mesh);
    REGION_CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.push_back(entry.clone());
        while cache.len() > 96
            || retained_complexity(&cache).0 > CACHE_SOLIDS
            || retained_complexity(&cache).1 > CACHE_TOPOLOGY
            || cache
                .iter()
                .map(|body| body.mesh.positions.len() * 8)
                .sum::<usize>()
                > 32 * 1024 * 1024
        {
            cache.pop_front();
            metrics::count("regionCacheEvictions", 1);
        }
    });
    Ok(entry)
}

#[wasm_bindgen]
pub fn export_cached_assembly(input: JsValue, keys: JsValue) -> Result<JsValue, JsValue> {
    // Internal output mode; the existing full-result boundary remains the default.
    let step_only = Reflect::get(&input, &JsValue::from_str("stepOnly"))?
        .as_bool()
        .unwrap_or(false);
    let ir: PreparedAssembly = deserialize(input)?;
    let keys: Vec<String> = deserialize(keys)?;
    if keys.len() != ir.bodies.len() {
        return Err(js_error("Invalid CAD cache keys"));
    }
    if step_only {
        return export_step_only(&ir, &keys);
    }
    let mut solids = Vec::new();
    let mut bodies = Vec::new();
    for (body, key) in ir.bodies.iter().zip(keys) {
        let cached = PREVIEW_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let index = cache.iter().position(|entry| entry.key == key)?;
            cache.remove(index)
        });
        let entry = if let Some(entry) = cached {
            metrics::count("bodyCacheHits", 1);
            entry
        } else {
            metrics::count("bodyCacheMisses", 1);
            let solids = build_body(body).map_err(js_error)?;
            let mesh = mesh_data(&solids).map_err(js_error)?;
            cached_body(key, solids.into_iter().map(Rc::new).collect(), mesh)
        };
        // Solid clones remain inside the CAD worker; only triangle buffers cross its boundary.
        solids.extend(entry.solids.iter().map(|solid| solid.as_ref().clone()));
        bodies.push(BodyMeshData {
            id: body.body.id.clone(),
            name: body.body.name.clone(),
            mesh: entry.mesh.clone(),
        });
        PREVIEW_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.push_back(entry);
            while cache.len() > 64
                || retained_complexity(&cache).0 > CACHE_SOLIDS
                || retained_complexity(&cache).1 > CACHE_TOPOLOGY
                || cache
                    .iter()
                    .map(|body| body.mesh.positions.len() * 8)
                    .sum::<usize>()
                    > 64 * 1024 * 1024
            {
                cache.pop_front();
                metrics::count("bodyCacheEvictions", 1);
            }
        });
    }
    case_result_to_js(export_case(solids, ir.revision, Some(bodies)).map_err(js_error)?)
}

fn export_step_only(ir: &PreparedAssembly, keys: &[String]) -> Result<JsValue, JsValue> {
    let mut solids = Vec::new();
    for (body, key) in ir.bodies.iter().zip(keys) {
        let cached = cached_preview(key);
        if let Some(entry) = cached {
            metrics::count("bodyCacheHits", 1);
            let _stage = Stage::new("cacheSolidCopy");
            solids.extend(entry.solids.iter().map(|solid| solid.as_ref().clone()));
            continue;
        }
        metrics::count("bodyCacheMisses", 1);
        for region in &body.regions {
            let key = region_key(&body.body, region);
            let cached = REGION_CACHE.with(|cache| {
                cache
                    .borrow()
                    .iter()
                    .find(|entry| entry.key == key)
                    .cloned()
            });
            if let Some(entry) = cached {
                metrics::count("regionCacheHits", 1);
                let _stage = Stage::new("cacheSolidCopy");
                solids.extend(entry.solids.iter().map(|solid| solid.as_ref().clone()));
            } else {
                metrics::count("regionCacheMisses", 1);
                solids.extend(build_region(&body.body, region).map_err(js_error)?);
            }
        }
    }
    if solids.is_empty() {
        return Err(js_error("OpenCascade returned an empty case solid"));
    }
    let mut step = Vec::new();
    {
        let _stage = Stage::new("stepSerialization");
        Solid::write_step(&solids, &mut step)
            .map_err(|error| js_error(format!("OpenCascade STEP export failed: {error}")))?;
    }
    let result = Object::new();
    set(&result, "revision", &JsValue::from_f64(ir.revision as f64))?;
    {
        let _stage = Stage::new("wasmStepCopy");
        set(&result, "step", &Uint8Array::from(step.as_slice()))?;
    }
    Ok(result.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(min: f64, max: f64) -> Vec<Vec2> {
        vec![
            Vec2 { x: min, y: min },
            Vec2 { x: max, y: min },
            Vec2 { x: max, y: max },
            Vec2 { x: min, y: max },
        ]
    }

    #[test]
    fn preview_ownership_survives_region_eviction_and_releases_with_assembly() {
        REGION_CACHE.with(|cache| cache.borrow_mut().clear());
        let body = CaseBody {
            id: "ownership".into(),
            name: "Ownership".into(),
            kind: CaseKind::Plate,
            thickness: 2.,
            z: None,
            wall_height: None,
            openings: None,
            gasket: None,
        };
        let region = PreparedRegion {
            outer: square(0., 20.),
            holes: vec![square(5., 15.)],
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![],
        };
        let entry = cached_region(&body, &region, || Ok(())).unwrap();
        let retained = cached_body("assembly".into(), entry.solids.clone(), entry.mesh.clone());
        assert!(Rc::ptr_eq(&entry.solids[0], &retained.solids[0]));
        let weak = Rc::downgrade(&entry.solids[0]);
        REGION_CACHE.with(|cache| cache.borrow_mut().clear());
        drop(entry);
        assert!(weak.upgrade().is_some());
        assert!((retained.solids[0].volume() - 600.).abs() < 0.01);
        // Mutation/export boundaries receive independent topology.
        let solids: Vec<_> = retained
            .solids
            .iter()
            .map(|solid| solid.as_ref().clone())
            .collect();
        let mut step = Vec::new();
        Solid::write_step(&solids, &mut step).unwrap();
        let imported = Solid::read_step(&mut std::io::Cursor::new(step)).unwrap();
        assert!((imported[0].volume() - 600.).abs() < 0.01);
        drop(retained);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn changing_one_region_does_not_retessellate_the_unchanged_region() {
        REGION_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut body = CaseBody {
            id: "case".into(),
            name: "Case".into(),
            kind: CaseKind::Plate,
            thickness: 2.,
            z: None,
            wall_height: None,
            openings: None,
            gasket: None,
        };
        let left = PreparedRegion {
            outer: square(0., 20.),
            holes: vec![],
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![],
        };
        let right = PreparedRegion {
            outer: square(30., 50.),
            holes: vec![],
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![],
        };
        let tessellations = std::cell::Cell::new(0);
        let notify = || {
            tessellations.set(tessellations.get() + 1);
            Ok(())
        };
        cached_region(&body, &left, notify).unwrap();
        cached_region(&body, &right, notify).unwrap();
        assert_eq!(tessellations.get(), 2);
        body.openings = Some(vec![CaseOpening {
            points: square(35., 40.),
            z: 0.,
            height: 2.,
        }]);
        cached_region(&body, &left, notify).unwrap();
        assert_eq!(
            tessellations.get(),
            2,
            "An opening in the other half must not invalidate this half"
        );
        let modified = cached_region(&body, &right, notify).unwrap();
        assert_eq!(tessellations.get(), 3);
        assert!(
            (modified
                .solids
                .iter()
                .map(|solid| solid.volume())
                .sum::<f64>()
                - 750.)
                .abs()
                < 0.01
        );
    }

    #[test]
    fn builds_and_roundtrips_a_holed_plate() {
        let ir = PreparedCase {
            revision: 7,
            body: CaseBody {
                id: "case".into(),
                name: "plate".into(),
                kind: CaseKind::Plate,
                thickness: 2.0,
                z: None,
                wall_height: None,
                openings: None,
                gasket: None,
            },
            regions: vec![PreparedRegion {
                outer: square(0.0, 20.0),
                holes: vec![square(5.0, 15.0)],
                cavities: vec![],
                gaskets: vec![],
                mounts: vec![],
            }],
        };
        let result = build_case_data(ir).expect("valid holed plate");
        assert_eq!(result.revision, 7);
        assert!(!result.step.is_empty());
        assert!(!result.mesh.positions.is_empty());
        assert_eq!(result.mesh.positions.len(), result.mesh.normals.len());
        let mut reader = std::io::Cursor::new(result.step);
        let imported = Solid::read_step(&mut reader).expect("STEP reimport");
        assert_eq!(imported.len(), 1);
        assert!((imported[0].volume() - 600.0).abs() < 0.1);
    }

    #[test]
    fn reads_transformed_multi_solid_component_step_in_millimeters() {
        let bytes = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step"));
        let model = read_step_model_data(bytes.to_vec()).expect("valid component STEP");

        assert_eq!(model.solid_count, 63);
        assert!((model.min[0] - -6.25).abs() < 0.01);
        assert!((model.min[1] - -21.7).abs() < 0.01);
        assert!((model.min[2] - -5.1).abs() < 0.01);
        assert!((model.max[0] - 37.8205).abs() < 0.01);
        assert!((model.max[1] - 12.5).abs() < 0.01);
        assert!((model.max[2] - 1.2).abs() < 0.01);
        assert!(model.mesh.positions.len() > 10_000);
        assert_eq!(model.mesh.positions.len(), model.mesh.normals.len());
        for normal in model.mesh.normals.chunks_exact(3) {
            let length = normal
                .iter()
                .map(|value| f64::from(*value).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!((length - 1.0).abs() < 0.001);
        }
    }
    #[test]
    fn regions_without_applicable_openings_do_not_retain_upstream_copies() {
        STAGE_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut body = CaseBody {
            id: "no-openings".into(),
            name: "No openings".into(),
            kind: CaseKind::Plate,
            thickness: 2.,
            z: None,
            wall_height: None,
            openings: None,
            gasket: None,
        };
        let region = PreparedRegion {
            outer: square(0., 20.),
            holes: vec![],
            cavities: vec![],
            gaskets: vec![],
            mounts: [5., 10., 15., 18.]
                .iter()
                .map(|x| Mount {
                    at: Vec2 { x: *x, y: 10. },
                    kind: MountKind::Hole,
                    hole_diameter: 1.,
                    boss_diameter: None,
                    height: None,
                })
                .collect(),
        };
        for openings in [
            None,
            Some(vec![]),
            Some(vec![CaseOpening {
                points: square(50., 55.),
                z: 0.,
                height: 2.,
            }]),
            Some(vec![CaseOpening {
                points: square(2., 4.),
                z: 4.,
                height: 2.,
            }]),
        ] {
            body.openings = openings;
            let expected = build_region_upstream(&body, &region, BuildMode::default()).unwrap();
            let actual = upstream_region(&body, &region, || {
                build_region_upstream(&body, &region, BuildMode::default())
            })
            .unwrap();
            assert!((expected[0].volume() - actual[0].volume()).abs() < 0.01);
            assert!(
                STAGE_CACHE.with(|cache| cache.borrow().is_empty()),
                "no downstream opening can consume this copy"
            );
        }
    }

    #[test]
    fn upstream_stage_reuses_exact_solids_without_opening_mutation_and_evicts() {
        STAGE_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut body = CaseBody {
            id: "stage".into(),
            name: "Stage".into(),
            kind: CaseKind::Plate,
            thickness: 2.,
            z: None,
            wall_height: None,
            openings: Some(vec![CaseOpening {
                points: square(1., 2.),
                z: 0.,
                height: 2.,
            }]),
            gasket: None,
        };
        let region = PreparedRegion {
            outer: square(0., 20.),
            holes: vec![],
            cavities: vec![],
            gaskets: vec![],
            mounts: [5., 10., 15., 18.]
                .iter()
                .map(|x| Mount {
                    at: Vec2 { x: *x, y: 10. },
                    kind: MountKind::Hole,
                    hole_diameter: 1.,
                    boss_diameter: None,
                    height: None,
                })
                .collect(),
        };
        let builds = std::cell::Cell::new(0);
        let get = |body: &CaseBody| {
            upstream_region(body, &region, || {
                builds.set(builds.get() + 1);
                build_region_upstream(body, &region, BuildMode::default())
            })
            .unwrap()
        };
        let first = get(&body);
        let original_volume: f64 = first.iter().map(Solid::volume).sum();
        for index in 0..40 {
            body.openings = Some(vec![CaseOpening {
                points: square(1. + index as f64 * 0.01, 2. + index as f64 * 0.01),
                z: 0.,
                height: 2.,
            }]);
            let mut edited = get(&body);
            apply_openings(&body, &region, &mut edited).unwrap();
            assert!(
                (edited.iter().map(Solid::volume).sum::<f64>() - (original_volume - 2.)).abs()
                    < 0.01
            );
            assert!(
                (get(&body).iter().map(Solid::volume).sum::<f64>() - original_volume).abs() < 0.01
            );
        }
        assert_eq!(
            builds.get(),
            1,
            "openings do not invalidate upstream construction"
        );
        let coarse = Solid::mesh(
            &get(&body),
            Tessellation {
                deflection_linear: 0.4,
                deflection_angular: 1.,
                relative_linear: false,
            },
        )
        .unwrap();
        let fine = Solid::mesh(
            &get(&body),
            Tessellation {
                deflection_linear: 0.05,
                deflection_angular: 0.2,
                relative_linear: false,
            },
        )
        .unwrap();
        assert!(fine.indices.len() >= coarse.indices.len());
        assert_eq!(
            builds.get(),
            1,
            "tessellation detail does not invalidate exact solids"
        );
        body.thickness = 3.;
        get(&body);
        assert_eq!(builds.get(), 2, "geometry changes invalidate the stage");
        for elevation in 0..80 {
            body.z = Some(elevation as f64);
            get(&body);
            STAGE_CACHE.with(|cache| {
                let cache = cache.borrow();
                assert!(cache.len() <= STAGE_ENTRIES);
                assert!(
                    cache.iter().map(|entry| entry.solids.len()).sum::<usize>() <= STAGE_SOLIDS
                );
                assert!(cache.iter().map(|entry| entry.topology).sum::<usize>() <= STAGE_TOPOLOGY);
            });
        }
        let before = builds.get();
        STAGE_CACHE.with(|cache| cache.borrow_mut().clear());
        get(&body);
        assert_eq!(
            builds.get(),
            before + 1,
            "worker restart discards stage ownership"
        );
    }
    #[test]
    fn region_cache_limits_retained_topology_during_long_edits() {
        REGION_CACHE.with(|cache| cache.borrow_mut().clear());
        let mut body = CaseBody {
            id: "budget".into(),
            name: "Budget".into(),
            kind: CaseKind::Plate,
            thickness: 2.,
            z: None,
            wall_height: None,
            openings: None,
            gasket: None,
        };
        let region = PreparedRegion {
            outer: square(0., 100.),
            holes: (0..20)
                .map(|index| {
                    let x = 2. + (index % 5) as f64 * 15.;
                    let y = 2. + (index / 5) as f64 * 15.;
                    vec![
                        Vec2 { x, y },
                        Vec2 { x: x + 5., y },
                        Vec2 {
                            x: x + 5.,
                            y: y + 5.,
                        },
                        Vec2 { x, y: y + 5. },
                    ]
                })
                .collect(),
            cavities: vec![],
            gaskets: vec![],
            mounts: vec![],
        };
        for index in 0..110 {
            body.z = Some(index as f64);
            cached_region(&body, &region, || Ok(())).unwrap();
            REGION_CACHE.with(|cache| {
                let cache = cache.borrow();
                assert!(cache.len() <= 96);
                assert!(retained_complexity(&cache).0 <= CACHE_SOLIDS);
                assert!(retained_complexity(&cache).1 <= CACHE_TOPOLOGY);
                assert!(
                    cache
                        .iter()
                        .map(|entry| entry.mesh.positions.len() * 8)
                        .sum::<usize>()
                        <= 32 * 1024 * 1024
                );
            });
        }
        REGION_CACHE.with(|cache| {
            assert!(
                cache.borrow().len() < 96,
                "topology cap, not just entry count, evicts large regions"
            )
        });
    }
}
