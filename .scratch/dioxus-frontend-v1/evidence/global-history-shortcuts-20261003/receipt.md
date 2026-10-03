# F2.4 global history shortcut receipt

**Scope:** Existing F2.4 workbench shortcut criterion 26, implemented on private base `54f627b1`.

**Pinned oracle:** React source `5a472a9426e6e38993361da402cd4ec730feb369`, `app/src/ui/Workbench.tsx` global `keydown` effect and `app/src/ui/workbenchGeometry.ts` `isTyping` helper.

**Source RED:** React handles Ctrl/Cmd+Z, Ctrl/Cmd+Shift+Z, and Ctrl/Cmd+Y globally, except when the target is an input, textarea, or select. Dioxus handled only the Z chord in the focused canvas handler, so shell controls had no history shortcut and Ctrl/Cmd+Y was unhandled.

**Change:** Added a private workbench shortcut handler mounted on the shared `m1-workbench` root. It dispatches existing `Runtime` Undo/Redo events for all three chords and mirrors the reference typing-target suppression. The existing canvas handler remains the owner when it already prevented a keyboard event; the root handler checks the native event’s prevented state to avoid duplicate submissions.

**Changed files:** `web/src/presentation/workbench_shortcuts.rs`, its module registration and root event seam in `web/src/presentation.rs`, the existing criterion 26 in `.scratch/dioxus-frontend-v1/issues/02-projects-shared-ui.md`, and this receipt.

**Architecture:** No new RF finding. The shortcut module dispatches through the existing Runtime/Session history owner and introduces no second state authority.

**Checks and limits:** Ran Rust formatting for the new module and `git diff --check`. No Cargo test/build, browser journey, broader matrix or independent review was run, per the 2026-10-03 integrate-more/test-later direction. Root owns the combined candidate check and changed paired journey.
