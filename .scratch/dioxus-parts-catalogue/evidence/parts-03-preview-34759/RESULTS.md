# Parts standalone 3D preview delta on 34759

This is the changed selected-definition 3D/header/readiness/return journey only. The accepted archive was opened on each origin as fixture setup; the prior import, error, Undo/reopen, and Project history journeys were not repeated.

## Candidate, reference, and fixture

- Candidate: `http://127.0.0.1:34759/`, exact served source `04c85b88eae2894b416979f785a1f0060d2eb27d`.
- React reference: `http://127.0.0.1:5173/`, pinned source `5a472a9426e6e38993361da402cd4ec730feb369`.
- Both origins opened the existing `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.
- Candidate browser session: `parts-preview-936481b4b15c`; React session: `parts-preview-react-20261003`.
- Root reports strict page WASM all-target Clippy PASS (16.58 s), eight fresh and 22 inherited package commands, 1,366 source inputs with zero drift, and 145 assets per route with zero mismatch. Provenance SHA-256: `81ee87aead500473ae22f79834a2bc921618442a081719860a08188c063e49aa`.

## Verified selected-definition journey

The candidate loaded the accepted `ergogen:ceoloide/switch_mx` definition (seven pads, Back side) in Parts. Selecting **3D model** mounted the shared viewer with the accessible name “Interactive 3D Parts sample preview. Navigate the isolated read-only sample; it cannot edit the active project.” The candidate showed **Fit sample**, Parts-specific camera/display/view control labels, and `3D preview ready.` React showed the same selected definition in its 3D sample viewer. Candidate and reference captures are retained below.

Returning to **2D footprint** restored the selected MX source preview and its layer control. The candidate footer remained `Sofle v2 · Revision 9 · Saved`; no document edit/history action was issued during preview setup, rendering, or return.

A no-linked-model fallback was also exercised with `ergogen:infused-kim/utility_text`. Candidate 3D reported `No 3D model is linked to this definition.` while the sample viewer reached `3D preview ready.` The React sample for the same definition showed the sample surface without an alert.

## Captures

Files are retained under `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/`.

| Capture | SHA-256 |
| --- | --- |
| Candidate MX 3D ready | `9a12cb2e7552a284c91bd58a34f892016b5f6b6592182962f56e4d80b908d6f3` |
| React MX 3D | `8bea8c143ae6d5767e9904f68d950dc3ab9693e0a3d3bc7048d13b8cee7d5ddf` |
| Candidate MX 2D return | `b7b40e661ec1f2e88e5f473e58cddefea12ce3ec1667664732792266e4b1d4f5` |
| Candidate no-model 3D fallback | `d52238ea6468464ce657aab318630bcc91e4d3a2767f3d4b335decbb88f1228e` |
| Candidate utility-text 2D return | `2335dca4011b7a31a7fbe04217a7be068f15a3e0c4d4091dd6498767f0afd06a` |
| React utility-text 2D | `753df0b84d210d1e27bd9e91fe440773b7038f4411b839211d330d6d9ab7c3fe` |
| React utility-text 3D ready | `c4a644bbc0971adcb826ba6aaf4e6b311c2f21bf56931af266b5fc3009930883` |

## Observed limit for the next Issue 03 packet

Selecting `utility_text` and returning to 2D produced a candidate alert, `Footprint preview failed: The selected definition contains no 2D footprint geometry.`, and no canvas. React presented its empty `0.0 × 0.0 mm` surface without an alert. This is recorded as an actual empty-geometry parity failure, separate from the new 3D producer; the next Issue 03 assembly/companion packet must render the legitimate empty surface without inventing geometry. The candidate source packet does not claim this path is green.

The next source-backed assembly frontier is also recorded in the Issue 03 ticket: Issue 02 owns the eight-preset/variant selection input, while Issue 03 consumes ordered recipe members and their accepted definitions, merged generator parameters, poses, rotations, and sides. This receipt closes neither that work nor any F4.4/F7.3/INT.2 parent acceptance join. RF-006 and RF-009 remain the relevant scope/evidence records; no new RF item was observed.


## F4.4-C01/C02 follow-up on 34805 / 5175

The existing source-backed 2D layer work above continues to cover the selected footprint’s named visibility path (F4.4-C01). In the paired active sessions, Sofle v2 was opened and `mcu nice nano` selected; the candidate and React both exposed named `F.Cu`, `Dwgs.User`, `F.SilkS`, Courtyard, Drills, and Pad numbers visibility, and both accepted hiding `F.SilkS` while retaining the other toggles. Existing imported-footprint evidence remains the coverage for imported geometry/layer mapping.

For the 3D side/pose path (F4.4-C02), both sessions rendered the controller on Back. Changing Board side Back→Front (and again Front→Back) then applying the generator settings reproduced a candidate-only stale failure: Dioxus stayed in 3D with `3D Parts preview failed: Accepted board preview source became stale`, while React automatically settled to a ready 1/1 model on the requested side. Toggling candidate 2D→3D manually retried and produced a ready Back preview, so the stale result is recoverable, but the primary accepted-edit journey is not yet verified green. Captures below show the post-retry candidate and corresponding React Back state; they do not depict the stale alert itself.

- Candidate served source: `916a4054`, `http://127.0.0.1:34805/`; React pinned source: `5a472a9426e6e38993361da402cd4ec730feb369`, `http://127.0.0.1:5175/`.
- Candidate post-retry Back: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/dioxus-mcu-back-after-retry-34805.png`, SHA-256 `626e233d8a29bfb95d4ab4c64f1245c408b47a64b9b6c5e083d907045aa14903`.
- React Back: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/react-mcu-back-5175.png`, SHA-256 `86dc893c97b7b5af48447f054f6c38821e880d7484179fbc0562b5e0be177fdc`.

