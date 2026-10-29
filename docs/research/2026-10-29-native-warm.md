# Native warm wall and budgets

Research for [ci-wall-time 14](../plans/ci-wall-time/issues/14-native-warm-wall.md), 2026-10-29.
It breaks down the first warm `native` run, sets per-step budgets under the 17-minute PR-gate
wall, and gives a verdict. Product code was not changed and no CI was triggered.

## Sources

| Tag | Run | What it is |
| --- | --- | --- |
| W | [run 40806171071 attempt 4, job 123247523854][W] | Warm, green. Draft [PR #11][PR11], probe head `008cb78f`, exact-key cache hit. |
| C | [run 40806171071 attempt 1, job 123225531784][C] | Cold transition, green, same head. Rust-cache miss, then save. |
| B | [run 40791505198, job 123177581080][B] | Cold baseline, green, head `d650e1ddf`. |

Job walls and step walls come from the Actions jobs API (`started_at`/`completed_at`). Command
walls come from the `scripts/check.py` log lines, cited by their log timestamp (UTC, 2026-10-29).
The per-step summary is the `check: step times:` line: W at `03:50:22.4053708Z`, C at
`02:53:37.7966931Z`. B's logs return HTTP 403 to this account, so B's check.py step numbers are
the ones recorded in the [map Notes](../plans/ci-wall-time/map.md) and the
[baseline wiki](../plans/ci-wall-time/wiki/baseline.md). B's job and step walls come from the API.

## Job-level breakdown

| Step | Warm W | Cold C | Cold B | Source |
| --- | ---: | ---: | ---: | --- |
| Job wall | **881 s (14m41s)** | 1762 s (29m22s) | 1819 s (30m19s) | jobs API |
| Set up job + checkout | 5 s | 6 s | 9 s | steps 1–2 |
| Install KiCad 10 (apt PPA, uncached) | 26 s | 27 s | **74 s** | step 3 |
| `rustup toolchain install` | 10 s | 11 s | — (B predates the step) | step 5 |
| rust-cache restore | 19 s (hit) | 1 s (miss) | 13 s | step 6 (B: step 5) |
| `check.py repo tooling lint test` | 818 s | 1694 s | 1719 s | step 7 (B: step 6) |
| Post rust-cache (save) | 0 s (exact hit, no save) | 21 s | 0 s | step 13 (B: 11) |

Cache bytes and times:

- W restore: `Cache hit for: v0-rust-native-Linux-x64-f1c5bde9-40f4f1b0` (`03:36:26.4163809Z`).
  The 1,573,641,708 B download (`Cache Size: ~1501 MB`) took about 6.6 s at 228 MB/s
  (`03:36:26.42` to `03:36:33.0247548Z`). Extraction ended at `Cache restored successfully`
  (`03:36:43.9149292Z`), about 10.9 s later. The restore step took 19 s in total, and the restore
  was a `full match: true` (`03:36:44.1842483Z`).
- C save: cleaning the four target dirs ran from `02:53:38.88` to `02:53:41.33`. Tar and zstd
  ran until the first `Sent` line at `02:53:54.5470933Z`, about 13.2 s. The 1,573,641,708 B
  upload took about 3.6 s (`02:53:58.1263814Z`). The post step took 21 s in total.
- C restore: `No cache found.` (`02:25:23.1351479Z`).

## check.py breakdown

`check.py` runs its four steps one after another. Inside `test` it runs three lanes at once, and
`test` ends when the slowest lane ends. The test-lane output is buffered, so timestamps show when
each lane flushed its output, not when each line ran.

| Step / command | Warm W | Cold C | Cold B | W log line |
| --- | ---: | ---: | ---: | --- |
| **repo** | **4.3 s** | 3.5 s | 3.5 s | step-times line |
| **tooling** | **41.0 s** | 139.6 s | 143 s | step-times line |
| ↳ `test-import-kicad-parts.py` (dev build of core) | 32.9 s | 133.4 s | — | `03:37:28.0229706Z`; cargo `Finished … 32.58s` at `03:37:27.8014331Z`; C `02:27:44.4106363Z` |
| ↳ the other ~25 tooling tests together | 8.1 s | 6.2 s | — | derived |
| **lint** | **132.0 s** | 475.4 s | 477 s | step-times line |
| ↳ `cargo fmt --all --check` | 6.9 s | 6.7 s | — | `03:37:36.8114683Z`; C `02:27:53.0530202Z` |
| ↳ native clippy `--workspace --all-targets` | 49.5 s | 215.7 s | — | `03:38:26.3295903Z`; C `02:31:28.7470613Z` |
| ↳ WASM clippy (web packages, wasm32) | 75.7 s | 253.0 s | — | `03:39:41.9857946Z`; C `02:35:41.7847815Z` |
| **test** (wall = slowest lane) | **640.4 s** | 1076.0 s | 1095 s | step-times line |
| ↳ lane: `cargo test --workspace --exclude footprints` | 426.0 s | 840.2 s | — | `03:46:50.1898562Z`; C `(840.2s)` |
| &nbsp;&nbsp;compile (`Finished test profile`) | 264 s (4m24s) | 689 s (11m29s) | — | `03:46:48.2433639Z` |
| &nbsp;&nbsp;test binaries run (sum of `finished in`) | 133.9 s | 130.2 s | — | core `tests/integration.rs` 895 tests 122.28 s at `03:46:50.1689100Z` |
| ↳ lane: `cargo test -p boardstudio-footprints` | 50.5 s | 120.2 s | — | `03:46:50.2387528Z` |
| &nbsp;&nbsp;compile / run | 8.41 s / 41.4 s | 56.0 s / 63.2 s | — | `tests/generators.rs` 36.81 s |
| ↳ lane: `cad/scripts/test-cad.py` (**critical path**) | **640.4 s** | 1076.0 s | — | `03:50:22.4052586Z` |
| &nbsp;&nbsp;compile (9 release cargo invocations, sum) | 526.8 s | 903.3 s | — | lines below |
| &nbsp;&nbsp;test binaries run (sum of `finished in`) | 110.1 s | 168.0 s | — | `boardstudio_cad` lib 42.52 s, `boardstudio_cad_wasm` lib 62.98 s |
| **check.py total** | **817.7 s** | 1694.5 s | — | step-times line |

The warm CAD lane runs nine release `cargo test` invocations, all flushed at `03:50:22`
([`cad_build.test_cad`](../../cad/scripts/cad_build.py)):

| # | Invocation | Target dir | Warm `Finished` | Cold `Finished` |
| --- | --- | --- | ---: | ---: |
| 1 | `monstertruck/Cargo.toml` (library) | `cad/monstertruck/target` | 3m45s (`.3456143Z`) | 6m01s |
| 2 | `wasm/Cargo.toml` (native tests of the WASM crate) | `cad/wasm/target` | 2m06s (`.3815723Z`) | 4m17s |
| 3 | qualification `-p monstertruck-io --test input` | `cad/monstertruck-qualification/target` | 1m14s (`.3881705Z`) | 2m58s |
| 4 | qualification `-p monstertruck-io --lib` | same | 33.70s (`.3917442Z`) | 32.41s |
| 5–6 | qualification `-p monstertruck-io --test input` ×2 | same | 0.13s each | 0.13s each |
| 7 | qualification `-p monstertruck-meshing --lib` | same | **1m02s** (`.3994303Z`) | 1m09s |
| 8 | qualification `-p monstertruck-meshing --test` | same | 5.75s (`.4016892Z`) | 5.52s |
| 9 | qualification `-p monstertruck-meshing --lib` | same | 0.08s | 0.08s |

### What the cache does and does not save

| Compile | Cold C | Warm W | Saved |
| --- | ---: | ---: | ---: |
| Native clippy | 215.7 s | 49.5 s | 77 % |
| WASM clippy | 253.0 s | 75.7 s | 70 % |
| Tooling dev build of core | 133.4 s | 32.9 s | 75 % |
| Workspace test compile | 689 s | 264 s | 62 % |
| Footprints test compile | 56.0 s | 8.4 s | 85 % |
| **CAD lane compile** | **903.3 s** | **526.8 s** | **42 %** |

The cache saves much less on the CAD lane than anywhere else. In the warm CAD lane, Cargo
recompiled every path crate: the vendored `monstertruck-{derive,core,traits,mesh,geometry,modeling,meshing,io}`
under `cad/monstertruck-qualification/vendor`, `cgmath` under `renderer/vendor`, and
`boardstudio-contracts`. It also recompiled the registry crates that `[patch]` puts downstream of
those path crates: `matext4cgmath` and `monstertruck-{assembly,topology,fillet,healing}`. The
warm log shows these `Compiling` lines after `$ python3 cad/scripts/test-cad.py`
(`03:50:22.3430766Z`).

That vendored stack is compiled four times per run. Invocations 1, 2 and 3 each build it in a
different target dir. Invocation 7 builds a second variant inside the qualification target.
Rust-cache runs with `cache-workspace-crates: false` (W config block, `03:36:25`). The likely
cause is Cargo's mtime-based freshness check for path sources after a fresh checkout, combined
with rust-cache dropping local crates. This cause is **not confirmed**: these logs carry no
fingerprint trace.

The root workspace has the same cascade on a smaller scale. Patching `cgmath` makes `three-d-asset`
and `three-d` rebuild in clippy (`03:37:37.8358087Z`) and in the test build.

**Workspace members always rebuild.** Warm runs still compile `boardstudio_core` and every web
crate in clippy, test and tooling. A warm run's cost therefore barely depends on which source
files a PR touches: a core-touch PR and a docs-only PR should take about the same native time.
This is an inference from the rebuild list above, not a measurement.

## Budgets

Each budget is a ceiling per step, based on W plus margin. If every step lands exactly on its
ceiling, the job takes 963 s (16m03s), still 57 s under the 1020 s wall. W itself finished in
881 s, 139 s (2m19s) under the wall.

| Step | Warm actual | Budget (ceiling) | Margin | Note |
| --- | ---: | ---: | ---: | --- |
| Set up + checkout | 5 s | 10 s | 5 s | |
| KiCad install | 26 s | 45 s | 19 s | B took 74 s; apt variance alone can exceed this |
| rustup install | 10 s | 15 s | 5 s | |
| rust-cache restore | 19 s | 25 s | 6 s | 1.47 GiB at about 228 MB/s plus about 11 s extraction |
| repo | 4.3 s | 8 s | 3.7 s | |
| tooling | 41.0 s | 50 s | 9 s | `test-import-kicad-parts.py` ≤ 40 s |
| lint | 132.0 s | 150 s | 18 s | fmt ≤ 10, native clippy ≤ 55, WASM clippy ≤ 85 |
| test | 640.4 s | 660 s | 19.6 s | CAD lane ≤ 660; workspace lane ≤ 480; footprints ≤ 70 |
| **Job total** | **881 s** | **963 s** | — | 57 s under 1020 s at full budget |
| rust-cache save (only on key change) | 0 s | +25 s | — | C took 21 s; at full budget plus a save the job takes 988 s |

**The test step is the only step with almost no margin.** The CAD lane sets the test wall: its
640.4 s equals the step's 640.4 s. The workspace lane is about 214 s shorter. So anything added
to the CAD lane, whether CAD tests or vendored-crate changes, lands directly on the job wall.
Anything added to the workspace lane is absorbed until it uses up those 214 s.

## Verdict

**Native fits.** A warm PR whose Cargo manifests are unchanged since the base cache was saved
runs in about 14m41s. That holds whether the PR touches core or not, because workspace members
rebuild either way. Even with every step at its budget, the job ends 57 s under 17 minutes.

Watch items, which can push native over the wall:

1. **Cache prefix change means a fully cold run (about 29m22s, 12m22s over the wall).** The prefix
   hash covers the Rust version (`rust-toolchain.toml`) and the env vars `CARGO_INCREMENTAL`,
   `CARGO_PROFILE_DEV_DEBUG`, `ImageOS` and `RUSTUP_HOME` (W config block). Any toolchain bump
   or change to one of those env vars makes every PR cold until a new base cache is saved.
2. **Missing base cache means a cold run.** C restored nothing (`No cache found.`). A PR reads
   only its own scope and its base branch's scope. Until [12](../plans/ci-wall-time/issues/12-land-cache-change.md)
   lands the cache change on `dev` and a `dev` push saves a cache, every new PR's first run is
   cold.
3. **Lockfile or manifest churn means a partial restore plus a 21 s save.** The cache key hashes
   31 manifests and lockfiles: the workspace members' `Cargo.toml` files, all four `Cargo.lock` files,
   `.cargo/config.toml` and `rust-toolchain.toml`. If any of them changes, the run restores from
   the prefix and rebuilds the changed dependencies. A small dependency bump stays near warm. A
   large bump, such as dioxus, wasm-bindgen or serde, moves toward cold. That cost was not
   measured here. A save adds about 25 s, which still fits at full budget (988 s).
4. **Cache quota eviction means a cold run.** [03](../plans/ci-wall-time/issues/03-warm-baseline-measure.md)
   counted about 8.56 GiB across 10 caches against the 10 GB limit. Every same-repo PR whose key
   misses saves about 1.47 GiB for native plus about 1.41 GiB for web into its own scope. With two
   or three such PRs open at once, the least recently used entry is evicted. If that entry is
   `dev`'s, new PRs start cold.
5. **KiCad apt variance.** KiCad installs from the PPA on every run and is not cached. It took 26
   to 27 s on the probe but 74 s on B. A 48 s swing uses about a third of the 139 s headroom.
   There is no separate "KiCad reinstall" risk: the install already happens on every run.
6. **CAD lane growth.** New CAD tests, or changes to vendored monstertruck, add directly to the
   critical path. The test step has 19.6 s of budget margin.
7. **Flaky re-runs.** Attempts 2 and 3 failed `native` in tooling after about 1 minute
   (`test-serve-web.py`, per 03). A flake does not lengthen any single wall, but every re-run
   costs another full wall.

## Follow-ups (optional; native fits without them)

Neither ticket is required to reach 17 minutes. Graduate them if watch item 6 trips, or if the
web split brings `web` under `native` and more PR-gate margin is wanted. Each fits in one session.

### F1: Stop the vendored CAD stack recompiling on a warm cache

- **Evidence:** the CAD lane compile only drops from 903.3 s to 526.8 s with a warm cache, and the
  vendored stack builds four times per run (tables above).
- **Work:**
  1. Reproduce locally. Build `cad/monstertruck` release tests, then simulate a fresh checkout:
     a new worktree with the target dir copied in. Run with
     `CARGO_LOG=cargo::core::compiler::fingerprint=info` and record why each path crate is
     marked dirty.
  2. Apply the fix that has no freshness hazard: one shared `CARGO_TARGET_DIR` for the three CAD
     workspaces if their resolved features and profiles allow sharing, plus `cache-workspace-crates`
     for those dirs if Cargo's mtime check is the cause.
  3. Reject the mtime-rewrite tricks (`touch`, `git-restore-mtime`). A PR restores `dev`'s cache,
     which may be built from a newer commit, so a PR's older but different vendored file could
     look fresh. Content-hash freshness (`-Zchecksum-freshness`) is nightly-only, and the
     toolchain is stable 1.98.0 (W config block).
- **Acceptance:** on a same-head cold→warm pair (the [PR #11][PR11] re-run pattern), the warm CAD
  lane is ≤ the workspace lane, or saves ≥ 120 s. Editing one vendored file in a probe commit
  must still rebuild it. Test counts must be unchanged.
- **Expected gain:** up to about 210 s, the gap between the CAD and workspace lanes. Beyond that
  the workspace lane becomes the critical path. That would put native at about 11 minutes; this
  is an estimate.
- **Do not** probe with `workflow_dispatch`: the workflow's release gate runs `release-build` and
  `release-deploy` on dispatch.

### F2: Collapse the qualification owner-test invocations

- **Evidence:** invocation 7 (`-p monstertruck-meshing --lib`) recompiles the vendored stack in
  the same target dir that invocation 3 already built: 1m02s warm (`03:50:22.3994303Z`), 1m09s
  cold. Building `-p monstertruck-io` and `-p monstertruck-meshing` separately probably resolves
  different features; this is not confirmed.
- **Work:** cut the seven `OWNER_TESTS` cargo calls down to one build with a single feature
  resolution. Keep the per-selection guard that rejects a selection with no executed tests. Check
  that unifying features does not hide a feature-gating bug that a per-package build would catch.
- **Expected gain:** about 60 s on the critical path. F1 makes this unnecessary if it shares the
  target and caches both variants.

[W]: https://github.com/runinit/boardstudio/actions/runs/40806171071/job/123247523854
[C]: https://github.com/runinit/boardstudio/actions/runs/40806171071/job/123225531784
[B]: https://github.com/runinit/boardstudio/actions/runs/40791505198/job/123177581080
[PR11]: https://github.com/runinit/boardstudio/pull/11
