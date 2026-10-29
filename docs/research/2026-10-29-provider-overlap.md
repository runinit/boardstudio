# Research 02 (ci-wall-time): Can the six provider builds overlap?

Date: 2026-10-29. Read-only investigation; no product code changed.
Question: can the six serial provider builds in `prepare_runtime_providers`
(`scripts/build-web.py`) run concurrently, and what wall-time saving does that predict?

## Verdict

**CONDITIONAL GO: three lanes, no target-dir isolation. The `dx build` page step
stays after the provider batch.**

- **GO:** overlap the batch as (A) the four `wasm-pack build` invocations kept
  serial on the shared workspace target dir, (B) `cad/scripts/build-cad-wasm.py`
  concurrently, (C) ergogen staging concurrently. Predicted saving **~90–110 s**
  off a ~624 s cold build (~15% of the build portion), from batch
  ~313 s to critical path ~214 s.
- **NO-GO:** overlapping the four `wasm-pack` builds with each other on one
  target dir (Cargo's target-dir lock serializes them anyway — zero gain, added
  failure modes), per-lane `CARGO_TARGET_DIR` isolation on CI (cold duplicate
  dependency compilation plus cache-config changes — likely net loss), and
  moving the `dx build` page compile ahead of the provider batch (packaging
  provably needs all providers; compile-time independence is unproven — see §5).
- Ticket 05 must confirm the numbers on real CI hardware before any
  implementation (§7).

## 1. The six commands: inputs, outputs, timing

`provider_commands` (`scripts/build-web.py:39-63`) returns six `(command, cwd)`
pairs; `prepare_runtime_providers` (`scripts/build-web.py:186-191`) runs them
strictly serially, then copies `imported-modules.json` and calls
`stage_runtime_assets`. Per-command cold timings below are **as stated in the
ticket** (green run `40791505198`, build 624.3 s total); ticket 05 must
re-measure both `--fast` and shipped paths (§7, item 1).

| # | Command (scripts/build-web.py) | Writes to | Cargo target dir | Cold |
|---|---|---|---|---|
| 1 | `wasm-pack build web … --out-name m1_core_worker --out-dir web/target/providers/core-worker … --features core-worker` (:45-47) | `web/target/providers/core-worker/` | workspace `target/` (shared) | ~162 s |
| 2 | `wasm-pack build web … --out-name m1_cad_worker --out-dir web/target/providers/cad-worker … --features cad-worker` (:48-50) | `web/target/providers/cad-worker/` | workspace `target/` (shared) | ~11 s |
| 3 | `wasm-pack build web … --out-name boardstudio_offline_worker --out-dir web/target/providers/offline-worker … --features service-worker` (:52-54) | `web/target/providers/offline-worker/` | workspace `target/` (shared) | ~15 s |
| 4 | `wasm-pack build renderer … --out-dir pkg --out-name boardstudio_renderer_wasm` (:55-56), cwd = repo root so `pkg` = `renderer/pkg/` | `renderer/pkg/` | workspace `target/` (shared) | ~26 s |
| 5 | `cad/scripts/build-cad-wasm.py [--no-opt]` (:57-58) → `wasm-pack build wasm … --out-dir pkg --out-name boardstudio_cad_wasm` with `CARGO_TARGET_DIR=cad/wasm/target` (`cad/scripts/cad_build.py:118-140`, env at :120) | `cad/wasm/pkg/` + stamp `cad/.cache/cad/wasm-build.sha256` | `cad/wasm/target/` (isolated) | ~99 s (`--no-opt`) |
| 6 | `scripts/stage-ergogen-models.py --source-root ergogen/library/vendor --destination web/assets/ergogen-models --manifest web/target/providers/ergogen-models.json` (:59-61) | symlinks/copies in `web/assets/ergogen-models/` + manifest json | none (no cargo; symlinks or copies, `scripts/stage-ergogen-models.py:56-80`) | seconds (ticket 05 to measure) |

Feature flags proving commands 1–3 are three builds of the **same** crate with
different features: `web/Cargo.toml:18-24` (`core-worker`, `cad-worker`,
`service-worker`, `page`). Workspace membership proving commands 1–4 share one
target dir: `Cargo.toml:3-25` lists `renderer`, `web`, and the other workspace
crates as members; `cad/wasm` is explicitly excluded (`Cargo.toml:21-24`) with
its own lockfile — consistent with its isolated `CARGO_TARGET_DIR`.

**No two commands write the same output directory.** The three worker out-dirs
are siblings, `renderer/pkg` and `cad/wasm/pkg` are in different crates, and
the ergogen destination/manifest touch no cargo output. Shared *parents*
(`web/target/providers/`, `web/assets/`) only gain disjoint children
(`mkdir -p` semantics; staging even refuses to touch unrecognized files,
`scripts/stage-ergogen-models.py:37-46`). Shared mutable caches
(`~/.cache/.wasm-pack`, cargo registry) are read-mostly after first fill and
are already shared by today's serial runs (CI caches the former,
`.github/workflows/check.yaml:49-53`).

