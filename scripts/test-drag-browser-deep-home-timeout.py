#!/usr/bin/env python3
"""Check that the deep-HOME control kills its own timed-out process tree."""

import importlib.util
import os
from pathlib import Path
import select
import signal
import subprocess
import sys
import tempfile
import threading
import unittest
from unittest import mock

spec = importlib.util.spec_from_file_location("deep_home", Path(__file__).with_name("test-drag-browser-deep-home.py"))
deep_home = importlib.util.module_from_spec(spec)
spec.loader.exec_module(deep_home)


class TimeoutControl(unittest.TestCase):
    def test_unverified_cleanup_preserves_owned_home(self):
        with tempfile.TemporaryDirectory(prefix="bs-preserve-") as directory:
            home_root = Path(directory) / "owned"
            home_root.mkdir()
            with mock.patch.object(deep_home.tempfile, "mkdtemp", return_value=str(home_root)), \
                    mock.patch.object(deep_home, "run_control", side_effect=deep_home.ProcessGroupCleanupError("probe")):
                with self.assertRaises(deep_home.ProcessGroupCleanupError):
                    deep_home.main()
            sentinel = home_root / ("deep-home-" + "x" * 110) / "sentinel"
            self.assertEqual(sentinel.read_text(), "keep")

    def test_timeout_kills_group_without_touching_unrelated_process(self):
        self.check_timeout(False)

    def test_timeout_kills_detached_descendant(self):
        self.check_timeout(True)

    def check_timeout(self, detached):
        with tempfile.TemporaryDirectory(prefix="bs-timeout-") as directory:
            pids = Path(directory) / "pids"
            child = """import os, pathlib, signal, sys, time
signal.signal(signal.SIGTERM, signal.SIG_IGN)
pid = os.fork()
ready = pathlib.Path(sys.argv[1] + '.ready')
if not pid:
    if sys.argv[2] == '1': os.setsid()
    ready.write_text('ready')
if pid:
    while not ready.exists(): time.sleep(.01)
    pathlib.Path(sys.argv[1]).write_text(f'{os.getpid()} {pid}')
while True: time.sleep(60)
"""
            unrelated = subprocess.Popen([sys.executable, "-c", "import time; time.sleep(300)"], start_new_session=True)
            errors = []
            def run():
                try:
                    deep_home.run_control([sys.executable, "-c", child, str(pids), str(int(detached))], cwd=directory,
                                          env=os.environ.copy(), timeout=3)
                except BaseException as error:
                    errors.append(error)

            worker = threading.Thread(target=run)
            worker.start()
            fds = []
            try:
                ids = []
                for _ in range(250):
                    if pids.exists():
                        ids = pids.read_text().split()
                        if len(ids) == 2:
                            break
                    threading.Event().wait(0.01)
                self.assertEqual(len(ids), 2, "owned child did not start")
                fds = [os.pidfd_open(int(pid)) for pid in ids]
                worker.join(timeout=10)
                self.assertFalse(worker.is_alive(), "timeout did not return")
                self.assertEqual(len(errors), 1)
                self.assertIsInstance(errors[0], subprocess.TimeoutExpired)
                self.assertIsNone(unrelated.poll(), "unrelated process was signalled")
                for fd in fds:
                    poll = select.poll()
                    poll.register(fd, select.POLLIN)
                    self.assertTrue(poll.poll(2000), "owned descendant survived timeout")
                for pid in ids:
                    with self.assertRaises(ChildProcessError):
                        os.waitpid(int(pid), os.WNOHANG)
            finally:
                for fd in fds:
                    poll = select.poll()
                    poll.register(fd, select.POLLIN)
                    if not poll.poll(0):
                        signal.pidfd_send_signal(fd, signal.SIGKILL)
                    os.close(fd)
                unrelated.terminate()
                unrelated.wait(timeout=5)
                worker.join(timeout=10)


if __name__ == "__main__":
    unittest.main()
