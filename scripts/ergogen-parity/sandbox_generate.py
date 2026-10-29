#!/usr/bin/env python3
"""Run generate.py with Node masked, loopback-only networking and a minimal filesystem view.

Before entering the sandbox: build cad-provider-pages from this checkout, verify the installed
Chrome for Testing tree byte-for-byte against the pinned archive, and copy only the locked npm
archives into a private run directory. Chrome itself still runs with --no-sandbox (shared driver);
this outer namespace is the boundary. Requires an existing pinned checkout (no fetch inside).
"""
import argparse
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import stat
import subprocess
import sys
import tempfile
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parent))
import generate
import verified_runtime

ROOT = verified_runtime.ROOT
PIN = json.loads((ROOT / 'scripts/chrome-for-testing.json').read_text())
CFT = Path(os.environ.get('BOARDSTUDIO_CFT_DIR', Path.home() / '.cache/boardstudio/chrome-for-testing' / PIN['version']))
CFT_ZIP = Path(os.environ.get('BOARDSTUDIO_CFT_ZIP', CFT.parent / f"chrome-{PIN['version']}.zip"))


def sha256(path):
    digest = hashlib.sha256()
    with open(path, 'rb') as file:
        for block in iter(lambda: file.read(1 << 20), b''):
            digest.update(block)
    return digest.hexdigest()


def verified_chrome(tree=CFT, archive=CFT_ZIP, expected=PIN['chrome_sha256']):
    """The pinned chrome binary, once every installed file equals its pinned archive member."""
    if sha256(archive) != expected:
        raise ValueError(f'Chrome archive does not match the pinned SHA-256: {archive}')
    with zipfile.ZipFile(archive) as bundle:
        members = {info.filename: info for info in bundle.infolist() if not info.is_dir()}
        disk = {}
        for directory, _, names in os.walk(tree / 'chrome-linux64'):
            for name in names:
                path = Path(directory, name)
                disk[path.relative_to(tree).as_posix()] = path
        if disk.keys() != members.keys():
            raise ValueError('Installed Chrome files differ from the pinned archive inventory')
        for name, path in disk.items():
            if not stat.S_ISREG(path.lstat().st_mode):
                raise ValueError(f'Installed Chrome entry is not a regular file: {name}')
            with bundle.open(members[name]) as packed, open(path, 'rb') as installed:
                while block := packed.read(1 << 20):
                    if installed.read(len(block)) != block:
                        raise ValueError(f'Installed Chrome file differs from the pinned archive: {name}')
                if installed.read(1):
                    raise ValueError(f'Installed Chrome file differs from the pinned archive: {name}')
    return tree / 'chrome-linux64/chrome'


def locked_archives(npm_cache, destination):
    """Copy only lockfile archives (verified again inside by verified_runtime)."""
    lock = json.loads(verified_runtime.LOCK.read_text(encoding='utf-8'))
    for name, metadata in lock['packages'].items():
        if name:
            digest = base64.b64decode(metadata['integrity'].removeprefix('sha512-'), validate=True).hex()
            relative = Path(digest[:2], digest[2:4], digest[4:])
            (destination / relative).parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(Path(npm_cache) / relative, destination / relative)


def command(run, checkout, driver, chrome, output, check):
    """unshare/bwrap argv: loopback only, Node masked, read-only inputs, one writable output dir.

    TMPDIR is the sandbox's private /tmp: short enough for Chrome's singleton socket path.
    """
    output_dir = os.path.realpath(output.parent)
    binds = ['--ro-bind', '/usr', '/usr', '--ro-bind', '/etc', '/etc',
             *(arg for link in ('bin', 'sbin', 'lib', 'lib64') for arg in ('--symlink', os.readlink('/' + link), '/' + link)),
             '--proc', '/proc', '--dev', '/dev', '--tmpfs', '/dev/shm', '--tmpfs', '/tmp',
             '--ro-bind', str(ROOT), str(ROOT), '--ro-bind', str(checkout), str(checkout),
             '--ro-bind', str(chrome.parent), str(chrome.parent), '--ro-bind', str(driver), str(driver),
             '--ro-bind' if check else '--bind', output_dir, output_dir,
             '--ro-bind', str(run / 'npm'), '/npm', '--ro-bind', '/dev/null', '/usr/bin/node']
    inner = ['bwrap', '--die-with-parent', '--unshare-pid', '--unshare-ipc', '--unshare-uts', *binds,
             '--clearenv', '--setenv', 'PATH', '/usr/bin', '--setenv', 'HOME', '/tmp', '--setenv', 'TMPDIR', '/tmp',
             '--setenv', 'LANG', 'C.UTF-8', '--setenv', 'PYTHONDONTWRITEBYTECODE', '1', '--setenv', 'CHROME_BIN', str(chrome),
             '/bin/sh', '-euc', 'if command -v node || command -v nodejs; then echo "node reachable" >&2; exit 97; fi; exec "$@"', 'sh',
             '/usr/bin/python3', '-I', str(Path(__file__).with_name('generate.py')), '--cache', str(checkout),
             '--output', os.path.join(output_dir, output.name), '--npm-cache', '/npm', '--driver', str(driver),
             *(('--check',) if check else ())]
    network = ('ip link set lo up; test "$(ip -o link show | cut -d: -f2 | tr -d " ")" = lo; '
               'test -z "$(ip route)"; test -z "$(ip -6 route)"; exec "$@"')
    return ['unshare', '--user', '--map-root-user', '--net', '/bin/sh', '-euc', network, 'sh', *inner]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--cache', type=Path, required=True, help='existing clean pinned Ergogen checkout')
    parser.add_argument('--output', type=Path, default=generate.FIXTURE)
    parser.add_argument('--check', action='store_true')
    parser.add_argument('--npm-cache', type=Path, default=generate.NPM_CACHE)
    parser.add_argument('--temp-parent', type=Path, required=True, help='trusted private directory outside the repository')
    args = parser.parse_args()
    try:
        checkout = args.cache.resolve(strict=True)
        parent = args.temp_parent.resolve(strict=True)
        if generate.inside(str(parent), os.path.realpath(ROOT)) or generate.inside(str(parent), str(checkout)):
            raise ValueError('Temporary parent must be outside the repository and checkout')
        chrome = verified_chrome()
        os.environ['CHROME_BIN'] = str(chrome)
        driver = generate.build_driver()
        with tempfile.TemporaryDirectory(prefix='ergogen-sandbox-', dir=parent) as run:
            run = Path(run)
            locked_archives(args.npm_cache, run / 'npm')
            return subprocess.run(command(run, checkout, driver, chrome, Path(os.path.abspath(args.output)), args.check)).returncode
    except Exception as error:
        print(f'Ergogen sandbox failed: {error}', file=sys.stderr)
        return 1


if __name__ == '__main__':
    sys.exit(main())
