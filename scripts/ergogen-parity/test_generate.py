#!/usr/bin/env python3
"""Node-free contract checks for generate.py, the replacement of scripts/test-ergogen-parity.cjs.

--hermetic: offline groups, no upstream checkout, browser, npm or Cargo.
--source:   evaluates cases.cjs in Chrome (driver built from this checkout) and rebuilds the
            committed fixture byte-for-byte from its recorded upstream poses; no upstream code.
--upstream: opt-in real pinned parser; set ERGOGEN_CHECKOUT (and optionally ERGOGEN_NPM_CACHE).
"""
import contextlib
import io
import json
import math
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate
import host_probe
import verified_runtime

SCRIPT = Path(__file__).with_name('generate.py')
TEXT = generate.FIXTURE.read_text(encoding='utf-8')
TRANSLATED = {
    'id': 'translated-and-turned',
    'boardStudioMatrices': [{'id': 'main', 'rows': 1, 'columns': 1}],
    'mapping': [{'ergogenName': 'm_a_r0', 'matrix': 'main', 'row': 0, 'column': 0}],
}
POINT = {'m_a_r0': {'x': 10, 'y': 20, 'r': 30}}


def fixture():
    return json.loads(TEXT)


def cli(*args, **kwargs):
    env = {'PATH': '/usr/bin:/bin', 'HOME': os.environ.get('TMPDIR', '/tmp'), 'TMPDIR': os.environ.get('TMPDIR', '/tmp')}
    return subprocess.run([sys.executable, '-I', str(SCRIPT), *args], capture_output=True, text=True, env=env, **kwargs)


def git(*args):
    env = {'PATH': '/usr/bin:/bin', 'GIT_CONFIG_NOSYSTEM': '1', 'GIT_CONFIG_GLOBAL': os.devnull}
    subprocess.run(['git', '-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgsign=false', *args],
                   check=True, capture_output=True, env=env)


class GeneratorContractTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix='boardstudio-parity-')
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.checkout = self.root / 'checkout'
        self.checkout.mkdir()
        self.output = self.root / 'fixture.json'

    def test_translated_point_maps_to_its_matrix_coordinate(self):
        self.assertEqual(generate.collect_expected(TRANSLATED, POINT), [
            {'ergogenName': 'm_a_r0', 'matrix': 'main', 'row': 0, 'column': 0, 'pose': {'x': 10, 'y': 20, 'rotation': 30}}])

    def test_duplicate_point_mappings_are_refused(self):
        with self.assertRaisesRegex(ValueError, '(?i)duplicate.*point'):
            generate.collect_expected({**TRANSLATED, 'mapping': TRANSLATED['mapping'] * 2}, POINT)

    def test_upstream_point_cannot_disappear(self):
        with self.assertRaisesRegex(ValueError, '(?i)unmapped.*extra'):
            generate.collect_expected(TRANSLATED, {**POINT, 'extra': {'x': 0, 'y': 0, 'r': 0}})

    def test_missing_nonfinite_aliased_and_absent_points(self):
        mapping = TRANSLATED['mapping'][0]
        for points, pattern in (({}, 'missing.*m_a_r0'), ({'m_a_r0': {'x': math.nan, 'y': 20, 'r': 30}}, 'finite'),
                                ({'m_a_r0': {'x': None, 'y': 20, 'r': 30}}, 'finite'),
                                ({'m_a_r0': {'x': True, 'y': 20, 'r': 30}}, 'finite')):
            with self.subTest(points=points), self.assertRaisesRegex(ValueError, '(?i)' + pattern):
                generate.collect_expected(TRANSLATED, points)
        with self.assertRaisesRegex(ValueError, '(?i)duplicate.*coordinate'):
            generate.collect_expected({**TRANSLATED, 'mapping': [mapping, {**mapping, 'ergogenName': 'other'}]},
                                      {**POINT, 'other': {'x': 0, 'y': 0, 'r': 0}})
        skipped = {**TRANSLATED, 'mapping': [{**mapping, 'absent': True}]}
        self.assertEqual(generate.collect_expected(skipped, {}), skipped['mapping'])
        for scenario, points, pattern in (
                (skipped, POINT, 'absent'),
                ({**TRANSLATED, 'mapping': [{**mapping, 'absent': 'no'}]}, {}, 'boolean'),
                ({**TRANSLATED, 'boardStudioMatrices': [{'id': 'main', 'rows': 2, 'columns': 1}]}, POINT, 'missing matrix coordinate'),
                ({**TRANSLATED, 'mapping': [{**mapping, 'row': 1}]}, POINT, 'invalid mapped coordinate'),
                ({**TRANSLATED, 'mapping': [{**mapping, 'row': 0.5}]}, POINT, 'invalid mapped coordinate'),
                ({**TRANSLATED, 'boardStudioMatrices': [{'id': 'main', 'rows': True, 'columns': 1}]}, POINT, 'matrix dimensions')):
            with self.subTest(pattern=pattern), self.assertRaisesRegex(ValueError, '(?i)' + pattern):
                generate.collect_expected(scenario, points)

    def test_committed_fixture_is_complete_finite_and_pinned(self):
        value = fixture()
        generate.validate_fixture(value)
        self.assertEqual(value['source']['commit'], '8286e18f18ac064aa6d2ebca9c449bf2f6ba4d37')
        self.assertEqual(value['source']['dependencies'], {'makerjs': '0.18.2', 'mathjs': '15.2.0'})
        self.assertEqual(len(value['cases']), 42)
        self.assertEqual(sum(len(generate.recorded_points(item)) for item in value['cases']), 360)
        for mask in ('none', 'x', 'y', 'r', 'xy', 'xr', 'yr', 'xyr'):
            self.assertTrue(any(item['id'] == f'anchor-follow-{mask}' for item in value['cases']))
        point = lambda case, name: next(e for e in next(i for i in value['cases'] if i['id'] == case)['expected']
                                        if e['ergogenName'] == name)
        self.assertEqual(point('translated-and-turned', 'm_a_r0')['pose'], {'x': 10, 'y': 20, 'rotation': 30})
        self.assertEqual(point('full-heading-not-modulo-180', 'mirror_m_a_r0')['pose']['rotation'], -315)
        self.assertIs(point('skipped-key-carries-and-anchors', 'm_b_r1')['absent'], True)
        self.assertEqual(point('skipped-key-carries-and-anchors', 'probe_a_r0')['pose'], {'x': 21, 'y': 19, 'rotation': 10})
        self.assertEqual(point('linked-partial-follow-reflects-zero', 'mirror_thumb_a_r0')['pose'], {'x': 100, 'y': 30, 'rotation': 0})
        self.assertEqual(point('S15-additive-offset-divergence', 'm_b_r1')['pose']['x'], 19)
        divergence = next(i for i in value['cases'] if i['id'] == 'S15-additive-offset-divergence')
        self.assertEqual(divergence['differences'][0]['pose']['x'], 23)
        padding = next(i for i in value['cases'] if i['id'] == 'S7-padding')
        self.assertEqual(padding['ergogenConfig']['points']['zones']['m']['rows']['r1']['padding'], 22)
        self.assertEqual(padding['boardStudioMatrices'][0]['rowLayout'][2]['padding'], 22)
        for corrupt, pattern in (
                (lambda f: f['boardStudioOnly'][0]['expected'][0]['pose'].update(x=math.nan), 'finite'),
                (lambda f: f['cases'][0]['expected'][0].update(column=1), 'match|coordinate'),
                (lambda f: f['cases'][1].update(id=f['cases'][0]['id']), 'identity'),
                (lambda f: f.update(fixtureVersion=True), 'fixtureVersion'),
                (lambda f: f['source'].update(commit='0' * 40), 'source'),
                (lambda f: divergence_of(f)['differences'][0]['pose'].update(x=19), 'divergence'),
                (lambda f: f['boardStudioOnly'][0].update(basis=''), 'basis'),
                (lambda f: f['cases'][0].update(comparison='close'), 'comparison')):
            changed = fixture()
            corrupt(changed)
            with self.subTest(pattern=pattern), self.assertRaisesRegex(ValueError, '(?i)' + pattern):
                generate.validate_fixture(changed)

    def test_render_is_json_stringify_exact(self):
        self.assertEqual(generate.render(fixture()), TEXT)
        for value, text in ((1e-6, '0.000001'), (1e-7, '1e-7'), (1e21, '1e+21'), (1e20, '100000000000000000000'),
                            (123.0, '123'), (-0.0, '0'), (2.598076211353316, '2.598076211353316'), (-315, '-315'),
                            (5e-324, '5e-324'), (1.7976931348623157e308, '1.7976931348623157e+308'),
                            (0.000001234, '0.000001234'), (10**22, '1e+22'), (math.nan, 'null'), (-1.5e-7, '-1.5e-7')):
            with self.subTest(value=value):
                self.assertEqual(generate.js_json(value), text)
        self.assertEqual(generate.js_json({'b': 1, '2': [], '10': {}, '01': 'q"\\\n\x01 \ud800'}),
                         '{"2":[],"10":{},"b":1,"01":"q\\"\\\\\\n\\u0001 \\ud800"}')
        self.assertEqual(generate.js_json({'a': [1, {'b': None}], 'c': {}}, '  '),
                         '{\n  "a": [\n    1,\n    {\n      "b": null\n    }\n  ],\n  "c": {}\n}')
        with self.assertRaises(TypeError):
            generate.js_json({'a': object()})

    def test_pin_minimal_lock_and_install_policy(self):
        package_root = generate.ROOT / 'scripts/ergogen-parity'
        self.assertEqual(json.loads((package_root / 'package.json').read_text())['dependencies'], generate.SOURCE['dependencies'])
        lock = json.loads((package_root / 'package-lock.json').read_text())
        self.assertEqual(lock['packages']['']['dependencies'], generate.SOURCE['dependencies'])
        self.assertFalse(any(name.endswith(('hull', 'kle-serial', 'node_modules/ergogen')) for name in lock['packages']))
        self.assertRegex((package_root / '.npmrc').read_text(), '(?m)^ignore-scripts=true$')
        verified_runtime.check_lock(lock, json.loads((package_root / 'package.json').read_text()))
        with mock.patch.object(subprocess, 'run') as run, self.assertRaisesRegex(ValueError, '(?i)outside.*repository'):
            generate.bootstrap(generate.ROOT / 'not-a-checkout')
        run.assert_not_called()
        with self.assertRaisesRegex(ValueError, '(?i)outside.*repository'):
            generate.generate(generate.ROOT, driver=mock.Mock(side_effect=AssertionError('driver built')))

    def test_cli_rejects_invalid_checkout_before_writing_or_building(self):
        run = cli('--cache', str(self.checkout), '--output', str(self.output))
        self.assertNotEqual(run.returncode, 0)
        self.assertRegex(run.stderr, '(?i)pinned|checkout|git')
        self.assertFalse(self.output.exists())
        with mock.patch.object(generate, 'build_driver', side_effect=AssertionError('driver built')) as build, \
                contextlib.redirect_stderr(io.StringIO()) as stderr:
            self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.output)]), 1)
        build.assert_not_called()
        self.assertIn('Ergogen fixture generation failed', stderr.getvalue())
        self.assertEqual(sorted(self.root.iterdir()), [self.checkout])
        help_run = cli('--help')
        self.assertEqual(help_run.returncode, 0)
        self.assertIn('--cache', help_run.stdout)

    def test_cli_cannot_overwrite_legacy_oracle_or_checkout(self):
        original = generate.LEGACY.read_bytes() if generate.LEGACY.exists() else None
        link = self.root / 'link.json'
        link.symlink_to(generate.LEGACY)
        for output, pattern in ((generate.LEGACY, 'current.*oracle'), (link, 'current.*oracle'),
                                (self.checkout / 'fixture.json', 'pinned source checkout')):
            with self.subTest(output=output):
                run = cli('--cache', str(self.checkout), '--output', str(output))
                self.assertNotEqual(run.returncode, 0)
                self.assertRegex(run.stderr, '(?i)' + pattern)
        self.assertEqual(generate.LEGACY.read_bytes() if generate.LEGACY.exists() else None, original)
        self.assertTrue(link.is_symlink())
        self.assertEqual(list(self.checkout.iterdir()), [])

    def test_cli_does_not_move_or_use_a_checkout_at_another_head(self):
        git('init', '--quiet', str(self.checkout))
        git('-C', str(self.checkout), '-c', 'user.name=Fixture test', '-c', 'user.email=fixture@example.invalid',
            'commit', '--quiet', '--allow-empty', '-m', 'Unrelated fixture commit')
        head = (self.checkout / '.git/HEAD').read_text()
        run = cli('--cache', str(self.checkout), '--output', str(self.output))
        self.assertNotEqual(run.returncode, 0)
        self.assertIn('pinned Ergogen commit', run.stderr)
        self.assertFalse(self.output.exists())
        self.assertEqual((self.checkout / '.git/HEAD').read_text(), head)

    def test_atomic_output_preserves_unrelated_files_and_cleans_failures(self):
        unrelated = self.root / f'fixture.json.tmp-{os.getpid()}'
        unrelated.write_text('unrelated file\n')
        generate.write_fixture(self.output, 'new fixture\n')
        self.assertEqual(self.output.read_text(), 'new fixture\n')
        blocked = self.root / 'directory'
        (blocked / 'inside').mkdir(parents=True)
        with self.assertRaises(OSError):
            generate.write_fixture(blocked, 'never\n')
        self.assertEqual(unrelated.read_text(), 'unrelated file\n')
        self.assertEqual(sorted(path.name for path in self.root.iterdir()),
                         sorted(['checkout', 'directory', 'fixture.json', unrelated.name]))

    def test_output_permissions_follow_umask(self):
        old = os.umask(0o022)
        try:
            generate.write_fixture(self.output, 'fixture\n')
            self.assertEqual(self.output.stat().st_mode & 0o777, 0o644)
            self.output.chmod(0o640)
            generate.write_fixture(self.output, 'replacement\n')
            self.assertEqual(self.output.stat().st_mode & 0o777, 0o644)
        finally:
            os.umask(old)

    def test_cli_rejects_unverified_prepared_runtime(self):
        with mock.patch.object(generate, 'generate') as run, contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                generate.main(['--cache', str(self.checkout), '--output', str(self.output),
                               '--prepared-runtime', str(self.root)])
        run.assert_not_called()

    def test_atomic_output_renames_within_destination_directory(self):
        replace = os.replace

        def same_directory(source, destination):
            if Path(source).parent != Path(destination).parent:
                raise OSError(18, 'cross-directory rename denied')
            replace(source, destination)

        with mock.patch.object(generate.os, 'replace', side_effect=same_directory):
            generate.write_fixture(self.output, 'fixture\n')
        self.assertEqual(self.output.read_text(), 'fixture\n')

    def test_oracle_results_fail_closed(self):
        value = fixture()
        source = {'cases': [{k: v for k, v in item.items() if k != 'expected'} for item in value['cases']],
                  'boardStudioOnly': value['boardStudioOnly']}
        actual = [generate.recorded_points(item) for item in value['cases']]
        self.assertEqual(generate.render(generate.assemble(source, actual)), TEXT)
        first = next(iter(actual[0]))
        for name, change in (('count', lambda a, s: a.pop()), ('missing', lambda a, s: a[0].pop(first)),
                             ('extra', lambda a, s: a[0].update(extra={'x': 0, 'y': 0, 'r': 0})),
                             ('null', lambda a, s: a[0][first].update(x=None)),
                             ('non-object', lambda a, s: a.__setitem__(0, [])),
                             ('duplicate-case', lambda a, s: s['cases'][1].update(id=s['cases'][0]['id']))):
            bad_actual, bad_source = json.loads(json.dumps(actual)), json.loads(json.dumps(source))
            change(bad_actual, bad_source)
            with self.subTest(name=name), self.assertRaises(ValueError):
                generate.assemble(bad_source, bad_actual)
        payload = json.dumps({**source, 'actual': actual})
        entry = {'suite': host_probe.SUITE, 'case': 'generate', 'ok': True, 'json': payload}
        self.assertEqual(host_probe.validate_payload(json.dumps({host_probe.SUITE: [entry]}), 'generate')['actual'], actual)
        for bad in ({**entry, 'case': 'source'}, {**entry, 'ok': 1}, {**entry, 'json': json.loads(payload)},
                    {**entry, 'json': json.dumps(source)}, {**entry, 'json': payload.replace('19', 'NaN', 1)},
                    {**entry, 'json': '{"cases":[],"cases":[],"boardStudioOnly":[],"actual":[]}'}, {**entry, 'extra': 1}):
            with self.subTest(bad=str(bad)[:80]), self.assertRaises(ValueError):
                host_probe.validate_payload(json.dumps({host_probe.SUITE: [bad]}), 'generate')

    def test_cli_failures_never_write_and_success_is_exact(self):
        self.output.write_text('previous\n')
        with mock.patch.object(generate, 'bootstrap'), contextlib.redirect_stderr(io.StringIO()), \
                contextlib.redirect_stdout(io.StringIO()):
            with mock.patch.object(generate, 'generate', side_effect=RuntimeError('browser failed (1)')):
                self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.output)]), 1)
            self.assertEqual(self.output.read_text(), 'previous\n')
            with mock.patch.object(generate, 'generate', return_value=fixture()):
                self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.output), '--check']), 1)
                self.assertEqual(self.output.read_text(), 'previous\n')
                self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.output)]), 0)
                self.assertEqual(self.output.read_text(encoding='utf-8'), TEXT)
                self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.output), '--check']), 0)
                self.assertEqual(generate.main(['--cache', str(self.checkout), '--output', str(self.root / 'none.json'),
                                                '--check']), 1)
        self.assertEqual(sorted(path.name for path in self.root.iterdir()), ['checkout', 'fixture.json'])


    def test_sandbox_rejects_invalid_inputs_before_build(self):
        import sandbox_generate
        with mock.patch.object(sandbox_generate, 'verified_chrome', return_value=Path('/chrome')), \
                mock.patch.object(sandbox_generate, 'locked_archives'), \
                mock.patch.object(sandbox_generate.verified_runtime, 'verified_runtime', side_effect=ValueError('corrupt')), \
                mock.patch.object(generate, 'build_driver') as build, \
                mock.patch.object(sys, 'argv', ['sandbox_generate.py', '--cache', str(self.checkout),
                                               '--temp-parent', str(self.root), '--check']), \
                contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(sandbox_generate.main(), 1)
        build.assert_not_called()

    def test_sandbox_never_binds_destination_writable(self):
        import sandbox_generate
        staging = self.root / 'scratch'
        argv = sandbox_generate.command(staging, self.checkout, Path('/d'), Path('/c/chrome'), self.output, False)
        writable = [argv[i + 1] for i, item in enumerate(argv[:-1]) if item == '--bind']
        self.assertNotIn(str(self.output.parent), writable)
        self.assertEqual(writable, [str(staging / 'output')])

    def test_sandbox_verifies_chrome_and_isolates_the_run(self):
        import hashlib
        import zipfile
        import sandbox_generate
        archive, tree = self.root / 'chrome.zip', self.root / 'tree'
        with zipfile.ZipFile(archive, 'w') as bundle:
            bundle.writestr('chrome-linux64/chrome', b'binary')
            bundle.writestr('chrome-linux64/lib/x.so', b'library')
        (tree / 'chrome-linux64/lib').mkdir(parents=True)
        (tree / 'chrome-linux64/chrome').write_bytes(b'binary')
        (tree / 'chrome-linux64/lib/x.so').write_bytes(b'library')
        digest = hashlib.sha256(archive.read_bytes()).hexdigest()
        self.assertEqual(sandbox_generate.verified_chrome(tree, archive, digest), tree / 'chrome-linux64/chrome')
        for change in ('hash', 'byte', 'longer', 'extra', 'symlink'):
            with self.subTest(change=change):
                if change == 'byte': (tree / 'chrome-linux64/chrome').write_bytes(b'binarY')
                if change == 'longer': (tree / 'chrome-linux64/chrome').write_bytes(b'binary+')
                if change == 'extra': (tree / 'chrome-linux64/lib/evil.so').write_bytes(b'')
                if change == 'symlink':
                    (tree / 'chrome-linux64/lib/x.so').unlink()
                    (tree / 'chrome-linux64/lib/x.so').symlink_to(self.output)
                    self.output.write_bytes(b'library')
                with self.assertRaises(ValueError):
                    sandbox_generate.verified_chrome(tree, archive, '0' * 64 if change == 'hash' else digest)
                (tree / 'chrome-linux64/chrome').write_bytes(b'binary')
                (tree / 'chrome-linux64/lib/evil.so').unlink(missing_ok=True)
        argv = sandbox_generate.command(self.root, self.checkout, Path('/d'), Path('/c/chrome'), self.output, True)
        self.assertEqual(argv[:4], ['unshare', '--user', '--map-root-user', '--net'])
        self.assertIn('--clearenv', argv)
        self.assertNotIn('--share-net', argv)
        self.assertEqual(argv[argv.index('/usr/bin/node') - 2:argv.index('/usr/bin/node')], ['--ro-bind', '/dev/null'])
        self.assertEqual(argv[argv.index(str(self.root / 'output')) - 1], '--bind')  # private staging only
        self.assertNotIn('--check', argv)


