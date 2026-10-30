#!/usr/bin/env python3
from __future__ import annotations

import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("check", HERE / "check.py")
check = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(check)
guard = check.check_native_tests

CORE_OUTPUT = """\
   Compiling boardstudio_core v0.1.0
     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)

running 4 tests
test alpha::tests::keeps_pairs ... ok
test alpha::tests::drops_pairs ... FAILED
test alpha::tests::needs_kicad ... ignored, requires KiCad CLI 10
test beta::roundtrip ... ok

test result: FAILED. 2 passed; 1 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests/durable_session.rs (target/debug/deps/durable_session-fedcba9876543210)

running 1 test
test restores_session ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests boardstudio_core

running 1 test
test src/lib.rs - documented (line 12) ... ok
"""
EXECUTED = {
    "boardstudio_core": ["alpha::tests::drops_pairs", "alpha::tests::keeps_pairs", "beta::roundtrip"],
    "durable_session": ["restores_session"],
}


class CollectTests(unittest.TestCase):
    def test_names_are_collected_per_binary_with_hashes_stripped(self):
        run = guard.collect([CORE_OUTPUT])
        self.assertEqual({k: sorted(v) for k, v in run.executed.items()}, EXECUTED)
        self.assertEqual(run.ignored, {"boardstudio_core": {"alpha::tests::needs_kicad"}})
        self.assertEqual(run.binaries, {"boardstudio_core", "durable_session"})

    def test_a_result_after_a_tests_own_stderr_on_the_same_line_is_still_collected(self):
        noisy = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "ZipWriter::drop: failed to finalize: Io(Custom { kind: Other })"
                 "test archive::tests::limits_apply ... ok\n")
        self.assertEqual(guard.collect([noisy]).executed, {"boardstudio_core": {"archive::tests::limits_apply"}})

    def test_a_should_panic_test_is_collected_by_its_name(self):
        output = ("     Running tests/integration.rs (target/debug/deps/integration-0123456789abcdef)\n"
                  "test boundary_parity::rejects_a_mismatch - should panic ... ok\n"
                  "test boundary_parity::plain ... ok\n")
        self.assertEqual(guard.collect([output]).executed,
                         {"integration": {"boundary_parity::rejects_a_mismatch", "boundary_parity::plain"}})

    def test_a_result_split_by_a_tests_stderr_inside_the_line_still_counts_as_executed(self):
        # Observed: the test's stderr lands between `...` and the verdict, leaving `Customok {`.
        split = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "ZipWriter::drop: failed to finalize archive: Io("
                 "test archive::tests::pack_verifies ... Customok { \n")
        self.assertEqual(guard.collect([split]).executed, {"boardstudio_core": {"archive::tests::pack_verifies"}})

    def test_a_result_glued_to_a_stderr_word_without_a_separator_still_counts_as_executed(self):
        # Observed 2026-10-12: stderr stopped mid-word (`Other`) and libtest's `test ...` followed
        # with no separator, then the verdict was split too (`, okerror`).
        glued = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "ZipWriter::drop: failed to finalize archive: Io(Custom { kind: Other"
                 "test archive::tests::stored_and_deflated_crc_corruption_is_rejected ... , okerror\n")
        self.assertEqual(
            guard.collect([glued]).executed,
            {"boardstudio_core": {"archive::tests::stored_and_deflated_crc_corruption_is_rejected"}})

    def test_an_ignored_verdict_with_stderr_glued_after_it_is_still_ignored_not_executed(self):
        glued = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "test alpha::tests::needs_kicad ... ignoredZipWriter::drop: failed\n")
        run = guard.collect([glued])
        self.assertEqual(run.executed, {})
        self.assertEqual(run.ignored, {"boardstudio_core": {"alpha::tests::needs_kicad"}})

    def test_a_name_mentioned_in_noise_without_a_result_marker_does_not_count(self):
        noise = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "kind: Othertest archive::tests::never_ran\n"
                 "test archive::tests::never_ran failed to start\n")
        self.assertEqual(guard.collect([noise]).executed, {})

    def test_doc_tests_are_not_baselined(self):
        run = guard.collect([CORE_OUTPUT])
        self.assertNotIn("src/lib.rs - documented (line 12)", run.executed["durable_session"])
        self.assertEqual(run.count(), 4)

    def test_outputs_of_several_commands_are_combined(self):
        other = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_footprints-0011223344556677)\n"
                 "test generates ... ok\n")
        run = guard.collect([CORE_OUTPUT, other])
        self.assertEqual(run.executed["boardstudio_footprints"], {"generates"})


