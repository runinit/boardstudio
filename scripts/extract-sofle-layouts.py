#!/usr/bin/env python3
"""Extract measurements only; demo footprints come from the bundled catalogue.

Usage: extract-sofle-layouts.py /path/to/SofleKeyboard [--check]

The checkout must be at the pinned upstream revision. `--check` compares the
extraction with content/layouts/sofle-layouts.json instead of rewriting it.
"""

from __future__ import annotations

import hashlib
import math
from pathlib import Path
import re
import subprocess
import sys

from catalogue_json import dumps
from kicad_forms import child, parse_forms, value

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "content/layouts/sofle-layouts.json"
EXPECTED_REVISION = "fb294f7c58d0f91c379a9ef339159d4d056643e9"
REPOSITORY = "https://github.com/josefadamcik/SofleKeyboard"
COMPONENT = re.compile(r"^(SW\d+|U1|J2|J3|RSW1|TH\d+|D3[1-7])$")


def children(node: list, head: str) -> list:
    return [item for item in node if isinstance(item, list) and item and item[0] == head]


def point(node: list) -> dict:
    return {"x": float(value(node[1])), "y": float(value(node[2]))}


def close(a: dict, b: dict) -> bool:
    return math.hypot(a["x"] - b["x"], a["y"] - b["y"]) < 0.002


def natural(reference: str):
    """Order like `localeCompare` with numeric collation: SW2 before SW10."""
    return [(0, int(part)) if part.isdigit() else (1, part) for part in re.split(r"(\d+)", reference) if part]


def area(points: list[dict]) -> float:
    total = 0.0
    for index, a in enumerate(points):
        b = points[(index + 1) % len(points)]
        total += a["x"] * b["y"] - b["x"] * a["y"]
    return abs(total)


def contours_of(board: list, variant: str) -> list[list[dict]]:
    edges = [node for node in board if isinstance(node, list) and (child(node, "layer") or [None, None])[1] == "Edge.Cuts"]
    if any(edge[0] != "gr_line" for edge in edges):
        raise ValueError("Expected polygonal source edges")
    segments = [[point(child(edge, "start")), point(child(edge, "end"))] for edge in edges]
    contours = []
    while segments:
        start, end = segments.pop(0)
        outline = [start, end]
        while not close(outline[0], outline[-1]):
            tail = outline[-1]
            index = next((i for i, (a, b) in enumerate(segments) if close(tail, a) or close(tail, b)), -1)
            if index < 0:
                raise ValueError(f"{variant}: disconnected outline at {dumps(tail)}")
            a, b = segments.pop(index)
            outline.append(b if close(tail, a) else a)
        outline.pop()
        contours.append(outline)
    return contours


def extract(checkout: Path) -> dict:
    revision = subprocess.run(["git", "-C", str(checkout), "rev-parse", "HEAD"], capture_output=True, text=True, check=True).stdout.strip()
    if revision != EXPECTED_REVISION:
        raise ValueError(f"Expected upstream revision {EXPECTED_REVISION}")
    layouts = {}
    for variant in ("v2", "RGB", "Choc"):
        path = f"Sofle_{variant}/PCB/SofleKeyboard.kicad_pcb"
        raw = (checkout / path).read_bytes()
        board = parse_forms(raw.decode("utf-8"))[0]
        components = []
        for module in children(board, "module"):
            reference = value(child(module, "fp_text")[2])
            if not COMPONENT.match(reference):
                continue
            at = child(module, "at")
            components.append({"reference": reference, **point(at), "rotation": float(value(at[3] if len(at) > 3 else "0"))})
        components.sort(key=lambda component: natural(component["reference"]))
        contours = contours_of(board, variant)
        contours.sort(key=area, reverse=True)
        outline, *cutouts = contours
        layouts[variant.lower()] = {
            "path": path, "sha256": hashlib.sha256(raw).hexdigest(), "components": components,
            "outline": outline, "sourceCutoutCount": len(cutouts),
        }
    return {"repository": REPOSITORY, "revision": revision, "layouts": layouts}


def main(argv: list[str]) -> int:
    check = "--check" in argv
    arguments = [argument for argument in argv if argument != "--check"]
    if not arguments:
        print("Usage: extract-sofle-layouts.py /path/to/SofleKeyboard [--check]", file=sys.stderr)
        return 2
    try:
        output = dumps(extract(Path(arguments[0]))) + "\n"
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
    if check:
        if OUTPUT.read_bytes().decode("utf-8") != output:
            print("Sofle layout drift: regenerate and review the measurements", file=sys.stderr)
            return 1
        print("Verified 3 Sofle layouts")
        return 0
    OUTPUT.write_bytes(output.encode("utf-8"))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
