import hashlib
import importlib.util
import json
from pathlib import Path
import tempfile
import tomllib
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("security_audit", Path(__file__).with_name("security-audit.py"))
audit = importlib.util.module_from_spec(spec)
spec.loader.exec_module(audit)


class SecurityAuditTests(unittest.TestCase):
    def test_registry_identity_is_restored_without_modifying_other_packages(self):
        source = '''version = 4
[[package]]
name = "cgmath"
version = "0.18.0"
dependencies = ["approx"]
[[package]]
name = "approx"
version = "0.4.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
'''
        provenance = {"version": "0.18.0", "archiveSha256": "a" * 64}
        result = audit.registry_lockfile(source, provenance)
        packages = tomllib.loads(result)["package"]
        self.assertEqual(packages[0]["checksum"], "a" * 64)
        self.assertEqual(packages[0]["dependencies"], ["approx"])
        self.assertEqual(packages[1], tomllib.loads(source)["package"][1])
        self.assertEqual(audit.registry_lockfile(result, provenance), result)

    def test_unknown_patch_version_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "Unexpected vendored cgmath-next version"):
            audit.registry_lockfile('[[package]]\nname = "cgmath"\nversion = "0.19.0"',
                                    {"version": "0.18.0"})

    def test_standalone_cad_registry_resolution_is_rejected(self):
        def metadata(command, **kwargs):
            manifest = audit.VENDOR / "Cargo.toml"
            if command[-1] == "cad/monstertruck/Cargo.toml":
                manifest = audit.ROOT / "registry/cgmath/Cargo.toml"
            return json.dumps({"packages": [{
                "name": "cgmath",
                "version": "0.18.0",
                "manifest_path": str(manifest),
                "targets": [{"name": "cgmath", "kind": ["lib"]}],
            }]}).encode()

        with patch.object(audit.subprocess, "check_output", side_effect=metadata):
            with self.assertRaisesRegex(
                    ValueError, "cad/monstertruck/Cargo.toml must resolve exclusively"):
                audit.verify_vendor()

    def test_standalone_cad_audit_failure_is_propagated(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for lock in ("Cargo.lock", "cad/wasm/Cargo.lock", "cad/monstertruck/Cargo.lock",
                         "renderer/vendor/cgmath-next-0.18.1/Cargo.lock"):
                path = root / lock
                path.parent.mkdir(parents=True, exist_ok=True)
                name = "standalone-cad" if lock == "cad/monstertruck/Cargo.lock" else "other"
                path.write_text(f'[[package]]\nname = "{name}"\nversion = "1.0.0"\n')
            metadata = {"packages": [{
                "name": "cgmath",
                "version": "0.18.0",
                "manifest_path": str(audit.VENDOR / "Cargo.toml"),
                "targets": [{"name": "cgmath", "kind": ["lib"]}],
            }]}

            def cargo_audit(command, **kwargs):
                source = Path(command[command.index("--file") + 1]).read_text()
                packages = tomllib.loads(source)["package"]
                failed = any(package["name"] == "standalone-cad" for package in packages)
                return audit.subprocess.CompletedProcess(command, int(failed))

            with patch.object(audit, "ROOT", root), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()), \
                    patch.object(audit.subprocess, "run", side_effect=cargo_audit):
                with self.assertRaises(SystemExit) as result:
                    audit.main()
                self.assertEqual(result.exception.code, 1)

    def test_accepted_unsound_advisory_is_ignored(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for lock in ("Cargo.lock", "cad/wasm/Cargo.lock", "cad/monstertruck/Cargo.lock",
                         "renderer/vendor/cgmath-next-0.18.1/Cargo.lock"):
                path = root / lock
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text('[[package]]\nname = "other"\nversion = "1.0.0"\n')
            metadata = {"packages": [{
                "name": "cgmath",
                "version": "0.18.0",
                "manifest_path": str(audit.VENDOR / "Cargo.toml"),
                "targets": [{"name": "cgmath", "kind": ["lib"]}],
            }]}
            seen = []

            def cargo_audit(command, **kwargs):
                seen.append(command)
                return audit.subprocess.CompletedProcess(command, 0)

            with patch.object(audit, "ROOT", root), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()), \
                    patch.object(audit.subprocess, "run", side_effect=cargo_audit):
                with self.assertRaises(SystemExit) as result:
                    audit.main()
                self.assertEqual(result.exception.code, 0)
            for command in seen:
                for advisory in audit.ACCEPTED_ADVISORIES:
                    self.assertIn(advisory, command)

    def test_tampered_vendor_and_wrong_resolution_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vendor = root / "vendor/cgmath-next-0.18.1"
            vendor.mkdir(parents=True)
            manifest = vendor / "Cargo.toml"
            manifest.write_text('[package]\nname = "cgmath"\nversion = "0.18.0"\n\n[lib]\nname = "cgmath"\n')
            upstream_manifest = vendor / "Cargo.toml.orig"
            upstream_manifest.write_text('[package]\nname = "cgmath-next"\nversion = "0.18.1"\n')
            provenance = {
                "upstreamRepo": audit.UPSTREAM_REPO,
                "upstreamVersion": "0.18.1",
                "upstreamRevision": audit.UPSTREAM_REVISION,
                "version": "0.18.0",
                "archiveSha256": audit.REGISTRY_ARCHIVE_SHA256,
                "upstreamArchiveSha256": audit.UPSTREAM_ARCHIVE_SHA256,
                "files": {
                    "Cargo.toml": hashlib.sha256(manifest.read_bytes()).hexdigest(),
                    "Cargo.toml.orig": hashlib.sha256(upstream_manifest.read_bytes()).hexdigest(),
                },
            }
            provenance_path = vendor.parent / "cgmath-next-security.json"
            provenance_path.write_text(json.dumps(provenance))
            provenance_hash = hashlib.sha256(provenance_path.read_bytes()).hexdigest()
            metadata = {"packages": [{
                "name": "cgmath",
                "version": "0.18.0",
                "manifest_path": str(manifest),
                "targets": [{"name": "cgmath", "kind": ["lib"]}],
            }]}
            with patch.object(audit, "ROOT", root), patch.object(audit, "VENDOR", vendor), \
                    patch.object(audit, "PROVENANCE", provenance_path), \
                    patch.object(audit, "PROVENANCE_SHA256", provenance_hash), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()):
                self.assertEqual(audit.verify_vendor(), provenance)
                manifest.write_text("unreviewed change")
                with self.assertRaisesRegex(ValueError, "differs from the reviewed"):
                    audit.verify_vendor()
                manifest.write_text('[package]\nname = "cgmath"\nversion = "0.18.0"\n\n[lib]\nname = "cgmath"\n')
                (vendor / "injected.rs").write_text("unreviewed file")
                with self.assertRaisesRegex(ValueError, "differs from the reviewed"):
                    audit.verify_vendor()
                (vendor / "injected.rs").unlink()
            metadata["packages"][0]["manifest_path"] = str(root / "registry/cgmath/Cargo.toml")
            with patch.object(audit, "ROOT", root), patch.object(audit, "VENDOR", vendor), \
                    patch.object(audit, "PROVENANCE", provenance_path), \
                    patch.object(audit, "PROVENANCE_SHA256", provenance_hash), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()):
                with self.assertRaisesRegex(ValueError, "resolve exclusively"):
                    audit.verify_vendor()
            wrong_cad_metadata = json.dumps(metadata).encode()
            metadata["packages"][0]["manifest_path"] = str(manifest)
            with patch.object(audit, "ROOT", root), patch.object(audit, "VENDOR", vendor), \
                    patch.object(audit, "PROVENANCE", provenance_path), \
                    patch.object(audit, "PROVENANCE_SHA256", provenance_hash), \
                    patch.object(audit.subprocess, "check_output", side_effect=[
                        json.dumps(metadata).encode(), wrong_cad_metadata,
                    ]):
                with self.assertRaisesRegex(ValueError, "resolve exclusively"):
                    audit.verify_vendor()


if __name__ == "__main__":
    unittest.main()
