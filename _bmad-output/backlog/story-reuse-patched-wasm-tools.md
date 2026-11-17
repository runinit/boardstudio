---
id: 1
type: story
title: "Reuse the patched Wasm tools across unrelated workspace changes"
covers: []
after: []
refined: true
hitl: false
risk: medium
---

# Reuse the patched Wasm tools across unrelated workspace changes

## Description

Both web jobs build the patched pair on a cold miss. On dev `a6cfa5183`, changing only the Library package version changed the cache input while the builder still produced both tools in 37.3 seconds locally. The risk is restoring an incompatible runner and invalidating browser verification.

## Requirements

1. **Reuse across unrelated changes.** Given a populated cache, when an unrelated workspace package version changes, both consumers compute the same key as before and reuse the pair without running the builder.
2. **Invalidate on build inputs.** List the inputs used to build the pair and the inputs used only to validate workspace compatibility. For each build input, show which part of the key covers it. Record why unrelated workspace lock entries can be excluded. Changing the patch, pinned tool sources, compiler identity, target OS or architecture, toolchain configuration or relevant Cargo configuration must change the key. On a genuine miss the builder produces both pinned executables.
3. **Check compatibility before lookup.** Given a populated cache, when the workspace lock contains a missing, ambiguous or incompatible wasm-bindgen version, validation fails before restore and before either consumer starts.
4. **Check restored tools.** Given a cache hit, when either executable is absent, cannot execute, or reports the wrong version, the job fails before using the pair. A valid hit verifies both executable names and versions and continues.

From the repository root, run `python3 scripts/test-build-patched-wasm-bindgen.py` and `python3 scripts/test-ci-workflow.py`. Confirm the named controls exercise local hits and misses, independent key-input changes, an incompatible workspace lock with a prospective hit, and damage to each executable. Both suites must pass. Label any elapsed time as a local observation.

## Boundaries

Keep the packaged and Wasm matrix jobs, their dev-specific checks, patched binary selection, and browser isolation. Work locally. Hosted miss frequency and workflow wall-time savings are unknown.

## References

- Local evidence — `.evidence/build-audit/pair-key-control.log`, workspace inputs.
- Local evidence — `.evidence/build-audit/pair-key-build.log`, successful build and incompatible version control.
