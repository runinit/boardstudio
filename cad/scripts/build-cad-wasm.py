#!/usr/bin/env python3
"""Build the pure-Rust Monstertruck CAD WASM provider."""
import argparse

from cad_build import build_wasm

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-opt", action="store_true", help="skip wasm-opt for checked builds")
    build_wasm(no_opt=parser.parse_args().no_opt)
