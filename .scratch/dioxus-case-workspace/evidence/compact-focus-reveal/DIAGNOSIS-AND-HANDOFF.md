# Compact Case focus reveal

## Actual public red

Candidate b6d2af49 at http://127.0.0.1:34679/, fresh task session `case-focus-diagnosis`, disk profile `/var/tmp/frontend-run/case-focus-diagnosis-profile`. Publicly imported the same configured revision3 REVIUNG41 archive SHA28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324, selected Case, opened Objects while Inspect was already open, viewport390×640. No storage writes or synthetic events were used.

`python3 /tmp/frontend-run/case-focus-occlusion/tab-regression.py case-focus-diagnosis /tmp/frontend-run/case-focus-occlusion/both-open-red.json` performed real CLI `press Tab` steps. From the Objects toggle, Tab13 focused Generate case. Its rect was x154.0625,y99,width98.125,height44. `document.elementFromPoint` at its center returned Inspector's `.m1-case-bodies-generated`; both panels were expanded. The script exited1 with `Real Tab focused a Case control covered by an open drawer`. Screenshot `both-open-focused-generate.png` and full trace retained. Independent frontend_ui_verifier also reproduced last Objects tree button → real Tab → covered Generate in its own profile.

## Cause and approved behavior

The compact CSS correctly prevents the two panels from covering each other, but the workspace remains underneath them and its controls remain tabbable. Existing panels.rs only applies inert to compact-closed panel content; it does not govern the underlying workspace. This is actual keyboard-focus occlusion, beyond a tabIndex or hit-test inventory alone.

Root approved the minimal focus-reveal policy: when focus enters a compact Case workspace, close both transient drawer signals and retain focus on the entered control. Single-drawer visible geometry is preserved until focus enters it. Desktop modes/preferences, other workspaces, camera, selection, document, and renderer lifetime receive no new action.

## Exact source change

Only integration `web/src/presentation.rs`,14 lines at the workspace section's onfocusin callback. Each event checks the current workspace signal and fresh `(max-width:760px)` matchMedia result. It writes existing objects_open/inspect_open signals only when true. No explicit focus(), event cancellation, observer, state model, API/visibility widening, or remount key.

Before SHA-256 `0cc0f32121d9336d6d9d0528e708b9de41949e89af95c0e791c3a9b8c150525e`; after SHA-256 `63d64acbc45df116ccde77f53e837ea12527d57e3f12bb0749706f0f169f506f`. Exact patch `focus-reveal.patch` beside this report. Owned-file rustfmt (edition2024,skip_children=true) and git diff --check passed. No Cargo/build/commit or implementation-level browser green is claimed.

## Resize edge and pending gates

The handler intentionally responds to focus entry only. If the workspace already owns focus while a desktop-to-compact resize exposes remembered open drawers, media-query changes alone do not dispatch focusin. Existing panels.rs use_compact_viewport listens for media changes and updates its own compact signal but does not close parent drawer-open signals. Therefore this change alone must not be claimed to solve pre-existing focus during resize. A fresh public resize control must determine the observed post-build behavior and, if covered focus persists, root should choose a separately evidenced lifecycle repair rather than adding an untested observer here.

Independent source review and root compilation/build remain pending. On the fresh build, rerun actual Tab and Shift+Tab with one/both drawers open at390×640 and390×844; verify focus stays on the child, expanded states become false, the focused control is visibly unobscured, controls activate normally, geometry stays mounted, and footer/nav remain usable. Include desktop/other-workspace controls and the resize edge above. This bounded fix is not full compact-panel parity acceptance.

## Follow-up: resize edge now has actual red

During root's build freeze, the immutable old34679 page reproduced the lifecycle edge. In the same fresh task profile with both compact drawers open, changed viewport to1280×640. Real Shift+Tab then Tab focused Generate case. The desktop precondition read proves workspace focus and an unobscured hit test. Ran `python3 /tmp/frontend-run/case-focus-occlusion/resize-regression.py case-focus-diagnosis /tmp/frontend-run/case-focus-occlusion/resize-red.json`, which changed viewport to390×640 and read focus. Focus remained Generate but hit test returned Inspector `.m1-case-bodies-generated`, and both expanded states remained true. It exited1: `Resize leaves the retained workspace focus behind compact drawers`. Screenshot `resize-covered-generate.png` retained. This is an actual old-build red; the in-progress focus-entry build has not been tested yet.

Smallest proposed separate repair, not applied: reuse `web/src/presentation/panels.rs` existing compact-transition effect (lines112–121) and existing viewport media listener (350–382). When entering compact, inspect the active element's nearest workspace-content ancestor; if that ancestor contains a direct Case panel, close this panel instance's existing compact_open signal via set_bool. Both existing panel instances receive the media transition; no additional listener, observer, state, prop, public API, or visibility change is needed. A private DOM helper can express that guard. Preserve focus and desktop preferences. Current panels source SHA-25656014451de70c5528922c743bc149cd2554ceb5e04fb76602c903cb783188b8b.

Acceptance for this separate lifecycle correction: the same resize command must keep Generate focused, show both expanded=false, and produce an unobscured focused control after media effects settle. Repeat with one drawer and both, keyboard and focused viewer, plus a no-workspace-focus control that preserves the user's open panel. Do not close drawers just because a viewport becomes compact when focus is elsewhere. No source changes were made during the build freeze.

## Approved resize lifecycle repair implemented

After freeze release, root authorized only panels.rs. The existing compact transition effect now calls private `case_workspace_has_focus()`: read current active element, resolve its nearest `.m1-workspace-content`, and require that workspace's direct `.m1-case-panel`. Only when compact and this guard succeeds does each existing panel instance set its transient compact_open false. There is no new listener/state/prop/API/visibility, focus() call, renderer remount, or document mutation. Focus elsewhere (including panel controls) leaves open drawer state unchanged. Existing use_drop media-listener cleanup is preserved.

Final panels.rs SHA-256 `aab64ce779f5980eb61e19f521fb9522bfb7c487538fd9542e417b710560fefb` (before56014451...). Exact17-line patch `resize-lifecycle.patch`. Owned-file rustfmt edition2024/skip_children and diff --check passed. No Cargo, build, or commit. Source released for independent review and root's final combined build. The real resize red and original focus-entry red remain preserved; fresh actual public green and no-workspace-focus controls remain pending.
