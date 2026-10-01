# BoardStudio milestone continuation

Run base: `96dd51d3e790c28f5554a8c9888a147c8814e2a7`.
Coordinator worktree: `/tmp/boardstudio-migration-specs-review-20261001`.
Branch: `docs/boardstudio-migration-specs-review` (existing protected worktrees
are read-only inputs). Accepted scope: [map](../../CAPABILITY-MAP.md) and
[ADR 0003](../adr/0003-rust-application-ownership.md).

## Authorization

The 2026-10-01 request first authorizes remaining module specifications,
technical validation and execution preflight. No implementation task set
existed when that request arrived. The subsequent user instruction,
**“Assume everything is approved in advance”**, removes routine phase/progress
approval pauses within this defined milestone. The later **“Install it ...?”**
explicitly authorizes installing the selected Dioxus CLI.

Record each exact spec/plan revision and bounded task set before its execution.
Advance approval is applied to those defined tasks, not a claim that undefined
future migration work has been specified. Continue safely through their graph;
actual failed gates, incompatible data or invalidated architecture still block
dependent work. No push, deployment, main merge, protected-worktree writes,
unauthorized model/provider switch or automatic prototype promotion is allowed.

## Current task set

| Task | Exact scope / authority | Status |
| --- | --- | --- |
| DOC-1 | Complete and technically validate the six mapped M1 specs; preserve accepted inputs | Complete: reviewed and integrated `47d6dbce`; documentation checks only |
| TOOL-1 | Install official Dioxus CLI **0.7.10** selected by ADR 0003; verify official digest and `dx --version`; add a command link only if absent | Complete: official archive digest and `dioxus 0.7.10 (57d6794)` verified, exit 0 |
| P1 | Existing worker/CAD packaging charter; exact bounded [P1-r1](../../tasks/plan.md), task set P1-CORE → P1-CAD in root TODO | P1-CORE blocked/unaccepted; P1-CAD dependency-blocked and not started; no safe remaining task |

TOOL-1 may write only its project-owned tool directory under
`/home/chris/.local/share/boardstudio/tools/dioxus-cli/0.7.10`, an absent
`/home/chris/.local/bin/dx` link, and this run's download/log/evidence paths.
Do not overwrite an existing executable or change PATH/config. Use the official
x86_64 Linux archive and published SHA-256; no provider/model is involved.

## Preflight evidence and limits

Codex CLI 0.159.3 and strict-config doctor load succeed. Actual configuration
contains a three-thread limit and Luna/high subagent defaults. The smoke request
used Luna/medium; state DB and stored turn context independently confirm both.
The source inventory used Luna/medium and performed no writes.
[Observable metadata](../../.scratch/migration-specs-validation/subagent-preflight.json)
retains the evidence; agent self-identification was not used as routing proof.

Current execution has never/danger-full-access permissions, and new-worktree
creation succeeds without changing them. Doctor describes its standalone
invocation sandbox, not active-thread overrides; actual tool operations are
the stronger evidence. Git, Cargo/Rust 1.98.0, pnpm 12.6.0, Node, wasm-pack,
Chromium 153 and agent-browser are available. A task-owned Chromium about:blank
open/snapshot/close succeeds. This proves launch capability, not application
browser acceptance. Dioxus CLI was absent before TOOL-1.

`codex --strict-config features list` is unsupported (exit 1); the diagnosed
fallback `codex --strict-config doctor --json` succeeds (exit 0) and validates
configuration load. Doctor retains a pre-existing rollout/DB parity warning;
no configuration or state repair is performed.

Closing threads is a capability limitation: `codex archive` for the completed
smoke thread returns exit 1. The collaboration interface exposes no close-thread
operation. Preserve completed results and do not mutate the Codex state DB or
spawn beyond three total open child threads. This may constrain delegation;
it does not block coordinator documentation/tool installation.

Official Dioxus latest release is rechecked as stable v0.7.10. Tagged CLI build,
target and web configuration sources verify build/base-path flags. Some web
retrievals fail; authoritative GitHub HTTP retrieval succeeds instead. Records:
[official-source checks](../../.scratch/migration-specs-validation/official-source-checks.json).

All inherited failed/unavailable checks and five missing historical report
links remain in the assessment. This run does not reinterpret them as green.
Task-specific commands, candidates/reviews, attempts and integration status
will be appended here before declaring any executable task accepted.

TOOL-1 evidence: [install record](../../.scratch/migration-specs-validation/tool-install.json).
Official archive SHA-256 is
`4363e4ed2a3f1eb7f4d38d2d59aed59ce43271c44c16b425e92c89a64761fbe7`;
the command link resolves to the project-owned versioned installation. No PATH
or configuration change, source build or existing-binary overwrite was needed.

## DOC-1 accepted revision and next task

Exact candidate/review/integration: `47d6dbce5af2285d1f886ef63d2d4c7b1de57925`,
reviewed by the fresh Luna/high reviewer against `38bbe42f`. The stale approval
sentence was corrected and re-reviewed. Repository check passes (553 authored,
369 reachable); whitespace and 145 local-link checks pass. Two diagnosed
checker repairs were used: provision fresh-worktree dependencies with frozen
lockfile, then relocate copied historical links to their original read-only
inputs. No checker or quality requirement was weakened.

