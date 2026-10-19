#!/usr/bin/env python3
"""Tests for the opt-in verification receipt and its `check.py` integration.

Run: python3 -B scripts/test-verification-receipt.py
"""
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name: str, filename: str):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


check = _load("check", "check.py")
import verification_receipt as receipt  # noqa: E402  (scripts dir on sys.path after check.py)


# Synthetic whole-crate launch fixtures with literal independent names. They exercise
# collision groups (4 and 14) and explicit base fixtures (3 and 12); they are not run evidence.
CASE_CURRENT = """\
running 6 tests
test case_collision::browser_tests::mounted_source_band_and_contact_boxes_preserve_witness_uncertainty ... ok
test case_collision::browser_tests::mounted_collision_witness_keeps_operand_and_source_occurrence_identity ... ok
test case_collision::browser_tests::mounted_collision_check_is_explicit_and_retires_findings_on_source_change ... ok
test case_collision::browser_tests::mounted_collision_check_clears_pending_when_runtime_retires_the_scene ... ok
test case_assembly_layers::tests::physical_rows_keep_preview_order_and_exact_model_id_join ... ok
test shared_viewer::tests::viewer_events_reject_changed_owner_identity_and_unmount ... ok

test result: ok. 6 passed; 0 failed; 0 ignored; 83 filtered out; finished in 0.16s
"""

RUNTIME_CURRENT = """\
running 16 tests
test runtime::collision_browser_tests::source_pcb_collision_preserves_curves_drills_placement_and_complete_cache_identity ... ok
test runtime::collision_browser_tests::physical_pcb_preparation_cancellation_rejects_analysis_and_recovers ... ok
test runtime::collision_browser_tests::mechanical_collision_flow_analyzes_physical_pcb_and_reports_case_pair_provenance ... ok
test runtime::collision_browser_tests::generated_analysis_cache_isolates_requested_accuracy_and_rejects_stale_or_cancelled_work ... ok
test runtime::collision_browser_tests::collision_worker_observation_stays_uncertain_and_rejects_a_late_result ... ok
test runtime::collision_browser_tests::controlled_v2_reply_reaches_runtime_supported_band_with_complete_binding ... ok
test runtime::collision_browser_tests::missing_generated_lease_prepares_once_per_batch_joins_pending_and_second_miss_stays_unknown ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_shares_one_preparation_without_merging_placements ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_isolates_source_repair_and_target_settings ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_shares_failure_and_new_check_can_recover ... ok
test runtime::collision_browser_tests::cancelled_tight_vendor_refinement_cannot_settle_a_replacement_check ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_keeps_one_separate_bounded_evidence_recovery ... ok
test runtime::collision_browser_tests::complete_check_shares_one_vendor_refinement_across_repeated_placements ... ok
test runtime::collision_browser_tests::obsolete_false_view_entry_cannot_retire_an_active_complete_check ... ok
test archive_export::tests::session_option_defaults_on_and_notifies_only_when_changed ... ok
test runtime::tests::accepted_kiswitch_preview_path_resolves_to_the_known_packaged_descriptor ... ok

test result: ok. 16 passed; 0 failed; 0 ignored; 44 filtered out; finished in 3.22s
"""

CASE_BASE = """\
running 3 tests
test case_collision::browser_tests::mounted_source_band_and_contact_boxes_preserve_witness_uncertainty ... ok
test case_collision::browser_tests::mounted_collision_witness_keeps_operand_and_source_occurrence_identity ... ok
test case_collision::browser_tests::mounted_collision_check_is_explicit_and_retires_findings_on_source_change ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 83 filtered out; finished in 0.16s
"""

