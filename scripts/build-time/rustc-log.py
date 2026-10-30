#!/usr/bin/env python3
"""RUSTC_WRAPPER that records one JSON line per compile unit; changes nothing about the compile."""
import json, os, subprocess, sys, time
argv = sys.argv[1:]
start = time.time()
code = subprocess.run(argv, close_fds=False).returncode
if "--crate-name" in argv:
    def after(flag):
        return argv[argv.index(flag) + 1] if flag in argv else None
    c = [a[2:] if a.startswith("-C") and len(a) > 2 else argv[i + 1] for i, a in enumerate(argv) if a.startswith("-C") and i + 1 < len(argv)]
    opts = dict(kv.split("=", 1) if "=" in kv else (kv, "") for kv in c)
    rec = dict(crate=after("--crate-name"), start=round(start, 2), secs=round(time.time() - start, 3),
               target=after("--target") or "host", opt=opts.get("opt-level", "0"), debug=opts.get("debuginfo", "0"),
               meta=opts.get("metadata"), out=after("--out-dir"), test="--test" in argv,
               incr=opts.get("incremental"),
               emit=next((a for a in argv if a.startswith("--emit=")), ""), types=[argv[i + 1] for i, a in enumerate(argv) if a == "--crate-type"],
               features=sum(1 for a in argv if a.startswith('feature="')), driver=os.path.basename(argv[0]), code=code, cwd=os.getcwd())
    with open(os.environ["BUILDTIME_LOG"], "a") as log:
        log.write(json.dumps(rec) + "\n")
sys.exit(code)