A narrow source repair now subscribes the preview panel to runtime version changes, rebinds its preview identity to the current accepted snapshot when the document/session still match, and refreshes accepted project definitions in the selected recipe. The regression assertion was added for accepted-source identity transition. The native focused command executes zero tests because this UI module is wasm-only; the wasm focused runner reached compilation but was blocked by unrelated existing `layout_workspace.rs` test code (`dyn_into` without `JsCast` in scope). The running browser server remains on the pre-edit source, so C02 stays open pending coordinator integration and a fresh mounted confirmation. No shared viewer/runtime files were touched.


## Qualified-candidate F4.4-C02 retest — FAIL

Candidate `frontend-functional-refresh-20261004`, served source `6d3e89fc` at `http://127.0.0.1:34806/boardstudio/`, opened Sofle v2 and selected `mcu nice nano` in Parts. Its 3D sample first showed `3D preview ready.` with Board side Front. Changing Board side to Back and applying reproduced the same failure after a 5-second settle: the 3D view remained selected but displayed `3D Parts preview failed: Accepted board preview source became stale`. I stopped there without a manual 2D→3D retry or reopen.

The paired React 5175 session on the same Sofle fixture accepted Back and remained ready with `1 / 1 models · 1.6 mm PCB`. Full mounted accessibility snapshots were inspected for both panels and generator side controls. Candidate stale capture: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/qualified-34806-c02-stale.png`, SHA-256 `d0a1a6e280ac10807f974505d9aaa1c13b8f3e9416d3fa04313b450d2155dc72`; React ready capture: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/react-5175-c02-back-ready.png`, SHA-256 `86dc893c97b7b5af48447f054f6c38821e880d7484179fbc0562b5e0be177fdc`. F4.4-C02 is still not green on this qualified candidate.


## F4.4-C02 lease lifecycle repair evidence

The qualified retest above still showed the stale-source alert. Source tracing found the generation-change `use_effect` invalidated the shared lease slot unconditionally. If a newer request had already installed its lease before that deferred effect ran, the old owner's effect revoked the current request. The earlier runtime-version/source-identity retarget was removed: the Parts workspace already receives refreshed accepted props, and rebinding props did not prevent this lease race.

