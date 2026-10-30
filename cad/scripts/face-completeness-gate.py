#!/usr/bin/env python3
"""Run the required face regression and its browser controls with the built harness."""
import importlib.util
import os
from pathlib import Path
import sys
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / 'scripts'))
import browser_tools

CONTROLS = (
    'test_browser_assertion_diagnostics_reach_consumer',
    'test_real_browser_failure_controls',
    'test_output_survives_failure_after_original_log',
    'test_output_transport_and_broken_stdout_fail_before_release',
    'test_fixture_and_wasm_failure_order',
    'test_detached_errors_before_completion_fail',
)


def run_checks(driver):
    # The explicit development override must never select the gate's executable.
    with patch.dict(os.environ, {'CAD_FACE_WEB_DRIVER': str(driver)}):
        path = Path(__file__).with_name('test-face-completeness-web.py')
        spec = importlib.util.spec_from_file_location('face_controls', path)
        controls = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(controls)
        controls.face_web.run()
        suite = unittest.TestSuite(controls.FaceWebTests(name) for name in CONTROLS)
        result = unittest.TextTestRunner(verbosity=2).run(suite)
        if (not result.wasSuccessful() or result.skipped or result.expectedFailures
                or result.testsRun != len(CONTROLS)):
            raise RuntimeError('required face browser controls failed or were skipped')


def run():
    driver = browser_tools._build_harness('face-completeness-web')
    if not driver.is_file():
        raise RuntimeError(f'face browser driver missing: {driver}')
    run_checks(driver)


if __name__ == '__main__':
    try:
        run()
    except (OSError, ValueError, RuntimeError, browser_tools.MissingBrowserTool) as error:
        sys.exit(str(error))
