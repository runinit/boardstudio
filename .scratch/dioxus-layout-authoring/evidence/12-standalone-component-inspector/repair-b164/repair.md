# F3.5a retained-owner and numeric-input repair

This isolated follow-up repairs the source HOLD reported against frozen F3.5a source `b164899fab2ed5a37c9785baf17c70802a6aaf4f`. It does not widen issue 12's standalone-component scope or claim parent F3.5 acceptance.

## Findings addressed

- A monotonic private Inspector lifetime now advances whenever the selected standalone component route changes across workspace, scope, accepted snapshot token/revision, or part identity. Each action checks its captured generation against this live lifetime before any Runtime operation. This prevents a retained A callback from becoming current again after A→B→A when the broader document and scope owner remain unchanged.
- The existing production callback now uses one private dispatcher that can be mounted in the focused regression fixture. Current edits still submit the existing MoveParts, ReplaceDocument, SetConstraint, and RemoveConstraint operations; PCB navigation uses the existing workspace signal.
- Part edge margin now follows the reference keyboard behavior: Enter blurs the field and commits through the existing blur handler; Escape restores the accepted margin draft and clears its local error without submitting an edit.

## Production-mounted coverage

The new browser fixture mounts `LayoutComponentInspector` with the production action dispatcher and a Runtime test adapter that captures submitted Events. It proves that after selecting component A, selecting B, and returning to A, a retained A handler submits no edit; the current A handler submits the existing MoveParts edit; and clearing selection retires the mounted route so its retained callback submits no edit. A second test opens the real Board outline section and verifies Enter commits an explicit margin through ReplaceDocument and Escape restores the accepted field value without a second edit.

## Red/green evidence

The disposable old-policy mutant restored the prior `owner.context_generation != layout_owner.generation` predicate. Its mounted A→B→A test failed for the expected reason: the stale handler emitted a `MoveParts` edit for A at X=42. The retained output is `old-owner-policy-red.log` (SHA-256 `e588f90cfda62080b01b7ce525d3bcd34bffdf4b48f43e9bbb6972b3b277adb9`). The functional patch passed all five focused Chrome/WASM tests in `mounted-inspector-green.log` (SHA-256 `f510638efdd5841cc6ed5e255af18767a0a3fbc61c3547b1db2164b8f6196465`). After that green run, strict Clippy identified two lint-only edits: removing redundant copies of `Signal` bindings and dereferencing the Copy `EventHandler`. Strict Clippy passed on the final source; the browser suite was not rerun after those behavior-preserving edits.

Earlier harness attempts failed before they were valid product checks: the mounted host did not read its rerender signal, test roots shared an ID, and the margin fixture reused a Locked component inside a collapsed details section. The fixture now subscribes to rerenders, uses unique roots, expands the section, and uses an unlocked part. Those harness failures are not counted as the regression's expected-red result.

## Verification

- Focused mounted Chrome/WASM suite: 5 passed, 0 failed.
- Strict affected WASM all-target Clippy with `page,core-worker` and `-D warnings`: passed.
- `cargo fmt --manifest-path web/Cargo.toml -- --check` and `git diff --check`: passed.
- Public paired edit/Undo/Redo/save/reopen acceptance and F3.5/F3.7 parent joins remain open.
