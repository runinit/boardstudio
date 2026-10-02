#!/usr/bin/env python3
"""Run unchanged repository performance gates serially with portable provenance."""
from pathlib import Path
from datetime import datetime, timezone
import argparse, hashlib, json, os, subprocess, sys
root=Path(__file__).resolve().parents[4]
parser=argparse.ArgumentParser(description=__doc__)
parser.add_argument('--build-record', required=True, help='exact successful release build record for the current candidate')
args=parser.parse_args()
build_record_path=Path(args.build_record)
if not build_record_path.is_absolute(): build_record_path=root/build_record_path
build_record_path=build_record_path.resolve()
build=json.loads(build_record_path.read_text())
if build.get('exit') != 0 or build.get('asset_hashes_verified') is not True: raise SystemExit(f'release build record is not a verified success: {build_record_path}')
run=root/'.scratch/m1-production/evidence/performance/runs'/('reference-gates-'+datetime.now(timezone.utc).strftime('%Y%m%dT%H%M%SZ'))
run.mkdir(parents=True,exist_ok=False)
record={'cwd':str(root),'started_at':datetime.now(timezone.utc).isoformat(),'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=root,text=True).strip(),'production_build':{'release_id':build_record_path.stem.removeprefix('release-'),'source_commit':build.get('source_commit'),'build_record_path':str(build_record_path),'build_record_sha256':hashlib.sha256(build_record_path.read_bytes()).hexdigest(),'provenance_path':build.get('portable_provenance'),'provenance_sha256':build.get('provenance_sha256')},'commands':[],'scope':'Unchanged reference regression gates; never substitute for an eligible M1 public-page comparison.'}
paths=['app/scripts/run-performance.mjs','app/performance-baseline.json','app/scripts/run-cad-benchmark.mjs','cad/bench/compare.mjs','cad/bench/budgets.json','cad/bench/results/baseline/summary.json','core/pkg/boardstudio_core_bg.wasm','renderer/pkg/boardstudio_renderer_wasm_bg.wasm','cad/wasm/pkg/boardstudio_cadrum_wasm_bg.wasm']
record['input_sha256']={p:hashlib.sha256((root/p).read_bytes()).hexdigest() for p in paths}
def save(): (run/'record.json').write_text(json.dumps(record,indent=2)+'\n')
def execute(label,argv,env=None):
 row={'label':label,'argv':argv,'started_at':datetime.now(timezone.utc).isoformat(),'environment':env or {}};record['commands'].append(row);save()
 with (run/(label+'.log')).open('w') as log:
  result=subprocess.run(argv,cwd=root,env={**os.environ,**(env or {})},stdout=log,stderr=subprocess.STDOUT)
 row.update(exit=result.returncode,finished_at=datetime.now(timezone.utc).isoformat(),log=label+'.log');save();print(label,result.returncode,flush=True)
execute('reference-ui-live',['pnpm','--dir','app','test:perf'],{'BOARDSTUDIO_PERF_REPORT':str(run/'ui-live.json'),'BOARDSTUDIO_TEST_PORT':'46883'})
execute('reference-cad',['pnpm','run','bench:cad','--','--output',str(run/'cad'),'--port','46884'])
execute('frozen-cad-compare',['node','cad/bench/compare.mjs',str(run/'cad')])
record['finished_at']=datetime.now(timezone.utc).isoformat();record['unchanged_inputs']={p:hashlib.sha256((root/p).read_bytes()).hexdigest()==sha for p,sha in record['input_sha256'].items()};save();print(run,flush=True)
sys.exit(0 if all(row['exit']==0 for row in record['commands']) else 1)
