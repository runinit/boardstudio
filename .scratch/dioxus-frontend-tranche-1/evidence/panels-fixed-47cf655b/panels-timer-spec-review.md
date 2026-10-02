# Independent Spec review: panel idle-timer repair

**Source pass for `a3a0224f...47cf655b`; no new Spec findings.** Reviewed the exact one-file diff (3 insertions, 5 deletions), the resulting timer/containment functions, retained `panel-timer-red.json` and `panel-timer-control.json`, and the independent public-control regression script. No source edits, tests, builds or browser execution by this reviewer.

T1-07 requires: “Honor the reference idle-hide behavior, including pointer hover and focus within the panel/rail.” The recorded pre-fix comparison isolates the violation: after compact opening and return to desktop, Auto-hide remains revealed with outside focus; the desktop-only control hides with the same mode and outside focus. The trace also confirms visible content is no longer incorrectly inert, so the earlier inert defect does not mask this reproduction.

The repair removes only the irrelevant compact-open argument and condition from the desktop timer and its three call sites. The 280ms delay, cancellation of replaced timers, current compact-viewport checks, pinned-mode checks, current hover state, and actual activeElement containment in both panel content and rail remain intact. Consequently the timer can hide an idle desktop panel regardless of its earlier compact-open preference, while focused/hovered panels remain protected. Compact visibility still follows the root compact-open signal elsewhere; this patch does not change compact opening, stored modes/widths, menu/focus handling, cleanup, child lifetime or document/camera ownership. Both panel sides share the corrected helper.

The coordinator reports strict WASM Clippy and formatting passes and a fresh build in progress. The source defect is addressed, but the required public green against this exact built revision is pending. Do not mark full T1-07 accepted: retain integrated panel/theme/viewport, reload/workspace/storage, keyboard/focus/hidden-content, cleanup, history/camera-neutrality and shared acceptance gates. Ticket 08 resize and ticket 09 final drawer thresholds remain separate; T1-02’s existing blockers are unaffected.

No new refactoring takeaway observed.
