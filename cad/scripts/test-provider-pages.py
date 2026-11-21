#!/usr/bin/env python3
"""Hermetic checks for the provider-page runner."""
import argparse
import hashlib
from pathlib import Path
import re
import tempfile
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
                self.assertEqual(pages.preflight(root), (package, wasm))
                for path in (package, wasm, stamp):
                    path.unlink()
                    with self.subTest(missing=path), self.assertRaisesRegex(RuntimeError, 'missing'):
                        pages.preflight(root)
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
        for suite in good:
            with self.subTest(suite=suite), self.assertRaises(RuntimeError):
                pages.validate_results({k: v for k, v in good.items() if k != suite}, list(good))

    def test_suite_assertion_sites_and_fixture_paths(self):
        pattern = r'\bassert(?:\.[A-Za-z]+)?\s*\('
        ports = {'collision-analysis': 66, 'step-repair-accounting': 45,
                 'step-zero-repair': 59, 'step-projected-line-repair': 16,
                 'step-planar-seam-repair': 16, 'step-complete-surface-preview': 39,
                 'qualified-analysis': 102, 'measure-cad-worker': 13}
        self.assertEqual(pages.STATIC_ASSERT_SITES, ports)
        fixtures = set()
        self.assertEqual(set(ports), set(pages.SUITES) - {'provider-support-controls', 'step-face-completeness'})
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

    def test_server_allowlist_headers_sha_and_failure_exit(self):
        with tempfile.TemporaryDirectory() as directory:
            package, wasm, fixture = (Path(directory) / name for name in ('package.js', 'package.wasm', 'fixture'))
            package.write_bytes(b'package')
            wasm.write_bytes(b'wasm')
            fixture.write_bytes(b'fixture bytes')

            def probe(root, suites, *, timeout):
                self.assertEqual(timeout, 900)
                self.assertEqual(suites, ['collision-analysis'])
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
                raise RuntimeError('driver failure')
            with mock.patch.object(pages, 'preflight', return_value=(package, wasm)), \
                 mock.patch.object(pages, 'FIXTURES', {'allowed': fixture}), \
                 mock.patch.object(pages.browser_tools, 'run_provider_page', side_effect=probe):
                with self.assertRaisesRegex(RuntimeError, 'driver failure'):
                    pages.run(['collision-analysis'])

    def test_bad_selections_fail_before_preflight(self):
        with mock.patch.object(pages, 'preflight') as preflight:
            for suites in ([], ['nope'], ['collision-analysis', 'collision-analysis']):
                with self.assertRaises(ValueError):
                    pages.run(suites)
            preflight.assert_not_called()


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--hermetic', action='store_true')
    parser.parse_args()
    unittest.main(argv=[__file__])
