# F3.5b source contract and implementation boundary

**Source base:** `dd71c27ba1f54fd0cfdfebc3b7f56c4c89304a5a` (`layout-findings-footer-20261003` isolated worktree).
**Pinned React source:** `5a472a94`.
**Canonical task graph:** F3.5 stays `implementing`; no task graph or 62-parent count/dependency/status changed.

## React source pins

| File | SHA-256 | Consumed behavior |
|---|---|---|
| `app/src/ui/findings.ts` | `8de1532452aaee26333e62f53631d8b3bdc5180d184140d5c059973fa01d0db7` | `presentedFindings`, target precedence and labels. |
| `app/src/ui/FindingList.tsx` | `63254e5e7c22a4a043a67123645c0012f52e46ec7aab27a8f96529ca712d081c` | Target grouping, severity/message, action labels and omission when unresolved. |
| `app/src/ui/Workbench.tsx` | `8c85a0857550c5ba860a4151a1d0a5789b7bcfaf5dbd349f6eadf29e300e6220` | `showFinding`, inspector page/back/Escape/focus, footer count/action and pending cross-board resume. |

## Existing Dioxus seams at source base

| File | SHA-256 | Reused capability |
|---|---|---|
| `web/src/presentation/keycaps_fit.rs` | `9cb18b05e9d21bc8eb5dc99c244b7e3c5b8c54107cd32dee37118b30ed02bc80` | Existing general target/grouping, accepted finding-marker bounds and Layout camera bounds. Layout request owns a distinct accepted source; no Keycaps fit lifecycle/state is transferred. |
| `web/src/presentation/objects/tree.rs` | `88fc92395311e9a842f1c8911414c02da6e27c881b151459fb45597cb2848257` | Current accepted Part/Matrix/Outline/Board target contexts. |
| `web/src/presentation.rs` | `c97d8c154d12881648d8d573a431ed7c91b01405431331833e9d13d4f4bf1975` | `LayoutOwnerIdentity`, current-scope selection adapter, `select_tree`, board navigation, existing fit and focus callbacks. |
| `web/src/presentation/layout_workspace.rs` | `462669645416b52ec79f655ee6fa6f552381bcc574b16b37f380c71e5d29b333` | Private Layout Inspector mount. |
| `web/src/presentation/canvas_status_footer.rs` | `f4d95b64c52e257d9613e3e8e5bf79a478aef019be250a9aac247745a4273123` | Existing shared canvas footer mount used by Layout and sibling 2D workspaces. |

## Scope and checks

The child implements the actual Layout findings footer and contextual page, deriving the count/list from the accepted snapshot scene and document. Same-board routes use the existing tree-selection callback and camera helper. Cross-board requests retain the source scope/token/revision/finding, invoke the existing board-navigation callback, and resume only when the same session/document/token/revision and resolved target board are current and the accepted scene still presents that finding. Existing unsupported Case-body/mechanical-layer targets receive no Layout action.

The changed paired public-browser journey and combined affected compile are pending the root integrated candidate. No new test suite or local heavy build is part of this ordinary reversible UI packet. `cargo fmt --all -- --check` exposed one pre-existing unrelated formatting drift in `presentation/library.rs`; that unrelated file was restored unchanged. Changed Rust source was formatted with `rustfmt`; `git diff --check` is clean.

**RF handoff:** No new refactoring takeaway observed in the F3.5b findings footer/navigation slice. Preserve RF-001–015 and the canonical register; this receipt is the slice-specific observation.
