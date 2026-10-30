import contextlib
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

import sandbox_generate as sandbox


class OuterEntryTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='ergogen-entry-test-')
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.checkout = self.root / 'checkout'
        self.checkout.mkdir()
        self.output = self.root / 'fixture.json'
        self.output.write_bytes(b'original')
        self.fixture = sandbox.generate.FIXTURE.read_bytes()
        self.argv = ['sandbox_generate.py', '--cache', str(self.checkout), '--output', str(self.output),
                     '--temp-parent', str(self.root)]
        self.stack = contextlib.ExitStack()
        self.addCleanup(self.stack.close)
        self.stack.enter_context(mock.patch.object(sys, 'argv', self.argv))
        self.stack.enter_context(mock.patch.dict(os.environ))
        self.stack.enter_context(mock.patch.object(sandbox, 'verified_chrome', return_value=Path('/chrome')))
        self.stack.enter_context(mock.patch.object(sandbox, 'locked_archives'))
        self.build = self.stack.enter_context(mock.patch.object(sandbox.generate, 'build_driver', return_value=Path('/driver')))
        self.stack.enter_context(mock.patch.object(sandbox, 'command', side_effect=lambda run, *args: run))
        self.stderr = io.StringIO()
        self.stack.enter_context(contextlib.redirect_stderr(self.stderr))
        self.stack.enter_context(contextlib.redirect_stdout(io.StringIO()))

    def git(self, *args):
        return subprocess.check_output(['git', '-C', str(self.checkout), *args], stderr=subprocess.PIPE).decode().strip()

    def init_checkout(self):
        self.git('init', '-q')
        self.git('config', 'user.name', 'Test')
        self.git('config', 'user.email', 'test@example.invalid')
        (self.checkout / 'tracked').write_text('original')
        self.git('add', 'tracked')
        self.git('commit', '-qm', 'initial')
        return self.git('rev-parse', 'HEAD')

    def test_wrong_head_tracked_dirty_and_untracked_fail_before_build(self):
        pin = self.init_checkout()
        for case in ('wrong head', 'tracked dirty', 'untracked'):
            with self.subTest(case=case):
                self.stderr.seek(0)
                self.stderr.truncate()
                if case == 'tracked dirty':
                    (self.checkout / 'tracked').write_text('changed')
                if case == 'untracked':
                    (self.checkout / 'tracked').write_text('original')
                    (self.checkout / 'untracked').write_text('unexpected')
                with mock.patch.object(sandbox.verified_runtime, 'PIN', '0' * 40 if case == 'wrong head' else pin):
                    self.assertEqual(sandbox.main(), 1)
                self.assertIn('pinned Ergogen commit' if case == 'wrong head' else 'tracked modifications', self.stderr.getvalue())
                self.build.assert_not_called()
                self.assertEqual(self.output.read_bytes(), b'original')

    def test_publication_failure_preserves_existing_output(self):
        def inner(run):
            (run / 'output/fixture.json').write_bytes(self.fixture)
            return subprocess.CompletedProcess([], 0)
        with mock.patch.object(sandbox.generate.os, 'replace', side_effect=OSError('publication failed')):
            self.assertEqual(self.run_inner(inner), 1)
        self.assertIn('publication failed', self.stderr.getvalue())
        self.assertEqual(self.output.read_bytes(), b'original')

    def prepared(self):
        return mock.patch.object(sandbox.verified_runtime, 'verified_runtime',
                                 return_value=contextlib.nullcontext(self.root / 'prepared'))

    def run_inner(self, effect):
        with self.prepared(), mock.patch.object(sandbox.subprocess, 'run', side_effect=effect):
            return sandbox.main()

    def test_nonzero_status_propagates_with_or_without_reset_text(self):
        for status in (7, 9):
            for reset in ('', 'WebSocket connection reset by peer\n'):
                with self.subTest(status=status, reset=reset):
                    def inner(run):
                        print(reset, file=sys.stderr, end='')
                        (run / 'output/fixture.json').write_bytes(self.fixture)
                        return subprocess.CompletedProcess([], status)
                    self.assertEqual(self.run_inner(inner), status)
                    self.assertEqual(self.output.read_bytes(), b'original')

    def test_staged_symlink_and_fifo_are_rejected(self):
        for kind in ('symlink', 'fifo'):
            with self.subTest(kind=kind):
                def inner(run):
                    staged = run / 'output/fixture.json'
                    if kind == 'symlink':
                        staged.symlink_to(sandbox.generate.FIXTURE)
                    else:
                        os.mkfifo(staged)
                    return subprocess.CompletedProcess([], 0)
                self.assertEqual(self.run_inner(inner), 1)
                self.assertEqual(self.output.read_bytes(), b'original')

    def test_check_match_and_mismatch_preserve_output(self):
        self.argv.append('--check')
        def inner(run):
            (run / 'output/fixture.json').write_bytes(self.fixture)
            return subprocess.CompletedProcess([], 0)
        self.assertEqual(self.run_inner(inner), 1)
        self.assertEqual(self.output.read_bytes(), b'original')
        self.output.write_bytes(self.fixture)
        self.assertEqual(self.run_inner(inner), 0)
        self.assertEqual(self.output.read_bytes(), self.fixture)

    def test_success_publishes_valid_fixture(self):
        def inner(run):
            (run / 'output/fixture.json').write_bytes(self.fixture)
            return subprocess.CompletedProcess([], 0)
        self.assertEqual(self.run_inner(inner), 0)
        self.assertEqual(self.output.read_bytes(), self.fixture)


if __name__ == '__main__':
    unittest.main()
