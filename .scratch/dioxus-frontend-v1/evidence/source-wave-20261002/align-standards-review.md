# Layout Align exact source Standards review — 2026-10-02

Disposition: BLOCKED for source integration, two actionable findings. Read-only review; no source edits.

Reviewed feature commit `533f9a5051c1d1cd62e1dccdf1233a8285c8945e` plus separately supplied shared join patch, SHA256 `0549911b823bb5a046a52c165746c89f6162454f68c5874543c59d51c631be22`. Issue08 SHA256 `eb2586d81883c19d7576af8d994eabeda309045d6eb5c2f6d5537ec1149b26ea`.

Exact leaf hashes:
- layout_align.rs `f273320466443af3237b759339a4c4dac92242a34d2d3c00f3f78124477adc8b`
- layout_align_controller.rs `dfcf8aad3be9724435e48b3c2a9981b8c7a076d7825f1ed08b7b269a5ae871a7`
- layout_align_geometry.rs `7863184ed48e081ffa89f1688bd632e4f88aa6630cea9c633d2fd6bdab93bc67`

## Findings

1. P1 — Empty reference list self-triggers the mounted effect. `layout_align_controller.rs:432–447`: `reconcile_reference` subscribes through `selected_reference.read()` then calls `set(None)` on every effect when a selected context exists but there is no eligible independent reference. Dioxus 0.7.10 tracks reads performed in effect bodies, including through use_reactive closures; writes notify subscribers without equality deduplication. A valid selection containing all independent parts therefore keeps the Editor effect running. Use nonreactive bookkeeping reads and write only when the computed reference changes. Require a mounted no-reference regression that fails on this exact source and settles after correction, rather than pure geometry tests.

2. P2 — Completed outcome from an old scope can permanently block new Align commands. `layout_align_controller.rs:880–915`: Completed is evaluated against the current snapshot before `!same_scope` retires pending. An old base revision10 followed by a new project/scope at revision0 makes the Completed branch return None indefinitely, so pending remains Some and every future click is ignored. No accepted snapshot also returns before scope retirement. Retire the terminal request for its captured old scope before testing new-scope accepted token/revision readiness. Require a delayed Completed plus lower-revision new-scope regression.

## Boundaries and limits

The reviewed source otherwise retains the root Editor hook lifetime, numeric version subscription, use_callback handler allocation, private presentation interfaces, existing SetMatrix/MoveParts authority, pre-submit exact OutcomeSlot registration and immutable accepted document input. The shared join mounts the actual Align toolbar. No new allowance or existing-member visibility widening observed. Native geometry checks and WASM compilation do not execute these effect/outcome lifecycles; previously reported checks are not acceptance evidence. Public paired journey, save/reopen, Undo/Redo and precise changed-ID proof remain open.

RF handoff: reuse the existing effect/subscription and root-outcome lifecycle takeaways (RF-001; existing exact-operation ownership ledger entry) plus preserve RF-005 local/world geometry boundary. Do not invent a new domain authority. Both findings sent to root and Align author.

## Corrected source80ad6603 re-review

Exact correction `80ad660399652e6c6353bcf6ee8f9dd9c1e1ebf8`, shared join patch still `0549911b823bb5a046a52c165746c89f6162454f68c5874543c59d51c631be22`.

The production control-flow changes resolve the two observed defects: reconcile_reference now peeks and writes only a changed choice; unavailable projection preserves the chosen preference; terminal old-scope retirement occurs before accepted snapshot advancement checks. Numeric version subscription/root hook/use_callback remain intact.

P2 verification blocker remains: `web/tests/layout_align_effect.rs::AlignReferenceEffectProbe` reconstructs its own effect and changed-only signal write. It imports only geometry helpers and never invokes production reconcile_reference/use_layout_align. Restoring the production unconditional None write would leave this regression green. Likewise the old-scope test checks a helper's boolean result without exercising actual pending retirement ordering. The required red→green lifecycle regression must traverse the production controller/hook or an extracted private production reconciliation/settlement seam; no visibility widening or mocked duplicate transition. Source author and root notified. Existing source fixes are acknowledged, but exact source-integration clearance is held until a meaningful production regression guards them. Public browser acceptance remains open regardless.

## Final re-review — source71075e71

**Standards source-integration CLEAR** for exact `71075e71040728c19eab0d9aa0bedd487e3e133c` (after533f9a50/80ad6603) plus unchanged shared join patch SHA256 `0549911b823bb5a046a52c165746c89f6162454f68c5874543c59d51c631be22`. All earlier actionable findings are resolved at this exact source.

The actual hook/controller now calls `reconcile_reference_choice` for its reference reconciliation, including the changed-only write callback; the mounted test invokes this same private production transition. `settle_pending` calls the actual `pending_settlement_gate`, which chooses old-scope retirement before accepted-revision waiting. The harness now exercises that gate rather than a duplicate decision. Recorded controlled regressions mutate those shared production seams: empty-reference effect runs30 times versus expected1; old-scope Completed returns WaitForAcceptedAdvance instead of RetireOldScope. Logs are retained in the author evidence directory as reconcile-red.log/settlement-red.log.

Independently ran `cargo test --locked --manifest-path web/Cargo.toml --no-default-features --features page --test layout_align_effect`:10 pass, including mounted effect/lifecycle and production settlement checks. Log `/tmp/layout-align-production-seams-green-20261002.log`. Git diff check clean; no new lint allowance/expect or public/member visibility widening observed. Author's page/native and strict WASM checks remain available and must be re-executed after serial integration as usual.

Exact SHA256:
- layout_align.rs `759aac9330d3e53ac2f2b765f59a39edaa1407c865a84463b3925bf9c54cb1ec`
- layout_align_controller.rs `dc533c10c62da3c89afc76a8934efd5cac6a9d47afcd0310d1c5da314215ef80`
- layout_align_geometry.rs `0ba7df748fdec23b6cac37c29ead5ff580c750c45b039230ac1897148aafe0cb`
- layout_align_effect.rs `59a3c542469767249ab7c8581e1c59a1044f30dfa9ca72253c25cef8b0ab1dcf`

This closes the source review findings, including meaningful regression coverage; it does not close the child/parent or certify paired public geometry/history/save/reopen behavior. RF handoff retains the prior effect/operation ownership and RF-005 local/world boundary entries. No new refactoring identity proposed.
