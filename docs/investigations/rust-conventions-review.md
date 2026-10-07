# Rust conventions review: what the Rust skills would change

Investigated 2026-10-06 at `be4643089`. Status: assessment only; no code was changed.
The `rust-skills` (265 rules) and `rust-best-practices` (Apollo handbook) skills are
installed locally under the gitignored `.agents/` and `.claude/`, so other
checkouts and cloud agents never see them. This note records which of their advice
fits this repository, measures each candidate, and ranks the changes by benefit.
The Rust section in [AGENTS.md](../../AGENTS.md) carries the rules that are
already decided.

## Method and limits

- Counts exclude `vendor/` and `target/` and come from `grep` over first-party
  `core`, `application`, `contracts`, `footprints`, `renderer` and `web` sources.
- "Production" unwrap/expect means occurrences before the first `#[cfg(test)]` in a
  file. Test modules gated `#[cfg(all(test, target_arch = "wasm32"))]` are test code:
  this is why `web/crates/library` (186) and `web/crates/case` (90) look large.
- The unused-import experiment ran `cargo check` with
  `RUSTFLAGS="--force-warn unused-imports"`, which overrides source `allow`
  attributes, on native (`--workspace --all-targets`), WASM (`--lib --bins
  --all-features`) and WASM `--tests` for the web crates. Feature combinations and
  `cfg` paths outside those three runs were not exercised.
- No Clippy lint was enabled and no behaviour was reproduced. Findings about
  possible panics are reading-based and need a reproduction before a repair, per the
  [backlog](../backlog.md) rule.

## Findings by candidate

### 1. Prefer `#[expect(lint)]` with a reason for new suppressions

`#[expect]` fails the build when the suppression stops being needed, so stale
suppressions cannot accumulate. First-party code has 1 `#[expect]` and about 115
`#[allow]` (68 of them `unused_imports`). The item-level `clippy::*` allows already
carry a reason comment.

| | |
| --- | --- |
| Benefit | Low to medium. Prevents future stale suppressions. |
| Cost | Near zero as a convention for new code. |
| Risk | None for new item-level lints. Unsafe for `unused_imports`, which fires on one target and not the other (see 2). |
| Verdict | **Adopt as a one-line AGENTS.md rule.** Do not bulk-convert existing allows. |

### 2. Remove dead `#[allow(unused_imports)]` re-exports

The `web/crates/*/src/lib.rs` files and `web/src/presentation.rs` re-export modules
to preserve the page binary's old paths. The experiment classified all 68 allows:

| Result | Count | Meaning |
| --- | --- | --- |
| Fires on native and WASM | 13 | Live; the allow is needed |
| Fires on native only | 22 | Live; WASM-only re-exports unused when built natively |
| Fires on WASM only | 15 | Live |
| Never fires on any checked target | 18 (17 after the WASM test run) | Dead |

Earlier in this review the allows were guessed to be mostly redundant; the
measurement shows the opposite, 51 of 68 suppress a real warning.

Dead allows (checked with the three runs above):
`case/src/lib.rs` 24, 32; `keymap/src/lib.rs` 13; `layout/src/lib.rs` 15, 20, 23;
`library/src/lib.rs` 10, 12; `parts/src/lib.rs` 15, 20, 27, 41;
`pcb/src/lib.rs` 15, 23, 40, 56; `web/src/presentation.rs` 42. Line numbers are at
`be4643089` and drift; re-run the experiment rather than trusting them.

| | |
| --- | --- |
| Benefit | Low. Cosmetic noise reduction; no behaviour change. |
| Cost | Low to moderate. Needs `lint`, `build` and `browser` to confirm, because a re-export used only under a feature combination not checked here would start warning and fail `-D warnings`. |
| Verdict | **Optional cleanup, bundle with a change that already touches these files.** Do not convert the 51 live ones to `expect`: target-dependent warnings would make `expect` fail on the other target. The 22 native-only ones could instead use `cfg_attr(not(target_arch = "wasm32"), allow(...))`, which is a separate, smaller win. |

### 3. Typed errors in place of `Result<_, String>`

