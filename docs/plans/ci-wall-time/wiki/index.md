# CI wall time wiki

Tribal knowledge for the stack-PR validation wall effort. Historical record, not a status page:
every claim is dated, and every claim about code cites the commit it was true at. Sources (issues,
PRs, runs, reports) are cited, never copied in.

- [Decisions](decisions.md): dated user and map decisions driving this effort.
- [Baseline](baseline.md): the green baseline and warm anchors all receipts compare against.
- [Face-completeness local decision](../issues/18-face-completeness-replacement.md#answer):
  opt-in port at `research/chromiumoxide-web-split@0d7ba8f5` and integration at `@0f5d8ae1` have
  focused evidence and combined review; local decision resolved 2026-10-29. Complete browser and
  full-gate verification remain blocked in [22](../issues/22-node-free-gate-verification.md);
  [19](../issues/19-node-free-ergogen-oracle.md) remains open; on 2026-10-29 the user
  reprioritized the seven remaining provider regressions; their later local decision is in [20](../issues/20-remaining-cad-provider-regressions.md#answer).
- [Collision pilot](../issues/20-remaining-cad-provider-regressions.md#collision-pilot--2026-10-29):
  an uncommitted, opt-in browser mapping on `research/chromiumoxide-web-split@0f5d8ae1` passed
  focused real-package and negative controls on 2026-10-29. Independent review identified blockers;
  the [follow-up](../issues/20-remaining-cad-provider-regressions.md#shared-batch-direction-and-exception-control--2026-10-29)
  records 6/6 control groups and the no-op exception-helper negative control. At that pilot handoff,
  re-review, integration and the other ports remained open; full-gate verification remains open.
- [CAD batch sequence](../issues/20-remaining-cad-provider-regressions.md#shared-batch-direction-and-exception-control--2026-10-29):
  A foundation, then [B](../issues/20a-step-provider-ports.md) and
  [C](../issues/20b-qualified-analysis-port.md) in parallel, then
  [E](../issues/20c-shared-batch-cutover.md); preserve both vendor fixture paths. The detailed
  planner thread was unavailable to this documentation pass (2026-10-29).
- [Seven-suite local replacement decision](../issues/20-remaining-cad-provider-regressions.md#answer):
  registered at `research/chromiumoxide-web-split@3d816250`, resolved locally after the reported
  independent final recheck (2026-10-29). The earlier pilot and WIP entries describe prior states.
  Complete browser/full-gate verification and hosted audit remain in [22](../issues/22-node-free-gate-verification.md);
  cold timing remains unproven. The private stamp does not authenticate output and the shared package is stale.
- [Ergogen original-parser prototype](../issues/19-node-free-ergogen-oracle.md#local-original-parser-prototype--2026-10-29):
  at `research/chromiumoxide-web-split@0f5d8ae1`, Chromium matched the separately run Node
  reference and full committed fixture for 42 cases using lock-verified local archives; the real
  Node-free CLI, safety parity, distribution provenance and independent review remain open.
- [Native topology follow-up](../issues/17-native-topology.md): ownership, the local CAD reuse
  observation and experimental opt-in probe at `1d314a45cbd3395aa9ccaf077a92170d81da65c7`;
  real parity and comparable cold/warm measurements remain pending (2026-10-29).
- [Web execution experiment](../issues/04-web-split-design.md): fail-closed runner policy at
  `feat/dependency-aware-web-execution@932bce6f2` and promotion blockers (2026-10-29); the
  earlier [partition proof](../issues/16-web-partition-proof.md) remains non-executing. One
  [local unsharded baseline](baseline.md#local-web-baseline--2026-10-29) passed in 746.5s;
  [local same-head test-result parity](../issues/04-web-split-design.md#local-same-head-split-parity--2026-10-29)
  was independently reviewed on 2026-10-29; authenticated provenance and comparable hosted
  performance remain unproven.
- [Selected web trust design](../issues/04-web-split-design.md#selected-trust-design--2026-10-29):
  independent validated lanes and a host-conclusion aggregate, subject to protected gate inputs
  and a full-rerun policy pending hosted verification; not deployed (2026-10-29).
- [Experimental web workflow](../issues/04-web-split-design.md#experimental-workflow-and-review--2026-10-29):
  split at `4e238b6f`, trusted cache writer at `3d4a3bc9`, and reproducible tooling-gate contract
  at `54e58b33`; hosted trust, behavior, parity and timing remain unverified (2026-10-29).
- [Native experiments](../issues/17-native-topology.md): blocked direct measurements at
  `perf/native-gate-topology@77e1e28e3` and split-planning tests at
  `perf/native-ci-topology@59de2f4ec`; isolated workflow split and stable aggregator at
  `perf/native-ci-topology@954c2d4b16d8f9325b8e031be0ffc0b7dfdbc170` remain unverified for
  promotion; full native guard fixes at `perf/native-ci-topology@ea94881c` and `@ad8a81a3`
  follow independent review and recheck. On 2026-10-29, `perf/native-ci-topology@90411c08`
  requires a recognized binary in each required full-run Cargo command's capture before the guard
  or baseline update; missing or unrecognized captures fail. The reported regression was red
  before the fix, then 38 native guard and 35 scheduler tests passed. Individual capture
  completeness is not proven. None changes the production gate; hosted required-check behavior
  and trust, real CAD parity and comparable cold performance remain unverified, and no speedup
  is claimed. A [single local run](../issues/17-native-topology.md#comments) at
  `perf/native-ci-topology@90411c08` passed `lint test` in 400.8s with cold target directories
  and warm registry/Git caches; it is not a same-head CI comparison or full-gate proof.
