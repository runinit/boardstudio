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
        with self.assertRaisesRegex(ValueError, "Unexpected vendored cgmath version"):
            audit.registry_lockfile('[[package]]\nname = "cgmath"\nversion = "0.19.0"',
                                    {"version": "0.18.0"})

    def test_tampered_vendor_and_wrong_resolution_fail_closed(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            vendor = root / "vendor/cgmath-0.18.0"
            vendor.mkdir(parents=True)
            manifest = vendor / "Cargo.toml"
            manifest.write_text("reviewed source")
            provenance = {"patchedAdvisory": audit.PATCHED_ADVISORY,
                          "files": {"Cargo.toml": hashlib.sha256(manifest.read_bytes()).hexdigest()}}
            (vendor.parent / "cgmath-security.json").write_text(json.dumps(provenance))
            metadata = {"packages": [{"name": "cgmath", "manifest_path": str(manifest)}]}
            with patch.object(audit, "ROOT", root), patch.object(audit, "VENDOR", vendor), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()):
                self.assertEqual(audit.verify_vendor(), provenance)
                manifest.write_text("unreviewed change")
                with self.assertRaisesRegex(ValueError, "differs from the reviewed"):
                    audit.verify_vendor()
                manifest.write_text("reviewed source")
                (vendor / "injected.rs").write_text("unreviewed file")
                with self.assertRaisesRegex(ValueError, "differs from the reviewed"):
                    audit.verify_vendor()
                (vendor / "injected.rs").unlink()
            metadata["packages"][0]["manifest_path"] = str(root / "registry/cgmath/Cargo.toml")
            with patch.object(audit, "ROOT", root), patch.object(audit, "VENDOR", vendor), \
                    patch.object(audit.subprocess, "check_output", return_value=json.dumps(metadata).encode()):
                with self.assertRaisesRegex(ValueError, "resolve exclusively"):
                    audit.verify_vendor()


if __name__ == "__main__":
    unittest.main()
