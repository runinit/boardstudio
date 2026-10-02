# Independent Standards: private host inclusion repair

Reviewed `89a5f46bcf160e50fa0088138a8a97b6b6778c65...cdbac52219666c7547d787ed08c1ba31f2ea649d` (two files). Original `renderer_host.rs` remains blob `261ef5bfe1dc984f24be44c95873a684c4940968`, SHA-256 `377f45e0ad51f5979dcba152d6b14dec71ea035991ee031b370829b5ac5b06e6`.

No material source-behavior Standards finding in the repair. Read-only byte comparison proves `renderer_host_page_base.rs` equals the original with exactly the three leading `//!` prefixes normalized to `//`; snapshot SHA-256 is `e0ab53bb6ee004b5cfbb4f360bc347ee022376a01968ee27bbcbbe4c05495c67`. Wrapper SHA-256 is `20640ccb088b8767087aa3cd2b6647b81b400a64d0fcd3e8590c9a0abd9010c2`. The wrapper still includes the baseline and extensions into one private page module. No library host changes, public visibility expansion, new build configuration, provider copies or global registry are introduced.

The copied implementation is a deliberate temporary duplication seam justified by the confirmed inner-documentation include failure and the unchanged-library/API constraint. Provenance is explicit and the assertion compares the entire normalized source. This does not close RF-002; do not maintain the snapshot independently.

Two integration obligations remain:

- Update `docs/migration/f7-case-viewer-private-contract.md:9–10`, which still describes unchanged direct host-source inclusion, to record the normalized private snapshot and synchronization responsibility.
- Make the synchronization check executable in the chosen verification path. At this commit the worker has no module registration; a wasm-only page registration will not execute this `#[cfg(test)]` assertion during ordinary native page tests. The assertion’s presence is not execution evidence.

The confirmed narrow E0753 diagnosis explains this repair but is not a successful page/WASM build. Compiler, mounted lifecycle/disposal, and public acceptance gates remain open. Previously reviewed pointer/applied-scene safeguards are unchanged. RF-012 reflective capability verification remains open. Diff whitespace passed; no Cargo/browser run or source mutation.
