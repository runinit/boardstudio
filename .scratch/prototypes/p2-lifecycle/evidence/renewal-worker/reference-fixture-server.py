from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path


FIXTURE = Path(__file__).parents[2] / "fixtures" / "reviung41.json"


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path != "/reviung41.json":
            self.send_error(404)
            return
        body = FIXTURE.read_bytes()
        self.send_response(200)
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, fmt, *args):
        pass


HTTPServer(("127.0.0.1", 4396), Handler).serve_forever()