Dedicated integration worktree: `/tmp/boardstudio-migration-m1-20261001`, branch
`migration/boardstudio-m1`. Fast-forward integration preserves the exact candidate.
The integrated repository check and whitespace check exit 0. See the
[review/integration record](../../.scratch/migration-specs-validation/doc-review-integration.json).
The five other specs and the editor-session spec are technically reviewed;
their runtime acceptance criteria remain unexecuted.

Next eligible task: **P1-CORE**, plan **P1-r1**, the six specs and ADR at the exact
reviewed commit above. Authority is the recorded advance approval plus local
commit/integration authorization. Prototype-only path ownership is
`.scratch/prototypes/p1-core/`; no production adoption. Coordinator implementation
is used because all three existing child threads are retained and native closure
is unavailable; the non-authoring reviewer remains available for exact-candidate
review. A new task-owned worktree is created and checked before any prototype
edit. P1-CAD depends on accepted P1-CORE; P2/P3 remain outside this task set.

## P1-CORE stopped: preserved-provider compatibility failure

Exact authority: P1-r1 and six module specs at reviewed `47d6dbce`, with the
recorded advance approval. Task base `e9950d17ed9d1a067df4660783740edd26343b19`;
owned worktree `/tmp/boardstudio-p1-core-20261001`, branch
`prototype/boardstudio-p1-core`. Coordinator authored only the prototype subtree
and the plan command correction; no provider/source/visibility change.
Protocol increment `cb76481e`; full candidate and exact reviewed revision
`3f25e9d9a6679d551acd6e9439e2f19ca844e0ee`; preserved failure-evidence branch
HEAD `6519a8c2215b68855cd1bc097001199ffd4b10e3`. **No prototype integration**.
The migration branch retains the accepted documentation/spec revision; it adds
only this status/evidence and the verified command-order correction.

Verified results: the unchanged provider native suite passes; four new native
protocol tests had RED/GREEN evidence and pass; the standalone native build,
format, native and WASM worker/host strict Clippy pass. Fresh release worker and
CLI host builds succeed. Root and `/boardstudio/` Chromium probes pass for the
revision-0 fixture: Open/Snapshot, invalid/stale replies, owned-buffer detachment
and close/crash/init caller settlement. Page error inspection is empty. These
checks prove only the tested transport/package path. They do not prove full
provider-field compatibility, M1 application parity, canvas, storage or CAD.

Blocking gate: native Open/Snapshot preserves valid revision `9007199254740993`
exactly. The pinned default serde-wasm-bindgen serializer rejects that u64.
Candidate request and reply serialization both use that default. A bounded real
Chromium reproduction posts that valid revision to the unchanged worker and
observes `9007199254740993 can't be represented as a JavaScript number`. The
provider compatibility assertion exits 1. The diagnostic harness exits 0 because
it observed the expected regression; that is a diagnosis, never a passing
compatibility gate. The independent Luna/high reviewer confirmed the critical
blocker and inspected the runtime addendum. Runtime routing is independently
confirmed again by Codex state DB/turn context, with no model/provider switch.

Two diagnosed repairs were used: wasm-pack argument ordering (pack-owned flags
precede Cargo extra options), then a host nested conditional rejected by strict
WASM Clippy. Affected release builds and both prefix browser checks were rerun
after the host repair. No third repair was attempted. P1-CORE remains unchecked
and unaccepted; P1-CAD has no task worktree, implementation or build. Read-only
inventory verified its existing public exports/build path and Podman capability;
that is not CAD acceptance. No safe task remains in this authorized graph.

[Blocked review, commits, commands and ownership](../../.scratch/migration-specs-validation/p1-core-blocked-review.json),
[compatibility diagnosis](../../.scratch/migration-specs-validation/p1-core-large-integer-compatibility.json),
[Chromium reproduction](../../.scratch/migration-specs-validation/p1-core-large-integer-runtime.json)
and [failed gate](../../.scratch/migration-specs-validation/p1-core-large-integer-contract-failure.log)
are retained here. Complete source/build/native/browser logs and generated
artifact hashes remain in the preserved prototype worktree and its local branch.
No failed worktree or uncommitted work was deleted; protected inputs are untouched.
All three completed child threads remain retained because supported archive
failed and no close-thread operation is exposed; their results are preserved.

Resume from the current integration branch and retained failed candidate, with
this blocker resolved in a separately bounded continuation. Preserve full u64
and arbitrary-JSON value meaning across both request and reply paths; add a
meaningful native/WASM regression oracle for those values, rerun affected locked
builds and both deployment prefixes, and review the exact new candidate. The
exhausted repair budget must be addressed explicitly before another repair run.
Do not adopt the prototype, infer CAD readiness or remove the React fallback.
P2/P3, archived failed checks and five missing historical reports remain open.
