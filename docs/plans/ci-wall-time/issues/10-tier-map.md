# 10: Tier map driven by rules

Status: resolved
Type: research
Parent: [CI wall time map](../map.md)

> **Superseded for execution 2026-10-29:** This resolved research and its tier assignments are
> historical evidence, not authorization to reduce PR coverage or remove tests. The full-gate
> goal in the [map](../map.md#full-gate-destination-and-sequence--2026-10-29) retains all checks.
> The truncated findings and provisional browser pins below remain limitations, not implementation
> instructions.

## Question

Which tests belong in the PR gate and which in a slower tier, assigned by explicit rules that
need no judgment calls — including the rule that picks the pinned cross-crate smoke set?

## Pointers

- Input: 2,104 native tests under the count guard plus all browser suites with their cold walls
  (repo 3.5s, tooling 143s, lint 477s, test 1095s, build 624s, browser 1363s).
- Rules (agent-applyable): every test explicitly assigned, untiered stays in the PR gate; the PR
  gate keeps build, lint, affected-crate checks, drag smoke, and a smoke set chosen by a stated
  rule (e.g. one fastest test per crate that kills distinct mutants — state the rule, don't
  hand-pick).
- `check.py quick` as PR-gate engine: set the diff base to the PR base SHA (not `main` — a
  stacked diff against `main` selects nearly everything), fix the checkout depth so the base
  exists, and resolve the build conflict explicitly (`quick` skips builds while the gate keeps
  build — say where build runs).
- Name the reference change sets the gate is measured against: a canonical small
  matrix-keys-only PR and a worst-case core-touch PR. State the predicted PR-gate wall for both —
  this is the number the 17-minute destination depends on.
- Full suite stays required on every push to `dev`/`main` and on `workflow_dispatch`; tier
  only on `pull_request`; keep job names stable for `release-build: needs` and required checks.
- Deliverable: the tier map plus predicted walls, and the redundancy list with mutation
  evidence that 11 consumes — so 09 and 11 implement without deciding anything.
## Answer

Tier map and redundancy list produced — full evidence in
`docs/research/2026-10-29-tier-map.md` on branch `research/tier-map` (pushed, no PR).
Predicted PR-gate walls: reference set A (small matrix-keys-only PR, 2 affected packages)
~20 min cold / **~12 min warm** (vs 33.1 / 22–27 full); reference set B (worst-case core-touch
PR, 15 packages) ~29 min cold / ~full wall warm — core-touch PRs get almost no tiering benefit
(gate stays lint-bound at 477s plus build-bound at 624s cold, both in the gate by rule).
Redundancy list: **1 proven item** (`polygon::tests::vertices_follow_the_same_rule`, covered by
`boundary_points_follow_the_half_open_rule`, mutant M1 kill proven with vertices skipped) plus 3
investigated-but-NOT-redundant items 11 must not remove (agreement-style tests are blind to
common-mode mutants) and static candidates for 11's expanded sampling. Limits: 4 mutants sampled
in 1 of 18 packages (`boardstudio-contracts`); core's 1,517 tests unsampled; zero browser runs, so
browser smoke pins are placeholders; walls are arithmetic over cold walls plus cited warm anchors,
not new measurements — 09 confirms same-head on hosted CI.

## Comments

- 2026-10-29: the committed findings doc is truncated at §5 (file ends with a truncation
  marker) — §§6–8 (reference-set details, provisional pins, mutation evidence) are missing
  from git. 09 implemented from §§3–5 plus the ticket Answer without blocking. Recovery, in
  order: (a) read the 10 delegate's thread transcript for the full text; (b) regenerate the
  redundancy list during 11, which needs the mutation evidence anyway.
