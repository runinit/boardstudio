- source_plan: `_bmad-output/plan-packaged-part-drag.md`
  summary: Diagnose the Stagger cancel scene mismatch after Escape.
  evidence: Two packaged runs passed the saved-document and revision checks before failing the exact scene comparison. Observe subsequent frames to distinguish delayed rendering from a persistent preview.
  resolution: Resolved. The packaged run restored the exact pre-drag scene within the bounded render wait, preserved the document and revision, and continued through Matrix move and Undo.
- source_plan: `_bmad-output/plan-packaged-part-drag.md`
  summary: Verify pointer capture before reporting it for Stagger cancel.
  evidence: The cancellation branch receives live.capture and records pointerCapture as true without checking the observation.
  resolution: Resolved. The harness now checks capture during the held preview, releases the pointer on failure, and records the observed value. [Packaged run](../.evidence/stagger-cancel/drag.log:175)
