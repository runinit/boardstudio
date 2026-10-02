## Standards: desktop panel timer correction

Reviewed `git diff a3a0224f...47cf655b`, commit `47cf655b`, plus `/tmp/frontend-run/panel-timer-red.json` and `panel-timer-control.json`. Source/evidence review only; no application edits, builds or browser execution. `git diff --check a3a0224f...47cf655b` passed. Root reports formatting and strict WASM Clippy passed; candidate build/public green remain pending.

**No new documented-standard violation or material Fowler smell found. Approve this bounded source repair.**

The change removes only `compact_open` from the three timer call sites, helper parameters and timer-expiry guard in `web/src/presentation/panels.rs:410–436`. Compact-open preference remains owned and used by the compact presentation. Desktop hiding now depends on current compact viewport state, current pinned mode, current pointer hover, and actual panel/rail focus containment. The 280ms timeout, replacement cancellation, unmount cleanup and compact-entry timer cancellation are preserved. Thus an old compact-open flag no longer suppresses desktop behavior, while a timer crossing back into compact still cannot hide its content.

The supplied public red shows compact opening at 720px, returning to desktop at 1280px, selecting auto-hide, then focus outside at `m1-tab-Layout`; after idle, reveal remains true and content remains non-inert (`pass:false`). The desktop-only control at the same URL hides under the corresponding final conditions (`pass:true`). Those paired observations support the removed flag as the bounded cause; this reviewer did not independently reproduce either trace or inspect the separately reported second red script.

The correction conforms to CONSTRAINTS.md UI behavior/responsive preservation and ticket 07's idle-hide policy. No Runtime/Session, document/history/camera, preference serialization, public API or browser-listener ownership change is introduced. Source supports the intended fix; it is not a substitute for a public green run against the new artifact.

T1-07 remains open for candidate public regression confirmation and all remaining shared acceptance gates, including applicable keyboard/focus, axe, assistive-technology and lifecycle evidence. No new refactoring takeaway observed in this narrow correction.
