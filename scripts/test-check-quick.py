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

    def test_layout_tests_generate_the_css_asset_before_browser_tests(self):
        plan = check.quick_plan(quick.select(["web/crates/layout/src/lib.rs"], PACKAGES), PACKAGES)
        names = [name for name, _ in plan]
        self.assertLess(names.index("layout-test-css"), names.index("browser"))
        command = dict(plan)["layout-test-css"][0]
        self.assertEqual(command, ["dx", "build", "--web", "--no-default-features",
                                   "--features", "page", "--cargo-args=--locked"])

    def test_other_crate_tests_do_not_generate_layout_css(self):
        self.assertNotIn("layout-test-css", self.plan(["web/crates/keymap/src/lib.rs"]))

    def test_a_page_edit_adds_the_page_runner_for_those_files(self):
        plan = self.plan(["web/src/presentation/a.rs"])
        runner = [command for command in plan["browser"] if "run-wasm-tests.py" in " ".join(command)]
        self.assertEqual(len(runner), 1)
        self.assertIn("web/src/presentation/a.rs", runner[0])

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

    def test_an_explicit_base_that_does_not_resolve_fails_instead_of_testing_nothing(self):
        def fake(args, **kwargs):
            return mock.Mock(returncode=128 if args[1] == "merge-base" else 0, stdout="")
        with mock.patch.object(quick.subprocess, "run", side_effect=fake):
            self.assertEqual(quick.changed_files(ROOT), [])
            with self.assertRaises(SystemExit):
                quick.changed_files(ROOT, "0123abc")


# The gate fixture: Core's dependents, footprints, and a leaf crate with no pin of its own.
GATE_PACKAGES = {package.name: package for package in (
    quick.Package("core", "core", set(), {"core", "integration"}),
    quick.Package("boardstudio-web-layout", "web/crates/layout", {"core"}, {"boardstudio_web_layout"}),
    quick.Package("boardstudio-web-keymap", "web/crates/keymap", {"core"}, {"boardstudio_web_keymap"}),
    quick.Package("boardstudio-web", "web", {"boardstudio-web-layout", "boardstudio-web-keymap"}, {"boardstudio_web"}),
    quick.Package(check.FOOTPRINTS, "footprints", set(), {"boardstudio_footprints", "context"}),
    quick.Package("contracts", "contracts/rust", set(), {"contracts", "region"}),
)}
GATE_BASELINE = {binary: [f"{binary}::a", f"{binary}::b"] for binary in (
    "core", "integration", "boardstudio_web_layout", "boardstudio_web_keymap", "boardstudio_footprints",
    "context", "contracts", "region")}
GATE_SMOKE = {
    "native": {binary: {"test": f"{binary}::a"} for binary in GATE_BASELINE if binary != "region"},
    "browser": {key: {"test": f"{key}::pin"} for key in ("keymap", "layout --skip setup_guide::", "layout setup_guide::")},
    "page": {"test": "page::pin", "file": "web/src/pin.rs"},
}


