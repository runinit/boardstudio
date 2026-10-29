#!/usr/bin/env python3
"""Outside the Node-free probe: emit a Node reference from verified local inputs."""
import argparse
from pathlib import Path
import subprocess
import sys

import verified_runtime

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('checkout', type=Path)
parser.add_argument('--npm-cache', type=Path, default=Path.home() / '.npm/_cacache/content-v2/sha512')
args = parser.parse_args()
with verified_runtime.verified_runtime(args.checkout, args.npm_cache) as prepared:
    subprocess.run(['node', str(Path(__file__).with_suffix('.cjs')), 'reference', str(prepared)],
                   check=True, stdout=sys.stdout)
