# Mechanical feedback extraction — independent Spec review

**Decision: source Spec clear for the bounded feedback extraction and production routing.** Reviewed integration working-tree sources; no source edits, Cargo or browser execution by this reviewer.

Exact reviewed SHA-256:
- `web/src/mechanical_feedback.rs`: `26ecb2bfc4a13f707984de9ef06753a305bb324df501229bd29ef0ec94275383` (supersedes c53b8fd5 only by production/test import cleanup).
- `web/src/presentation/mechanical_settings.rs`: `096ea22ec7582dc2b4130d756cf6f81a2a5c900eb3c85bc338727e47a36a67ae`.
- `web/src/presentation/mechanical_settings_mount.rs`: `5ac2db60f526943036318bb383a0c3339d81df3751bac4dcc0ac5749620b68d9`.
- `web/src/main.rs`: `bd4c9de87de5a57af298d6f1a69b5253ce49c4fa2b0c01474c02a3ff3c40eae2`.

## Production behavior

The actual page binary registers the same pure module for page WASM and native tests. UI feedback/identity types are re-exported from that module; there is no parallel test-only policy. The mount calls `field_feedback` for the numeric field list and `relevant_summary` solely for global status. Field delivery retains every exact current-owner record regardless of token advance, busy status or current field value. DimensionField then matches its own complete submitted identity/request/field and releases its submitted guard on Failed, preserving the draft. Pending and matching terminal handling precede accepted-value resets.

Summary relevance is separate: Pending wins while busy; otherwise the most recent terminal record must still match its affected accepted field basis. Unrelated accepted changes and token advances do not hide an actionable failure; Undo changing the affected field hides stale Saved text. Method relevance includes standard process methods/materials while ignoring unrelated custom processes and thickness-only changes. Full scope/editor/presentation generations still exclude obsolete owners. Publication retains immutable Rc configuration evidence, replaces the exact request row, and keeps the existing 12-record bound with active Pending pinned. No predicted document or additional authoritative state is introduced.

## Executed evidence inspected and limits

Read actual old-policy red log `/tmp/frontend-run/mechanical-feedback-native-red.log` (SHA `42aeb5554182dbcdf76ae402caa23706b8698c43579e64a6dd789ee028d3b4c6`): three assertion failures, covering failure across token advancement, raced-request terminal summary after A advances acceptance, and unaffected-field summary relevance. The Undo-summary test already passed under that red mutation; do not call it an independent red reproduction.

Read restored green `/tmp/frontend-run/mechanical-feedback-native-green.log` (SHA `15ea2ec506d1db5716f5ab84dc1d59e21c772e1c3d9292bae989c7ae8997f327`): 12 library +22 binary tests pass, including all four production-policy tests. The tests directly call the production functions and assert both busy field delivery and summary relevance. The retained red's busy field-delivery assertions themselves did not fail; it is not evidence of a complete old mounted B-field stall reproduction.

No remaining material Spec blocker in this extraction. Strict WASM rerun remains root-owned. Native pure-policy coverage does not prove Dioxus effect scheduling, real Runtime A-pending/B-rejected/A-saved/B-retry, mounted reconciliation-failure retention, no-op draft correction, or bounded-store pressure/lifetime behavior. Those public/integration checks remain acceptance work; neither these tests nor this review close the Case/INT.2 parent gates. No new refactoring finding observed.