class GatePlanTests(unittest.TestCase):
    def plan(self, files, half):
        return dict(check.gate_plan(quick.select(files, GATE_PACKAGES), GATE_PACKAGES, half,
                                    GATE_SMOKE, GATE_BASELINE))

    def test_a_leaf_edit_runs_affected_crates_whole_and_the_rest_by_pin(self):
        native = self.plan(["web/crates/layout/src/matrix.rs"], "native")
        self.assertEqual(list(native), ["repo", "lint", "typecheck", "test"])
        self.assertEqual(native["test"], [
            ["cargo", "test", "--locked", "-p", "boardstudio-web", "-p", "boardstudio-web-layout", "-p", "contracts"],
            ["cargo", "test", "--locked", "--tests", "-p", "boardstudio-web-keymap", "-p", "core",
             "--", "--exact", "boardstudio_web_keymap::a", "core::a", "integration::a"],
            ["cargo", "test", "--locked", "--tests", "-p", check.FOOTPRINTS,
             "--", "--exact", "boardstudio_footprints::a", "context::a"],
        ])  # contracts' `region` binary has no pin, so contracts stays whole in the gate
        web = self.plan(["web/crates/layout/src/matrix.rs"], "web")
        self.assertEqual(list(web), ["build", "layout-test-css", "browser"])
        launches = [command for command in web["browser"] if check.is_crate_launch(command)]
        self.assertEqual(launches, [
            command if check.launch_crate(command) == "layout" else check.smoke_launch(command, "keymap::pin")
            if check.launch_crate(command) == "keymap" else command  # no pin: untiered launches run whole
            for command in check.browser() if check.is_crate_launch(command)])
        self.assertIn(check.DRAG_SMOKE, web["browser"])
        self.assertIn([check.PY, "cad/scripts/test-cad-browser.py"], web["browser"])
        self.assertTrue(any("--list" in command for command in web["browser"]))
        self.assertIn([check.PY, "scripts/run-wasm-tests.py", "--files", "web/src/pin.rs", "--result-json",
                       "web/target/test-results/browser-quick.json"], web["browser"])

    def test_a_core_edit_runs_everything_downstream_whole(self):
        native = self.plan(["core/src/lib.rs"], "native")
        self.assertEqual(native["test"][0], ["cargo", "test", "--locked", "-p", "boardstudio-web",
                                             "-p", "boardstudio-web-keymap", "-p", "boardstudio-web-layout",
                                             "-p", "contracts", "-p", "core"])
        self.assertEqual(native["test"][1][-3:], ["--exact", "boardstudio_footprints::a", "context::a"])
        web = self.plan(["core/src/lib.rs"], "web")
        self.assertFalse(any(command[-2:] == ["--", "keymap::pin"] for command in web["browser"]))

    def test_docs_only_still_lints_builds_and_runs_the_drag_smoke(self):
        native = self.plan(["docs/x.md"], "native")
        self.assertEqual(list(native), ["repo", "lint", "test"])
        web = self.plan(["docs/x.md"], "web")
        self.assertEqual(list(web), ["build", "browser"])
        self.assertIn(check.DRAG_SMOKE, web["browser"])
        self.assertFalse(any("--list" in command for command in web["browser"]))

    def test_a_shared_input_runs_the_full_native_and_browser_commands(self):
        self.assertEqual(self.plan(["Cargo.lock"], "native")["test"], check.CARGO_TESTS)
        self.assertEqual(self.plan(["Cargo.lock"], "web")["browser"], check.browser())

    def test_cad_and_tooling_changes_add_their_suites(self):
        native = self.plan(["cad/wasm/src/lib.rs", "scripts/check.py"], "native")
        self.assertIn("tooling", native)
        self.assertEqual(native["test"][-1], [check.PY, "cad/scripts/test-cad.py"])


class SmokeManifestTests(unittest.TestCase):
    def test_every_baselined_binary_and_browser_launch_has_a_current_pin(self):
        smoke = check.load_smoke()
        baseline = check.check_native_tests.load_baseline(check.check_native_tests.BASELINE)
        self.assertEqual(smoke["native"].keys(), baseline.keys())
        for binary, entry in smoke["native"].items():
            self.assertIn(entry["test"], baseline[binary], binary)
        launches = {check.launch_key(command) for command in check.browser() if check.is_crate_launch(command)}
        self.assertEqual(smoke["browser"].keys(), launches)
        self.assertTrue((ROOT / smoke["page"]["file"]).is_file())

    def test_a_smoke_launch_fails_when_its_pin_did_not_pass(self):
        pin = check.load_smoke()["browser"]["host"]["test"]
        command = check.smoke_launch(check.select_browser_crate_launches(["host"])[0], pin)
        self.assertEqual(check.smoke_failure(command, 0, f"test {pin} ... ok\n")[0], 0)
        self.assertEqual(check.smoke_failure(command, 0, "running 0 tests\n")[0], 1)


if __name__ == "__main__":
    unittest.main()
