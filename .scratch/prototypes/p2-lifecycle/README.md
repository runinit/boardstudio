# P2 lifecycle feasibility probe

Status: **accepted within the isolated P2 scope**, source `e5d4e748` on
`migration/boardstudio-m1-continuation`. The [current run](../../../docs/migration/continuation-run.json)
records authorization and integration. Production M1 and full application parity
remain unfinished; prototype adoption requires deliberate review.

The copied Reviung41 probe exercises the Rust core worker, Dioxus 0.7.10 gesture
presentation, and the existing WebGL renderer. Public Rust payload boxing keeps
JSON, saved documents, and generated TypeScript unchanged. Renderer disposal
breaks the program/context ownership cycle and releases a departing canvas;
resource imports resolve against the current deployment origin.

[Gate evidence](evidence/continuation/gates.json) and
[fresh reviews](evidence/continuation/review-final.json) cover native/WASM checks,
reference builds/tests, exact provider provenance, root/subpath lifecycle,
keyboard/history, focus, DPR, compact layout, context loss and late imports.
Axe reports zero violations and one incomplete overlapping-label contrast check;
computed contrast is at least 7.41:1. Screen-reader speech and overlapping-label
readability were not established. Measurements do not create new performance
thresholds or a full UI accessibility claim.

From the repository root, rebuild both providers and both deployments with a
new output identifier:

```sh
python3 .scratch/prototypes/p2-lifecycle/wasm/build-release.py <unique-run-id>
```

The builder records source/tool/asset identities. Staging rejects changed
provider inputs, including embedded files. The final accepted shell is recorded
in [shell-final4.json](evidence/continuation/shell-final4.json), using verified
final3 provider assets. Run `wasm/tests/browser-acceptance.mjs` with the served
URL, a new evidence directory, and the other deployment URL for cross-navigation.
Keep failed runs and their outputs.

The [original charter](PLAN.md) and [earlier checkpoints](evidence/renewal/task-status.json)
retain the failed attempts. Their preflight blocks and finite repair budgets are
historical: the user authorized their fixes and removed repair caps. No React
retirement, production adoption, push, deployment, or worktree deletion occurred.
