# Compact Case page overflow: confirmed inherited containment defect

## Decision

The 390×844 page overflow is real, but it is not a newly introduced mechanical-settings regression. Public import of the identical archive into both current :34675 (0cad7577) and prior :34671 reproduces the same heights before and after real Generate completes. No source change, DOM/style injection, scripted storage write, or fake provider was used. No fix is yet applied or verified.

## Input and public workflow

Archive: integration `.scratch/dioxus-case-workspace/evidence/mechanical-ui-layout/candidate-reviung41.boardstudio`, SHA-256 `28fb3f3b989ea2eb6d3a7bb6f09e6997794696a0576dd98f99bb38a5e3dca324`. This is the verifier's exported revision-3 configured Gasket REVIUNG41 ProjectDoc; both fresh profiles imported these exact archive bytes through the visible file input. Candidate and prior profiles are isolated task profiles under `/var/tmp/frontend-run/mechanical-overflow-{current,prior}-profile`; sessions `mechanical-overflow-current` and `mechanical-overflow-prior`.

Open URL, set viewport 390 844, upload the archive to `input[type=file]`, select Case using the public Workspace combobox, toggle Inspect using its exact accessible button name. Generate using the actual Generate case button. Ready snapshots retained in this directory show Cancel generation disabled and the interactive 3D preview. Objects remains closed. Generation was not cancelled; timings were not benchmarked.

## Actual red oracle

`python3 /tmp/frontend-run/mechanical-overflow/measure.py mechanical-overflow-current current-after-generation-inspect-open --assert-fit`

This real read-only browser oracle reports document 390×924 versus viewport 390×844, then exits 1: `AssertionError: document vertically overflows viewport`. The equivalent prior-build command (`mechanical-overflow-prior prior-ready-inspect-open`) also exits 1 at 924. With Inspect closed, both builds measure 853. Before-generation current Inspect open is also 924; closed is 853. JSON measurements and paired ready screenshots are beside this report. The oracle checks document dimensions, rather than assuming visible canvas means no page overflow.

## Ranked hypotheses and source diagnosis

1. **Confirmed: normal-flow compact Inspector adds to an already excessive fixed Case height.** Current immutable packaged CSS and source `web/assets/m1.css` agree: lines318–345 set the workbench height auto/min-height100svh, editor body to a vertical flex layout with overflow visible, and Inspector to an ordinary relative-positioned slot. Lines432–434 give Case workspace `height:calc(100svh - 140px); min-height:390px`.
2. **Falsified for this observation: a hidden transformed drawer creates overflow.** Closed compact Inspector computes display:none. Its removal changes document height from924 to853. Open Inspector is in normal flow and contributes71px.
3. **Confirmed contributing cause: fixed140px allowance undercounts actual shell chrome.** At844px height, Case workspace is704px. Actual topbar44 + compact navigation43 + footer34 + status28.390625 =149.390625px. Total closed height853.390625. Adding Inspector71 gives924.390625. This accounts for the entire measured excess without attributing it to settings content or CAD geometry.

The prior `case-viewport-layout/public-final/compact-no-scroll-intersection.json` already records a924.390625px ancestor bottom. Its successful viewport-intersection check proves usable visible canvas, not absence of document scrolling; it cannot be used as a no-overflow baseline.

## Smallest coherent proposed repair, not applied

Make the existing compact **Case** shell allocate the real remaining viewport height through flex sizing. Replace Case's fixed `100svh - 140px`/390px minimum with a shrinking flex child (`flex:1; height:auto; min-height:0`) inside a Case-scoped bounded workbench/editor-body. Keep header/navigation/footer/status in their existing shell tracks. Give open compact panel content bounded internal scrolling so it cannot increase shell height; preserve access to its controls. Do not merely change140 to149, hide overflowing document content, or round a measured status height into another viewport subtraction.

Root should select the narrow Case-scoped panel sizing policy while preserving the currently deferred general compact-panel work. The pinned React source already keeps its root at100dvh/overflow:hidden (`app/src/ui/workbench.css:114–127`) and makes compact Inspector a bounded fixed drawer (`unified-workbench.css:111–113`). Adopting that full drawer behavior is a broader compact-panel change; a Case-only flow-preserving containment repair must not claim complete responsive parity. No public API or state/renderer change is required for either private CSS choice.

## Required green and controls

Rerun the exact document-fit oracle after a fresh production build for Inspect open/closed and Objects open/closed, including both open if supported. Retain canvas intersection and nonzero usable dimensions, visible/reachable settings and Inspector controls via their own scroll containers, footer access, pointer/keyboard panel toggles, and public Generate/selection behavior. Include390×844 and a shorter viewport; do not obtain green solely by clipping controls. Fresh same-archive prior control is already red, so report this as an inherited compact containment gap. Full React drawer parity and general responsive acceptance remain separate.

Source inspected at integration HEAD9ef5bc5cad09ab3064711b45fc348b4cc09a74d4; CSS SHA-2560b50ba5e8c92e1325ee5fe344cd48442153402fe07aa46a01e0b7247b198f2d8. Production candidate remains immutable0cad7577. No Cargo/build was run.
