# Spec review: `4ba411e7`

The prior P2 finding is **resolved**. On `Completion::ExportFailed`, the session now removes the matching operation from `self.exports` before settling it (`application/src/session.rs:886-897` in this candidate). The added regression test verifies that a failed export is no longer current and that reopening does not emit a redundant `CancelExport` (`application/tests/durable_session.rs:1017-1059`). This matches the existing archive/STEP snapshot identity and one-terminal-outcome contracts.

I found no new actionable missing or partial source requirement, scope expansion, or incorrect behavior in the `application`/`web`/`scripts` delta from `97a779a6` to `4ba411e7`. The delta only changes export failure registration and adds its focused regression test; remaining changes are evidence/docs.

Release/offline acceptance, full browser storage exchange, renderer lifecycle, keyboard/focus and screen-reader/axe, performance/resource comparisons, and independent exact-candidate gates remain open as documented. They are incomplete acceptance evidence, not additional source findings.
