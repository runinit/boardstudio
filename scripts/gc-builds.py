#!/usr/bin/env python3
"""Prune web/target/builds safely. Dry run by default; --apply deletes.

Always kept: the newest --keep-latest complete builds, the served candidate,
page-only/fixture-refresh baselines (provenance base_build) of kept builds
(transitively), and builds named by accepted parent decisions or the
qualification active_scope. Dirs without a complete provenance.json are
partial/failed builds: removable, listed separately. Symlinks are never
followed. Before removal, provenance.json is copied into an already-existing
.scratch/dioxus-frontend-v1/evidence/<build-id>/ directory (never created).
"""
import argparse
import json
import os
from pathlib import Path
import shutil
import sys

REPO = Path(__file__).resolve().parent.parent
BUILD_ROOT = REPO / "web" / "target" / "builds"
RUN_JSON = REPO / "docs" / "migration" / "dioxus-frontend-v1-run.json"
EVIDENCE_ROOT = REPO / ".scratch" / "dioxus-frontend-v1" / "evidence"


def dir_size(path):
    total = 0
    for root, dirs, files in os.walk(path, followlinks=False):
        for name in files + dirs:
            try:
                total += os.lstat(os.path.join(root, name)).st_size
            except OSError:
                pass
    return total


def fmt(n):
    for unit in ("B", "KiB", "MiB", "GiB"):
        if n < 1024 or unit == "GiB":
            return f"{n:.1f}{unit}" if unit != "B" else f"{n}B"
        n /= 1024


def read_provenance(build):
    """Return provenance dict if complete, else None."""
    path = build / "provenance.json"
    try:
        if path.is_symlink() or not path.is_file():
            return None
        data = json.loads(path.read_text())
    except (OSError, ValueError):
        return None
    if not isinstance(data, dict) or not data.get("source_commit"):
        return None
    if data.get("status", "complete") != "complete":
        return None
    return data


def timestamp(build, provenance):
    stamp = None
    if provenance:
        rows = [r for r in provenance.get("commands", []) if isinstance(r, dict)]
        stamp = next((r.get("finished") for r in reversed(rows) if r.get("finished")), None)
    if stamp:
        try:
            from datetime import datetime
            return datetime.fromisoformat(stamp).timestamp()
        except ValueError:
            pass
    return os.lstat(build).st_mtime


def strings(value):
    if isinstance(value, str):
        yield value
    elif isinstance(value, dict):
        for item in value.values():
            yield from strings(item)
    elif isinstance(value, list):
        for item in value:
            yield from strings(item)


def mentioned(texts, names):
    found = set()
    for text in texts:
        for name in names:
            if name in text:
                found.add(name)
    return found


def protected_from_run(run_path, repo, names):
    """Return (served_id, {build_id: reason}) from the run record."""
    reasons = {}
    served = None
    try:
        progress = json.loads(Path(run_path).read_text()).get("current_progress", {})
    except (OSError, ValueError):
        return None, reasons
    served = (progress.get("served_candidate") or {}).get("build_id")
    scope = (progress.get("qualification") or {}).get("active_scope")
    for name in mentioned(strings(scope), names):
        reasons[name] = "qualification active_scope"
    texts = []
    for entry in progress.get("parent_acceptances") or []:
        texts.extend(strings(entry))
        for value in entry.values() if isinstance(entry, dict) else []:
            if isinstance(value, str) and value.endswith(".json"):
                try:
                    texts.extend(strings(json.loads((Path(repo) / value).read_text())))
                except (OSError, ValueError):
                    pass
    for name in mentioned(texts, names):
        reasons.setdefault(name, "accepted parent decision")
    return served, reasons


