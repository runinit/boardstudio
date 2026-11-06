#!/usr/bin/env python3
"""Node-free Ergogen fixture generator: same contract as generate-ergogen-parity-fixtures.cjs.

The pinned parser runs in Chrome from a verified private tree (verified_runtime.py); assembly,
validation and JSON.stringify-exact output happen here. See README.md for limits.
"""
import argparse
import math
import os
from pathlib import Path
import re
import secrets
import shutil
import subprocess
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import host_probe
import verified_runtime

ROOT = verified_runtime.ROOT
SOURCE = {
    'repository': 'https://github.com/ergogen/ergogen',
    'commit': verified_runtime.PIN,
    'version': verified_runtime.VERSION,
    'entry': 'src/points.js:parse',
    'license': 'MIT',
    'dependencies': {'makerjs': '0.18.2', 'mathjs': '15.2.0'},
}
TOLERANCES = {'upstream': 1e-6, 'boardStudioOnly': 1e-9}
FIXTURE = ROOT / 'core/tests/fixtures/ergogen-layout-parity.json'
LEGACY = ROOT / 'core/tests/fixtures/matrix-projections.json'
NPM_CACHE = Path.home() / '.npm/_cacache/content-v2/sha512'


# JavaScript value semantics the original generator relies on.
def number(value):
    return type(value) in (int, float)


def finite(value):
    return number(value) and math.isfinite(value)


def integer(value):
    return finite(value) and value == int(value)


def truthy(value):
    if value is None or value is False:
        return False
    if isinstance(value, str):
        return value != ''
    return not number(value) or (value == value and value != 0)


def same(a, b):
    """assert.deepStrictEqual for JSON values: no bool/number coercion, Object.is for numbers."""
    if isinstance(a, dict):
        return isinstance(b, dict) and a.keys() == b.keys() and all(same(a[key], b[key]) for key in a)
    if isinstance(a, list):
        return isinstance(b, list) and len(a) == len(b) and all(map(same, a, b))
    if number(a):
        return number(b) and ((a != a and b != b) or (a == b and math.copysign(1, a) == math.copysign(1, b)))
    return type(a) is type(b) and a == b


# JSON.stringify(value, null, indent): ECMAScript Number::toString, string quoting and key order.
def js_number(value):
    if not finite(value):
        return 'null'
    value = float(value)
    if value == 0:
        return '0'
    mantissa, _, exponent = repr(abs(value)).partition('e')
    whole, _, fraction = mantissa.partition('.')
    raw = whole + fraction
    digits = raw.lstrip('0')
    n = len(whole) + int(exponent or 0) - (len(raw) - len(digits))
    digits = digits.rstrip('0')
    k = len(digits)
    sign = '-' if value < 0 else ''
    if k <= n <= 21:
        return sign + digits + '0' * (n - k)
    if 0 < n <= 21:
        return sign + digits[:n] + '.' + digits[n:]
    if -6 < n <= 0:
        return sign + '0.' + '0' * -n + digits
    power = f"e{'+' if n > 0 else '-'}{abs(n - 1)}"
    return sign + digits[0] + ('.' + digits[1:] if k > 1 else '') + power


ESCAPES = {'"': '\\"', '\\': '\\\\', '\b': '\\b', '\f': '\\f', '\n': '\\n', '\r': '\\r', '\t': '\\t'}


def js_string(value):
    return '"' + ''.join(ESCAPES.get(char) or (f'\\u{ord(char):04x}' if ord(char) < 0x20 or 0xD800 <= ord(char) <= 0xDFFF
                                               else char) for char in value) + '"'


def js_keys(value):
    index = [key for key in value if re.fullmatch(r'0|[1-9][0-9]*', key) and int(key) < 2**32 - 1]
    return sorted(index, key=int) + [key for key in value if key not in index]


