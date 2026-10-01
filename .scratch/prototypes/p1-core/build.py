from pathlib import Path
import shutil, sys
# Initialization-only generated entry; every transport/engine policy is Rust.
p = Path(__file__).resolve().parent
out = p / "worker-pkg"
(out / "worker-entry.js").write_text("import init, { start_worker } from './boardstudio_p1_core.js';\nawait init();\nstart_worker();\n")
if len(sys.argv) == 3:
    # Caller passes the CLI-reported public directory and the tested prefix.
    public, prefix = Path(sys.argv[1]).resolve(), sys.argv[2]
    if prefix not in ("/", "/boardstudio/"): raise ValueError("out-of-scope prefix")
    if not (public / "index.html").is_file(): raise ValueError("missing CLI host artifact")
    site = p / "evidence/site" / prefix.lstrip("/")
    shutil.copytree(public, site, dirs_exist_ok=True)
    shutil.copytree(out, site / "worker", dirs_exist_ok=True)
