# Case 13 paired public battery settings qualification

Status: completed for Case 13 battery settings on the root-verified integrated candidate. Scope is limited to the battery controls and related transport projection; this is not full Case Issue05/F7.4c closure.

The candidate was served at `http://127.0.0.1:34735/` and verified by root from source `7d09d0a60fbb2cc12e259541614b9483ee618d29`, build `frontend-authoring-layers-integrated-20261002`, provenance SHA-256 `aabc367a69f40dce0601226a7909a21f71950824534137a2d41a7253d66b7a66`. Paired reference was pinned React at `http://127.0.0.1:5173/`, commit `5a472a9426e6e38993361da402cd4ec730feb369`.

Both journeys began with `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. It is a wired Sofle v2 archive with left and right physical instances, both initially unflipped and without an explicit battery in their mechanical configurations. Browser sessions were named and isolated; no ambient user profile was used.

## Wired battery journey: passed

On the candidate, importing the exact archive and navigating Case → Configure mechanical stack → Battery → Include produced the source defaults: Width 30, Depth 20, Height 6, Cable width 2, Position X/Y 0, Cable exit X/Y 0. All eight fields were present. Public DOM attributes were min=0.1/step=0.1 on size and cable fields; min=−1,000,000/step=0.1 on position and cable-exit fields.

I committed width 32.5, depth 21.5, height 7.5, cable width 2.5, position (3.2, −4.8), and cable exit (18, 1.5). Width Undo restored 30 and Redo restored 32.5. Setting Width to 0 and pressing Enter showed “Enter a finite value of 0.1 mm or greater.” The project stayed at revision 21 / Saved; focusing the field and pressing Escape restored 32.5.

I saved the project as `candidate-left-battery-accepted.boardstudio` and imported that archive in a fresh named browser session. All eight values survived. Selecting Right PCB/right half showed Include unchecked; selecting Left PCB again restored the accepted left values. This confirms the physical battery edit stayed with the selected instance under the document’s mapping.

The public archive comparison confirms that battery commits preserved the process arrays already present after the separate Configure mechanical stack action. That action materializes normal mechanical defaults at revision 10; Include alone changes only the battery projection and revision 10→11. Subsequent field edits change the battery values and revision, while process entries remain identical. React and Dioxus include-only exports also match on the accepted battery shape and process entries. See `payload-comparison.txt` for the exported JSON values and hashes.

## Wireless transport journey: accepted projection passed; selector write failed

On the original candidate wired archive, selecting Right PCB/right half → Assembly setup → Half connection “Wireless · local battery on each half” changed the visible select value but did not change the accepted document. The adjacent guidance still described wired TRRS; Battery still showed the non-wireless Include checkbox; the app remained green Saved at revision 21. There was no error or rejected-operation message. Saving the project produced an archive byte-identical to the prior wired accepted archive, with `/hardware/transport = "wired"`; after reload the selector reset to Wired. This is a public transport-selector admission/persistence gap. The battery UI correctly remains on the accepted wired branch because no wireless state was accepted.

For comparison, the same Right wireless selection on pinned React was accepted. The exported React archive has `/hardware/transport = "wireless"`; after turning over the right half, `/hardware/instances[id=right]/flipped = true`. React showed no Include checkbox, showed the established wireless guidance, and projected all eight fields. The reflected defaults were Width/Depth/Height 30/20/6, cable width 2, position (−251.0183896, −57.5529677), and cable exit (−234.0183896, −57.5529677), preserving the +17 mm X offset.

I imported that publicly saved wireless/flipped archive into a fresh candidate session. With the accepted right wireless state, the candidate correctly hid Include and showed the wireless guidance plus the same eight reflected values. This confirms the candidate’s accepted-wireless display/default projection; the remaining gap is the candidate transport selector not persisting the attempted change from the original wired archive.

## Evidence and limits

Key archives:

- `candidate-after-config-before-toggle.boardstudio` — candidate revision 10 before Include.
- `candidate-after-include-only.boardstudio` — candidate revision 11 after Include only.
- `candidate-left-battery-accepted.boardstudio` — candidate left edit accepted and portable.
- `candidate-wireless-right-after-left-battery.boardstudio` — unchanged wired archive after the transient selector choice; byte-identical to the left accepted archive.
- `react-before-battery-toggle.boardstudio` and `react-after-include-only.boardstudio` — paired React payloads.
- `react-wireless-right-flipped.boardstudio` — accepted React wireless/flipped input used to verify candidate projection.

Relevant screenshots are under `screenshots/`: wired defaults and edited fields, Undo/Redo, invalid draft and recovery, save/reopen, right-instance absence, candidate transient Wireless selection and post-reload Wired reset, and paired React/candidate accepted wireless flipped values. `artifact-manifest.sha256` contains hashes for the key archives, screenshots, and payload comparison; its SHA-256 is `c6af4b7e37ee2ced9f917a12110f84b6234c6fd4159d774c086cadc8bfb67d6a`.

No relevant browser page errors or console errors were observed in the checked journeys. Case 11 manual preview regeneration remains a known adjacent gap and was not used as a battery-settings acceptance criterion. No broader Case closure is claimed. No repository source or docs were changed by this QA stream.
