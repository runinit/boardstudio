# UI wave serial integration — 2026-10-02

Integration worktree `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`, branch `codex/rust-v1-ui-parity-20261001`.
Start `bd671ae8db8897388d26ebf973380c69efb9bffd`.
Released HEAD **`dd79af5ba6bc2af304c1412b29e62f2de8b3f8a8`**.

Integrated only the already Spec+Standards-cleared F6 firmware-position editor:
- `32dd07c0a5179fa4c995a1da668365b4e11df2d3` -> `8f237d5f` (source)
- `e4366bdb096f509d18127408f3a0f67f505a6764` -> `dd79af5b` (existing browser evidence)

Both cherry-picks applied cleanly; presentation.rs auto-merge required no manual edits. All four feature leaf files (firmware position choices/presentation, PCB controller, firmware CSS) are byte-identical to the reviewed source. No feature semantics/public API/visibility change, lint suppression or acceptance closure.

Executed at integrated source:
- `cargo fmt --manifest-path web/Cargo.toml -- --check`: PASS
- `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page`: **55 pass** (12 library,37 binary,6 mounted Keycaps harness), `/tmp/frontend-ui-wave-native-integration.log`
- `cargo clippy --locked --manifest-path web/Cargo.toml --no-default-features --features page --target wasm32-unknown-unknown --all-targets -- -D warnings`: PASS, `/tmp/frontend-ui-wave-wasm-integration.log`
- `git diff --check`: PASS
- Original tracked dirty diff is byte-identical and every original untracked path remains present; snapshot `/tmp/frontend-ui-wave-integration-before.json`.

Source/demo distinction: current source includes F6 at dd79af5b; the latest full build remains root's bd671ae8 `frontend-keycaps-integrated-20261002` served at34720. That demo does not contain the new F6 integration. Existing source-specific browser evidence does not automatically certify this new integrated demo.

Not merged:
- Numeric07 final f0328aee: Standards repaired-source CLEAR in `/tmp/numeric07-standards-source-review-20261002.md`; independent Spec explicitly pending. Do not integrate until its ack.
- Provider/archive f3f58a6a: independent Spec pending; complete source findings/red-green/repair report `/tmp/project-archive-provider-standards-review-20261002.md`.
- Align533f9a50: Standards blocked on two lifecycle defects; `/tmp/layout-align-standards-source-review-20261002.md`.
- Keycaps docs47a02cff/f622c259: inspected overlap and preserved existing untracked F6C.4 findings `spec.md`.47a02cff would add F6C.2 settings at the same path. Author is committing unique `settings-spec.md` and corrected links; replay equivalent distinct final files without overwriting existing F6C.4 spec.

Root owns subsequent ledger/report copies and source/demo-status reconciliation after this explicit checkout release. No further integration write is in progress by this merger.