## 2. Contention mechanism: one Cargo build per target dir

Commands 1–4 all shell out (via `wasm-pack`) to `cargo build` in the **same
workspace with the same target directory** (repo-root `target/`;
`run()` passes `env=None` so no `CARGO_TARGET_DIR` override,
`scripts/build-web.py:31-33`; CI's rust-cache maps `. -> target`,
`.github/workflows/check.yaml:55-61`).

Cargo manages the whole target directory under
`target/<platform>/<profile>` with **a single lock**: concurrent builds block
on each other rather than interleave. Primary sources:

- Cargo layout docs (`cargo::compiler::layout`): `.cargo-lock` is the
  "file used to lock the directory to prevent multiple cargo processes from
  using it at the same time" (both `artifact-dir` and `build-dir` layouts).
- Inside Rust blog, "This Development Cycle in Cargo: 1.93" (2026-01-07),
  Target-dir-locking section: "the entire target directory underneath
  `target/<platform>/<profile>` is managed by a single lock to avoid
  concurrent builds from negatively impacting each other… being blocked on
  another build" (e.g. CLI `cargo check` waiting on rust-analyzer).
- In-repo precedent: `cad/scripts/cad_build.py` deliberately uses **three
  separate** `CARGO_TARGET_DIR`s for its three workspaces
  (`cad/wasm/target`, `monstertruck/target`,
  `monstertruck-qualification/target` — `:120`, `:142`, `:154`, `:166`), and
  `cad/scripts/test-cad-build.py` pins that isolation. The repo's own
  scheduler only parallelizes *independent workspaces* (`test` step) and
  browser launches, never two builds on one target
  (`scripts/check.py:472-475`); repo guardrails forbid overlapping
  cargo/wasm-pack on the same target directory (`AGENTS.md`, build guardrails).

Consequence: running commands 1–4 concurrently on the shared target dir does
**not corrupt** the cache (same-target locking is sound; the known historical
gap was cross-*target-triple* locking, `rust-lang/cargo#5968`, which does not
apply — all four build `wasm32-unknown-unknown`), but it **does not speed up**
either: lock-wait serializes them back into sequence, with extra process
churn. Overlap is only real where target dirs (or build tools) differ.

Isolation option examined and rejected for CI: `wasm-pack build --help`
(wasm-pack 0.15.0, verified locally) exposes **no `--target-dir` flag**; the
only cargo passthrough is `[EXTRA_OPTIONS]… List of extra options to pass to
cargo build`. Per-invocation isolation would therefore ride on `CARGO_TARGET_DIR`
(the proven `cad_build.py:120` pattern) or an untested `-- --target-dir`
passthrough. Either way each fresh target dir recompiles the shared dependency
closure from scratch on a cold runner, CI's rust-cache only warms
`. -> target` and `cad/wasm -> target`
(`.github/workflows/check.yaml:55-61`, `:99-105`), and 4–5 lanes ×
`jobs = 12` (`.cargo/config.toml:1-4`) oversubscribe the runner. Ticket 05 may
bound this experimentally on a large dev machine, but it is not the proposal.

## 3. Safe overlap design (no isolation, no new caches)

Three lanes; within a lane, keep today's order:

- **Lane A — shared workspace target, stays serial (~214 s cold):**
  core-worker (~162) → cad-worker (~11) → offline-worker (~15) →
  renderer (~26). All four hold the same target-dir lock; internal order does
  not change the lane total.
- **Lane B — CAD WASM (~99 s):** already fully isolated
  (`CARGO_TARGET_DIR=cad/wasm/target`, own lockfile/workspace). Runs
  concurrently with lane A with zero lock interaction. Its stamp short-circuit
  (`cad/scripts/cad_build.py:124-129`) is a pure win on warm runs.
- **Lane C — ergogen staging (seconds):** no cargo at all; reads the
  read-only vendor tree (`ergogen/library/vendor`, 151 MB), writes only its
  own destination + manifest. Concurrent with A and B.

Everything after the batch stays serial and unchanged: the
`imported-modules.json` copy + `stage_runtime_assets`
(`scripts/build-web.py:189-191`; entry.js synthesis for both workers,
`:94-108`) and then the whole page step (`build()`:
`prepare_runtime_providers` → `dx build` → `copy_runtime_assets` →
`embed_offline_worker`, `:156-183`).

