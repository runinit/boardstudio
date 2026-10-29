#!/usr/bin/env python3
"""Hermetic and real-browser checks for the provider-page trust boundaries."""
import contextlib
import hashlib
import io
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock
from urllib.error import HTTPError
from urllib.request import urlopen

import provider_pages as pages
import cad_build


class ProviderPagesTests(unittest.TestCase):
    def test_preflight_rejects_missing_and_stale_package(self):
        with mock.patch.object(Path, 'is_file', return_value=False):
            with self.assertRaisesRegex(RuntimeError, 'missing'):
                pages.preflight()
        with mock.patch.object(Path, 'is_file', return_value=True), \
             mock.patch.object(Path, 'read_text', return_value='old'), \
             mock.patch.object(cad_build, 'wasm_inputs_digest', return_value='new'):
            with self.assertRaisesRegex(RuntimeError, 'stale'):
                pages.preflight()

        face = pages.load_module('face_web_preflight_control', pages.CAD / 'scripts/face-completeness-web.py')
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / 'wasm/pkg/boardstudio_cad_wasm.js'
            wasm = package.with_name('boardstudio_cad_wasm_bg.wasm')
            stamp = root / '.cache/cad/wasm-build.sha256'
            package.parent.mkdir(parents=True)
            stamp.parent.mkdir(parents=True)
            for path in (package, wasm, stamp):
                path.write_text('fresh')
            with mock.patch.object(cad_build, 'wasm_inputs_digest', return_value='fresh'):
                self.assertEqual(face.preflight(root), (package, wasm))
                for path in (package, wasm, stamp):
                    path.unlink()
                    with self.subTest(missing=path), self.assertRaisesRegex(RuntimeError, 'missing'):
                        face.preflight(root)
                    path.write_text('fresh')

    def test_inventory_is_exact_and_boolean(self):
        good = {suite: [{'suite': suite, 'case': case, 'ok': True} for case in cases]
                for suite, cases in pages.SUITES.items()}
        pages.validate_results(good, list(pages.SUITES))
        for entries in ([], good['collision-analysis'] * 2,
                        [{'suite': 'collision-analysis', 'case': 'other', 'ok': True}],
                        [{'suite': 'collision-analysis', 'case': 'main', 'ok': 1}],
                        [{'suite': 'collision-analysis', 'case': 'main', 'ok': True, 'error': 'bad'}]):
            with self.subTest(entries=entries), self.assertRaises(RuntimeError):
                pages.validate_results({'collision-analysis': entries}, ['collision-analysis'])
        with self.assertRaises(RuntimeError):
            pages.validate_results({**good, 'extra': []}, ['collision-analysis'])
        with self.assertRaisesRegex(ValueError, 'duplicate result key'):
            json.loads('{"ok": false, "ok": true}', object_pairs_hook=pages.unique_object)
        for suite in good:
            with self.subTest(suite=suite), self.assertRaises(RuntimeError):
                pages.validate_results({k: v for k, v in good.items() if k != suite}, list(good))

    def test_suite_assertion_sites_and_fixture_paths(self):
        pattern = r'\bassert(?:\.[A-Za-z]+)?\s*\('
        ports = {'collision-analysis': 66, 'step-repair-accounting': 45,
                 'step-zero-repair': 53, 'step-projected-line-repair': 16,
                 'step-planar-seam-repair': 16, 'step-complete-surface-preview': 39,
                 'qualified-analysis': 102}
        self.assertEqual(pages.STATIC_ASSERT_SITES, ports)
        fixtures = set()
        self.assertEqual(set(ports), set(pages.SUITES) - {'provider-support-controls'})
        for suite, count in ports.items():
            with self.subTest(suite=suite):
                port = (pages.TESTS / (suite + '.mjs')).read_text()
                self.assertEqual(len(re.findall(pattern, port)), count)
                fixtures.update(re.findall(r"(?:readBytes|readText|fixture)\(['\"]([^'\"]+)['\"]", port))
        self.assertEqual(fixtures, set(pages.FIXTURES))
        for name, path in pages.FIXTURES.items():
            self.assertEqual(path, pages.ROOT / name)
            self.assertTrue(path.is_file(), name)
        self.assertIn('asset.analysisHandle.free()', (pages.TESTS / 'collision-analysis.mjs').read_text())

    def test_driver_results_fail_closed(self):
        good = {'collision-analysis': [{'suite': 'collision-analysis', 'case': 'main', 'ok': True}]}
        responses = [(0, json.dumps(good), ''), (9, json.dumps(good), 'driver diagnostic'),
                     (0, 'not json', ''), (0, '{"collision-analysis":[],"collision-analysis":[]}', ''),
                     (0, '{"collision-analysis":[{"suite":"collision-analysis","case":"main","ok":false,"ok":true}]}', ''),
                     (0, json.dumps({'collision-analysis': good['collision-analysis'] * 2}), ''),
                     (0, json.dumps({'collision-analysis': [{'suite': 'collision-analysis', 'case': 'main', 'ok': 1}]}), '')]
        with tempfile.TemporaryDirectory() as directory:
            package, wasm = (Path(directory) / name for name in ('package.js', 'package.wasm'))
            package.write_bytes(b'package')
            wasm.write_bytes(b'wasm')
            for code, stdout, stderr in responses:
                with self.subTest(code=code, stdout=stdout), \
                     mock.patch.object(pages, 'preflight', return_value=(package, wasm)), \
                     mock.patch.object(pages, 'load_module', return_value=SimpleNamespace(
                         run_probe=lambda command, timeout: SimpleNamespace(returncode=code, stdout=stdout, stderr=stderr))), \
                     contextlib.redirect_stderr(io.StringIO()):
                    if code == 0 and stdout == json.dumps(good):
                        pages.run(['collision-analysis'], driver='/fake')
                    else:
                        with self.assertRaises((ValueError, RuntimeError)) as error:
                            pages.run(['collision-analysis'], driver='/fake')
                        if code:
                            self.assertIn('driver diagnostic', str(error.exception))
                            self.assertIn(stdout, str(error.exception))

    def test_server_allowlist_headers_sha_and_failure_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            package, wasm, fixture = (Path(directory) / name for name in ('package.js', 'package.wasm', 'fixture'))
            package.write_bytes(b'package')
            wasm.write_bytes(b'wasm')
            fixture.write_bytes(b'fixture bytes')

            def probe(command, timeout):
                self.assertEqual(timeout, 900)
                root = command[1]
                with urlopen(root + 'fixture/allowed') as response:
                    self.assertEqual(response.read(), b'fixture bytes')
                    self.assertEqual(response.headers['X-Fixture-SHA256'], hashlib.sha256(b'fixture bytes').hexdigest())
                    self.assertEqual(response.headers['Cross-Origin-Opener-Policy'], 'same-origin')
                    self.assertEqual(response.headers['Cross-Origin-Embedder-Policy'], 'require-corp')
                    self.assertEqual(response.headers['Cross-Origin-Resource-Policy'], 'same-origin')
                    self.assertEqual(response.headers['Cache-Control'], 'no-store')
                for route in ('fixture/unknown', 'fixture/../allowed', 'tests/unknown.mjs'):
                    with self.assertRaises(HTTPError) as error:
                        urlopen(root + route)
                    self.assertEqual(error.exception.code, 404)
                    self.assertEqual(error.exception.headers['Cross-Origin-Embedder-Policy'], 'require-corp')
                    error.exception.close()
                fixture.unlink()
                with self.assertRaises(HTTPError) as error:
                    urlopen(root + 'fixture/allowed')
                self.assertEqual(error.exception.code, 500)
                error.exception.close()
                return SimpleNamespace(returncode=7, stderr='driver failure', stdout='')

            fake = SimpleNamespace(run_probe=probe)
            with mock.patch.object(pages, 'preflight', return_value=(package, wasm)), \
                 mock.patch.object(pages, 'FIXTURES', {'allowed': fixture}), \
                 mock.patch.object(pages, 'load_module', return_value=fake):
                with self.assertRaisesRegex(RuntimeError, 'driver failure'):
                    pages.run(['collision-analysis'], driver='/fake')

    def test_bad_selections_fail_before_preflight(self):
        with mock.patch.object(pages, 'preflight') as preflight:
            for suites in ([], ['nope'], ['collision-analysis', 'collision-analysis']):
                with self.assertRaises(ValueError):
                    pages.run(suites)
            preflight.assert_not_called()


