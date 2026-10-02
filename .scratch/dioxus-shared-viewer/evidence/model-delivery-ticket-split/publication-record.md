# F7.3 model-delivery children 07–08 publication record

Both local child issues were published after the corrected KiCad bridge proposal and both ticket drafts received independent Spec and Standards clearance. The 62-parent graph is unchanged; issue 02 remains the Case integration umbrella, and issue03–06 bodies are unchanged.

| Child | Independently reviewed body SHA-256 | Published issue SHA-256 |
|---|---|---|
| 07 imported/archive-backed BoardReference models | `40115b3ff4a869ac90c0e30e13585db283d5ed65818d9745e570c8fbdf7b9647` | `81fea3d20f215c173975a7a1cbfed490a5ef632768ee0ddd01c4058457d34e12` |
| 08 generated Ergogen preview models | `a9fe9f411c4beb07172a506a767f6092a5afb9989477243cc246e531f098ad9d` | `c842d6879f7caaca2c3c591acfbbcaad11855ad5f426644a2e11438626a7c8b9` |

Post-review publication edits set status to `ready-for-agent`, replaced “proposed child 07” with published issue 07, and removed obsolete review-pending phrases from the blocker/status text. No scope, acceptance criterion, ownership boundary, actual start/acceptance join, or parent edge changed after review.

Final independent review records are `final-spec-review.md` and `final-standards-review.md`. The corrected generated bridge proposal is `.scratch/dioxus-shared-viewer/evidence/model-delivery-private-ports/kicad-bridge-proposal.md`, SHA-256 `de3bb2a43f8d4db5ee6235819a1c1129e07193e57b779078d3b91b952763702a`; its source identity manifest is retained in that proposal.

Published bounded-ticket count is 41. Issue02 remains the Case integration acceptance join. Canonical F7.3 starts after F7.1, retains INT.2 and BND.1 acceptance joins, and later F7.8 remains unchanged. Child07’s model module source is reviewed but unregistered/uncompiled. Child08 generator worker authoring starts in the shared-viewer worker lane; Runtime/CoreWorker, page-host registration, shared Case mount/CSS and build integration remain root-owned serial work.
