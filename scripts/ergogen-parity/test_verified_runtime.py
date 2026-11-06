import base64
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import verified_runtime as runtime


def archive(members):
    result = io.BytesIO()
    with tarfile.open(fileobj=result, mode="w:gz") as tar:
        for name, data in members:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            tar.addfile(info, io.BytesIO(data))
    return result.getvalue()


class VerifiedRuntimeTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="ergogen-runtime-test-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.checkout = self.root / "checkout"
        self.checkout.mkdir()
        self.pin = "1" * 40
        self.status = b""
        self.source = archive([("package.json", b'{"version":"4.2.1"}'), ("points.js", b"pinned source")])
        self.tree = b"".join(
            b"100644 blob " + hashlib.sha1(b"blob " + str(len(data)).encode() + b"\0" + data).hexdigest().encode()
            + b"\t" + name.encode() + b"\0"
            for name, data in (("package.json", b'{"version":"4.2.1"}'), ("points.js", b"pinned source"))
        )
        self.cache = self.root / "cache"
        self.lock = self.root / "lock.json"
        self.package = self.root / "package.json"
        self.package.write_text(json.dumps({"dependencies": {"example": "1.0.0"}}))
        self.data = archive([("package/package.json", b'{"name":"example","version":"1.0.0"}'), ("package/index.js", b"module source")])
        self.set_archive(self.data)
        self.pin_patch = patch.object(runtime, "PIN", self.pin)
        self.lock_patch = patch.object(runtime, "LOCK", self.lock)
        self.package_patch = patch.object(runtime, "PACKAGE", self.package)
        self.git_patch = patch.object(runtime, "git", side_effect=self.fake_git)
        self.pin_patch.start()
        self.lock_patch.start()
        self.package_patch.start()
        self.git_patch.start()
        self.addCleanup(self.pin_patch.stop)
        self.addCleanup(self.lock_patch.stop)
        self.addCleanup(self.package_patch.stop)
        self.addCleanup(self.git_patch.stop)

    def fake_git(self, checkout, *args):
        if checkout != self.checkout:
            raise subprocess.CalledProcessError(128, "git")
        if args == ("rev-parse", "--show-toplevel"):
            return os.fsencode(self.checkout)
        if args == ("rev-parse", "HEAD"):
            return self.pin.encode()
        if args == ("config", "--null", "--name-only", "--list"):
            return b""
        if args == ("ls-files", "-v", "-z"):
            return b"H points.js\0"
        if args[0] == "status":
            return self.status
        if args[0] == "archive":
            return self.source
        if args[0] == "ls-tree":
            return self.tree
        raise AssertionError(args)

    def set_archive(self, data):
        digest = hashlib.sha512(data).digest()
        name = digest.hex()
        self.archive_path = self.cache / name[:2] / name[2:4] / name[4:]
        self.archive_path.parent.mkdir(parents=True, exist_ok=True)
        self.archive_path.write_bytes(data)
        self.lock.write_text(json.dumps({"lockfileVersion": 3, "packages": {"": {"dependencies": {"example": "1.0.0"}}, "node_modules/example": {
            "version": "1.0.0", "resolved": "https://registry.npmjs.org/example/-/example-1.0.0.tgz", "integrity": "sha512-" + base64.b64encode(digest).decode()
        }}}))

    def test_authenticated_source_archive_and_cleanup(self):
        with runtime.verified_runtime(self.checkout, self.cache, self.root) as prepared:
            self.assertEqual((prepared / "upstream/points.js").read_text(), "pinned source")
            self.assertEqual((prepared / "node_modules/example/index.js").read_text(), "module source")
            self.assertTrue(prepared.exists())
        self.assertFalse(prepared.exists())

    def test_rejects_dirty_wrong_head_invalid_and_git_failure(self):
        self.status = b"?? untracked"
        with self.assertRaisesRegex(ValueError, "untracked"):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail("dirty checkout accepted")
        self.status = b""
        with patch.object(runtime, "PIN", "0" * 40):
            with self.assertRaisesRegex(ValueError, "pinned Ergogen commit"):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail("wrong HEAD accepted")
        with self.assertRaises(subprocess.CalledProcessError):
            with runtime.verified_runtime(self.root, self.cache, self.root):
                self.fail("invalid checkout accepted")
        with patch.object(runtime, "git", side_effect=OSError("git unavailable")):
            with self.assertRaisesRegex(OSError, "git unavailable"):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail("git failure accepted")

    def test_rejects_corrupt_archive_and_cleans_private_tree(self):
        self.archive_path.write_bytes(self.data + b"corruption")
        with self.assertRaisesRegex(ValueError, "integrity mismatch"):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail("corrupt archive accepted")
        self.assertEqual(list(self.root.glob("ergogen-verified-*")), [])

    def test_rejects_modified_or_missing_git_archive_content(self):
        for source in (
            archive([("package.json", b'{"version":"4.2.1"}'), ("points.js", b"modified")]),
            archive([("package.json", b'{"version":"4.2.1"}')]),
        ):
            self.source = source
            with self.subTest(source=source), self.assertRaisesRegex(ValueError, "Pinned source"):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail("modified source archive accepted")

    def test_rejects_archive_traversal_and_wrong_version(self):
        self.set_archive(archive([("package/../../escape", b"bad")]))
        with self.assertRaisesRegex(ValueError, "Unsafe archive path"):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail("unsafe archive accepted")
        self.assertFalse((self.root / "escape").exists())
        self.set_archive(archive([("package/package.json", b'{"name":"example","version":"wrong"}')]))
        with self.assertRaisesRegex(ValueError, "version mismatch"):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail("wrong version accepted")

    def test_rejects_hidden_index_changes(self):
        for flags in (b"h points.js\0", b"S points.js\0"):
            with self.subTest(flags=flags), patch.object(runtime, "git", side_effect=lambda checkout, *args:
                    flags if args == ("ls-files", "-v", "-z") else self.fake_git(checkout, *args)):
                with self.assertRaisesRegex(ValueError, "hidden index"):
                    with runtime.verified_runtime(self.checkout, self.cache, self.root):
                        self.fail("hidden index changes accepted")

    def test_inherited_git_redirect_does_not_replace_checkout(self):
        self.git_patch.stop()
        with patch.dict(os.environ, {"GIT_DIR": "/nonexistent", "GIT_WORK_TREE": "/nonexistent"}), \
             patch.object(runtime.subprocess, "check_output", return_value=b"ok") as run:
            runtime.git(self.checkout, "status")
        self.assertNotIn("GIT_DIR", run.call_args.kwargs["env"])
        self.assertNotIn("GIT_WORK_TREE", run.call_args.kwargs["env"])

    def test_rejects_temporary_parent_inside_checkout(self):
        for parent in (self.checkout, None):
            with self.subTest(parent=parent), patch.dict(os.environ, {'TMPDIR': str(self.checkout)}):
                with self.assertRaisesRegex(ValueError, "Temporary"):
                    with runtime.verified_runtime(self.checkout, self.cache, parent):
                        self.fail("checkout used as temporary parent")
        self.assertEqual(list(self.checkout.iterdir()), [])

    def test_rejects_incomplete_lock_before_yield(self):
        for packages in ({"": {}}, {"": {"dependencies": {"example": "1.0.0"}},
                                     "node_modules/example": {"version": "1.0.0", "resolved": "https://registry.npmjs.org/example/-/example-1.0.0.tgz", "dependencies": {"missing": "1.0.0"}}}):
            self.lock.write_text(json.dumps({"lockfileVersion": 3, "packages": packages}))
            with self.subTest(packages=packages), self.assertRaisesRegex(ValueError, "dependencies|dependency"):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail("incomplete lock accepted")

    def test_rejects_archive_with_unlisted_dependency(self):
        self.set_archive(archive([('package/package.json', b'{"name":"example","version":"1.0.0","dependencies":{"missing":"1"}}')]))
        with self.assertRaisesRegex(ValueError, 'dependency mismatch'):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail('archive with unlisted dependency accepted')

    def test_rejects_wrong_package_identity(self):
        self.set_archive(archive([('package/package.json', b'{"name":"different","version":"1.0.0"}')]))
        with self.assertRaisesRegex(ValueError, 'name mismatch'):
            with runtime.verified_runtime(self.checkout, self.cache, self.root):
                self.fail('wrong package accepted')

    def test_rejects_nonempty_optional_peer_or_bundled_dependencies(self):
        for field in ('optionalDependencies', 'peerDependencies', 'bundleDependencies', 'bundledDependencies'):
            package = {'name': 'example', 'version': '1.0.0', field: {'other': '1.0.0'}}
            self.set_archive(archive([('package/package.json', json.dumps(package).encode())]))
            with self.subTest(field=field), self.assertRaisesRegex(ValueError, 'Unsupported optional'):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail('unsupported dependencies accepted')
            self.assertEqual(list(self.root.glob('ergogen-verified-*')), [])

    def test_accepts_empty_optional_peer_and_dev_dependencies(self):
        package = {'name': 'example', 'version': '1.0.0', 'optionalDependencies': {},
                   'peerDependencies': {}, 'devDependencies': {'development-only': '1.0.0'}}
        self.set_archive(archive([('package/package.json', json.dumps(package).encode())]))
        with runtime.verified_runtime(self.checkout, self.cache, self.root) as prepared:
            self.assertTrue((prepared / 'node_modules/example/package.json').is_file())

    def test_default_temp_selection_does_not_probe_inside_checkout(self):
        original_open = os.open
        calls = []
        def observed_open(path, *args, **kwargs):
            calls.append(Path(path))
            return original_open(path, *args, **kwargs)
        with patch.dict(os.environ, {'TMPDIR': str(self.checkout)}), \
                patch.object(tempfile, 'tempdir', None), patch.object(os, 'open', side_effect=observed_open):
            with self.assertRaisesRegex(ValueError, 'Temporary'):
                with runtime.verified_runtime(self.checkout, self.cache):
                    self.fail('protected default temp accepted')
        self.assertFalse(any(path.is_relative_to(self.checkout) for path in calls), calls)

    def test_rejects_nested_node_modules_in_package_archive(self):
        data = archive([('package/node_modules/example/index.js', b'unlocked code')])
        with self.assertRaisesRegex(ValueError, 'node_modules'):
            runtime.extract(data, self.root, strip_root=True)
        self.assertFalse((self.root / 'node_modules').exists())

    def test_rejects_archive_links_and_absolute_paths(self):
        for kind in (tarfile.SYMTYPE, tarfile.LNKTYPE, None):
            payload = io.BytesIO()
            with tarfile.open(fileobj=payload, mode='w:gz') as tar:
                entry = tarfile.TarInfo('package/link' if kind else '/absolute')
                if kind:
                    entry.type = kind
                    entry.linkname = 'target'
                tar.addfile(entry)
            self.set_archive(payload.getvalue())
            with self.subTest(kind=kind), self.assertRaisesRegex(ValueError, 'Unsupported archive member|Unsafe archive path'):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail('unsafe archive accepted')
            self.assertEqual(list(self.root.glob('ergogen-verified-*')), [])

    def test_git_status_does_not_refresh_index(self):
        self.git_patch.stop()
        def git(*args):
            return subprocess.check_output(['git', '-C', str(self.checkout), *args], stderr=subprocess.PIPE)
        git('init', '-q')
        git('config', 'user.email', 'test@example.invalid')
        git('config', 'user.name', 'Test')
        (self.checkout / 'points.js').write_text('original')
        git('add', 'points.js')
        git('commit', '-qm', 'initial')
        index = self.checkout / '.git/index'
        before = (index.read_bytes(), index.stat().st_mtime_ns)
        self.assertFalse(runtime.git(self.checkout, 'status', '--porcelain', '--untracked-files=all').strip())
        self.assertEqual((index.read_bytes(), index.stat().st_mtime_ns), before)

    def test_checkout_filter_cannot_execute_during_validation(self):
        self.git_patch.stop()
        def git(*args):
            return subprocess.check_output(['git', '-C', str(self.checkout), *args], stderr=subprocess.PIPE)
        git('init', '-q')
        git('config', 'user.email', 'test@example.invalid')
        git('config', 'user.name', 'Test')
        (self.checkout / 'points.js').write_text('original')
        git('add', 'points.js')
        git('commit', '-qm', 'initial')
        pin = git('rev-parse', 'HEAD').decode().strip()
        marker = self.root / 'filter-executed'
        git('config', 'filter.untrusted.clean', f'sh -c "touch {marker}; cat"')
        (self.checkout / '.git/info/attributes').write_text('points.js filter=untrusted\n')
        (self.checkout / 'points.js').write_text('changed!')
        with patch.object(runtime, 'PIN', pin):
            with self.assertRaises((ValueError, subprocess.CalledProcessError)):
                with runtime.verified_runtime(self.checkout, self.cache, self.root):
                    self.fail('untrusted checkout accepted')
        self.assertFalse(marker.exists(), 'checkout filter was executed')


