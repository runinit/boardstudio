#!/usr/bin/env python3
"""Exercise static preview routes, isolation headers, and browser asset MIME."""

from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import time
import unittest
from unittest import mock
from urllib.error import HTTPError, URLError
from urllib.request import urlopen

SCRIPT = Path(__file__).with_name("serve-web.py")


class ServeWebTests(unittest.TestCase):
    def test_startup_retries_response_timeout(self):
        for test in (self.test_root_subpath_headers_and_wasm_mime,
                     self.test_subpath_only_build_redirects_root):
            with self.subTest(test=test.__name__):
                real_urlopen = urlopen
                first_request = True

                def timeout_once(*args, **kwargs):
                    nonlocal first_request
                    if first_request:
                        first_request = False
                        raise TimeoutError("response timed out")
                    return real_urlopen(*args, **kwargs)

                with mock.patch(f"{__name__}.urlopen", side_effect=timeout_once):
                    test()

    def test_root_subpath_headers_and_wasm_mime(self):
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            for path, content in (
                ("site-root/index.html", "root"),
                ("site-root/worker.wasm", "wasm"),
                ("site-subpath/boardstudio/index.html", "subpath"),
            ):
                file = build / path
                file.parent.mkdir(parents=True, exist_ok=True)
                file.write_bytes(content.encode())
            with socket.socket() as listener:
                listener.bind(("127.0.0.1", 0))
                port = listener.getsockname()[1]
            process = subprocess.Popen(
                [sys.executable, str(SCRIPT), str(build), str(port)],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
            )
            try:
                response = None
                for _ in range(80):
                    try:
                        response = urlopen(f"http://127.0.0.1:{port}/", timeout=0.2)
                        break
                    except (URLError, ConnectionError, TimeoutError):
                        if process.poll() is not None:
                            self.fail(process.stderr.read().decode())
                        time.sleep(0.025)
                self.assertIsNotNone(response)
                self.assertEqual(response.read(), b"root")
                self.assertEqual(response.headers["Cross-Origin-Opener-Policy"], "same-origin")
                self.assertEqual(response.headers["Cross-Origin-Embedder-Policy"], "require-corp")
                with urlopen(f"http://127.0.0.1:{port}/boardstudio/") as subpath:
                    self.assertEqual(subpath.read(), b"subpath")
                with urlopen(f"http://127.0.0.1:{port}/worker.wasm") as wasm:
                    self.assertEqual(wasm.headers.get_content_type(), "application/wasm")
            finally:
                process.terminate()
                process.wait(timeout=3)
                if process.stderr:
                    process.stderr.close()

    def test_subpath_only_build_redirects_root(self):
        with tempfile.TemporaryDirectory() as directory:
            build = Path(directory)
            site = build / "site-subpath/boardstudio"
            site.mkdir(parents=True)
            (site / "index.html").write_text("subpath")
            with socket.socket() as listener:
                listener.bind(("127.0.0.1", 0))
                port = listener.getsockname()[1]
            process = subprocess.Popen(
                [sys.executable, str(SCRIPT), str(build), str(port)],
                stdout=subprocess.DEVNULL, stderr=subprocess.PIPE,
            )
            try:
                response = None
                for _ in range(80):
                    try:
                        response = urlopen(f"http://127.0.0.1:{port}/", timeout=0.2)
                        break
                    except (URLError, ConnectionError, TimeoutError):
                        if process.poll() is not None:
                            self.fail(process.stderr.read().decode())
                        time.sleep(0.025)
                self.assertIsNotNone(response)
                self.assertEqual(response.url, f"http://127.0.0.1:{port}/boardstudio/")
                self.assertEqual(response.read(), b"subpath")
            finally:
                process.terminate()
                process.wait(timeout=3)
                if process.stderr:
                    process.stderr.close()

    def test_incomplete_build_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            process = subprocess.run([sys.executable, str(SCRIPT), directory, "0"],
                                     capture_output=True, text=True)
            self.assertNotEqual(process.returncode, 0)
            self.assertIn("build output is incomplete", process.stderr)


if __name__ == "__main__":
    unittest.main()
