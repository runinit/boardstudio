#!/usr/bin/env python3
"""Pinned board sources -> Rust-owned snapshots. No browser-side PCB parser.

Build the driver first: cargo build --manifest-path core/Cargo.toml --example artifact_request

Usage: import-vik-modules.py [--check]
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import re
import sys

import artifact_driver
from catalogue_json import dumps

ROOT = Path(__file__).resolve().parents[1]
MANIFEST_PATH = ROOT / "catalogue/modules/import-manifest.json"
OUTPUT_PATH = ROOT / "catalogue/modules/imported-modules.json"
REQUIRED = ("row", "name", "file", "sha256", "repository", "revision", "sourcePath", "license", "family", "variant")
AXES = (("X", "XMin", "XMax", "XLength"), ("Y", "YMin", "YMax", "YLength"), ("Z", "ZMin", "ZMax", "ZLength"))


def slug(text: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", text, flags=re.IGNORECASE)


def is_finite_number(value) -> bool:
    return isinstance(value, (int, float)) and not isinstance(value, bool)


def verify_assets(assets: list[dict], variants_by_row: dict[str, set[str]]) -> None:
    for asset in assets:
        if hashlib.sha256((ROOT / asset["bundledFile"]).read_bytes()).hexdigest() != asset["sha256"]:
            raise ValueError(f"Model hash mismatch: {asset['path']}")
        bounds = asset.get("nativeBoundsMm") or {}
        for axis, low, high, length in AXES:
            lower, upper, extent = bounds.get(low), bounds.get(high), bounds.get(length)
            if not all(is_finite_number(item) for item in (lower, upper, extent)) or upper <= lower or abs((upper - lower) - extent) > 0.00001:
                raise ValueError(f"Invalid {axis} bounds in model ledger: {asset['path']}")
        if "appliesToVariants" in asset:
            if not isinstance(asset["appliesToVariants"], list) or not asset["appliesToVariants"]:
                raise ValueError(f"Empty model variant applicability: {asset['path']}")
            known = variants_by_row.get(asset["catalogRow"], set())
            for variant in asset["appliesToVariants"]:
                if variant not in known:
                    raise ValueError(f"Unknown {asset['catalogRow']} model variant {variant}: {asset['path']}")


def import_entry(entry: dict, repair: str | None, assets: list[dict], ids: set[str]) -> dict:
    for field in REQUIRED:
        if not isinstance(entry.get(field), str) or not entry[field].strip():
            raise ValueError(f"Missing {field}")
    variant = f"{entry['variant']} · 3V3 pullups, JP1 bridged" if repair else entry["variant"]
    module_id = f"vik:{slug(entry['row'])}:{slug(variant)}"
    if module_id in ids:
        raise ValueError(f"Duplicate module identity: {module_id}")
    ids.add(module_id)
    raw = (MANIFEST_PATH.parent / entry["file"]).read_bytes()
    if hashlib.sha256(raw).hexdigest() != entry["sha256"]:
        raise ValueError(f"Source hash mismatch: {entry['file']}")
    provenance = {"repository": entry["repository"], "revision": entry["revision"], "path": entry["sourcePath"], "license": entry["license"]}
    if "status" in entry:
        provenance["upstreamStatus"] = entry["status"]
    payload = {
        "id": "module-import", "kind": "import-module-board", "definitionId": module_id, "name": entry["name"],
        "source": raw.decode("utf-8"), "provenance": provenance, "family": entry["family"], "variant": variant,
        **({"repair": repair} if repair else {}),
    }
    reply = artifact_driver.request(payload, f"Importer failed for {entry['file']}")
    if reply["kind"] != "import-module-board":
        message = (reply.get("error") or {}).get("message", "Unexpected module reply")
        raise RuntimeError(f"{entry['file']}: {message}")
    definition = reply["result"]
    definition["catalogueRow"] = entry["row"]
    purchased = entry.get("purchasedReferences") or []
    for constituent in definition["constituents"]:
        constituent["purchased"] = constituent["reference"] in purchased
    candidates = []
    for asset in assets:
        if (asset["catalogRow"] == entry["row"] or asset["catalogRow"] == "shared connector") \
                and ("appliesToVariants" not in asset or entry["variant"] in asset["appliesToVariants"]):
            bounds = asset["nativeBoundsMm"]
            candidates.append({
                "assetId": asset["assetId"], "name": asset["path"].split("/")[-1],
                "source": {"repository": f"https://github.com/{asset['repository']}", "revision": asset["revision"], "path": asset["path"], "license": asset["declaredLicense"], "sha256": asset["sha256"]},
                "boundsMin": {"x": bounds["XMin"], "y": bounds["YMin"], "z": bounds["ZMin"]},
                "boundsMax": {"x": bounds["XMax"], "y": bounds["YMax"], "z": bounds["ZMax"]},
                "verifiedAlignment": False,
            })
    definition["candidateModels"] = candidates
    aligned = next((asset for asset in assets if asset["catalogRow"] == entry["row"] and entry["variant"] in asset.get("appliesToVariants", []) and asset.get("boardToModelTransform")), None)
    if aligned:
        transform = aligned["boardToModelTransform"]
        definition["models"] = [{"assetId": aligned["assetId"], "offset": transform["offset"], "rotation": transform["rotation"], "scale": transform["scale"]}]
        profile = aligned.get("nominalProfile")
        if profile:
            definition["volumes"] = [{"id": f"{module_id}/nominal-occupied", "geometry": profile["occupied"], "source": f"{aligned['path']} · source-derived native bounds transformed into PCB midplane", "purpose": "nominal occupied", "qualified": False}]
            definition["openings"] = [{"id": f"{module_id}/nominal-viewing-aperture", "geometry": profile["viewingAperture"], "source": profile["viewingApertureSource"], "purpose": "nominal viewing aperture", "qualified": False}]
            definition["gates"].append({"output": "mechanical", "code": "nominal-display-assumptions", "message": "Review the manufacturer active-area rectangle and its centering on the source display-body bounds in the app before qualifying case output."})
    electrical = entry["electrical"]
    definition["electrical"] = electrical
    if electrical.get("rotaryProfile") and entry["row"] == "ec11-evqwgd001":
        definition["gates"] = [gate for gate in definition["gates"] if not (gate["output"] == "firmware" and gate["code"] == "module-driver")]
    if electrical.get("protocol") == "pass-through":
        definition["gates"] = [gate for gate in definition["gates"] if gate["output"] != "firmware"]
    if entry.get("notice") and not repair:
        definition["gates"].append({"output": "electrical", "code": "source-caveat", "message": entry["notice"]})
    if electrical.get("protocol") == "nonstandard":
        definition["gates"].append({"output": "electrical", "code": "nonstandard-interface", "message": "Select and qualify the source-specific signal mapping, analog or half-duplex capabilities and isolated bus resources before electrical handoff."})
    if entry["row"] == "haptic-drv2605l":
        definition["gates"].append({"output": "mechanical", "code": "actuator", "message": "Choose and qualify the separate actuator body, mounting, current and calibration."})
    return {"row": entry["row"], "definition": definition}


def import_modules() -> list[dict]:
    manifest = json.loads(MANIFEST_PATH.read_text(encoding="utf-8"))
    assets = json.loads((ROOT / "catalogue/modules/asset-ledger.json").read_text(encoding="utf-8"))["models"]
    if manifest.get("formatVersion") != 1 or not isinstance(manifest.get("entries"), list):
        raise ValueError("Expected a version 1 module manifest")
    variants_by_row: dict[str, set[str]] = {}
    for entry in manifest["entries"]:
        variants = variants_by_row.setdefault(entry["row"], set())
        if entry["variant"] in variants:
            raise ValueError(f"Duplicate source variant: {entry['row']} / {entry['variant']}")
        variants.add(entry["variant"])
    verify_assets(assets, variants_by_row)
    ids: set[str] = set()
    modules = []
    for entry in manifest["entries"]:
        modules.append(import_entry(entry, None, assets, ids))
        if entry["row"] == "haptic-drv2605l":
            modules.append(import_entry(entry, "drv2605l-pullups3v3", assets, ids))
    return modules


def main(argv: list[str]) -> int:
    try:
        modules = import_modules()
        output = dumps({"formatVersion": 1, "modules": modules}) + "\n"
        if "--check" in argv:
            if OUTPUT_PATH.read_bytes().decode("utf-8") != output:
                raise ValueError("Module catalogue drift: regenerate and review the snapshots")
            print(f"Verified {len(modules)} module variants")
        else:
            OUTPUT_PATH.write_bytes(output.encode("utf-8"))
            print(f"Imported {len(modules)} module variants")
    except (ValueError, RuntimeError, OSError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
