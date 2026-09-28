"""Add isolated E2/E3/E8 options to an already prepared fresh experiment stage."""
from pathlib import Path
import sys

stage = Path(sys.argv[1]).resolve()
vendor = stage / 'cadrum-0.8.20/src'
comparison = stage / 'cad/wasm/src/model/construction/bounded_experiment.rs'

def replace(path, old, new):
    text = path.read_text()
    assert text.count(old) == 1, (path, old)
    path.write_text(text.replace(old, new))

instrumentation = vendor / 'bounded_experiment.rs'
replace(instrumentation, 'options: [bool; 4]', 'options: [bool; 9]')
replace(instrumentation, 'options: [skip_edges, skip_ids, timed, split]',
        'options: [skip_edges, skip_ids, timed, split, false, false, false, false, false]')
with instrumentation.open('a') as output:
    output.write('''
// E2/E3/E8 switches exist only in this staged crate.
pub fn configure_occt(relay: bool, history: bool, obb: bool, cut: bool, non_destructive: bool) {
    STATE.with(|state| state.borrow_mut().options[4..].copy_from_slice(&[relay, history, obb, cut, non_destructive]));
}
''')
replace(comparison, '"control" => (false, false, false),',
        '"control" | "no-relay" | "no-history" | "obb" | "multi-cut" | "non-destructive" => (false, false, false),')
replace(comparison, 'diagnostic::configure(edges, ids, timed, split);', '''diagnostic::configure(edges, ids, timed, split);
    diagnostic::configure_occt(variant == "no-relay", variant == "no-history",
        variant == "obb", variant == "multi-cut", variant == "non-destructive");''')
replace(comparison, 'for variant in ["control", "no-edges", "no-ids", "lean", "profiles", "split-timing"] {',
        'for variant in ["control", "no-relay", "no-history", "obb", "multi-cut", "non-destructive"] {')
cpp = vendor / 'ffi.cpp'
replace(cpp, '#include <BOPAlgo_PaveFiller.hxx>', '#include <BOPAlgo_PaveFiller.hxx>\n#include <BRepAlgoAPI_Cut.hxx>')
replace(cpp, '    BOPAlgo_CellsBuilder cb;', '''    // Only a single DNF clause with one positive operand is a pure batched cut.
    // Mixed/general expressions retain CellsBuilder. All result copies remain.
    if (experiment_option(7)) {
        int positives = 0, negatives = 0, ends = 0;
        NCollection_List<TopoDS_Shape> arguments, tools;
        for (auto literal : clauses) {
            if (literal == 0) { ++ends; continue; }
            auto index = static_cast<size_t>((literal > 0 ? literal : -literal) - 1);
            if (index >= solids.size()) throw std::runtime_error("invalid experiment operand");
            if (literal > 0) { ++positives; arguments.Append(solids[index]); }
            else { ++negatives; tools.Append(solids[index]); }
        }
        if (positives == 1 && negatives > 0 && ends == 1 && clauses[clauses.size()-1] == 0) {
            BRepAlgoAPI_Cut cut;
            cut.SetArguments(arguments);
            cut.SetTools(tools);
            auto start = experiment_start();
            cut.Build();
            experiment_finish(2, start);
            if (cut.HasErrors() || !cut.IsDone()) throw std::runtime_error("experiment multi-tool cut failed");
            start = experiment_start();
            std::unordered_map<uint64_t, uint64_t> relay1, relay2;
            for (const auto& source : solids) relay_from_builder(cut, source, relay1);
            experiment_finish(5, start);
            start = experiment_start();
            BRepBuilderAPI_Copy copier(cut.Shape(), true, false);
            auto shape = std::make_unique<TopoDS_Shape>(copier.Shape());
            experiment_finish(6, start);
            start = experiment_start();
            relay_from_pair(cut.Shape(), copier.Shape(), relay2);
            relay_into_history(&relay1, &relay2, out_history);
            experiment_finish(5, start);
            return shape;
        }
    }
    BOPAlgo_CellsBuilder cb;
    if (experiment_option(5)) cb.SetToFillHistory(false);
    if (experiment_option(6)) cb.SetUseOBB(true);
    if (experiment_option(8)) cb.SetNonDestructive(true);''')
replace(cpp, '    for (const auto& s : solids) {\n        relay_from_builder(cb, s, relay1);\n    }', '''    if (!experiment_option(4) && !experiment_option(5)) {
        for (const auto& s : solids) relay_from_builder(cb, s, relay1);
    }''')
replace(cpp, '''    relay_from_pair(cb.Shape(), copier.Shape(), relay2);
    relay_into_history(&relay1, &relay2, out_history);''', '''    if (!experiment_option(4) && !experiment_option(5)) {
        relay_from_pair(cb.Shape(), copier.Shape(), relay2);
        relay_into_history(&relay1, &relay2, out_history);
    }''')
print(stage)
replace(comparison, 'if variant == "profiles" { profiles(ir) } else { current(ir) }', '''match variant {
        "profiles" => profiles(ir),
        "combined-cuts" => planned_cuts(ir, false),
        "profile-holes" => planned_cuts(ir, true),
        "final-clean" => current(ir)?.iter().map(|solid| solid.clean().map_err(cadrum_error)).collect(),
        _ => current(ir),
    }''')
replace(comparison, '"control" | "no-relay"', '"combined-cuts" | "profile-holes" | "final-clean" | "control" | "no-relay"')
replace(comparison, 'for variant in ["control", "no-relay", "no-history", "obb", "multi-cut", "non-destructive"] {',
        'for variant in ["control", "no-relay", "no-history", "obb", "multi-cut", "non-destructive", "combined-cuts", "profile-holes", "final-clean"] {')
with comparison.open('a') as output:
    output.write((Path(__file__).resolve().parent/'construction-options.rs').read_text())
# Extra stress coverage is derived without changing any original fixture.
import subprocess
subprocess.run(['node', '--input-type=module', '-e', '''
import fs from 'node:fs';
const path=process.argv[1];
const fixture=JSON.parse(fs.readFileSync(path+'/regression.json'));
const angle=0.31,c=Math.cos(angle),s=Math.sin(angle);
const rotate=p=>{const{x,y}=p;p.x=x*c-y*s;p.y=x*s+y*c;};
for(const r of fixture.regions){
    if(r.cavities.length||r.gaskets.length)throw Error('Unsupported rotation');
    r.outer.forEach(rotate);r.holes.flat().forEach(rotate);r.mounts.forEach(m=>rotate(m.at));
}
fixture.body.openings.forEach(o=>o.points.forEach(rotate));
fs.writeFileSync(path+'/rotated-stress.json',JSON.stringify(fixture));
''', str(stage/'inputs')], check=True)
