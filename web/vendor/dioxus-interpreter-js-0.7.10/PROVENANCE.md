# Dioxus interpreter provenance

- Published package: `dioxus-interpreter-js` 0.7.10, extracted from its Cargo registry archive.
- Published archive checksum recorded by BoardStudio's original lockfile:
  `faa22cc3431e7efdbae28069542a1763c977d5edbe4d4fbfb0b7667b9c9966e9`.
- Upstream source: <https://github.com/DioxusLabs/dioxus/tree/57d6794ad60b949e5bd8aa282f6f8c3dc97a365e/packages/interpreter>.
- Licenses: upstream [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), copied from that revision.
- Integration: root Cargo patch; original version, dependencies, features and public API retained.

The baseline commit contains the published source unchanged, plus this provenance, license texts
and a Node listener regression. The subsequent repair is limited to listener routing-id lifetime
in `src/unified_bindings.rs` and event/callback bookkeeping in `src/ts/core.ts`. The upstream bundle
mechanism regenerates `src/js/core.js` and its existing hash metadata.

Run the focused regression with `node tests/listener-removal.cjs` from this directory. It exercises
the actual bundled interpreter and the registration/removal statement bodies used by the Rust
binding generator. It is a Node EventTarget unit test; mounted WASM checks are separately required.

Rationale, exact source/test pins, failures and verification remain in
`docs/plans/rust-ui-build/shared-control-prerequisites.md` and
`docs/plans/rust-ui-build/pinned-renderer-repair-proposal.md` at the BoardStudio repository root.
