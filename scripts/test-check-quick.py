#!/usr/bin/env python3
from __future__ import annotations

from pathlib import Path
import sys
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import check  # noqa: E402
import check_quick as quick  # noqa: E402

PACKAGES = {
    "core": quick.Package("core", "core"),
    "app": quick.Package("app", "application", {"core"}),
    "layout": quick.Package("layout", "web/crates/layout", {"core", "app"}),
    "keymap": quick.Package("keymap", "web/crates/keymap", {"core"}),
    "footprints": quick.Package("boardstudio-footprints", "catalogue/footprints"),
    "web": quick.Package("web", "web", {"layout", "keymap"}),
}
PACKAGES = {package.name: package for package in PACKAGES.values()}


class QuickSelectionTests(unittest.TestCase):
    def test_documentation_only_changes_need_no_rust_checks(self):
        selection = quick.select(["docs/plans/x/map.md", "README.md"], PACKAGES)
        self.assertFalse(selection.rust)
        self.assertEqual(check.quick_plan(selection, PACKAGES), [("repo", check.STEPS["repo"][1])])

    def test_a_crate_edit_selects_the_crate_and_its_dependents_only(self):
        selection = quick.select(["web/crates/keymap/src/lib.rs"], PACKAGES)
        self.assertEqual(selection.packages, {"keymap", "web"})
        self.assertFalse(selection.full_rust)

    def test_a_core_edit_selects_everything_that_depends_on_core(self):
        selection = quick.select(["core/src/lib.rs"], PACKAGES)
        self.assertEqual(selection.packages, {"core", "app", "layout", "keymap", "web"})
        self.assertNotIn("boardstudio-footprints", selection.packages)

    def test_the_longest_matching_directory_owns_a_path(self):
        self.assertEqual(quick.owner("web/crates/layout/src/a.rs", PACKAGES).name, "layout")
        self.assertEqual(quick.owner("web/src/main.rs", PACKAGES).name, "web")
        self.assertIsNone(quick.owner("unknown/file.rs", PACKAGES))

    def test_shared_inputs_and_unplaced_files_fall_back_to_every_rust_check(self):
        shared = quick.select(["Cargo.lock"], PACKAGES)
        self.assertTrue(shared.full_rust)
        self.assertEqual(shared.packages, set(PACKAGES))
        unplaced = quick.select(["mystery/file.rs"], PACKAGES)
        self.assertTrue(unplaced.full_rust)
        self.assertEqual(unplaced.unplaced, ["mystery/file.rs"])

    def test_tooling_scripts_select_the_tooling_step_without_rust(self):
        selection = quick.select(["scripts/check.py"], PACKAGES)
        self.assertTrue(selection.tooling)
        self.assertFalse(selection.rust)

    def test_measurement_protocol_edits_keep_the_tooling_only_quick_selection(self):
        for path in ("cad/scripts/test-measure-cad.py", "cad/scripts/test-measure-cad-runtime.cjs"):
            with self.subTest(path=path):
                selection = quick.select([path], PACKAGES)
                self.assertEqual(check.quick_plan(selection, PACKAGES),
                                 [("repo", check.STEPS["repo"][1]),
                                  ("tooling", check.STEPS["tooling"][1])])
                self.assertFalse(selection.rust)

    def test_page_sources_select_the_page_tests_by_file(self):
        selection = quick.select(["web/src/presentation/a.rs"], PACKAGES)
        self.assertEqual(selection.page_files, ["web/src/presentation/a.rs"])