The preview now selects the active owner generation synchronously. The lease slot invalidates only a lease from the retired generation and refuses a late lease install from an old async owner; the deferred effect is also generation-scoped. This preserves the current accepted-scope admission checks in Runtime.

Focused regression `parts_preview::tests::late_old_generation_invalidation_cannot_revoke_new_parts_preview_lease` exercises old lease retirement, late old-owner install rejection, and the prior owner's deferred invalidation after the new lease is installed. RED: temporarily restoring unconditional invalidation failed at `the old deferred effect must not revoke the new lease`. GREEN: `python3 scripts/migration-deliver.py focused-test -- cargo test --manifest-path web/Cargo.toml -p boardstudio-web --bin boardstudio-web --features page parts_preview::tests::late_old_generation_invalidation_cannot_revoke_new_parts_preview_lease -- --exact` passed (1 passed, 0 failed). `rustfmt` and `git diff --check` pass.

At the time of this repair note, mounted retest was pending because source `6d3e89fc` predates the lease repair. The qualified retest on source `3b5bb1b3` is recorded below.

## Qualified-candidate F4.4-C02 retest — GREEN

Candidate `frontend-current-actions-20261004`, source `3b5bb1b3`, `http://127.0.0.1:34807/boardstudio/`, opened Sofle v2 and selected `ergogen:ceoloide/mcu_nice_nano` in Parts. The mounted 3D sample reached `3D preview ready.` on Front. With the 3D tab left selected throughout, Back→Apply settled within 4 seconds to `3D preview ready.` and side Back; the reverse Front→Apply also settled within 4 seconds to `3D preview ready.` and side Front. Full accessibility DOM snapshots showed the selected `mcu nice nano`, generator side, mounted 3D footprint model preview, and ready status after each Apply; no stale-source alert or manual 2D→3D toggle occurred. This qualifies the repaired accepted-side Apply journey and rapid successive owner-generation suppression on this candidate. React 5175 was not needed for this retest; its matching behavior is retained above. Candidate browser session `parts-c02-current-retest-7dd31abc1fc4` was closed.

## F4.4-C01 paired isolated 2D preview — GREEN

On candidate `http://127.0.0.1:34807/boardstudio/` (source `3b5bb1b3`) and pinned React `http://127.0.0.1:5175/` (source `5a472a9426e6e38993361da402cd4ec730feb369`), opened the Sofle v2 demo independently and selected the `ceoloide/mcu_nice_nano` definition in Parts. Both selected-definition previews showed 24 pads, 24 drills, 24 pad numbers, 16 drawing graphics, 24 `F.SilkS` text labels, and the same named layers: `F.Cu`, `Dwgs.User`, `F.SilkS`, `Courtyard`, `Drills`, and `Pad numbers`. Both layer menus also named the part (`mcu nice nano`). Hiding that part removed all measured geometry/text from the 2D SVG; showing it restored the same counts. Preview and visibility actions did not edit the project. This reuses the existing C01/C02 fixture and receipt and does not repeat 3D side checks.

- Dioxus visible and hidden: `/home/chris/.local/share/boardstudio/retained-tmp/20261003/parts-preview-34759/f44-c01-20261004/dioxus-nice-nano-visible.png` (`78203967d94f5ccf793b4403c340f3a20dcfa23dc0bb8b1bd01b6cfe8cae0566`) and `dioxus-nice-nano-hidden.png` (`93196b3eba40f3e6bc7de125d2bf30bf1c01b9afa74984bfcc1de6f393c3437c`).
- React visible and hidden: same directory, `react-nice-nano-visible.png` (`031eb3d7b4fcafdd99cdd8ee520a9bec740349b5e47b0f2341bc48047cf210ca`) and `react-nice-nano-hidden.png` (`06359143beed5dfe734a8d15f0dcc2720645c33fdeeb10a2334b5ec34eb41275`).
