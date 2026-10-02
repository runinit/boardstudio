# Private tree projection contract harness

Read-only native harness for the tree source from the migration worktree. The harness includes the copied `tree.rs` in the same `crate::presentation::objects::tree` module path, preserving its private visibility. It opens real REVIUNG41 and Sofle ProjectDoc fixtures through `Session::Open`, `CoreEngine::handle`, and Session Core/Persist completions. Board-switch checks use `Session::Navigate`.

## Provenance

- Worktree HEAD at test run: `96811f921b685e65fe664e7c37062026ae8f75c2`; requested source baseline `8ea71a84`.
- `web/src/presentation/objects/tree.rs` SHA-256: `cabf98db27bea8e52f562268f017c7ed4185e4a1bbdd6d59710e32649c2510ec`.
- Copied source `tree.rs.source-8ea71a84` SHA-256: same (`cabf98db27bea8e52f562268f017c7ed4185e4a1bbdd6d59710e32649c2510ec`). This copy keeps the tested implementation fixed if the worktree source changes.
- REVIUNG41 fixture `web/target/builds/m1-release-20261001-050e9282/fixtures/reviung41.json` SHA-256: `b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae`.
- Sofle fixture `web/target/builds/m1-release-20261001-050e9282/fixtures/sofle.json` SHA-256: `800080b686e449d3e14490e3f176a3c069d0d4d0f50e3e25a94440f6201f64e4`.

## Coverage and result

`cargo test --manifest-path /tmp/frontend-run/tree-contract-tests/Cargo.toml -- --nocapture` passed: 5 tests, 0 failures.

- REVIUNG41: tree rows, live key counts, matrix source membership order, row/column coordinate filters, and real generated diode companion IDs. The expected set is derived from the accepted Core scene plus actual document membership and board membership.
- Sofle: left/right board filtering before and after Session navigation.
- Modified REVIUNG41: disabled configured cell remains a semantic key with `Some([])`, while a matrix removed from the actual ProjectDoc resolves as `None`.
- Modified REVIUNG41: a real fixture component is removed from `parts` while its board membership is stale; Core accepts this after removing its layout ownership, and tree resolution rejects the stale component ID.
- Modified REVIUNG41: an actual layout receives a mirror link and Core accepts the document; tree shows half groups and both source and target as Linked.

All harness sources, generated lockfile and build output are under `/tmp/frontend-run/tree-contract-tests`. No repository files were changed by this verifier. `git status --short` showed preexisting untracked `.scratch/dioxus-case-workspace/`, `.scratch/dioxus-export-workspace/`, and `.scratch/dioxus-shared-viewer/`; they were left untouched.
