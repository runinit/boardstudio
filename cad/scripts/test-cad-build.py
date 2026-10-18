#!/usr/bin/env python3
"""Verify pure-Rust CAD build stamp invalidation, cache reuse, and failure propagation."""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib
import unittest
from unittest.mock import patch

import cad_build as build

_spec = importlib.util.spec_from_file_location("cad_browser", Path(__file__).with_name("test-cad-browser.py"))
browser = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(browser)

REPO_ROOT = build.CAD_ROOT.parent
CAD_CRATES = ("monstertruck", "wasm")


class CadBuildTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.project = Path(self.directory.name)
        self.root = self.project / "cad"
        self.root.mkdir()
        self.patch = patch.multiple(build, CAD_ROOT=self.root)
        self.patch.start()
        self.addCleanup(self.patch.stop)

    def wasm_sources(self):
        (self.root / "wasm/src/model").mkdir(parents=True)
        (self.root / "wasm/src/lib.rs").write_text("// provider")
        (self.root / "wasm/src/model/source_validation_tests.rs").write_text("// test only")
        (self.root / "wasm/Cargo.toml").write_text('[package]\nname = "boardstudio-cad-wasm"\nversion = "0.1.0"\n')
        (self.root / "wasm/Cargo.lock").write_text("version = 4\n")
        (self.root / "monstertruck/src").mkdir(parents=True)
        (self.root / "monstertruck/src/lib.rs").write_text("// library")
        (self.root / "monstertruck/src/tests.rs").write_text("// library test only")
        (self.root / "monstertruck/Cargo.toml").write_text('[package]\nname = "boardstudio-cad"\nversion = "0.1.0"\n')
        for crate in build.VENDOR_CRATES:
            vendor_dir = self.root / "monstertruck-qualification/vendor" / crate / "src"
            vendor_dir.mkdir(parents=True)
            (vendor_dir / "lib.rs").write_text(f"// {crate}")
            (vendor_dir.parent / "Cargo.toml").write_text(f'[package]\nname = "{crate}"\nversion = "0.4.1"\n')
        (self.project / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["contracts/rust"]\n')
        contracts = self.project / "contracts/rust"
        (contracts / "src").mkdir(parents=True)
        (contracts / "src/lib.rs").write_text("// contracts")
        (contracts / "Cargo.toml").write_text('[package]\nname = "boardstudio-contracts"\nversion = "0.1.0"\n')

    def test_no_opt_is_parsed_by_wasm_pack_before_cargo_options(self):
        self.wasm_sources()
        with patch.object(build, "run") as run:
            build.build_wasm(no_opt=True)
        command = run.call_args.args[0]
        self.assertEqual(command[:2], ["wasm-pack", "build"])
        self.assertIn("boardstudio_cad_wasm", command)
        self.assertLess(command.index("--no-opt"), command.index("--locked"))

    def test_cgmath_source_changes_invalidate_wasm_cache(self):
        self.wasm_sources()
        source = self.project / "renderer/vendor/cgmath-next-0.18.1/src/lib.rs"
        source.parent.mkdir(parents=True)
        source.write_text("// reviewed fork")
        previous = build.wasm_inputs_digest()
        source.write_text("// updated fork")
        self.assertNotEqual(build.wasm_inputs_digest(), previous)

    def test_workspace_library_vendor_and_build_script_changes_invalidate_wasm_cache(self):
        self.wasm_sources()
        script = self.project / "cad_build.py"
        script.write_text("# build script")
        with patch.object(build, "__file__", str(script)):
            for path in (
                self.project / "Cargo.toml",
                script,
                self.root / "wasm/src/lib.rs",
                self.root / "monstertruck/src/lib.rs",
                self.root / "monstertruck-qualification/vendor/monstertruck-core/src/lib.rs",
                self.project / "contracts/rust/src/lib.rs",
            ):
                with self.subTest(path=path):
                    previous = build.wasm_inputs_digest()
                    path.write_text(path.read_text() + ("\n# changed" if path.suffix != ".rs" else "// changed"))
                    self.assertNotEqual(build.wasm_inputs_digest(), previous)

            for test_only in (
                self.root / "wasm/src/model/source_validation_tests.rs",
                self.root / "monstertruck/src/tests.rs",
            ):
                with self.subTest(test_only=test_only):
                    previous = build.wasm_inputs_digest()
                    test_only.write_text(test_only.read_text() + "\n// test change")
                    self.assertEqual(build.wasm_inputs_digest(), previous)

    def test_unchanged_provider_inputs_skip_rebuild_and_remove_stale_cadrum_files(self):
        self.wasm_sources()
        pkg = self.root / "wasm/pkg"
        pkg.mkdir(parents=True)
        (pkg / "package.json").write_text("generated")
        (pkg / "boardstudio_cadrum_wasm.js").write_text("stale")
        output = pkg / "boardstudio_cad_wasm_bg.wasm"
        with patch.object(build, "run", side_effect=lambda *a, **k: output.write_bytes(b"wasm")) as run:
            build.build_wasm()
        run.assert_called_once()
        self.assertFalse((pkg / "package.json").exists())
        self.assertFalse((pkg / "boardstudio_cadrum_wasm.js").exists())
        with patch.object(build, "run") as run, contextlib.redirect_stdout(io.StringIO()) as printed:
            build.build_wasm()
        run.assert_not_called()
        self.assertIn("skipping the WASM build", printed.getvalue())

    def test_native_tests_run_library_and_provider_and_propagate_failures(self):
        with patch.object(build, "run") as run:
            build.test_cad()
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(
            [command[3] for command in commands],
            ["monstertruck/Cargo.toml", "wasm/Cargo.toml"],
        )
        self.assertTrue(all("--locked" in command and "--release" in command for command in commands))
        with patch.object(build, "run", side_effect=[None, subprocess.CalledProcessError(1, "cargo")]):
            with self.assertRaises(subprocess.CalledProcessError):
                build.test_cad()


class CadWorkspaceTests(unittest.TestCase):
    """cargo must find each CAD crate's own root, even when this checkout is nested in another."""

    def test_cad_crates_declare_their_own_workspace(self):
        for crate in CAD_CRATES:
            with self.subTest(crate=crate):
                manifest = tomllib.loads((build.CAD_ROOT / crate / "Cargo.toml").read_text())
                self.assertIn("workspace", manifest)

    def test_nested_checkout_resolves_cad_crates_without_the_parent_workspace(self):
        exclude = tomllib.loads((REPO_ROOT / "Cargo.toml").read_text())["workspace"]["exclude"]
        with tempfile.TemporaryDirectory() as directory:
            outer = Path(directory)
            (outer / "app/src").mkdir(parents=True)
            (outer / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["app"]\n')
            (outer / "app/Cargo.toml").write_text('[package]\nname = "app"\nversion = "0.1.0"\nedition = "2021"\n')
            (outer / "app/src/lib.rs").write_text("")
            nested = outer / ".worktrees/nested"
            (nested / "contracts/rust/src").mkdir(parents=True)
            (nested / "contracts/rust/src/lib.rs").write_text("")
            (nested / "contracts/rust/Cargo.toml").write_text(
                '[package]\nname = "boardstudio-contracts"\nversion = "0.1.0"\nedition = "2021"\n')
            (nested / "Cargo.toml").write_text('[workspace]\nresolver = "3"\nmembers = ["contracts/rust"]\n'
                                               f"exclude = {json.dumps(exclude)}\n")
            for vendor_crate in build.VENDOR_CRATES:
                src_vendor = build.CAD_ROOT / "monstertruck-qualification/vendor" / vendor_crate
                dst_vendor = nested / "cad/monstertruck-qualification/vendor" / vendor_crate
                (dst_vendor / "src").mkdir(parents=True)
                (dst_vendor / "src/lib.rs").write_text("")
                shutil.copy(src_vendor / "Cargo.toml", dst_vendor / "Cargo.toml")
            for crate in CAD_CRATES:
                (nested / "cad" / crate / "src").mkdir(parents=True)
                (nested / "cad" / crate / "src/lib.rs").write_text("")
                shutil.copy(build.CAD_ROOT / crate / "Cargo.toml", nested / "cad" / crate / "Cargo.toml")
            for crate in CAD_CRATES:
                with self.subTest(crate=crate):
                    manifest = nested / "cad" / crate / "Cargo.toml"
                    done = subprocess.run(["cargo", "metadata", "--no-deps", "--format-version", "1",
                                           "--manifest-path", str(manifest)], capture_output=True, text=True)
                    self.assertEqual(done.returncode, 0, done.stderr)


class BrowserSmokeParserTests(unittest.TestCase):
    def test_result_is_read_from_the_rendered_page_and_unescaped(self):
        dom = '<html><pre id="result">{"ok":true,"error":null,"checks":[{"name":"a &amp; b","ok":true,"detail":"1 &lt; 2"}]}</pre></html>'
        result = browser.parse_result(dom)
        self.assertTrue(result["ok"])
        self.assertEqual(result["checks"][0]["name"], "a & b")
        self.assertEqual(result["checks"][0]["detail"], "1 < 2")

    def test_missing_or_unfinished_page_is_an_error_not_a_pass(self):
        with self.assertRaisesRegex(RuntimeError, "did not render"):
            browser.parse_result("<html></html>")
        with self.assertRaisesRegex(RuntimeError, "never finished"):
            browser.parse_result('<pre id="result">pending</pre>')

    def test_a_failed_check_makes_the_run_fail(self):
        failing = '{"ok":false,"error":null,"checks":[{"name":"x","ok":false,"detail":"d"}]}'
        with patch.object(browser, "serve"), patch.object(browser, "run_page", return_value=f'<pre id="result">{failing}</pre>'), patch.object(browser.Path, "exists", return_value=True):
            with contextlib.redirect_stdout(io.StringIO()), self.assertRaises(SystemExit) as raised:
                browser.main()
        self.assertEqual(raised.exception.code, 1)


if __name__ == "__main__":
    unittest.main()
