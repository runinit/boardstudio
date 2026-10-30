#!/usr/bin/env python3
import os
from pathlib import Path
import signal
import subprocess
import sys
import tempfile
import time
import unittest

ROOT = Path(__file__).resolve().parents[1]

class ParallelInterruptTest(unittest.TestCase):
    def test_child_inherits_caller_mask_and_handles_cancellation(self):
        for custom in (False, True):
            with self.subTest(custom=custom), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                child = r"""
import json, os, signal, sys, time
from pathlib import Path
root = Path(sys.argv[1])
def terminate(*_):
    (root / 'handled').write_text('term')
    sys.exit(0)
signal.signal(signal.SIGTERM, terminate)
(root / 'mask').write_text(json.dumps(sorted(signal.pthread_sigmask(signal.SIG_BLOCK, set()))))
while True: time.sleep(.05)
"""
                runner = r"""
import os, signal, sys
sys.path.insert(0, sys.argv[1])
import check
signal.pthread_sigmask(signal.SIG_SETMASK, {signal.SIGUSR1, signal.SIGINT} if sys.argv[4] == 'True' else set())
check.run_parallel([[sys.executable, '-c', sys.argv[2], sys.argv[3]]], dict(os.environ))
"""
                proc = subprocess.Popen([sys.executable, '-c', runner, str(ROOT / 'scripts'), child, directory, str(custom)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
                try:
                    deadline = time.monotonic() + 5
                    while not (root / 'mask').exists() and time.monotonic() < deadline: time.sleep(.01)
                    self.assertTrue((root / 'mask').exists(), 'child did not start')
                    import json
                    mask = set(json.loads((root / 'mask').read_text()))
                    os.kill(proc.pid, signal.SIGTERM)
                    out, err = proc.communicate(timeout=6)
                    self.assertEqual(proc.returncode, 143, (out, err))
                    self.assertEqual(mask, {signal.SIGUSR1, signal.SIGINT} if custom else set())
                    self.assertTrue((root / 'handled').exists(), 'child did not handle SIGTERM')
                finally:
                    if proc.poll() is None: os.killpg(proc.pid, signal.SIGKILL)
                    proc.communicate(timeout=5)

    def test_wrapper_preserves_native_signals_and_environment(self):
        import threading
        sys.path.insert(0, str(ROOT / 'scripts'))
        import check
        with tempfile.TemporaryDirectory() as directory:
            Path(directory, 'sitecustomize.py').write_text("print('SITE MARKER', flush=True)\n")
            env = dict(os.environ, LC_CTYPE='C', PYTHONPATH=directory,
                       WRAPPER_BYTES=os.fsdecode(b'byte-\xff'))
            registry = {'lock': threading.Lock(), 'cancelled': False,
                        'processes': set(), 'mask': signal.pthread_sigmask(signal.SIG_BLOCK, set())}
            def run(command):
                result = check._parallel_process(command, registry, env=env, stdout=subprocess.PIPE,
                                                 stderr=subprocess.STDOUT, text=True,
                                                 encoding='utf-8', errors='surrogateescape')
                for process in registry['processes']:
                    process.wait()
                    process.stdout.close()
                registry['processes'].clear()
                return result
            result = run(['/usr/bin/env', '-0'])
            self.assertEqual(result.returncode, 0)
            self.assertEqual(dict(item.split('=', 1) for item in result.stdout.split('\0') if item), env)
            result = run(['/bin/sh', '-c', 'kill -PIPE $$'])
            self.assertEqual(result.returncode, -signal.SIGPIPE, result.stdout)
            result = run([str(Path(directory, 'missing'))])
            self.assertEqual(result.returncode, 127)
            self.assertIn('startup failed:', result.stdout)
            self.assertNotIn('SITE MARKER', result.stdout)

    def test_missing_command_reports_startup_failure(self):
        runner = "import os,sys; sys.path.insert(0,sys.argv[1]); import check; sys.exit(bool(check.run_parallel([[sys.argv[2]]], dict(os.environ))))"
        with tempfile.TemporaryDirectory() as directory:
            missing = str(Path(directory) / 'missing-command')
            result = subprocess.run([sys.executable, '-c', runner, str(ROOT / 'scripts'), missing], capture_output=True, text=True, timeout=5)
        self.assertEqual(result.returncode, 1, result.stderr)
        self.assertIn('exit 127', result.stdout)
        self.assertIn('startup failed:', result.stdout)
        self.assertIn(missing, result.stdout)

    def test_signal_boundaries_restore_handlers_and_mask(self):
        runner = r"""
import os, signal, sys
sys.path.insert(0, sys.argv[1])
import check
mode = sys.argv[2]
old_int = signal.getsignal(signal.SIGINT)
old_term = signal.getsignal(signal.SIGTERM)
signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGUSR1})
old_mask = signal.pthread_sigmask(signal.SIG_BLOCK, set())
original = signal.signal
sent = False
def boundary(sig, handler):
    global sent
    result = original(sig, handler)
    if not sent and ((mode == 'install' and sig == signal.SIGTERM and callable(handler)) or
                     (mode == 'cleanup' and sig == signal.SIGINT and handler == signal.SIG_IGN)):
        sent = True
        os.kill(os.getpid(), signal.SIGTERM)
    return result
signal.signal = boundary
try:
    check.run_parallel([['unused']], dict(os.environ))
except SystemExit as error:
    assert error.code == 143
else:
    raise AssertionError('SIGTERM lost')
assert sent
assert signal.getsignal(signal.SIGINT) == old_int
assert signal.getsignal(signal.SIGTERM) == old_term
assert signal.pthread_sigmask(signal.SIG_BLOCK, set()) == old_mask
"""
        for mode in ('install', 'cleanup'):
            with self.subTest(mode=mode):
                # Cleanup starts after this deterministic initial interruption.
                source = runner.replace("signal.signal = boundary", "signal.signal = boundary\n" +
                    ("check.ThreadPoolExecutor.submit = lambda *args: (_ for _ in ()).throw(SystemExit(143))" if mode == 'cleanup' else ""))
                result = subprocess.run([sys.executable, '-c', source, str(ROOT / 'scripts'), mode],
                                        capture_output=True, text=True, timeout=10)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_workers_inherit_blocked_signals_and_main_mask_is_restored(self):
        runner = r"""
import os, signal, sys
sys.path.insert(0, sys.argv[1])
import check
signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGUSR1})
old_mask = signal.pthread_sigmask(signal.SIG_BLOCK, set())
original = check._parallel_process
def process(*args, **kwargs):
    mask = signal.pthread_sigmask(signal.SIG_BLOCK, set())
    assert {signal.SIGINT, signal.SIGTERM} <= mask, mask
    return original(*args, **kwargs)
check._parallel_process = process
check.run_parallel([[sys.executable, '-c', 'pass']], dict(os.environ))
assert signal.pthread_sigmask(signal.SIG_BLOCK, set()) == old_mask
"""
        result = subprocess.run([sys.executable, '-c', runner, str(ROOT / 'scripts')],
                                capture_output=True, text=True, timeout=10)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_second_signal_at_exception_transition(self):
        runner = r"""
import os, signal, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check
root = Path(sys.argv[2])
first_name, second_name = sys.argv[3:]
first = getattr(signal, first_name) if first_name != 'ordinary' else None
second = getattr(signal, second_name)
old_int, old_term = signal.getsignal(signal.SIGINT), signal.getsignal(signal.SIGTERM)
signal.pthread_sigmask(signal.SIG_BLOCK, {signal.SIGUSR1})
old_mask = signal.pthread_sigmask(signal.SIG_BLOCK, set())
original_wait = check.wait
sent = False
def wait(*args, **kwargs):
    global sent
    if not sent:
        deadline = time.monotonic() + 5
        while not (root / 'pid').exists() and time.monotonic() < deadline:
            time.sleep(.01)
        assert (root / 'pid').exists()
        sent = True
        if first is None:
            raise ValueError('original failure')
        os.kill(os.getpid(), first)
    return original_wait(*args, **kwargs)
check.wait = wait
def trace(frame, event, arg):
    if frame.f_code is check.run_parallel.__code__ and event == 'line' and frame.f_lineno == transition:
        sys.settrace(None)
        os.kill(os.getpid(), second)
    return trace
import inspect
lines, start = inspect.getsourcelines(check.run_parallel)
transition = start + next(i for i, line in enumerate(lines) if line.strip() == 'error = caught')
sys.settrace(trace)
child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
try:
    check.run_parallel([[sys.executable, '-c', child, str(root / 'pid')]], dict(os.environ))
except (KeyboardInterrupt, SystemExit, ValueError) as error:
    if first is None:
        assert isinstance(error, ValueError) and str(error) == 'original failure'
    else:
        assert isinstance(error, KeyboardInterrupt) if first == signal.SIGINT else isinstance(error, SystemExit) and error.code == 143
else:
    raise AssertionError('signal lost')
assert sent
assert signal.getsignal(signal.SIGINT) == old_int
assert signal.getsignal(signal.SIGTERM) == old_term
assert signal.pthread_sigmask(signal.SIG_BLOCK, set()) == old_mask
pid = int((root / 'pid').read_text())
assert not Path(f'/proc/{pid}').exists(), 'owned child survived'
"""
        for first, second in (('SIGINT', 'SIGTERM'), ('SIGTERM', 'SIGINT'), ('ordinary', 'SIGTERM')):
            with self.subTest(first=first, second=second), tempfile.TemporaryDirectory() as directory:
                proc = subprocess.Popen([sys.executable, '-c', runner, str(ROOT / 'scripts'), directory, first, second],
                                        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
                try:
                    stdout, stderr = proc.communicate(timeout=5)
                    self.assertEqual(proc.returncode, 0, stdout + stderr)
                finally:
                    if proc.poll() is None:
                        os.killpg(proc.pid, signal.SIGKILL)
                    pidfile = Path(directory) / 'pid'
                    if pidfile.exists():
                        try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                        except ProcessLookupError: pass
                    proc.communicate(timeout=5)

    def test_interrupt_cleans_descendant_after_command_completed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runner = r"""
import os, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check
root = Path(sys.argv[2])
original = check._parallel_process
def run(command, registry, **kwargs):
    result = original(command, registry, **kwargs)
    if str(root / 'pid') in command: (root / 'completed').touch()
    return result
check._parallel_process = run
child = "import os,sys,time; p=os.fork(); p and sys.exit(0); os.close(1); os.close(2); open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
waiting = "import time; time.sleep(60)"
check.run_parallel([[sys.executable, '-c', child, str(root / 'pid')], [sys.executable, '-c', waiting]], dict(os.environ))
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), directory],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            pidfile = root / 'pid'
            try:
                deadline = time.monotonic() + 5
                while not pidfile.exists() and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue(pidfile.exists(), 'descendant did not start')
                deadline = time.monotonic() + 5
                while not (root / 'completed').exists() and time.monotonic() < deadline:
                    time.sleep(.01)
                self.assertTrue((root / 'completed').exists(), 'command did not complete')
                os.kill(proc.pid, signal.SIGINT)
                proc.communicate(timeout=8)
                pid = int(pidfile.read_text())
                stat = Path(f'/proc/{pid}/stat')
                self.assertTrue(not stat.exists() or stat.read_text().split(') ')[1][0] == 'Z',
                                'same-group descendant survived cancellation')
            finally:
                if proc.poll() is None:
                    os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.kill(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_python_worker_can_delay_interpreter_exit_after_cleanup(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runner = r"""
import os, sys, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check
root = Path(sys.argv[2])
def blocked(*args, **kwargs):
    (root / 'ready').touch()
    while not (root / 'release').exists(): time.sleep(.01)
    return None
check._parallel_process = blocked
submit = ThreadPoolExecutor.submit
def interrupted(pool, fn, *args):
    future = submit(pool, fn, *args)
    while not (root / 'ready').exists(): time.sleep(.01)
    raise KeyboardInterrupt()
ThreadPoolExecutor.submit = interrupted
try: check.run_parallel([['blocked']], dict(os.environ))
except KeyboardInterrupt: (root / 'returned').touch()
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), directory],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
            try:
                deadline = time.monotonic() + 8
                while not (root / 'returned').exists() and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue((root / 'returned').exists(), 'cleanup did not return')
                self.assertIsNone(proc.poll(), 'interpreter should still be joining the worker')
                (root / 'release').touch()
                out, err = proc.communicate(timeout=5)
                self.assertEqual(proc.returncode, 0, (out, err))
                self.assertIn('interpreter exit may wait indefinitely', err)
            finally:
                (root / 'release').touch()
                if proc.poll() is None:
                    proc.kill()
                proc.communicate(timeout=5)

    def test_submit_interrupt_waits_for_untracked_workers(self):
        with tempfile.TemporaryDirectory() as directory:
            runner = r"""
import json, os, sys, threading, time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check, verification_receipt
root = Path(sys.argv[2])
commands = [[sys.executable, '-c', "import os,pathlib,sys,time; pathlib.Path(sys.argv[1]).write_text(str(os.getpid())); time.sleep(60)", str(root / 'ready')], ['second']]
c = verification_receipt.Collector(path=root / 'receipt', source={})
c.plan_commands([('test', commands)])
entered, release = threading.Event(), threading.Event()
prepare = c.prepare
def blocked_prepare(command, **kwargs):
    if command == ['second']:
        entered.set()
        if not release.wait(4): raise RuntimeError('barrier timed out')
    return prepare(command, **kwargs)
c.prepare = blocked_prepare
submit = ThreadPoolExecutor.submit
count = 0
def interrupted(pool, fn, *args):
    global count
    future = submit(pool, fn, *args)
    count += 1
    if count == 2:
        if not entered.wait(4): raise RuntimeError('worker did not enter')
        deadline = time.monotonic() + 4
        while not (root / 'ready').exists() and time.monotonic() < deadline: time.sleep(.01)
        if not (root / 'ready').exists(): raise RuntimeError('process did not start')
        threading.Timer(.2, release.set).start()
        raise KeyboardInterrupt()
    return future
ThreadPoolExecutor.submit = interrupted
try:
    check.run_parallel(commands, dict(os.environ), receipt=c, step='test')
except KeyboardInterrupt:
    records = c.finish(130)['commands']
    print('RECORDS ' + json.dumps([x['status'] for x in records]), flush=True)
else: raise AssertionError('interrupt lost')
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), directory],
                                    stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                out, err = proc.communicate(timeout=8)
                self.assertEqual(proc.returncode, 0, (out, err))
                self.assertIn('RECORDS ["ran", "interrupted"]', out)
            finally:
                if proc.poll() is None:
                    os.killpg(proc.pid, signal.SIGKILL)
                ready = Path(directory) / 'ready'
                if ready.exists():
                    try:
                        os.killpg(int(ready.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                proc.communicate(timeout=5)

    def test_interrupt_during_error_shutdown_terminates_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "pid"
            runner = r"""
import os, sys, time
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, sys.argv[1])
import check
pidfile = sys.argv[2]
original = check._parallel_process
def run(command, registry, **kwargs):
    if command == ['error']:
        deadline = time.monotonic() + 4
        while not os.path.exists(pidfile) and time.monotonic() < deadline: time.sleep(.01)
        raise ValueError('first error')
    return original(command, registry, **kwargs)
