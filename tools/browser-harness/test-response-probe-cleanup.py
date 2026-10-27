import importlib.util
import pathlib
import signal
import subprocess
import sys
import tempfile
import time
import unittest

path = pathlib.Path('tools/browser-harness/test-cad-target-responses.py')
spec = importlib.util.spec_from_file_location('probe', path)
probe = importlib.util.module_from_spec(spec)
spec.loader.exec_module(probe)

@unittest.skipUnless(pathlib.Path('/proc/self/stat').exists(), 'requires Linux /proc')
class Cleanup(unittest.TestCase):
    def test_owned_descendant_after_exit_and_timeout(self):
        for mode in ('exit', 'failure', 'timeout'):
            with self.subTest(mode=mode), tempfile.TemporaryDirectory() as tmp:
                pidfile = pathlib.Path(tmp, 'pid')
                code = '''import subprocess,sys,time
p=subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(3)'])
open(sys.argv[1], 'w').write(str(p.pid))
if sys.argv[2] == 'timeout': time.sleep(60)
if sys.argv[2] == 'failure': sys.exit(7)
'''
                if mode == 'timeout':
                    with self.assertRaises(subprocess.TimeoutExpired):
                        probe.run_probe([sys.executable, '-c', code, str(pidfile), mode], 1)
                else:
                    self.assertEqual(probe.run_probe([sys.executable, '-c', code, str(pidfile), mode], 5).returncode, 7 if mode == 'failure' else 0)
                pid = int(pidfile.read_text())
                for _ in range(100):
                    stat = pathlib.Path(f'/proc/{pid}/stat')
                    if not stat.exists() or stat.read_text().split(') ')[1].startswith('Z'):
                        break
                    time.sleep(.01)
                else: self.fail(f'child {pid} still running')

    def test_sigterm_cleans_probe_group(self):
        with tempfile.TemporaryDirectory() as tmp:
            pidfile = pathlib.Path(tmp, 'pid')
            code = """import importlib.util,sys
s=importlib.util.spec_from_file_location('probe',sys.argv[1]); m=importlib.util.module_from_spec(s); s.loader.exec_module(m)
m.run_probe([sys.executable,'-c','import os,sys,time; open(sys.argv[1],\"w\").write(str(os.getpid())); time.sleep(60)',sys.argv[2]],60)
"""
            parent = subprocess.Popen([sys.executable, '-c', code, str(path), str(pidfile)])
            try:
                for _ in range(500):
                    if pidfile.exists() and pidfile.read_text():
                        break
                    time.sleep(.01)
                else:
                    self.fail('probe did not start')
                pid = int(pidfile.read_text())
                parent.send_signal(signal.SIGTERM)
                self.assertEqual(parent.wait(timeout=5), 128 + signal.SIGTERM)
                for _ in range(100):
                    stat = pathlib.Path(f'/proc/{pid}/stat')
                    if not stat.exists() or stat.read_text().split(') ')[1].startswith('Z'):
                        break
                    time.sleep(.01)
                else:
                    self.fail(f'probe {pid} still running')
            finally:
                if parent.poll() is None:
                    parent.kill()
                    parent.wait()

unittest.main()
