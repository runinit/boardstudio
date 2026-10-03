"""Focused tests for publish-candidate proof derivation and atomicity."""

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
from urllib.parse import unquote, urlsplit


SCRIPTS = Path(__file__).resolve().parent
SPEC = importlib.util.spec_from_file_location("migration_candidate_test", SCRIPTS / "migration_candidate.py")
candidate = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(candidate)
PROGRESS_SPEC = importlib.util.spec_from_file_location(
    "migration_candidate_progress_test",
    SCRIPTS.parent / ".scratch/dioxus-frontend-v1/progress.py",
)
progress = importlib.util.module_from_spec(PROGRESS_SPEC)
PROGRESS_SPEC.loader.exec_module(progress)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class HTTPResponse:
    def __init__(self, body, headers=None, status=200):
        self.body = body
        self.headers = headers or {
            "Cross-Origin-Opener-Policy": "same-origin",
            "Cross-Origin-Embedder-Policy": "require-corp",
        }
        self.status = status

    def read(self):
        return self.body

    def getcode(self):
        return self.status

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False


class PublishCandidateTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="publish-candidate-test-")
        self.root = Path(self.temp.name)
        self.build_id = "build-test-1"
        self.build = self.root / "web/target/builds" / self.build_id
        self.build.mkdir(parents=True)
        self.proof_relative = Path(".scratch/dioxus-frontend-v1/evidence") / self.build_id / "package-proof.json"
        self.old_proof_relative = Path(".scratch/dioxus-frontend-v1/evidence/old/package-proof.json")
        self.old_proof = self.root / self.old_proof_relative
        self.old_proof.parent.mkdir(parents=True)
        self.old_proof.write_text("old finalized proof\n")
        self.root_url = "http://candidate.test/"
        self.subpath_url = "http://candidate.test/boardstudio/"
        self.route_assets = {"index.html": b"<html>candidate</html>", "assets/app.js": b"app"}
        routes = {}
        for name, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
            site_rel = Path("web/target/builds") / self.build_id / f"site-{name}"
            site = self.root / site_rel
            site.mkdir(parents=True)
            for asset, content in self.route_assets.items():
                target = site / asset
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(content)
            routes[name] = {
                "prefix": prefix,
                "site": str(site_rel),
                "assets": {asset: sha(data) for asset, data in self.route_assets.items()},
            }
        self.commands = []
        for route, prefix in (("root", "/"), ("subpath", "/boardstudio/")):
            log = self.build / f"page-{route}.log"
            log.write_text("Finished release build\n")
            self.commands.append({
                "argv": ["dx", "build", "--release", "--base-path", prefix],
                "log": str(log), "exit": 0,
                "started": "2026-10-03T10:00:00+00:00",
                "finished": "2026-10-03T10:00:02+00:00",
            })
        self.provenance = {
            "build_id": self.build_id,
            "source_commit": "a" * 40,  # May predate HEAD when inventory is unchanged.
            "status": "complete",
            "sources": {"web/src/main.rs": sha(b"current source")},
            "commands": self.commands,
            **routes,
        }
        self.provenance_path = self.build / "provenance.json"
        self.write_provenance()
        self.run_path = self.root / progress.RUN
        self.run_path.parent.mkdir(parents=True)
        self.original_run = {
            "current_progress": {
                "updated_at": "old-time",
                "served_candidate": {"build_id": "old-build", "root_url": self.root_url,
                                     "subpath_url": self.subpath_url},
            }
        }
        self.run_path.write_text(json.dumps(self.original_run))
        self.inventory = lambda: dict(self.provenance["sources"])

    def tearDown(self):
        self.temp.cleanup()

    def write_provenance(self):
        self.provenance_path.write_text(json.dumps(self.provenance, indent=2) + "\n")

    def publish(self):
        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress):
            return candidate.publish(
                self.root, self.build_id, self.root_url, self.subpath_url,
                inventory=self.inventory, progress_module=progress,
            )

    def mock_routes(self):
        def open_url(request, timeout=10):
            path = unquote(urlsplit(request.full_url).path)
            path = path.removeprefix("/boardstudio/").removeprefix("/") or "index.html"
            return HTTPResponse(self.route_assets[path])
        return patch.object(progress, "urlopen", side_effect=open_url)

    def test_full_build_produces_zero_inherited_commands_and_warning_counts_from_logs(self):
        self.commands[0]["log"] and Path(self.commands[0]["log"]).write_text("warning: first\nnote: detail\n")
        self.commands[1]["log"] and Path(self.commands[1]["log"]).write_text("warning[unused]: second\n")
        self.write_provenance()
        with self.mock_routes():
            self.publish()
        proof = json.loads((self.root / self.proof_relative).read_text())
        self.assertEqual(proof["inherited_commands"], 0)
        self.assertEqual(proof["release_warnings"], {"root": 1, "subpath": 1})
        self.assertEqual(proof["duration_seconds"], 2.0)

    def test_reuse_accepts_integer_count_with_matching_lineage(self):
        self.provenance["reuse_mode"] = "page-only-provider-reuse"
        self.provenance["base_build"] = "provider-base"
        self.provenance["inherited_full_build_commands"] = 1
        self.provenance["inherited_full_build_lineage"] = [{"label": "provider", "exit": 0}]
        self.write_provenance()
        with self.mock_routes():
            self.publish()
        proof = json.loads((self.root / self.proof_relative).read_text())
        self.assertEqual(proof["inherited_commands"], 1)

    def test_malformed_or_failed_provenance_preserves_previous_candidate_and_proof(self):
        self.provenance["status"] = "failed-page-root"
        self.write_provenance()
        before = self.run_path.read_bytes()
        with self.assertRaisesRegex(ValueError, "not complete"):
            self.publish()
        self.assertEqual(self.run_path.read_bytes(), before)
        self.assertEqual(self.old_proof.read_text(), "old finalized proof\n")
        self.assertFalse((self.root / self.proof_relative).exists())

    def test_source_drift_preserves_old_candidate(self):
        before = self.run_path.read_bytes()
        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress):
            with self.assertRaisesRegex(ValueError, "source inventory drift"):
                candidate.publish(self.root, self.build_id, self.root_url, self.subpath_url,
                                  inventory=lambda: {"web/src/main.rs": sha(b"changed")},
                                  progress_module=progress)
        self.assertEqual(self.run_path.read_bytes(), before)

    def test_missing_release_log_is_not_reported_as_zero_warnings(self):
        Path(self.commands[0]["log"]).unlink()
        before = self.run_path.read_bytes()
        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress):
            with self.assertRaisesRegex(ValueError, "release log is missing"):
                candidate.publish(self.root, self.build_id, self.root_url, self.subpath_url,
                                  inventory=self.inventory, progress_module=progress)
        self.assertEqual(self.run_path.read_bytes(), before)

    def test_initial_proof_write_failure_after_replace_rolls_back_proof_and_run(self):
        for existing in (False, True):
            with self.subTest(existing_proof=existing):
                if existing:
                    with self.mock_routes():
                        self.publish()
                proof_path = self.root / self.proof_relative
                previous_proof = proof_path.read_bytes() if proof_path.exists() else None
                previous_run = self.run_path.read_bytes()
                real_atomic_write = candidate._atomic_bytes
                calls = 0

                def fail_after_replace(path, content):
                    nonlocal calls
                    calls += 1
                    real_atomic_write(path, content)
                    if calls == 1:
                        raise OSError("directory fsync failed after replace")

                with self.mock_routes(), patch.object(candidate, "_atomic_bytes", side_effect=fail_after_replace):
                    with self.assertRaisesRegex(OSError, "after replace"):
                        self.publish()
                self.assertEqual(self.run_path.read_bytes(), previous_run)
                if previous_proof is None:
                    self.assertFalse(proof_path.exists(), "new proof must be removed after write failure")
                else:
                    self.assertEqual(proof_path.read_bytes(), previous_proof)
                    self.assertEqual(calls, 2, "existing proof must be restored after post-replace failure")

    def test_bad_local_asset_hash_and_bad_server_health_preserve_old_candidate(self):
        site = self.root / self.provenance["root"]["site"]
        (site / "assets/app.js").write_bytes(b"tampered")
        before = self.run_path.read_bytes()
        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress):
            with self.assertRaisesRegex(ValueError, "asset hash mismatch"):
                candidate.publish(self.root, self.build_id, self.root_url, self.subpath_url,
                                  inventory=self.inventory, progress_module=progress)
        self.assertEqual(self.run_path.read_bytes(), before)

        # Repair local bytes, then make a served response fail during the preflight.
        (site / "assets/app.js").write_bytes(self.route_assets["assets/app.js"])
        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress), \
                patch.object(progress, "urlopen", return_value=HTTPResponse(
                    b"unavailable", status=503,
                )):
            with self.assertRaisesRegex(ValueError, "did not return HTTP 200"):
                candidate.publish(self.root, self.build_id, self.root_url, self.subpath_url,
                                  inventory=self.inventory, progress_module=progress)
        self.assertEqual(self.run_path.read_bytes(), before)

        with patch.object(progress, "ROOT", self.root), \
                patch.object(candidate, "_load_progress", return_value=progress), \
                patch.object(progress, "urlopen", return_value=HTTPResponse(b"wrong bytes")):
            with self.assertRaisesRegex(ValueError, "Served asset hash mismatch"):
                candidate.publish(self.root, self.build_id, self.root_url, self.subpath_url,
                                  inventory=self.inventory, progress_module=progress)
        self.assertEqual(self.run_path.read_bytes(), before)

    def test_same_arguments_are_idempotent_and_different_finalized_proof_is_not_overwritten(self):
        with self.mock_routes(), patch.object(
                progress, "verify_candidate_routes", wraps=progress.verify_candidate_routes,
        ) as live_verify:
            self.publish()
            live_verify.assert_called_once()
        proof_path = self.root / self.proof_relative
        proof_before = proof_path.read_bytes()
        run_before = self.run_path.read_bytes()
        with self.mock_routes(), patch.object(
                progress, "verify_candidate_routes", wraps=progress.verify_candidate_routes,
        ) as live_verify:
            self.assertFalse(self.publish())
            live_verify.assert_called_once()
        self.assertEqual(proof_path.read_bytes(), proof_before)
        self.assertEqual(self.run_path.read_bytes(), run_before)

        proof_path.write_text("different finalized proof\n")
        with self.mock_routes():
            with self.assertRaisesRegex(ValueError, "Refusing to replace"):
                self.publish()
        self.assertEqual(proof_path.read_text(), "different finalized proof\n")
        self.assertEqual(self.run_path.read_bytes(), run_before)


if __name__ == "__main__":
    unittest.main(verbosity=2)
