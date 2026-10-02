# Compact Case workspace focus — Standards review

Reviewed the sole new workspace-section `onfocusin` handler in integration `web/src/presentation.rs`, SHA-256 `63d64acbc45df116ccde77f53e837ea12527d57e3f12bb0749706f0f169f506f`.

**Clear; no material Standards finding.** The synchronous handler reads the current workspace Signal and current matchMedia result at event time, closes only existing compact-panel visibility Signals when Case is active at widths≤760px, and avoids redundant writes. Objects and Inspector are sibling sections, so their own focus events do not invoke this workspace handler. Focus remains on the actual workspace control; no imperative focus movement, new component key, hook, timer, listener registration or remount is introduced.

The media query matches the compact CSS boundary. Desktop and other workspaces do not mutate panel visibility. This is presentation-only state: it neither captures a document/selection nor submits a Session operation, so additional domain Scope/token guards are unnecessary. Existing panel `inert`/aria-hidden/display rules consume the same visibility owner. No API/member widening or changes to renderer ownership.

This source fix addresses focus entering workspace controls beneath compact panel overlays. Root must still verify visible focus and actual control reachability in the built page for closed/one-open/both-open panels, Tab/Shift+Tab and desktop/other-workspace controls. A source review or temporary diagnostic probe does not prove public accessibility acceptance. No Cargo/source edits/browser actions performed. No new refactoring takeaway observed.
