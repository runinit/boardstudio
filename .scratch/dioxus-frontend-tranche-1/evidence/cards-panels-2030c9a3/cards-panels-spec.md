# Spec review — 2030c9a3 against cb8203fc

Reviewed `git diff cb8203fc...2030c9a3` and its commit list, tickets 02/07, shared ACCEPTANCE/AUTHORITY, corrected contracts, author handoffs, and actual React source at `5a472a9426e6e38993361da402cd4ec730feb369`. Review only; no builds/browser/tests or source edits.

## T1-07: two new source findings

1. **P2 — Compact state disables desktop idle hiding.** Ticket: “Honor the reference idle-hide behavior, including pointer hover and focus within the panel/rail.” `web/src/presentation/panels.rs:428–430` requires `!compact_open()` even after `compact()` becomes false. Open either compact panel, enlarge past 760px, choose Auto-hide, then leave pointer/focus outside: the retained root compact-open flag prevents every 280ms timeout from hiding it. Reference `WorkspacePanel.tsx:64–70` does not use compact-open state for this decision. Remove that desktop dependency and characterize this transition before/after repair.

2. **P2 — Responsive CSS overrides the new mode/width grid.** Ticket: “Implement both panels with the reference default pinned layout and pin/auto-hide/collapse controls”; corrected contract: “autohide/collapsed consume 32px rail space.” Later rules at `web/assets/m1.css:244,287` override the new grid with fixed 220/300px tracks at 761–1050px widths or desktop heights ≤520px. Collapsed panels retain large blank columns; persisted widths and absent-Inspector zero-width disposition are ignored. Make these media rules preserve the computed panel tracks. This affects current desktop acceptance, independently of ticket 09’s future 980/820 drawers.

Editor-owned settings survive conditional Inspector unmount; independent preference decoding, optional-storage fallback, transient ownership, media/listener/timer cleanup and local-only panel actions otherwise match the corrected source contract. Source review does not establish browser camera neutrality or accessibility parity.

## T1-02: approved partial implementation; full ticket remains open

No additional source defect found in the cleared card/list UI and parent composition. Two previously recorded blockers remain: “One preview failure must not hide healthy records” is unmet for malformed stored records because BrowserStore atomically decodes the list; “Repeated or superseding opens … cannot overwrite newer project/list state” remains violated by the reproduced post-submit open race. Neither provider nor Session was repaired here; both joins retain their approval/verification requirements.

No scope creep found. Shared acceptance’s integrated build, reference/candidate browser, keyboard/focus/axe and applicable assistive-technology evidence remain required; pending evidence is not a source failure. Both author handoffs explicitly record no new refactoring takeaway. No new refactoring takeaway observed.
