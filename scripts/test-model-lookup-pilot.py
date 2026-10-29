#!/usr/bin/env python3
"""Hermetic controls for the experimental model-lookup page runner."""
import contextlib
import importlib.util
import io
import json
import os
from pathlib import Path
import re
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock
from urllib.error import HTTPError
from urllib.request import urlopen

spec = importlib.util.spec_from_file_location('pilot', Path(__file__).with_name('model-lookup-pilot.py'))
pilot = importlib.util.module_from_spec(spec)
spec.loader.exec_module(pilot)


EXPECTED_CASES = (
    'helper source uses no Node globals',
    'helper loads as one function declaration in a restricted classic worker',
    'the file text is exactly the function source',
    'exact pair selects from the large catalog',
    'selection carries the exact provider/model IDs and label',
    'selection carries the target options only',
    'rendered selection omits unrelated sentinel data',
    'rendered selection is compact',
    'a model ID shared by another provider resolves inside the requested provider',
    'a matching label under another provider cannot change the selection',
    'a repeated exact pair reports duplicate-match',
    'an unknown provider reports provider-not-found',
    'an unknown model reports model-not-found',
    'the three diagnostics are distinct',
    'diagnostics carry only ok/code/message',
    'validated JSON text is a fallback when structured content is absent',
    'a response carrying both forms is handled once',
    'a non-text block before the JSON block is skipped',
    'malformed fallback JSON reports malformed-text-json',
    'unexpected response shapes report unexpected-shape',
    'malformed present structured content is rejected rather than rescued by fallback',
    'malformed selected options are not reported as a valid empty option',
    'malformed selected provider models are not reported as a missing model',
    'a valid empty options list stays a success',
    'absent model options stay a success',
    'empty options are distinguishable from an error',
    'missing provider/model IDs report missing-input',
    'missing-input stays distinct from unexpected-shape',
    'no representation or missing-input diagnostic leaks sentinel data',
    'the recipe probe literal matches the tested helper',
    'the documented behavior probe stays in sync with the tested literal',
    'boundary-controls',
)


class PilotTests(unittest.TestCase):
    def test_inventory_is_the_exact_reviewed_list(self):
        self.assertEqual(pilot.CASES, EXPECTED_CASES)
        self.assertEqual(len(set(EXPECTED_CASES)), 32)

    def test_inventory_still_covers_original_node_checks_while_they_exist(self):
        original = pilot.ROOT / 'scripts/test-model-lookup.cjs'
        if original.exists():  # transitional cross-check; the literal above is authoritative
            names = re.findall(r"check\('([^']+)'", original.read_text())
            names[1] = 'helper loads as one function declaration in a restricted classic worker'
            self.assertEqual(tuple(names) + ('boundary-controls',), pilot.CASES)

    def test_exact_inventory(self):
        good = {pilot.SUITE: [{'suite': pilot.SUITE, 'case': name, 'ok': True} for name in pilot.CASES]}
        pilot.validate_results(json.dumps(good))
        flipped = [dict(entry) for entry in good[pilot.SUITE]]
        flipped[3].update(ok=False, error='failed')
        bad = [None, {}, {pilot.SUITE: []}, {pilot.SUITE: good[pilot.SUITE][::-1]},
               {pilot.SUITE: good[pilot.SUITE][:-1]}, {pilot.SUITE: flipped}, {pilot.SUITE: good[pilot.SUITE] + good[pilot.SUITE]},
               {pilot.SUITE: [{**entry, 'ok': 1} for entry in good[pilot.SUITE]]},
               {pilot.SUITE: [{**entry, 'error': 'hidden'} for entry in good[pilot.SUITE]]},
               {**good, 'extra': []}]
        for result in bad:
            with self.subTest(result=result), self.assertRaises(RuntimeError):
                pilot.validate_results(json.dumps(result))
        with self.assertRaisesRegex(ValueError, 'duplicate result key'):
            pilot.validate_results('{"x":1,"x":2}')

    def test_allowlisted_routes_and_driver_failure(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source.js'
            source.write_text('controlled source')
            driver = Path(directory) / 'driver'
            driver.write_text('#!/bin/sh\n')
            driver.chmod(0o755)

            def run_probe(command, timeout):
                self.assertEqual(command[0], str(driver))
                self.assertEqual(command[2:], [pilot.SUITE])
                self.assertEqual(timeout, pilot.TIMEOUT)
                with urlopen(command[1] + 'model-lookup.js') as response:
                    self.assertEqual(response.read(), b'controlled source')
                    self.assertEqual(response.headers['Cache-Control'], 'no-store')
                with urlopen(command[1] + 'model-lookup.md') as response:
                    self.assertEqual(response.read(), (pilot.ROOT / 'docs/agents/model-lookup.md').read_bytes())
                with urlopen(command[1] + 'model-lookup-pilot-worker.js') as response:
                    self.assertEqual(response.headers['Content-Security-Policy'], "default-src 'none'; script-src 'self'")
                    self.assertTrue(response.read())
                for path in ('missing', 'fixture/../model-lookup.js', 'model-lookup-pilot.py'):
                    with self.subTest(path=path), self.assertRaises(HTTPError) as error:
                        urlopen(command[1] + path)
                    self.assertEqual(error.exception.code, 404)
                    error.exception.close()
                return SimpleNamespace(returncode=7, stdout='diagnostic stdout', stderr='driver error')

            with mock.patch.object(pilot.probe, 'run_probe', run_probe), \
                 contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, 'driver error') as error:
                pilot.run(source, driver)
            self.assertIn('diagnostic stdout', str(error.exception))

    def test_driver_must_be_an_absolute_executable_file(self):
        with self.assertRaisesRegex(RuntimeError, 'CAD_PROVIDER_PAGES_DRIVER'):
            pilot.run('/nonexistent/source', None)
        with tempfile.TemporaryDirectory() as directory:
            plain = Path(directory) / 'plain'
            plain.write_text('')
            relative = Path(directory) / 'relative-driver'
            relative.write_text('#!/bin/sh\n')
            relative.chmod(0o755)
            cwd = os.getcwd()
            os.chdir(directory)
            try:
                with self.assertRaisesRegex(RuntimeError, 'absolute path to an executable'):
                    pilot.run('/nonexistent/source', 'relative-driver')
            finally:
                os.chdir(cwd)
            for driver in (directory, plain, Path(directory) / 'absent'):
                with self.subTest(driver=driver), self.assertRaisesRegex(RuntimeError, 'absolute path to an executable'):
                    pilot.run('/nonexistent/source', driver)


