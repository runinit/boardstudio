# Independent Standards: final Case mount

Reviewed integration HEAD `75139d74` plus released dirty source: presentation SHA-256 `3ffa2a3d8954328465f1d95fce5b405981e2473486cd880aad4a5986310b95c8` (blob `ab958ca1`), CSS SHA-256 `b2630fd1faa94b6cf0fc002da35de72a34489f656354d0773ebb0ac42be5287b` (blob `f89415b6`), CAD presentation blob `d71bfa4e`, controller `6cac7c86`, editor `ff666cdc`, helper `a9af159e`.

**Resolved P2 — Size the Case scrollport to the space below the panel heading.** `web/assets/m1.css:275` assigns `min-height:100%` to `.m1-case-bodies` and its generated variant. These are direct children of `.m1-panel-content`, which has a definite `height:100%` (`:127`), alongside the desktop heading with `flex:0 0 42px` (`:130`; `panels.rs:248`). Their minimum therefore consumes the entire panel before the heading, preventing flex shrink and extending the scrollport at least 42px beyond the ancestor clipped by `.m1-editor-body`. Bottom content/scrollbar can become inaccessible. Follow the existing Parts/Keymap child pattern: `flex:1; min-height:0; overflow:auto`. This conflicts with `CONSTRAINTS.md:161–163`, preserving desktop/compact overflow behavior and long-content access. Source-derived finding; public reproduction remains pending.

Otherwise the bounded mount is coherent. One `has_inspector` controls the grid track, compact Inspect control and panel mount. Case joins the existing Inspector rather than introducing another panel owner; new child hooks are unconditional within their own components. Workspace entry opens compact Inspect and closes Objects through the existing effect. Parts/Layout branches are retained.

Configured-board navigation wraps the existing closure with captured full Scope and `instance_id=None`. It validates generation, current scope and target membership before drag/context/anchor cleanup. CAD settings suppression and fresh admission both use the effective Case projection, preserving another board’s configuration. CSS uses existing light/dark tokens, visible focus and compact controls; visual/AT behavior is not verified here.

Runtime-only helper exception is appropriately bounded; provider source/asset assertions remain. Stage/hash currently untracked modules before build provenance capture.

RF: centralized Inspector eligibility removes duplicated workspace conditions; no new material Fowler smell. No edits/Cargo/browser execution. Tracked diff whitespace passed. Prior editor/controller findings remain source-closed; public, compiler, persistence/retry and responsive/AT gates remain open.

## Narrow correction verification

Verified final CSS SHA-256 `864e3749aa2e9b34c1ea90559e7560dc03e9d23149020c4af890ceb592191b45`, blob `508205e13d4f81e9c7c133acc0c0f9a9529f88c6`. Both Case classes now use `flex:1; min-height:0; overflow:auto`, allowing the scrollport to occupy and shrink within the remaining space below the fixed heading. The reported P2 is source-closed; no material finding remains in the reviewed mount/CSS delta. CSS diff whitespace check passed. No browser/layout execution was performed. Subsequent compiler-driven Rust corrections are outside this CSS verification and await their final source release.
