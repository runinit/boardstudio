#!/usr/bin/env python3
"""Run native Cadrum tests (build-cadrum-wasm.py builds the WASM provider)."""
from cadrum_build import test_cadrum

if __name__ == "__main__":
    test_cadrum()
