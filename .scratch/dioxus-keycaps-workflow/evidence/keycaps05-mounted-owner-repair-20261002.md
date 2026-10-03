# Keycaps finding navigation owner repair

This bounded source packet repairs the held mounted-owner finding from the exact 5fd source review. It does not close issue05, the Keycaps browser journey, issue07 marker rendering, the original missing fixture identity, or any parent joins.

## Frozen source

- Worktree: `/home/chris/.local/share/boardstudio/worktrees/keycaps-navigation-mounted-owner-20261002`
- Branch: `codex/keycaps-navigation-mounted-owner-20261002`
- Parent: `5fd4bb767fc558ed4ac90c6c9c39b821c8669129`
- Source commit: `646151d59113fb1aed2f4f7434904bc81e3699cf`
- `web/src/presentation.rs` SHA-256: `8c4ebf1c7fe4b912453ade80d7cf49ed7c60732b59a341f3628a7de076c99b78`
- `web/src/presentation/keycaps_navigation.rs` SHA-256: `795081ca5efa24c542fd7450b628bfe1321b0654c7fe280c7bbbbdd3821d119c`

The source remains private to the Dioxus presentation layer. It adds no public API and does not alter Session/Core ownership.

## Change

The production Editor now captures a typed navigation owner with the intended destination, workspace, runtime scope/generation, and accepted snapshot token/revision. The post-switch Layout fit checks that owner against the current selected tree context before it emits `SetCamera`; the check is repeated immediately before applying the fit. This catches selection replacement within the same board and generation.

The delayed Inspector focus callback captures the same owner and an Editor lifetime flag. Its first operation checks that the Editor is still mounted, before the callback reads any scoped Dioxus Signal. It then verifies that workspace, scope/generation, accepted snapshot identity, and destination selection still match. This protects a new Inspector from a stale animation-frame callback after same-scope selection replacement or unmount.

The production route keeps generated Case mechanical layers on their existing no-pin early route, and ordinary Part/Matrix/Body navigation still pins the desktop Inspector. Live Case owner state retains both independently selected body and mechanical-layer destinations because those selections are separate presentation state.

## Verification

The exact frozen source passed:

- `wasm-pack test --headless --chrome . --no-default-features --features page --bin boardstudio-web -- keycaps_navigation::tests` — 7 passed, 0 failed.
- `cargo check --target wasm32-unknown-unknown --no-default-features --features page --all-targets` — passed.
- `cargo clippy --target wasm32-unknown-unknown --no-default-features --features page --all-targets -- -D warnings` — passed.
- `cargo fmt -- --check` and `git diff --check` — passed.

The mounted cases cover: current Layout companion fit from destination geometry; same-scope selected-context replacement before fit suppressing camera effects; same-scope replacement between fit and animation-frame suppressing focus; a current owner receiving focus; and a queued frame after Editor unmount stopping before Signal access. Route tests retain generated-layer behavior and desktop pin behavior.

A controlled expected-red mutation removed only the destination-membership comparison from `navigation_owner_is_current`, simulating the prior scope/revision-only admission. The mounted `mounted_layout_fit_drops_when_selection_changes_without_scope_change` test then failed at the assertion that camera effects remain empty. After restoring the comparison, the same filtered browser test passed 1/1. The mutation was never committed.

## Review and remaining gates

The pre-repair HOLD is recorded in `/home/chris/.local/share/boardstudio/reviews/keycaps05-navigation-owner-review-5fd4bb76-sol-20261002.md` (SHA-256 `df68028373e98c112c5562a5c7d1bf008f92252e388131845aee6cedf9ce2788`). Sol has been asked to review the frozen `646151d5` source against that HOLD. No clearance is claimed until the new report arrives.

This packet tests the production-used owner/fit/focus functions through a mounted Dioxus lifecycle probe. It is not the paired React/Dioxus public browser journey and does not close source-to-root integration. The focused SVG finding marker remains the separate reviewed issue07 child, whose planning packet was cleared at docs commit `1ff36a50aa4f407eff7a0431d128300794d8ee4d`.
