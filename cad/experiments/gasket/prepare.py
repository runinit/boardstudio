"""Stage production sources and a verified Cadrum patch outside production builds."""
import hashlib
import json
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import urllib.request

SOURCE = Path(__file__).resolve().parent
ROOT = SOURCE.parents[2]
DEST = Path(sys.argv[1]).resolve()
CHECKSUM = "c666eb59f8fdf88e484dfa929e59d7d9f2ba4b7ff1dddd73a66d43c2720f6962"
DEST.mkdir(parents=True, exist_ok=False)
archive = DEST / "cadrum-0.8.20.crate"
with urllib.request.urlopen("https://static.crates.io/crates/cadrum/cadrum-0.8.20.crate") as response:
    archive.write_bytes(response.read())
assert hashlib.sha256(archive.read_bytes()).hexdigest() == CHECKSUM
with tarfile.open(archive) as tar:
    tar.extractall(DEST, filter="data")
vendor = DEST / "cadrum-0.8.20"
crate = DEST / "cad/wasm"
crate.mkdir(parents=True)
shutil.copytree(ROOT / "cad/wasm/src", crate / "src")
shutil.copytree(ROOT / "cad/bench/fixtures", DEST / "cad/bench/fixtures")
step = Path("ergogen/library/vendor/infused-kim/3d_models/trackpoint/TP_Red_T460S_platform_z_offset_+0.0_pcb_offset_-2.0.step")
(DEST / step).parent.mkdir(parents=True)
shutil.copy2(ROOT / step, DEST / step)
shutil.copy2(SOURCE / "cadrum.Cargo.lock", crate / "Cargo.lock")
manifest = (ROOT / "cad/wasm/Cargo.toml").read_text()
manifest = manifest.replace('[dev-dependencies]', 'i_overlay = "=9.0.0"\nserde_json = "=1.0.151"\n\n[dev-dependencies]')
manifest += '\n[patch.crates-io]\ncadrum = { path = "../../cadrum-0.8.20" }\n'
(crate / "Cargo.toml").write_text(manifest)
inputs = DEST / "inputs"
inputs.mkdir()
shutil.copy2(ROOT / "cad/bench/fixtures/live-gasket-bottom-regression.json", inputs / "regression.json")
# Match the application's JSON number serialization and retained input hashes.
subprocess.run(["node", "--input-type=module", "-e", """
import fs from 'node:fs';
const captured=JSON.parse(fs.readFileSync(process.argv[1]));
for(const revision of [0,2,5]) {
    const body=captured.gaskets[revision].bodies.find(b=>b.body.id==='bottom');
    if(!body)throw Error('Missing bottom');
    fs.writeFileSync(process.argv[2]+`/edit-${revision}.json`,JSON.stringify(body));
}
""", str(ROOT / "app/performance-results/phase2-attribution-before/prepared.json"), str(inputs)], check=True)

def replace(path, old, new):
    data = path.read_text()
    assert data.count(old) == 1, (str(path), old, data.count(old))
    path.write_text(data.replace(old, new))

shutil.copy2(SOURCE / "comparison.rs", crate / "src/model/construction/bounded_experiment.rs")
with (crate / "src/model/construction.rs").open("a") as out:
    out.write('\nmod bounded_experiment;\n')
shutil.copy2(SOURCE / "instrumentation.rs", vendor / "src/bounded_experiment.rs")
with (vendor / "src/lib.rs").open("a") as out:
    out.write('\npub mod bounded_experiment;\n')
with (vendor / "Cargo.toml").open("a") as out:
    out.write('\n[target.\'cfg(target_arch = "wasm32")\'.dependencies.wasm-bindgen]\nversion = "=0.2.129"\n')

ffi = vendor / "src/ffi.rs"
replace(ffi, 'extern "Rust" {', '''extern "Rust" {
        fn experiment_start() -> f64;
        fn experiment_finish(index: usize, start: f64);
        fn experiment_option(index: usize) -> bool;''')
