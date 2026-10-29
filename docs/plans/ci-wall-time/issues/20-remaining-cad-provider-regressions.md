# 20: Remaining CAD provider regressions

Status: ready-for-agent
Type: task
Blocked by: [Node-free Ergogen oracle](19-node-free-ergogen-oracle.md)
Parent: [CI wall time map](../map.md)
Related: [Face-completeness replacement](18-face-completeness-replacement.md)

## Question

What replacement approach preserves the seven other packaged-provider regressions' cases,
assertions, package freshness, failure propagation and relevant runtime behavior without Node?

## Work and completion

- Inventory the original scripts and validate the approach against one regression and the actual
  package. Identify where shared or detached buffers and process-memory diagnostics need distinct
  treatment. Compare named cases and failure paths with that original.
- Record the approach or a blocker, then create appropriately sized follow-up tickets for the
  remaining replacements and wire them as blockers of gate verification. Keep each original
  required until its own replacement is validated. No push, merge or hosted CI.
