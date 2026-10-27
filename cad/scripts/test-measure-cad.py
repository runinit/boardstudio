#!/usr/bin/env python3
"""Untimed protocol checks: no compiler, provider, container or browser execution."""

import csv
import difflib
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SPEC = importlib.util.spec_from_file_location("measure_cad", Path(__file__).with_name("measure-cad.py"))
PERF = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(PERF)


class CaptureSelectionTests(unittest.TestCase):
    def test_rust_runtime_rejects_diagnostic_only_workloads_at_launcher(self):
        evidence = PERF.ROOT / ".evidence/011-prerequisite-test"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as directory:
            for workload, reason in (("startup:cold", "timing"), ("U1:page", "samples")):
                for action, driver in (("runtime", "rust"), ("validate", "rust"), ("runtime", "node")):
                    with self.subTest(workload=workload, action=action, driver=driver):
                        args = ["measure-cad.py", action, "--driver", driver, "--workload", workload,
                                "--output", str(Path(directory) / f"{workload.replace(':', '-')}-{action}-{driver}")]
                        with patch.object(sys, "argv", args), patch.object(PERF, "binding", side_effect=RuntimeError("binding reached")), \
                             patch("sys.stderr", new_callable=io.StringIO) as stderr:
                            if (action, driver) == ("runtime", "rust"):
                                with self.assertRaises(SystemExit):
                                    PERF.main()
                                self.assertIn(reason, stderr.getvalue())
                                self.assertIn("validate", stderr.getvalue())
                            else:
                                with self.assertRaisesRegex(RuntimeError, "binding reached"):
                                    PERF.main()

    def test_rust_selection_accepts_worker_workloads_only(self):
        self.assertEqual(PERF.capture_driver("node", "C1plate:preview"), "node")
        for workload in (*PERF.PAIRED, *PERF.UNPAIRED):
            self.assertEqual(PERF.capture_driver("rust", workload), "rust")
        self.assertEqual(PERF.capture_driver("rust", "startup:cold"), "rust")
        for workload in ("U1:freeze", "U1:page"):
            self.assertEqual(PERF.capture_driver("rust", workload), "rust")
        with self.assertRaisesRegex(ValueError, "does not support"):
            PERF.capture_driver("rust", "startup:warm")

    def test_selected_driver_controls_build_and_validation_prerequisites(self):
        evidence = PERF.ROOT / ".evidence/011-prerequisite-test"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as directory:
            root = Path(directory) / "checkout"
            worker = root / "web/target/site/site-subpath/boardstudio/assets/cad-worker/entry.js"
            worker.parent.mkdir(parents=True)
            worker.touch()
            receipt = Path(directory) / "receipt"
            receipt.touch()
            for action in ("build", "validate"):
                for driver in ("rust", "node"):
                    with self.subTest(action=action, driver=driver):
                        out = Path(directory) / f"{action}-{driver}"
                        args = ["measure-cad.py", action, "--checkout", str(root), "--source-head", "head",
                                "--driver", driver, "--output", str(out)]
                        args += (["--state", "noop"] if action == "build" else
                                 ["--workload", "C1plate:preview", "--artifact-receipt", str(receipt)])
                        with patch.object(sys, "argv", args), patch.object(PERF, "git", return_value="head"), \
                             patch.object(PERF, "binding", return_value=([], True)), \
                             patch.object(PERF, "existing_artifacts", return_value="fixture"), \
                             patch.object(PERF.browser_tools, "locate", side_effect=PERF.browser_tools.MissingBrowserTool("playwright-core unavailable")), \
                             patch.object(PERF.browser_tools, "locate_chrome", return_value="/chrome"), \
                             patch.object(PERF.browser_tools, "_build_harness", return_value=Path("/capture")), \
                             patch.object(PERF, "capture", return_value=(1, 0)), \
                             patch.object(PERF, "sizes"), patch.object(PERF.subprocess, "Popen") as process:
                            process.return_value.wait.return_value = 0
                            if driver == "node":
                                with self.assertRaisesRegex(PERF.browser_tools.MissingBrowserTool, "playwright-core"):
                                    PERF.main()
                            else:
                                self.assertEqual(PERF.main(), 1 if action == "build" else 0)


class CaptureLockTests(unittest.TestCase):
    def test_capture_inherits_build_lock_descriptor(self):
        with tempfile.TemporaryDirectory() as directory, tempfile.TemporaryFile() as lock:
            with patch.object(PERF.browser_tools, "_inherited_build_lock", return_value=lock.fileno()):
                code, _ = PERF.capture([sys.executable, "-c", f"import os; os.fstat({lock.fileno()})"],
                                       PERF.ROOT, Path(directory), 10)
            self.assertEqual(code, 0, Path(directory, "command.log").read_text())


