"""Offline real-parser proof; set ERGOGEN_CHECKOUT and CAD_PROVIDER_PAGES_DRIVER."""
import http.client
import json
import math
import os
from pathlib import Path
import threading
import unittest
from unittest import mock

import host_probe
import verified_runtime


class HostBoundaryTests(unittest.TestCase):
    @staticmethod
    def fixture_reference():
        fixture = json.loads((verified_runtime.ROOT / 'core/tests/fixtures/ergogen-layout-parity.json').read_text())
        return [{'case': item['id'], 'config': item['ergogenConfig'],
                 'result': {p['ergogenName']: {'x': p['pose']['x'], 'y': p['pose']['y'], 'r': p['pose']['rotation']}
                            for p in item['expected'] if not p.get('absent')}} for item in fixture['cases']]

    def test_case_inventory_and_values_fail_closed(self):
        reference = self.fixture_reference()
        entries = [{'suite': host_probe.SUITE, 'case': item['case'], 'ok': True, 'result': item['result']}
                   for item in reference]
        self.assertEqual(host_probe.validate_output(json.dumps({host_probe.SUITE: entries}), reference),
                         [item['result'] for item in reference])
        for change in ('altered', 'missing', 'extra', 'nonfinite', 'empty', 'missing-case', 'duplicate-case'):
            bad = json.loads(json.dumps(entries))
            points = bad[0]['result']
            if change == 'altered': points['m_a_r0']['x'] += 1
            elif change == 'missing': del bad[1]['result'][next(iter(bad[1]['result']))]
            elif change == 'extra': points['extra'] = {'x': 0, 'y': 0, 'r': 0}
            elif change == 'nonfinite': points['m_a_r0']['x'] = math.nan
            elif change == 'empty': bad[0]['result'] = {}
            elif change == 'missing-case': bad.pop()
            else: bad[1]['case'] = bad[0]['case']
            with self.subTest(change=change), self.assertRaises(ValueError):
                host_probe.validate_output(json.dumps({host_probe.SUITE: bad}), reference)

    def test_rejects_malformed_successful_driver_results(self):
        entry = {'suite': host_probe.SUITE, 'case': 'pinned-parser', 'ok': True,
                 'result': {'m_a_r0': {'x': 10, 'y': 20, 'r': 30}}}
        self.assertEqual(host_probe.validate_output(json.dumps({host_probe.SUITE: [entry]})), entry['result'])
        invalid = [{}, {'extra': {'x': 10, 'y': 20, 'r': 30}},
                   {'m_a_r0': {'x': '10', 'y': 20, 'r': 30}},
                   {'m_a_r0': {'x': 10, 'y': 20}},
                   {'m_a_r0': {'x': True, 'y': 20, 'r': 30}},
                   {'m_a_r0': {'x': math.nan, 'y': 20, 'r': 30}},
                   {'m_a_r0': {'x': math.inf, 'y': 20, 'r': 30}}]
        for value in invalid:
            with self.subTest(value=value), self.assertRaises(ValueError):
                host_probe.validate_output(json.dumps({host_probe.SUITE: [{**entry, 'result': value}]}))

    def test_unknown_mode_fails_before_preparing(self):
        with mock.patch.object(verified_runtime, 'verified_runtime') as prepare:
            with self.assertRaisesRegex(ValueError, 'mode'):
                host_probe.run('/unused', '/unused', '/unused', mode='typo')
        prepare.assert_not_called()


class HostProbeTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.checkout = Path(os.environ['ERGOGEN_CHECKOUT'])
        cls.driver = Path(os.environ['CAD_PROVIDER_PAGES_DRIVER'])
        cls.cache = Path(os.environ.get('ERGOGEN_NPM_CACHE', str(Path.home() / '.npm/_cacache/content-v2/sha512')))
        cls.temp = Path(os.environ['TMPDIR'])

    def test_real_pinned_parser_result(self):
        self.assertEqual(host_probe.run(self.checkout, self.cache, self.driver),
                         {'m_a_r0': {'x': 10, 'y': 20, 'r': 30}})
        self.assertEqual(list(self.temp.glob('ergogen-verified-*')), [])

    def test_all_authored_cases_and_invalid_input(self):
        reference = host_probe.read_json(Path(os.environ['ERGOGEN_HOST_REFERENCE']).read_text())
        actual = host_probe.run(self.checkout, self.cache, self.driver, 'cases', reference)
        self.assertEqual(len(actual), 42)
        self.assertEqual(sum(map(len, actual)), 360)
        with self.assertRaisesRegex(RuntimeError, 'Field.*points.*object'):
            host_probe.run(self.checkout, self.cache, self.driver, 'invalid-case', reference)

    def test_browser_rejects_missing_builtin_traversal_and_parser_errors(self):
        for mode, message in [('missing', 'missing module'), ('builtin', 'unlocked or invalid module'),
                              ('traversal', 'escapes its root'), ('invalid', 'Field.*points.*object')]:
            with self.subTest(mode=mode), self.assertRaisesRegex(RuntimeError, message):
                host_probe.run(self.checkout, self.cache, self.driver, mode)
            self.assertEqual(list(self.temp.glob('ergogen-verified-*')), [])

    def test_exact_routes_and_invalid_checkout(self):
        with verified_runtime.verified_runtime(self.checkout, self.cache) as prepared:
            server = host_probe.serve(prepared, 'normal')
            thread = threading.Thread(target=server.serve_forever)
            thread.start()
            try:
                connection = http.client.HTTPConnection('127.0.0.1', server.server_port)
                for path in ('/missing', '/files/..%2F..%2Fetc%2Fpasswd', '/files/upstream%2Fmissing.js'):
                    connection.request('GET', path)
                    response = connection.getresponse()
                    self.assertEqual(response.status, 404, path)
                    response.read()
                connection.close()
            finally:
                server.shutdown()
                server.server_close()
                thread.join()
        with self.assertRaises(Exception):
            host_probe.run(self.temp / 'missing-checkout', self.cache, self.driver)
        with self.assertRaises(FileNotFoundError):
            host_probe.run(self.checkout, self.cache, self.temp / 'missing-driver')
        self.assertEqual(list(self.temp.glob('ergogen-verified-*')), [])

    def test_duplicate_result_keys_are_rejected(self):
        with self.assertRaisesRegex(ValueError, 'duplicate'):
            json.loads('{"x":1,"x":2}', object_pairs_hook=host_probe.unique)

    def test_failed_module_is_reloaded_on_retry(self):
        read_bytes = Path.read_bytes
        serve = host_probe.serve
        ready, requests = False, []
        def start(prepared, mode):
            nonlocal ready
            server = serve(prepared, mode)
            ready = True
            return server
        def controlled_read(path):
            if ready and path.name == 'units.js' and path.parent.name == 'src':
                requests.append(path)
                if len(requests) == 1:
                    return b"throw Error('controlled module failure');"
            return read_bytes(path)
        with mock.patch.object(host_probe, 'serve', side_effect=start), \
                mock.patch.object(Path, 'read_bytes', controlled_read):
            self.assertEqual(host_probe.run(self.checkout, self.cache, self.driver, 'retry'),
                             {'m_a_r0': {'x': 10, 'y': 20, 'r': 30}})
        self.assertEqual(len(requests), 2)


if __name__ == '__main__':
    unittest.main(verbosity=2)