def divergence_of(value):
    return next(item for item in value['cases'] if item['comparison'] == 'intentional-divergence')


class SourceConsistencyTests(unittest.TestCase):
    """cases.cjs plus recorded poses reproduce the committed fixture exactly (no upstream code)."""

    @classmethod
    def setUpClass(cls):
        cls.driver = generate.build_driver()

    def source(self):
        return host_probe.run(None, None, self.driver, 'source')

    def test_source_reproduces_committed_fixture_bytes(self):
        value, source = fixture(), self.source()
        self.assertEqual(len(source['cases']), len(value['cases']))
        self.assertTrue(any(item['id'] == 'S1-grid' for item in source['cases']))
        padding = next(item for item in source['cases'] if item['id'] == 'S7-padding')
        self.assertEqual(padding['ergogenConfig']['points']['zones']['m']['rows']['r1']['padding'], 22)
        self.assertEqual(padding['boardStudioMatrices'][0]['rowLayout'][2]['padding'], 22)
        rebuilt = generate.assemble(source, [generate.recorded_points(item) for item in value['cases']])
        self.assertEqual(generate.render(rebuilt), TEXT)

    def test_source_edit_without_regeneration_is_detected(self):
        read_bytes = Path.read_bytes
        def edited(path):
            data = read_bytes(path)
            return data.replace(b"r1: { padding: 22 }", b"r1: { padding: 23 }", 1) if path == host_probe.CASES else data
        with mock.patch.object(Path, 'read_bytes', edited):
            source = self.source()
        rebuilt = generate.assemble(source, [generate.recorded_points(item) for item in fixture()['cases']])
        self.assertNotEqual(generate.render(rebuilt), TEXT)

    def test_source_errors_fail_closed(self):
        read_bytes = Path.read_bytes
        with mock.patch.object(Path, 'read_bytes', lambda path: b"throw Error('controlled cases failure');"
                               if path == host_probe.CASES else read_bytes(path)):
            with self.assertRaisesRegex(RuntimeError, 'controlled cases failure'):
                self.source()