with ffi.open("a") as out:
    out.write('''
fn experiment_start() -> f64 { crate::bounded_experiment::start() }
fn experiment_finish(index: usize, start: f64) { crate::bounded_experiment::finish(index, start); }
fn experiment_option(index: usize) -> bool { crate::bounded_experiment::option(index) }
''')
cpp = vendor / "src/ffi.cpp"
replace(cpp, '#include <BOPAlgo_CellsBuilder.hxx>', '#include <BOPAlgo_CellsBuilder.hxx>\n#include <BOPAlgo_PaveFiller.hxx>')
replace(cpp, '    cb.Perform();', '''    // Split-filler mode is attribution-only; normal mode retains cb.Perform().
    auto perform_start = experiment_start();
    std::unique_ptr<BOPAlgo_PaveFiller> filler;
    if (experiment_option(3)) {
        filler = std::make_unique<BOPAlgo_PaveFiller>();
        filler->SetArguments(args);
        filler->Perform();
        experiment_finish(0, perform_start);
        if (filler->HasErrors()) throw std::runtime_error("experiment filler failed");
        perform_start = experiment_start();
        cb.PerformWithFiller(*filler);
        experiment_finish(1, perform_start);
    } else {
        cb.Perform();
        experiment_finish(2, perform_start);
    }''')
replace(cpp, '    const int material = 1;', '    auto selection_start = experiment_start();\n    const int material = 1;')
replace(cpp, '    cb.RemoveInternalBoundaries();', '''    experiment_finish(3, selection_start);
    auto cleanup_start = experiment_start();
    cb.RemoveInternalBoundaries();
    experiment_finish(4, cleanup_start);''')
replace(cpp, '    std::unordered_map<uint64_t, uint64_t> relay1, relay2;', '    auto history_start = experiment_start();\n    std::unordered_map<uint64_t, uint64_t> relay1, relay2;')
replace(cpp, '    BRepBuilderAPI_Copy copier(cb.Shape(), true, false);', '''    experiment_finish(5, history_start);
    auto copy_start = experiment_start();
    BRepBuilderAPI_Copy copier(cb.Shape(), true, false);''')
replace(cpp, '    relay_from_pair(cb.Shape(), copier.Shape(), relay2);', '''    experiment_finish(6, copy_start);
    history_start = experiment_start();
    relay_from_pair(cb.Shape(), copier.Shape(), relay2);''')
replace(cpp, '    relay_into_history(&relay1, &relay2, out_history);', '    relay_into_history(&relay1, &relay2, out_history);\n    experiment_finish(5, history_start);')
replace(cpp, '    BRepMesh_IncrementalMesh mesher(shape, linear, relative, angular, false);', '''    auto mesh_start = experiment_start();
    BRepMesh_IncrementalMesh mesher(shape, linear, relative, angular, false);
    experiment_finish(7, mesh_start);
    auto extraction_start = experiment_start();''')
replace(cpp, '            result.face_tshape_ids.push_back(face_id);', '            if (!experiment_option(1)) result.face_tshape_ids.push_back(face_id);')
replace(cpp, '    result.success = true;', '    experiment_finish(8, extraction_start);\n    result.success = true;')
io = vendor / "src/occt/io.rs"
replace(io, '\tlet mut edges: Vec<DVec3> = Vec::new();', '\tlet edge_start = crate::bounded_experiment::start();\n\tlet mut edges: Vec<DVec3> = Vec::new();\n\tif !crate::bounded_experiment::option(0) {')
replace(io, '\n\t#[cfg(feature = "color")]\n\tlet colormap = {', '\n\t}\n\tcrate::bounded_experiment::finish(9, edge_start);\n\t#[cfg(feature = "color")]\n\tlet colormap = {')

hashes = {}
for path in sorted((ROOT / "cad/wasm/src").rglob("*.rs")):
    hashes[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
for path in sorted(SOURCE.glob("*")):
    if path.is_file():
        hashes[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()
(DEST / "provenance.json").write_text(json.dumps({"cadrumCrateSha256": CHECKSUM, "sourceHashes": hashes}, indent=2) + '\n')
print(DEST)
