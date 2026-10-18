#!/usr/bin/env python3
"""Run required regressions against the current packaged WEB provider without compiling it."""
from cad_build import test_provider

if __name__ == "__main__":
    test_provider()