class QuickPlanTests(unittest.TestCase):
    def plan(self, files):
        return dict(check.quick_plan(quick.select(files, PACKAGES), PACKAGES))

    def test_a_web_crate_edit_runs_its_launches_without_full_web_build(self):
        plan = self.plan(["web/crates/keymap/src/lib.rs"])
        self.assertNotIn("build", plan)
        launches = [command for command in plan["browser"] if check.is_crate_launch(command)]
        self.assertTrue(launches)
        self.assertTrue(all("web/crates/keymap" in command for command in launches))
        self.assertEqual(plan["test"][0][:3], ["cargo", "test", "--locked"])
        self.assertIn("keymap", plan["test"][0])
        self.assertIn("web", plan["test"][0])

    def test_quick_layout_runs_skip_the_unconsumed_css_build(self):
        plan = check.quick_plan(quick.select(["web/crates/layout/src/lib.rs"], PACKAGES), PACKAGES)
        names = [name for name, _ in plan]
        self.assertIn("browser", names)
        self.assertNotIn("layout-test-css", names)

    def test_other_crate_tests_do_not_generate_layout_css(self):
        self.assertNotIn("layout-test-css", self.plan(["web/crates/keymap/src/lib.rs"]))

    def test_a_page_edit_adds_the_page_runner_for_those_files(self):
        plan = self.plan(["web/src/presentation/a.rs"])
        runner = [command for command in plan["browser"] if "run-wasm-tests.py" in " ".join(command)]
        self.assertEqual(len(runner), 1)
        self.assertIn("--files", runner[0])
        self.assertIn("web/src/presentation/a.rs", runner[0])

    def test_page_runner_batches_filters_like_the_full_gate(self):
        plan = self.plan(["web/src/presentation/a.rs"])
        runner = [command for command in plan["browser"] if "run-wasm-tests.py" in " ".join(command)]
        self.assertEqual(len(runner), 1)
        self.assertIn("--depth", runner[0])
        self.assertIn("1", runner[0])
        for name in check.ISOLATED:
            self.assertIn(f"--isolate={name}", runner[0])

    def test_unfilterable_page_inputs_keep_full_consuming_test_coverage(self):
        for path in ("web/src/presentation/layout_layers_fixtures/paired-rejoin-preview.json",
                     "web/tests/page.rs"):
            for files in ([path], ["web/src/presentation/a.rs", path],
                          [path, "web/src/presentation/a.rs"]):
                with self.subTest(files=files):
                    selection = quick.select(files, PACKAGES)
                    self.assertNotIn(path, selection.page_files)
                    self.assertTrue(selection.full_rust)
                    self.assertEqual(selection.packages, set(PACKAGES))
                    self.assertTrue(any(path in note and "not a web/src Rust file" in note
                                        and "every Rust check applies" in note for note in selection.notes))
                    plan = dict(check.quick_plan(selection, PACKAGES))
                    self.assertEqual(plan["test"], check.CARGO_TESTS)
                    self.assertEqual(plan["browser"], [command for command in check.browser()
                                                     if "run-drag-browser.py" not in " ".join(command)])
                    runner = [command for command in plan["browser"]
                              if "scripts/run-wasm-tests.py" in command]
                    self.assertEqual(len(runner), 1)
                    self.assertIn("--all", runner[0])
                    self.assertNotIn("--files", runner[0])
                    self.assertNotIn(path, runner[0])

    def test_a_shared_edit_runs_the_full_native_and_browser_commands(self):
        plan = self.plan(["Cargo.lock"])
        self.assertEqual(plan["test"], check.CARGO_TESTS)
        self.assertEqual(plan["browser"], [command for command in check.browser()
                                             if "run-drag-browser.py" not in " ".join(command)])

    def test_quick_full_rust_plan_skips_page_drag_smoke_without_building(self):
        plan = self.plan(["Cargo.lock"])
        self.assertNotIn("build", plan)
        self.assertFalse(any("run-drag-browser.py" in " ".join(command) for command in plan["browser"]))

    def test_changed_files_come_from_git_and_include_untracked_files(self):
        def fake(args, **kwargs):
            outputs = {"merge-base": "abc\n", "diff": "a.rs\nb.rs\n", "ls-files": "c.rs\n"}
            return mock.Mock(returncode=0, stdout=outputs[args[1]])
        with mock.patch.object(quick.subprocess, "run", side_effect=fake):
            self.assertEqual(quick.changed_files(ROOT), ["a.rs", "b.rs", "c.rs"])


if __name__ == "__main__":
    unittest.main()
