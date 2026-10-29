#!/usr/bin/env python3
"""Hermetic checks for the required face-completeness gate."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location('face_gate', HERE / 'face-completeness-gate.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class FaceGateTests(unittest.TestCase):
    def test_every_face_test_is_classified(self):
        spec = importlib.util.spec_from_file_location('face_tests', HERE / 'test-face-completeness-web.py')
        tests = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tests)
        groups = (set(tests.HERMETIC_TESTS), set(gate.CONTROLS), {tests.NODE_REFERENCE_TEST})
        self.assertEqual(sum(map(len, groups)), len(set.union(*groups)))
        self.assertEqual(set(unittest.defaultTestLoader.getTestCaseNames(tests.FaceWebTests)), set.union(*groups))

    def test_builds_selected_driver_and_overrides_untrusted_environment(self):
        with tempfile.TemporaryDirectory() as directory:
            driver = Path(directory) / 'face'
            driver.touch()
            with mock.patch.dict(os.environ, {'CAD_FACE_WEB_DRIVER': '/spoof'}), \
                    mock.patch.object(gate.browser_tools, '_build_harness', return_value=driver) as build, \
                    mock.patch.object(gate, 'run_checks') as checks:
                gate.run()
        build.assert_called_once_with('face-completeness-web')
        checks.assert_called_once_with(driver)

    def test_build_failure_never_runs_checks(self):
        with mock.patch.object(gate.browser_tools, '_build_harness', side_effect=SystemExit('build failed')), \
                mock.patch.object(gate, 'run_checks') as checks:
            with self.assertRaisesRegex(SystemExit, 'build failed'):
                gate.run()
        checks.assert_not_called()

    def test_missing_built_driver_fails_before_checks(self):
        with mock.patch.object(gate.browser_tools, '_build_harness', return_value=Path('/missing/face')), \
                mock.patch.object(gate, 'run_checks') as checks:
            with self.assertRaisesRegex(RuntimeError, 'driver.*missing'):
                gate.run()
        checks.assert_not_called()

    def test_controls_use_built_driver_not_spoofed_override(self):
        seen = []
        def check(_self):
            seen.append(os.environ['CAD_FACE_WEB_DRIVER'])
        cases = type('Cases', (unittest.TestCase,), {name: check for name in gate.CONTROLS})
        controls = SimpleNamespace(FaceWebTests=cases, face_web=SimpleNamespace(
            run=lambda: seen.append(os.environ['CAD_FACE_WEB_DRIVER'])))
        spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda module: None))
        with mock.patch.dict(os.environ, {'CAD_FACE_WEB_DRIVER': '/spoof'}), \
                mock.patch.object(gate.importlib.util, 'spec_from_file_location', return_value=spec), \
                mock.patch.object(gate.importlib.util, 'module_from_spec', return_value=controls), \
                contextlib.redirect_stderr(io.StringIO()):
            gate.run_checks(Path('/built/face'))
            self.assertEqual(os.environ['CAD_FACE_WEB_DRIVER'], '/spoof')
        self.assertEqual(seen, ['/built/face'] * (len(gate.CONTROLS) + 1))

    def test_skipped_required_control_fails(self):
        @unittest.skip('not available')
        def skipped(_self):
            pass
        cases = type('Cases', (unittest.TestCase,), {name: skipped for name in gate.CONTROLS})
        controls = SimpleNamespace(FaceWebTests=cases, face_web=SimpleNamespace(run=lambda: None))
        spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda module: None))
        with mock.patch.object(gate.importlib.util, 'spec_from_file_location', return_value=spec), \
                mock.patch.object(gate.importlib.util, 'module_from_spec', return_value=controls), \
                contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaisesRegex(RuntimeError, 'skipped'):
                gate.run_checks(Path('/built/face'))

    def test_inventory_keeps_other_node_regressions(self):
        import cad_build
        original = {
            'step-repair-accounting.cjs', 'step-zero-repair.cjs',
            'step-projected-line-repair.cjs', 'step-planar-seam-repair.cjs',
            'step-face-completeness.cjs', 'step-complete-surface-preview.cjs',
            'collision-analysis.cjs', 'qualified-analysis.cjs',
        }
        self.assertEqual(set(cad_build.PROVIDER_TESTS) | {'step-face-completeness.cjs'}, original)
        self.assertNotIn('step-face-completeness.cjs', cad_build.PROVIDER_TESTS)


if __name__ == '__main__':
    unittest.main()