class PilotBrowserTests(unittest.TestCase):
    def test_browser_failures_are_fatal(self):
        driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
        self.assertTrue(driver, 'CAD_PROVIDER_PAGES_DRIVER is required')
        original = Path.read_bytes
        controls = [
            ("export async function run() { throw Error('controlled rejection'); }", 'controlled rejection'),
            ("export async function run(suite) { return [{suite,case:'source',ok:1}]; }", 'invalid result'),
            ("export async function run(suite) { const done = new Promise(r => window.addEventListener('error', r, {once:true})); setTimeout(() => { throw Error('detached error'); }, 0); await done; return [{suite,case:'source',ok:true}]; }", 'detached error'),
        ]
        for module, expected in controls:
            def controlled_read(path):
                return module.encode() if path.name == 'model-lookup-pilot.mjs' else original(path)

            with self.subTest(expected=expected), mock.patch.object(Path, 'read_bytes', controlled_read), \
                 contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, expected):
                pilot.run(pilot.ROOT / 'scripts/model-lookup.js', driver)

    def test_worker_failures_are_fatal(self):
        driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
        self.assertTrue(driver, 'CAD_PROVIDER_PAGES_DRIVER is required')
        original = Path.read_bytes
        controls = [
            ("throw Error('worker startup failure');", 'worker startup failure'),
            ("postMessage({error: 'controlled worker failure'});", 'controlled worker failure'),
            ("postMessage({error: 'first failure wins'}); postMessage([]);", 'first failure wins'),
            ("postMessage([]);", 'invalid results'),
            ("// no result", 'worker timeout'),
        ]
        for worker, expected in controls:
            def controlled_read(path):
                return worker.encode() if path.name == 'model-lookup-pilot-worker.js' else original(path)

            with self.subTest(expected=expected), mock.patch.object(Path, 'read_bytes', controlled_read), \
                 contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, expected):
                pilot.run(pilot.ROOT / 'scripts/model-lookup.js', driver)

    def test_mutated_helper_is_fatal(self):
        driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
        self.assertTrue(driver, 'CAD_PROVIDER_PAGES_DRIVER is required')
        helper = (pilot.ROOT / 'scripts/model-lookup.js').read_text()
        controls = [
            (helper + "\nPromise.reject(Error('helper rejection control'));\n", 'helper rejection control'),
            (helper + "\n(async () => { await null; await null; await null; throw Error('deep helper rejection'); })();\n",
             'deep helper rejection'),
            (helper + "\nvar leaked = 1;\n", 'unexpected global after helper load'),
            ('// leading comment\n' + helper, 'expected a function declaration'),
            (helper.replace("'duplicate-match'", "'duplicate'"), 'a repeated exact pair reports duplicate-match'),
            (helper.replace("ok: true,\n    providerInstanceId,", "ok: true, extra: 1,\n    providerInstanceId,"), 'rendered selection is compact'),
        ]
        for text, expected in controls:
            with self.subTest(expected=expected), tempfile.TemporaryDirectory() as directory:
                source = Path(directory) / 'source.js'
                source.write_text(text)
                self.assertNotEqual(text, helper)
                with contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, expected):
                    pilot.run(source, driver)

    def test_recipe_probe_drift_is_fatal(self):
        driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
        self.assertTrue(driver, 'CAD_PROVIDER_PAGES_DRIVER is required')
        with tempfile.TemporaryDirectory() as directory:
            recipe = Path(directory) / 'recipe.md'
            text = (pilot.ROOT / 'docs/agents/model-lookup.md').read_text()
            self.assertIn('"code":"duplicate-match"', text)
            recipe.write_text(text.replace('"code":"duplicate-match"', '"code":"duplicate"'))
            with contextlib.redirect_stderr(io.StringIO()), self.assertRaisesRegex(RuntimeError, 'documented behavior probe stays in sync'):
                pilot.run(pilot.ROOT / 'scripts/model-lookup.js', driver, recipe)


if __name__ == '__main__':
    unittest.main()
