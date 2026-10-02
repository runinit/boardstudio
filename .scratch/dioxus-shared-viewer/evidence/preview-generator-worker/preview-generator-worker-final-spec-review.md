# Preview generator final reserved-net delta

**Spec clear at commit `f6dd310bbf449b64f165780b1a00429d9d4e9d50`.** Exact source hashes:

- Worker: `6bae601ac2f452c542de9fe91b31a1c07a5ec58bad59efe4813889b43f8a03f6`
- Builder: `3896278f0022672fc98ed9d6e3c42a5effdfc227c6dfb011e63f3ac642af754e`
- Tests: `cd6a791d2320eaea9d33995e93ffa843ac04ecc5a20f1bc954897b7572d61961`

The final validation delta removes the extra nonempty/unique-name restrictions on reserved nets. This matches Core’s `validate_net_transition`: the existing reserved prefix must remain exactly unchanged, repeated reserved names are permitted, and indices remain unique/nonzero. Empty strings are likewise not rejected by that Core transition. The worker still requires strings and bounded integer indices.

React’s allocator uses the first `nets.find(net => net.name === name)` match. The worker retains that exact behavior for repeated and empty names, clones the reserved table without collapsing entries, and appends only previously unseen names with monotonically increasing indices. Core remains responsible for the complete plan/result validation. The added repeated-name test asserts both unchanged reserved rows and first-index use in actual converter output. Empty-name acceptance was verified in source; no separately executed empty-name regression is claimed.

Previous source clearance for ordered jobs, identities, converter/path/arc reuse, correlated replies and the relative-module build remains applicable. No new API, provider, Core initialization or React UI dependency was added.

No source edits, Cargo or test rerun performed. Actual Runtime/Prepare-Finish integration, physical projection, waiter lifecycle, public model delivery and root/subpath offline checks remain open.
