#!/usr/bin/env python3
"""Stage React-compatible Ergogen models at safe static Dioxus asset paths."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re

EXTENSIONS = {".step", ".stp", ".wrl", ".stl"}
ALIASES = {
    ("thqwgd001", "THQWGD001-rotation.stp"): "THQWGD001 #1.stp",
    ("thqwgd001", "THQWGD001C-2pin.stp"): "THQWGD001C [2pin] #1.stp",
    ("thqwgd001", "THQWGD001C-4pin.stp"): "THQWGD001C [4pin] #1.stp",
}


def stable_model_token(source_relative_path: str) -> int:
    token = 0xCBF29CE484222325
    for byte in source_relative_path.encode("utf-8"):
        token = ((token ^ byte) * 0x100000001B3) & 0xFFFFFFFFFFFFFFFF
    return token


def catalogue(source_root: Path) -> list[dict[str, str]]:
    files = sorted(
        (path for path in source_root.glob("*/3d_models/**/*")
         if path.is_file() and path.suffix.lower() in EXTENSIONS),
        key=lambda path: path.relative_to(source_root).as_posix(),
    )
    entries: list[dict[str, str]] = []
    paths: set[str] = set()
    ids: set[str] = set()
    tokens: set[str] = set()
    for path in files:
        relative = path.relative_to(source_root).as_posix()
        vendor, model_path = relative.split("/3d_models/", maxsplit=1)
        filename = ALIASES.get((vendor, model_path), model_path)
        asset_id = f"ergogen:model:{vendor}/{filename}"
        extension = path.suffix.lower().removeprefix(".")
        media_type = {"wrl": "model/vrml", "stl": "model/stl"}.get(extension, "model/step")
        token = f"{stable_model_token(relative):016x}"
        if token in tokens:
            raise ValueError(f"model source-path token collision: {relative}")
        tokens.add(token)
        emitted_name = f"model-{token}.{extension}"
        emitted_path = f"assets/ergogen-models/{emitted_name}"
        if asset_id in ids or emitted_path in paths:
            raise ValueError(f"duplicate model identity or static path: {relative}")
        ids.add(asset_id)
        paths.add(emitted_path)
        entries.append({
            "id": asset_id,
            "filename": filename,
            "mediaType": media_type,
            "sourceRelativePath": relative,
            "emittedPath": emitted_path,
        })
    return entries


def stage_models(source_root: Path, destination: Path, manifest_path: Path) -> list[dict[str, str]]:
    source_root = source_root.resolve()
    destination = destination.resolve()
    entries = catalogue(source_root)
    destination.mkdir(parents=True, exist_ok=True)

    # This directory is reserved for generated provider files. Never remove an
    # unrecognized entry, so hand-added assets cannot be silently discarded.
    expected_names = {Path(entry["emittedPath"]).name for entry in entries}
    for existing in destination.iterdir():
        if not re.fullmatch(r"model-[0-9a-f]{16}\.(step|stp|wrl|stl)", existing.name):
            raise ValueError(f"unexpected file in generated model directory: {existing}")
        if existing.name not in expected_names:
            if existing.is_dir() and not existing.is_symlink():
                raise ValueError(f"unexpected directory in generated model directory: {existing}")
            existing.unlink()

    for entry in entries:
        source = source_root / entry["sourceRelativePath"]
        output = destination / Path(entry["emittedPath"]).name
        if output.exists() or output.is_symlink():
            if output.is_symlink():
                if output.resolve() != source.resolve():
                    raise ValueError(f"staged model path conflicts with its catalogue source: {output}")
            elif output.read_bytes() != source.read_bytes():
                raise ValueError(f"staged model copy differs from its catalogue source: {output}")
            continue
        try:
            output.symlink_to(source)
        except OSError:
            # Static builds subsequently copy these into self-contained output.
            # The copy fallback also supports hosts that disallow symlink creation.
            import shutil
            shutil.copyfile(source, output)

    manifest = {
        "sourceRoot": str(source_root),
        "destination": str(destination),
        "entries": entries,
    }
    manifest_path.parent.mkdir(parents=True, exist_ok=True)
    manifest_path.write_text(json.dumps(manifest, indent=2) + "\n")
    return entries


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, required=True)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    args = parser.parse_args()
    entries = stage_models(args.source_root, args.destination, args.manifest)
    print(f"staged {len(entries)} Ergogen model files at {args.destination}")


if __name__ == "__main__":
    main()