INTERRUPTED = """\
     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)

running 100 tests
test mirrored_half::reconcile::tests::alpha ... ok
"""


class LaunchesTests(unittest.TestCase):
    """Per-launch announced counts, named outcomes and terminal summaries."""

    def test_each_binary_keeps_its_own_announced_count_and_summary(self):
        found = guard.launches([CORE_OUTPUT])
        self.assertEqual([launch.name for launch in found], ["boardstudio_core", "durable_session"])
        self.assertEqual(found[0].announced, 4)
        self.assertEqual(found[0].summary, ("FAILED", 2, 1, 1, 0, 0))
        self.assertEqual(found[0].outcomes["alpha::tests::needs_kicad"], "ignored")
        self.assertEqual(found[1].announced, 1)
        self.assertEqual(found[1].summary, ("ok", 1, 0, 0, 0, 0))

    def test_a_doc_tests_section_is_not_a_launched_binary(self):
        start = CORE_OUTPUT.index("   Doc-tests boardstudio_core")
        self.assertEqual(guard.launches([CORE_OUTPUT[start:]]), [])
        self.assertEqual(len(guard.launches([CORE_OUTPUT])), 2, "the doc-test summary is not a third launch")

    def test_an_interrupted_launch_keeps_its_pass_without_a_terminal_summary(self):
        launch, = guard.launches([INTERRUPTED])
        self.assertEqual(launch.name, "boardstudio_core")
        self.assertEqual(launch.announced, 100)
        self.assertEqual(launch.outcomes, {"mirrored_half::reconcile::tests::alpha": "passed"})
        self.assertIsNone(launch.summary)

    def test_a_launch_interrupted_before_announcing_tests_has_no_count(self):
        output = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                  "test alpha::tests::one ... ok\n")
        launch, = guard.launches([output])
        self.assertIsNone(launch.announced)
        self.assertIsNone(launch.summary)

    def test_output_of_several_commands_keeps_each_launch_separate(self):
        other = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                 "running 1 test\ntest only ... ok\n"
                 "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n")
        names = [launch.name for launch in guard.launches([CORE_OUTPUT, other, INTERRUPTED])]
        self.assertEqual(names, ["boardstudio_core", "durable_session", "boardstudio_core", "boardstudio_core"])
        self.assertIsNone(guard.launches([other, INTERRUPTED])[1].summary)

    def test_a_zero_test_launch_is_announced_and_summarised(self):
        output = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                  "running 0 tests\n\n"
                  "test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 88 filtered out; finished in 0.00s\n")
        launch, = guard.launches([output])
        self.assertEqual(launch.announced, 0)
        self.assertEqual(launch.summary, ("ok", 0, 0, 0, 0, 88))

    def test_a_summary_stopped_after_filtered_out_is_not_terminal(self):
        # libtest ends its summary with `; finished in N.NNs`; a line stopped earlier is truncated.
        truncated = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_core-0123456789abcdef)\n"
                     "running 1 test\ntest only ... ok\n\n"
                     "test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 1 filtered out\n")
        launch, = guard.launches([truncated])
        self.assertIsNone(launch.summary)

    def test_the_name_guard_still_collects_the_same_executed_names(self):
        self.assertEqual(sorted(guard.collect([CORE_OUTPUT, INTERRUPTED]).executed["boardstudio_core"]),
                         ["alpha::tests::drops_pairs", "alpha::tests::keeps_pairs",
                          "beta::roundtrip", "mirrored_half::reconcile::tests::alpha"])


class VerifyTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "baseline.json"
        self.path.write_text(json.dumps(EXECUTED))

    def verify(self, output, **kwargs):
        errors = io.StringIO()
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(errors):
            result = guard.verify([output], baseline_path=self.path, **kwargs)
        return result, errors.getvalue()

    def test_an_identical_run_passes(self):
        self.assertEqual(self.verify(CORE_OUTPUT, full=True)[0], (True, False))

    def test_a_deleted_test_fails_and_is_named(self):
        output = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n", "")
        (ok, written), errors = self.verify(output, full=True)
        self.assertFalse(ok)
        self.assertFalse(written)
        self.assertIn("beta::roundtrip (absent)", errors)
        self.assertIn("--update-native-baseline", errors)

    def test_a_test_that_is_now_ignored_is_no_longer_executed(self):
        output = CORE_OUTPUT.replace("test beta::roundtrip ... ok", "test beta::roundtrip ... ignored")
        (ok, _), errors = self.verify(output, full=True)
        self.assertFalse(ok)
        self.assertIn("beta::roundtrip (ignored)", errors)

    def test_added_tests_do_not_fail(self):
        output = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n",
                                     "test beta::roundtrip ... ok\ntest beta::brand_new ... ok\n")
        self.assertEqual(self.verify(output, full=True)[0], (True, False))

    def test_a_vanished_binary_fails_a_full_run_but_not_a_partial_one(self):
        output = CORE_OUTPUT.split("     Running tests/durable_session.rs")[0]
        self.assertFalse(self.verify(output, full=True)[0][0])
        self.assertTrue(self.verify(output, full=False)[0][0])

    def test_a_partial_run_still_fails_for_a_binary_it_ran(self):
        output = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n", "")
        self.assertFalse(self.verify(output, full=False)[0][0])

    def test_a_missing_baseline_fails_until_it_is_recorded(self):
        self.path.unlink()
        self.assertFalse(self.verify(CORE_OUTPUT, full=True)[0][0])
        self.assertEqual(self.verify(CORE_OUTPUT, full=True, update=True)[0], (True, True))
        self.assertEqual(json.loads(self.path.read_text()), EXECUTED)

    def test_update_replaces_the_baseline_on_a_full_run(self):
        output = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n", "").replace(
            "     Running tests/durable_session.rs (target/debug/deps/durable_session-fedcba9876543210)",
            "     Running tests/renamed.rs (target/debug/deps/renamed-fedcba9876543210)")
        self.assertEqual(self.verify(output, full=True, update=True)[0], (True, True))
        recorded = json.loads(self.path.read_text())
        self.assertEqual(sorted(recorded), ["boardstudio_core", "renamed"], "a full run drops vanished binaries")
        self.assertEqual(recorded["boardstudio_core"], ["alpha::tests::drops_pairs", "alpha::tests::keeps_pairs"])
        self.assertEqual(self.verify(output, full=True)[0], (True, False))

    def test_update_from_a_partial_run_keeps_the_binaries_it_did_not_run(self):
        self.path.write_text(json.dumps({**EXECUTED, "elsewhere": ["kept"]}))
        self.assertEqual(self.verify(CORE_OUTPUT, full=False, update=True)[0], (True, True))
        self.assertEqual(json.loads(self.path.read_text())["elsewhere"], ["kept"])

    def test_an_unreadable_baseline_fails(self):
        self.path.write_text("{not json")
        self.assertFalse(self.verify(CORE_OUTPUT, full=True)[0][0])


