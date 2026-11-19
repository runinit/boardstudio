# Cold build: audit and port plan

Audited 2026-11-18 against `dev` at `2ce123403`. The aim is that a fresh worktree's `python3 scripts/check.py` stops taking about 1,000 s, with every gate and count floor kept. No filesystem tricks: no reflink seeding and no shared target.

Inputs:
- Sol's full cold run at `2ce123403` (1,010.7 s), in that worktree's `.evidence/shared-wasm-test-build/full.log`.
- Measurements taken today on an idle machine (load under 1 at the start), listed under each cause.
- The unmerged branch `perf/cold-build-lanes` (39 commits). Its plan lives on that branch in `docs/plans/cold-build-time/`. It cut a cold run from 1,273.8 s to 762.0 s on an older base.

## Where the time goes on `dev`

| Step | Wall | What is in it |
|---|---:|---|
| repo, tooling | 79 s | Serial Python tests. `test-import-kicad-parts.py` takes 29.7 s, because it compiles a native driver. |
| build | 177 s | Six builds one after another: the Core worker (release wasm, 76 s), the CAD worker, the offline worker, the renderer, the CAD wasm (36 s), then the `dx` page. |
| lint | 152 s | `fmt`, then three Clippy runs one after another: native 36.6 s, wasm32 76.7 s, harness 34.1 s. |
| test | 281 s | Workspace tests (148 s), footprints and `test-cad.py` already run at once. `test-cad.py` is the long pole: three CAD trees in series, with about 245 s of release compiles. |
| browser | 322 s | Drag smoke 98.7 s, eight gates 53.4 s, crate test build 0.5 s, 14 crate launches 67.4 s, page suite 102.2 s. |

Only `browser` needs `build` to finish first, yet the steps never overlap.

The 1,010.7 s figure is low for a truly fresh worktree. In that run, the browser crate test build and the page test binary were already compiled (0.5 s and 0.15 s). Compiling them cold adds about 25 to 60 s.

## Causes, one at a time

Gains are estimates unless marked measured. "Old" means a ticket from `perf/cold-build-lanes`.

