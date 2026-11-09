# 01: Retire the unused vendored cgmath 0.18.0 fallback

Status: resolved
Type: build
Blocked by: None (can start immediately)

**What to build:** Remove the retained, unused `cgmath 0.18.0` fallback vendor tree and its superseded provenance manifest so only the active, security-audited `cgmath-next 0.18.1` fork remains in the repository, removing ~11.3k lines of dead vendored Rust and keeping workspace metadata, vendor documentation, and security/provenance checks accurate.

## Acceptance criteria

- [x] The unused `cgmath 0.18.0` vendor directory, its superseded `cgmath-security.json` manifest, and its workspace `exclude` entry are removed, and the vendor README no longer describes the retired fallback as present.
- [x] All Cargo workspaces continue to resolve `cgmath` exclusively to the reviewed `cgmath-next 0.18.1` source without lockfile drift.
- [x] `python3 scripts/check.py repo tooling security lint` passes.

## Answer

Removed the unused fallback and superseded manifest; the reviewed successor and its provenance manifest are unchanged. Locked Cargo metadata for both the root and `cad/wasm` workspaces resolves `cgmath` exclusively to `renderer/vendor/cgmath-next-0.18.1/Cargo.toml`; neither lockfile changed.

Verification: `python3 scripts/check.py repo tooling security lint` passed (25.7s); log: `.evidence/cleanup/ticket01-check.log`. Combined spec/standards review found no blocking issues.