def js_json(value, indent=None, level=''):
    if value is None:
        return 'null'
    if isinstance(value, bool):
        return 'true' if value else 'false'
    if number(value):
        return js_number(value)
    if isinstance(value, str):
        return js_string(value)
    inner = level + (indent or '')
    if isinstance(value, list):
        brackets, items = '[]', [js_json(item, indent, inner) for item in value]
    elif isinstance(value, dict):
        colon = ': ' if indent else ':'
        brackets, items = '{}', [js_string(key) + colon + js_json(value[key], indent, inner) for key in js_keys(value)]
    else:
        raise TypeError(f'not a JSON value: {type(value).__name__}')
    if not items or not indent:
        return brackets[0] + ','.join(items) + brackets[1]
    return brackets[0] + '\n' + inner + (',\n' + inner).join(items) + '\n' + level + brackets[1]


def render(fixture):
    return js_json(fixture, '  ') + '\n'


def matrix_index(values):
    if not isinstance(values, list):
        raise ValueError('Invalid matrix dimensions')
    matrices, slots = {}, 0
    for matrix in values:
        identity = matrix.get('id') if isinstance(matrix, dict) else None
        if isinstance(identity, str) and identity in matrices:
            raise ValueError('Duplicate matrix identity')
        rows, columns = (matrix.get('rows'), matrix.get('columns')) if isinstance(matrix, dict) else (None, None)
        if (not isinstance(identity, str) or not identity or not integer(rows) or abs(rows) > 2**53 - 1 or rows < 1
                or not integer(columns) or abs(columns) > 2**53 - 1 or columns < 1):
            raise ValueError('Invalid matrix dimensions')
        matrices[identity] = matrix
        slots += rows * columns
    if slots > 2**53 - 1 or slots < 1:
        raise ValueError('Invalid matrix coordinate count')
    return matrices, slots


def coordinate_key(matrices, entry):
    matrix = matrices.get(entry.get('matrix')) if isinstance(entry.get('matrix'), str) else None
    row, column = entry.get('row'), entry.get('column')
    if (not matrix or not integer(row) or not integer(column)
            or not 0 <= row < matrix['rows'] or not 0 <= column < matrix['columns']):
        raise ValueError(f'Invalid mapped coordinate: {js_json(entry)}')
    return js_json([entry['matrix'], row, column])


def collect_expected(scenario, points):
    seen, coordinates = set(), set()
    matrices, slots = matrix_index(scenario['boardStudioMatrices'])
    expected = []
    for mapping in scenario['mapping']:
        name = mapping.get('ergogenName')
        if not isinstance(name, str) or not name:
            raise ValueError('Missing point name')
        if name in seen:
            raise ValueError(f'Duplicate point mapping: {name}')
        seen.add(name)
        coordinate = coordinate_key(matrices, mapping)
        if coordinate in coordinates:
            raise ValueError(f'Duplicate mapped coordinate: {coordinate}')
        coordinates.add(coordinate)
        if 'absent' in mapping and not isinstance(mapping['absent'], bool):
            raise ValueError('Absent marker must be boolean')
        if mapping.get('absent'):
            if name in points:
                raise ValueError(f'Expected absent point: {name}')
            expected.append(dict(mapping))
            continue
        if name not in points:
            raise ValueError(f'Missing point: {name}')
        point = points[name]
        if not isinstance(point, dict) or not all(finite(point.get(axis)) for axis in 'xyr'):
            raise ValueError(f'Point must be finite: {name}')
        expected.append({**mapping, 'pose': {'x': point['x'], 'y': point['y'], 'rotation': point['r']}})
    for name in points:
        if name not in seen:
            raise ValueError(f'Unmapped upstream point: {name}')
    if len(coordinates) != slots:
        raise ValueError(f"Missing matrix coordinate mapping in {scenario.get('id')}")
    return expected


def recorded_points(item):
    """The upstream points a fixture case records, keyed as points.parse returns them."""
    return {entry['ergogenName']: {'x': entry['pose']['x'], 'y': entry['pose']['y'], 'r': entry['pose']['rotation']}
            for entry in item['expected'] if not truthy(entry.get('absent'))}