RUNTIME_BASE = """\
running 12 tests
test runtime::collision_browser_tests::source_pcb_collision_preserves_curves_drills_placement_and_complete_cache_identity ... ok
test runtime::collision_browser_tests::physical_pcb_preparation_cancellation_rejects_analysis_and_recovers ... ok
test runtime::collision_browser_tests::mechanical_collision_flow_analyzes_physical_pcb_and_reports_case_pair_provenance ... ok
test runtime::collision_browser_tests::generated_analysis_cache_isolates_requested_accuracy_and_rejects_stale_or_cancelled_work ... ok
test runtime::collision_browser_tests::collision_worker_observation_stays_uncertain_and_rejects_a_late_result ... ok
test runtime::collision_browser_tests::controlled_v2_reply_reaches_runtime_supported_band_with_complete_binding ... ok
test runtime::collision_browser_tests::missing_generated_lease_prepares_once_per_batch_joins_pending_and_second_miss_stays_unknown ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_shares_one_preparation_without_merging_placements ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_isolates_source_repair_and_target_settings ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_shares_failure_and_new_check_can_recover ... ok
test runtime::collision_browser_tests::cancelled_tight_vendor_refinement_cannot_settle_a_replacement_check ... ok
test runtime::collision_browser_tests::tight_vendor_refinement_keeps_one_separate_bounded_evidence_recovery ... ok

test result: ok. 12 passed; 0 failed; 0 ignored; 44 filtered out; finished in 3.22s
"""

EARLY_STOP = """\
running 2 tests
test case_collision::browser_tests::mounted_source_band_and_contact_boxes_preserve_witness_uncertainty ... ok
console output:
    Invoking test: case_collision::browser_tests::mounted_collision_witness_keeps_operand_and_source_occurrence_identity
driver status: signal: 9 (SIGKILL)
"""

MALFORMED = "warning: unused import\nchecking boardstudio-web-case v0.1.0\n"

NATIVE_CORE = """\
     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)

running 3 tests
test mirrored_half::reconcile::tests::alpha ... ok
test mirrored_half::reconcile::tests::beta ... FAILED
test mirrored_half::reconcile::tests::gamma ... ignored
"""

PAGE_REPORT = {
    "schema_version": 1,
    "expected_tests": ["presentation::x::tests::a", "presentation::x::tests::b"],
    "terminal_outcomes": [
        {"test": "presentation::x::tests::a", "status": "passed"},
        {"test": "presentation::x::tests::b", "status": "failed"},
    ],
    "passed_count": 1,
    "failed_count": 1,
    "incomplete_count": 0,
    "failed_known_exclusions": ["presentation::x::tests::b"],
    "problems": [],
    "complete": True,
}

CASE_COMMAND = ["wasm-pack", "test", "--headless", "--chrome", "web/crates/case", "--locked", "--lib"]
RUNTIME_COMMAND = ["wasm-pack", "test", "--headless", "--chrome", "web/crates/runtime", "--locked", "--lib"]
PAGE_COMMAND = [check.PY, "scripts/run-wasm-tests.py", "--all", "--result-json", "web/target/test-results/browser.json"]


def observation(step, command, output, *, exit_code=0, elapsed=1.0):
    return receipt.Observation(step, list(command), "ran", exit_code, elapsed, output)