check._parallel_process = run
shutdown = ThreadPoolExecutor.shutdown
called = False
def interrupt(pool, **kwargs):
    global called
    if not called:
        called = True
        raise KeyboardInterrupt()
    return shutdown(pool, **kwargs)
ThreadPoolExecutor.shutdown = interrupt
child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
check.run_parallel([['error'], [sys.executable, '-c', child, pidfile]], dict(os.environ))
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                out, err = proc.communicate(timeout=6)
                self.assertNotEqual(proc.returncode, 0, (out, err))
                self.assertIn('first error', err)
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(pidfile.read_text()), 0)
            finally:
                if proc.poll() is None:
                    os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_sigterm_during_error_shutdown_terminates_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / "pid"
            runner = r"""
import os, signal, sys, time
from concurrent.futures import ThreadPoolExecutor
sys.path.insert(0, sys.argv[1])
import check
pidfile = sys.argv[2]
original = check._parallel_process
def run(command, registry, **kwargs):
    if command == ['error']:
        deadline = time.monotonic() + 4
        while not os.path.exists(pidfile) and time.monotonic() < deadline: time.sleep(.01)
        raise ValueError('first error')
    return original(command, registry, **kwargs)
check._parallel_process = run
shutdown = ThreadPoolExecutor.shutdown
called = False
def interrupt(pool, **kwargs):
    global called
    if not called:
        called = True
        os.kill(os.getpid(), signal.SIGTERM)
    return shutdown(pool, **kwargs)
ThreadPoolExecutor.shutdown = interrupt
child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
check.run_parallel([['error'], [sys.executable, '-c', child, pidfile]], dict(os.environ))
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                out, err = proc.communicate(timeout=6)
                self.assertNotEqual(proc.returncode, 0, (out, err))
                self.assertIn('first error', err)
                with self.assertRaises(ProcessLookupError):
                    os.kill(int(pidfile.read_text()), 0)
            finally:
                if proc.poll() is None:
                    os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_cleanup_returns_while_escaped_descendant_holds_stdout(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            runner = r"""
