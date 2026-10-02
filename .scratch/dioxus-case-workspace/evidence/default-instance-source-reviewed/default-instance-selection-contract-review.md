# Independent Standards/design: default-instance selection repair

Reviewed draft SHA-256 `42dca71cce5f8f8bf40cc1e348bc228a8627db93cfec517ba1522882faf06bfc` (blob `59dd70d0`), proposal SHA-256 `3427e351ab3617a43a6d88bae11ba5cf5e4a8dbe24b4f4b5c7f33ae95cf0e99f`, and diagnosis SHA-256 `0d53a1808fae41e24a3af89e31c02f4fc6f9fe2958fabdaf73c7c0a2c43a2d7b`. Integration HEAD `49b792eb11d94b8535f00fac46005f04c1e12c1a`; evidenced candidate `dc81237c`, reference `5a472a94`.

**No material design finding; bounded private repair is author-ready.** The proposal addresses the demonstrated upstream scope discrepancy without changing the authored/generated predicate or inventing a physical projection.

Actual React `main.tsx:36–37` resolves explicit matching instance, then first matching instance in document order. Board changes preserve raw preference; project open resets it. The proposed epoch/document-scoped preference retains temporarily absent explicit IDs for board return/Undo and never records automatic fallback as explicit. It remains UI preference; existing Session Scope stays authoritative. Invalid explicit IDs are rejected, and clicking an already effective fallback still records an explicit preference.

Existing `Event::Navigate` validates board/instance membership and changes view scope without editing document/revision/history. Reusing its private guarded caller retains gesture/context/anchor cleanup. Automatic reconciliation adds Ready/Saved-current/no-preview/no-gesture admission and fresh Scope/token/revision/generation/preference checks; stale scheduled work cannot override newer explicit choice. Hooks remain stable, no second subscription is proposed, and deferred automatic selection must preserve active gesture DOM/capture.

Neutral pending rendering plus live callback guards prevent wrong-scope Case actions before normalization. Removing the candidate-only Canonical option when instances exist is an explicit source-backed parity correction, including configured-board routing. Boards without instances retain None. Canonical Layout/Keymap/Keycaps geometry ownership remains unchanged.

Minor citation correction: reset source is `app/src/useProjectSession.ts:72`, not `app/src/ui/useProjectSession.ts`. This does not change the validated policy.

Required exact-source reviews and production verification remain: original import→Case path without manual normalization, ordering/explicit preference/Undo/reopen, busy deferral and stale effects, camera/history neutrality, pointer cleanup and delayed feedback. Diagnostic normalized green is not repaired-build acceptance. Full F7.7 starts/acceptance joins and F5.6/F7.3/F7.4/F7.6 gates stay open.

RF-006/009 evidence belongs in the existing coordinator handoff. No source edits/Cargo/browser execution; contract review only.
