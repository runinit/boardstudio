#!/usr/bin/env python3
"""Retain measured key positions, not upstream footprints, circuits, or artwork.

Usage: extract-keyboard-layouts.py /path/to/checkouts [--check]

Each upstream repository is expected in its own folder under the checkouts path,
at the revision recorded in content/layouts/keyboard-layouts.json. `--check`
compares the extraction with the committed file instead of rewriting it.
"""

from __future__ import annotations

import hashlib
import math
from pathlib import Path
import re
import shutil
import subprocess
import sys

from catalogue_json import dumps
from kicad_forms import child, parse_forms, value

ROOT = Path(__file__).resolve().parents[1]
OUTPUT = ROOT / "content/layouts/keyboard-layouts.json"
LICENSES = ROOT / "content/demos/licenses"


def numbered(prefix: str, count: int):
    pattern = re.compile(rf"^{prefix}\d+$")
    return lambda reference, _name: bool(pattern.match(reference)) and int(reference[len(prefix):]) <= count


def matching(pattern: str):
    expression = re.compile(pattern)
    return lambda reference, _name: bool(expression.search(reference))


def excluding(names: list[str], pattern: str | None = None):
    expression = re.compile(pattern) if pattern else None
    return lambda reference, _name: reference not in names and not (expression and expression.search(reference))


# id, name, folder, path, split, choc, count, include(reference, footprint)
CONFIGURATIONS = [
    ("corne", "Corne", "corne-v3", "corne-cherry/pcb/corne-cherry.kicad_pcb", True, False, 21, numbered("SW", 21)),
    ("lily58", "Lily58", "Lily58", "Pro/PCB/Lily58_Pro.kicad_pcb", True, False, 29, numbered("SW", 29)),
    ("sweep", "Ferris Sweep", "Sweep", "Sweep v2.2/sweepv2.kicad_pcb", True, True, 17, matching(r"^SW\d+$")),
    ("chocofi", "Chocofi", "chocofi", "pcb/chocofi.kicad_pcb", True, True, 18, matching(r"^SW\d+$")),
    ("reviung41", "REVIUNG41", "reviung", "reviung41/pcb/ver1.3/reviung41.kicad_pcb", False, False, 41, numbered("SW", 41)),
    ("totem", "TOTEM", "TOTEM", "PCB/totem_0-3/totem_0_3.kicad_pcb", True, True, 19, matching(r"^SWL\d+$")),
    ("klor", "KLOR", "KLOR", "PCB/klor1_3/klor1_3.kicad_pcb", True, False, 21, lambda *_: True),
    ("cantor", "Cantor", "cantor", "Cantor_Classic/keyboard_pcb.kicad_pcb", True, True, 21, lambda *_: True),
    ("gh60", "GH60 · ANSI", "gh60", "keyboard.kicad_pcb", False, False, 61, lambda _reference, name: name == "mx1a:MX1A"),
    ("discipline", "Discipline · ANSI", "discipline", "discipline-pcb.kicad_pcb", False, False, 68, numbered("SW", 68)),
    ("mysterium", "Mysterium · ANSI", "mysterium", "mysterium-pcb.kicad_pcb", False, False, 87,
     lambda reference, _name: bool(re.search(r"^SW\d+$", reference)) and int(reference[2:]) not in (51, 77, 80, 81, 84, 86, 89, 90)),
    ("voyager97", "Voyager97 · 103-key layout", "Voyager97", "Voyager97.kicad_pcb", False, False, 103,
     excluding(["BS2", "BS1-2", "CL1-2", "LM1-2", "LM3-2", "RM1-2", "RM3-2", "SP2"], r"^MX_NUM(11|15|19|20|21|23)$")),
    ("voyager104", "Voyager104 · ANSI", "Voyager104", "Voyager104.kicad_pcb", False, False, 104,
     excluding(["MX_\\2"], r"^MX_(BACK[23]|CLOCK2|LCTRL2|LSHIFT[23]|RETURN2|RSHIFT[23]|SP2|LALT2|LGUI2|RALT2|RCTRL2|RGUI2|NUM(1|2|3|4|12|16|20|21|22|24))$")),
    ("plaid", "Plaid", "plaid", "pcb/plaid.kicad_pcb", False, False, 48, numbered("SW", 48)),
    ("lumberjack", "Lumberjack", "lumberjack-keyboard", "lumberjack.kicad_pcb", False, False, 60, numbered("MX", 60)),
]
# Source switches with a separately drawn stabilizer have no key size metadata.
WIDTHS = {
    "gh60": {"S66": 2, "S02": 1.5, "S67": 1.5, "S03": 1.75, "S68": 2.25, "S04": 2.25, "S69": 2.75, "S30": 6.25,
             "S05": 1.25, "S10": 1.25, "S15": 1.25, "S55": 1.25, "S60": 1.25, "S65": 1.25, "S70": 1.25},
    "discipline": {"SW62": 6.25, "SW63": 1, "SW64": 1},
    "mysterium": {"SW83": 6.25},
}
SWITCH = re.compile(r"MX|PG1350|choc-v1|SK6812MINI_and_cherry", re.IGNORECASE)
NOT_A_SWITCH = re.compile(r"(?:^|:)(?:Stab|MXST)|(?:_|-)Stabilizer$|wire|led", re.IGNORECASE)
WIDTH = re.compile(r"(?:_|-)([\d.]+)[uU]")


