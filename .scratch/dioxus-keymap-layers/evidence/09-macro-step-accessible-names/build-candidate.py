#!/usr/bin/env python3
"""Rebuild only presentation/offline packages against verified unchanged M1 providers."""
from pathlib import Path
import datetime,hashlib,json,os,shutil,subprocess,sys
ROOT=Path('/home/chris/.local/share/boardstudio/worktrees/keymap-macro-labels-20261002')
BASE=Path('/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/web/target/builds/frontend-keycaps-integrated-20261002')
WEB=ROOT/'web'
name=sys.argv[1]
if not name.replace('-','').isalnum():raise SystemExit('unique alphanumeric/hyphen build id required')
out=WEB/'target/builds'/name;out.mkdir(parents=True,exist_ok=False)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
prior=json.loads((BASE/'provenance.json').read_text())
# runtime.rs is page-only: the private terminal-outcome observer is absent from reused providers.
# All provider source and asset hashes remain mandatory checks.
allowed={
 'application/tests/durable_session.rs','scripts/build-m1.py','scripts/web/build-layout-generators.mjs','scripts/web/test-physical-setup-proposal.mjs',
 'web/Cargo.lock','web/Cargo.toml','web/assets/m1.css','web/build.rs','web/src/cad_presentation.rs','web/src/host/storage.rs','web/src/lib.rs','web/src/main.rs','web/src/physical_setup.rs','web/src/presentation.rs','web/src/presentation/case_viewer.rs','web/src/presentation/case_workspace.rs','web/src/presentation/context_summary.rs','web/src/presentation/keycaps_scene.rs','web/src/presentation/keycaps_settings.rs','web/src/presentation/keycaps_workspace.rs','web/src/presentation/keymap/macro_editor.rs','web/src/presentation/keymap/panel.rs','web/src/presentation/keymap_workspace.rs','web/src/presentation/layout_workspace.rs','web/src/presentation/library.rs','web/src/presentation/mechanical_settings.rs','web/src/presentation/mechanical_settings_mount.rs','web/src/presentation/model_delivery.rs','web/src/presentation/objects.rs','web/src/presentation/objects/layout_toolbar.rs','web/src/presentation/objects/matrix_inspector.rs','web/src/presentation/objects/tree.rs','web/src/presentation/panels.rs','web/src/presentation/parts.rs','web/src/presentation/parts/catalogue.rs','web/src/presentation/parts/physical_setup.rs','web/src/presentation/parts_workspace.rs','web/src/presentation/pcb_wiring.rs','web/src/presentation/pcb_wiring/controller.rs','web/src/presentation/pcb_workspace.rs','web/src/presentation/shared_viewer.rs','web/src/presentation/workspace_composition.rs','web/src/renderer_host_page_extensions.rs','web/src/runtime.rs'
}
changed=[p for p,h in prior['sources'].items() if sha(ROOT/p)!=h]
assert not set(changed)-allowed,changed
for mode in ['root','subpath']:
 site=Path(prior[mode]['site'])
 for p,h in prior[mode]['assets'].items():assert sha(site/p)==h,(mode,p)
source={p:sha(ROOT/p) for p in prior['sources']}
for p in subprocess.check_output(['git','ls-files','-z','web/'],cwd=ROOT).decode().split('\0'):
 if p:source[p]=sha(ROOT/p)
record={'source_commit':subprocess.check_output(['git','rev-parse','HEAD'],cwd=ROOT,text=True).strip(),'build_id':name,'scope':'Frontend page and scoped offline bundle rebuilt; exact unchanged provider/fixture assets reused from verified maintained M1 release.','base_build':str(BASE),'base_provenance_sha256':sha(BASE/'provenance.json'),'changed_base_inputs':changed,'sources':source,'commands':[]}
env=dict(os.environ,TMPDIR=str(out/'tmp'));(out/'tmp').mkdir()
def save():(out/'provenance.json').write_text(json.dumps(record,indent=2)+'\n')
def run(label,args,cwd=ROOT,extra=None):
 print(label,flush=True);log=out/f'{label}.log';start=datetime.datetime.now(datetime.timezone.utc).isoformat()
 with log.open('w') as f:r=subprocess.run(args,cwd=cwd,env=dict(env,**(extra or {})),stdout=f,stderr=subprocess.STDOUT)
 record['commands'].append({'label':label,'argv':args,'cwd':str(cwd),'started':start,'finished':datetime.datetime.now(datetime.timezone.utc).isoformat(),'exit':r.returncode,'log':str(log),'extra_env':extra or {}});save()
 if r.returncode:print(log.read_text()[-6000:]);raise SystemExit(r.returncode)
run('layout-generators',['node','scripts/web/build-layout-generators.mjs',str(out/'layout-generator-assets')])
for p in ['ergogen/src/index.ts','ergogen/generated/catalogue.mjs','scripts/web/build-layout-generators.mjs']:
 source[p]=sha(ROOT/p)
run('preview-generator',['node','scripts/web/build-preview-generator.mjs',str(out/'preview-generator-assets')])
for p in ['scripts/web/build-preview-generator.mjs','scripts/web/preview-generator-worker.ts','kicad/src/ergogen.ts']:
 source[p]=sha(ROOT/p)
for mode,prefix in [('root','/'),('subpath','/boardstudio/')]:
 public=WEB/'target/dx/boardstudio-web/release/web/public'
 if public.exists():shutil.move(public,out/f'previous-dx-public-{mode}')
 run('page-'+mode,['dx','build','--web','--release','--base-path',prefix,'--no-default-features','--features','page','--cargo-args=--locked'],WEB)
 dest=out/f'site-{mode}'
 if mode=='subpath':dest/='boardstudio'
 shutil.copytree(Path(prior[mode]['site']),dest)
 # Remove stale owned copied page files and replace with fresh generated bundle.
 for p in (dest/'assets').glob('boardstudio-web*'):p.unlink()
 for p in public.rglob('*'):
  if p.is_file():target=dest/p.relative_to(public);target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(p,target)
 shutil.copytree(WEB/'assets',dest/'assets',dirs_exist_ok=True)
 shutil.copytree(out/'layout-generator-assets',dest/'assets',dirs_exist_ok=True)
 shutil.copytree(out/'preview-generator-assets',dest/'assets',dirs_exist_ok=True)
 manifest=out/f'offline-manifest-{mode}.json'
 assets=sorted({str(p.relative_to(dest)) for p in dest.rglob('*') if p.is_file()}|{'service-worker.js','boardstudio_offline_worker.js'})
 manifest.write_text(json.dumps({'version':name+'-'+mode,'assets':assets},indent=2)+'\n')
 run('offline-'+mode,['wasm-pack','build',str(WEB),'--target','web','--out-name','boardstudio_offline_worker','--out-dir',str(out/f'offline-{mode}'),'--release','--locked','--no-default-features','--features','service-worker'],extra={'BOARDSTUDIO_OFFLINE_MANIFEST':str(manifest)})
 run('embed-offline-'+mode,['node',str(ROOT/'scripts/web/embed-worker-wasm.mjs'),str(out/f'offline-{mode}'),str(manifest),str(dest/'service-worker.js')])
 record[mode]={'site':str(dest),'prefix':prefix,'assets':{str(p.relative_to(dest)):sha(p) for p in sorted(dest.rglob('*')) if p.is_file()}};save()
assert all(sha(ROOT/p)==h for p,h in source.items()),'source changed during build'
record['source_hashes_verified']=len(source);record['status']='complete';save();print(str(out),flush=True)
