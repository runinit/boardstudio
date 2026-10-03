"""Focused tests for the frontend progress record commands."""

import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
from urllib.parse import urlsplit
import unittest
from unittest.mock import patch


MODULE_PATH = Path(__file__).with_name("progress.py")
SPEC = importlib.util.spec_from_file_location("frontend_progress", MODULE_PATH)
progress = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(progress)


def sha(data):
    return hashlib.sha256(data).hexdigest()


class Response:
    def __init__(self, body, headers=None, status=200):
        self.body = body
        self.headers = headers or {}
        self.status = status

    def read(self):
        return self.body

    def getcode(self):
        return self.status

    def __enter__(self):
        return self

    def __exit__(self, *_args):
        return False


class ProgressTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)
        self.proof_relative = Path("evidence/candidate/package-proof.json")
        self.proof_path = self.root / self.proof_relative
        self.proof_path.parent.mkdir(parents=True)
        self.provenance_relative = Path("build/provenance.json")
        self.provenance_path = self.root / self.provenance_relative
        self.provenance_path.parent.mkdir(parents=True)
        self.run_path = self.root / "run.json"
        self.root_url = "http://candidate.test/"
        self.subpath_url = "http://candidate.test/boardstudio/"
        self.assets = {
            "index.html": b"<html>candidate</html>",
            "assets/app.js": b"console.log('candidate')",
        }
        self.site_paths = {}
        for route in ("root", "subpath"):
            site_relative = Path("build") / f"site-{route}"
            site = self.root / site_relative
            site.mkdir(parents=True)
            for name, body in self.assets.items():
                target = site / name
                target.parent.mkdir(parents=True, exist_ok=True)
                target.write_bytes(body)
            self.site_paths[route] = site_relative
        route_map = {name: sha(body) for name, body in self.assets.items()}
        self.provenance = {
            "build_id": "build-older-than-head",
            "source_commit": "a" * 40,
            "status": "complete",
            "commands": [{"label": "page", "exit": 0}],
            "inherited_full_build_commands": 2,
            "inherited_full_build_lineage": [{"label": "inherited", "exit": 0}] * 2,
            "sources": {"one": sha(b"one"), "two": sha(b"two"), "three": sha(b"three")},
            "root": {"prefix": "/", "site": str(self.site_paths["root"]), "assets": route_map},
            "subpath": {"prefix": "/boardstudio/", "site": str(self.site_paths["subpath"]), "assets": route_map},
        }
        self.write_provenance()
        self.proof = {
            "build_id": self.provenance["build_id"],
            "source_commit": self.provenance["source_commit"],
            "provenance": str(self.provenance_relative),
            "provenance_sha256": sha(self.provenance_path.read_bytes()),
            "source_count": 3,
            "source_mismatches": [],
            "routes": {
                route: {
                    "http": 200,
                    "headers": {
                        "Cross-Origin-Opener-Policy": "same-origin",
                        "Cross-Origin-Embedder-Policy": "require-corp",
                    },
                    "asset_count": len(self.assets),
                    "mismatches": [],
                }
                for route in ("root", "subpath")
            },
            "fresh_commands": 1,
            "inherited_commands": 2,
            "duration_seconds": 1.5,
            "release_warnings": {"root": 0, "subpath": 0},
            "qualification": "Package/source/HTTP assets only.",
        }
        self.write_proof()
        self.original_run = {
            "schema_version": 1,
            "history": ["preserve me"],
            "current_progress": {
                "updated_at": "old-time",
                "parent_counts": {"accepted": 1},
                "served_candidate": {
                    "build_id": "old-build",
                    "source_commit": "b" * 40,
                    "root_url": self.root_url,
                    "subpath_url": self.subpath_url,
                },
                "active_streams": [{"stream": "Layout"}],
            },
        }
        self.run_path.write_text(json.dumps(self.original_run, indent=2) + "\n")

    def tearDown(self):
        self.temp.cleanup()

    def write_provenance(self):
        self.provenance_path.write_text(json.dumps(self.provenance, indent=2) + "\n")

    def write_proof(self):
        self.proof_path.write_text(json.dumps(self.proof, indent=2) + "\n")

    def opener(self, request, **_kwargs):
        url = request.full_url
        path = urlsplit(url).path.removeprefix("/boardstudio/").removeprefix("/")
        if path == "":
            path = "index.html"
        if path not in self.assets:
            raise AssertionError(f"Unexpected live URL: {url}")
        headers = {
            "Cross-Origin-Opener-Policy": "same-origin",
            "Cross-Origin-Embedder-Policy": "require-corp",
        }
        return Response(self.assets[path], headers)

    def record(self):
        with patch.object(progress, "ROOT", self.root), \
                patch.object(progress, "RUN", Path("run.json")), \
                patch.object(progress, "urlopen", self.opener):
            progress.record_candidate(self.proof_relative, self.root_url, self.subpath_url)

    def test_record_updates_only_candidate_and_timestamp_idempotently(self):
        self.record()
        updated = json.loads(self.run_path.read_text())
        self.assertEqual(updated["schema_version"], self.original_run["schema_version"])
        self.assertEqual(updated["history"], self.original_run["history"])
        current = updated["current_progress"]
        self.assertEqual(current["parent_counts"], self.original_run["current_progress"]["parent_counts"])
        self.assertEqual(current["active_streams"], self.original_run["current_progress"]["active_streams"])
        self.assertEqual(current["served_candidate"]["build_id"], "build-older-than-head")
        self.assertEqual(current["served_candidate"]["source_commit"], "a" * 40)
        self.assertEqual(current["served_candidate"]["package_proof"], str(self.proof_relative))
        first_bytes = self.run_path.read_bytes()
        self.record()
        self.assertEqual(self.run_path.read_bytes(), first_bytes)

    def assert_rejected_without_write(self, mutation):
        mutation()
        original = self.run_path.read_bytes()
        with self.assertRaises(ValueError):
            self.record()
        self.assertEqual(self.run_path.read_bytes(), original)

    def test_rejects_provenance_hash_mismatch_without_write(self):
        self.proof["provenance_sha256"] = "0" * 64
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_proof_provenance_linkage_mismatch_without_write(self):
        self.proof["build_id"] = "another-build"
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_incomplete_route_schema_without_write(self):
        del self.proof["routes"]["subpath"]
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_proof_command_and_source_count_drift_without_write(self):
        self.proof["fresh_commands"] = 2
        self.proof["source_count"] = 4
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_absent_source_mismatch_list_without_write(self):
        del self.proof["source_mismatches"]
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_failed_command_without_write(self):
        self.provenance["commands"][0]["exit"] = 1
        self.write_provenance()
        self.proof["provenance_sha256"] = sha(self.provenance_path.read_bytes())
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_live_asset_hash_mismatch_without_write(self):
        self.assets["assets/app.js"] = b"changed served bytes"
        self.assert_rejected_without_write(lambda: None)

    def test_root_index_response_must_match_mapped_index_hash(self):
        self.assets["index.html"] = b"wrong root document"
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_missing_route_map_asset_without_write(self):
        del self.provenance["subpath"]["assets"]["assets/app.js"]
        self.write_provenance()
        self.proof["provenance_sha256"] = sha(self.provenance_path.read_bytes())
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_asset_path_escape_and_scheme_without_write(self):
        assets = self.provenance["root"]["assets"]
        digest = assets.pop("assets/app.js")
        assets["../outside.js"] = digest
        self.write_provenance()
        self.proof["provenance_sha256"] = sha(self.provenance_path.read_bytes())
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_asset_uri_scheme_without_write(self):
        assets = self.provenance["root"]["assets"]
        digest = assets.pop("assets/app.js")
        assets["https://other.test/asset.js"] = digest
        self.write_provenance()
        self.proof["provenance_sha256"] = sha(self.provenance_path.read_bytes())
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_rejects_symlink_asset_escaping_package_site_without_write(self):
        outside = self.root / "outside.js"
        outside.write_bytes(self.assets["assets/app.js"])
        link = self.root / self.site_paths["root"] / "assets" / "link.js"
        link.symlink_to(outside)
        self.provenance["root"]["assets"]["assets/link.js"] = sha(outside.read_bytes())
        self.proof["routes"]["root"]["asset_count"] += 1
        self.write_provenance()
        self.proof["provenance_sha256"] = sha(self.provenance_path.read_bytes())
        self.write_proof()
        self.assert_rejected_without_write(lambda: None)

    def test_atomic_replace_failure_preserves_run_and_cleans_temp_file(self):
        original = self.run_path.read_bytes()
        with patch.object(progress, "ROOT", self.root), \
                patch.object(progress, "RUN", Path("run.json")), \
                patch.object(progress, "urlopen", self.opener), \
                patch.object(progress.os, "replace", side_effect=OSError("replace failed")):
            with self.assertRaises(OSError):
                progress.record_candidate(self.proof_relative, self.root_url, self.subpath_url)
        self.assertEqual(self.run_path.read_bytes(), original)
        self.assertEqual(list(self.root.glob(".run.json.*.tmp")), [])

    def test_report_renders_nested_and_unknown_rf_fields(self):
        ledger = {
            "findings": [{
                "id": "RF-001",
                "title": "Example",
                "observation": "Summary",
                "evidence": [],
                "handoff_evidence": [{"observation": "handoff"}],
                "observations_20261003": ["one", {"source": "two", "notes": ["detail"]}],
                "future_field": {"new": True},
                "fenced_text": "This contains ``` and remains inside the JSON block.",
            }]
        }
        with patch.object(progress, "read", return_value=ledger):
            report = progress.refactor_report()
        self.assertIn('"observations_20261003"', report)
        self.assertIn('"future_field"', report)
        self.assertIn('"detail"', report)
        self.assertIn('"new": true', report)
        self.assertIn('````json', report)
        self.assertNotIn("additional source observations are indexed", report)


if __name__ == "__main__":
    unittest.main()