Predicted batch wall time: `max(214, 99, ~5) ≈ 214 s` vs serial
`162+11+15+26+99 ≈ 313 s` (+ staging). **Saving ≈ 90–110 s** (range covers
staging time, wasm-opt scheduling jitter, and lane-B CPU contention with lane
A's 12 rustc jobs). Total cold build ≈ 624 s → **≈ 515–535 s (~15% off)**.

## 4. Baseline arithmetic closes

Serial sum with ticket timings: providers 313 s + page 311 s = 624 s, matching
the reported 624.3 s cold total for run `40791505198`
(`https://github.com/runinit/boardstudio/actions/runs/40791505198`) to within
staging/manifest overhead — i.e. the build step is ~50% providers, ~50% page,
so the provider batch is the right first target and fully overlapping it is
worth at most ~313 s even in theory.

## 5. Why the `dx build` page step stays after the batch

`build()` order is prepare-providers → `dx build --web` → `copy_runtime_assets`
→ `embed_offline_worker` (`scripts/build-web.py:160-181`). The two packaging
functions provably consume **all** provider outputs:

- `copy_runtime_assets` (`:70-89`) copies `web/assets`, `imported-modules.json`,
  `cad/wasm/pkg`, `web/target/providers/{core-worker,cad-worker}`, and
  `renderer/pkg` into the site, then synthesizes both worker `entry.js` files.
- `embed_offline_worker` (`:129-153`) needs the offline-worker build
  (`OFFLINE_WORKER`, `:18`) plus the finished site to hash the manifest.

What is *not* proven is whether the `dx` compile itself reads the staged
providers. Evidence cuts both ways: provider URLs are runtime fetches, not
compile-time embeds (`assets/renderer/…` fetched at runtime,
`web/crates/host/src/renderer_host.rs:46`; stylesheets are runtime `href`s,
`web/src/presentation.rs:621-623`; ergogen model paths are runtime URL strings,
`web/crates/runtime/src/bundled_models.rs:248`; `dev-web.py` proxies `/assets`
from the live tree rather than from `dx` output, `:37-66`), which suggests the
compile *could* overlap — but `stage_runtime_assets` deliberately populates
`web/assets` *before* `dx build`, and the duplicate full-tree copy in
`copy_runtime_assets` (`:73-74`) leaves open that `dx` bundles `web/assets`
into `public/`. **Do not pipeline the `dx` compile ahead of providers without
the §7-item-4 experiment.** Upside if proven (follow-up work, needs a
`build()` restructure): up to ~300 s additional.

## 6. What ticket 05 must measure to confirm

1. **Per-command cold times, both paths**, on a cold CI runner
   (`actions/runs` log of a clean run): each of the six commands separately
   for `build-web.py --fast` (4× `--no-opt` pin,
   `scripts/test-build-web.py:32-40`) and shipped (with `wasm-opt`), plus
   staging time and `nproc`/memory of the runner. The ticket's blended numbers
   (e.g. cad-wasm `--no-opt` 99 s next to shipped-path others) are not a
   like-for-like baseline.
2. **Lock-wait ≈ 0 check:** run the 3-lane prototype (threads, one lane per
   §3) and assert wall ≈ `max(lane A, lane B, lane C)` ± 5% across two cold
   runs; any excess is lock contention or CPU starvation to root-cause.
3. **No shared writes:** `strace`- or manifest-level audit that concurrent
   lanes create no file outside their §1 out-dirs (plus the already-cached
   `~/.cache/.wasm-pack`); confirm `web/assets` staged copies remain
   untracked-but-unignored (only `**/target/`-matched paths are ignored today)
   so parallel staging does not change `git status` behavior.
4. **H1/H2 test for the follow-up:** `dx build` with providers staged in
   `web/assets` vs with staging absent; diff `target/dx/…/public/`. Identical
   ⇒ compile is provider-independent and a pipelined follow-up is viable;
   different ⇒ the page step must keep waiting for the batch.
5. **Warm-run behavior:** with warm caches the batch shrinks (notably the
   cad-wasm stamp skip, `cad_build.py:124-129`) and absolute savings shrink
   with it — report warm numbers separately so CI-cache-hit runs are not
   misread.

## Appendix: pairwise overlap matrix

| Pair | Share target? | Share out-dir? | Safe concurrently? |
|---|---|---|---|
| core × cad × offline workers | yes (`target/`, same crate, different features) | no (sibling out-dirs) | **No gain** — target lock serializes; keep serial |
| any worker × renderer | yes (renderer is a workspace member, `Cargo.toml:3-25`) | no | **No gain** — same lock; keep serial in lane A |
| any of 1–4 × cad-wasm | no (`cad/wasm/target`, own workspace+lockfile) | no | **Yes** (lane B) |
| any of 1–5 × ergogen | no cargo involved | no | **Yes** (lane C) |
| any provider × `dx build` compile | same `target/` for page profile (`target/dx/…`, `dx_public`, `:24-27` — disjoint profile dir, but see §5) | packaging reads all outputs | **No** — packaging dependency proven; compile independence unproven |
