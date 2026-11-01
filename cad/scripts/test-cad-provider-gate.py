#!/usr/bin/env python3
"""Hermetic checks for the required packaged-provider gate."""
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
spec = importlib.util.spec_from_file_location('provider_gate', HERE / 'test-cad-provider.py')
gate = importlib.util.module_from_spec(spec)
spec.loader.exec_module(gate)


class ProviderGateTests(unittest.TestCase):
    def test_controls_and_hermetic_tests_are_registered(self):
        spec = importlib.util.spec_from_file_location('provider_tests', HERE / 'test-provider-pages.py')
        tests = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(tests)
        self.assertEqual(set(gate.CONTROLS), set(tests.BROWSER_TESTS))
        self.assertEqual(set(tests.BROWSER_TESTS), set(unittest.defaultTestLoader.getTestCaseNames(tests.ProviderPagesBrowserTests)))
        self.assertEqual(set(tests.HERMETIC_TESTS), set(unittest.defaultTestLoader.getTestCaseNames(tests.ProviderPagesTests)))
        self.assertEqual(len(tests.HERMETIC_TESTS), 6)
        self.assertEqual(len(gate.CONTROLS), 4)

    def test_build_and_missing_driver_fail_before_checks(self):
        for result in (SystemExit('build failed'), Path('/missing/provider-driver')):
            with self.subTest(result=result), mock.patch.object(gate.browser_tools, '_build_harness', side_effect=[result]) as build, \
                    mock.patch.object(gate, 'run_checks') as checks:
                with self.assertRaises((SystemExit, RuntimeError)):
                    gate.run()
                build.assert_called_once_with('cad-provider-pages')
                checks.assert_not_called()
        with tempfile.TemporaryDirectory() as directory:
            driver = Path(directory) / 'driver'
            driver.touch()
            with mock.patch.object(gate.browser_tools, '_build_harness', return_value=driver) as build, \
                    mock.patch.object(gate, 'run_checks') as checks:
                gate.run()
            build.assert_called_once_with('cad-provider-pages')
            checks.assert_called_once_with(driver)

    def test_trusted_driver_is_used_before_import_and_in_controls(self):
        seen = []
        def check(_self):
            seen.append(os.environ['CAD_PROVIDER_PAGES_DRIVER'])
        cases = type('Cases', (unittest.TestCase,), {name: check for name in gate.CONTROLS})
        controls = SimpleNamespace(ProviderPagesBrowserTests=cases, pages=SimpleNamespace(
            run=lambda driver: seen.append((str(driver), os.environ['CAD_PROVIDER_PAGES_DRIVER']))))
        def load(_module):
            seen.append(os.environ['CAD_PROVIDER_PAGES_DRIVER'])
        spec = SimpleNamespace(loader=SimpleNamespace(exec_module=load))
        with mock.patch.dict(os.environ, {'CAD_PROVIDER_PAGES_DRIVER': '/spoof'}), \
                mock.patch.object(gate.importlib.util, 'spec_from_file_location', return_value=spec), \
                mock.patch.object(gate.importlib.util, 'module_from_spec', return_value=controls), \
                contextlib.redirect_stderr(io.StringIO()):
            gate.run_checks(Path('/built/driver'))
            self.assertEqual(os.environ['CAD_PROVIDER_PAGES_DRIVER'], '/spoof')
        self.assertEqual(seen, ['/built/driver', ('/built/driver', '/built/driver'), *(['/built/driver'] * 4)])

    def test_control_skip_expected_failure_and_count_mismatch_fail(self):
        def check(_self):
            pass
        for method in (unittest.skip('unavailable')(check), unittest.expectedFailure(lambda _self: self.fail()), check):
            cases = type('Cases', (unittest.TestCase,), {name: method for name in gate.CONTROLS})
            controls = SimpleNamespace(ProviderPagesBrowserTests=cases, pages=SimpleNamespace(run=lambda **_kwargs: None))
            spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda _module: None))
            runner = mock.patch.object(gate.unittest.TextTestRunner, 'run', return_value=SimpleNamespace(
                wasSuccessful=lambda: True, skipped=[], expectedFailures=[], testsRun=3)) if method is check else contextlib.nullcontext()
            with self.subTest(method=method), mock.patch.object(gate.importlib.util, 'spec_from_file_location', return_value=spec), \
                    mock.patch.object(gate.importlib.util, 'module_from_spec', return_value=controls), runner, \
                    contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, 'failed or were skipped'):
                gate.run_checks(Path('/built/driver'))

    def test_regression_failure_stops_before_controls(self):
        controls = SimpleNamespace(pages=SimpleNamespace(run=mock.Mock(side_effect=ValueError('malformed output'))))
        spec = SimpleNamespace(loader=SimpleNamespace(exec_module=lambda _module: None))
        with mock.patch.object(gate.importlib.util, 'spec_from_file_location', return_value=spec), \
                mock.patch.object(gate.importlib.util, 'module_from_spec', return_value=controls), \
                self.assertRaisesRegex(ValueError, 'malformed output'):
            gate.run_checks(Path('/built/driver'))


if __name__ == '__main__':
    unittest.main()
