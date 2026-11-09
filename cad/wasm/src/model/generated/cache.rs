use super::*;
use boardstudio_cad::ExactModel;
use std::{cell::RefCell, collections::VecDeque, rc::Rc};

pub(super) struct CachedBody {
    key: String,
    pub(super) model: Rc<ExactModel>,
    pub(super) mesh: Option<Rc<MeshData>>,
    complexity: (usize, usize),
}

const CACHE_ENTRIES: usize = 64;
const CACHE_BYTES: usize = 64 * 1024 * 1024;
const CACHE_SOLIDS: usize = 256;
const CACHE_TOPOLOGY: usize = 8192;
thread_local! {
    static CACHE: RefCell<VecDeque<Rc<CachedBody>>> = const { RefCell::new(VecDeque::new()) };
}

fn geometry_key(ir: &PreparedCaseIR, scope: &str) -> String {
    let mut geometry = ir.clone();
    geometry.revision = 0;
    geometry.body.id.clear();
    geometry.body.name.clear();
    format!(
        "layered-v1/mesh-mm0.1-rad0.5/{}:{scope}/{geometry:?}",
        scope.len()
    )
}

fn retained(cache: &VecDeque<Rc<CachedBody>>) -> (usize, usize, usize) {
    cache.iter().fold(
        (0usize, 0usize, 0usize),
        |(solids, topology, bytes), entry| {
            let mesh_bytes = entry.mesh.as_ref().map_or(0, |mesh| {
                mesh.positions
                    .capacity()
                    .saturating_add(mesh.normals.capacity())
                    .saturating_mul(4)
            });
            (
                solids.saturating_add(entry.complexity.0),
                topology.saturating_add(entry.complexity.1),
                bytes
                    .saturating_add(entry.key.capacity())
                    .saturating_add(mesh_bytes),
            )
        },
    )
}

fn remember(entry: Rc<CachedBody>) {
    CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        if let Some(index) = cache.iter().position(|old| old.key == entry.key) {
            cache.remove(index);
        }
        cache.push_back(entry);
        while cache.len() > CACHE_ENTRIES || {
            let (solids, topology, bytes) = retained(&cache);
            solids > CACHE_SOLIDS || topology > CACHE_TOPOLOGY || bytes > CACHE_BYTES
        } {
            cache.pop_front();
            metrics::count("bodyCacheEvictions", 1);
        }
    });
}

pub(super) fn body(
    ir: &PreparedCaseIR,
    scope: &str,
    want_mesh: bool,
    progress: &mut impl FnMut(&'static str) -> Result<(), String>,
) -> Result<Rc<CachedBody>, String> {
    if ir.revision > MAX_REVISION {
        return Err("Invalid CAD revision".into());
    }
    let key = geometry_key(ir, scope);
    let cached = CACHE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.key == key)?;
        let entry = cache.remove(index)?;
        cache.push_back(Rc::clone(&entry));
        Some(entry)
    });
    if let Some(entry) = cached
        .as_ref()
        .filter(|entry| !want_mesh || entry.mesh.is_some())
    {
        metrics::count("bodyCacheHits", 1);
        return Ok(Rc::clone(entry));
    }
    let (model, complexity) = if let Some(entry) = cached {
        (Rc::clone(&entry.model), entry.complexity)
    } else {
        metrics::count("bodyCacheMisses", 1);
        progress("building")?;
        let _stage = Stage::new("caseConstruction");
        let model = Rc::new(boardstudio_cad::build_case(ir)?);
        let complexity = model.cache_complexity();
        (model, complexity)
    };
    let mesh = if want_mesh {
        progress("tessellating")?;
        let _stage = Stage::new("tessellation");
        metrics::count("tessellations", 1);
        let mesh = boardstudio_cad::mesh_profile(&model)?;
        Some(Rc::new(MeshData {
            positions: mesh.positions,
            normals: mesh.normals,
        }))
    } else {
        None
    };
    let entry = Rc::new(CachedBody {
        key,
        model,
        mesh,
        complexity,
    });
    remember(Rc::clone(&entry));
    Ok(entry)
}
