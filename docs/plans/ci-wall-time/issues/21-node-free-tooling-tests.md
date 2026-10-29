# 21: Node-free tooling tests

Status: ready-for-agent
Type: task
Blocked by: [Node-free Ergogen oracle](19-node-free-ergogen-oracle.md)
Parent: [CI wall time map](../map.md)
Related: [Web-split design](04-web-split-design.md)

## Question

Can the model-lookup and CAD worker tooling regressions preserve their actual code, assertions and
execution-context behavior without Node?

## Work and completion

- Compare each replacement with its original test. A normal browser page is not automatically
  equivalent to the model-lookup test's bare V8 context; the CAD worker test also exercises its real
  download STL/oracle helper.
- Exercise success and failure paths, retain the original required checks until parity is shown,
  and remove the gate's incidental Node version probe only when it is no longer required.
- Record evidence or a concrete blocker. No push, merge or hosted CI.