def git(checkout: Path, *arguments: str) -> str:
    return subprocess.run(["git", "-C", str(checkout), *arguments], capture_output=True, text=True, check=True).stdout.strip()


def snap_rotation(angle: float) -> float:
    """((angle + 90) % 180 + 180) % 180 - 90 with JavaScript's sign-preserving remainder."""
    shifted = angle + 90
    return math.fmod(math.fmod(shifted, 180) + 180, 180) - 90


def measure(board: list, count_label: str, include) -> list[dict]:
    keys = []
    for module in board:
        if not (isinstance(module, list) and module and module[0] in ("module", "footprint")):
            continue
        footprint = value(module[1])
        if not SWITCH.search(footprint) or NOT_A_SWITCH.search(footprint):
            continue
        ref = next((node for node in module if isinstance(node, list) and node and (
            (node[0] == "fp_text" and node[1] == "reference") or (node[0] == "property" and value(node[1]) == "Reference"))), None)
        reference = value(ref[2]) if ref is not None and len(ref) > 2 else ""
        if not include(reference, footprint):
            continue
        at = child(module, "at")
        found = WIDTH.search(footprint)
        keys.append({
            "reference": reference, "x": float(value(at[1])), "y": float(value(at[2])),
            "rotation": snap_rotation(float(value(at[3] if len(at) > 3 else "0"))),
            "width": float(found.group(1)) if found else 1.0,
        })
    keys.sort(key=lambda key: (key["y"], key["x"]))
    return keys


def extract(root: Path) -> dict:
    layouts = {}
    for layout_id, name, folder, path, split, choc, count, include in CONFIGURATIONS:
        checkout = root / folder
        revision = git(checkout, "rev-parse", "HEAD")
        repository = re.sub(r"\.git$", "", git(checkout, "remote", "get-url", "origin"))
        raw = (checkout / path).read_bytes()
        board = parse_forms(raw.decode("utf-8"))[0]
        keys = measure(board, layout_id, include)
        if len(keys) != count:
            raise ValueError(f"{layout_id}: expected {count}, got {len(keys)}: {','.join(key['reference'] for key in keys)}")
        for key in keys:
            key["width"] = WIDTHS.get(layout_id, {}).get(key["reference"], key["width"])
        license_file = next((file for file in ("LICENSE", "LICENSE.md", "LICENSE.txt") if (checkout / file).exists()), None)
        layouts[layout_id] = {
            "name": name, "split": split, "choc": choc, "repository": repository, "revision": revision, "path": path,
            "sha256": hashlib.sha256(raw).hexdigest(), "licenseFile": f"licenses/{layout_id}.txt" if license_file else None,
            "keys": keys, "_license": checkout / license_file if license_file else None,
        }
    return layouts


def main(argv: list[str]) -> int:
    check = "--check" in argv
    arguments = [argument for argument in argv if argument != "--check"]
    if not arguments:
        print("Usage: extract-keyboard-layouts.py /path/to/checkouts [--check]", file=sys.stderr)
        return 2
    try:
        layouts = extract(Path(arguments[0]))
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(error, file=sys.stderr)
        return 1
    licenses = {layout_id: layout.pop("_license") for layout_id, layout in layouts.items()}
    output = dumps(layouts) + "\n"
    if check:
        if OUTPUT.read_bytes().decode("utf-8") != output:
            print("Keyboard layout drift: regenerate and review the measurements", file=sys.stderr)
            return 1
        print(f"Verified {len(layouts)} layouts")
        return 0
    LICENSES.mkdir(parents=True, exist_ok=True)
    for layout_id, license_path in licenses.items():
        if license_path:
            shutil.copyfile(license_path, LICENSES / f"{layout_id}.txt")
    OUTPUT.write_bytes(output.encode("utf-8"))
    print(f"Measured {len(layouts)} layouts")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
