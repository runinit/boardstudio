#!/usr/bin/env python3
"""Compare cold-run unit logs from .evidence/cold-build-port/.

Usage: report.py <run> [<run>...] [--lanes] [--attrib crate,crate,...] [--sccache]
"""
import collections
import importlib.util
import json
import re
import sys
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parent.parent.parent / ".evidence" / "cold-build-port"


def cold():
    spec = importlib.util.spec_from_file_location("_cold", Path(__file__).resolve().parent / "cold.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def load(name):
    with (EVIDENCE / f"{name}.jsonl").open() as log:
        records = [json.loads(line) for line in log]
    with (EVIDENCE / f"{name}.log").open(errors="replace") as log:
        lines = log.read().splitlines()
    summary = next(line for line in reversed(lines) if line.startswith("check: step times: "))
    walls = {step: float(wall) for step, wall in re.findall(r"(\w+) (\d+\.\d+)s", summary)}
    run_time = next(line for line in reversed(lines) if line.startswith("cold: wall "))
    return records, walls, run_time


def tree(r):
    o = r["out"] or ""
    t = "cad" if "/cad/" in o else "root"
    p = next((x for x in ("wasm-check", "wasm-dev", "release", "debug") if f"/{x}/" in o), "?")
    return f"{t}/{r['target'].split('-')[0]}/{p}"


def active_union(records):
    total, end = 0.0, None
    for a, b in sorted((r["start"], r["start"] + r["secs"]) for r in records):
        if end is None or a >= end:
            total += b - a
            end = b
        elif b > end:
            total += b - end
            end = b
    return total


def attributed_wall(records, crates):
    """Wall time split equally among concurrently running units; the selected set's share."""
    points = sorted({r["start"] for r in records} | {r["start"] + r["secs"] for r in records})
    total = 0.0
    for a, b in zip(points, points[1:]):
        running = [r for r in records if r["start"] <= a < r["start"] + r["secs"]]
        if running:
            total += (b - a) * sum(1 for r in running if r["crate"] in crates) / len(running)
    return total


def real_cpu(records):
    """Summed user+system time of the units that recorded it; None for logs from before `cpu`."""
    if not any("cpu" in r for r in records):
        return None
    return sum(r.get("cpu", 0.0) for r in records)


def cpu_display(records):
    cpu = real_cpu(records)
    return f"{cpu:.0f}" if cpu is not None else "-"


def compare(names, data):
    print(f"{'step':10s}" + "".join(f"{name + ' wall':>16s}{'compile':>9s}{'cpu':>7s}" for name in names))
    for step in ("repo", "tooling", "lint", "build", "test", "browser", "total"):
        row = f"{step:10s}"
        for name in names:
            records, walls, _ = data[name]
            units = records if step == "total" else [r for r in records if r["step"] == step]
            if step not in walls:
                row += f"{'-':>16s}{'-':>9s}{'-':>7s}"
                continue
            row += f"{walls[step]:16.1f}{active_union(units):9.1f}{cpu_display(units):>7s}"
        print(row)
    for name in names:
        records, _, run_time = data[name]
        print(f"{name}: real compiler child CPU {cpu_display(records)}s, {run_time.removeprefix('cold: ')}")

    groups = collections.defaultdict(lambda: collections.defaultdict(list))
    for name in names:
        for record in data[name][0]:
            groups[tree(record)][name].append(record)
    print()
    for lane, runs in sorted(groups.items()):
        print(f"{lane:30s}" + "".join(f"{cpu_display(runs[name]):>9s}" for name in names))


def lanes(names, data):
    for n in names:
        R, _, _ = data[n]
        groups = collections.defaultdict(list)
        for r in R:
            groups[tree(r)].append(r)
        print(f"{n}: lane (target/profile) start/end are offsets from the first unit")
        if not R:
            continue
        base = min(r["start"] for r in R)
        for k, rs in sorted(groups.items(), key=lambda x: min(r["start"] for r in x[1])):
            start = min(r["start"] for r in rs) - base
            end = max(r["start"] + r["secs"] for r in rs) - base
            elapsed = sum(r["secs"] for r in rs)
            print(f"  {k:30s} {start:8.1f} {end:8.1f} cpu {cpu_display(rs):>7s}s  par {elapsed / max(end - start, 0.01):4.1f}x  units {len(rs)}")


def attrib(names, data, crates):
    for n in names:
        R, _, _ = data[n]
        wall = attributed_wall(R, crates)
        cpu = cpu_display([r for r in R if r["crate"] in crates])
        print(f"{n}: {','.join(sorted(crates))}: attributed wall {wall:.1f}s, cpu {cpu}s")
        for crate in sorted(crates):
            rs = [r for r in R if r["crate"] == crate]
            if rs:
                print(f"  {crate:30s} {len(rs):4d}x cpu {cpu_display(rs):>6s}s attrib {attributed_wall(R, {crate}):6.1f}s")


def sccache(names):
    """One-line sccache counter deltas per run from its before/after snapshots."""
    mod = cold()
    for name in names:
        before = EVIDENCE / f"{name}-sccache-before.log"
        after = EVIDENCE / f"{name}-sccache-after.log"
        if not before.is_file() or not after.is_file():
            print(f"{name}: no sccache snapshots (run without cache or daemon unreachable)")
            continue
        print(f"{name}: " + mod.format_sccache_delta(
            mod.parse_sccache_stats(before.read_text(errors="replace")),
            mod.parse_sccache_stats(after.read_text(errors="replace"))).removeprefix("cold: "))


def main(argv):
    lanes_flag = "--lanes" in argv
    sccache_flag = "--sccache" in argv
    attrib_set = None
    names = []
    it = iter(range(len(argv)))
    for i in it:
        if argv[i] == "--attrib":
            j = next(it, None)
            attrib_set = set(argv[j].split(",")) - {""} if j is not None else set()
        elif argv[i] not in ("--lanes", "--sccache"):
            names.append(argv[i])
    if not names:
        print(__doc__, file=sys.stderr)
        return 2
    if sccache_flag:
        sccache(names)
        return 0
    data = {n: load(n) for n in names}
    if lanes_flag:
        lanes(names, data)
    elif attrib_set is not None:
        attrib(names, data, attrib_set)
    else:
        compare(names, data)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
