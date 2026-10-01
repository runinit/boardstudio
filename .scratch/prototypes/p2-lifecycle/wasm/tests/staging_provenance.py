"""The staging CLI must reject provider bytes without matching source provenance."""
import hashlib
import importlib.util
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

WASM = Path(__file__).resolve().parents[1]


class StagingProvenance(unittest.TestCase):
    def test_embedded_non_rust_input_changes_provider_fingerprint(self):
        spec = importlib.util.spec_from_file_location('p2_build_release', WASM / 'build-release.py')
        builder = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(builder)
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            (repo / 'core/src').mkdir(parents=True)
            (repo / 'core/tests/fixtures').mkdir(parents=True)
            (repo / 'rust-toolchain.toml').write_text('[toolchain]\nchannel = "1.98.0"')
            (repo / 'core/src/lib.rs').write_text('const PART: &str = include_str!("../tests/fixtures/part.kicad_mod");')
            fixture = repo / 'core/tests/fixtures/part.kicad_mod'
            fixture.write_text('(footprint "original")')
            builder.REPO = repo
            builder.P1 = repo / '.scratch/prototypes/p1-core'
            original = builder.provider_sources()
            fixture.write_text('(footprint "changed")')
            changed = builder.provider_sources()
            self.assertNotEqual(original, changed, 'embedded source changes must invalidate provider provenance')

    def test_changed_provider_source_cannot_be_staged_as_current(self):
        with tempfile.TemporaryDirectory() as temporary:
            repo = Path(temporary)
            wasm = repo / '.scratch/prototypes/p2-lifecycle/wasm'
            wasm.mkdir(parents=True)
            shutil.copy2(WASM / 'stage-site.py', wasm / 'stage-site.py')
            shutil.copytree(WASM / 'assets', wasm / 'assets')
            (wasm.parent / 'fixtures').mkdir()
            (wasm.parent / 'fixtures/reviung41.json').write_text('{}')
            (wasm.parent / 'evidence/renewal-worker').mkdir(parents=True)
            public = repo / 'public'
            (public / 'assets').mkdir(parents=True)
            (public / 'index.html').write_text('<html></html>')
            source = repo / 'core/src/lib.rs'
            source.parent.mkdir(parents=True)
            source.write_text('// old source')
            source_hash = hashlib.sha256(source.read_bytes()).hexdigest()
            assets = {str(p.relative_to(wasm / 'assets')): hashlib.sha256(p.read_bytes()).hexdigest()
                      for p in (wasm / 'assets').rglob('*') if p.is_file()}
            (wasm / 'asset-provenance.json').write_text(json.dumps({
                'schema': 1, 'source_files': {'core/src/lib.rs': source_hash}, 'assets': assets,
            }))
            source.write_text('// changed source without rebuilding WASM')
            result = subprocess.run(['python3', str(wasm / 'stage-site.py'), 'root', 'test', str(public)],
                                    capture_output=True, text=True)
            self.assertNotEqual(result.returncode, 0, result.stdout)
            self.assertIn('provider source changed', result.stderr)


if __name__ == '__main__':
    unittest.main()
