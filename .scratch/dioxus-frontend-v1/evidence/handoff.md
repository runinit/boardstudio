Current follow-up: [F3a Layout layers, keycaps and footprint view](layout-layers/handoff.md), source `f44a3d1b`, demo http://127.0.0.1:34643/. The F1 evidence below remains historical.

# F1 frontend shell handoff

**Implemented and verified first increment; full frontend v1 remains open.**

Executable source: `242184509c1ec2d97ab0754dd29faa95edf0b342`.
Branch: `codex/rust-v1-ui-parity-20261001`.
Demo: http://127.0.0.1:34233/ and http://127.0.0.1:34233/boardstudio/.
Choose **REVIUNG41 copy** or **Sofle v2 copy** in a fresh browser.

Immutable build: `web/target/builds/frontend-f1-24218450-20261001`.
[Provenance](build-final/provenance.json) verifies 925 source inputs and all root/subpath
assets. React reference `5a472a9426e6e38993361da402cd4ec730feb369` remains available
at http://127.0.0.1:46881/. Original M1 source/demo are preserved.

## Delivered

Dioxus presents the Objects/canvas/Inspect workbench, exact six workflow labels
(Layout, PCB, Keymap, Keycaps, Case, Parts) and separate Export. Project fixtures,
saved projects and import remain available; choosing one closes the menu and
restores summary focus. Escape dismisses the menu. Light/Dark/System uses reference
tokens/fonts, persists preference and follows live system changes. Compact controls
expose Objects/Inspect; Undo/Redo and save status remain visible without overlap.
Committed edits/history survive workspace changes; unfinished previews cancel.

Existing Layout, Case generation, archive and STEP exports remain connected.
PCB, Keymap, Keycaps and Parts show explicit temporary unavailable states with
a Back to Layout action. These placeholders do not complete their full ports.

## Validation

- Native web tests: **12 passed**, [log](checks/native-tests.log). Native code unchanged afterward.
- Formatting, WASM page compilation, strict all-target WASM Clippy and diff checks:
  passed, [logs](checks/review-fixes/). The only later executable delta is two CSS
  positioning declarations; Rust sources are identical.
- Repository check passed: [log](checks/repo.log).
- Fresh root/subpath page and offline-worker builds: **six commands passed**,
  [logs/provenance](build-final/). Unchanged providers were hash-verified and reused.
- [Public browser flow](browser-7d46cd9a/browser-steps.json): tabs/placeholders,
  Arrow/Home/End, live themes/reload, preview cancellation, committed edits/Undo,
  fixture menu dismissal. [Import/saved-project checks](browser-7d46cd9a/import-saved-menu.json)
  verify menu closure and summary focus.
- [Case](browser-7d46cd9a/case-exact.json): exact geometry ready with mounted canvas.
  [Exports](browser-7d46cd9a/exports.json): valid 1,397,533-byte archive and
  13,662,781-byte STEP with complete envelope. These checks ran on `7d46cd9a`;
  the only later change is compact footer CSS. This establishes UI wiring, not
  new CAD geometric-equivalence evidence.
- [Final compact trace](compact-history-final/browser-steps.json): actual numeric
  edit/Undo with open and closed panels; footer/status rectangles do not overlap
  and hit-testing reaches Undo.
- Final offline reopening passed at [root](compact-history-final/root-offline.json)
  and [subpath](subpath-final/subpath-offline.json), retaining scoped worker/theme.
  [Final browser errors](compact-history-final/browser-errors.json): none.
- Final axe: **zero violations** in desktop light/dark and compact light.
  [Raw checks/captures](visual-final/). SVG-label contrast remains an automated
  incomplete result; actual screen-reader verification is still open.
- Independent [Standards](standards-review-final.md) and [Spec](spec-review-final.md)
  reviews: no actionable source findings at the final executable tip.
- [Visual verdict](finish-verdict-final.md): **ship for all six tracked fixes**,
  not whole-application parity approval. Both fix rounds and initial red evidence remain.
- [Design documentation verification](design-documentation.md) preserved incumbent
  files. Generic fresh subagents used the skill reviewer/documenter workflows
  because this harness has no shipped-agent role loader.

Valid captures cover 1440×900 light/dark, 390×844 light/dark, expanded compact panels,
and 920×500 short desktop. All supplied images were opened. Detector ran once;
11px support text was corrected to 12px. SVG 2.8 is a geometry user-space size.

## Remaining frontend work

**F2 is next:** complete projects/library/onboarding, shared menus, resizable panels,
full compact drawers, preferences and focus. F3 ports the full Layout tree/canvas/
inspector, including grouping, controls and richer geometry presentation.
F4–F8 complete Parts, PCB, Keymap/Keycaps, Case/3D and Export. F9 qualifies the
whole frontend before React retirement. See the
[roadmap](../../../docs/migration/DIOXUS-FRONTEND-V1.md) and [task graph](../PLAN.md).

The fixed-width shell and collapsible compact panels are a bounded F1 adaptation.
Existing M1 canvas labels/rendering remain visibly different from React. Main and
Workbench remain partial in the inventory; no entire production TSX file is claimed
complete from the shell alone.

Actual screen-reader testing remains blocked by missing host AT. Existing M1
acceptance, frozen-comparison and material/resource limits remain separately recorded.
No backend rewrite, production entrypoint/writer switch, React deletion, publication,
or API/schema change occurred.
