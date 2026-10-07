#!/usr/bin/env python3
"""Download, verify and unpack the pinned native or WASM OCCT archive."""
import argparse
from cadrum_build import prepare

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", choices=("native", "wasm"))
    print(prepare(parser.parse_args().target))