class BuildBindingTests(unittest.TestCase):
    HEAD = "2ea2ca48d1b0a55e99429fe06c7bdec0611a428b"
    COMMENT = "// BoardStudio CAD performance incremental rebuild probe.\n"
    SOURCE = "// Controlled incremental source fixture, not historical provider evidence.\npub fn initialize_cadrum() {}\n"

    def setUp(self):
        evidence = PERF.ROOT / ".evidence/011-build-binding-fix"
        evidence.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=evidence)
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        # Both fixture generation and validation read this source, even in shallow CI checkouts.
        def source(command, **kwargs):
            self.assertIn(command, [prefix + ["show", f"{head}:cad/wasm/src/lib.rs"]
                                   for prefix in (["git"], ["git", "-C", str(PERF.ROOT)])
                                   for head in (self.HEAD, PERF.OLD_HEAD)])
            self.assertTrue(kwargs["text"])
            return self.SOURCE

        self.enterContext(patch.object(PERF.subprocess, "check_output", side_effect=source))

    def bound(self, backend="monstertruck", state="clean", built=False, **changes):
        head = PERF.OLD_HEAD if backend == "cadrum" else self.HEAD
        diff = ""
        if state == "incremental":
            path = "cad/wasm/src/lib.rs"
            base = PERF.git(PERF.ROOT, "show", f"{head}:{path}") + "\n"
            diff = (f"diff --git a/{path} b/{path}\nindex 1234567..abcdef0 100644\n" +
                    "\n".join(difflib.unified_diff(base.splitlines(), (base + self.COMMENT).splitlines(),
                                                  fromfile=f"a/{path}", tofile=f"b/{path}", lineterm="")))
        fields = dict(backend=backend, head=head,
                      source_sha256=hashlib.sha256((diff + "\n" if diff else "").encode()).hexdigest(),
                      status=repr(" M cad/wasm/src/lib.rs" if diff else ""),
                      profile="checked", provider_profile="release-no-opt",
                      driver_head=self.HEAD, driver_source_sha256="f" * 64, driver_status=repr(""),
                      official_provider_inputs="b" * 64,
                      provider_stamp="b" * 64 if built else "MISSING")
        fields["artifact_cad/wasm/pkg"] = "c" * 64 if built else "MISSING"
        fields["artifact_web/target/site/site-subpath/boardstudio"] = "d" * 64 if built else "MISSING"
        fields["resolved_target"] = f"/controlled/target exists={built}"
        for path in sorted(PERF.FIXTURES.iterdir()):
            if path.suffix in (".json", ".boardstudio"):
                fields[f"workload_blob_{path.name}"] = "e" * 40
        fields.update(changes)
        return "\n".join(f"{key}={value}" for key, value in fields.items()) + "\nowned_diff:\n" + diff + "\n"

    def sample(self, name, backend="monstertruck", state="clean", pair=1, before=None, after=None, **row_changes):
        directory = self.root / name
        directory.mkdir()
        (directory / "command.log").write_text("controlled subprocess fixture; not benchmark data\nexit=0\n")
        (directory / "binding-before.log").write_text(before or self.bound(backend, state))
        (directory / "binding-after.log").write_text(after or self.bound(backend, state, built=True))
        row = dict(backend=backend, profile="checked", pair=pair, state=state, seconds=1, status="ok")
        row.update(row_changes)
        PERF.write_csv(directory / "build.csv", tuple(row), [row])
        return directory

    def reduce(self, inputs):
        output = self.root / "report"
        output.mkdir(exist_ok=True)
        PERF.summarize(inputs, output)
        return output

    def test_page_capture_requires_successful_matching_build_receipt(self):
        build = self.sample("page-build", state="noop")
        capture = self.root / "page-capture"
        capture.mkdir()
        bound = (build / "binding-after.log").read_text()
        (capture / "binding-before.log").write_text(bound)
        (capture / "binding-after.log").write_text(bound)
        PERF.validate_page_build_receipt(capture, build)
        (capture / "binding-after.log").write_text(bound.replace("head=", "head=wrong"))
        with self.assertRaisesRegex(ValueError, "drift"):
            PERF.validate_page_build_receipt(capture, build)
        (capture / "binding-after.log").write_text(bound)
        (build / "build.csv").write_text("backend,profile,state,status\nmonstertruck,checked,noop,binding-rejected\n")
        with self.assertRaisesRegex(ValueError, "build receipt"):
            PERF.validate_page_build_receipt(capture, build)

    def test_checked_receipt_requires_matching_successful_build_capture(self):
        build = self.sample("checked-build", state="noop")
        bound = (build / "binding-after.log").read_text()
        receipt = self.root / "receipt.json"
        record = {"schema_version": 1, "generated_by": "scripts/check.py",
                  "source": {"revision": self.HEAD, "dirty": False, "local_changes": 0},
                  "invocation": {"accepted": True, "exit_code": 0, "steps": ["build"],
                                 "planned_commands": 1, "ran_commands": 1, "not_run_commands": []},
                  "commands": [{"step": "build", "command": ["python3", "scripts/build-web.py", "--fast"],
                                "status": "ran", "passed": True, "exit_code": 0}]}
        receipt.write_text(json.dumps(record))
        PERF.existing_artifacts(self.root, receipt, "checked", build, bound)
        with self.assertRaisesRegex(ValueError, "match"):
            PERF.existing_artifacts(self.root, receipt, "checked", build, bound.replace("head=", "head=wrong"))
        record["commands"][0]["passed"] = False
        receipt.write_text(json.dumps(record))
        with self.assertRaisesRegex(ValueError, "successful"):
            PERF.existing_artifacts(self.root, receipt, "checked", build, bound)

    def test_u1_scope_rejects_identity_drift(self):
        import zipfile
        with zipfile.ZipFile(PERF.FIXTURES / "U1.boardstudio") as archive:
            project = json.loads(archive.read("project.json"))
        PERF.validate_u1_scope(project)
        for key in ("id", "formatVersion", "revision"):
            changed = dict(project, **{key: "wrong"})
            with self.subTest(key=key), self.assertRaisesRegex(ValueError, "U1 scope"):
                PERF.validate_u1_scope(changed)
        changed = json.loads(json.dumps(project))
        changed["boards"][0]["id"] = "wrong"
        with self.assertRaisesRegex(ValueError, "U1 scope"):
            PERF.validate_u1_scope(changed)
        changed = json.loads(json.dumps(project))
        changed["boards"][0]["outlineIds"] = ["wrong"]
        with self.assertRaisesRegex(ValueError, "U1 scope"):
            PERF.validate_u1_scope(changed)

    def test_reduction_accepts_generated_outputs_and_exact_incremental_transition(self):
        inputs = [self.sample(f"{backend}-{state}", backend, state)
                  for backend in ("cadrum", "monstertruck") for state in ("clean", "noop", "incremental")]
        output = self.reduce(inputs)
        with (output / "pairs.csv").open() as source:
            self.assertEqual(len(list(csv.DictReader(source))), 3)

    def test_incremental_fixture_generation_needs_no_git_history(self):
        for backend in ("cadrum", "monstertruck"):
            with self.subTest(backend=backend), patch.object(
                PERF.subprocess, "run", side_effect=subprocess.CalledProcessError(
                    128, ["git", "show"], stderr="fatal: historical object unavailable in shallow checkout")
            ):
                self.bound(backend, state="incremental")

    def test_incremental_validation_needs_no_git_history(self):
        for backend in ("cadrum", "monstertruck"):
            bound = self.bound(backend, state="incremental", built=True)
            with self.subTest(backend=backend), patch.object(
                PERF.subprocess, "run", side_effect=subprocess.CalledProcessError(
                    128, ["git", "show"], stderr="fatal: historical object unavailable in shallow checkout")
            ):
                PERF.validate_build_bindings(bound, bound, backend, "checked", "incremental")

    def test_existing_output_changes_and_git_function_label_are_allowed(self):
        before = self.bound(built=True, provider_stamp="0" * 64,
                            **{"artifact_cad/wasm/pkg": "1" * 64,
                               "artifact_web/target/site/site-subpath/boardstudio": "2" * 64})
        self.reduce([self.sample("rebuilt", before=before, after=self.bound(built=True))])
        for profile in PERF.PROFILES:
            bound = self.bound(state="incremental", built=True, profile=profile,
                               provider_profile="release-no-opt" if profile == "checked" else "release")
            lines = bound.splitlines()
            hunk = next(i for i, line in enumerate(lines) if line.startswith("@@"))
            lines[hunk] += " pub fn initialize_cadrum() {"
            diff = "\n".join(lines).split("\nowned_diff:\n", 1)[1] + "\n"
            old_hash = hashlib.sha256(bound.split("\nowned_diff:\n", 1)[1].encode()).hexdigest()
            new_hash = hashlib.sha256(diff.encode()).hexdigest()
            bound = "\n".join(lines).replace("source_sha256=" + old_hash, "source_sha256=" + new_hash) + "\n"
            row = self.sample(profile, state="incremental", before=bound, after=bound, profile=profile)
            self.reduce([row])

    def test_reduction_rejects_missing_build_bindings(self):
        for name in ("binding-before.log", "binding-after.log"):
            with self.subTest(name=name):
                directory = self.sample(name)
                (directory / name).unlink()
                with self.assertRaisesRegex(ValueError, "binding"):
                    self.reduce([directory])

    def test_reduction_rejects_incomplete_or_drifted_bindings(self):
        before = self.bound()
        after = self.bound(built=True)
        cases = {
            "source-drift": after.replace(f"head={self.HEAD}\n", "head=" + "1" * 40 + "\n", 1),
            "patch-drift": self.bound(state="incremental", built=True),
            "harness-drift": after.replace("driver_source_sha256=" + "f" * 64, "driver_source_sha256=" + "0" * 64),
            "workload-drift": after.replace("workload_blob_C1plate.json=" + "e" * 40, "workload_blob_C1plate.json=" + "0" * 40),
            "stale-stamp": after.replace("provider_stamp=" + "b" * 64, "provider_stamp=MISSING"),
            "missing-artifact": after.replace("artifact_cad/wasm/pkg=" + "c" * 64, "artifact_cad/wasm/pkg=MISSING"),
            "missing-source": after.replace("source_sha256=" + hashlib.sha256(b"").hexdigest() + "\n", ""),
            "missing-harness": after.replace(f"driver_head={self.HEAD}\n", ""),
            "missing-workload": after.replace("workload_blob_C1plate.json=" + "e" * 40 + "\n", ""),
            "missing-owned-patch": after.split("owned_diff:")[0],
            "missing-profile": after.replace("provider_profile=release-no-opt\n", ""),
            "invalid-driver": after.replace(f"driver_head={self.HEAD}", "driver_head=MISSING"),
            "target-drift": after.replace("/controlled/target", "/other/target"),
        }
        for name, changed in cases.items():
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, "binding|drift|stamp|artifact"):
                    self.reduce([self.sample(name, before=before, after=changed)])

    def test_reduction_rejects_row_profile_backend_and_arbitrary_patch(self):
        for name, changes in (("backend", dict(backend="cadrum")), ("profile", dict(profile="release")),
                              ("state", dict(state="incremental"))):
            with self.subTest(name=name):
                row = self.sample(name, before=self.bound(), after=self.bound(built=True), **changes)
                with self.assertRaisesRegex(ValueError, "binding|patch"):
                    self.reduce([row])
        for name, bound in (("unowned", self.bound(source_sha256="a" * 64, status=repr(" M other.rs"))),
                            ("other-comment", self.bound(state="incremental").replace(self.COMMENT.strip(), "// arbitrary edit"))):
            with self.subTest(name=name):
                with self.assertRaisesRegex(ValueError, "binding|patch"):
                    self.reduce([self.sample(name, state="incremental", before=bound, after=bound)])

    def test_reduction_rejects_mixed_source_and_harness_across_cycle(self):
        first = self.sample("first")
        cases = {
            "source": dict(head="1" * 40),
            "harness": dict(driver_head="2" * 40),
            "workload": {"workload_blob_C1plate.json": "3" * 40},
        }
        for name, changes in cases.items():
            with self.subTest(name=name):
                second = self.sample(name, state="noop", before=self.bound(**changes),
                                     after=self.bound(built=True, **changes))
                with self.assertRaisesRegex(ValueError, "mixed|different harness/workload"):
                    self.reduce([first, second])
        other = self.sample("other-backend", "cadrum", before=self.bound("cadrum", driver_head="4" * 40),
                            after=self.bound("cadrum", built=True, driver_head="4" * 40))
        with self.assertRaisesRegex(ValueError, "different harness/workload"):
            self.reduce([first, other])
        different_inputs = self.sample("different-inputs", state="noop",
                                       before=self.bound(official_provider_inputs="a" * 64),
                                       after=self.bound(built=True, official_provider_inputs="a" * 64,
                                                        provider_stamp="a" * 64))
        with self.assertRaisesRegex(ValueError, "mixed build patch/provider inputs"):
            self.reduce([first, different_inputs])

    def test_build_capture_checks_bindings_after_successful_controlled_subprocess(self):
        # Exercise main's real capture path without invoking a compiler or version probes.
        original_capture = PERF.capture

        def controlled_capture(command, root, out, timeout):
            return original_capture([sys.executable, "-c", "print('controlled subprocess, diagnostic only')"],
                                    root, out, 15)

        cases = {"outputs-only": self.bound(built=True),
                 "source": self.bound(built=True, head="1" * 40),
                 "harness": self.bound(built=True, driver_head="2" * 40),
                 "patch": self.bound(state="incremental", built=True),
                 "owned-incremental": self.bound(state="incremental", built=True)}
        checkout = self.root / "checkout"
        checkout.mkdir()
        for name, after in cases.items():
            with self.subTest(name=name):
                output = self.root / ("capture-" + name)
                state = "incremental" if name == "owned-incremental" else "clean"
                before = self.bound(state=state)
                argv = ["measure-cad.py", "build", "--checkout", str(checkout), "--source-head", self.HEAD,
                        "--state", state, "--output", str(output)]
                with patch.object(sys, "argv", argv), patch.object(PERF, "git", return_value=self.HEAD), \
                     patch.object(PERF, "binding", side_effect=[(before.splitlines(), False), (after.splitlines(), True)]), \
                     patch.object(PERF.browser_tools, "locate"), patch.object(PERF, "sizes"), \
                     patch.object(PERF, "environment", return_value="controlled environment; diagnostic only\n"), \
                     patch.object(PERF, "capture", side_effect=controlled_capture):
                    result = PERF.main()
                with (output / "build.csv").open() as source:
                    row, = list(csv.DictReader(source))
                self.assertIn("\nexit=0\n", (output / "command.log").read_text())
                accepted = name in ("outputs-only", "owned-incremental")
                self.assertEqual(row["status"] == "ok", accepted)
                self.assertEqual(result == 0, accepted)
                if not accepted:
                    self.assertIn("binding drift", (output / "binding-rejection.log").read_text())


