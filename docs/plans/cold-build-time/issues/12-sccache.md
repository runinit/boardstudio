# 12: `sccache` across worktrees

Status: ready-for-human
Type: task
Blocked by: [03](03-seed-worktree-targets.md)
Parent: [map](../map.md)

## Question

Does `sccache` beat a shared target for new worktrees, and is it worth installing machine-wide?

## Answer to plan against

`sccache` beats a shared target, but on effort and blast radius it comes after
[03](03-seed-worktree-targets.md). Install it only if 03 fails. The installation is your decision.

The brief's "half or more of a new worktree's cold build" is right for CPU and wrong for wall:

| Set | Units | CPU-s | Attributed wall | Cacheable across worktrees? |
|---|---:|---:|---:|---|
| Registry and git rlibs | 1724 | 1566 (51%) | 289 s | Yes, cwd is `~/.cargo/registry/src/...` in every worktree |
| Registry proc macros, build scripts, bins | 303 | 127 | 13.5 s | No, "invokes the system linker" (`sccache` docs/Rust.md) |
| Path crates in `release` | 75 | 456 | 222 s | Only with `SCCACHE_BASEDIRS`, unverified |
| Path crates in `dev` and `wasm-dev` | 270 | 920 | 385 s | No, incremental, and `AGENTS.md` forbids `CARGO_INCREMENTAL=0` |

So for a new worktree: -51% CPU and about -290 s (-23%) of wall, reaching about -66% CPU and -510 s only if
`SCCACHE_BASEDIRS` works for path crates. Attributed wall is an estimate. One side benefit that does not
need a second worktree: the CAD host trees already duplicate 142 CPU-s by metadata, and a cache removes
the registry part of that in a single run.

## Shared target rejected

Pointing all worktrees at one `CARGO_TARGET_DIR` or `target` symlink, against `sccache`:

1. **Output collisions.** `scripts/build-web.py:29` hard-codes `ROOT/target/dx/boardstudio-web/...`, and the
   name carries no worktree identity. Concurrent checks would overwrite one another's site.
2. **Locks.** Cargo locks `target/<profile>`, so every worktree serializes against every other. `AGENTS.md`
   forbids overlapping builds on a target, and `check-speed` 10 traced an `invalid type: sequence` failure to
   overlap. Many agent sessions run at once on this machine.
3. **It misses 38% of CPU.** `cad/scripts/cad_build.py:115, 137, 149, 161` force CAD target dirs inside each
   worktree (1164 CPU-s), and the harness has its own dir.
4. **No reuse for workspace crates.** Their package id has the absolute path, so each worktree adds its own
   copies, with no stable garbage collection for what is already 252 G.

## Evidence on this machine

- 103 worktrees, 36 real and 3 symlinked target dirs in a two-level scan, not a dozen. `AGENTS.md` says session
  worktrees share the main checkout's target; this one does not (`readlink -f target` is local).
- `sccache` and `mold` are not installed.
- `dx` becomes the workspace wrapper for workspace crates (`driver: dx` in the unit log), so a wrapper chain is
  `sccache dx rustc`. **Not verified**: `sccache` may treat `dx` as an unknown compiler. Those units are
  incremental and would not be cached anyway, but they must not fail.
- `.claude/settings.json` is unreadable from this sandbox, so its allowlist for a cache dir was not checked.

## Gain

| Cold wall (new worktree) | CPU | Warm |
|---:|---:|---|
| -290 s, or up to -510 s with basedirs | -1566, or up to -2022 | none |

Effort: 2 hours after install. Risk: low to medium. A global `RUSTC_WRAPPER` affects every Rust project of yours
on this machine; a daemon and a cache dir must work under the sandbox; about 15 to 20 G of cache versus
234 G free; a toolchain bump empties the useful cache.

## Work

1. **You decide:** install `sccache` and set `rustc-wrapper` in `~/.cargo/config.toml`, or per-shell in the
   agent environment. Never in the repo's `.cargo/config.toml`: that would change CI and every contributor.
2. Set `SCCACHE_DIR` under `~/.cache`, `SCCACHE_CACHE_SIZE=20G`, and try `SCCACHE_BASEDIRS` with the worktree
   roots. `basedirs` is documented in `sccache`'s `docs/Configuration.md`; confirm the installed version has it.
3. Chain the unit logger when measuring (`RUSTC_WRAPPER` allows one value, so `rustc-log.py` must call `sccache`).

## Verification

```sh
sccache --zero-stats
python3 scripts/build-time/cold.py sccache-fill                     # first worktree fills the cache
sccache --show-stats > .evidence/sccache-fill.txt
# then in a second, new worktree at the same head:
sccache --zero-stats && python3 scripts/build-time/cold.py sccache-hit
sccache --show-stats > .evidence/sccache-hit.txt
python3 scripts/build-time/report.py rebaseline sccache-hit
grep -E '^check: (native tests|all steps)' .evidence/build-time/sccache-hit.log
```

## Stop rule

Remove the wrapper if the second worktree gains under 150 s, if any check fails only with the wrapper, or if
`dx` builds break. An incorrect cache hit means uninstall at once and report it.