class LockValidationTests(unittest.TestCase):
    def test_supported_ranges_and_unsupported_forms(self):
        cases = [('4.2.2', '^4.2.1', True), ('5.0.0', '^4.2.1', False),
                 ('0.0.6', '^0.0.6', True), ('0.0.7', '^0.0.6', False),
                 ('0.7.36', '^0.7.34', True), ('0.8.0', '^0.7.0', False),
                 ('0.0.1', '^0.0.0', False), ('7.10.14', '*', True),
                 ('0.18.3', '0.18.2', False)]
        for version, spec, expected in cases:
            with self.subTest(version=version, spec=spec):
                self.assertEqual(runtime.satisfies(version, spec), expected)
        for spec in ('~4.2.1', '4.x', '>=4 <5', '^4.2', '^4.2.1 || ^5.0.0',
                     '=4.2.1', 'v4.2.1', '^4.2.1-beta', 'latest', 'npm:typed-function@4', ''):
            with self.subTest(spec=spec), self.assertRaises(ValueError):
                runtime.satisfies('4.2.2', spec)
        for version in ('4.2.2-rc.1', '01.2.3', '9007199254740992.0.0'):
            with self.subTest(version=version), self.assertRaises(ValueError):
                runtime.satisfies(version, '*')

    def test_real_lock_and_incompatible_transitive_version(self):
        lock = json.loads(runtime.LOCK.read_text())
        manifest = json.loads(runtime.PACKAGE.read_text())
        runtime.check_lock(lock, manifest)
        for version in ('5.0.0', '4.2.0'):
            changed = json.loads(json.dumps(lock))
            changed['packages']['node_modules/typed-function']['version'] = version
            changed['packages']['node_modules/typed-function']['resolved'] = f'https://registry.npmjs.org/typed-function/-/typed-function-{version}.tgz'
            with self.subTest(version=version), self.assertRaisesRegex(ValueError, 'typed-function'):
                runtime.check_lock(changed, manifest)
        changed['packages']['node_modules/typed-function']['version'] = '4.99.0'
        changed['packages']['node_modules/typed-function']['resolved'] = 'https://registry.npmjs.org/typed-function/-/typed-function-4.99.0.tgz'
        runtime.check_lock(changed, manifest)

    def test_rejects_unsupported_lock_features_and_unreachable_packages(self):
        lock = json.loads(runtime.LOCK.read_text())
        manifest = json.loads(runtime.PACKAGE.read_text())
        for change in ('lock-version', 'unreachable', 'optional', 'non-registry', 'override', 'bad-path'):
            changed, changed_manifest = json.loads(json.dumps(lock)), dict(manifest)
            if change == 'lock-version': changed['lockfileVersion'] = 2
            elif change == 'unreachable': changed['packages']['node_modules/hull'] = {'version': '1.0.0', 'resolved': 'https://registry.npmjs.org/hull/-/hull-1.0.0.tgz'}
            elif change == 'optional': changed['packages']['node_modules/mathjs']['optional'] = True
            elif change == 'non-registry': changed['packages']['node_modules/mathjs']['resolved'] = 'file:mathjs.tgz'
            elif change == 'override': changed_manifest['overrides'] = {'mathjs': '1'}
            elif change == 'bad-path': changed['packages']['node_modules/@types'] = {'version': '1.0.0'}
            with self.subTest(change=change), self.assertRaises(ValueError):
                runtime.check_lock(changed, changed_manifest)

    def test_rejects_url_delimiters_in_package_names(self):
        for name in ('x?y', 'x#y', 'x%2fy', '@bad?scope/pkg'):
            manifest = {'dependencies': {name: '1.0.0'}}
            lock = {'lockfileVersion': 3, 'packages': {
                '': {'dependencies': manifest['dependencies']},
                'node_modules/' + name: {'version': '1.0.0',
                    'resolved': f'https://registry.npmjs.org/{name}/-/{name.split("/")[-1]}-1.0.0.tgz'},
            }}
            with self.subTest(name=name), self.assertRaisesRegex(ValueError, 'package path'):
                runtime.check_lock(lock, manifest)


if __name__ == "__main__":
    unittest.main()
