import importlib.util, json, sys, hashlib
from pathlib import Path
root = Path.cwd()
ev = root / ".scratch/dioxus-frontend-v1/evidence/pcb-routed-folder-20261005"
attempt = sys.argv[1]
spec = importlib.util.spec_from_file_location("runner", root / "scripts/run-wasm-tests.py")
r = importlib.util.module_from_spec(spec); spec.loader.exec_module(r)
raw_run = r.subprocess.run

def capture_list(*args, **kwargs):
    result = raw_run(*args, **kwargs)
    if args and "--list" in args[0]:
        (ev / f"case-preview-{attempt}-list.log").write_text(result.stdout + "\n" + result.stderr)
    return result
r.subprocess.run = capture_list
raw_filter = r.run_filter

def capture_filter(filter_, env, root):
    result = raw_filter(filter_, env, root)
    with (ev / f"case-preview-{attempt}-full.log").open("a") as f:
        f.write(filter_ + "\n" + result[2] + "\n")
    return result
r.run_filter = capture_filter
paths = ["web/src/case_preview.rs", "web/src/runtime.rs", "web/src/presentation/shared_viewer.rs", "web/src/presentation/case_viewer.rs"]
hashes = {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths}
filters = [
    "presentation::case_viewer::gesture_cancellation_tests::",
    "presentation::shared_viewer::tests::imported_case_projection_keeps_routed_contours_and_pose_in_pcb_and_case_scene_packets",
    "runtime::case_preview_source_tests::",
]
counts = [3, 1, 1]
if len(sys.argv) > 2 and sys.argv[2] == "projection":
    filters = [filters[1]]; counts = [1]
with r.desktop_webdriver_config(r.runner_environment(root), root) as env:
    listed = r.list_wasm_tests(env, root)
    expected = {f: [n for n in listed if n.startswith(f)] for f in filters}
    assert [len(expected[f]) for f in filters] == counts, expected
    summary = r.run(filters, {}, root, expected, env, [], listed,
        {"mode": "bounded-required-case-owner-and-projection", "source_sha256": hashes},
        ev / f"case-preview-{attempt}.json")
print(json.dumps({"count": summary[0], "failed": summary[1], "problems": summary[3]}, indent=2))
for log in summary[4]: print(log)
assert hashes == {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in paths}, "source changed during focused validation"
sys.exit(bool(summary[1] or summary[3]))
