# PCB clearance, layers and mounted-instance ownership

Candidate `frontend-case-extraction-lifecycle-20261005`, production source `cc728fc644d57b985feed9128ffdb40f9c75a709`, root http://127.0.0.1:34822/, observed script `boardstudio-web-dxhd9c1c044c4d075.js`. Reference http://127.0.0.1:5175/, pinned `5a472a9426e6e38993361da402cd4ec730feb369`, observed `main-CYxchQWA.js`. HEAD b53 adds only test/config/evidence over that release source; live next-batch edits are not served.

## Fixture and actual actions

Both imported retained `candidate-before-route.boardstudio` from `/home/chris/.local/share/boardstudio/reviews/pcb-public-next-20261003/`, SHA256 `c1120b91502216be6e702b16542e7c9118bd2d65d596209cf5c4b2dfcbcf6302`, project `vik-module-review`, revision6. This is generic saved-module compatibility, not restoration of the removed VIK catalogue scope.

Both publicly opened splitter-above by Enter. In Parts Assembly geometry, authored a service volume X1/Y2, width16/depth8, Z-1/height4, with evidence text **Synthetic QA envelope for clearance projection; not a physical measurement.** Added and saved the project module profile. Review completeness remained unchecked; rendered polygons are `data-qualified=false`. This synthetic fixture must not be represented as measured hardware.

Returned to PCB, enabled Clearance & service. Both render above polygon `22,18 38,18 38,26 22,26` and back-host/180-degree below polygon `64,22 48,22 48,14 64,14`; EC11 has no volume and no clearance polygon. Exact rendered attributes are in observations.json.

| Layer action | Host pads, both | Module pads, both | Clearances, both |
| --- | ---: | ---: | ---: |
| Both visible | 56 | 159 | 2 |
| Hide host Pads | 0 | 159 | 2 |
| Restore host; Hide module Footprints | 56 | 0 | 2 |

Restored both layers. Space on splitter-below routes to its Parts source in both frontends; candidate Edit selected mounted placement returns to Main board/Board-wide, host Back/facing Front, X57/Y20/yaw180/gap3, Board attachment/service4. Reference selected placement is Main board/back/splitter-below, with matching values. Actual pointer activation of EC11 source footprint routes both frontends to EC11 and EVQWDG001, with reference front/ec11-rotary X85/Y20/yaw0. Earlier splitter-above pointer/Enter/Space proof is retained; this journey covers every mounted placement in the fixture with an actual input path, not every possible input permutation.

## Saved outputs and reopen

Actual Save project copy downloads retained as candidate-clearance-r7.boardstudio and reference-clearance-r7.boardstudio, with parsed projects beside them. saved-archive-comparison.json records download identity/hash and differences. Candidate input→output has exactly revision6→7 plus one module-definition service volume. Module placements, host parts, nets and all unrelated accepted data are unchanged by the routing and visibility sequence.

Candidate/reference parsed projects differ in generated volume ID and three Y values (-9.020000000000001 versus -9.02), besides equal numeric representation (3.0/3 etc.). Do not claim byte equality. Both actual archives were reimported through public Open project. After settled PCB entry and enabling Clearance & service, both show the same two owner-labelled polygons and 56 host/159 module pads. Reopen observations also record actual loaded scripts. Visibility preferences are transient: both reopened with Clearance & service hidden.

## Joins and limits

Reuse retained 435 normalized geometry leaf comparison in `.scratch/dioxus-pcb-view/evidence/15-mounted-module-layers-20261003/public-receipt.md`, source-pad ownership in issue14, actual Findings-list pointer/keyboard→Layout selection and hidden-layer retention in issue24 receipt, and placement/supports Save/Undo/Redo/reopen in issue22/23 receipts. The obsolete marker-only focus override is not current behavior and is not claimed; issue24 records the paired actual action and correction.

No new product defect or source change established. This adds the missing nonempty clearance and independent visibility/remaining-instance routes. Independent review must determine exact F5.4 and F5.5-C01 closure and identify any original requirement still unproven. It does not claim pending/error/cancel placement outcomes, stale async routes, routed reference poses, full F5.5, or any F8/F9 acceptance. Existing module-free/board-switch and flip obligations require attributable retained evidence or remain open. No optional/mobile checks performed.

## True flipped geometry and board-switch follow-up

Independent review correctly distinguished host/front-facing/back from resolved `flipped=true`. Publicly changed splitter-above to Front/Front in both. Initial Save was rejected by both with “Board support PART0 must span the selected module-to-host gap from its facing PCB surface”. Actual attempted saves remained revision7/front-back; rejected-flip-* files retain that failed attempt and must not be used as successful flip evidence. Removed both now-incompatible support rows from this test fixture and saved again: both actual downloaded revision8 archives now contain hostFace front/facingFace front. Other below-module support geometry remains present.

All selected artwork/board/clearance/hole/standoff layers were shown. flipped-rendered.json retains every rendered leaf AND ancestor group class/transform attribute, including actual flipped host-side artwork classes. The comparison normalizes only m1/wb class prefixes, whitespace and numeric formatting to12 significant digits; all628 elements (201above,207below,220EC11) match. Actual front/front saved placement establishes true flip; it is not inferred from a label. Both now also show matching nonempty clearance geometry for flipped module. Full raw values retained before normalization.

New board in both publicly creates/selects empty Board2. Both contain zero module graphics there. Selecting Main board again restores exactly splitter-above,splitter-below,EC11 and their corresponding geometry; board-switch-observations.json retains empty and returned states. This establishes nonempty→empty→original board ownership without injecting app state. New board is fixture-only; no newboardfeature acceptance is claimed. Stale asynchronous/callback route rejection still requires owning execution evidence.
