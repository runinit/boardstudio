import importlib.util
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

SCRIPT = Path(__file__).with_name('verify-no-node.py')
spec = importlib.util.spec_from_file_location('verify_no_node', SCRIPT)
verify = importlib.util.module_from_spec(spec)
spec.loader.exec_module(verify)


class TraceTests(unittest.TestCase):
    def test_execve_and_execveat_including_unfinished(self):
        trace = '''123 execve("/usr/bin/node", ["node"], 0x0) = 0
124 execveat(AT_FDCWD, "/tmp/npm", ["npm"], 0x0, 0 <unfinished ...>
125 execve("/bin/true", ["true"], 0x0) = 0'''
        self.assertEqual(verify.executions(trace), ['/usr/bin/node', '/tmp/npm', '/bin/true'])

    def test_malformed_trace_fails_closed(self):
        with self.assertRaises(ValueError):
            verify.executions('123 execve(???')

    def test_missing_strace_is_actionable(self):
        with patch.object(verify.shutil, 'which', return_value=None):
            self.assertEqual(verify.main(['--', 'true']), 2)

    def test_failed_positive_control_does_not_run_command(self):
        with patch.object(verify.shutil, 'which', return_value='/fake/strace'):
            with patch.object(verify, 'trace_command', return_value=(0, ['/bin/sh'])) as trace:
                self.assertEqual(verify.main(['--', 'true']), 2)
                self.assertEqual(trace.call_count, 1)

    def test_named_execution_and_failed_command_fail(self):
        def run_trace(_strace, trace, _command, env=None):
            if env is not None:
                return 0, [str(trace.parent / 'node'), str(trace.parent / 'npm')]
            return result

        with patch.object(verify.shutil, 'which', return_value='/fake/strace'):
            with patch.object(verify, 'trace_command', side_effect=run_trace):
                for result in ((0, ['/usr/bin/node']), (0, ['/tmp/npm']), (1, ['/bin/false'])):
                    self.assertEqual(verify.main(['--', 'command']), 1)
                result = (0, ['/bin/true'])
                self.assertEqual(verify.main(['--', 'command']), 0)

    @unittest.skipUnless(shutil.which('strace'), 'strace is not installed')
    def test_live_controls_and_clean_command(self):
        self.assertEqual(subprocess.run([sys.executable, SCRIPT, '--', '/bin/true']).returncode, 0)

    @unittest.skipUnless(shutil.which('strace'), 'strace is not installed')
    def test_live_path_and_absolute_invocations(self):
        with tempfile.TemporaryDirectory() as temp:
            for name in ('node', 'npm'):
                path = Path(temp) / name
                path.write_text('#!/bin/sh\nexit 0\n')
                path.chmod(0o700)
            env = os.environ.copy()
            env['PATH'] = temp + os.pathsep + env.get('PATH', '')
            for command in (['/bin/sh', '-c', 'node'], [str(Path(temp) / 'npm')]):
                result = subprocess.run([sys.executable, SCRIPT, '--', *command], env=env,
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 1, result.stderr)
                self.assertIn('execution attempt detected', result.stderr)


if __name__ == '__main__':
    unittest.main()
