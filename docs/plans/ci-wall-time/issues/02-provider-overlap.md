# 02: Provider-build overlap

Status: resolved
Type: research
Parent: [CI wall time map](../map.md)

## Question

Can the six serial provider builds in `prepare_runtime_providers` (`scripts/build-web.py`) overlap
safely, and what saving does that predict against the cold 624s build (core-worker ~162s,
cad-wasm ~99s, rest ~50s, `dx` page ~311s)?

## Pointers

- `provider_commands` in `scripts/build-web.py` (three `wasm-pack build` providers, renderer,
  `cad/scripts/build-cad-wasm.py`, ergogen staging).
- `.cargo/config.toml` (`jobs = 12`) versus the 4-vCPU CI runner; shared target-dir lock behavior.
- Green cold log per-build walls in `.evidence/web-ci-green-baseline-d650e1dd/receipt.md`.
- Constraint: test counts unaffected (build-only change), but overlapping Cargo invocations must
  not corrupt the shared target dir.

## Answer

Conditional GO for three-lane overlap with **no** target-dir isolation: keep the four
`wasm-pack` builds (core ~162s, cad ~11s, offline ~15s, renderer ~26s) serial in lane A — they
share the workspace `target/` dir where Cargo's lock serializes concurrent builds anyway (zero
gain, added churn). Run the CAD WASM build (~99s, already isolated at `cad/wasm/target`) and
ergogen staging (no cargo, disjoint outputs) concurrently in lanes B and C. The `dx build` page
step stays after the batch (`copy_runtime_assets`/`embed_offline_worker` consume all provider
outputs); whether the `dx` compile itself reads staged `web/assets` is unproven — that H1/H2
test plus cold re-measurement of both `--fast` and shipped paths is specified for ticket 05.
Per-lane `CARGO_TARGET_DIR` isolation is NO-GO on CI (cold duplicate dep compilation, cache
config warms only `. -> target`, 4–5x12-job oversubscription).

Predicted saving: **~90–110s** off the ~624s cold build (batch 313s → critical path ~214s,
~15%), assuming ~zero lock-wait between isolated lanes and staging under ~10s; warm-run savings
shrink with caches and must be reported separately.

Full evidence: `docs/research/2026-10-29-provider-overlap.md` on branch
`research/provider-overlap` (pushed, no PR).

## Comments

- Claimed by the charting session for read-only research delegate
  `node:delegated-task:command%3Amcp%3A4a120a31-63bc-4bf2-95d6-d041fca261b3%3Adelegate-task%3A056289f0-5928-4beb-9e80-5300f7a455c8`
  (findings go to `docs/research/2026-10-29-provider-overlap.md` on branch
  `research/provider-overlap`, pushed, no PR).
