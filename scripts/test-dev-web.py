#!/usr/bin/env python3
"""Verify development asset proxy config and live static response behavior."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import importlib.util
import tempfile
import threading
import tomllib
import unittest
from urllib.request import urlopen

SPEC = importlib.util.spec_from_file_location("dev_web", Path(__file__).with_name("dev-web.py"))
DEV = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEV)


class DevWebTests(unittest.TestCase):
    def test_dioxus_proxies_assets_to_the_live_source_directory(self):
        config = tomllib.loads((DEV.WEB / "Dioxus.toml").read_text())
        self.assertIn("src", config["web"]["watcher"]["watch_path"])
        self.assertFalse(config["web"]["watcher"]["index_on_404"])
        self.assertNotIn("proxy", config.get("web", {}))

    def test_asset_server_serves_live_css_worker_and_fixture_with_real_mime(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            samples = {
                "m1.css": (b".board { color: red }", "text/css"),
                "core-worker/entry.js": (b"start_core_worker();", "text/javascript"),
                "fixtures/example.boardstudio": (b"PK\x03\x04fixture", "application/octet-stream"),
            }
            for relative, (body, _) in samples.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(body)
            class AppHandler(BaseHTTPRequestHandler):
                def do_GET(self):
                    body = b"dioxus-dev-page"
                    self.send_response(200)
                    self.send_header("Content-Type", "text/html")
                    self.send_header("Content-Length", str(len(body)))
                    self.end_headers()
                    self.wfile.write(body)

            app_server = ThreadingHTTPServer(("127.0.0.1", 0), AppHandler)
            app_server.daemon_threads = True
            app_thread = threading.Thread(target=app_server.serve_forever, daemon=True)
            app_thread.start()
            server = ThreadingHTTPServer(("127.0.0.1", 0),
                                         DEV.dev_handler(app_server.server_port, root))
            server.daemon_threads = True
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            try:
                for relative, (expected, mime) in samples.items():
                    with urlopen(f"http://127.0.0.1:{server.server_port}/assets/{relative}") as response:
                        self.assertEqual(response.status, 200)
                        self.assertEqual(response.headers.get_content_type(), mime)
                        self.assertEqual(response.headers["Cross-Origin-Opener-Policy"], "same-origin")
                        self.assertEqual(response.headers["Cross-Origin-Embedder-Policy"], "require-corp")
                        self.assertEqual(response.headers["Cross-Origin-Resource-Policy"], "same-origin")
                        self.assertEqual(response.read(), expected)
                with urlopen(f"http://127.0.0.1:{server.server_port}/") as response:
                    self.assertEqual(response.read(), b"dioxus-dev-page")
                    self.assertEqual(response.headers["Cross-Origin-Embedder-Policy"], "require-corp")
                css = root / "m1.css"
                changed = samples["m1.css"][0] + b"\n/* live edit */"
                css.write_bytes(changed)
                with urlopen(f"http://127.0.0.1:{server.server_port}/assets/m1.css") as response:
                    self.assertEqual(response.read(), changed)
            finally:
                server.shutdown()
                server.server_close()
                app_server.shutdown()
                app_server.server_close()


if __name__ == "__main__":
    unittest.main()
