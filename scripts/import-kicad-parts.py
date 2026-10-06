#!/usr/bin/env python3
"""Reviewed KiCad sources -> reusable Board Studio definitions via the Rust importer.

Build the driver first: cargo build --manifest-path core/Cargo.toml --example artifact_request

Usage: import-kicad-parts.py [--check] [MANIFEST [OUTPUT]]
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import sys

import artifact_driver
from catalogue_json import dumps

ROOT = Path(__file__).resolve().parents[1]
KINDS = ("switch", "controller", "connector", "encoder", "passive", "custom", "utility")
REQUIRED = ("id", "name", "kind", "file", "sha256", "repository", "revision", "sourcePath", "license", "licenseFile")


def artifact(payload: dict) -> dict:
    reply = artifact_driver.request({"id": "catalogue-import", **payload}, "Rust importer failed")
    if reply["kind"] == "error":
        raise RuntimeError(reply["error"]["message"])
    if reply["kind"] != payload["kind"]:
        raise RuntimeError("Unexpected importer response")
    return reply["result"]


def import_parts(manifest_path: Path) -> list[dict]:
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if manifest.get("formatVersion") != 1 or not isinstance(manifest.get("entries"), list):
        raise ValueError("Expected a version 1 import manifest")
    base = manifest_path.parent
    ids: set[str] = set()
    parts = []
    for entry in manifest["entries"]:
        for field in REQUIRED:
            if not isinstance(entry.get(field), str) or not entry[field].strip():
                raise ValueError(f"Missing {field}")
        if entry["id"] in ids:
            raise ValueError(f"Duplicate definition ID: {entry['id']}")
        ids.add(entry["id"])
        (base / entry["licenseFile"]).read_bytes()
        raw = (base / entry["file"]).read_bytes()
        if hashlib.sha256(raw).hexdigest() != entry["sha256"]:
            raise ValueError(f"Source hash mismatch: {entry['file']}")
        source = raw.decode("utf-8")
        imported = artifact({"kind": "import-footprint", "definitionId": entry["id"], "source": source})
        definition = {**imported["definition"], "name": entry["name"], "kind": entry["kind"]}
        if entry["kind"] not in KINDS:
            raise ValueError(f"Invalid kind: {entry['kind']}")
        # Roles are explicit review decisions; never guess a circuit from pad numbers.
        if entry.get("terminals"):
            terminals = {}
            for role, numbers in entry["terminals"].items():
                if not isinstance(numbers, list) or not numbers or any(not isinstance(number, str) or not number for number in numbers):
                    raise ValueError(f"Invalid terminal mapping: {role}")
                pads = []
                for number in numbers:
                    matches = [pad for pad in definition["pads"] if pad["number"] == number and not (pad.get("drill") and pad.get("plated") is False)]
                    if not matches:
                        raise ValueError(f"Missing conductive pad {number} for {role}")
                    pads.extend(pad["id"] for pad in matches)
                terminals[role] = list(dict.fromkeys(pads))
            definition["terminals"] = terminals
        if entry.get("matrixTerminals"):
            for role in (entry["matrixTerminals"]["row"], entry["matrixTerminals"]["column"]):
                if not definition.get("terminals", {}).get(role):
                    raise ValueError(f"Missing matrix terminal: {role}")
            definition["matrixTerminals"] = entry["matrixTerminals"]
        for field in ("inputProfile", "hardwareProfile", "models"):
            if entry.get(field):
                definition[field] = entry[field]
        mechanical = entry.get("mechanical")
        if mechanical:
            extracted = artifact({"kind": "extract-mechanical", "source": source, "mappings": mechanical["mappings"], "maxDeviationMm": 0.02})
            definition["mechanicalProfile"] = {
                "definitionId": entry["id"],
                "source": f"{entry['repository']}/blob/{entry['revision']}/{entry['sourcePath']}",
                "cutouts": extracted["plateCutouts"], "pcbHoles": extracted["pcbHoles"],
                "clearances": extracted["clearanceEnvelopes"], "sourceGeometry": extracted["sourceGeometry"],
                "plateToPcb": mechanical["plateToPcb"],
                **({"switchFamily": mechanical["switchFamily"]} if mechanical.get("switchFamily") else {}),
            }
            if mechanical.get("clearanceVolumes"):
                definition["mechanicalProfile"]["clearanceVolumes"] = mechanical["clearanceVolumes"]
        parts.append({
            "definition": definition, "diagnostics": imported["diagnostics"],
            "provenance": {key: entry[key] for key in ("repository", "revision", "sourcePath", "sha256", "license", "licenseFile")},
        })
    return parts


def main(argv: list[str]) -> int:
    check = "--check" in argv
    args = [arg for arg in argv if arg != "--check"]
    manifest_path = Path(args[0]).resolve() if args else ROOT / "catalogue/parts/import-manifest.json"
    output_path = Path(args[1]).resolve() if len(args) > 1 else manifest_path.parent / "imported-parts.json"
    try:
        parts = import_parts(manifest_path)
        output = dumps({"formatVersion": 1, "parts": parts}) + "\n"
        if check:
            if output_path.read_bytes().decode("utf-8") != output:
                raise ValueError("Imported catalogue drift: regenerate and review the changed definitions")
            print(f"Verified {len(parts)} imported parts")
        else:
            # All sources and conversions must succeed before publishing a catalogue.
            output_path.write_bytes(output.encode("utf-8"))
            print(f"Imported {len(parts)} parts into {output_path}")
    except (ValueError, RuntimeError, OSError) as error:
        print(error, file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