def plan(root, run_path, repo, keep_latest):
    root = Path(root)
    rows = {}
    for child in sorted(root.iterdir()):
        info = {"path": child, "symlink": child.is_symlink(), "dir": child.is_dir() and not child.is_symlink()}
        info["prov"] = read_provenance(child) if info["dir"] else None
        info["ts"] = timestamp(child, info["prov"]) if info["dir"] or info["symlink"] else 0
        rows[child.name] = info
    served, reasons = protected_from_run(run_path, repo, set(rows))
    keep = {}
    if served and served in rows:
        keep[served] = "served candidate"
    elif served:
        served = served
    complete = sorted((n for n, i in rows.items() if i["prov"] is not None and i["dir"]),
                      key=lambda n: rows[n]["ts"], reverse=True)
    for name in complete[:keep_latest]:
        keep.setdefault(name, f"newest {keep_latest}")
    for name, why in reasons.items():
        keep.setdefault(name, why)
    queue = list(keep)
    while queue:
        name = queue.pop()
        prov = rows[name]["prov"] or {}
        base = prov.get("base_build")
        if base:
            base_id = Path(base).name
            if base_id in rows and base_id not in keep:
                keep[base_id] = f"baseline of {name}"
                queue.append(base_id)
    remove, partial = {}, {}
    for name, info in rows.items():
        if name in keep:
            continue
        if info["dir"] and info["prov"] is None:
            partial[name] = "missing/incomplete provenance.json (failed or partial build)"
        else:
            remove[name] = "older than kept set" if info["dir"] else "not a directory"
    return {"rows": rows, "keep": keep, "remove": remove, "partial": partial, "served": served}


def check_targets(result, root):
    root = Path(root).resolve()
    served = result["served"]
    for name in list(result["remove"]) + list(result["partial"]):
        path = result["rows"][name]["path"]
        if path.is_symlink():
            raise SystemExit(f"refusing --apply: {path} is a symlink")
        if path.resolve().parent != root:
            raise SystemExit(f"refusing --apply: {path} is outside {root}")
        if name == served:
            raise SystemExit(f"refusing --apply: {path} is the served directory")


def preserve_provenance(name, path, evidence_root):
    evidence = Path(evidence_root) / name
    source = path / "provenance.json"
    if evidence.is_dir() and not evidence.is_symlink() and source.is_file() and not source.is_symlink():
        target = evidence / "provenance.json"
        if not target.exists():
            shutil.copyfile(source, target)
            return True
    return False


def main(argv=None, root=BUILD_ROOT, run_path=RUN_JSON, repo=REPO, evidence_root=EVIDENCE_ROOT):
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--apply", action="store_true", help="delete (default is a dry run)")
    parser.add_argument("--keep-latest", type=int, default=3)
    args = parser.parse_args(argv)
    if args.keep_latest < 0:
        parser.error("--keep-latest must be >= 0")
    result = plan(root, run_path, repo, args.keep_latest)
    rows = result["rows"]
    sizes = {n: dir_size(i["path"]) if i["dir"] else 0 for n, i in rows.items()}
    mode = "APPLY" if args.apply else "DRY RUN"
    print(f"gc-builds {mode}: {len(rows)} entries under {root}")
    for name in sorted(result["keep"], key=lambda n: -rows[n]["ts"]):
        print(f"KEEP    {fmt(sizes[name]):>9}  {name}  [{result['keep'][name]}]")
    for name in sorted(result["remove"], key=lambda n: -rows[n]["ts"]):
        print(f"REMOVE  {fmt(sizes[name]):>9}  {name}  [{result['remove'][name]}]")
    for name in sorted(result["partial"]):
        print(f"PARTIAL {fmt(sizes[name]):>9}  {name}  [{result['partial'][name]}]")
    targets = list(result["remove"]) + list(result["partial"])
    freed = sum(sizes[n] for n in targets)
    if args.apply:
        check_targets(result, root)
        for name in targets:
            path = rows[name]["path"]
            if preserve_provenance(name, path, evidence_root):
                print(f"preserved provenance.json for {name}")
            if path.is_symlink():
                raise SystemExit(f"refusing to remove symlink {path}")
            shutil.rmtree(path)
    verb = "removed" if args.apply else "would remove"
    print(f"summary: kept {len(result['keep'])} ({fmt(sum(sizes[n] for n in result['keep']))}), "
          f"{verb} {len(targets)} ({len(result['remove'])} old + {len(result['partial'])} partial) "
          f"({fmt(freed)}), {'freed' if args.apply else 'would free'} {freed} bytes")
    return 0


if __name__ == "__main__":
    sys.exit(main())
