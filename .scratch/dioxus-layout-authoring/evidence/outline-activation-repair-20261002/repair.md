# Outline activation observed-owner repair — 2026-10-02

Required follow-up to cc74310a, starting after bridge facade correction73e112ae. This extends the current T1-11/F3.4 lifecycle contract; it is not a scope waiver or parent closure.

The old root SelectOutline dispatch was extracted unchanged into a private adapter and exercised with the actual mounted lifecycle owner. Three expected regression failures were reproduced: activation during recovery instead of Ready/Saved; stale rendered token admitted after callback refresh; no exact observed slot surviving Editor unmount. The red log records11 prior positive tests plus these3 failures. Extraction changed only composition, not admission or settlement behavior.

Final composition removes the direct root Edit path. Both mouse and keyboard Objects rows construct an immutable Activate payload using the actual accepted snapshot, full scope and selection generation. The refreshed root callback validates that payload before selection side effects and routes it through the same Editor-owned lifecycle callback as Copy/Delete. The controller rechecks full identity, Ready/Saved, preview/gesture exclusion, actual selected version and accepted version membership; already active versions add no history. Exact operation observation occurs before submit and pending/failure/saved feedback and detached retention reuse the existing owner. Case and non-outline selection requests explicitly carry no activation payload. All added contracts remain private; no existing member visibility is widened.

Final tests execute the actual payload factory, source admission, controller hook and Inspector in a mounted VirtualDom, with deterministic Runtime/outcome/async ports.17 tests pass, including stale generation, hidden exact Generated success, target membership/selection consistency, no-op and pending deduplication. Full native12+36+6+17+1 passes. Focused native strict Clippy and strict WASM page all-target Clippy pass, as do formatting/diff checks. The strict WASM pass includes the bridge author's disjoint uncommitted needless-borrow/native-harness fixes; those require the author's separate follow-up commit when integrating. No browser package was rebuilt here.

The root event-handler join and actual Objects click/key construction were source inspected and compiled; the mounted test does not claim the entire Editor tree/browser composition. Independent source reack and exact packaged public paired activation/history/reopen remain required. Prior880 packaged/browser evidence is historical, not proof for this patch. Bridge occurrence projection is a separately authored feature with separate review.

RF proposal: carry forward RF-006 captured owner/terminal lifetime and RF-009 exact evidence provenance; no shared-ledger edits.

Exact source SHA256:
- `web/src/presentation.rs`: `7722ec84ccedf12c0a6d203a52ea6a0c1de6e05acf1a0f7530018691807d41c5`
- `web/src/presentation/objects.rs`: `1c84b2eae0fe9d86edf82a82f77eac7262444a20dca28fea17961e75a2fc8053`
- `web/src/presentation/case_workspace.rs`: `771b8123754f6eb114f311571394a1bb9c6f5d5c708c461cd4b7254692c77dc6`
- `web/src/presentation/outline_lifecycle.rs`: `707c212a9a5a776b9c5f310748d7c092f879ec74c11d1b85b796352bc33b42b4`
- `web/src/presentation/outline_lifecycle_tests.rs`: `b443373f31c9b917a79a4c5c9106cac5fa5bb96c657a1dba3b0cd524e6f1b896`
