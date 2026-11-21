"""Serve packaged CAD provider suites to a single isolated-page browser driver."""
import hashlib
import http.server
from pathlib import Path
import sys
from urllib.parse import urlsplit

sys.path.insert(0, str(Path(__file__).resolve().parents[2] / "scripts"))
import browser_tools  # noqa: E402
import cad_build

CAD = Path(__file__).resolve().parents[1]
ROOT = CAD.parent
TESTS = CAD / 'wasm/tests'
SUITES = {
    'provider-support-controls': ['truth', 'equal', 'deepEqual', 'match', 'throws', 'rejects'],
    'collision-analysis': ['main'],
    'step-face-completeness': ['sliver'],
    'step-repair-accounting': ['main'],
    'step-zero-repair': [
        'actual WASM integer cuboid reports complete zero and unchanged default/provenance',
        'independently authored shifted cuboid and reversed native senses qualify',
        'explicit precision, budget and refinement remain separate from zero movement',
        'rounded source lexemes, inconsistent native LINE and selected PCurves stay null',
        'mixed asset cannot borrow a supported cuboid proof',
        'existing tetra plane-gap default and explicit budget policy stays unchanged',
        'unretained styled finite curve and alternate presentation roots explicitly refuse',
        'styles of retained finite geometry preserve complete mixed and surface assets',
    ],
    'step-projected-line-repair': ['main'],
    'step-planar-seam-repair': ['main'],
    'step-complete-surface-preview': ['main'],
    'qualified-analysis': ['main'],
    'measure-cad-worker': [
        'preview', 'step', 'import', 'validation', 'malformed', 'null', 'missing', 'identity', 'epoch',
        'token', 'revision', 'mesh', 'bodies', 'failure', 'correlation', 'step-fault', 'vendor', 'source-sha',
    ],
}
STATIC_ASSERT_SITES = {
    'collision-analysis': 66,
    'step-repair-accounting': 45,
    'step-zero-repair': 59,
    'step-projected-line-repair': 16,
    'step-planar-seam-repair': 16,
    'step-complete-surface-preview': 39,
    'qualified-analysis': 102,
    'measure-cad-worker': 13,
}
FIXTURE_NAMES = [
    'cad/wasm/tests/fixtures/generated-analysis-contract.json',
    'cad/test/fixtures/holed-plate.json',
    'cad/test/fixtures/performance/C1plate.json',
    'cad/test/fixtures/followup-manifest.json',
    'cad/scripts/measure-cad-worker.cjs',
    'cad/monstertruck/tests/fixtures/pcb-source/circle-tracer.json',
    *('cad/monstertruck/tests/fixtures/' + name for name in (
        'tetra-mm.step', 'two-occurrences-mm.step', 'repair-finite-domain.step',
        'unsupported-non-ruled-sphere.step', 'zero-repair-cuboid-mm.step',
        'zero-repair-cuboid-offset-mm.step', 'source-pcurve-cube.step',
        'circular-segment-sliver-prism.step', 'tetra-and-finite-wires.step',
        'repair-projected-line.step', 'repair-planar-seam.step', 'touching-unit-boxes.step')),
    'cad/monstertruck/tests/fixtures/solid32377-source.step',
    'ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step',
    'ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step',
]
FIXTURES = {name: ROOT / name for name in FIXTURE_NAMES}
RUN_JS = b'''import init, * as cad from '/pkg/boardstudio_cad_wasm.js';
export async function run(suite) {
  const memory = await init({module_or_path: '/pkg/boardstudio_cad_wasm_bg.wasm'});
  const log = (...args) => {
    const request = new XMLHttpRequest();
    request.open('POST', '/output', false);
    request.send(args.join(' '));
    if (request.status !== 204) throw Error('cannot forward regression output');
  };
  console.log = log;
  return (await import('/tests/' + suite + '.mjs')).run(cad, memory);
}
'''


def preflight(cad=CAD):
    package = cad / "wasm/pkg/boardstudio_cad_wasm.js"
    wasm = package.with_name("boardstudio_cad_wasm_bg.wasm")
    stamp = cad / ".cache/cad/wasm-build.sha256"
    if not all(path.is_file() for path in (package, wasm, stamp)):
        raise RuntimeError("CAD WEB package or freshness stamp is missing")
    if stamp.read_text().strip() not in (cad_build.wasm_inputs_digest(no_opt=True), cad_build.wasm_inputs_digest()):
        raise RuntimeError("CAD WEB package is stale")
    return package, wasm


