# T1-07 additional candidate checks

Exact candidate supplied by coordinator: `http://127.0.0.1:34651/`, build `frontend-panels-47cf655b-20261002`, source `47cf655b`. Sessions use isolated agent-browser contexts. Browser-run QA only; no repository files changed.

## Independent modes and timer protection

A fresh editor session selected Objects → Auto-hide and Inspector → Collapse via each panel's public Options menu. The settled public state was left `mode=autohide`, right `mode=collapsed`, both rail tracks 32px, Objects content revealed until idle, Inspector content hidden. Stored keys remained side-specific (`boardstudio:v2:panel:left` / `...:right`). Opening a collapsed Inspector rail returns it to pinned, as the current control defines.

I tested both panels independently for hover and focus protection with >450 ms idle (above the 280 ms reference delay):

- Objects hover-only: with focus on the Layout tab (outside panel), pointer over Objects for 450 ms left content revealed (`aria-hidden=false`); after moving pointer to canvas and waiting 450 ms it hid (`aria-hidden=true`, `data-revealed=false`).
- Objects focus-only: clicking its reveal rail left focus on the rail inside Objects. Pointer outside for 450 ms kept it revealed. Clicking Layout moved focus outside; another 450 ms outside hid it.
- Inspector hover-only and focus-only produced the same result using its public rail/options actions and panel region.

Final hidden-state probe: Objects and Inspector content `inert=true` and `aria-hidden=true`; Objects component list remained mounted (85 buttons) at scrollTop 2463. Reopening Objects by its rail retained all 85 rows and scrollTop 2463. The container dimensions were clientHeight 262 and scrollHeight 2725. This confirms hidden does not unmount long content or reset its scroll position.

## Reload/workspace retention

Starting from public-selected left Auto-hide and right Collapse, I seeded only the isolated panel preference width fields to 280 and 340 through test-only localStorage input (there is no public resize control in this ticket's candidate). Public reload restored left `autohide`, width 280px and right `collapsed`, width 340px. Public tab sequence Layout → PCB → Layout removed Inspector while on PCB (`#m1-inspector-panel` absent, inspector grid track 0px) and restored it on Layout with the same mode and stored width. Left mode/width stayed autohide/280px throughout. The three grid tracks on Layout with both hidden were `32px 1216px 32px` at 1280×650; on PCB `32px 1248px 0px`.

## Partial, malformed, and unavailable optional storage

The first two cases use a clean isolated app profile and test-only localStorage fixture values followed by a browser reload; they do not touch production profile/project data.

- Valid partial JSON: left `{"mode":"autohide"}` loaded as Auto-hide with no saved width; right `{"width":475}` loaded pinned at 475px. Both panels worked and no alert appeared.
- Malformed/invalid fields: left raw `not-json`; right `{"mode":"future-mode","width":"wide"}`. Both fell back to usable pinned defaults (default tracks 235px/320px at 1280×650), normalized their preference values, and showed no alert.
- Invalid mode plus out-of-range widths: left `{"mode":"future-mode","width":460}` became pinned at the left max 420px; right `{"mode":"collapsed","width":9999}` remained collapsed at the right max 480px. Values normalized back to storage; no alert.
- Unavailable optional storage: init script `/tmp/frontend-run/ui-verifier/panel-storage-unavailable.init.js` intercepts only get/set calls for the `boardstudio:v2:panel:` keys and throws `SecurityError`; all other browser storage stays untouched. On a fresh named session, entering the real REVIUNG41 demo still loaded pinned panels (235/320px defaults). The fixture counted two failed reads and two attempted failed writes. Selecting Auto-hide through the public Objects menu still changed in-memory mode to `autohide`, content stayed usable, and no alert appeared; storage-call counts became 2 reads / 3 writes.

## Document/history/camera neutrality

Compared a clean REVIUNG41 editor baseline against the same session after public Objects Auto-hide, Inspector Collapse, both rail reveals, and idle hides. Footer stayed `REVIUNG41 · Revision 3 · Saved`; Undo and Redo remained enabled in both captures; active document key and selected tab/layer/footprints buttons were unchanged. Canvas SVG viewBox and the first ten scene-part transform strings were identical. The SVG's client rectangle grew when panel tracks collapsed, as expected from layout; camera geometry (`viewBox` and scene transforms) did not move.

## Evidence paths

Earlier concise candidate artifacts remain under `candidate-*` in this folder. Browser screenshots for the additional cases can be rerun using the named sessions `final-t1-07-state`, `final-t1-07-settings`, `final-t1-07-no-storage`, `final-t1-07-neutrality`, and `final-t1-07-neutrality-baseline`. `agent-browser` is installed and `AGENT_BROWSER_SESSION` selects the isolated session. No assistive technology host was available; this report does not claim an AT pass.