import os, sys
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check
root = Path(sys.argv[2])
child = "import os,sys,time; os.setsid(); open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
leader = "import subprocess,sys,time; subprocess.Popen([sys.executable,'-c',sys.argv[1],sys.argv[2]]); time.sleep(60)"
try: check.run_parallel([[sys.executable,'-c',leader,child,str(root/'pid')]],dict(os.environ))
except KeyboardInterrupt: (root/'returned').touch()
"""
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), directory], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not (root / 'pid').exists() and time.monotonic() < deadline: time.sleep(.02)
                self.assertTrue((root / 'pid').exists(), 'escaped child did not start')
                os.kill(proc.pid, signal.SIGINT)
                deadline = time.monotonic() + 8
                while not (root / 'returned').exists() and time.monotonic() < deadline: time.sleep(.02)
                self.assertTrue((root / 'returned').exists(), 'cleanup blocked closing stdout')
            finally:
                if (root / 'pid').exists():
                    try: os.kill(int((root / 'pid').read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                try: proc.communicate(timeout=5)
                except subprocess.TimeoutExpired:
                    os.killpg(proc.pid, signal.SIGKILL)
                    proc.communicate(timeout=5)

    def test_invalid_output_with_open_stdout_terminates_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / 'pid'
            child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); os.write(1,b'\\xff'); time.sleep(60)"
            runner = "import os,sys; sys.path.insert(0,sys.argv[1]); import check; check.run_parallel([[sys.executable,'-c',sys.argv[2],sys.argv[3]]],dict(os.environ))"
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), child, str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                out, err = proc.communicate(timeout=6)
                self.assertNotEqual(proc.returncode, 0, (out, err))
                self.assertIn('UnicodeDecodeError', err)
                with self.assertRaises(ProcessLookupError): os.kill(int(pidfile.read_text()), 0)
            finally:
                if proc.poll() is None: os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_invalid_child_output_terminates_running_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / 'pid'
            child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); os.write(1,b'\\xff'); os.close(1); os.close(2); time.sleep(60)"
            runner = "import os,sys; sys.path.insert(0,sys.argv[1]); import check; check.run_parallel([[sys.executable,'-c',sys.argv[2],sys.argv[3]]],dict(os.environ))"
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), child, str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                out, err = proc.communicate(timeout=6)
                self.assertNotEqual(proc.returncode, 0, (out, err))
                self.assertIn('UnicodeDecodeError', err)
                with self.assertRaises(ProcessLookupError): os.kill(int(pidfile.read_text()), 0)
            finally:
                if proc.poll() is None: os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_later_read_failures_stop_blocked_first_child(self):
        for injected in (False, True):
            with self.subTest(injected=injected), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                runner = r"""
