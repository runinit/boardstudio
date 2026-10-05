"""Real-Git regression checks for frozen candidate inputs and snapshot ownership."""

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
from types import SimpleNamespace


SPEC = importlib.util.spec_from_file_location("migration_snapshot_test", Path(__file__).with_name("migration_snapshot.py"))
snapshot = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(snapshot)
BUILD_SPEC = importlib.util.spec_from_file_location("snapshot_build_test", Path(__file__).with_name("build-m1.py"))
build = importlib.util.module_from_spec(BUILD_SPEC)
BUILD_SPEC.loader.exec_module(build)


class SnapshotTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="migration-snapshot-test-")
        self.root = Path(self.temp.name)
        self.git("init", "-b", "integration")
        self.git("config", "user.name", "Snapshot Test")
        self.git("config", "user.email", "snapshot@example.invalid")
        (self.root / "web/src").mkdir(parents=True)
        (self.root / "web/src/main.rs").write_text("fn main() {}\n")
        (self.root / ".gitignore").write_text("web/target/\n")
        self.git("add", ".gitignore", "web/src/main.rs")
        self.git("commit", "-m", "base")
        self.commit = self.git("rev-parse", "HEAD")
        self.select = lambda path: path.startswith("web/") and not path.startswith("web/target/")

    def tearDown(self):
        self.temp.cleanup()

    def git(self, *args):
        return subprocess.check_output(["git", "-C", str(self.root), *args], stderr=subprocess.PIPE).decode().strip()

    def sources(self, commit=None):
        return snapshot.committed_sources(self.root, commit or self.commit, self.select)

    def prepare(self, commit=None):
        return snapshot.prepare_checkout(self.root, commit or self.commit,
                                         prepare_dependencies=False, seed_cache=False)

    def test_committed_inputs_ignore_new_author_edits_and_match_exact_bytes(self):
        expected = {"web/src/main.rs": hashlib.sha256(b"fn main() {}\n").hexdigest()}
        (self.root / "web/src/main.rs").write_text("new uncommitted author work\n")
        (self.root / "web/src/next.rs").write_text("another author\n")
        self.assertEqual(self.sources(), expected)
        checkout = self.prepare()
        self.assertEqual((checkout / "web/src/main.rs").read_text(), "fn main() {}\n")
        self.assertFalse((checkout / "web/src/next.rs").exists())
        self.assertEqual((self.root / "web/src/main.rs").read_text(), "new uncommitted author work\n")

    def test_source_identity_rejects_missing_added_or_changed_input(self):
        valid = self.sources()
        for invalid in ({}, valid | {"web/src/new.rs": "b" * 64}, {"web/src/main.rs": "c" * 64}):
            with self.subTest(invalid=invalid), self.assertRaisesRegex(ValueError, "differ from their committed"):
                snapshot.source_identity(self.root, invalid, self.select, self.commit)

    def test_published_identity_stays_valid_after_coordinator_commits_more_work(self):
        sources = self.sources()
        identity = snapshot.source_identity(self.root, sources, self.select, self.commit)
        provenance = {"source_commit": self.commit, "sources": sources, "source_snapshot": identity}
        (self.root / "web/src/main.rs").write_text("fn main() { changed(); }\n")
        self.git("add", "web/src/main.rs")
        self.git("commit", "-m", "next batch")
        self.assertEqual(snapshot.validate_identity(self.root, provenance, self.select), identity)
        for key in ("source_tree", "manifest_sha256", "source_commit"):
            bad = dict(provenance, source_snapshot=identity | {key: "0" * 40})
            with self.subTest(key=key), self.assertRaises(ValueError):
                snapshot.validate_identity(self.root, bad, self.select)

    def test_internal_symlink_hashes_target_bytes_at_the_selected_commit(self):
        (self.root / "web/assets").mkdir()
        (self.root / "web/assets/source.json").write_text('{"source":1}\n')
        (self.root / "web/assets/imported.json").symlink_to("source.json")
        self.git("add", "web/assets")
        self.git("commit", "-m", "linked asset")
        sources = self.sources(self.git("rev-parse", "HEAD"))
        self.assertEqual(sources["web/assets/source.json"], sources["web/assets/imported.json"])

    def test_external_and_recursive_source_symlinks_fail_closed(self):
        for target in ("/tmp/external", "../../../escape", "main.rs"):
            with self.subTest(target=target):
                link = self.root / "web/src/link.rs"
                if link.is_symlink():
                    link.unlink()
                link.symlink_to("link.rs" if target == "main.rs" else target)
                self.git("add", "web/src/link.rs")
                self.git("commit", "-m", "link fixture")
                with self.assertRaises(ValueError):
                    self.sources(self.git("rev-parse", "HEAD"))

    def test_owned_checkout_updates_commit_and_preserves_generated_cache(self):
        checkout = self.prepare()
        cache = checkout / "web/target/cache-marker"
        cache.parent.mkdir(parents=True)
        cache.write_text("warm\n")
        (self.root / "web/src/main.rs").write_text("fn main() { next(); }\n")
        self.git("add", "web/src/main.rs")
        self.git("commit", "-m", "next")
        self.assertEqual(self.prepare(self.git("rev-parse", "HEAD")), checkout)
        self.assertEqual(cache.read_text(), "warm\n")
        self.assertIn("next()", (checkout / "web/src/main.rs").read_text())

    def test_dirty_owned_checkout_is_preserved_instead_of_reset(self):
        checkout = self.prepare()
        source = checkout / "web/src/main.rs"
        source.write_text("uncommitted snapshot investigation\n")
        with self.assertRaisesRegex(ValueError, "preserving"):
            self.prepare()
        self.assertEqual(source.read_text(), "uncommitted snapshot investigation\n")

    def test_unowned_directory_and_bad_marker_are_preserved(self):
        checkout = self.root / snapshot.SNAPSHOT_DIRECTORY
        checkout.mkdir(parents=True)
        sentinel = checkout / "user.txt"
        sentinel.write_text("keep\n")
        with self.assertRaisesRegex(ValueError, "not owned"):
            self.prepare()
        self.assertEqual(sentinel.read_text(), "keep\n")

    def test_package_slot_serializes_packages_without_freezing_source_commits(self):
        with snapshot.package_slot(self.root):
            with self.assertRaisesRegex(ValueError, "another migration package"):
                with snapshot.package_slot(self.root):
                    self.fail("second package acquired the slot")
            (self.root / "web/src/main.rs").write_text("fn main() { next(); }\n")
            self.git("add", "web/src/main.rs")
            self.git("commit", "-m", "independent source integration")
        with snapshot.package_slot(self.root):
            pass

    def install_build_helpers(self):
        for name in ("build-m1.py", "migration_snapshot.py"):
            source = Path(__file__).with_name(name)
            destination = self.root / "scripts" / name
            destination.parent.mkdir(exist_ok=True)
            destination.write_bytes(source.read_bytes())
        self.git("add", "scripts/build-m1.py", "scripts/migration_snapshot.py")
        self.git("commit", "-m", "snapshot helpers")
        return self.git("rev-parse", "HEAD")

    def test_build_source_scopes_reads_and_restores_globals_on_failure(self):
        commit = self.install_build_helpers()
        original_prepare = snapshot.prepare_checkout
        guard = SimpleNamespace(validate_checkout=lambda root, required: (root, {"role": "coordinator"}))
        output = self.root / "web/target/builds"
        with patch.object(build, "REPO", self.root), patch.object(build, "WEB", self.root / "web"), \
                patch.object(build, "BUILD_ROOT", output), \
                patch.object(snapshot, "prepare_checkout", side_effect=lambda root, revision: original_prepare(
                    root, revision, prepare_dependencies=False, seed_cache=False)):
            with self.assertRaisesRegex(RuntimeError, "build failed"):
                with build.snapshot_build_source(commit, guard, snapshot):
                    self.assertEqual(build.REPO, self.root / snapshot.SNAPSHOT_DIRECTORY)
                    self.assertEqual(build.BUILD_ROOT, output)
                    inputs = build.sources()
                    self.assertEqual(build.source_context(inputs)["source_snapshot"]["source_commit"], commit)
                    (self.root / "web/src/main.rs").write_text("next author's source\n")
                    self.assertEqual(build.sources(), inputs)
                    raise RuntimeError("build failed")
            self.assertEqual(build.REPO, self.root)
            self.assertIsNone(build.SNAPSHOT_COMMIT)
            self.assertEqual((self.root / "web/src/main.rs").read_text(), "next author's source\n")

    def test_snapshot_build_refuses_uncommitted_helper_or_author_role_before_checkout(self):
        commit = self.install_build_helpers()
        for role, change_helper, message in (("author", False, "registered coordinator"),
                                             ("coordinator", True, "helper differs")):
            with self.subTest(role=role):
                if change_helper:
                    path = self.root / "scripts/build-m1.py"
                    path.write_text(path.read_text() + "# new edit\n")
                guard = SimpleNamespace(validate_checkout=lambda root, required: (root, {"role": role}))
                with patch.object(build, "REPO", self.root), patch.object(snapshot, "prepare_checkout") as prepare:
                    with self.assertRaisesRegex(ValueError, message):
                        with build.snapshot_build_source(commit, guard, snapshot):
                            self.fail("invalid snapshot source accepted")
                    prepare.assert_not_called()

    def test_full_command_identity_retains_donor_root_and_canonical_output(self):
        donor = Path("/owned/snapshot")
        output = Path("/coordinator/web/target/builds/full-donor")
        commands = build.expected_full_commands(output, source_root=donor)
        worker = next(row for row in commands if row[0] == "core-worker")
        self.assertEqual(worker[2], str(donor))
        self.assertIn(str(donor / "web"), worker[1])
        self.assertIn(str(output / "core-worker"), worker[1])
        page = next(row for row in commands if row[0] == "page-root")
        self.assertEqual(page[2], str(donor / "web"))


if __name__ == "__main__":
    unittest.main()