def sliver_module():
    # Share the regression body with the existing standalone test.
    suite = 'step-face-completeness'
    source = (TESTS / (suite + '.cjs')).read_text()
    start = '// Display mesh must be the f32 image'
    end = '\n\n})().catch('
    if source.count(start) != 1 or source.count(end) != 1:
        raise ValueError('sliver regression source boundary changed')
    body = source[source.index(start):source.index(end)]
    return f'''import {{assert, readBytes, sha256Hex}} from '/tests/provider-support.mjs';
export async function run(cad) {{
  const bytes = await readBytes('cad/monstertruck/tests/fixtures/circular-segment-sliver-prism.step');
  const result = cad.read_step_model(new Uint8Array(bytes), {{sourceSha256: await sha256Hex(bytes)}});
{body}
  return [{{suite: '{suite}', case: 'sliver', ok: true}}];
}}
'''.encode()


def validate_results(results, suites):
    if not isinstance(results, dict) or set(results) != set(suites):
        raise RuntimeError(f'invalid suite inventory: {results!r}')
    for suite in suites:
        entries = results[suite]
        if not isinstance(entries, list) or len(entries) != len(SUITES[suite]):
            raise RuntimeError(f'invalid case inventory for {suite}: {entries!r}')
        names = []
        for entry in entries:
            if (not isinstance(entry, dict) or set(entry) != {'suite', 'case', 'ok'}
                    or entry.get('suite') != suite or entry.get('ok') is not True
                    or not isinstance(entry.get('case'), str) or not entry['case']):
                raise RuntimeError(f'invalid result for {suite}: {entry!r}')
            names.append(entry['case'])
        if len(set(names)) != len(names) or set(names) != set(SUITES[suite]):
            raise RuntimeError(f'invalid case inventory for {suite}: {names!r}')


def run(suites=None):
    suites = list(SUITES if suites is None else suites)
    if not suites or len(set(suites)) != len(suites) or any(s not in SUITES for s in suites):
        raise ValueError('unknown, empty, or duplicate suite selection')
    package, wasm = preflight()
    routes = {
        '/pkg/boardstudio_cad_wasm.js': ('text/javascript', package.read_bytes()),
        '/pkg/boardstudio_cad_wasm_bg.wasm': ('application/wasm', wasm.read_bytes()),
        '/run.js': ('text/javascript', RUN_JS),
    }
    modules = {'provider-support', *SUITES}
    if 'step-face-completeness' in suites:
        routes['/tests/step-face-completeness.mjs'] = ('text/javascript', sliver_module())

    class Handler(http.server.BaseHTTPRequestHandler):
        def end_headers(self):
            self.send_header('Cross-Origin-Opener-Policy', 'same-origin')
            self.send_header('Cross-Origin-Embedder-Policy', 'require-corp')
            self.send_header('Cross-Origin-Resource-Policy', 'same-origin')
            self.send_header('Cache-Control', 'no-store')
            super().end_headers()

        def do_GET(self):
            path = urlsplit(self.path).path
            key = path.removeprefix('/fixture/') if path.startswith('/fixture/') else None
            try:
                if path == '/':
                    content_type, data = 'text/html', b'<!doctype html><title>CAD provider suites</title>'
                elif key in FIXTURES:
                    content_type, data = 'application/octet-stream', FIXTURES[key].read_bytes()
                elif path in routes:
                    content_type, data = routes[path]
                elif path.startswith('/tests/') and path.endswith('.mjs') and path[7:-4] in modules:
                    content_type, data = 'text/javascript', (TESTS / (path[7:-4] + '.mjs')).read_bytes()
                else:
                    self.send_error(404)
                    return
            except OSError as error:
                self.send_error(500, str(error))
                return
            self.send_response(200)
            self.send_header('Content-Type', content_type)
            self.send_header('Content-Length', str(len(data)))
            if key in FIXTURES:
                self.send_header('X-Fixture-SHA256', hashlib.sha256(data).hexdigest())
            self.end_headers()
            self.wfile.write(data)

        def do_POST(self):
            try:
                length = int(self.headers.get('Content-Length', '0'))
                if self.path != '/output' or not 0 < length <= 65536:
                    self.send_error(400)
                    return
                text = self.rfile.read(length).decode('utf-8')
                print(text, flush=True)
            except (ValueError, UnicodeError, OSError):
                self.send_error(500, 'output relay failed')
                return
            self.send_response(204)
            self.end_headers()

        def log_message(self, *_args):
            pass

    with browser_tools.serve(Handler) as url:
        results = browser_tools.run_provider_page(url + '/', suites, timeout=900)
    validate_results(results, suites)


if __name__ == '__main__':
    try:
        run(sys.argv[1:] or None)
    except (OSError, ValueError, RuntimeError) as error:
        sys.exit(str(error))
