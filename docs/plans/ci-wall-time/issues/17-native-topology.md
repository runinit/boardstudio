# 17: Native topology and critical path

Status: claimed
Type: research
Parent: [CI wall time map](../map.md)
Related: [14: Native warm wall](14-native-warm-wall.md), [04: Web-split design](04-web-split-design.md)

## Ownership and scope

2026-10-29: The chief-of-staff/coordinator owns triage and handoff for this follow-up. The
ongoing parent effort claims this ticket for the native topology probe; no separate delegate or
session ID is asserted. Check active ownership and build activity before using either existing
worktree: `research/native-cad-artifact-reuse` and
`research/native-cad-reuse-experiment` were both at `8d6bbd0fe` when inspected, but branch
presence does not establish owner liveness. Do not delete or overwrite either. The local-first,
no-hosted-run and no-tuning-matrix constraints in the [map](../map.md#near-term-focus--2026-10-29)
remain in force.

## Question

What full-coverage native job topology can reduce the actual longest path, including setup,
tooling, lint, workspace and CAD tests? Measure before asking the user to choose topology; do not
assume target sharing alone solves it. Preserve all tests, feature configurations, qualification
selections, nonzero-test guards and failure propagation.

## Evidence and limits — 2026-10-29

- Ticket [14](14-native-warm-wall.md) reports warm 14m41s at probe `008cb78f`: tooling 41s,
  lint 132s, workspace test lane 426s and CAD lane 640s. Setup and serial steps still matter if
  CAD is shortened. The inference that checkout freshness causes all rebuilds is not proven;
  F1/F2's estimated savings are hypotheses, not measured full-gate gains.
- The local experiment at `8d6bbd0fe` compared sequential offline `--locked` release tests: WASM
  after the library in a shared target took 59.1s versus 68.0s in a fresh separate target, one
  noisy comparison (~9s). Monstertruck and cgmath still rebuilt with differing dependency-unit
  fingerprints; 57 WASM names matched. A vendor comment edit and restore each caused rebuilds,
  but restored-cache freshness, qualification selections, injected failures and the full CAD
  lane were not tested. No sharing change or gate saving was established. Source: experiment
  handoff, 2026-10-29; local logs in
  `/home/chris/.cache/tmp/opencode/boardstudio-native-cad-reuse-experiment/.evidence/native-cad-reuse-experiment/`.
  Park shared-target adoption unless stronger representative evidence warrants revisiting it.

## Work and acceptance

1. Before restructuring, investigate a CAD executed-test-name guard and its baseline: the
   existing owner-selection nonzero guard is not a name-parity guard. Establish an unsharded
   reference with test identities and outcomes for library, WASM native and all qualification
   selections; test missing, ignored, failed and incomplete selections and failure propagation.
   Evaluate `scripts/verification_receipt.py` and the native guard for reuse, explicitly identifying
   CAD coverage gaps rather than assuming nested output is captured. Do not reduce coverage.
2. Measure the actual DAG and longest paths on a named revision with comparable warm and cold
   states, setup/restore costs, resource limits, concurrency and interference. Profile tooling,
   lint, workspace and CAD invocation groups; distinguish compile, execution and dependencies.
   Check no competing build is active and never overlap Cargo on a target. Local runner emulation
   is a diagnostic, not a hosted receipt; record differences rather than assuming a scaling factor.
3. Compare safe splits and qualification's repeated vendored builds (14's F2) without changing
   feature-specific coverage. Any renewed artifact reuse study must verify feature/profile and
   input compatibility, changed-source rebuilds and restored-cache freshness, all selection and
   failure parity, and representative full-lane walls. Report negative results as well as gains.
4. Present measured candidate topologies, overheads and risks for human decision alongside 04;
   build aggregation only for the chosen topology. Require actual execution evidence and
   fail-closed missing/failed shards, not merely reported partition membership. Hosted same-head
   full-gate receipts and required-check verification wait until GitHub work is authorized.

Reject mtime rewrites such as `touch` or `git-restore-mtime` to force Cargo cache hits. No
content-keyed mtime manipulation is approved; any cache proposal requires a separate trust and
correctness review. Cold versus warm target, runner capacity and cache quota are human decisions.

## Comments

- 2026-10-29: Claimed for the ongoing parent effort. Experimental, opt-in local CAD topology probe
  on `perf/native-gate-topology` at `1d314a45cbd3395aa9ccaf077a92170d81da65c7`; its author
  report is `docs/research/2026-10-29-native-topology-probe.md` at that commit. It is not a gate
  replacement and changes no required gate. The handoff reports 134 focused tests passed; this
  is not real CAD execution parity. No full qualification reference or real CAD timings were
  obtained. Real unsharded parity and comparable cold/warm measurements remain pending; the
  ticket is not resolved and no speedup or full-gate result is claimed.
- 2026-10-29: `perf/native-gate-topology@77e1e28e3` makes direct probe helpers fail closed until
  effective Cargo configuration and the compiler can be controlled. The handoff reports 17
  probe tests passed; no timing or real CAD parity was obtained. This remains experimental.
- 2026-10-29: `perf/native-ci-topology@59de2f4ec` adds split-planning tests retaining the full
  native workflow command and workspace-name guard. Further implementation is active; the
  production gate is unchanged. Fail-closed required-check aggregation, branch-protection
  policy, real CAD parity and timing remain unresolved; no speedup is claimed.