def validate_fixture(fixture):
    for key, value in (('fixtureVersion', 1), ('targetDocumentFormatVersion', 7),
                       ('source', SOURCE), ('tolerances', TOLERANCES)):
        if not same(fixture.get(key), value):
            raise ValueError(f'Fixture {key} must be {js_json(value)}')
    ids = set()

    def identity(item):
        if not isinstance(item.get('id'), str) or not item['id'] or item['id'] in ids:
            raise ValueError('Missing/duplicate case identity')
        ids.add(item['id'])

    def poses(values, expected, complete=True):
        matrices, slots = matrix_index(values)
        seen = set()
        for entry in expected:
            key = coordinate_key(matrices, entry)
            if key in seen:
                raise ValueError(f'Duplicate expected coordinate: {key}')
            seen.add(key)
            pose = entry.get('pose')
            if not isinstance(pose, dict) or not all(finite(pose.get(axis)) for axis in ('x', 'y', 'rotation')):
                raise ValueError(f'Expected pose must be finite: {key}')
        if complete and len(seen) != slots:
            raise ValueError('Missing worked coordinate')

    for item in fixture['cases']:
        identity(item)
        if not same(collect_expected(item, recorded_points(item)), item['expected']):
            raise ValueError(f"Expected points do not match their mapping: {item['id']}")
        if item.get('comparison') == 'intentional-divergence':
            differences = item.get('differences')
            if not isinstance(differences, list) or not differences:
                raise ValueError('Divergence needs independent BoardStudio expectations')
            poses(item['boardStudioMatrices'], differences, False)
            for entry in differences:
                counterpart = next((point for point in item['expected'] if all(
                    same(point.get(key), entry.get(key)) for key in ('matrix', 'row', 'column'))), None)
                if not counterpart or not isinstance(counterpart.get('pose'), dict) or all(
                        abs(entry['pose'][axis] - counterpart['pose'][axis]) <= fixture['tolerances']['upstream']
                        for axis in ('x', 'y', 'rotation')):
                    raise ValueError('Declared divergence does not differ from its upstream point')
        elif item.get('comparison') != 'match':
            raise ValueError('Unknown comparison kind')
    for item in fixture['boardStudioOnly']:
        identity(item)
        if not truthy(item.get('basis')):
            raise ValueError('Worked example needs its independent basis')
        if truthy(item.get('beforeMatrices')):
            poses(item['beforeMatrices'], item['expectedBefore'])
            poses(item['afterMatrices'], item['expectedAfter'])
        else:
            poses(item['boardStudioMatrices'], item['expected'])


def assemble(source, actual):
    """Fixture from evaluated cases.cjs and per-case upstream points, as the original generate()."""
    if not isinstance(actual, list) or len(actual) != len(source['cases']):
        raise ValueError('Oracle case count mismatch')
    ids, cases = set(), []
    for scenario, points in zip(source['cases'], actual):
        if scenario.get('id') in ids:
            raise ValueError(f"Duplicate case identity: {scenario.get('id')}")
        ids.add(scenario.get('id'))
        if not isinstance(points, dict):
            raise ValueError(f"Oracle points must be an object: {scenario.get('id')}")
        cases.append({**scenario, 'expected': collect_expected(scenario, points)})
    fixture = {'fixtureVersion': 1, 'targetDocumentFormatVersion': 7, 'source': SOURCE,
               'tolerances': TOLERANCES, 'cases': cases, 'boardStudioOnly': source['boardStudioOnly']}
    validate_fixture(fixture)
    return fixture


def real_destination(destination):
    existing, suffix = os.path.abspath(destination), []
    while not os.path.exists(existing):
        parent = os.path.dirname(existing)
        if parent == existing:
            raise ValueError(f'Cannot resolve path: {destination}')
        suffix.insert(0, os.path.basename(existing))
        existing = parent
    return os.path.join(os.path.realpath(existing), *suffix)


