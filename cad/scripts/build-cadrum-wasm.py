#!/usr/bin/env python3
"""Build the Cadrum WASM provider using the pinned container toolchain."""
import argparse

from cadrum_build import build_wasm

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-opt", action="store_true", help="skip wasm-opt for checked builds")
    build_wasm(no_opt=parser.parse_args().no_opt)
