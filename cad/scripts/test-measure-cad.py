#!/usr/bin/env python3
"""Untimed protocol checks: no compiler, provider, container or browser execution."""

import csv
import importlib.util
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("measure_cad", Path(__file__).with_name("measure-cad.py"))
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)


class PerformancePreparationTests(unittest.TestCase):
    def test_initial_build_cycle_is_single_and_retained_upper_plan_is_explicit(self):
        schedule = PERF.schedule()
        builds = [row for row in schedule if row["workload"] == "build"]
        self.assertEqual(len(builds), 12)
        self.assertEqual(len([row for row in PERF.schedule(3) if row["workload"] == "build"]), 36)
        for profile in PERF.PROFILES:
            for workload in PERF.PAIRED:
                rows = [row for row in schedule if row["profile"] == profile
                        and row["workload"] == workload and row["pair"] > 0]
                self.assertEqual([row["backend"] for row in rows],
                                 ["cadrum", "monstertruck", "monstertruck", "cadrum",
                                  "cadrum", "monstertruck", "monstertruck", "cadrum", "cadrum", "monstertruck"])
        self.assertFalse(any(row["backend"] == "cadrum" and row["workload"] in PERF.UNPAIRED for row in schedule))

    def test_existing_artifact_receipt_rejects_changed_worker_and_changed_production(self):
        evidence = PERF.ROOT / ".evidence/011-preparation"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as temporary:
            root = Path(temporary)
            site = root / "web/target/site/site-subpath/boardstudio"
            site.mkdir(parents=True)
            (site / "index.html").write_text("controlled fixture")
            pairs = [("cad/boardstudio_cad_wasm_bg.wasm", "cad/wasm/pkg/boardstudio_cad_wasm_bg.wasm"),
                     ("cad/boardstudio_cad_wasm.js", "cad/wasm/pkg/boardstudio_cad_wasm.js"),
                     ("cad-worker/m1_cad_worker_bg.wasm", "web/target/providers/cad-worker/m1_cad_worker_bg.wasm"),
                     ("cad-worker/m1_cad_worker.js", "web/target/providers/cad-worker/m1_cad_worker.js")]
            text = "Current combined source " + "1"*40 + "; checked --no-opt; Dioxus page debug\n"
            for packaged, original in pairs:
                for file in (site / "assets" / packaged, root / original):
                    file.parent.mkdir(parents=True, exist_ok=True)
                    file.write_bytes(b"controlled-artifact")
                text += f"BYTE_IDENTICAL packaged assets/{packaged} == {original}; 19 bytes.\n"
            receipt = root / "receipt.log"
            receipt.write_text(text)
            with patch.object(PERF, "git", return_value=""):
                self.assertIn("validation_only=true", PERF.existing_artifacts(root, receipt, "checked"))
                (site / "assets/cad-worker/m1_cad_worker.js").write_bytes(b"different-artifact!")
                with self.assertRaisesRegex(ValueError, "artifact differs"):
                    PERF.existing_artifacts(root, receipt, "checked")
            with patch.object(PERF, "git", return_value="web/crates/host/src/cad_worker.rs"):
                with self.assertRaisesRegex(ValueError, "production changes"):
                    PERF.existing_artifacts(root, receipt, "checked")

    def test_diagnostic_validation_is_never_a_benchmark_sample(self):
        evidence = PERF.ROOT / ".evidence/011-preparation"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as temporary:
            root = Path(temporary)
            (root / "diagnostic.csv").write_text("diagnostic-not-benchmark\n")
            with self.assertRaisesRegex(ValueError, "cannot become benchmark"):
                PERF.summarize([root], root)

    def test_frozen_inputs_preserve_assembly_and_prepared_fixture_identity(self):
        for name in ("C1plate", "C2trayboss", "C3gasket", "P1analyticPCB"):
            assembly = json.loads((PERF.FIXTURES / f"{name}.json").read_text())
            self.assertTrue(assembly["stepOnly"])
            self.assertTrue(all(body["revision"] == assembly["revision"] for body in assembly["bodies"]))
        for name, original in (("C3gasket", "gasketed-pair.prepared.json"), ("P1analyticPCB", "pcb-simple.json")):
            frozen = json.loads((PERF.FIXTURES / f"{name}.json").read_text())
            authored = json.loads((PERF.ROOT / "cad/monstertruck/tests/fixtures/layered" / original).read_text())
            self.assertEqual(frozen["bodies"], authored["bodies"] if "bodies" in authored else [authored])
        caps = json.loads((PERF.FIXTURES / "K1keycap.json").read_text())["specs"]
        self.assertEqual(len(caps), 25)
        self.assertTrue(all(spec["dishDepth"] == .65 and spec["legend"] == "O" for spec in caps))

    def test_report_pairs_session_medians_and_retains_failure(self):
        evidence = PERF.ROOT / ".evidence/011-preparation"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as temporary:
            root = Path(temporary)
            inputs = []
            for backend, values in (("cadrum", [10, 11, 12, 13, 100]), ("monstertruck", [20, 21, 24, 25, 26])):
                directory = root / backend
                directory.mkdir()
                inputs.append(directory)
                (directory / "command.log").write_text("command\nexit=0\n")
                for name in ("binding-before.log", "binding-after.log"):
                    (directory / name).write_text(f"head={backend}\nsource_sha256={backend}\ndriver_head=same-driver\nworkload_blob_C1plate.json=same-request\n")
                fields = ("backend", "profile", "workload", "pair", "state", "ordinal", "metric", "value", "unit", "status")
                PERF.write_csv(directory / "runtime.csv", fields, [
                    dict(backend=backend, profile="checked", workload="C1plate:preview", pair=1,
                         state="warm", ordinal=i, metric="latency", value=value, unit="ms", status="observed")
                    for i, value in enumerate(values)])
            output = root / "report"
            output.mkdir()
            PERF.summarize(inputs, output)
            with (output / "pairs.csv").open() as source:
                row, = list(csv.DictReader(source))
            self.assertEqual(float(row["new_over_old"]), 2)
            self.assertEqual(float(row["new_minus_old"]), 12)
            with self.assertRaisesRegex(ValueError, "duplicate observation"):
                PERF.summarize([inputs[0], inputs[0]], output)
            for name in ("binding-before.log", "binding-after.log"):
                file = inputs[1] / name
                file.write_text(file.read_text().replace("driver_head=same-driver", "driver_head=other-driver"))
            with self.assertRaisesRegex(ValueError, "different harness/workload"):
                PERF.summarize(inputs, output)
            (inputs[1] / "command.log").write_text("command\nexit=1\n")
            with self.assertRaisesRegex(ValueError, "failed observations"):
                PERF.summarize(inputs, output)
            self.assertIn("monstertruck", (output / "failures.log").read_text())


if __name__ == "__main__":
    unittest.main()
