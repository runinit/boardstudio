# 18: Face-completeness replacement

Status: ready-for-agent
Type: task
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Can the existing opt-in Chromiumoxide face-completeness port replace the required Node regression
without changing its cases, assertions, real packaged-provider inputs or failure behavior?

## Work and completion

- Review the original `cad/wasm/tests/step-face-completeness.cjs` against the four uncommitted
  port files in `research/chromiumoxide-web-split`, including freshness and failure paths.
- Preserve the existing required test while correcting and validating the port. Exercise the real
  package and negative controls; distinguish observed behavior from assumed protocol semantics.
- Record an equivalent-or-blocked decision with evidence and remaining limits. Only register a
  replacement after equivalence is established; do not remove coverage to obtain a timing.
- Preserve unrelated work and use isolated build targets. No push, merge or hosted CI.