class ReceiptBoundaryTests(unittest.TestCase):
    def test_wasm_crate_launches_keep_separate_full_names_and_group_counts(self):
        plan = [("browser", CASE_COMMAND), ("browser", RUNTIME_COMMAND)]
        observations = [
            observation("browser", CASE_COMMAND, CASE_CURRENT),
            observation("browser", RUNTIME_COMMAND, RUNTIME_CURRENT),
        ]
        built = receipt.build_receipt(plan, observations, source={"revision": "abc", "dirty": False},
                                      exit_code=0, partial=True)
        crates = {record["crate"]: record for record in built["commands"]}
        self.assertEqual(set(crates), {"case", "runtime"})
        case = crates["case"]
        self.assertTrue(case["complete"])
        self.assertEqual(case["counts"]["passed"], 6)
        self.assertEqual(len(case["tests"]), 6, "every observed name is retained, not a bounded sample")
        self.assertEqual(case["groups"]["case_collision::browser_tests"], 4)
        runtime = crates["runtime"]
        self.assertEqual(runtime["counts"]["passed"], 16)
        self.assertEqual(runtime["groups"]["runtime::collision_browser_tests"], 14)
        self.assertEqual(built["before_after"]["status"], "unknown")

    def test_native_outputs_keep_binary_counts_outcomes_and_retention_matches(self):
        baseline = {
            "boardstudio_core": [
                "mirrored_half::reconcile::tests::alpha",
                "mirrored_half::reconcile::tests::gamma",
                "mirrored_half::reconcile::tests::delta",
            ],
            "boardstudio_footprints": ["footprints::tests::one"],
        }
        record = receipt.native_record([NATIVE_CORE], full=False, baseline=baseline,
                                       provenance={"path": "check-native-tests-baseline.json",
                                                   "recording_revision": "deadbeef", "dirty": False})
        self.assertEqual([binary["name"] for binary in record["binaries"]], ["boardstudio_core"])
        self.assertEqual(record["counts"], {"passed": 1, "failed": 1, "ignored": 1, "incomplete": 0})
        outcomes = {test["name"]: test["outcome"] for test in record["binaries"][0]["tests"]}
        self.assertEqual(outcomes["mirrored_half::reconcile::tests::beta"], "failed")
        self.assertEqual(record["baseline"]["recording_revision"], "deadbeef")
        self.assertEqual(record["baseline"]["matched"], 1)
        self.assertEqual(record["baseline"]["missing"],
                         {"boardstudio_core": ["mirrored_half::reconcile::tests::delta",
                                               "mirrored_half::reconcile::tests::gamma"]})
        self.assertEqual(record["baseline"]["new"], ["boardstudio_core::mirrored_half::reconcile::tests::beta"])

    def test_absent_planned_command_is_not_run_and_claims_no_counts(self):
        plan = [("test", ["cargo", "test", "--workspace"]),
                ("test", ["cargo", "test", "-p", "boardstudio-footprints"])]
        built = receipt.build_receipt(plan, [observation("test", plan[0][1], NATIVE_CORE)],
                                      source={"revision": "abc", "dirty": False}, exit_code=0, partial=True)
        second = built["commands"][1]
        self.assertEqual(second["status"], "not_run")
        self.assertIsNone(second["exit_code"])
        self.assertEqual(built["invocation"]["not_run_commands"][0]["command"], plan[1][1])

    def test_page_suite_keeps_raw_failures_and_known_exclusions_separate(self):
        plan = [("browser", PAGE_COMMAND)]
        built = receipt.build_receipt(plan, [observation("browser", PAGE_COMMAND, "")],
                                      source={"revision": "abc", "dirty": False}, exit_code=0, partial=False,
                                      page_loader=lambda path: PAGE_REPORT)
        record = built["commands"][0]
        self.assertEqual(record["scope"], "page-suite")
        self.assertTrue(record["complete"])
        self.assertFalse(record["passed"], "a complete run with a failing test is not passing")
        self.assertEqual(record["known_exclusions"], ["presentation::x::tests::b"])
        self.assertEqual(record["raw_failures"], [], "the named exclusion is not a raw failure")
        self.assertTrue(record["fresh"])

    def test_unexpected_page_failure_is_raw_and_not_accepted_as_passing(self):
        report = dict(PAGE_REPORT, failed_known_exclusions=[],
                      terminal_outcomes=[{"test": "presentation::x::tests::b", "status": "failed"}])
        built = receipt.build_receipt([("browser", PAGE_COMMAND)],
                                      [observation("browser", PAGE_COMMAND, "", exit_code=1)],
                                      source={"revision": "abc", "dirty": False}, exit_code=1, partial=False,
                                      page_loader=lambda path: report)
        record = built["commands"][0]
        self.assertEqual(record["raw_failures"], ["presentation::x::tests::b"])
        self.assertFalse(record["passed"])
        self.assertIn("NOT accepted", receipt.render_summary(built))

    def test_partial_and_malformed_launches_are_incomplete_not_zero_failures(self):
        for label, output in (("early-stop", EARLY_STOP), ("malformed", MALFORMED)):
            with self.subTest(label=label):
                built = receipt.build_receipt([("browser", CASE_COMMAND)],
                                              [observation("browser", CASE_COMMAND, output, exit_code=1)],
                                              source={"revision": "abc", "dirty": False}, exit_code=1,
                                              partial=False)
                record = built["commands"][0]
                self.assertFalse(record["complete"], label)
                self.assertFalse(record["passed"], label)
                self.assertTrue(record["problems"], label)

    def test_invoked_test_without_terminal_result_is_incomplete(self):
        built = receipt.build_receipt([("browser", CASE_COMMAND)],
                                      [observation("browser", CASE_COMMAND, EARLY_STOP, exit_code=1)],
                                      source={"revision": "abc", "dirty": False}, exit_code=1, partial=False)
        record = built["commands"][0]
        self.assertEqual(record["counts"]["incomplete"], 1)
        self.assertIn("without a terminal result", " ".join(record["problems"]))

    def test_unreadable_page_json_is_incomplete_and_fresh_only(self):
        built = receipt.build_receipt([("browser", PAGE_COMMAND)], [observation("browser", PAGE_COMMAND, "")],
                                      source={"revision": "abc", "dirty": False}, exit_code=0, partial=False,
                                      page_loader=lambda path: None)
        record = built["commands"][0]
        self.assertFalse(record["complete"])
        self.assertTrue(record["problems"])

        def refuse(path):  # a cached file must never be read for a command that did not run
            raise AssertionError(f"the page result {path} was read for an unrun command")

        unrun = receipt.build_receipt([("browser", PAGE_COMMAND)], [],
                                      source={"revision": "abc", "dirty": False}, exit_code=1, partial=False,
                                      page_loader=refuse)
        self.assertEqual(unrun["commands"][0]["status"], "not_run")

    def test_group_deltas_need_explicit_base_evidence(self):
        current_plan = [("browser", CASE_COMMAND), ("browser", RUNTIME_COMMAND)]
        current = receipt.build_receipt(
            current_plan,
            [observation("browser", CASE_COMMAND, CASE_CURRENT),
             observation("browser", RUNTIME_COMMAND, RUNTIME_CURRENT)],
            source={"revision": "current", "dirty": False}, exit_code=0, partial=True)
        self.assertEqual(current["before_after"]["status"], "unknown")

        base_plan = [("browser", CASE_COMMAND), ("browser", RUNTIME_COMMAND)]
        base = receipt.build_receipt(
            base_plan,
            [observation("browser", CASE_COMMAND, CASE_BASE),
             observation("browser", RUNTIME_COMMAND, RUNTIME_BASE)],
            source={"revision": "base", "dirty": False}, exit_code=0, partial=True)
        compared = receipt.build_receipt(
            current_plan,
            [observation("browser", CASE_COMMAND, CASE_CURRENT),
             observation("browser", RUNTIME_COMMAND, RUNTIME_CURRENT)],
            source={"revision": "current", "dirty": False}, exit_code=0, partial=True,
            base=base, base_path=Path("base-receipt.json"))
        self.assertEqual(compared["before_after"]["status"], "compared")
        self.assertTrue(compared["before_after"]["reused"])
        deltas = {entry["crate"]: entry for entry in compared["before_after"]["groups"]}
        self.assertEqual(deltas["case"]["base"]["case_collision::browser_tests"], 3)
        self.assertEqual(deltas["case"]["delta"]["case_collision::browser_tests"], 1)
        self.assertEqual(deltas["runtime"]["base"]["runtime::collision_browser_tests"], 12)
        self.assertEqual(deltas["runtime"]["delta"]["runtime::collision_browser_tests"], 2)

    def test_retention_list_alone_leaves_the_comparison_unknown(self):
        baseline = {"boardstudio_core": ["mirrored_half::reconcile::tests::alpha"]}
        built = receipt.build_receipt(
            [("test", ["cargo", "test", "--workspace"])],
            [observation("test", ["cargo", "test", "--workspace"], NATIVE_CORE)],
            source={"revision": "abc", "dirty": False}, exit_code=1, partial=True,
            native=receipt.native_record([NATIVE_CORE], full=False, baseline=baseline,
                                         provenance={"path": "check-native-tests-baseline.json",
                                                     "recording_revision": "deadbeef", "dirty": False}))
        self.assertEqual(built["before_after"]["status"], "unknown")

    def test_summary_keeps_scopes_limitations_and_smoke_separate(self):
        smoke_command = [check.PY, "scripts/run-drag-browser.py"]
        plan = [("browser", CASE_COMMAND), ("browser", PAGE_COMMAND), ("browser", smoke_command)]
        built = receipt.build_receipt(
            plan,
            [observation("browser", CASE_COMMAND, CASE_CURRENT),
             observation("browser", PAGE_COMMAND, ""),
             observation("browser", smoke_command, "", exit_code=0)],
            source={"revision": "c0d14cd9", "branch": "dev", "dirty": True}, exit_code=0, partial=True,
            page_loader=lambda path: PAGE_REPORT)
        text = receipt.render_summary(built)
        self.assertIn("partial selection", text)
        self.assertIn("WASM crate launches", text)
        self.assertIn("Page suite", text)
        self.assertIn("separate evidence scope", text)
        self.assertIn("unknown", text)
        self.assertNotIn("all tests passed", text)


