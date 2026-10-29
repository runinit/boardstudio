#!/usr/bin/env python3
"""Hermetic checks for the opt-in packaged WEB provider runner."""
import contextlib
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest import mock

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("face_web", HERE / "face-completeness-web.py")
face_web = importlib.util.module_from_spec(spec)
spec.loader.exec_module(face_web)


class FaceWebTests(unittest.TestCase):
    def test_source_mapping_uses_original_assertions_and_release(self):
        source = (HERE.parent / "wasm/tests/step-face-completeness.cjs").read_text()
        mapped = face_web.regression_body(source)
        self.assertEqual(mapped, source[source.index('// Display mesh'):source.index('\n\n})().catch')])
        self.assertIn("fallback meshed faces count", mapped)
        self.assertIn("Math.fround", mapped)
        self.assertIn("triangle tag must be valid", mapped)

    def test_source_mapping_rejects_missing_or_duplicate_boundaries(self):
        source = (HERE.parent / "wasm/tests/step-face-completeness.cjs").read_text()
        for bad in (source.replace('// Display mesh', '// missing'), source + '\n// Display mesh'):
            with self.subTest(bad=bad[-30:]), self.assertRaisesRegex(ValueError, "source boundary"):
                face_web.regression_body(bad)

    def test_source_mapping_rejects_checks_outside_mapped_body(self):
        source = (HERE.parent / "wasm/tests/step-face-completeness.cjs").read_text()
        for bad in (source.replace('const result =', 'assert(false);\nconst result ='),
                    source + 'assert(false);\n'):
            with self.subTest(bad=bad[-50:]), self.assertRaisesRegex(ValueError, "source boundary"):
                face_web.regression_body(bad)

        added = source.replace('// Display mesh', 'assert(false);\n// Display mesh')
        self.assertIn('assert(false);', face_web.regression_body(added))

    def test_package_preflight_requires_all_files_and_current_stamp(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package = root / "wasm/pkg/boardstudio_cad_wasm.js"
            wasm = package.with_name("boardstudio_cad_wasm_bg.wasm")
            stamp = root / ".cache/cad/wasm-build.sha256"
            package.parent.mkdir(parents=True)
            stamp.parent.mkdir(parents=True)
            with mock.patch.object(face_web.cad_build, "wasm_inputs_digest", side_effect=lambda no_opt=False: "fresh" if no_opt else "release"):
                with self.assertRaisesRegex(RuntimeError, "missing"):
                    face_web.preflight(root)

                package.write_text("package")
                wasm.write_bytes(b"\0asm\x01\0\0\0")
                stamp.write_text("stale")
                with self.assertRaisesRegex(RuntimeError, "stale"):
                    face_web.preflight(root)
                stamp.write_text("fresh\n")
                self.assertEqual(face_web.preflight(root), (package, wasm))
                package.unlink()
                with self.assertRaisesRegex(RuntimeError, "missing"):
                    face_web.preflight(root)

    def test_provider_initialization_precedes_fixture_fetch(self):
        module = face_web.browser_module('')
        self.assertLess(module.index('await init('), module.index("fetch('/fixture.step')"))

    def test_driver_diagnostics_are_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            package, wasm, driver = (root / name for name in ('package.js', 'package.wasm', 'driver'))
            package.write_text('package')
            wasm.write_bytes(b'wasm')
            for code, stdout, stderr in (
                (0, '[{"suite":"step-face-completeness","case":"sliver","ok":true}]', 'success diagnostic'),
                (1, 'failure stdout', 'failure stderr'),
            ):
                driver.write_text(f'#!/usr/bin/python3\nimport sys\nsys.stdout.write({stdout!r})\nsys.stderr.write({stderr!r})\nsys.exit({code})\n')
                driver.chmod(0o755)
                captured_out, captured_err = io.StringIO(), io.StringIO()
                with self.subTest(code=code), mock.patch.object(face_web, 'preflight', return_value=(package, wasm)), \
                     mock.patch.dict(os.environ, {'CAD_FACE_WEB_DRIVER': str(driver)}), \
                     contextlib.redirect_stdout(captured_out), contextlib.redirect_stderr(captured_err):
                    if code:
                        with self.assertRaises(RuntimeError) as error:
                            face_web.run()
                        self.assertIn(stdout, str(error.exception))
                        self.assertIn(stderr, str(error.exception))
                    else:
                        face_web.run()
                        self.assertEqual(captured_out.getvalue(), '')
                        self.assertIn(stderr, captured_err.getvalue())

    @unittest.skipUnless(os.environ.get("CAD_FACE_NODE_REFERENCE"), "local Node comparison outside the browser gate")
    def test_assertion_diagnostics_match_local_node_reference(self):
        module = face_web.browser_module('')
        helper = module[module.index('function fail('):module.index('export async function run()')]
        script = r'''
const nodeAssert = require('node:assert/strict');
const vm = require('node:vm');
const browserAssert = vm.runInNewContext(process.argv[1] + '\nassert');
const cases = [
  [a => a(false, 'first failure'), 'first failure', 'false', 'true', '=='],
  [a => a(false), null, 'false', 'true', '=='],
  [a => a.equal(0, -0, 'signed zero'), 'signed zero', '0', '-0', 'strictEqual'],
  [a => a.equal(-0, 0, 'reverse zero'), 'reverse zero', '-0', '0', 'strictEqual'],
  [a => a.equal(4, 5, 'face count'), 'face count', '4', '5', 'strictEqual'],
  [a => a.equal('4', 4, 'types'), 'types', '"4"', '4', 'strictEqual'],
  [a => a.equal(NaN, 4, 'NaN'), 'NaN', 'null', '4', 'strictEqual'],
];
function value(v) { return Object.is(v, -0) ? '-0' : JSON.stringify(v); }
for (const [check, message, actual, expected, operator] of cases) {
  let reference, observed;
  try { check(nodeAssert); throw Error('Node assertion passed'); } catch (e) { reference = e; }
  try { check(browserAssert); throw Error('browser assertion passed'); } catch (e) { observed = e; }
  nodeAssert.equal(reference.name, 'AssertionError');
  nodeAssert.equal(reference.code, 'ERR_ASSERTION');
  nodeAssert.equal(reference.operator, operator);
  nodeAssert.equal(value(reference.actual), actual);
  nodeAssert.equal(value(reference.expected), expected);
  nodeAssert.equal(observed.name, reference.name);
  nodeAssert.equal(observed.constructor.name, reference.constructor.name);
  nodeAssert.equal(Object.getPrototypeOf(observed.constructor.prototype).constructor.name, 'Error');
  nodeAssert.equal(observed.code, reference.code);
  nodeAssert.equal(observed.operator, operator);
  nodeAssert.equal(value(observed.actual), actual);
  nodeAssert.equal(value(observed.expected), expected);
  if (message !== null) nodeAssert.ok(observed.message.includes(message));
  const renderedActual = Number.isNaN(observed.actual) ? 'NaN' : actual;
  for (const field of ['ERR_ASSERTION', `actual: ${renderedActual}`, `expected: ${expected}`, `operator: ${operator}`])
    nodeAssert.ok(observed.stack.includes(field), field);
}
browserAssert.equal(NaN, NaN);
let first;
try { browserAssert(false, 'first failure'); browserAssert(false, 'second failure'); } catch (e) { first = e; }
nodeAssert.ok(first.stack.includes('first failure'));
nodeAssert.ok(!first.stack.includes('second failure'));
'''
        subprocess.run(["node", "-e", script, helper], check=True)

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_browser_assertion_diagnostics_reach_consumer(self):
        original = face_web.regression_body
        cases = (
            ("assert(false, 'first failure'); assert(false, 'second failure');", 'first failure', 'false', 'true', '=='),
            ("assert(false);", None, 'false', 'true', '=='),
            ("assert.equal(0, -0, 'signed zero');", 'signed zero', '0', '-0', 'strictEqual'),
            ("assert.equal(-0, 0, 'reverse zero');", 'reverse zero', '-0', '0', 'strictEqual'),
            ("assert.equal(false, 0, 'types');", 'types', 'false', '0', 'strictEqual'),
            ("assert.equal(NaN, 4, 'NaN mismatch');", 'NaN mismatch', 'NaN', '4', 'strictEqual'),
        )
        for statement, message, actual, expected, operator in cases:
            with self.subTest(statement=statement), mock.patch.object(face_web, 'regression_body', lambda source: statement + '\n' + original(source)):
                with self.assertRaises(RuntimeError) as error:
                    face_web.run()
                text = str(error.exception)
                for field in ('AssertionError', 'ERR_ASSERTION', f'actual: {actual}', f'expected: {expected}', f'operator: {operator}'):
                    self.assertIn(field, text)
                if message is not None:
                    self.assertIn(message, text)
                self.assertNotIn('second failure', text)

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_real_browser_failure_controls(self):
        original = face_web.regression_body

        def wrong_face_count(source):
            body = original(source)
            expected = "assert.equal(vs.sourceFaces.length, 4, 'sliver fixture has 4 source faces');"
            self.assertEqual(body.count(expected), 1)
            return body.replace(expected, expected.replace("length, 4", "length, 5"))

        controls = (
            ("regression_body", wrong_face_count, "sliver fixture has 4 source faces"),
            ("regression_body", lambda source: original(source) + "\nassert(false, 'late assertion');", "late assertion"),
            ("browser_module", lambda body: "export async function run() { throw Error('controlled rejection'); }", "controlled rejection"),
            ("browser_module", lambda body: "export async function run() { return []; }", "invalid face completeness suite/case result"),
        )
        for target, replacement, message in controls:
            with self.subTest(message=message), mock.patch.object(face_web, target, replacement):
                with self.assertRaisesRegex(RuntimeError, message) as error:
                    face_web.run()
                if message == 'sliver fixture has 4 source faces':
                    self.assertIn('actual: 4', str(error.exception))
                    self.assertIn('expected: 5', str(error.exception))

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_output_survives_failure_after_original_log(self):
        original = face_web.regression_body
        def fail_after_output(source):
            body = original(source)
            marker = 'vs.analysisHandle.free();'
            self.assertEqual(body.count(marker), 1)
            return body.replace(marker, "throw Error('controlled release failure');\n" + marker)

        output = io.StringIO()
        with contextlib.redirect_stdout(output), mock.patch.object(face_web, 'regression_body', fail_after_output):
            with self.assertRaisesRegex(RuntimeError, 'controlled release failure'):
                face_web.run()
        self.assertRegex(output.getvalue(), r'PASS step-face-completeness: sliver fixture retains \d+ f64 triangles across 4 source faces\n')

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_output_transport_and_broken_stdout_fail_before_release(self):
        original_body = face_web.regression_body
        original_module = face_web.browser_module

        def fail_on_release(source):
            body = original_body(source)
            marker = 'vs.analysisHandle.free();'
            self.assertEqual(body.count(marker), 1)
            return body.replace(marker, "throw Error('controlled release failure');\n" + marker)

        def reject_output(body):
            module = original_module(body)
            self.assertEqual(module.count("'/output'"), 1)
            return module.replace("'/output'", "'/invalid-output'")

        class BrokenOutput(io.StringIO):
            attempts = 0

            def write(self, text):
                self.attempts += 1
                raise BrokenPipeError('controlled broken stdout')

        with mock.patch.object(face_web, 'regression_body', fail_on_release):
            with mock.patch.object(face_web, 'browser_module', reject_output):
                with self.assertRaisesRegex(RuntimeError, 'cannot forward regression output') as error:
                    face_web.run()
                self.assertNotIn('controlled release failure', str(error.exception))

            output = BrokenOutput()
            with contextlib.redirect_stdout(output), contextlib.redirect_stderr(io.StringIO()):
                with self.assertRaises(RuntimeError) as error:
                    face_web.run()
            self.assertGreater(output.attempts, 0)
            self.assertNotIn('controlled release failure', str(error.exception))

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_fixture_and_wasm_failure_order(self):
        read_bytes = Path.read_bytes
        for corrupt in (False, True):
            reads = []
            def controlled_read(path):
                if path.name == 'circular-segment-sliver-prism.step':
                    reads.append('fixture')
                    raise FileNotFoundError('controlled missing fixture')
                if corrupt and path.name == 'boardstudio_cad_wasm_bg.wasm':
                    return b'corrupt wasm'
                return read_bytes(path)

            with self.subTest(corrupt=corrupt), mock.patch.object(Path, 'read_bytes', controlled_read):
                with self.assertRaises(RuntimeError) as error:
                    face_web.run()
                if corrupt:
                    self.assertNotIn('controlled missing fixture', str(error.exception))
                    self.assertEqual(reads, [])
                else:
                    self.assertIn('controlled missing fixture', str(error.exception))
                    self.assertEqual(reads, ['fixture'])

    @unittest.skipUnless(os.environ.get("CAD_FACE_WEB_DRIVER"), "requires the real browser driver and package")
    def test_detached_errors_before_completion_fail(self):
        cases = (
            ('error', "throw Error('detached exception')", 'detached exception'),
            ('unhandledrejection', "Promise.reject(Error('late rejection'))", 'late rejection'),
        )
        for event, action, message in cases:
            module = f'''export async function run() {{
              const observed = new Promise(resolve => window.addEventListener('{event}', resolve, {{once:true}}));
              setTimeout(() => {{ {action}; }}, 0);
              await observed;
              return [{{suite:'step-face-completeness',case:'sliver',ok:true}}];
            }}'''
            with self.subTest(message=message), mock.patch.object(face_web, 'browser_module', return_value=module):
                with self.assertRaisesRegex(RuntimeError, message):
                    face_web.run()


if __name__ == "__main__":
    unittest.main()
