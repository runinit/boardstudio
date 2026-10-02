# Compact Case focus reveal: independent Spec review

**Source clear for focus-entry repair; resize and public green remain open.** Reviewed integration `presentation.rs` SHA-256 `63d64acbc45df116ccde77f53e837ea12527d57e3f12bb0749706f0f169f506f` against HEAD `a93eb5237a1d8240c468a0b073f33f971bf4e75c`. I did not author the handler. Read the retained public red and handoff in `case-focus-occlusion/DIAGNOSIS-AND-HANDOFF.md`; no browser replay performed here.

The single workspace `onfocusin` uses the live workspace signal and fresh max-width760 media query. Only compact Case focus entry closes the existing Objects/Inspect open signals, and only when needed. It does not prevent the focus event, explicitly move focus, change component keys or invoke domain/camera actions. Thus the actual Tab→covered Generate trigger is addressed by revealing the focused workspace control, without introducing a focus trap or renderer remount.

Panel contents and visibility buttons are siblings outside this workspace section, so their own focus does not immediately close reopened panels. Existing toggle, selected-object handoff, closed/inert and desktop mode behavior remain owned by their current components. Forward/reverse keyboard traversal should remain possible; that is a source conclusion awaiting actual event verification.

**Explicit limit:** a desktop→compact resize while focus already remains on a workspace control emits no focusin. Existing panel media listeners update local compact state, not the parent open signals, so remembered drawers can still cover that focused control. Similarly, opening a panel without a new workspace focus event is outside this handler. This is not clearance of all focus-visibility/resize behavior; retain the fresh public resize/reopen control and repair any reproduced failure separately.

`git diff --check` passed. Required post-build evidence: actual Tab/Shift+Tab with one/both drawers at390×640 and390×844, retained child focus after rerender, unobscured hit target, normal activation/reopening, unchanged viewer instance, desktop/other-workspace controls and the resize edge. No source edits, Cargo or implementation browser-green claim.