- 2026-10-29: The isolated workflow experiment at
  `perf/native-ci-topology@954c2d4b16d8f9325b8e031be0ffc0b7dfdbc170` splits repo tooling and
  lint test into separate jobs and retains a stable `native` aggregator. The handoff reports
  35 scheduler, 16 CAD driver, 34 native guard and 37 verification receipt hermetic tests
  passed; these were not independently rerun for this update. No production gate or hosted CI
  changed. Branch protection and GitHub skip, cancellation and rerun semantics are unverified;
  real CAD parity, isolated prerequisites and comparable cold timings remain unverified. The
  ticket remains claimed and unresolved, and no speedup is claimed.
- 2026-10-29: An independent review of `perf/native-ci-topology@954c2d4b` identified a
  fail-open full-workspace name guard when no test binary was recognized, alongside unresolved
  hosted required-check/trust and real CAD parity gaps. On the experimental branch,
  `ea94881c` fails full runs with no recognized binary; an independent recheck found normal
  capture paths protected but identified a remaining empty-capture bypass. `ad8a81a3` also
  fails full steps with entirely empty captures, without changing partial empty behavior or
  overwriting the baseline. An independent recheck of `ad8a81a3` confirmed the empty-capture
  fix and noted a hypothetical lost individual capture could leave an explicit baseline update
  incomplete; current runners capture each Cargo test command. The handoff reports regressions
  observed red before the fixes,
  followed by 37 native guard and 35 scheduler tests passing; not independently rerun here.
  The production gate is unchanged. Hosted required-check behavior and trust protections,
  real CAD parity and comparable cold performance remain unverified; no speedup is claimed.
- 2026-10-29: Experimental `perf/native-ci-topology@90411c08` requires each required full-run
  Cargo command to have a capture containing a recognized binary before the normal guard or a
  baseline update; a missing or unrecognized per-command capture fails. The handoff reports the
  regression observed red before the fix, then 38 native guard and 35 scheduler tests passing;
  these were not independently rerun here. This does not prove each individual capture is fully
  complete. The branch is not in production; real CAD parity, hosted required-check behavior
  and comparable cold performance remain open.
- 2026-10-29: One local `python3 scripts/check.py lint test` run at
  `perf/native-ci-topology@90411c08` passed (exit 0) in 400.8s: lint 135.6s and test 265.1s.
  The parallel test lanes reported workspace 133.1s, footprints 27.6s and CAD 265.0s. The native
  guard confirmed 2,104 baselined tests; CAD reported 514 passed and 2 ignored, with seven
  nonzero selections. Target directories were absent before the run, but registry and Git caches
  were warm. Defaults were 12 Cargo jobs and 4 scheduler jobs; the workflow's
  `CARGO_PROFILE_DEV_DEBUG=line-tables-only` environment setting was not set. Source: local run
  handoff and gitignored log at
  `/home/chris/.t3/worktrees/ts-boardstudio2/t3-84998151/.evidence/native-critical-path-90411c08-vj2r0nxA.log`.
  This is not a same-head CI comparison, real CAD name-parity proof, 10-minute cold full-gate
  proof, or proof of capture completeness. The ticket remains claimed and unresolved; no speedup
  is claimed.
- 2026-10-29: Independent analysis of the `90411c08` log reconstructs nine CAD Cargo invocations
  with 514 passed and 2 ignored, including zero-test library and native-WASM doc-test sections.
  The log does not explicitly record the nine nested command boundaries or numeric exits. A test
  occurs in two qualification selections, so merging by binary and test name loses an occurrence.
  Existing receipts also exclude doc-tests. A fresh unsharded reference with per-invocation,
  fail-closed evidence is required before split parity can be established; instrumentation is
  under investigation. The log is local diagnostic evidence, not authenticated CI provenance.
- 2026-10-29: `perf/native-ci-topology@866e104a` adds opt-in framed per-invocation CAD
  observations; `@3017199c` hardens failure handling. An independent re-review found observed
  frames can reconcile while an entire expected binary is missing: `inventory_verified` remains
  false, so accepted receipts are provisional, not complete parity references. It also identified
  baseline-update publication, cancellation and receipt-pair limitations. The handoff reports
  18 CAD evidence, 16 CAD driver, 39 scheduler, 15 quick-planning, 38 native guard and 42 receipt
  hermetic tests passing after hardening; these were not independently rerun for this entry.
- 2026-10-29: Static manifest and Cargo-source analysis predicted 14 binary and two doctest
  launches across the nine invocations at `3017199c`; this is a prediction, not a hardcoded
  inventory or verified reference. An isolated toy project with Rust 1.98.0 and its own target
  exercised Cargo offline: default and explicit selectors, zero doctests, name filters,
  required features, `test = false`, `harness = false`, a test-enabled example and renamed
  targets. It found metadata omits the harness setting, explicit selectors override test/doctest
  flags, a default name filter suppresses the doctest header, and source-path headers can be
  ambiguous across packages. Captures used separate stdout and stderr; cross-stream ordering was
  not established. A synthetic removal of one complete launch showed why observed frames alone
  are insufficient; it did not exercise the repository parser. The toy and captures are at
  `/home/chris/.cache/tmp/opencode/cad-cargo-inventory-vsd1tee9`; offline mode was requested but
  network isolation was not independently instrumented. No real CAD reference or parity is proven.
  Any inventory implementation must validate its selection, configuration and identity assumptions,
  reject ambiguous or unsupported cases, and keep observed-frame completeness distinct from
  verified launch inventory.