class PerformancePreparationTests(unittest.TestCase):
    def test_default_schedule_is_monstertruck_only_with_one_session_and_one_build_cycle(self):
        schedule = PERF.schedule()
        self.assertEqual({row["backend"] for row in schedule}, {"monstertruck"})
        builds = [row for row in schedule if row["workload"] == "build"]
        self.assertEqual(len(builds), 6)
        self.assertEqual(len([row for row in PERF.schedule(3) if row["workload"] == "build"]), 18)
        for profile in PERF.PROFILES:
            for workload in (*PERF.PAIRED, *PERF.UNPAIRED):
                rows = [row for row in schedule if row["profile"] == profile
                        and row["workload"] == workload and row["pair"] > 0]
                self.assertEqual([row["pair"] for row in rows], [1])

    def test_historical_two_backend_schedule_remains_available_for_old_captures(self):
        schedule = PERF.schedule(1, 2, ("cadrum", "monstertruck"))
        rows = [row for row in schedule if row["profile"] == "checked"
                and row["workload"] == "C1plate:preview" and row["pair"] > 0]
        self.assertEqual([row["backend"] for row in rows],
                         ["cadrum", "monstertruck", "monstertruck", "cadrum"])
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

    def test_monstertruck_only_report_is_descriptive_without_pairs_or_baseline(self):
        evidence = PERF.ROOT / ".evidence/011-preparation"
        evidence.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=evidence) as temporary:
            root = Path(temporary)
            directory = root / "monstertruck"
            directory.mkdir()
            (directory / "command.log").write_text("command\nexit=0\n")
            for name in ("binding-before.log", "binding-after.log"):
                (directory / name).write_text("head=h\nsource_sha256=s\ndriver_head=d\nworkload_blob_C1plate.json=w\n")
            fields = ("backend", "profile", "workload", "pair", "state", "ordinal", "metric", "value", "unit", "status")
            PERF.write_csv(directory / "runtime.csv", fields, [
                dict(backend="monstertruck", profile="checked", workload="C1plate:preview", pair=1,
                     state="warm", ordinal=i, metric="dispatch-to-validated-reply", value=value, unit="ms",
                     status="observed") for i, value in enumerate([20, 21, 24, 25, 26])])
            output = root / "report"
            output.mkdir()
            PERF.summarize([directory], output)
            with (output / "pairs.csv").open() as source:
                self.assertEqual(list(csv.DictReader(source)), [])
            with (output / "summary.csv").open() as source:
                row, = list(csv.DictReader(source))
            self.assertEqual((row["backend"], float(row["median"])), ("monstertruck", 24.0))
            with (output / "coverage.csv").open() as source:
                self.assertTrue({r["backend"] for r in csv.DictReader(source)} == {"monstertruck"})

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