import os, sys, time
from pathlib import Path
sys.path.insert(0, sys.argv[1])
import check
root = Path(sys.argv[2])
blocked = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
invalid = "import os,sys,time; p=sys.argv[1]; exec(\"while not os.path.exists(p): time.sleep(.01)\"); os.write(1,b'\\xff'); os.close(1); os.close(2); time.sleep(60)"
if sys.argv[3] == 'True':
    original = check.subprocess.Popen
    def popen(command, **kwargs):
        process = original(command, **kwargs)
        if command[-1] == 'inject':
            stream = process.stdout
            class Broken:
                encoding, errors = stream.encoding, stream.errors
                @property
                def buffer(self): return self
                def read1(self, size):
                    while not (root / 'pid').exists(): time.sleep(.01)
                    raise OSError('injected read error')
                def close(self): stream.close()
            process.stdout = Broken()
        return process
    check.subprocess.Popen = popen
    second = [sys.executable, '-c', 'import time; time.sleep(60)', 'inject']
else:
    second = [sys.executable, '-c', invalid, str(root / 'pid')]
check.run_parallel([[sys.executable, '-c', blocked, str(root / 'pid')], second], dict(os.environ))
"""
                proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), directory, str(injected)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
                try:
                    out, err = proc.communicate(timeout=5)
                    self.assertNotEqual(proc.returncode, 0, (out, err))
                    self.assertIn('injected read error' if injected else 'UnicodeDecodeError', err)
                    with self.assertRaises(ProcessLookupError): os.kill(int((root / 'pid').read_text()), 0)
                finally:
                    if proc.poll() is None: os.killpg(proc.pid, signal.SIGKILL)
                    if (root / 'pid').exists():
                        try: os.killpg(int((root / 'pid').read_text()), signal.SIGKILL)
                        except ProcessLookupError: pass
                    proc.communicate(timeout=5)

    def test_sigterm_terminates_owned_child(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / 'pid'
            child = "import os,sys,time; open(sys.argv[1],'w').write(str(os.getpid())); time.sleep(60)"
            runner = "import os,sys; sys.path.insert(0,sys.argv[1]); import check; check.run_parallel([[sys.executable,'-c',sys.argv[2],sys.argv[3]]],dict(os.environ))"
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), child, str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not pidfile.exists() and time.monotonic() < deadline: time.sleep(.02)
                self.assertTrue(pidfile.exists(), 'child did not start')
                os.kill(proc.pid, signal.SIGTERM)
                out, err = proc.communicate(timeout=6)
                self.assertEqual(proc.returncode, 143, (out, err))
                with self.assertRaises(ProcessLookupError): os.kill(int(pidfile.read_text()), 0)
            finally:
                if proc.poll() is None: os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try: os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError: pass
                proc.communicate(timeout=5)

    def test_two_interrupts_terminate_owned_process_group(self):
        with tempfile.TemporaryDirectory() as directory:
            pidfile = Path(directory) / 'pid'
            child = "import os,signal,time,sys; signal.signal(signal.SIGTERM,signal.SIG_IGN); open(sys.argv[1],'w').write(str(os.getpid())); print('READY',flush=True); time.sleep(60)"
            runner = "import os,sys; sys.path.insert(0,sys.argv[1]); import check; check.run_parallel([[sys.executable,'-c',sys.argv[2],sys.argv[3]]],dict(os.environ))"
            proc = subprocess.Popen([sys.executable, '-u', '-c', runner, str(ROOT / 'scripts'), child, str(pidfile)], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, start_new_session=True)
            try:
                deadline = time.monotonic() + 5
                while not pidfile.exists() and time.monotonic() < deadline:
                    time.sleep(.02)
                self.assertTrue(pidfile.exists(), 'worker did not start')
                pid = int(pidfile.read_text())
                os.kill(proc.pid, signal.SIGINT)
                time.sleep(.1)
                os.kill(proc.pid, signal.SIGINT)
                out, err = proc.communicate(timeout=5)
                self.assertNotEqual(proc.returncode, 0, (out, err))
                self.assertIn('KeyboardInterrupt', err)
                with self.assertRaises(ProcessLookupError):
                    os.kill(pid, 0)
            finally:
                if proc.poll() is None:
                    os.killpg(proc.pid, signal.SIGKILL)
                if pidfile.exists():
                    try:
                        os.killpg(int(pidfile.read_text()), signal.SIGKILL)
                    except ProcessLookupError:
                        pass
                proc.communicate(timeout=5)

if __name__ == '__main__':
    unittest.main()