`Result<_, String>` appears 76 times in `core/src`, 311 in `web/crates` and 2 in
`footprints/src`. Typed errors exist where callers branch: `ArtifactErrorCode`
(`core/src/model.rs`), `GeneratorError` (`footprints/src/error.rs`) and
`CadJobError` (`web/crates/host/src/cad_jobs.rs`).

Measured cost of the string convention:

- No production code branches on error text. The `contains`/`starts_with` hits in
  `core/src` match footprint source ids, not errors.
- 22 test assertions compare against error message text. That is the only
  coupling, and it is deliberate scenario checking.
- About 167 `map_err(|e| e.to_string())` and 285 `Err(format!(...))` sites
  flatten errors, and the browser boundary serializes errors to JS strings anyway.

| | |
| --- | --- |
| Benefit | Low today. Real only where a caller needs to branch, retry or localize. |
| Cost | High. Roughly 390 signatures plus every call site, test and boundary. |
| Verdict | **Do not convert wholesale.** Introduce a typed error when a caller first needs to branch on the cause. The AGENTS.md bullet already says to follow the surrounding module. |

### 4. Production `unwrap`/`expect` and `clippy::unwrap_used`

About 70 production sites in `core`, `application` and `footprints`, almost all
`expect` with a message naming an invariant established earlier ("validated",
"checked", "reply serializes"). Those are reasonable. Weaker cases:

- Bare `find(...).unwrap()` on identifiers that come from edits or documents:
  `core/src/inputs.rs` (`part` by `id`), `core/src/modules/circuit.rs`
  (`board` by `board_id`), `core/src/mechanical/gasket.rs` (`region` by
  `layout.id`). A malformed or stale id would panic instead of returning an error.
- `application/src/session.rs` unwraps `self.model.accepted` in two places
  (stale-revision check, scene snapshot). Other paths use
  `expect("queued edits always have an accepted document")`; a bare unwrap is
  inconsistent and loses that explanation.
- `expect("checked")` repeated in `core/src/artifact/kicad/output.rs` and
  `planning.rs` names no invariant.

| | |
| --- | --- |
| Benefit | Medium for the three `find` sites and the two Session sites, if any is reachable with real input: a panic in the WASM Core or page loses the session. Unknown until reproduced. |
| Cost | A workspace `unwrap_used` lint would warn on about 70 sites and needs `clippy.toml` `allow-unwrap-in-tests`, so it is noisy for little return. A targeted audit of about six sites is small. |
| Verdict | **Audit the six sites, do not add the lint.** Reproduce before repairing; where a panic is reachable, return the module's usual error. Reword `expect("checked")` to name the invariant when touching those files. |

## Rejected

| Advice | Why it does not fit |
| --- | --- |
| Box large enum variants | `large_enum_variant = "allow"` is a recorded decision in the root `Cargo.toml` with its reason. |
| `thiserror` / `anyhow` by default | Two of the three error enums are hand-written and small; a dependency does not pay for itself. `cargo deny` would also review it. |
| `#![deny(missing_docs)]` | Application crates, not published libraries. |
| One assertion per test | Tests assert scenario outcomes, for example `stale_rendered_copy_generation_is_rejected_after_callback_refresh`. |
| `Arc`, `Mutex`, `Send`/`Sync` guidance | The browser crates are single-threaded WASM on Dioxus; about 1,000 `Rc`/`RefCell` uses are correct there. |
| Never `unwrap()` outside tests | Contradicts the invariant-`expect` pattern above, which is sound. |

## Recommended order

1. Add the `#[expect]` rule to AGENTS.md (candidate 1). Doc-only.
2. Audit the six unwrap sites (candidate 4). Reproduce first; record results in
   the [backlog](../backlog.md) entry.
3. Remove the dead `unused_imports` allows alongside other work in those files
   (candidate 2).
4. Candidate 3 only on demand.

## Completion conditions

- Candidate 1: AGENTS.md states the `#[expect]` rule.
- Candidate 2: re-run the force-warn experiment on current `dev`, remove the sites
  that never fire, and pass `python3 scripts/check.py lint build browser`.
- Candidate 4: each audited site either gains a reproduction and a fix, or an
  `expect` message naming its invariant.
