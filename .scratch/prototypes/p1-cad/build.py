from pathlib import Path
import shutil, sys
p=Path(__file__).resolve().parent
out=p/'worker-pkg'
# Generated initialization only; all executor/CAD/cache policy is authored Rust.
(out/'worker-entry.js').write_text("import init, {start_worker} from './boardstudio_p1_cad.js';\nawait init();\nawait start_worker();\n")
if len(sys.argv)==3:
    public,prefix=Path(sys.argv[1]).resolve(),sys.argv[2]
    if prefix not in ('/','/boardstudio/'):raise ValueError('out-of-scope prefix')
    if not (public/'index.html').is_file():raise ValueError('missing dx host artifact')
    site=p/'evidence/site'/prefix.lstrip('/')
    shutil.copytree(public,site,dirs_exist_ok=True)
    shutil.copytree(out,site/'worker',dirs_exist_ok=True)
    shutil.copytree(p.parents[2]/'cad/wasm/pkg',site/'worker/cad',dirs_exist_ok=True)
