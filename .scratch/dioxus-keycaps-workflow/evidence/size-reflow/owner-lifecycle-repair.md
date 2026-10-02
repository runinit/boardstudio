# F6C.3 delayed action and feedback ownership repair

Repair branch: `codex/keycaps-size-feedback-repair-20261002`, based on reviewed integration HEAD `1ac59081180df9f87d50e4bdc357c844d60ad33a`. Independent source-review report: `/tmp/keycaps-size-reflow-source-independent-review-20261002.md` (SHA-256 `ff1abcb0e6f80bca864695e3d6b3feda636a8462fb6e1ca103db8a52d4615ed8`).

## Red/green browser regressions

The production `KeySizeControls` is mounted in Chromium through `wasm-bindgen-test`. Its callback applies the same live owner admission condition as the editor controller.

1. `delayed_keyboard_resize_cannot_be_retargeted_to_a_new_mixed_selection` schedules A's width change, changes the mounted projection to mixed selection B before the 150 ms deadline, then asserts no request is admitted for B. Against the reviewed behavior it failed with the test message that A's delayed keyup became a B-owned edit. After the correction it passes.
2. `delayed_keyboard_resize_commits_for_its_unchanged_owner` asserts the normal same-owner debounced path still submits the captured owner and width. It passes.
3. `feedback_from_another_selection_is_not_shown_in_the_inspector` mounts A's failure feedback, changes the mounted projection to B, and verifies A's status/error are hidden while B-owned failure feedback remains visible. It failed with request-ID-only rendering and passes with owner-qualified feedback.

Run all three mounted regressions:

```sh
WASM_BINDGEN_TEST_WEBDRIVER_JSON=/home/chris/.local/share/boardstudio/worktrees/keycaps-size-feedback-repair-20261002/.scratch/dioxus-frontend-tranche-1/evidence/new17-compact-guide-repair-20261002/webdriver.json \
  wasm-pack test --headless --chrome web --no-default-features --features page \
  --bin boardstudio-web -- keycap_size_controller::tests
```

Result after repair: **3 passed**. The root-scope key-size ticket and F3.2/F3.5 joins remain open.

## Source change

- Keyboard intents now retain the event-time owner, snapshot token, revision, axis, and units. The submit callback rejects that immutable envelope if a rerender has installed a different owner/source.
- Owner/source changes invalidate the pending keyboard timer using a nonreactive generation cell, so a delayed A action cannot adopt B's draft.
- Every controller feedback state carries the originating owner and the controls render it only for the current owner.

## Checks and limits

- `cargo fmt --manifest-path web/Cargo.toml --check`: passed.
- `cargo clippy --manifest-path web/Cargo.toml --target wasm32-unknown-unknown --all-targets --features page -- -D warnings`: passed.
- `cargo test --manifest-path web/Cargo.toml --bin boardstudio-web presentation::objects::keycap_resize::tests -- --nocapture`: 4 passed, 1 fixture-gated ignored.
- `git diff --check`: passed.
- The previous accepted Core linked-layout test was not rerun here because both `/tmp/keycaps-fit-fixture.boardstudio` and `/tmp/keycaps-fit-fixture-project.json` are absent. The earlier green result and fixture hashes remain in `replace-document-mirror-core.md`; this repair did not recreate or modify those paths.
- The reviewed required overlap warning is still absent from the Layout Inspector. Keep it as explicit incomplete F6C.3 work. Paired React/Dioxus journey, visual Inspector placement, full history/save/reopen, and parent joins are not cleared by these mounted component tests.
