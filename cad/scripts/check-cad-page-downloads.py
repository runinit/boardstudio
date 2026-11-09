#!/usr/bin/env python3
"""Untimed U1 archive/STEP delivery QA; optional installed FreeCAD is a one-time independent reader."""

import argparse
import hashlib
import importlib.util
import json
from pathlib import Path
import re
import subprocess
import sys
import zipfile

ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("measure_cad", ROOT / "cad/scripts/measure-cad.py")
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)


def unpack(archive, destination):
    with zipfile.ZipFile(archive) as source:
        names = source.namelist()
        assert len(set(names)) == len(names), "duplicate archive paths"
        for name in names:
            path = Path(name)
            assert not path.is_absolute() and ".." not in path.parts, "unsafe archive path"
            target = destination / path
            target.parent.mkdir(parents=True, exist_ok=True)
            target.write_bytes(source.read(name))  # ZipFile verifies CRC and complete payload.
    return names


def validate(directory, build_run, freecad=False):
    assert directory.is_relative_to(ROOT / ".evidence"), "QA outputs belong to this lane's evidence"
    # Page files precede teardown; only the launcher's complete, bound capture can be admitted.
    events = [json.loads(line) for line in (directory / "events.log").read_text().splitlines()]
    if not events or events[-1].get("name") != "result" or events[-1].get("failed") is not False:
        raise ValueError("capture has no successful terminal result")
    validation_log = directory / "validation.log"
    terminal = validation_log.read_text() if validation_log.exists() else (directory / "command.log").read_text()
    pattern = r"(?m)^validation_exit=0;" if validation_log.exists() else r"(?m)^exit=0$"
    if not re.search(pattern, terminal):
        raise ValueError("capture launcher did not finish successfully")
    before = (directory / "binding-before.log").read_text()
    after = (directory / "binding-after.log").read_text()
    if before != after or not all(any(line.startswith(key) and line != key and not line.endswith("=MISSING")
                                      for line in before.splitlines()) for key in
                                  ("head=", "source_sha256=", "provider_stamp=",
                                   "artifact_cad/wasm/pkg=", "artifact_web/target/site/site-subpath/boardstudio=")):
        raise ValueError("missing or changed capture source/artifact binding")
    PERF.validate_page_build_receipt(directory, build_run)
    receipt = directory / "page-validation.log"
    if not receipt.exists():
        receipt = directory / "page-diagnostic.log"
    page = json.loads(receipt.read_text())
    if page.get("validationOnly") is not True:
        raise ValueError("missing U1 page validation")
    imported = any(phase.get("name") == "frozen-project-import" for phase in page.get("phases", []))
    project = ROOT / "cad/test/fixtures/performance/U1.boardstudio" if imported else directory / "U1.boardstudio"
    accepted = json.loads((directory / "accepted-project.json").read_text())
    if imported:
        PERF.validate_u1_scope(accepted)
    workers = json.loads((directory / "page-workers.log").read_text())
    with zipfile.ZipFile(project) as source:
        archived = json.loads(source.read("project.json"))
        assert {k: v for k, v in archived.items() if k != "assets"} == {k: v for k, v in accepted.items() if k != "assets"}
        for asset in archived["assets"]:
            assert hashlib.sha256(source.read(f"assets/{asset['sha256']}")).hexdigest() == asset["sha256"]
        assert json.loads(source.read("archive.json"))["embedUsedModels"] is True
    mechanical_root, keycap_root = directory / "downloads/mechanical", directory / "downloads/keycaps"
    mechanical = unpack(directory / "mechanical.zip", mechanical_root)
    keycaps = unpack(directory / "keycaps.zip", keycap_root)
    assembly = json.loads((mechanical_root / "assembly.json").read_text())
    assert assembly["revision"] == accepted["revision"]
    assert assembly["pcbReference"] == "Nominal unpopulated PCB only; component solids are not included"
    assert assembly["pcbSource"] is None  # Frozen U1 is the common analytic PCB, not the source-PCB proof fixture.
    assert {row["id"] for row in assembly["stack"]} == {"plate", "plate-foam", "pcb", "bottom-foam", "bottom"}
    expected = next(request for request in workers["requests"] if request["operation"] == "exact"
                    and len(request.get("prepared", {}).get("bodies", [])) == 5)["prepared"]
    assert {body["body"]["id"] for body in expected["bodies"]} == {"plate", "bottom", "plate-foam", "bottom-foam", "pcb-reference"}
    steps = [name for name in mechanical if name.endswith(".step")]
    assert len(steps) == 5 and "assembly.step" in steps
    raw = [bytes(step["bytes"]) for step in workers["steps"]]
    for name in steps:
        data = (mechanical_root / name).read_bytes()
        assert data.startswith(b"ISO-10303-21;") and data.rstrip().endswith(b"END-ISO-10303-21;")
        assert data in raw, f"downloaded {name} must equal an actual completed worker STEP buffer"
        assert b".MILLI.,.METRE." in data, "explicit millimetre STEP units"
    manifest = json.loads((keycap_root / "keycaps.json").read_text())
    assert set(keycaps) == {"keycaps.json", *(cap["file"] for cap in manifest["keycaps"])}, "complete STL/manifest ZIP inventory"
    with (directory / "stl-oracle.log").open("w") as log:
        subprocess.run(["node", str(ROOT / "cad/scripts/check-cad-page-downloads.cjs"), str(directory)],
                       check=True, stdout=log, stderr=subprocess.STDOUT, timeout=120)
    reader = {"state": "not requested", "scope": "prior unchanged 008 source-accuracy proofs remain separate"}
    if freecad:
        import FreeCAD
        import Part
        rows = []
        for name in steps:
            shape = Part.read(str(mechanical_root / name))
            assert shape.isValid() and shape.isClosed() and shape.Volume > 0
            assert len(shape.Solids) == (5 if name == "assembly.step" else 1), "complete expected solid inventory"
            rows.append({"file": name, "solids": len(shape.Solids), "volumeMm3": shape.Volume,
                         "faces": len(shape.Faces), "valid": True, "closed": True})
        # The fifth assembly solid is the physical PCB; retain the authored 1.6mm span.
        shape = Part.read(str(mechanical_root / "assembly.step"))
        pcb = expected["bodies"][-1]["body"]
        assert pcb["id"] == "pcb-reference"
        matches = [solid for solid in shape.Solids if abs(solid.BoundBox.ZMin - pcb["z"]) <= 1e-6
                   and abs(solid.BoundBox.ZLength - pcb["thickness"]) <= 1e-6]
        assert len(matches) == 1, "physical PCB z/thickness, retained exact-output 1e-6mm coordinate QA gate"
        reader = {"state": "passed", "reader": "installed independent FreeCAD Part.read", "version": FreeCAD.Version(),
                  "occt": Part.OCC_VERSION, "rows": rows,
                  "scope": "one-time emitted-file inventory/validity/PCB span QA; existing 008 independent source-accuracy gates are reused, not replaced"}
    result = {"validationOnly": True, "passed": True, "revision": accepted["revision"], "STEP": steps,
              "STLs": [cap["file"] for cap in manifest["keycaps"]], "embeddedAssets": len(archived["assets"]),
              "reader": reader, "diagnostics": assembly["diagnostics"], "values": "diagnostic-not-benchmark"}
    (directory / "downloads-check.log").write_text(json.dumps(result, indent=2) + "\n")
    print("PASS: ordinary project/asset bytes, real STEP delivery, full Runtime STL/manifest/ZIP and independent output QA")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path)
    parser.add_argument("--build-run", type=Path, required=True, help="successful source-bound measure-cad build capture")
    parser.add_argument("--freecad", action="store_true", help="use the already installed independent reader, never provision it")
    args = parser.parse_args()
    validate(args.capture.resolve(), args.build_run.resolve(), args.freecad)