class ProviderPagesBrowserTests(unittest.TestCase):
    def setUp(self):
        self.driver = os.environ.get('CAD_PROVIDER_PAGES_DRIVER')
        if not self.driver:
            self.fail('CAD_PROVIDER_PAGES_DRIVER must name the trusted real browser driver')

    def run_modules(self, modules, run_js=None):
        read_bytes = Path.read_bytes

        def controlled_read(path):
            if path.parent == pages.TESTS and path.name in modules:
                return modules[path.name].encode()
            return read_bytes(path)

        with mock.patch.object(Path, 'read_bytes', controlled_read), \
             mock.patch.object(pages, 'RUN_JS', pages.RUN_JS if run_js is None else run_js), \
             contextlib.redirect_stderr(io.StringIO()):
            pages.run([name.removesuffix('.mjs') for name in modules], driver=self.driver)

    def test_batch_failures_and_later_suite(self):
        modules = {
            'provider-support-controls.mjs': "export async function run() { return [{suite:'provider-support-controls',case:'truth',ok:1}]; }",
            'collision-analysis.mjs': "import {assert, runCases} from './provider-support.mjs'; export async function run() { return runCases('collision-analysis', [['main', () => assert.equal(1, 2, 'controlled assertion')]]); }",
            'step-repair-accounting.mjs': "export async function run() { throw Error('controlled rejection'); }",
            'step-zero-repair.mjs': "export async function run() { return [{suite:'step-zero-repair',case:'same',ok:true},{suite:'step-zero-repair',case:'same',ok:true}]; }",
            'step-projected-line-repair.mjs': "export async function run() { const done = new Promise(r => window.addEventListener('error', r, {once:true})); setTimeout(() => { throw Error('detached exception'); }, 0); await done; return [{suite:'step-projected-line-repair',case:'main',ok:true}]; }",
            'step-planar-seam-repair.mjs': "export async function run() { const done = new Promise(r => window.addEventListener('unhandledrejection', r, {once:true})); setTimeout(() => { Promise.reject(Error('detached rejection')); }, 0); await done; return [{suite:'step-planar-seam-repair',case:'main',ok:true}]; }",
            'step-complete-surface-preview.mjs': "export async function run() { console.log('controlled output'); throw Error('after output'); }",
            'qualified-analysis.mjs': "export async function run() { return [{suite:'qualified-analysis',case:'main',ok:true}]; }",
        }
        self.assertEqual(pages.RUN_JS.count(b"'/output'"), 1)
        with self.assertRaises(RuntimeError) as error:
            self.run_modules(modules, pages.RUN_JS.replace(b"'/output'", b"'/invalid-output'"))
        text = str(error.exception)
        for expected in ('controlled assertion', 'ERR_ASSERTION', 'actual: 1', 'expected: 2',
                         'controlled rejection', 'invalid suite/case', 'invalid result for provider-support-controls/truth', 'detached exception',
                         'detached rejection', 'cannot forward regression output', 'qualified-analysis'):
            self.assertIn(expected, text)
        self.assertNotIn('after output', text)
        results = json.loads(text.split('\nstdout: ', 1)[1])
        self.assertEqual(results['qualified-analysis'], [{'suite': 'qualified-analysis', 'case': 'main', 'ok': True}])

    def test_broken_stdout_fails_before_later_statement(self):
        class BrokenOutput(io.StringIO):
            attempts = 0

            def write(self, text):
                self.attempts += 1
                raise BrokenPipeError('controlled broken stdout')

        modules = {
            'collision-analysis.mjs': "export async function run() { console.log('controlled output'); throw Error('after output'); }",
            'qualified-analysis.mjs': "export async function run() { return [{suite:'qualified-analysis',case:'main',ok:true}]; }",
        }
        output = BrokenOutput()
        with contextlib.redirect_stdout(output), self.assertRaises(RuntimeError) as error:
            self.run_modules(modules)
        self.assertGreater(output.attempts, 0)
        self.assertIn('cannot forward regression output', str(error.exception))
        self.assertNotIn('after output', str(error.exception))
        self.assertIn('qualified-analysis', json.loads(str(error.exception).split('\nstdout: ', 1)[1]))

    def test_fixture_and_corrupt_wasm_initialization_order(self):
        read_bytes = Path.read_bytes
        fixture = 'generated-analysis-contract.json'
        for corrupt in (False, True):
            reads = []

            def controlled_read(path):
                if path.name == fixture:
                    reads.append(str(path))
                    raise FileNotFoundError('controlled missing fixture')
                if corrupt and path.name == 'boardstudio_cad_wasm_bg.wasm':
                    return b'corrupt wasm'
                return read_bytes(path)

            with self.subTest(corrupt=corrupt), mock.patch.object(Path, 'read_bytes', controlled_read), \
                 contextlib.redirect_stderr(io.StringIO()), self.assertRaises(RuntimeError) as error:
                pages.run(['collision-analysis'], driver=self.driver)
            if corrupt:
                self.assertEqual(reads, [])
                self.assertNotIn('controlled missing fixture', str(error.exception))
                self.assertRegex(str(error.exception).lower(), 'wasm|compile|magic')
            else:
                self.assertEqual(len(reads), 1)
                self.assertIn('controlled missing fixture', str(error.exception))

    def test_driver_rejects_invalid_startup(self):
        for args, env, expected in (
            (['http://localhost:1234/', 'collision-analysis'], {}, 'loopback'),
            (['http://127.0.0.1:1234/', 'collision-analysis', 'collision-analysis'], {}, 'distinct'),
            (['http://127.0.0.1:1234/', 'collision-analysis'], {'CHROME_BIN': '/nonexistent/chrome'}, 'cannot start Chrome'),
        ):
            with self.subTest(args=args, env=env):
                result = subprocess.run([self.driver, *args], env={**os.environ, **env},
                                        capture_output=True, text=True, timeout=30)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(result.stdout)
                self.assertTrue(result.stderr)
                self.assertIn(expected, result.stderr)


HERMETIC_TESTS = (
    'test_preflight_rejects_missing_and_stale_package',
    'test_inventory_is_exact_and_boolean',
    'test_suite_assertion_sites_and_fixture_paths',
    'test_driver_results_fail_closed',
    'test_server_allowlist_headers_sha_and_failure_exit',
    'test_bad_selections_fail_before_preflight',
)
BROWSER_TESTS = tuple(name for name in dir(ProviderPagesBrowserTests) if name.startswith('test_'))


if __name__ == '__main__':
    if sys.argv[1:] == ['--hermetic']:
        result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(ProviderPagesTests(name) for name in HERMETIC_TESTS))
        sys.exit(0 if result.wasSuccessful() and not result.skipped and not result.expectedFailures
                 and result.testsRun == len(HERMETIC_TESTS) else 1)
    unittest.main()
