# Case closure handle paired reproduction — 2026-10-05

Candidate: frontend-case-export-20261005, source eac47f42, http://127.0.0.1:34822/. Reference: pinned 5a472a9426e6e38993361da402cd4ec730feb369 at :5175. Neither source changed for this reproduction.

Both imported input.boardstudio, project d0b8c0da-1fc0-4a6d-950c-ec840dc1270d, revision 18. This is the earlier dimensions fixture with only project ID/name changed. Both reached current exact Right case geometry with export available and warnings. The four closure handles were visually identified in Edit mounts / Bottom view; screenshots were used only to locate the actual visible handles and are not retained.

Candidate: click upper handle (680,304), then short right drag to (684,304), and separately upward to (680,300), each reported “Blocked move was not saved.” Exact geometry remained ready and the saved project stayed revision 18 with all closure mounts unchanged.

Reference: upper handle at (674,347), short right drag to (678,347) showed “Saving placement… Previous geometry is shown.” It settled to current geometry/export available with warnings at revision 19. Actual saved archive changes proposal47 from (208.2962435897436,-2.7308064516129065) to (205.6639709472656,-2.2012462615966797). Other closure coordinates remain equal apart from floating serialization noise. See mount-comparison.json and both saved archives.

The viewport scales differ, so these are not identical world-coordinate gestures. This establishes a public mismatch requiring deterministic diagnosis, not yet its cause or a claim that the exact reference endpoint is rejected by candidate state logic. No repair or passing criterion is claimed. F7.5-C01–C04 remain open. Next: evaluate the reference-accepted endpoint against candidate constraints and obtain owning-layer RED before a source fix.

## Deterministic public layout defect localized

On a fresh import of the same revision18 input, select Right case assembly, wait Exact geometry ready, Edit mounts→Bottom. Actual screenshot located proposal47 upper handle at(680,304). DOM-measured canvas before drag: CSS701×509.802 at(247,164.198), backing1050×762. Drag inward/down to(680,312) again reported Blocked move was not saved. Afterward the same canvas width was542.09375/backing812; `.m1-case-edit-hint` was static-positioned,158.90625px wide in the flex canvas shell. Heights and left/top remained constant. canvas-feedback-reflow.json retains actual DOM measurements. This proves feedback changes the public canvas coordinate mapping; it does not yet prove the eventual fixed gesture outcome.

Independent diagnosis using the exact published candidate CoreEngine provider derives one Bottom outer and no holes/openings. The original point has4.082806mm edge clearance against3.5mm required; the actual reference accepted endpoint passes candidate constraints at3.553246mm. Provider/hash/input details are in prepared-constraint-analysis.json. No clearance relaxation or speculative End-sample rewrite is warranted. F7.5-C02 is now missing for the demonstrated feedback-layout defect. Source is still unchanged pending current integration; mounted RED/GREEN and public repaired replay remain required.
