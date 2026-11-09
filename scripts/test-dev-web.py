#!/usr/bin/env python3
"""Verify development asset proxy config and live static response behavior."""

from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import importlib.util
import tempfile
import threading
import tomllib
import unittest
from urllib.error import HTTPError
from urllib.request import Request, urlopen

SPEC = importlib.util.spec_from_file_location("dev_web", Path(__file__).with_name("dev-web.py"))
DEV = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(DEV)


class DevWebTests(unittest.TestCase):
    def test_dioxus_proxies_assets_to_the_live_source_directory(self):
        config = tomllib.loads((DEV.WEB / "Dioxus.toml").read_text())
        self.assertIn("src", config["web"]["watcher"]["watch_path"])
        self.assertFalse(config["web"]["watcher"]["index_on_404"])
        self.assertNotIn("proxy", config.get("web", {}))

    def test_staged_vendor_models_load_without_allowing_asset_escape(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            assets = root / "assets"
            models = assets / "ergogen-models"
            models.mkdir(parents=True)
            vendor = root / "vendor"
            vendor.mkdir()
            model = vendor / "switch.stp"
            model.write_bytes(b"ISO-10303-21; model fixture")
            secret = root / "private.txt"
            secret.write_bytes(b"must not be served")
            (models / "model-switch.stp").symlink_to(model)
            (models / "outside.stp").symlink_to(secret)
            (assets / "outside.txt").symlink_to(secret)
            (assets / "unstaged.stp").symlink_to(model)
            server = ThreadingHTTPServer(
                ("127.0.0.1", 0), DEV.dev_handler(0, assets, model_root=vendor))
            server.daemon_threads = True
            thread = threading.Thread(target=server.serve_forever, daemon=True)
            thread.start()
            base = f"http://127.0.0.1:{server.server_port}/assets/"
            try:
                for method in ("GET", "HEAD"):
                    with urlopen(Request(base + "ergogen-models/model-switch.stp", method=method)) as response:
                        self.assertEqual(response.status, 200)
                        self.assertEqual(int(response.headers["Content-Length"]), model.stat().st_size)
                        self.assertEqual(response.headers["Cross-Origin-Embedder-Policy"], "require-corp")
                        self.assertEqual(response.read(), model.read_bytes() if method == "GET" else b"")
                for path in ("ergogen-models/outside.stp", "outside.txt", "unstaged.stp",
                             "../private.txt", "%2e%2e/private.txt",
                             "ergogen-models/../../vendor/switch.stp"):
                    with self.subTest(path=path), self.assertRaises(HTTPError) as error:
                        urlopen(base + path)
                    self.assertEqual(error.exception.code, 404)
                    error.exception.close()
            finally:
                server.shutdown()
                server.server_close()

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
                    generated = self.path == "/assets/focus-trap-dx123.js"
                    body = b"export const focusTrap = true;" if generated else b"dioxus-dev-page"
                    self.send_response(200)
                    self.send_header("Content-Type", "text/javascript" if generated else "text/html")
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
                with urlopen(f"http://127.0.0.1:{server.server_port}/assets/focus-trap-dx123.js") as response:
                    self.assertEqual(response.headers.get_content_type(), "text/javascript")
                    self.assertEqual(response.read(), b"export const focusTrap = true;")
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
