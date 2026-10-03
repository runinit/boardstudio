# PCB wiring mode source repair receipt

Base implementation: `1e5ef6ac561136101f13cb5438a4f3e5d52cb6ba`.  
Contract: `d5e527a150ea354efc68091084bf29e2d24f0da9` amended by `882b64f96bccf70a2b41650ae89bc16432eeda15`.  
Reviewer findings addressed: `/home/chris/.local/share/boardstudio/reviews/pcb-wiring-mode-source-review-1e5ef6ac-sol-20261002.md` (SHA-256 `60f1d41c4da19026f5bc628f740976fb6a479d00a1ad665e302b96969269d431`).

The repair separates two identities with different lifetimes. The edit request retains accepted board-plan token/revision and rendered selection identity, so stale controls remain rejected. Post-settlement feedback uses a stable UI target (scope, selected part and generation), so successful feedback survives the accepted revision/token advancing. Failure feedback is shown only while the original accepted plan remains current.

The new mounted VirtualDom tests mount the production `use_board_wiring_mode_edits` owner against a native Runtime test adapter. They verify one current `ReplaceDocument` edit, successful exact-operation settlement after accepted revision advance, stale retained-control rejection after selection changes, and rejected/executor/persistence/cancelled outcomes leaving the accepted Matrix mode untouched. The identity tests verify selection and generation admission separately from board-plan identity, and that feedback survives plan revision advancement while selection changes still hide it.

Verification logs were captured after the repair and are immutable under this directory:

| Check | Result | Log SHA-256 |
|---|---|---|
| `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web` | 149 passed, 1 ignored | `4f18bd6a6f208094c114300725a9314b94da87d086a663ec04daeeeb392cf5b0` |
| `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page` | Passed | `beed191cf5cd39a57477a6d66e9acb53f3b4efe7566a30e0ada805357a0fe6ff` |
| `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page -- -D warnings` | Passed | `8950be1e8f4499f41b8e38ef15f09a0e2671be53a43aa5dec4d794320908881d` |
| `cargo fmt --manifest-path web/Cargo.toml -- --check` | Passed | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |
| `git diff --check` | Passed | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The paired packaged browser journey, save/reopen, Undo/Redo, and full parent acceptance remain open. Apply Wiring remains a separate follow-up.
