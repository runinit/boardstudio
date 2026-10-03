# PCB wiring mode source receipt

Implementation commit: `1e5ef6ac561136101f13cb5438a4f3e5d52cb6ba`  
Base commit: `3f5453e14b9610257c635de975d12c8ddeb505bc`  
Contract: `d5e527a150ea354efc68091084bf29e2d24f0da9` amended by `882b64f96bccf70a2b41650ae89bc16432eeda15`.

The packet adds a private Matrix/Direct selector to the board-level Wiring Inspector and an Editor-lifetime action owner. The request preserves selection-independent board-plan identity separately from the rendered UI selection and generation. Admission checks PCB workspace, current runtime scope, session/document/board/instance, snapshot token/revision, selected part, generation, current plan, Ready/Saved lifecycle, and the absence of preview or gesture. A changed request submits one normal `ReplaceDocument` edit after registering its exact outcome observer. The accepted source remains the value oracle. The proposal changes only the target board mode and preserves the target configuration, other boards, and all other project data; selecting the effective existing mode is a no-op.

Verification was rerun against the frozen implementation commit in this worktree. The logs are immutable source receipts for these commands:

| Check | Result | Log SHA-256 |
|---|---|---|
| `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web` | 144 passed, 1 ignored | `8a2cf4da09c5bb44984e1cba51d0c48d05352d27588fe8f7f86425cd1a133ac1` |
| `cargo check --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page` | Passed | `89a12e3b8d6d5800e8f43b3aa65c9e6fde195dba7ec2e2a20bbdd6c8f0eb22e9` |
| `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --features page -- -D warnings` | Passed | `beed191cf5cd39a57477a6d66e9acb53f3b4efe7566a30e0ada805357a0fe6ff` |
| `cargo fmt --manifest-path web/Cargo.toml -- --check` | Passed | `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855` |

The pure proposal tests verify the existing configuration fields (controller, locks, assignments, key bindings, jumper states, protected handoff), unrelated document values, other-board configuration, default creation, missing-board rejection, and no-op mode selection.

This source receipt does not claim production-mounted settlement or packaged UI acceptance. The paired public mode change, exact accepted save transition, single history entry, Undo/Redo, and save/reopen journey remain open for the serial integrated build. Apply Wiring remains a separate follow-up.