class CheckCliTests(unittest.TestCase):
    def setUp(self):
        locate = mock.patch.object(check.browser_tools, "locate", return_value=("node", "chrome"))
        self.locate = locate.start()
        self.addCleanup(locate.stop)
        for patcher in (
            mock.patch.object(check.verification_receipt, "source_identity",
                              return_value={"revision": "c0d14cd9", "branch": "dev", "dirty": False}),
            mock.patch.object(check.verification_receipt, "baseline_provenance",
                              return_value={"path": "scripts/check-native-tests-baseline.json",
                                            "recording_revision": "deadbeef", "dirty": False}),
        ):
            patcher.start()
            self.addCleanup(patcher.stop)
        self.temporary = tempfile.TemporaryDirectory(prefix="boardstudio-receipt-")
        self.addCleanup(self.temporary.cleanup)
        self.addCleanup(os.environ.pop, "BOARDSTUDIO_CHECK_JOBS", None)
        check._RECEIPT = None
        self.addCleanup(setattr, check, "_RECEIPT", None)

    def _fake(self, outputs_by_crate, *, failing_crates=(), page_path=None):
        def fake(command, **kwargs):
            crate = check.launch_crate(command)
            if crate in outputs_by_crate:
                code = 1 if crate in failing_crates else 0
                return mock.Mock(returncode=code, stdout=outputs_by_crate[crate])
            if page_path is not None and "--result-json" in command:
                Path(command[command.index("--result-json") + 1]).write_text(json.dumps(PAGE_REPORT))
            return mock.Mock(returncode=0, stdout="")
        return fake

    def test_cli_writes_receipt_and_summary_from_captured_launch_output(self):
        receipt_path = Path(self.temporary.name) / "receipt.json"
        with mock.patch.object(check.subprocess, "run", side_effect=self._fake({"case": CASE_CURRENT, "runtime": RUNTIME_CURRENT})), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "2"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [CASE_COMMAND, RUNTIME_COMMAND])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--receipt", str(receipt_path)]), 0)
        built = json.loads(receipt_path.read_text())
        self.assertEqual(built["source"]["revision"], "c0d14cd9")
        self.assertTrue(built["invocation"]["partial"], "a named step is a focused, not full, selection")
        crates = {record["crate"]: record for record in built["commands"]}
        self.assertEqual(crates["case"]["groups"]["case_collision::browser_tests"], 4)
        self.assertEqual(crates["runtime"]["groups"]["runtime::collision_browser_tests"], 14)
        summary = Path(str(receipt_path) + ".md")
        self.assertTrue(summary.is_file())
        self.assertIn("Verification receipt summary", summary.read_text())

    def test_cli_without_receipt_writes_nothing_and_keeps_default_exit(self):
        receipt_path = Path(self.temporary.name) / "absent.json"
        with mock.patch.object(check.subprocess, "run", side_effect=self._fake({"case": CASE_CURRENT})), \
                mock.patch.dict(check.STEPS, {"browser": (True, [CASE_COMMAND])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser"]), 0)
        self.assertFalse(receipt_path.exists())

    def test_cli_failure_writes_a_partial_receipt_and_marks_unrun_commands(self):
        receipt_path = Path(self.temporary.name) / "failed.json"
        with mock.patch.object(check.subprocess, "run", side_effect=self._fake(
                {"case": CASE_CURRENT, "runtime": RUNTIME_CURRENT}, failing_crates={"runtime"})), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                mock.patch.dict(check.STEPS, {"browser": (True, [CASE_COMMAND, RUNTIME_COMMAND, PAGE_COMMAND])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--receipt", str(receipt_path)]), 1)
        built = json.loads(receipt_path.read_text())
        self.assertEqual(built["invocation"]["exit_code"], 1)
        self.assertFalse(built["invocation"]["accepted"])
        statuses = {record["scope"]: record["status"] for record in built["commands"]}
        self.assertEqual(statuses["page-suite"], "not_run")
        self.assertEqual(built["invocation"]["not_run_commands"][0]["scope"], "page-suite")

    def test_cli_compares_against_a_supplied_base_receipt(self):
        base_path = Path(self.temporary.name) / "base.json"
        base = receipt.build_receipt(
            [("browser", CASE_COMMAND)],
            [receipt.Observation("browser", CASE_COMMAND, "ran", 0, 1.0, CASE_BASE)],
            source={"revision": "base", "dirty": False}, exit_code=0, partial=True)
        receipt.write_receipt(base_path, base)
        with mock.patch.object(check.subprocess, "run", side_effect=self._fake({"case": CASE_CURRENT})), \
                mock.patch.dict(check.STEPS, {"browser": (True, [CASE_COMMAND])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--receipt", str(base_path.with_name("current.json")),
                                         "--base-receipt", str(base_path)]), 0)
        built = json.loads((base_path.with_name("current.json")).read_text())
        self.assertEqual(built["before_after"]["status"], "compared")
        self.assertEqual(built["before_after"]["groups"][0]["delta"]["case_collision::browser_tests"], 1)

    def test_cli_records_native_binaries_through_the_shared_name_guard(self):
        receipt_path = Path(self.temporary.name) / "native.json"
        cargo = ["cargo", "test", "--workspace", "--locked", "--exclude", "boardstudio-footprints"]
        baseline = {"boardstudio_core": ["mirrored_half::reconcile::tests::alpha",
                                         "mirrored_half::reconcile::tests::delta"]}
        with mock.patch.object(check.subprocess, "run",
                               side_effect=lambda command, **kwargs: mock.Mock(returncode=0,
                                                                              stdout=NATIVE_CORE)), \
                mock.patch.object(check, "guard_native_tests", return_value=(True, False)), \
                mock.patch.object(check.check_native_tests, "load_baseline", return_value=baseline), \
                mock.patch.dict(check.STEPS, {"test": (True, [cargo])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["test", "--receipt", str(receipt_path)]), 0)
        native = json.loads(receipt_path.read_text())["native"]
        self.assertEqual([binary["name"] for binary in native["binaries"]], ["boardstudio_core"])
        self.assertEqual(native["baseline"]["matched"], 1)
        self.assertEqual(native["baseline"]["missing"],
                         {"boardstudio_core": ["mirrored_half::reconcile::tests::delta"]})
        self.assertEqual(native["baseline"]["recording_revision"], "deadbeef")

    def test_default_plan_is_full_but_named_and_quick_plans_are_partial(self):
        def partial_flag(plan):
            collector = mock.Mock()
            with mock.patch.object(check, "_RECEIPT", collector), \
                    mock.patch.object(check, "run_commands", return_value=(0.0, True)), \
                    contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(check.run_plan(plan), 0)
            return collector.finish.call_args.kwargs["partial"]

        full = [(name, [["true"]]) for name in check.DEFAULT]
        self.assertFalse(partial_flag(full))
        self.assertTrue(partial_flag([("browser", [["true"]])]))

    def test_cli_rejects_a_base_receipt_without_a_receipt(self):
        error = io.StringIO()
        with contextlib.redirect_stderr(error), self.assertRaises(SystemExit) as raised:
            check.main(["browser", "--base-receipt", "x.json"])
        self.assertEqual(raised.exception.code, 2)
        self.assertIn("--base-receipt requires --receipt", error.getvalue())

    def test_failed_native_command_still_records_observed_binary_results(self):
        path = Path(self.temporary.name) / "native-failed.json"
        command = ["cargo", "test", "--workspace"]
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=1, stdout=NATIVE_CORE)), \
                mock.patch.dict(check.STEPS, {"test": (True, [command])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["test", "--receipt", str(path)]), 1)
        built = json.loads(path.read_text())
        self.assertIsNotNone(built["native"], "a failed native launch still produced test evidence")
        self.assertEqual(built["native"]["counts"]["failed"], 1)

    def test_browser_preflight_failure_still_writes_unrun_plan(self):
        path = Path(self.temporary.name) / "preflight.json"
        with mock.patch.object(check.browser_tools, "locate",
                               side_effect=check.browser_tools.MissingBrowserTool("missing browser")), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--receipt", str(path)]), 1)
        self.assertTrue(path.exists(), "preflight failure must retain the unrun verification plan")
        built = json.loads(path.read_text())
        self.assertEqual(built["invocation"]["ran_commands"], 0)
        self.assertTrue(built["invocation"]["not_run_commands"])

    def test_interrupted_launch_writes_a_partial_receipt_before_reraising(self):
        path = Path(self.temporary.name) / "interrupted.json"
        with mock.patch.object(check.subprocess, "run", side_effect=KeyboardInterrupt), \
                mock.patch.dict(check.STEPS, {"browser": (True, [CASE_COMMAND])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(KeyboardInterrupt):
                check.main(["browser", "--receipt", str(path)])
        self.assertTrue(path.exists(), "an interrupted verification must still record its limits")
        self.assertFalse(json.loads(path.read_text())["invocation"]["accepted"])

    def test_failed_page_launch_cannot_reuse_a_preexisting_result_file(self):
        path = Path(self.temporary.name) / "page-receipt.json"
        page = Path(self.temporary.name) / "old-page.json"
        page.write_text(json.dumps(dict(PAGE_REPORT, failed_count=0, terminal_outcomes=[])))
        command = [check.PY, "scripts/run-wasm-tests.py", "--result-json", str(page)]
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=1, stdout="early failure")), \
                mock.patch.dict(check.STEPS, {"browser": (True, [command])}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(check.main(["browser", "--receipt", str(path)]), 1)
        record = json.loads(path.read_text())["commands"][0]
        self.assertFalse(record["fresh"], "launching a command does not make its old output fresh")
        self.assertFalse(record["passed"])
        self.assertFalse(record["complete"])

    def test_incomplete_or_different_filter_base_cannot_produce_a_group_delta(self):
        current = observation("browser", CASE_COMMAND, CASE_CURRENT)
        for base_command, base_output, code in (
                (CASE_COMMAND, EARLY_STOP, 1),
                ([*CASE_COMMAND, "--", "different_module::"], CASE_BASE, 0)):
            base = receipt.build_receipt(
                [("browser", base_command)],
                [observation("browser", base_command, base_output, exit_code=code)],
                source={"revision": "base", "dirty": False}, exit_code=code, partial=True)
            built = receipt.build_receipt(
                [("browser", CASE_COMMAND)], [current],
                source={"revision": "current", "dirty": False}, exit_code=0, partial=True,
                base=base, base_path=Path("base.json"))
            self.assertEqual(built["before_after"]["status"], "unknown")

    def test_retention_counts_keep_identical_names_in_different_binaries_distinct(self):
        outputs = ["Running unittests (target/debug/deps/alpha-0123456789abcdef)\n"
                   "test tests::same_name ... ok\n",
                   "Running unittests (target/debug/deps/beta-0123456789abcdef)\n"
                   "test tests::same_name ... ok\n"]
        record = receipt.native_record(outputs, full=True,
                                       baseline={"alpha": ["tests::same_name"], "beta": ["tests::same_name"]})
        self.assertEqual(record["baseline"]["known"], 2)
        self.assertEqual(record["baseline"]["matched"], 2)

    def test_malformed_page_payload_is_incomplete_without_throwing(self):
        for report in ({}, {"failed_count": "bad"}, {"terminal_outcomes": [None]}):
            with self.subTest(report=report):
                record = receipt.page_record(report, path="page.json", exit_code=0)
                self.assertFalse(record["complete"])
                self.assertFalse(record["passed"])
                self.assertTrue(record["problems"])


if __name__ == "__main__":
    unittest.main()
