# 17: Native topology and critical path

Status: needs-triage
Type: research
Parent: [CI wall time map](../map.md)
Related: [14: Native warm wall](14-native-warm-wall.md), [04: Web-split design](04-web-split-design.md)

## Ownership and scope

2026-10-29: The chief-of-staff/coordinator owns triage and handoff for this follow-up; no
implementation delegate is claimed here. Check active ownership and build activity before
claiming or using either existing worktree: `research/native-cad-artifact-reuse` and
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