class CheckIntegrationTests(unittest.TestCase):
    """`check.py test` applies the guard to the output its cargo commands already produce."""

    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "baseline.json"
        self.path.write_text(json.dumps(EXECUTED))
        self.runs = []

    def execute(self, outputs, *, jobs, update=False, commands=None, names=("test",)):
        commands = commands or check.CARGO_TESTS

        def fake(command, **kwargs):
            self.runs.append(command)
            return mock.Mock(returncode=0, stdout=outputs.get(tuple(command), ""))

        errors = io.StringIO()
        with mock.patch.object(check.subprocess, "run", side_effect=fake), \
                mock.patch.object(check, "_parallel_process", side_effect=lambda command, registry, **kwargs: fake(command, **kwargs)), \
                mock.patch.object(guard, "BASELINE", self.path), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": str(jobs)}), \
                mock.patch.dict(check.STEPS, {"test": (True, commands)}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(errors):
            return check.run(list(names), update_native_baseline=update), errors.getvalue()

    def outputs(self, core):
        footprints = ("     Running unittests src/lib.rs (target/debug/deps/boardstudio_footprints-0011223344556677)\n"
                      "test generates ... ok\n")
        self.path.write_text(json.dumps({**EXECUTED, "boardstudio_footprints": ["generates"]}))
        return {tuple(check.CARGO_TESTS[0]): core, tuple(check.CARGO_TESTS[1]): footprints}

    def test_a_deleted_test_fails_the_step_serially_and_in_parallel(self):
        gutted = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n", "")
        for jobs in (1, 4):
            with self.subTest(jobs=jobs):
                code, errors = self.execute(self.outputs(gutted), jobs=jobs)
                self.assertEqual(code, 1)
                self.assertIn("beta::roundtrip (absent)", errors)
                self.assertIn("step 'test' failed", errors)

    def test_an_unchanged_suite_passes_and_runs_each_command_once(self):
        for jobs in (1, 4):
            with self.subTest(jobs=jobs):
                self.runs.clear()
                self.assertEqual(self.execute(self.outputs(CORE_OUTPUT), jobs=jobs)[0], 0)
                self.assertEqual(self.runs, check.CARGO_TESTS)

    def test_the_update_flag_records_a_deliberate_removal(self):
        gutted = CORE_OUTPUT.replace("test beta::roundtrip ... ok\n", "")
        outputs = self.outputs(gutted)
        self.assertEqual(self.execute(outputs, jobs=1, update=True)[0], 0)
        self.assertNotIn("beta::roundtrip", json.loads(self.path.read_text())["boardstudio_core"])
        self.assertEqual(self.execute(outputs, jobs=1)[0], 0)

    def test_the_update_flag_without_a_cargo_test_is_an_error(self):
        code, errors = self.execute({}, jobs=1, update=True, commands=[["python3", "-c", "pass"]])
        self.assertEqual(code, 1)
        self.assertIn("baseline is unchanged", errors)

    def test_a_failing_cargo_command_is_not_second_guessed_by_the_guard(self):
        with mock.patch.object(check.subprocess, "run", return_value=mock.Mock(returncode=101, stdout="")), \
                mock.patch.object(guard, "BASELINE", self.path), \
                mock.patch.dict(os.environ, {"BOARDSTUDIO_CHECK_JOBS": "1"}), \
                contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as errors:
            self.assertEqual(check.run(["test"]), 1)
        self.assertNotIn("native tests", errors.getvalue())

    def test_the_update_flag_is_rejected_for_steps_that_run_no_tests(self):
        with contextlib.redirect_stderr(io.StringIO()), self.assertRaises(SystemExit) as raised:
            check.main(["repo", "--update-native-baseline"])
        self.assertEqual(raised.exception.code, 2)

    def test_the_committed_baseline_is_well_formed(self):
        baseline = guard.load_baseline(HERE / "check-native-tests-baseline.json")
        self.assertTrue(baseline, "the committed baseline must not be empty")
        for binary, names in baseline.items():
            self.assertEqual(names, sorted(set(names)), f"{binary}: sorted, unique names")
        self.assertIn("boardstudio_core", baseline)


if __name__ == "__main__":
    unittest.main()
