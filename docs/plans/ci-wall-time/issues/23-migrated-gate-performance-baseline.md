# 23: Migrated-gate performance baseline

Status: ready-for-agent
Type: task
Blocked by: [Node-free gate verification](22-node-free-gate-verification.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md), [Native topology and critical path](17-native-topology.md)

## Question

What do comparable full-coverage Chromiumoxide gate runs show about the local critical path and
the proposed split, after the Node-free gate is established?

## Work and completion

- Before building, verify the intended source revision, browser implementation, prerequisites,
  fresh inputs and isolated target paths. Fail with a specific reason if any prerequisite differs.
- Measure unsharded and split runs at the same head with comparable cache and resource conditions;
  reconcile named tests, failures and execution inventories. Separate sequential parity evidence
  from genuine concurrent wall time.
- Record local results and limitations. Historical Playwright results are not Chromiumoxide
  baselines; local timings do not prove a ten-minute hosted gate. Hosted runs and integration need
  further authorization.
