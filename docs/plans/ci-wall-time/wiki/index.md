# CI wall time wiki

Tribal knowledge for the stack-PR validation wall effort. Historical record, not a status page:
every claim is dated, and every claim about code cites the commit it was true at. Sources (issues,
PRs, runs, reports) are cited, never copied in.

- [Decisions](decisions.md): dated user and map decisions driving this effort.
- [Baseline](baseline.md): the green baseline and warm anchors all receipts compare against.
- [Native topology follow-up](../issues/17-native-topology.md): ownership, the local CAD reuse
  observation and experimental opt-in probe at `1d314a45cbd3395aa9ccaf077a92170d81da65c7`;
  real parity and cold/warm measurements remain pending (2026-10-29).
- [Web execution experiment](../issues/04-web-split-design.md): fail-closed runner policy at
  `feat/dependency-aware-web-execution@932bce6f2` and promotion blockers (2026-10-29); the
  earlier [partition proof](../issues/16-web-partition-proof.md) remains non-executing.
- [Native experiments](../issues/17-native-topology.md): blocked direct measurements at
  `perf/native-gate-topology@77e1e28e3` and split-planning tests at
  `perf/native-ci-topology@59de2f4ec`; isolated workflow split and stable aggregator at
  `perf/native-ci-topology@954c2d4b16d8f9325b8e031be0ffc0b7dfdbc170` remain unverified for
  promotion. None changes the production gate; no speedup is claimed (2026-10-29).
