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
    def test_valid_launch_without_baseline_and_legacy_update(self):
        output = CORE_OUTPUT.replace("drops_pairs ... FAILED", "drops_pairs ... ok").replace(
            "FAILED. 2 passed; 1 failed", "ok. 3 passed; 0 failed")
        for update in (False, True):
            with self.subTest(update=update):
                self.assertEqual(guard.verify([output], full=True, update=update), (True, False))

    def test_failed_truncated_and_inconsistent_launches_fail(self):
        valid = CORE_OUTPUT.replace("drops_pairs ... FAILED", "drops_pairs ... ok").replace(
            "FAILED. 2 passed; 1 failed", "ok. 3 passed; 0 failed")
        for output in (CORE_OUTPUT, valid.split("test result:")[0],
                       valid.replace("running 4 tests", "running 5 tests")):
            with self.subTest(output=output), contextlib.redirect_stderr(io.StringIO()):
                self.assertEqual(guard.verify([output], full=True), (False, False))

    def test_missing_explicit_baseline_does_not_fail(self):
        with tempfile.TemporaryDirectory() as directory:
            output = CORE_OUTPUT.replace("drops_pairs ... FAILED", "drops_pairs ... ok").replace(
                "FAILED. 2 passed; 1 failed", "ok. 3 passed; 0 failed")
            self.assertEqual(guard.verify([output], full=True, baseline_path=Path(directory) / "missing"),
                             (True, False))

    def test_no_binary_output(self):
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(guard.verify([], full=True), (True, False))
            self.assertEqual(guard.verify([], full=True, update=True), (False, False))


if __name__ == "__main__":
    unittest.main()
