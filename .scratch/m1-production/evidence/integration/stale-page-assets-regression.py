from pathlib import Path
import json
import sys
p=json.loads(Path(sys.argv[1] if len(sys.argv) > 1 else 'web/target/builds/m1-release-20261001-bc1c6b52/provenance.json').read_text())
for mode in ['root','subpath']:
 files=[name for name in p[mode]['assets'] if name.startswith('assets/boardstudio-web_bg-') and name.endswith('.wasm')]
 print(mode,files,flush=True)
 assert len(files)==1, f'{mode}: release includes obsolete Dioxus page WASM binaries'
