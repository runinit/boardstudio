#!/usr/bin/env python3
"""Compare cold-run unit logs from .evidence/build-time/.

Usage: report.py <run> [<run>...] [--lanes] [--attrib crate,crate,...]
"""
import collections, json, re, sys
import datetime as D
from pathlib import Path

EVIDENCE = Path(__file__).resolve().parent.parent.parent / ".evidence" / "build-time"


def load(n):
    R = [json.loads(l) for l in open(EVIDENCE / f"{n}.jsonl")]
    marks = []
    for l in open(EVIDENCE / f"{n}.log", errors="replace").read().splitlines():
        m = re.match(r"^== (\w[\w-]*)", l)
        if m:
            marks.append([m[1], None, None])
        m = re.match(r"^check: (start|end) (\S+)", l)
        if m and marks:
            t = D.datetime.fromisoformat(m[2]).timestamp()
            if marks[-1][1] is None:
                marks[-1][1] = t
            marks[-1][2] = t
    return R, marks


def tree(r):
    o = r["out"] or ""
    t = "cad" if "/cad/" in o else "harness" if "browser-harness" in o else "root"
    p = next((x for x in ("wasm-dev", "release", "debug") if f"/{x}/" in o), "?")
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


def compare(names, data):
    print(f"{'step':10s}" + "".join(f"{n + ' wall':>16s}{'compile':>9s}{'cpu':>7s}" for n in names))
    for s in ["repo", "tooling", "lint", "build", "test", "browser"]:
        row = f"{s:10s}"
        for n in names:
            R, M = data[n]
            m = next((x for x in M if x[0] == s), None)
            if not m or m[1] is None:
                row += f"{'-':>16s}{'-':>9s}{'-':>7s}"
                continue
            ivs = sorted((r["start"], r["start"] + r["secs"], r["secs"]) for r in R if m[1] <= r["start"] <= m[2])
            act, end = 0.0, m[1]
            for a, b, _ in ivs:
                if b <= end:
                    continue
                act += b - max(a, end)
                end = b
            row += f"{m[2] - m[1]:16.1f}{act:9.1f}{sum(i[2] for i in ivs):7.0f}"
        print(row)
    print(f"{'total':10s}" + "".join(f"{sum(x[2] - x[1] for x in data[n][1] if x[1]):16.1f}{'':9s}{sum(r['secs'] for r in data[n][0]):7.0f}" for n in names))
    g = collections.defaultdict(lambda: {n: 0 for n in names})
    for n in names:
        for r in data[n][0]:
            g[tree(r)][n] += r["secs"]
    print()
    for k, v in sorted(g.items(), key=lambda x: -max(x[1].values())):
        if max(v.values()) > 5:
            print(f"{k:30s}" + "".join(f"{v[n]:9.0f}" for n in names))
    print()
    c = collections.defaultdict(lambda: {n: [0, 0.0] for n in names})
    for n in names:
        for r in data[n][0]:
            c[r["crate"]][n][0] += 1
            c[r["crate"]][n][1] += r["secs"]
    for k, v in sorted(c.items(), key=lambda x: -x[1][names[0]][1])[:30]:
        print(f"{k:30s}" + "".join(f"{v[n][0]:5d}x{v[n][1]:6.0f}s" for n in names))


def lanes(names, data):
    for n in names:
        R, _ = data[n]
        groups = collections.defaultdict(list)
        for r in R:
            groups[tree(r)].append(r)
        print(f"{n}: lane (target/profile) start/end are offsets from the first unit")
        base = min(r["start"] for r in R)
        for k, rs in sorted(groups.items(), key=lambda x: min(r["start"] for r in x[1])):
            start = min(r["start"] for r in rs) - base
            end = max(r["start"] + r["secs"] for r in rs) - base
            cpu = sum(r["secs"] for r in rs)
            print(f"  {k:30s} {start:8.1f} {end:8.1f} cpu {cpu:7.0f}s  par {cpu / max(end - start, 0.01):4.1f}x  units {len(rs)}")


def attrib(names, data, crates):
    for n in names:
        R, _ = data[n]
        wall = attributed_wall(R, crates)
        cpu = sum(r["secs"] for r in R if r["crate"] in crates)
        print(f"{n}: {','.join(sorted(crates))}: attributed wall {wall:.1f}s, cpu {cpu:.0f}s")
        for crate in sorted(crates):
            rs = [r for r in R if r["crate"] == crate]
            if rs:
                print(f"  {crate:30s} {len(rs):4d}x cpu {sum(r['secs'] for r in rs):6.0f}s attrib {attributed_wall(R, {crate}):6.1f}s")


def main(argv):
    lanes_flag = "--lanes" in argv
    attrib_set = None
    names = []
    it = iter(range(len(argv)))
    for i in it:
        if argv[i] == "--attrib":
            j = next(it, None)
            attrib_set = set(argv[j].split(",")) - {""} if j is not None else set()
        elif argv[i] != "--lanes":
            names.append(argv[i])
    if not names:
        print(__doc__, file=sys.stderr)
        return 2
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