class UpstreamTests(unittest.TestCase):
    """Opt-in: the real pinned parser through the real CLI."""

    @classmethod
    def setUpClass(cls):
        cls.checkout = os.environ['ERGOGEN_CHECKOUT']
        cls.cache = os.environ.get('ERGOGEN_NPM_CACHE', str(generate.NPM_CACHE))
        cls.driver = str(generate.build_driver())

    def main(self, *args):
        with contextlib.redirect_stdout(io.StringIO()), contextlib.redirect_stderr(io.StringIO()) as stderr:
            status = generate.main(['--cache', self.checkout, '--npm-cache', self.cache, '--driver', self.driver, *args])
        return status, stderr.getvalue()

    def test_check_generate_and_mismatch(self):
        self.assertEqual(self.main('--check')[0], 0)
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory, 'fixture.json')
            self.assertEqual(self.main('--output', str(output))[0], 0)
            self.assertEqual(output.read_text(encoding='utf-8'), TEXT)
            corrupt = TEXT.replace('"x": 10,', '"x": 10.5,', 1)
            output.write_text(corrupt, encoding='utf-8')
            status, stderr = self.main('--output', str(output), '--check')
            self.assertEqual(status, 1)
            self.assertIn('Fixture differs', stderr)
            self.assertEqual(output.read_text(encoding='utf-8'), corrupt)

    def test_untracked_content_and_missing_archives_fail_before_browser(self):
        probe = Path(self.checkout, f'boardstudio-parity-probe-{os.getpid()}.txt')
        probe.write_text('untracked provenance probe\n')
        try:
            status, stderr = self.main('--check')
        finally:
            probe.unlink()
        self.assertEqual(status, 1)
        self.assertRegex(stderr, '(?i)modifications|untracked')
        with tempfile.TemporaryDirectory() as empty, \
                mock.patch.object(host_probe, 'serve', side_effect=AssertionError('browser served')):
            with contextlib.redirect_stderr(io.StringIO()) as stderr:
                status = generate.main(['--cache', self.checkout, '--npm-cache', empty, '--driver', self.driver, '--check'])
        self.assertEqual(status, 1)
        self.assertIn('No such file', stderr.getvalue())


SUITES = {'--hermetic': GeneratorContractTests, '--source': SourceConsistencyTests, '--upstream': UpstreamTests}

if __name__ == '__main__':
    if len(sys.argv) == 2 and sys.argv[1] in SUITES:
        case = SUITES[sys.argv[1]]
        names = unittest.TestLoader().getTestCaseNames(case)
        result = unittest.TextTestRunner(verbosity=2).run(unittest.TestSuite(case(name) for name in names))
        sys.exit(0 if result.wasSuccessful() and not result.skipped and result.testsRun == len(names) else 1)
    sys.exit('usage: test_generate.py --hermetic | --source | --upstream')