def inside(path, root):
    return path == root or path.startswith(root.rstrip(os.sep) + os.sep)


def bootstrap(cache):
    """Create a missing checkout of the pin, as the original; existing ones are never changed."""
    if inside(real_destination(cache), os.path.realpath(ROOT)):
        raise ValueError('The pinned source checkout must be outside the repository')
    if os.path.lexists(cache):
        return
    # shortcut: an interrupted fetch leaves this new cache for inspection, as the original does.
    cache.parent.mkdir(parents=True, exist_ok=True)
    env = {key: value for key, value in os.environ.items() if not key.startswith('GIT_')}
    env.update(GIT_CONFIG_NOSYSTEM='1', GIT_CONFIG_GLOBAL=os.devnull, GIT_TERMINAL_PROMPT='0')
    for args in (['init', '--quiet', str(cache)],
                 ['-C', str(cache), 'fetch', '--quiet', '--depth=1', SOURCE['repository'], SOURCE['commit']],
                 ['-C', str(cache), 'checkout', '--quiet', '--detach', 'FETCH_HEAD']):
        subprocess.run(['git', '-c', 'core.hooksPath=/dev/null', *args], env=env, check=True)


def build_driver():
    """Build cad-provider-pages from this checkout under the shared build lock."""
    sys.path.insert(0, str(ROOT / 'scripts'))
    import browser_tools
    os.environ.setdefault('CHROME_BIN', browser_tools.locate_chrome())
    return browser_tools._build_harness('cad-provider-pages')


def generate(cache, npm_cache=NPM_CACHE, driver=None, *, prepared=None):
    payload = host_probe.run(cache, npm_cache, driver or build_driver, 'generate', prepared=prepared)
    return assemble(payload, payload['actual'])


def write_fixture(output, text):
    output = Path(output)
    output.parent.mkdir(parents=True, exist_ok=True)
    while True:
        temporary = output.parent / ('.ergogen-parity-' + secrets.token_hex(16))
        try:
            fd = os.open(temporary, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o666)
            break
        except FileExistsError:
            continue
    try:
        with os.fdopen(fd, 'wb') as file:
            file.write(text.encode('utf-8'))
        os.replace(temporary, output)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def main(argv=None, *, prepared=None):
    parser = argparse.ArgumentParser(
        description='Generate or --check the Ergogen layout-parity fixture without Node. '
                    '--check regenerates in memory and refuses differences without writing.')
    parser.add_argument('--cache', type=Path, default=Path.home() / '.cache/boardstudio/ergogen' / SOURCE['commit'],
                        help='external pinned Ergogen checkout (created by a pinned fetch if missing)')
    parser.add_argument('--output', type=Path, default=FIXTURE)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--npm-cache', type=Path, default=NPM_CACHE,
                        help='npm content-v2/sha512 directory holding the locked archives (never downloaded)')
    parser.add_argument('--driver', type=Path,
                        help='cad-provider-pages already built from this checkout (default: build it)')
    args = parser.parse_args(argv)
    try:
        cache, output = Path(os.path.abspath(args.cache)), Path(os.path.abspath(args.output))
        destination = real_destination(output)
        if destination == real_destination(LEGACY):
            raise ValueError('Refusing to overwrite the current projection oracle before its replacement is wired')
        if inside(destination, real_destination(cache)):
            raise ValueError('Fixture output must not modify the pinned source checkout')
        if prepared is None:
            bootstrap(cache)
        fixture = render(generate(cache, args.npm_cache, args.driver, prepared=prepared))
        if args.check:
            if output.read_bytes() != fixture.encode('utf-8'):
                raise ValueError(f'Fixture differs: {output}')
            print(f'Fixture is reproducible: {output}')
            return 0
        write_fixture(output, fixture)
        print(f'Wrote {output}')
        return 0
    except Exception as error:  # every failure, including browser and Git, exits nonzero without writing
        print(f'Ergogen fixture generation failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
