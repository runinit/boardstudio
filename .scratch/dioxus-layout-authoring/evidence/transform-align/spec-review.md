# Transform / Align planning — independent Spec review

Reviewed in `frontend-layout-transform-align-20261002`, React pin `5a472a9426e6e38993361da402cd4ec730feb369`:
- Spec `1f06861e8b0736b3bfc14e93d5615a43263939f61776c813856676fcd5abc00c`
- 07 `af96eb5ac4a82a71d01543b24a67e2159e3cf6de0a0b2297f9f478a45ff5bf39`
- 08 `e624b7ccc6a512b50556d0e5b27d50a476d516fac2bd983a67a549fdd53e8547`
- 09 `0f57153d6538644c116ee6f334ff7bd6cacba665e768c7508eaab07df982e694`
- Inventory `21de46db19b1a84d901eed7132d4266a3c06c0a6a5bdf11134883ad73c87b5a5`
- Graph `059a20e714f6f047fb2ce0972fea764494bf2c5b0919193882d3c42bbbdd049d`
- RF handoff `a5909b33912772eeb9c1876e324915ff85c9bbf13dd66f7910b166ead349802e`

**07: hold for an exact preview-lifetime capability.** SetMatrix/SetMatrixSplay and normal Event::Edit prove mutation authority, but not the promised cancellable pointer-preview lifecycle. Session queues normal Edit with no gesture generation; its Preview reply publishes whenever gesture_generation is None (`session.rs:551–554, 1090–1109, 1203–1217`). GestureCancel invalidates only registered gesture work (`1624–1656`), whose current positions schema does not carry these matrix operations. Name/review the private root preview/restore/late-result protocol before tool dispatch, or explicitly gate the pointer-tool portion on that narrow capability. It must prevent late cancelled previews or queued stale matrix commands from becoming current. This is not a whole-parent blocker or authorization for new Session/public APIs. Numeric field capability can be treated separately.

**08: hold for exact linked-reference membership wording.** React `Workbench.tsx:910–915` excludes the entire partner matrix’s partIds whenever linkedSelection is true, not merely those individual counterparts a given key/column alignment will move. Preserve the predicate’s component/assembly exception as well. Current wording “linked counterparts that move with it” can produce a different eligible reference list. Pin the source predicate and test single-key/column and attached-component contexts.

**09: Spec-clear as a conditional composition join**, once genuine leaves satisfy its explicit start gate. No placeholder controls or duplicate state are proposed.

The private alignment-envelope seam is otherwise appropriate: accepted switch keycap override/definition fallback or courtyard, transformed corners and axis bounds, existing SetMatrix/MoveParts authority, no new geometry/API owner. Row lockout, fixed reference, lock/constraint checks and no snapping match source. F3.3a standalone-only Snap and RF-005 matrix-local free-drag limits remain open; structural Inspector and F3.5 Relationships ownership remain separate. Parent F3.1/F3.2 joins are preserved. Browser inventory correctly limits menu observations and unknown candidate build provenance.

No implementation, Cargo or browser checks performed by this review; no source/central graph/ledger edits.

## Amendment review

Verified spec `b5582c2e4a3b829284f2de6337dd2dfc6bc82daa9a945064c849f2180ee7c1b5`, issue07 `5e736b7239d0ac7f47b723c8bd3b9e47e3ed0d084e8e207e6779bd7bc1416bd9`, issue08 `4ac09ee0bc45ed93b07b05efd763bf2b2ac308c237f13f565db35a53a79bffb1`, unchanged issue09 `0f57153d6538644c116ee6f334ff7bd6cacba665e768c7508eaab07df982e694`, inventory `f291d06703943fde7db0792bb9ff74d0952f0ba460f6504d5f1c86eca50f6895`, graph `285a35f1c01b3dbd421cadc6b37ee4a5472a4c3792bacd13692c05860619bc2f`, RF `c47646ad926a1bbf327cc7ad52ffe338dcac5b2aade316a6a42d4ad11c422d94`.

Prior findings closed: issue07 is now fields-only and explicitly retains the pointer-preview capability gap; issue08 uses the exact full partner-matrix exclusion and component exception. Issue07 is Spec-clear, as is the conditional issue09 composition scope.

One final issue08 precision correction remains: its new reference-picker draft is “keyed by full Scope and selected target,” while React retains `alignTargetId` across moving-selection changes if it remains eligible (`Workbench.tsx:913–914`). Separate root-owned retained reference preference from target-qualified command admission/feedback; moving-target changes must not reset an otherwise eligible reference. Exact submitted alignment still settles only its captured target, with fresh-click retry and no automatic replay. This preserves both the original retained-picker behavior and the new operation protocol.

## Final disposition — Spec-clear

Issue08 final SHA `eb2586d81883c19d7576af8d994eabeda309045d6eb5c2f6d5537ec1149b26ea` explicitly preserves eligible reference preference across moving-selection changes and separately keys immediate command outcomes by Scope/target/reference/command. The final finding is closed. The complete amended packet at the hashes above (with this final issue08 hash replacing 4ac09ee0) is Spec-clear for publication/planning. Fields-only issue07, one-shot issue08 and conditional composition issue09 have bounded actionable scopes; deferred pointer tools, full F3.3/F3.3a/F3.5/F3.7 joins and RF-005 remain open. This is no implementation, compiler or public acceptance claim.