| # | Cause | Evidence | Fix | Source | Expected gain |
|---|---|---|---|---|---|
| 1 | Each browser launch re-runs `wasm-bindgen` over a huge debug test binary | The page test wasm is 145 MB (84 MB function names, 56 MB code), `layout` is 104 MB and `host` is 56 MB. Measured `wasm-bindgen` time: page 11.1 to 15.6 s, `layout` 11.7 s, `host` 6.6 s. A one-test page launch took 16.6 s, and 15.7 s of that passed before Chrome started. There are 18 launches per run. | Run fewer passes in series (rows 2 and 3). If split launches still cost much after that, cache the bindgen output per binary in the patched runner the repo already builds for CI. | New | See rows 2 and 3 |
| 2 | The page suite runs its launches one after another | 102.2 s total for 23.5 s of test bodies: one `--list` launch (4.5 s, measured; it parses the whole module) and three filter launches of about 14 s overhead each | Run the three filters at once, keeping report order deterministic | Old 17 part 2 (`403e13a69`, `2145e8630`) | About −50 s on `browser` |
| 3 | `wasm-bindgen` oversubscribes the CPU | Measured on the page wasm: by default 23.7 s user plus 22.7 s system time for 11.1 s wall; with `RAYON_NUM_THREADS=1`, 11.7 s wall on 11.6 s of CPU. Four heavy crates at once: 10.8 to 11.0 s by default, 9.7 s on one thread each. | Set `RAYON_NUM_THREADS=1` in `runner_environment` (`scripts/run-wasm-tests.py:243`), which every launch uses | New | About −35 CPU-s per page launch. Wall is about the same alone, and the freed cores help everything running alongside. |
| 4 | The drag smoke compiles its harness inside `browser` | 41 s pass between the smoke's start and its browser opening. Measured cold harness build: 34.3 s, plus 3.1 s for the other harness binaries. | Build every harness binary early, beside other work. It has its own workspace and target directory, so it takes no lock that the root build needs. | Old 06.2 (`445a1cbb7`), reworked | About −35 s on `browser` |
| 5 | The browser test binaries compile inside `browser` on a fresh worktree | Hidden in the 1,010.7 s run because they were warm | Prebuild the crate tests (Sol's shared build) and the page test binary during the compile steps | Old 17 part 1 (`05e2dabb5`), redone on Sol's build | About −25 to −60 s on a fresh worktree |
| 6 | The CAD tree compiles run one after another | About 245 s of release compiles in series inside `test-cad.py` | Drop `regex` from the CAD trees, merge the library and wasm workspaces, then run the trees as parallel lanes | Old 07, 20 and 04 | Test step from 281 s to about 150 s; −300 to −500 CPU-s |
| 7 | The provider and page builds run one after another | The Core worker alone takes 76 s before anything else starts | Overlap the page and provider builds | Old 05 (`7102a22d0`), hand-ported | About −85 s on `build` |
| 8 | The steps run in series on a 16-CPU machine | lint, test and build add up to 610 s of serial wall | Run them at once on 8 or more CPUs, sharing one jobserver capped at the CPU count. Give `lint` its own target directory, because Cargo locks a whole target directory. | Old 19 (`5ca187739`, `88dc66963`) | Old branch measured −107 s; it is CPU-bound, so expect less once rows 6 and 7 land |
| 9 | Tooling compiles a native driver | 29.7 s in the serial tooling step | Move the KiCad parts test into `test`, and build its driver with the workspace test features so it shares Core | Old 06.3 (`59542cea1`, `c8b0ef432`, `88db7ddbb`) | About −25 s |
| 10 | The harness Clippy waits for the root Clippys | 34.1 s at the end of `lint` | Run it beside them | Old 06.1 (`183545277`) | Old branch measured −24 s on `lint` |
| 11 | Core always compiles `wasm-bindgen` and `js-sys` | Unconditional in `core/Cargo.toml` | Make them wasm32-only | Old 11, redone on the split Core | About −4 s, −20 CPU-s |
| 12 | A rare page-load stall costs up to 300 s | The runner waits for the page's `load` event with a timeout of `max(300 s, test timeout)`. The old branch hit about 300 s stalls under contention. | `pageLoadStrategy: "eager"` | Old 17 (`bf6ba7288`) | Guard only. Measured no change on an idle machine (16.6 to 16.9 s either way). |
| 13 | A fresh worktree compiles every registry crate once | About 1,700 CPU-s on the old branch's kit | Machine-wide `sccache` 0.19.0, approved on 2026-11-18 | Old 12 | About −290 s on a new worktree. Registry crates only: 0.19.0 still hashes the working directory and `CARGO_MANIFEST_DIR` for Rust, so worktree crates never hit across worktrees. |

## What today's measurements ruled out

- **Stripping the name section.** It shrinks the page wasm from 145 to 59 MB but cuts bindgen only from 11.1 to 9.9 s, and failure summaries lose wasm frame names.
- **Harness without debug info.** It cuts its cold build only from 34.3 to 31.7 s.
- **A faster linker.** Rust 1.90 made `rust-lld` the default on `x86_64-unknown-linux-gnu`, and wasm32 always uses it. `mold` would add little.

## The browser harness: do we need it?

`tools/browser-harness/` is a standalone Cargo workspace on `chromiumoxide`, a Chrome DevTools Protocol client. It has nine binaries. The drag smoke needs things nothing else here provides:
- trusted pointer drags with pointer capture;
- response bodies from the page and its workers;
- dedicated and service worker sessions.

The CAD binaries add downloads, screenshots and worker timing.

**Alternatives considered:**
- **The test runner, `wasm-bindgen-test-runner`.** It only runs tests inside the page, so it cannot make trusted input.
- **Plain WebDriver through the pinned `chromedriver`.** It can make trusted drags. Network and worker capture need WebDriver BiDi over a WebSocket, which Python's standard library lacks.
- **A thin hand-written CDP client.** It would drop the generated protocol types.

Each of the last two means rewriting nine checks for about 35 s, and the harness was written in Rust to remove Node and Playwright.

**Decision:** keep the harness, but take its build off the critical path.
- **What it costs.** A cold build measured 39.2 s, and 25.5 s of that is one registry crate, `chromiumoxide_cdp`.
- **What the build depends on.** It has no root-workspace dependencies. It embeds only its own CAD measurement scripts.
- **What changes:**
  - `sccache` caches `chromiumoxide_cdp` across worktrees.
  - The rest compiles early, beside other compile work. It has its own target directory, so it takes no root lock.
- **Not doing:** caching the finished binaries. `cad-capture` compiles in `CARGO_MANIFEST_DIR` to find a fixture at runtime, so a copied binary would read another worktree's fixture.

## Cargo locks, as they affect overlap

Cargo locks each `target/<triple>/<profile>` directory, plus the host directory of the same profile. It does not lock the whole target directory.

| Work | Lock it takes | Conflicts with |
|---|---|---|
| Four release `wasm-pack` providers | root `target/wasm32-unknown-unknown/release` and `target/release` | each other only |
| `dx` page and both browser test builds | root `wasm-check` dirs | each other |
| Root Clippy and the native tests | root `target/debug` | each other, so `lint` moves to `target/lint` |
| CAD wasm build | its own `CARGO_TARGET_DIR` | nothing in `test`, once it keeps its own directory after ticket 20 |
| Harness build and harness Clippy | `tools/browser-harness/target` | each other |

## The old branch's commits

Dry-run cherry-picks onto `dev` with `git merge-tree`. Many conflicts are only the plan docs, which do not exist on `dev`; bringing `docs/plans/cold-build-time/` over first clears those.

| Commits | Ticket | Applies cleanly? | Verdict |
|---|---|---|---|
| `7aefe4ce1`, `10bb7dac4` | 02, measurement kit | Docs and kit conflicts only | Take first. It gives real CPU per rustc unit for a fresh-worktree baseline. |
| `403e13a69`, `2145e8630`, `bf6ba7288`, `f82c10263` | 17, 16 | Clean | Take. Only measure `f82c10263` (6 jobs) after row 3 lands. |
| `5f2078dab`, `bd82bfc4a` | 07 | Qualification lockfile only | Take; regenerate that lock offline |
| `ba82951a2`, `88dc66963` | 20 | `check.yaml`, `cad_build.py`, CAD locks, seeding script | Take, dropping the seeding edits. Read the lock diff in review. |
| `a634279f3`, `7db1d9106` | 04 | `cad_build.py` | Take after 20 |
| `7102a22d0` | 05 | `build-web.py` | Hand-port: `dev` added the `wasm-check` page profile and `--wasm-js-cfg false` |
| `183545277`, `445a1cbb7`, `59542cea1`, `c8b0ef432`, `88db7ddbb` | 06 | `check.py`, `test-check.py`; the last two clean | Hand-port. Build the harness in the compile steps rather than in the crate batch. |
| `05e2dabb5` | 17 prebuild | `check.py` | Redo on Sol's shared crate build |
| `5ca187739`, `165061bf1` | 19 | `check.py`, `test-check.py` | Hand-port last; it is the largest `check.py` change |
| `675d9189f` | 11 | `core/Cargo.toml`, `core/src/lib.rs` | Redo on the split Core |
| `da651f484` and round one | 01 | — | Skip. `dev` did most of it another way (`a88019ef6`, the `wasm-check` profile). |
| `62a274623` | single browser prebuild | — | Skip; Sol's `2ce123403` supersedes it |
| `5d2b30c78`, `65376d318` | 03, seeding | — | Skip: ruled out |

## Upstream status (checked 2026-11-18)

- **rustc's parallel front end** is still nightly-only. Nightly is only now starting to enable it by default ([2026 goal](https://rust-lang.github.io/rust-project-goals/2026/parallel-front-end.html), [Phoronix](https://www.phoronix.com/news/Rust-Nightly-Parallel-Frontend)). This repo pins stable 1.98, so it is out of reach for now. It would help most on the serial crate chain.
- **Fine-grained target-directory locking** is still unstable ([cargo#16155](https://github.com/rust-lang/cargo/pull/16155)). On stable, Cargo locks a whole target directory, so overlapping steps need separate target directories, as row 8 does.
- **Cargo's cross-workspace cache** is a 2026 goal, nightly first, with no build scripts or proc macros in its first milestone ([goal](https://rust-lang.github.io/rust-project-goals/2026/cargo-cross-workspace-cache.html)). It is not usable yet. It may replace `sccache` later.
- **`sccache` basedirs** (`SCCACHE_BASEDIRS`) are documented in 0.15 ([Configuration.md](https://docs.rs/crate/sccache/0.15.0/source/docs/Configuration.md)). Rust support is not stated there, so caching path crates across worktrees is still unconfirmed. Registry crates are cacheable without it.
- **A shared jobserver** across concurrent Cargo processes is the standard way to cap total parallelism ([GNU make job slots](https://www.gnu.org/s/hello/manual/make/Job-Slots.html), [Gentoo](https://blogs.gentoo.org/mgorny/?p=2439)). Tools outside the protocol are not counted: here that means `wasm-bindgen`, which is another reason for row 3, plus Chrome and the test binaries.
- No upstream issue was found for the per-launch bindgen cost or the 300 s page-load stalls.

## Implementation plan

Decided on 2026-11-18:
- **Testing waits until every fix is in.** Each commit gets only its quick checks: the self-tests of the scripts it touches, and `cargo metadata`/`cargo tree --locked --offline` for manifest or lockfile changes. There is no baseline run, no per-fix timing and no `check.py` run between commits.
- **`sccache` goes machine-wide**, outside the repo.
- **One implementer works through the items in order** on branch `perf/cold-build-port`, cut from `dev` at `2ce123403`.

Each item is one commit. "Old" names the `perf/cold-build-lanes` commits to start from.

### A. Record and kit

1. **Plan docs.** Bring `docs/plans/cold-build-time/` over from the branch as the record, plus this file. Mark at the top of that `map.md` that porting status lives here.
2. **Measurement kit** (old `7aefe4ce1`, `10bb7dac4`, `35116597c`), without seeding:
   - Leave out `seed-worktree-target.py` and the seed tests.
   - Fix three things in `report.py`: add `wasm-check` to the profile buckets, sum real CPU rather than unit elapsed time, and attribute units correctly when steps overlap.
   - Make the wipe list match today's outputs, including the offline worker and the CAD paths after item C2.
   - Chain the unit logger to the `sccache` wrapper when one is configured.
   - Add `test-build-time.py` to `TOOLING_TESTS`.

### B. Browser step

1. **The page suite runs the runner directly.**
   - **Build first:** `run-wasm-tests.py` builds the page test binary once, using the same flags as `WASM_PACK` today: `wasm-check`, locked, no default features, `page`, `--bin boardstudio-web`, plus `--no-run --message-format=json`.
   - **Then run it directly:** listing and filters call `wasm-bindgen-test-runner` on that artifact, with the environment Sol's crate launches already use, instead of `wasm-pack test`.
   - **Keep the outputs the same:** `browser.json`, the per-filter logs, the known-failure and owner handling, and the duplicate checks.
   - **Keep manual debugging:** the `wasm-pack test` instructions stay in the README.
2. **One `wasm-bindgen` thread per launch.** Set `RAYON_NUM_THREADS=1` only in the runner's own process environment, for crate launches (`check.py` around line 1102) and page launches. It must not reach Cargo or rustc.
3. **Page filters at the same time** (old `403e13a69`, `2145e8630`), adapted to the direct launches.
   - Pool width comes from `check_jobs()`. On the branch, a setting of 2 still allowed 3.
   - Report order stays deterministic.
4. **Eager page load** (old `bf6ba7288`). It must cover generated crate configs and page configs. A caller's own capabilities still win.
5. **Harness built once, before the browser checks.**
   - Add a command that builds every harness binary the plan runs. It must use exactly the flags, features and target of `browser_tools._build_harness`, so the later per-gate builds are no-ops.
   - `check.py browser`, `drag-smoke` and `browser-packaged` must still work alone.
6. **The page suite beside the crate launch batch.** The page filters share the `check_jobs()` budget with the crate launches. If the final run shows stalls or flakes from this, revert only this commit.
7. **CPU-aware jobs default** (old `f82c10263`): `min(6, CPUs)`.

### C. CAD

1. **Ticket 07** (old `5f2078dab`, `bd82bfc4a`): vendor `Inflector` with defaults off and patch all three CAD workspaces.
   - Regenerate the qualification lock offline with minimal movement.
   - Show that `cargo tree -i regex` is empty in all three trees.
2. **Ticket 20** (old `ba82951a2`, plus the CAD parts of `88dc66963`): merge `cad/wasm` into the `cad/monstertruck` workspace.
   - **Drop** the seeding edits.
   - **Keep the wasm build's own target dir:** `build_wasm` keeps `CARGO_TARGET_DIR=cad/wasm/target`, so the `build` step does not lock against the `test` step's native CAD compile. The native tests of both packages use `cad/monstertruck/target`.
   - **Lockfile versions:** keep existing versions where possible, using `cargo update --precise --offline`. Allow no non-patch movement in geometry crates. Keep `parry3d-f64 =0.31.1`, `spade 2.15.1` and `glam 0.33.11`. List every moved package in the commit message.
   - **Update every reference:** `check.yaml` cache entries in all three jobs, `security-audit.py` and its test, `check.py` preflight, `test-ci-workflow.py`, Dependabot, `cad/.dockerignore`, `measure-cad.py`, and the comments in the root `Cargo.toml` and `deny.toml`.
3. **Ticket 04** (old `a634279f3`, `7db1d9106`): run two CAD test lanes, the combined production tests and the ordered owner selections.
   - Every lane's failure output must print, including the zero-tests error.
   - `BOARDSTUDIO_CHECK_JOBS=1` stays serial.

### D. Build step

1. **Ticket 05** (old `7102a22d0`), on `dev`'s `build-web.py`, for fast single-route builds only. Three lanes run at once:
   - lane A, the four release `wasm-pack` providers, one after another;
   - lane B, the CAD wasm;
   - lane C, the `dx` page in `wasm-check`.

   Ergogen staging runs first. Join the lanes before copying provider assets or embedding the offline worker. Shipped and multi-route builds stay serial.

### E. Overlap

1. **KiCad parts test into `test`** (old `59542cea1`, `c8b0ef432`, `88db7ddbb`). Quick runs must still select it.
2. **Harness Clippy beside the root Clippys** (old `183545277`).
3. **Browser prebuild during the compile steps.** After `build-web.py --fast`, the `build` step also runs the crate test build (`WASM_CRATE_BUILD` through `with_crate_build`), the page test build (B1) and the harness build (B5). In the browser step, the same commands are then no-ops that still return the artifact paths. A failed prebuild must stop the step, never run stale binaries.
4. **Ticket 19** (old `5ca187739`, the rest of `88dc66963`, `165061bf1`). `lint`, `test` and `build` run as child `check.py` processes under these rules:
   - **Jobserver:** one fifo jobserver with `CPUs - 1` tokens.
   - **Lint target:** `CARGO_TARGET_DIR=target/lint`.
   - **When it applies:** on 8 or more CPUs, with `check_jobs() > 1` and no `--receipt`. Hosted, quick and receipt plans stay serial.
   - **Output and failures:** output prints in plan order, and every failure is reported. `--update-native-baseline` goes to `test` only.

### F. Core

1. **Ticket 11**, redone on the `boardstudio_core` facade. The wasm-only dependencies go in a `wasm32` target table, with `cfg` on the JS-only code. Native APIs stay. The worker `.d.ts` files must stay byte-identical.

### G. `sccache`, machine-wide

1. **Install 0.19.0** to `~/.cargo/bin` with `cargo binstall`.
2. **Wrap it so a build can't fail because of the cache.** Set `~/.cargo/config.toml` `[build] rustc-wrapper` to a small fail-open wrapper. Read that file first and keep a backup. The wrapper must:
   - run the compiler uncached whenever the server is unreachable, so sandboxed agent shells with a private network still build;
   - skip the cache when the workspace wrapper is `dx`, unless `dx build` and `dx serve` hot-patching are shown to work through it;
   - run the server as a user service, with `SCCACHE_DIR` under `~/.cache/sccache` and a 30 GB cap.
3. **Verify end to end:** plain `cargo`, `cargo clippy`, `wasm-pack`, `dx build`, and a build with the server stopped. Write down the undo steps.

### H. Final verification: the only heavy testing

1. **Self-tests:** every touched script's self-tests.
2. **Full check:** the full `python3 scripts/check.py` in the implementation worktree must pass every gate, with the count floors:
   - the same crate-launch test names;
   - 126 page tests;
   - every baselined native test, plus all executed names in the retained H2 log
     after applying the deliberate splits and renames, including CAD;
   - 15 drag checks;
   - the CAD browser checks.

   Find any failure by running single steps. If a failure appears only under overlap, revert that item; don't retune it.
3. **Fresh-worktree cold runs.** Make a new disposable worktree at the branch head and use the kit to run twice:
   - once with `sccache` warm from H2;
   - once with the wrapper bypassed.

   Report the step-times lines, total wall and real CPU, against the 1,010.7 s baseline.
4. **Equivalence checks:**
   - the page wasm is identical between serial and overlapped builds;
   - the CAD wasm size before and after;
   - every lockfile change, listed.

Tooling runs three times during H, once in the H2 full gate and once in each
H3 cold run. If any of those tooling runs fails, rerun the step serially under
the build lock with `BOARDSTUDIO_CHECK_JOBS=1 python3 scripts/check.py tooling`.
If the failure does not reproduce serially, revert J8.

### Afterwards, on data only

These trade one cost for another, so each gets its own A/B after H:
- **`opt-level = 1` for workspace crates in `wasm-check`.** Smaller test binaries mean faster bindgen, Chrome, tests and drag smoke, but the web crates compile more slowly.
- **Narrow `dev`'s `"*" = { opt-level = 2 }`.**
- **Tooling tests in parallel,** with a flake sweep.
- **A bindgen cache per binary in the patched runner.**

Expected result: about 550 to 700 s for a cold run with no cache hits. A new worktree with `sccache` warm should go lower. H3 measures both.

## Implementation record

### G1. Cache installation

Installed `sccache 0.19.0` in `~/.cargo/bin/sccache` using `cargo binstall`
with the pinned version and `--no-track` to preserve Cargo's installation registry.

### G2. Cache service and wrapper

The user unit `boardstudio-sccache.service` puts its Unix socket at
`$XDG_RUNTIME_DIR/boardstudio-sccache/server.sock` and limits
`~/.cache/sccache` to 30 GiB. It uses sccache client-side mode so Rust
compilation on cache misses runs in Cargo's process tree. The wrapper tries the
service only for non-incremental Rust compilation. It calls `dx`, Clippy's
driver, and other workspace wrappers directly. It also calls rustc directly
if the socket is unavailable or sccache itself fails, which it signals with exit
status 2. Any other status is the compiler's own and is returned as is, so a
failed compile is never run twice or its diagnostics repeated. Set
`BOARDSTUDIO_SCCACHE_BYPASS=1` to bypass the wrapper's cache path for a run.

The machine configuration files were absent before setup. Their prior-state
record is `/home/chris/.cache/tmp/boardstudio-sccache-before.SUHur1Em/prior-state`.
To undo the setup, remove the four files below. Disabling the service first
removes its enablement symlink and stops its process.

```sh
systemctl --user disable --now boardstudio-sccache.service
rm -- ~/.cargo/config.toml ~/.local/bin/rustc-sccache-fail-open \
    ~/.config/systemd/user/boardstudio-sccache.service ~/.cargo/bin/sccache
systemctl --user daemon-reload
```

The cache directory can be removed separately after stopping the service.

### G3. Cache verification

Locked native Cargo build and Clippy passed with the service running. A fresh
target built while the service was stopped. The renderer's locked wasm-pack
build and the checked Dioxus page build also passed. The first Dioxus probe
exposed a retry hazard for rustc inputs from stdin. The wrapper now bypasses
those invocations and the stable compiler rejects the deliberately unstable
stdin probe. The Dioxus build passed after cleaning the affected target.

### H1. Focused self-tests

All 358 tests in 14 focused suites passed, including the real KiCad importer
driver build. The measurement control attributes two simultaneous rustc units
to their own steps and sums recorded CPU independently of compiler wall time.

### H2. Full gate

```text
check: step times: repo 6.3s, tooling 49.3s, build 113.6s, lint 82.6s, test 121.3s, browser 193.9s, total 370.9s
check: native tests: all 2384 baselined test(s) executed; 27 new, record with --update-native-baseline
```

All 14 crate launches matched the baseline's 799 test names and
multiplicities. The page suite matched all 126 names, drag smoke passed all
15 checks and the CAD browser gate matched all 364 check labels. The warm
full run followed two corrected failures. A live CAD lockfile link needed
the new workspace path. The page runner initially served assets from the
wrong directory, which failed the same renderer-import test in isolation.
The runner now starts in the page package, and both the isolated test and
the full check pass. The vendored crate retains its lint checks without
adding lint suppressions.

## I. Follow-up implementation

### I1. Share the Wasm test feature set

Keep the crate test build first and build only the page binary in the second
invocation. The full locked, offline Wasm trees match on all 224 distinct
package and feature rows after removing Cargo's repetition markers and blank
lines. The page package is the only extra row. Runtime and Case already had
`test-support` in both invocations. Host's added feature exposes the raw
IndexedDB fixture writer and its private helper.

### I2. Run independent native fixture cases through libtest

Split the STEP, keycap legend and footprint golden checks by asset, roof and
legend, and generator fixture respectively. The two CAD tests already spawned
threads internally. Libtest now schedules those cases alongside other tests
with its own thread budget. Fallback tests still cover newly added STEP and
registered generator fixtures. The commit records every old and new name.
Only the footprints names enter the current native baseline. The CAD tests
run through their separate production lane.

### I3a. Rejoin the UI shared panel launch

Restore the resize fixture's localStorage value and browser spies before
asserting its observations. The scoped mount owns the Dioxus render loop
and removes the root when it is disposed. The panel tests now run in the
ordinary UI shared launch.

### I3b. Rejoin the layout setup guide launch

Keep the 980 px viewport assertion. Query controls and panel IDs under the
guide test's own root, dispose both guide mounts, and restore the fixture's
panel and guide preference keys.

### I3c. Rejoin the Case workspace page filter

Scope the two previously leaking tests to their roots and dispose the VDOMs
and roots of all four Case workspace mounts. Keep the page runner's manual
isolation option. The default and quick plans no longer request it, and the
full launch reserves two page slots for its two depth-one filters.

### I4. Wait for named layout states

Wait for the mounted SVG, focus, menu, hover, anchor preview and drag-toast
states. The live expiry test still exercises the production timeout. The two
unmount tests use a short test-only timer and let its callback proceed once
the overlay is gone. Both wait for the callback to finish before inspecting
the panic log. The key-size test waits for its new owner and retains the
negative observation window for a late stale submission. The nine settlement
helpers now share one bounded driver and preserve the Case suite's synchronous
effect drain. The remaining short gesture waits and the launch-order stopping
point need browser evidence at H.

### I5. Link the footprints suites together

The 12 named suites now run as modules of `footprints_integration`. The
explicit target name keeps their native receipts separate from Core's
`integration` target. The footprint library harness and three other
integration binaries keep their target names. The suites remain in place so
fixture paths are unchanged, and the manual capture tests keep their ignore
attributes and environment requirements. The commit message lists every
binary and test name mapping.

### Section H commands after the VM is off

Run H1, H2 and H4 from the implementation worktree using a shell outside the
sandbox. Use the same shell for the lock and all commands it protects.

```bash
set -euo pipefail
root=$(git rev-parse --show-toplevel)
cd "$root"
evidence="$root/.evidence/cold-build-port"
reference=/home/chris/.t3/worktrees/ts-boardstudio2/t3-d04ee7db-eda77b8bc79b8ab2
mkdir -p "$evidence"
exec 9<~/.cache/tmp/boardstudio-build.lock
flock 9
export BOARDSTUDIO_BUILD_LOCK_FD=9
```

#### H1. Focused self-tests

```bash
for suite in scripts/test-{build-time,check,check-quick,run-wasm-tests,build-web,dev-web,browser-tools,ci-workflow,parallel-interrupt,verification-receipt,check-wasm-tests}.py \
    scripts/security-audit.test.py cad/scripts/test-{cad-build,measure-cad}.py; do
    env -u BOARDSTUDIO_BUILD_LOCK_FD python3 -B "$suite" \
        > "$evidence/H1-I5-${suite##*/}.log" 2>&1
done
python3 -B scripts/test-import-kicad-parts.py > "$evidence/H1-I5-import-kicad.log" 2>&1
python3 -B scripts/test-check-core-tests.py > "$evidence/H1-J-test-registration.log" 2>&1
python3 -B scripts/test-check-old-css.py > "$evidence/H1-J-test-old-css.log" 2>&1
```

#### H2. Record deliberate name changes, then run the full gate

```bash
python3 scripts/check.py test --update-native-baseline > "$evidence/H2-I5-native-update.log" 2>&1
git --no-pager diff -- scripts/check-native-tests-baseline.json | tee "$evidence/H2-I5-native-baseline.diff.log"
python3 - <<'PY'
import json
from pathlib import Path
import subprocess

baseline = "scripts/check-native-tests-baseline.json"
before = json.loads(subprocess.check_output(["git", "show", f"HEAD:{baseline}"]))
after = json.loads(Path(baseline).read_text())
i5 = subprocess.check_output(
    ["git", "log", "-1", "--format=%B", "--grep=^I5 Link the selected footprints"], text=True)
renames = dict(line.split(" -> ") for line in i5.splitlines()
               if " -> footprints_integration::" in line)
i2 = subprocess.check_output(["git", "show", "-s", "--format=%B", "ee3680cd0"], text=True)
old_golden = "generators::bundled_generators_match_the_golden_baseline"
golden = i2.split(old_golden + " ->\n", 1)[1].splitlines()
replacements = {old_golden: [renames[name] for name in golden if name]}

expected = set()
for binary, names in before.items():
    for name in names:
        old = f"{binary}::{name}"
        expected.update(replacements.get(old, [renames.get(old, old)]))
actual = {f"{binary}::{name}" for binary, names in after.items() for name in names}
assert not expected - actual, sorted(expected - actual)
print(f"{len(expected)} expected native names retained or migrated. {len(actual)} names recorded.")
PY
python3 scripts/check.py > "$evidence/H2-I5-full.log" 2>&1
rg '^check: (step times:|native tests:|all steps passed:)|^PASS drag browser smoke: 15 checks$' "$evidence/H2-I5-full.log"
python3 - "$reference" "$root" "$evidence/H2-I5-full.log" <<'PY' | tee "$evidence/H2-I5-inventories.log"
from collections import Counter
import json
from pathlib import Path
import re
import sys

reference, root, full_log = map(Path, sys.argv[1:])
wasm_launch = re.compile(r"/wasm-bindgen-test-runner \S*/boardstudio_web_(\w+?)-[0-9a-f]+\.wasm")
wasm_test = re.compile(r"^test (\S+) \.\.\. ok$")

def inventory(log):
    crates, cad = Counter(), Counter()
    launches = 0
    crate = None
    in_cad = False
    for line in log.read_text().splitlines():
        if line.startswith("$ "):
            match = wasm_launch.search(line)
            crate = match[1] if match else None
            launches += bool(match)
            in_cad = line == "$ python3 cad/scripts/test-cad-browser.py"
        elif crate and (match := wasm_test.match(line)):
            crates[crate, match[1]] += 1
        elif in_cad and line.startswith("ok   "):
            cad[line.removeprefix("ok   ")] += 1
    return launches, crates, cad

before_launches, before, before_cad = inventory(reference / ".evidence/shared-wasm-test-build/full.log")
after_launches, after, after_cad = inventory(full_log)
assert (before_launches, after_launches) == (14, 12)
assert before.total() == 799 and before == after, (before - after, after - before)
assert before_cad.total() == 364 and before_cad == after_cad, (before_cad - after_cad, after_cad - before_cad)
results = Path("web/target/test-results/browser.json")
page = json.loads((root / results).read_text())
base_page = json.loads((reference / results).read_text())
assert page["complete"] and page["passed_count"] == 126 and len(page["filters"]) == 2
assert len(page["expected_tests"]) == len(page["listed_tests"]) == len(page["terminal_outcomes"]) == 126
assert Counter(page["listed_tests"]) == Counter(base_page["listed_tests"])
assert page["duplicate_terminal_outcomes"] == page["duplicate_listed_tests"] == page["problems"] == []
assert all(item["status"] == "passed" for item in page["terminal_outcomes"])
assert "PASS drag browser smoke: 15 checks" in full_log.read_text()
print("799 crate test names across 12 launches, 126 page tests, 15 drag checks, and 364 CAD browser labels.")
PY
```

Review the JSON diff with the user and record the generated baseline on the
branch before H3. If H2 fails, run the failing step alone
(`python3 scripts/check.py <step>`, adding `--only <owner>` where supported).
Revert the corresponding I3 commit if its merged launch alone introduces an
order-dependent failure. Compare crate tests by crate and name multiplicity
after the revert, using the resulting number of launches. Review the
layout crate launch duration against the next-longest crate launch before
making further I4 changes.

Compare executed native names with the retained pre-I H2 log as well as the
committed floor. Include the CAD production and owner lanes from the
`test-cad.py` output. I2 and I5 supply the mappings below. Batch J leaves
test names alone.

```bash
PYTHONPATH="$root/scripts" python3 -B - "$evidence/H2-full.log" "$evidence/H2-I5-full.log" <<'PY' \
    | tee "$evidence/H2-J-native-names.log"
from collections import defaultdict
from pathlib import Path
import re
import sys

from check_native_tests import collect

def executed(log):
    outputs = defaultdict(list)
    for block in re.split(r"^\$ ", log.read_text(), flags=re.M)[1:]:
        command, _, output = block.partition("\n")
        if command.startswith("cargo test ") and "--no-run" not in command.split():
            outputs["workspace"].append(output)
        elif command == "python3 cad/scripts/test-cad.py":
            outputs["CAD"].append(output)
    assert len(outputs["workspace"]) == 2 and len(outputs["CAD"]) == 1, log
    inventory = {}
    for lane, blocks in outputs.items():
        names = collect(blocks).executed
        if lane == "CAD":
            assert names.get("boardstudio_cad") and names.get("boardstudio_cad_wasm"), log
        inventory[lane] = {f"{binary}::{name}" for binary, tests in names.items() for name in tests}
    return inventory

plan = Path("docs/plans/cold-build-port.md").read_text()
mapping = plan.split("\n### Exact native test name mappings\n", 1)[1].split("\n## J.", 1)[0]
i2, i5 = re.findall(r"```text\n(.*?)\n```", mapping, re.S)
renames = dict(line.split(" -> ") for line in i5.strip().splitlines())
splits = {}
for group in i2.strip().split("\n\n"):
    old, *names = group.splitlines()
    old = old.removesuffix(" ->")
    prefix = "boardstudio_cad_wasm::" if old.startswith("model::") else ""
    splits[prefix + old] = {prefix + name for name in names}

before, after = (executed(Path(path)) for path in sys.argv[1:])
assert splits.keys() <= (before["workspace"] | before["CAD"]), "I2 mapping does not match the retained log"
missing = False
for lane, names in before.items():
    expected = {renames.get(name, name) for old in names for name in splits.get(old, {old})}
    absent = sorted(expected - after[lane])
    print(f"{lane}: {len(names)} before, {len(expected)} expected after mappings, {len(after[lane])} executed")
    for name in absent:
        print(f"{lane}: missing {name}")
    missing |= bool(absent)
sys.exit(int(missing))
PY
```

#### H3. Warm-cache and cache-bypassed cold runs

Use a fresh, detached worktree at the revision with the reviewed baseline.
Leave it in place. Run each cold command once machine load is below the kit's
threshold.

```bash
verify="$(dirname "$root")/cold-build-port-i5-verification"
git worktree add --detach "$verify" HEAD
(
    cd "$verify"
    mkdir -p .evidence/cold-build-port
    sccache_socket="${XDG_RUNTIME_DIR:-/run/user/$(id -u)}/boardstudio-sccache/server.sock"
    test -S "$sccache_socket"
    SCCACHE_SERVER_UDS="$sccache_socket" ~/.cargo/bin/sccache --show-stats \
        > .evidence/cold-build-port/H3-I5-sccache-before.log
    python3 scripts/build-time/cold.py H3-I5-warm
    SCCACHE_SERVER_UDS="$sccache_socket" ~/.cargo/bin/sccache --show-stats \
        > .evidence/cold-build-port/H3-I5-sccache-after.log
    python3 scripts/build-time/cold.py H3-I5-bypass --without-cache
    python3 scripts/build-time/report.py H3-I5-warm H3-I5-bypass --lanes \
        | tee .evidence/cold-build-port/H3-I5-report.log
    rg '^(check: step times:|cold: wall)' .evidence/cold-build-port/H3-I5-{warm,bypass}.log
)
```

Use the before and after statistics deltas for the warm-run hit rate. Compare
both runs with the 1,010.7 s baseline and report real CPU from the kit.

#### H4. Compare artifacts and lockfiles

```bash
cd "$root"
python3 - "$reference" <<'PY' > "$evidence/H4-I5-artifacts.log" 2>&1
import os
from pathlib import Path
import subprocess
import sys

lock = int(os.environ["BOARDSTUDIO_BUILD_LOCK_FD"])
page = Path("web/target/site/site-subpath/boardstudio/wasm/boardstudio-web_bg.wasm")
build = [sys.executable, "scripts/build-web.py", "--fast"]

def page_build(jobs):
    subprocess.run(
        ["cargo", "clean", "-p", "boardstudio-web", "--profile", "wasm-check",
         "--target", "wasm32-unknown-unknown"],
        check=True, pass_fds=(lock,))
    subprocess.run(build, env=dict(os.environ, BOARDSTUDIO_CHECK_JOBS=str(jobs)),
                   check=True, pass_fds=(lock,))
    return page.read_bytes()

overlapped = page_build(2)
serial = page_build(1)
assert overlapped == serial, (len(overlapped), len(serial))
print(f"Page WASM byte-identical, {len(serial)} bytes")

reference = Path(sys.argv[1])
cad = Path("web/assets/cad/boardstudio_cad_wasm_bg.wasm")
print(f"CAD WASM bytes before {(reference / cad).stat().st_size}, after {cad.stat().st_size}")
for assets in (Path("web/assets/core-worker"), Path("web/assets/cad-worker")):
    declarations = {path.name for path in assets.glob("*.d.ts")}
    assert declarations and declarations == {p.name for p in (reference / assets).glob("*.d.ts")}
    for name in sorted(declarations):
        file = assets / name
        assert file.read_bytes() == (reference / file).read_bytes(), str(file)
        print(f"Unchanged declaration {file}")
PY
git diff 2ce123403 -- Cargo.lock '**/Cargo.lock' > "$evidence/H4-I5-lockfiles.log"
git show --format=%B --no-patch 3fde0a3af \
    > "$evidence/H4-I5-CAD-lock-commit.log"
```

### Exact native test name mappings

I2 maps each grouped test to the case names on the lines immediately following
it. Apply the I5 mapping to the resulting footprints names. CAD test names are
listed without a binary prefix.

```text
model::source_validation_tests::pinned_vik_step_meshes_preserve_source_bounds_and_hashes ->
model::source_validation_tests::vik_step_pers60_cirque_leds_pcb
model::source_validation_tests::vik_step_kiwano
model::source_validation_tests::vik_step_trackpad_mount
model::source_validation_tests::vik_step_trackball_mount_semi
model::source_validation_tests::vik_step_trackball_mount_low_profile
model::source_validation_tests::vik_step_trackball_mount_full
model::source_validation_tests::vik_step_knob_ribbed
model::source_validation_tests::vik_step_knob_low_profile
model::source_validation_tests::vik_step_knob_full
model::source_validation_tests::vik_step_knob_base
model::source_validation_tests::vik_step_crenshaw
model::source_validation_tests::vik_step_fh12_30s_0_5sh
model::source_validation_tests::vik_step_fh12_12s_0_5sh
model::source_validation_tests::vik_step_vik_playground
model::source_validation_tests::vik_step_2inch_0_8mm
model::source_validation_tests::vik_step_lcd_lcd_ips
model::source_validation_tests::vik_step_1_47inch
model::source_validation_tests::vik_step_gc9a01
model::source_validation_tests::vik_step_1_28inch_round
model::source_validation_tests::vik_step_matrix_inator_rev2
model::source_validation_tests::vik_step_peacock_low_case_v5
model::source_validation_tests::vik_step_peacock_low_case_v5_5deg_tilt
model::source_validation_tests::vik_step_vik_connector_horizontal
model::source_validation_tests::vik_step_vik_connector_horizontal_mini
model::source_validation_tests::vik_step_ili9341_4inch_housing
model::source_validation_tests::vik_step_ili9341_4inch_cover
model::source_validation_tests::vik_step_vikropad_choc_glp_xivik
model::source_validation_tests::unlisted_vik_step_meshes_preserve_source_bounds_and_hashes

model::keycaps::print_legend_tests::ordinary_legend_text_prints_as_one_closed_body ->
model::keycaps::print_legend_tests::dcs_legend_a
model::keycaps::print_legend_tests::dcs_legend_o
model::keycaps::print_legend_tests::dcs_legend_t
model::keycaps::print_legend_tests::dcs_legend_digit_1
model::keycaps::print_legend_tests::dcs_legend_digit_4
model::keycaps::print_legend_tests::dcs_legend_q
model::keycaps::print_legend_tests::dcs_legend_enter
model::keycaps::print_legend_tests::dcs_legend_backspace
model::keycaps::print_legend_tests::dcs_legend_shift
model::keycaps::print_legend_tests::dcs_legend_f12
model::keycaps::print_legend_tests::dcs_legend_left_arrow
model::keycaps::print_legend_tests::dcs_legend_right_arrow
model::keycaps::print_legend_tests::dcs_legend_backtick
model::keycaps::print_legend_tests::sa_legend_a
model::keycaps::print_legend_tests::sa_legend_o
model::keycaps::print_legend_tests::sa_legend_t
model::keycaps::print_legend_tests::sa_legend_digit_1
model::keycaps::print_legend_tests::sa_legend_digit_4
model::keycaps::print_legend_tests::sa_legend_q
model::keycaps::print_legend_tests::sa_legend_enter
model::keycaps::print_legend_tests::sa_legend_backspace
model::keycaps::print_legend_tests::sa_legend_shift
model::keycaps::print_legend_tests::sa_legend_f12
model::keycaps::print_legend_tests::sa_legend_left_arrow
model::keycaps::print_legend_tests::sa_legend_right_arrow
model::keycaps::print_legend_tests::sa_legend_backtick

generators::bundled_generators_match_the_golden_baseline ->
generators::golden_ceoloide_battery_connector_jst_ph_2
generators::golden_ceoloide_battery_connector_molex_pico_ezmate_1x02
generators::golden_ceoloide_diode_tht_sod123
generators::golden_ceoloide_display_nice_view
generators::golden_ceoloide_display_ssd1306
generators::golden_ceoloide_led_sk6812mini_e
generators::golden_ceoloide_mcu_nice_nano
generators::golden_ceoloide_mcu_supermini_nrf52840
generators::golden_ceoloide_mounting_hole_npth
generators::golden_ceoloide_mounting_hole_plated
generators::golden_ceoloide_power_switch_smd_side
generators::golden_ceoloide_reset_switch_smd_side
generators::golden_ceoloide_reset_switch_tht_top
generators::golden_ceoloide_rotary_encoder_ec11_ec12
generators::golden_ceoloide_switch_choc_v1_v2
generators::golden_ceoloide_switch_gateron_ks27_ks33
generators::golden_ceoloide_switch_mx
generators::golden_ceoloide_trrs_pj320a
generators::golden_ceoloide_utility_filled_zone
generators::golden_ceoloide_utility_keepout_zone
generators::golden_ceoloide_utility_logo
generators::golden_ceoloide_utility_point_debugger
generators::golden_ceoloide_utility_router
generators::golden_ceoloide_utility_text
generators::golden_infused_kim_conn_molex_pico_ezmate_1x02
generators::golden_infused_kim_conn_molex_pico_ezmate_1x05
generators::golden_infused_kim_icon_bat
generators::golden_infused_kim_mounting_hole
generators::golden_infused_kim_nice_view
generators::golden_infused_kim_pads
generators::golden_infused_kim_point_debugger
generators::golden_infused_kim_smd_0805
generators::golden_infused_kim_switch_power
generators::golden_infused_kim_switch_reset
generators::golden_infused_kim_text
generators::golden_infused_kim_trackpoint_mount
generators::unlisted_bundled_generators_match_the_golden_baseline
```

I5 uses `binary::test` on both sides of the arrow.

```text
choc_socket_qualification::configured_terminals_and_effective_pad_layers_are_exact_and_distinct -> footprints_integration::choc_socket_qualification::configured_terminals_and_effective_pad_layers_are_exact_and_distinct
choc_socket_qualification::datasheet_drills_and_dimensioned_left_land_match_nominals_at_both_faces_and_angles -> footprints_integration::choc_socket_qualification::datasheet_drills_and_dimensioned_left_land_match_nominals_at_both_faces_and_angles
context::the_context_matches_the_javascript_provider -> footprints_integration::context::the_context_matches_the_javascript_provider
context::unknown_generators_and_versions_keep_their_baseline_text -> footprints_integration::context::unknown_generators_and_versions_keep_their_baseline_text
diode_sod123_qualification::absent_null_false_and_malformed_side_keep_legacy_rendering -> footprints_integration::diode_sod123_qualification::absent_null_false_and_malformed_side_keep_legacy_rendering
diode_sod123_qualification::model_binding_keeps_emitted_defaults_overrides_and_empty_omission -> footprints_integration::diode_sod123_qualification::model_binding_keeps_emitted_defaults_overrides_and_empty_omission
diode_sod123_qualification::opt_in_lands_match_vishay_callouts_and_fixed_pose_oracles -> footprints_integration::diode_sod123_qualification::opt_in_lands_match_vishay_callouts_and_fixed_pose_oracles
diode_sod123_qualification::opt_in_requires_a_front_or_back_nonreversible_smd_configuration -> footprints_integration::diode_sod123_qualification::opt_in_requires_a_front_or_back_nonreversible_smd_configuration
diode_sod123_qualification::part_null_overrides_definition_true_with_the_declaration_default -> footprints_integration::diode_sod123_qualification::part_null_overrides_definition_true_with_the_declaration_default
diode_sod123_qualification::schema_exposes_the_opt_in_as_a_default_false_boolean -> footprints_integration::diode_sod123_qualification::schema_exposes_the_opt_in_as_a_default_false_boolean
encoder_ec11e15244g1_qualification::declared_parameters_are_explicitly_available_for_this_pattern -> footprints_integration::encoder_ec11e15244g1_qualification::declared_parameters_are_explicitly_available_for_this_pattern
encoder_ec11e15244g1_qualification::front_pattern_geometry_roles_and_export_match_independent_nominals -> footprints_integration::encoder_ec11e15244g1_qualification::front_pattern_geometry_roles_and_export_match_independent_nominals
encoder_ec11e15244g1_qualification::normalization_preserves_saved_pad_ids_and_nets_by_literal_index -> footprints_integration::encoder_ec11e15244g1_qualification::normalization_preserves_saved_pad_ids_and_nets_by_literal_index
generators::diode_sod123_baseline_capture -> footprints_integration::generators::diode_sod123_baseline_capture
generators::diode_sod123_baseline_matches_frozen_expected_bytes -> footprints_integration::generators::diode_sod123_baseline_matches_frozen_expected_bytes
generators::diode_sod123_explicit_false_matches_frozen_outputs -> footprints_integration::generators::diode_sod123_explicit_false_matches_frozen_outputs
generators::every_registered_generator_has_a_fixture -> footprints_integration::generators::every_registered_generator_has_a_fixture
generators::golden_ceoloide_battery_connector_jst_ph_2 -> footprints_integration::generators::golden_ceoloide_battery_connector_jst_ph_2
generators::golden_ceoloide_battery_connector_molex_pico_ezmate_1x02 -> footprints_integration::generators::golden_ceoloide_battery_connector_molex_pico_ezmate_1x02
generators::golden_ceoloide_diode_tht_sod123 -> footprints_integration::generators::golden_ceoloide_diode_tht_sod123
generators::golden_ceoloide_display_nice_view -> footprints_integration::generators::golden_ceoloide_display_nice_view
generators::golden_ceoloide_display_ssd1306 -> footprints_integration::generators::golden_ceoloide_display_ssd1306
generators::golden_ceoloide_led_sk6812mini_e -> footprints_integration::generators::golden_ceoloide_led_sk6812mini_e
generators::golden_ceoloide_mcu_nice_nano -> footprints_integration::generators::golden_ceoloide_mcu_nice_nano
generators::golden_ceoloide_mcu_supermini_nrf52840 -> footprints_integration::generators::golden_ceoloide_mcu_supermini_nrf52840
generators::golden_ceoloide_mounting_hole_npth -> footprints_integration::generators::golden_ceoloide_mounting_hole_npth
generators::golden_ceoloide_mounting_hole_plated -> footprints_integration::generators::golden_ceoloide_mounting_hole_plated
generators::golden_ceoloide_power_switch_smd_side -> footprints_integration::generators::golden_ceoloide_power_switch_smd_side
generators::golden_ceoloide_reset_switch_smd_side -> footprints_integration::generators::golden_ceoloide_reset_switch_smd_side
generators::golden_ceoloide_reset_switch_tht_top -> footprints_integration::generators::golden_ceoloide_reset_switch_tht_top
generators::golden_ceoloide_rotary_encoder_ec11_ec12 -> footprints_integration::generators::golden_ceoloide_rotary_encoder_ec11_ec12
generators::golden_ceoloide_switch_choc_v1_v2 -> footprints_integration::generators::golden_ceoloide_switch_choc_v1_v2
generators::golden_ceoloide_switch_gateron_ks27_ks33 -> footprints_integration::generators::golden_ceoloide_switch_gateron_ks27_ks33
generators::golden_ceoloide_switch_mx -> footprints_integration::generators::golden_ceoloide_switch_mx
generators::golden_ceoloide_trrs_pj320a -> footprints_integration::generators::golden_ceoloide_trrs_pj320a
generators::golden_ceoloide_utility_filled_zone -> footprints_integration::generators::golden_ceoloide_utility_filled_zone
generators::golden_ceoloide_utility_keepout_zone -> footprints_integration::generators::golden_ceoloide_utility_keepout_zone
generators::golden_ceoloide_utility_logo -> footprints_integration::generators::golden_ceoloide_utility_logo
generators::golden_ceoloide_utility_point_debugger -> footprints_integration::generators::golden_ceoloide_utility_point_debugger
generators::golden_ceoloide_utility_router -> footprints_integration::generators::golden_ceoloide_utility_router
generators::golden_ceoloide_utility_text -> footprints_integration::generators::golden_ceoloide_utility_text
generators::golden_infused_kim_conn_molex_pico_ezmate_1x02 -> footprints_integration::generators::golden_infused_kim_conn_molex_pico_ezmate_1x02
generators::golden_infused_kim_conn_molex_pico_ezmate_1x05 -> footprints_integration::generators::golden_infused_kim_conn_molex_pico_ezmate_1x05
generators::golden_infused_kim_icon_bat -> footprints_integration::generators::golden_infused_kim_icon_bat
generators::golden_infused_kim_mounting_hole -> footprints_integration::generators::golden_infused_kim_mounting_hole
generators::golden_infused_kim_nice_view -> footprints_integration::generators::golden_infused_kim_nice_view
generators::golden_infused_kim_pads -> footprints_integration::generators::golden_infused_kim_pads
generators::golden_infused_kim_point_debugger -> footprints_integration::generators::golden_infused_kim_point_debugger
generators::golden_infused_kim_smd_0805 -> footprints_integration::generators::golden_infused_kim_smd_0805
generators::golden_infused_kim_switch_power -> footprints_integration::generators::golden_infused_kim_switch_power
generators::golden_infused_kim_switch_reset -> footprints_integration::generators::golden_infused_kim_switch_reset
generators::golden_infused_kim_text -> footprints_integration::generators::golden_infused_kim_text
generators::golden_infused_kim_trackpoint_mount -> footprints_integration::generators::golden_infused_kim_trackpoint_mount
generators::unlisted_bundled_generators_match_the_golden_baseline -> footprints_integration::generators::unlisted_bundled_generators_match_the_golden_baseline
golden_forms::export_matches_apart_from_arcs_core_upgrades -> footprints_integration::golden_forms::export_matches_apart_from_arcs_core_upgrades
golden_forms::geometry_matches_the_recorded_pads_and_courtyards -> footprints_integration::golden_forms::geometry_matches_the_recorded_pads_and_courtyards
golden_forms::model_references_match -> footprints_integration::golden_forms::model_references_match
golden_forms::parser_behaviour_matches_the_provider_fixture -> footprints_integration::golden_forms::parser_behaviour_matches_the_provider_fixture
golden_forms::preview_paths_follow_the_recorded_rules -> footprints_integration::golden_forms::preview_paths_follow_the_recorded_rules
golden_forms::recorded_forms_round_trip_through_the_parser -> footprints_integration::golden_forms::recorded_forms_round_trip_through_the_parser
licenses::declared_licences_match_the_file_headers -> footprints_integration::licenses::declared_licences_match_the_file_headers
mx_socket_qualification::recommended_land_sizes_match_drawing_reference_dimensions -> footprints_integration::mx_socket_qualification::recommended_land_sizes_match_drawing_reference_dimensions
mx_socket_qualification::recommended_socket_hole_pair_spacing_survives_side_and_placement_rotation -> footprints_integration::mx_socket_qualification::recommended_socket_hole_pair_spacing_survives_side_and_placement_rotation
mx_socket_qualification::selected_sofle_switch_pose_preserves_source_layout_hole_spacing -> footprints_integration::mx_socket_qualification::selected_sofle_switch_pose_preserves_source_layout_hole_spacing
mx_socket_qualification::socket_terminals_keep_distinct_nets_and_effective_side_layers -> footprints_integration::mx_socket_qualification::socket_terminals_keep_distinct_nets_and_effective_side_layers
numeric::hypot_matches_v8 -> footprints_integration::numeric::hypot_matches_v8
numeric::number_to_string_matches_v8 -> footprints_integration::numeric::number_to_string_matches_v8
numeric::string_to_number_matches_v8 -> footprints_integration::numeric::string_to_number_matches_v8
numeric::to_fixed_matches_v8 -> footprints_integration::numeric::to_fixed_matches_v8
numeric::trigonometry_is_bit_identical_to_v8 -> footprints_integration::numeric::trigonometry_is_bit_identical_to_v8
numeric::uri_component_encoding_matches_v8 -> footprints_integration::numeric::uri_component_encoding_matches_v8
parameter_check::an_undeclared_name_is_reported_and_null_is_absent -> footprints_integration::parameter_check::an_undeclared_name_is_reported_and_null_is_absent
parameter_check::an_unknown_generator_is_an_error -> footprints_integration::parameter_check::an_unknown_generator_is_an_error
parameter_check::every_bundled_default_passes_its_own_check -> footprints_integration::parameter_check::every_bundled_default_passes_its_own_check
parameter_check::every_problem_is_reported_in_name_order -> footprints_integration::parameter_check::every_problem_is_reported_in_name_order
parameter_check::non_empty_list_text_is_a_wrong_type_for_an_array_parameter -> footprints_integration::parameter_check::non_empty_list_text_is_a_wrong_type_for_an_array_parameter
parameter_check::text_for_a_number_is_a_wrong_type_named_by_parameter -> footprints_integration::parameter_check::text_for_a_number_is_a_wrong_type_named_by_parameter
reset_switch_smd_side_qualification::boss_model_default_and_explicit_model_transforms_are_preserved -> footprints_integration::reset_switch_smd_side_qualification::boss_model_default_and_explicit_model_transforms_are_preserved
reset_switch_smd_side_qualification::land_pattern_contacts_layers_bosses_and_poses_match_independent_oracle -> footprints_integration::reset_switch_smd_side_qualification::land_pattern_contacts_layers_bosses_and_poses_match_independent_oracle
text_literal_escaping::bare_references_stay_atoms_until_quoting_is_needed -> footprints_integration::text_literal_escaping::bare_references_stay_atoms_until_quoting_is_needed
text_literal_escaping::both_text_generators_emit_the_literal_text_on_front_and_back -> footprints_integration::text_literal_escaping::both_text_generators_emit_the_literal_text_on_front_and_back
text_literal_escaping::logo_layer_parameter_is_literal_on_both_sides -> footprints_integration::text_literal_escaping::logo_layer_parameter_is_literal_on_both_sides
text_literal_escaping::pad_row_labels_and_reference_are_literal_on_front_and_back -> footprints_integration::text_literal_escaping::pad_row_labels_and_reference_are_literal_on_front_and_back
text_literal_escaping::plain_text_controls_keep_their_existing_output -> footprints_integration::text_literal_escaping::plain_text_controls_keep_their_existing_output
text_literal_escaping::smd_labels_and_reference_are_literal_on_front_and_back -> footprints_integration::text_literal_escaping::smd_labels_and_reference_are_literal_on_front_and_back
```

## J. Follow-up implementation

### J1. Classify the page prebuild as a build command

Treat `run-wasm-tests.py --build-only` like the other prebuilds in a receipt.
Its exit status determines success. Page suite launches still require a fresh
result, and the receipt summary includes only those launches as page evidence.

### J2. Guard Footprints test registration

The repository registration gate checks both Core and Footprints. A top-level
test file must be an explicit Cargo test target or a module of the integration
target. Core keeps its existing diagnostics.

### J3. Bound the production toast expiry check

The expiry test returns as soon as the toast disappears. It uses the shared
bounded polling helper with a 2,500 ms limit so a substantially longer
production timeout still fails the test.

### J4. Keep the KiCad importer in tooling-focused quick runs

Selecting tooling in a quick run also selects the importer suite in the test
step. Changes to shared helpers retain the importer coverage they had before E1.

### J5. Shorten idle HTTP server shutdowns

The provider pages, face completeness, CAD-build fake driver, and three dev-web
server sites poll for shutdown every 50 ms. The CAD deletion delay and timeout
checks keep their original settings. The static-preview test terminates its
server process directly.

### J6. Load the catalogue only for the VIK display adapter

Read the imported catalogue inside the STEP display-adapter assertion branch.
The named cases and the fallback still check the same assets and bounds.

### J7. Skip ignored old-CSS scan inputs

Git supplies the tracked and non-ignored untracked paths under `web/`. The
scanner still checks untracked sources, retains its build-directory exclusions,
and walks the tree when Git cannot supply a file list.

### J8. Run independent tooling commands together

The scheduler uses `check_jobs()` workers and emits complete command output in
plan order. It reports each failure and records each command in the receipt.
One worker keeps the serial path. Only tooling commands in
`BUILD_LOCK_SCRIPTS` inherit the lock descriptor and its environment variable.
The mandatory hermetic Ergogen test stays in the tooling plan.

Serial boundaries in plan order:

- `scripts/test-serve-web.py` releases a reserved loopback port before the
  preview subprocess binds it. Another tooling server could claim it in between.
- `scripts/test-parallel-interrupt.py` owns detached process groups and
  exercises nested cancellation. Keep its groups outside the outer batch.

The remaining commands use unique temporary fixtures or read repository
inputs. The CAD measurement self-tests create unique fixture directories
under their existing evidence parents. The CAD-build self-test reads
workspace metadata from a temporary fixture and mocks its build and driver.
Other HTTP fixtures bind ephemeral ports directly. The response-probe tests
install their own SIGTERM handler to clean up the probe group. Check and CI
self-tests use mocked commands, fake binaries or private lock files when
exercising the scheduler and build wrappers.
