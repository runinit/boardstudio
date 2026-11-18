- source_plan: `_bmad-output/plan-packaged-part-drag.md`
  summary: Diagnose the Stagger cancel scene mismatch after Escape.
  evidence: Two packaged runs passed the saved-document and revision checks before failing the exact scene comparison. Observe subsequent frames to distinguish delayed rendering from a persistent preview.
- source_plan: `_bmad-output/plan-packaged-part-drag.md`
  summary: Verify pointer capture before reporting it for Stagger cancel.
  evidence: The cancellation branch receives live.capture and records pointerCapture as true without checking the observation.
