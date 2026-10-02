# BoardStudio milestone continuation

Current priority: **100% frontend Dioxus migration**. See [frontend v1 roadmap](DIOXUS-FRONTEND-V1.md), [task graph](../../.scratch/dioxus-frontend-v1/PLAN.md) and [current run](dioxus-frontend-v1-run.json). The 2026-10-01 clarification prioritizes porting all existing TSX, themes and interface behavior. F1 is implemented and verified as the first shell/theme increment ([handoff](../../.scratch/dioxus-frontend-v1/evidence/handoff.md)); The user-requested F3a correction now restores default keycaps, five Layout layers and the shared footprint toggle ([current demo/evidence](../../.scratch/dioxus-frontend-v1/evidence/layout-layers/handoff.md)). Full F2–F9 remain open. Existing engines/providers remain underneath. The prior M1 run below is retained foundation/evidence, with its acceptance limits unchanged.

## Current frontend planning — 2026-10-02

The user requested remaining-work planning with multiple agents for each workflow.
Workflow authors and different reviewers expanded F2–F9 into 58 workflow slices,
two shared integration tasks and two bounded adapter investigations. The
[plan](../../.scratch/dioxus-frontend-v1/PLAN.md),
[ownership and dispatch](../../.scratch/dioxus-frontend-v1/EXECUTION.md),
[machine graph](../../.scratch/dioxus-frontend-v1/tasks.json),
[source coverage](../../.scratch/dioxus-frontend-v1/coverage.json) and
[review resolutions](../../.scratch/dioxus-frontend-v1/evidence/planning/review-resolution.md)
are the active frontend execution documents. First visible tranche: project
library, panels/drawers and Layout tree/selection; other workspace work joins only
its exact dependencies. The graph separates starts from acceptance joins.

Planning baseline `c827c4e6`; executable `f44a3d1b` and its demo/build are unchanged.
No frontend milestone is closed by this documentation update. Existing M1 limits,
actual screen-reader host gap and final concrete cutover approval remain separate.
The retained production M1 run below is historical foundation, not the active
frontend dispatch queue.

Retained foundation run: [remaining M1 production plan](../../.scratch/m1-production/PLAN.md),
[canonical state](m1-production-run.json) and
[editable candidate handoff](../../.scratch/m1-production/HANDOFF.md).
The reviewed implementation includes pointer allocation repairs at `b9748745`.
Its complete maintained rebuild and bounded root/subpath smoke pass. Active-project
startup restoration and physical-instance selection pass focused online root
checks on reviewed source `a49bb798`; its complete rebuild and final race/offline
checks now pass. Earlier `8f509433` build and
root/subpath browser checks pass, with scoped resource and same-input STEP
geometry evidence retained. All five final paired pointer sessions pass the unchanged caps; unchanged reference gates completed
with UI/live failures and an ineligible frozen CAD comparison; relevant screen-reader testing is blocked. Earlier
sections retain historical proposals, failures and accepted feasibility work.

Initial specification-run base: `96dd51d3e790c28f5554a8c9888a147c8814e2a7`.
Initial coordinator worktree: `/tmp/boardstudio-migration-specs-review-20261001`.
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

## Historical initial task set

| Task | Exact scope / authority | Status |
| --- | --- | --- |
| DOC-1 | Complete and technically validate the six mapped M1 specs; preserve accepted inputs | Complete: reviewed and integrated `47d6dbce`; documentation checks only |
| TOOL-1 | Install official Dioxus CLI **0.7.10** selected by ADR 0003; verify official digest and `dx --version`; add a command link only if absent | Complete: official archive digest and `dioxus 0.7.10 (57d6794)` verified, exit 0 |
| P1 | Existing worker/CAD packaging charter; exact bounded [P1-r1](../../tasks/plan.md), task set P1-CORE → P1-CAD in root TODO | P1-CORE accepted via reviewed `cce8e78b`; original P1-CAD blocked run preserved; P1-CAD bounded feasibility accepted through reviewed/integrated `8015b57f` |
| P1-CAD-F1 | Explicit “do it” approval of [P1-CAD-F1-r1](../../.scratch/prototypes/p1-cad/evidence/findings-repair/PLAN.md): two serial reviewed findings | Complete: exact source reviewed, integrated and validated; [current handoff](#p1-cad-f1-bounded-findings-repair) |
| P1-U64 | Explicit renewed defect-only [P1-U64-r1 authority](../../.scratch/prototypes/p1-core/evidence/u64-repair/task-start.json) | Complete; exact source `cce8e78b` reviewed, integrated and validated; [handoff](#p1-u64-accepted-source-and-resumable-handoff) |
| P2-LIFECYCLE | Explicit “approved” response to exact [P2-r1](../../.scratch/prototypes/p2-lifecycle/PLAN.md), proposal SHA e9a715cb…; one bounded lifecycle task | Blocked before executable implementation: unchanged renderer fmt fails; [handoff](#p2-r1-preflight-blocked) |

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

Initial CLI closure failed: `codex archive` for the completed smoke thread
returned exit 1. No direct close-thread operation was identified in the exposed
collaboration interface, so completed threads were retained during implementation
and the three-thread cap was respected. Final deferred-tool discovery located
`mcp__codex_tui__set_thread_archived`; it successfully archived all three completed
children after preserving their results. Actual state DB now independently shows
`archived=1` for each. No direct state-DB mutation or permission change was used.
See [final closure record](../../.scratch/migration-specs-validation/thread-closure-final.json).

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

## Historical DOC-1 accepted revision and next task

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
was used while all three existing child threads were retained after the CLI
closure failure; the non-authoring reviewer performed exact-candidate review.
The later-discovered deferred archive tool resolved closure before final handoff. A new task-owned worktree is created and checked before any prototype
edit. P1-CAD depends on accepted P1-CORE; P2/P3 remain outside this task set.

## Historical P1-CORE stopped: preserved-provider compatibility failure

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
All three completed child threads have now been archived through the supported
deferred TUI tool after preserving their results. The earlier CLI archive failure
is retained as historical preflight evidence, with final resolution above.

Resume from the current integration branch and retained failed candidate, with
this blocker resolved in a separately bounded continuation. Preserve full u64
and arbitrary-JSON value meaning across both request and reply paths; add a
meaningful native/WASM regression oracle for those values, rerun affected locked
builds and both deployment prefixes, and review the exact new candidate. The
exhausted repair budget must be addressed explicitly before another repair run.
Do not adopt the prototype, infer CAD readiness or remove the React fallback.
P2/P3, archived failed checks and five missing historical reports remain open.

Final handoff review: the independent reviewer inspected exact metadata candidate
`113689f2eb7b07f64da97d5e3b1ab7ede3b1eabf` against `e9950d17` and found no
material inconsistency. It confirms no prototype integration, the failed valid-u64
gate, two exhausted repairs, unstarted CAD and open M1 acceptance. The subsequent
closure/protected-input audit updates only administrative records. All 31
protected source inputs still match their initial hashes; the migration and
failed-prototype working trees are clean. Download/source/check logs remain
preserved as untracked run artifacts in the original review worktree.

## Renewed bounded P1-U64 repair

User request `$ask-matt resolve this u64 serialization falure` explicitly
authorizes this defect repair after the previous P1-CORE run stopped. Scope:
P1-U64-r1, one task, the approved P1 transport seam, preserve full u64/i64 and
nested arbitrary-JSON values in both directions. Reuse the accepted specs/ADR
and diagnosis. Import the failed prototype from immutable `6519a8c2` into a
new worktree based on `8b293b08`; preserve the original failed worktree/branch.
See its exact [authority, scope and checks](../../.scratch/prototypes/p1-core/evidence/u64-repair/task-start.json).
The previous two attempts remain historical; this explicit new repair run has
a two-attempt limit. No CAD task, kernel/API change or prototype production
adoption is part of this user request. The Matt router uses diagnosing-bugs,
TDD and two-axis code review. Existing local Markdown tracking is mapped for
the skills; no repository-wide agent configuration or external tracker changes.

### Historical development checkpoints

The following records describe checkpoints before acceptance; the accepted
source and current completion are recorded in the next subsection.

P1-U64 development evidence: the strengthened public host/worker regression failed
with the default codec on `i64::MIN` before the revision cases completed
([RED](../../.scratch/prototypes/p1-core/evidence/u64-repair/red-command.json)).
Rust serde_json opaque text frames replace the four host/worker conversion calls;
engine/provider source, typed envelopes, manifests, lockfile, versions and buffer
transfer are unchanged. Fresh locked worker and Dioxus root/subpath release
builds and both Chromium assertions passed at that checkpoint. The original
P1-CORE failure records remain historical. Candidate 55ae had not yet completed
exact Standards/Spec review or integration validation, so completion stayed open.

Candidate `55ae024a` was invalidated before integration: an extra isolated
finite-f64 characterization found default serde_json parsing can change the
last bit; both independent reviews report that precision blocker. The stronger
real host/worker browser test reproduced it (exit 1). Repair 2 enables the
already-pinned library's float_roundtrip feature; no version or lockfile change.
This feature is unified with the probe engine dependency, so the full provider
suite is rerun with that feature, in addition to fresh protocol, build, lint
and browser gates. No repair attempts remain after this correction.
Repair 1 only packaged raw Cargo output losslessly as gzip to satisfy the
unchanged staged whitespace gate; all raw bytes and SHA are retained.

The final repaired source passes the full provider suite (334 passed, the same
six pre-existing opt-in tests ignored), four native transport tests/build, fmt
and all native/WASM Clippy gates. Both fresh release browser paths preserve
the original lifecycle/transfer assertions plus integer extrema and the new
f64 counterexamples/normal/subnormal/max values. The isolated finite-f64 probe
checks 9,999 values with no bit changes. Exact source/artifact hashes and raw/
gzip byte sizes are retained; performance and production adoption remain open.
At that checkpoint, the repaired source was awaiting repeated exact-candidate
review and integration validation; both later passed as recorded below.

### P1-U64 accepted source and resumable handoff

Repeated independent Standards and Spec reviews report zero material findings
for exact corrected candidate `cce8e78bc107cadb39a8457f0ed17e3e57035181` against
integration base `8b293b0847b50b865334dd9078e8e2b20a84966e`. Both requested and
observable runtime routes are gpt-6-luna/high. Results and prior invalidated
reviews are preserved; both reviewer threads are archived. The read-only smoke
used observable gpt-6-luna/medium. Only the coordinator wrote or integrated.

The dedicated `migration/boardstudio-m1` branch fast-forwarded to that exact
reviewed candidate (exit 0). The resulting checkout passes fresh native
transport tests/build, fmt, repository and whitespace checks. All eight source
hashes and both release host/worker asset sets match the reviewed candidate.
Chromium evidence is reused from those exact unchanged artifacts, not claimed
as a fresh integration-worktree browser build. See [integration validation](../../.scratch/prototypes/p1-core/evidence/u64-repair/integration-validation.json),
[separate reviews](../../.scratch/prototypes/p1-core/evidence/u64-repair/accepted-candidate-reviews.json)
and [task status](../../.scratch/prototypes/p1-core/evidence/u64-repair/task-status.json).

P1-U64 is complete; the P1-CORE bounded packaging/transport gate is now accepted.
P1-CAD is next eligible but remains unstarted and outside this defect-only run.
P2 rendering, P3 durability/offline, representative layout/case parity, Undo,
CAD/export and performance remain open; no prototype code is promoted.
Two diagnosed repairs were used (lossless log packaging, exact float parser);
no unlimited retry, version/provider routing change or gate weakening occurred.
The failed P1-CORE worktree, new task-owned worktree and all protected work are
retained. No push, deploy or main merge was performed.


## P1-CAD blocked reference comparison

The explicit `$implement /tmp/boardstudio-migration-m1-20261001/docs/migration/RUN.md`
request resumes the approved **P1-r1 P1-CAD** task after accepted P1-CORE.
Exact integration base: `c1605a1c6018a77ac7636143bc0dda3fc8d86498`; authority
is the recorded advance approval plus this explicit continuation. No architecture
rediscovery, competing plan, P2/P3 or automatic production promotion is included.
Owned worktree: `/home/chris/.local/share/boardstudio/worktrees/p1-cad-20261001`,
branch `prototype/boardstudio-p1-cad`; isolated source candidate
`f0fe6e2f77ad53bd63f67056a6d08f528bac5a7c`. Source stays on that branch.
Canonical integration contains status/evidence only. Full commits, commands,
logs, review routing and worktree ownership are in the
[task record](../../.scratch/migration-specs-validation/p1-cad-blocked-review.json).

Preflight verifies Codex0.159.3 recognized configuration, pinned Dioxus0.7.10/
bindings and actual local CAD toolchain image, Git base/status/worktrees,
permissions, frozen dependencies and repository commands. A fresh read-only smoke
requests Luna/medium; Codex DB and turn-context confirm runtime model/effort.
Fresh Standards/Spec reviews request Luna/high, independently confirmed likewise.
The smoke child is archived; review routing snapshots are taken before closure.
After final results are retained, supported reviewer closure is recorded separately.
No writes were delegated and no alternate model/provider or permission change was made.

Verified on the final prototype source: three meaningful native transport/cache
RED/GREEN tests, native build, formatting and strict native/WASM host/worker Clippy;
CAD typecheck; fresh CAD package/worker and both pinned release hosts; actual local
Chromium root and `/boardstudio/` assertions. Existing CAD native tests pass23 with
four unchanged ignored diagnostic benchmarks; all49 JavaScript CAD regressions
pass with zero skipped. The copied holed plate is prepared by the public core
example. Real workers return preview/export meshes with expected bounds and
720mm³ volume, committed STEP that reopens, exact bodyKey/full-delta identities,
active/queued cancellation without completed-cache advancement, raced cache
consumption, retry and close/crash/init settlement. Actual owned-buffer detachment
and byte counts, remaining copies, source and served artifact hashes are retained.
Repository/protected-input checks pass; provider sources/manifests/locks are unchanged.
These are bounded fixture/package results, not full-M1/platform/performance parity.

**Failed gate retained:** `node evidence/check-step.mjs` exits1 at the independent
reference preview vertex comparison: isolated cold preview192 versus reference
preview-after-export96. Literal bounds and volume agree; root STEP/solid/export
checks preceding this assertion pass, but the harness stops before completing the
subpath independent comparison. Existing `cached_region` may use the planar
mesher; an export miss uses the kernel mesh and populates `PREVIEW_CACHE`, which
later preview returns. This supports a cache-state mismatch explanation, not a
passing equivalence result or a confirmed geometry regression. The failing
assertion is unchanged; no baseline/test normalization or kernel alteration occurs.

Two diagnosed repairs were used: build the required public core preparation
executable before the unchanged CAD native suite; then satisfy pinned Rust1.98
Clippy with a buffer type alias, if-let chains and `as_chunks`. All affected final
checks/builds/browser probes pass. No third implementation repair is attempted.
Verbatim upstream docs and generated STEP include trailing spaces; gzip packaging
preserves their exact bytes/hashes and raw owned-worktree copies for review rather
than normalizing artifacts. This administrative evidence packaging changes no
assertion, provider or check. Initial failed logs and browser artifacts remain.

Exact two-axis review keeps the candidate **blocked/unaccepted**. Standards finds
one documented gap: the README describes the experimental worker boundary, but
CONSTRAINTS requires an architecture-document entry too. Spec finds the unresolved
reference/cached-uncached comparison; no scope creep or weakened checks is reported.
The source review and final evidence-only follow-up are separately recorded.
Architecture documentation and equivalent-cache-state control evidence remain open;
neither is silently supplied by promotion of the prototype. Existing CAD's JS
Number revision ABI above2^53-1 is also unproven here; this fixture uses revision7.
Accepted P1-CORE's full-u64 text-frame proof remains intact.

No safe unblocked task remains in this defined graph. Resume from the preserved
branch/worktree with a separately bounded repair that explicitly addresses the
exhausted attempt limit, compares equivalent cold/warm preview/export cache
lifetimes, resolves the architecture documentation gap, and reruns affected gates
and exact review. Preserve the failed comparison and all work. P1-CAD is unchecked;
P2/P3, full M1 and historical missing/failed acceptance remain open. No push,
deploy, main merge, protected-worktree modification or failed worktree deletion.


Final P1-CAD execution record: exact status-only integration
`03da0f1d7d5ccd976a806bdc41bb8ce2caf3602e` was independently reviewed on both
axes against `c1605a1c`; its diff contains only the five canonical status/evidence
files. Integrated repository and whitespace checks pass, all31 protected inputs
still match, and no prototype/provider source was integrated. All three run
children are now archived through the supported tool with independent DB
confirmation. The preserved source/evidence branch HEAD is
`3b2008408b50d282b0d752caedb7b6ff41bbb4b7`; only administrative result/closure
records follow the reviewed source and handoff. This final audit adds execution
facts without accepting or changing the implementation candidate.
[Exact integration/check/closure record](../../.scratch/migration-specs-validation/p1-cad-status-integration.json).


## P1-CAD-F1 bounded findings repair

The user's **“do it”** explicitly approves the preceding two-finding repair
proposal: documentation correction and a conditional reference-harness defect
correction through diagnosis, TDD and fresh two-axis review. Exact scope is
**P1-CAD-F1-r1**, based on `90d857156fae5f8279840763b10d8571090b8724`;
[task authority](../../.scratch/prototypes/p1-cad/evidence/findings-repair/task-start.json)
and [bounded plan](../../.scratch/prototypes/p1-cad/evidence/findings-repair/PLAN.md)
record the existing specs/ADR, ownership, public seams and required gates before
implementation. Coordinator-owned worktree:
`/home/chris/.local/share/boardstudio/worktrees/p1-cad-findings-20261001`,
branch `prototype/boardstudio-p1-cad-findings`. The original failed worktree and
immutable source `3b2008408b50d282b0d752caedb7b6ff41bbb4b7` remain unchanged.
Its two exhausted attempts are historical; this newly scoped run permits two
diagnosed repairs and has used one (reference signed-zero byte encoding). No implementation writes are delegated.

Current preflight verifies actual checkout/status, permissions, tool versions,
frozen dependencies, configuration and source/provider identity. A fresh read-only
smoke requests Luna/medium; independent Codex DB/turn-context metadata confirms
that route, and supported archival is independently confirmed. Configuration's
pre-existing rollout/DB warning is retained. No framework/API/pin change requires
new framework research; the existing version-matched official source evidence
is reused. Protected inputs and original failed worktree are checked unchanged.

The original 192-versus96 assertion and a smaller equivalent assertion both fail
for the expected reason. Four independent public CAD facade processes establish:
preview cold192; export cold96; preview→export both192; export→preview both96.
Matching paths have exact position/normal byte hashes, the same body ID/revision,
finite unit normals, literal volume720 and expected bounds. Identical source and
WASM artifacts rule out artifact drift. This proves shared-reference cache-order
contamination: the browser used independent preview/export modules, while the
reference exported before previewing in one module.

The corrected oracle uses separate reference processes and keeps exact vertex
assertions; it additionally requires complete mesh position/normal byte equality
with actual browser results at matching lifetimes. A missing-byte-evidence RED
precedes Rust report instrumentation; only prototype reporting changes. Existing
provider/kernel, schema, pins, worker lifecycle and geometry tolerances are
unchanged. Independent libcascade reopening also compares cold export and both
shared cache orders for one solid, volume720 and bounds. Original failed scripts
and logs remain historical evidence, not passing gates. Architecture documentation
now records the experimental worker/CAD boundary, owners, errors/cancellation,
cleanup, measured copies/costs and named retirement criteria.

**P1-CAD bounded feasibility accepted at exact source `8015b57f`.**
Three native protocol/cache tests, native build, fmt, strict native/WASM host/worker
Clippy and CAD typecheck pass. Fresh provider builds pass23 native CAD tests with
four existing ignored benchmarks and all49 JavaScript regressions with zero skipped.
Fresh worker and both Dioxus releases pass actual Chromium lifecycle/transfer gates
at root and subpath. Complete preview/export position and normal bytes match
independent reference lifetimes; cold export and both cache orders pass independent
STEP one-solid,720mm³-volume and bounds comparisons. Repair1 preserves signed
zero in reference byte transport; its expected RED and original typed-byte hash
GREEN are retained. No CAD provider behavior or tolerance changes.
[Final verification records](../../.scratch/prototypes/p1-cad/evidence/findings-repair/verification-summary.json)
link commands, exit statuses, compressed logs and source/served artifact identities.
Fresh Standards and Spec reviews report zero material findings for exact
`8015b57f7d149ceb8d14aa5c5adff40bb18deab5` against base `90d85715`; requested
and independent runtime routes confirm Luna/high. Both reports are preserved
[separately](../../.scratch/prototypes/p1-cad/evidence/findings-repair/REVIEW.md).
The coordinator fast-forwarded dedicated migration/boardstudio-m1 to that exact
source, then verified the actual integration checkout with native tests/build,
fmt, repository and whitespace checks. All source and served-asset hashes match
the reviewed candidate; provider Git trees are unchanged. Browser evidence is
reused from exact unchanged task-owned artifacts, not claimed as a fresh browser
build in the integration worktree.
[Integration validation](../../.scratch/prototypes/p1-cad/evidence/findings-repair/integration-validation.json)
and [task status](../../.scratch/prototypes/p1-cad/evidence/findings-repair/task-status.json)
record bases/candidates/commits, commands/exits/logs, worktree ownership and limits.

No next eligible approved task remains. This final administrative handoff is
reviewed separately before its integration; it changes no executable source.
Reviewer closure and final integration commit/checks are persisted after the
review in the task-owned `target/final-handoff.json` (and integration-owned copy),
without rewriting immutable reviewed commits. All worktrees/failures remain.
P2/P3, full-M1, CAD revisions above JS Number's safe range, performance and
production adoption remain unproved and outside this run. No push/deployment/
main merge or protected-worktree mutation is authorized.


## P2-r1 preflight blocked

The user explicitly **“approved”** the proposed P2-r1 next task set after the
completed P1 handoff. Exact original proposal SHA-256:
`e9a715cbc6049cd9e6baf9dddb0db20aedad33124e7f23ef4888053e36e700db`.
[PLAN](../../.scratch/prototypes/p2-lifecycle/PLAN.md) preserves that supplied
proposal's initial approval-pending wording as history;
[task-start](../../.scratch/prototypes/p2-lifecycle/evidence/task-start.json)
records the actual approval, exact specifications, ownership, pre-agreed public
engine/gesture/lifecycle seams. Preflight started at zero repairs; documentation/
API evidence repair1 and explicit README-link repair2 are recorded separately;
the two-attempt limit is exhausted. Base:
`7ed7b5ec9a58449615b05bf7e95eb888711d864b`. Coordinator-owned worktree
`/home/chris/.local/share/boardstudio/worktrees/p2-lifecycle-20261001`, branch
`prototype/boardstudio-p2-lifecycle`; actual clean checkout was verified before
recording plan/evidence. No executable probe source is written.

Codex0.159.3 strict-config doctor succeeds; installed Dioxus0.7.10/CLI, Rust1.98
and pnpm12.6 match accepted pins. Frozen dependency installation succeeds with
no manifest/lock changes. Configuration retains max3 child threads and Luna/high
defaults; a fresh read-only Luna/medium smoke is independently confirmed by DB/
turn-context, preserved and archived through the supported tool with DB proof.
No writes were delegated. Context7 resolves exact Dioxus0.7.10 and tagged
side-effect sources. The initial review found independent API-source verification
incomplete; repair1 finishes mount/DOM conversion, pointer events, effects and
use_drop using five tagged official sources byte-identical to installed0.7.10.
Installed web-sys0.3.106 verifies capture/RAF/observer/SVG focus signatures.
[API record](../../.scratch/prototypes/p2-lifecycle/evidence/API.md) distinguishes
these source facts from unperformed compile/runtime proof. A provider API source smoke finds resize/dispose/
generated free, but proves no actual GPU disposal or painted-frame behavior.
Doctor's standalone sandbox metadata does not override the current never/
danger-full-access execution permissions. No routing/config/permission changes.

**Required failed gate:**
`cargo fmt --manifest-path renderer/Cargo.toml --all -- --check` exits1 with
formatting differences in `renderer/src/geometry.rs`, `math.rs`, `mechanical.rs`
and `wasm.rs`. All four files are byte-identical to the approved base. This is a
pre-existing formatting failure, not a P2 regression; its raw output/SHA and
provenance are in [blocker](../../.scratch/prototypes/p2-lifecycle/evidence/blocker.json).
The approved file boundary permits the isolated prototype and coordinator docs;
it excludes renderer changes. No renderer edit or gate waiver is inferred.

A concrete [formatting-only patch](../../.scratch/prototypes/p2-lifecycle/evidence/proposed-renderer-format.patch.gz)
is generated from independent temporary copies with rustfmt; it is **unapplied**,
and `git apply --check` passes.
[Exact scope extension](../../.scratch/prototypes/p2-lifecycle/evidence/scope-extension.json)
requests only those four source files, preserving renderer behavior/API/pins and
the strict formatter gate. The current two-attempt run is exhausted after documentation/evidence repairs.
Proposed **P2-r1-R1** needs explicit approval of this four-file addition plus an
explicitly renewed two-attempt limit, with no automatic reset or third repair.
After that bounded approval, create a new task-owned worktree from the exact
recorded integration revision, preserve this exhausted P2 worktree, apply the
patch there, run all affected renderer checks, then resume the
original P2 TDD/build/browser/lifecycle/paired-resource/review/integration gates.
Native/WASM tests/lints/builds, reference frontend, actual Chromium/P2 painting,
gesture/history, resize/DPR, focus/a11y, resource cleanup and paired acceptance
are unperformed. No gate is silently omitted, green or weakened.

No safe unblocked task remains in this defined one-task set. Preserve this
worktree/branch, failed output and patch; P2 remains unchecked. P3, full M1,
CAD-u64/provider changes and production adoption remain outside approved scope.
Only status/evidence is eligible for local integration after fresh exact
Standards/Spec review; no executable probe, provider code or format patch is
applied. Final review/integration/check/closure commits and results are persisted
in the owned `target/final-handoff.json` and integration-owned copy after review.
No main merge, push, deploy or protected-worktree changes.

## P2-r1-R1 approved Luna Fast renewal

User **approved** the exact renewal/execution policy after the Wayfinder routing
investigation. Proposal commit e4d622bd and SHA-256
`8d44c8b16fc5f7e4245dc3894ab912d0e14fc19713a7d1b13f073923ebfe5d45`;
[authority](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/task-start.json),
[approved decision](../../.scratch/dioxus-browser-trial/issues/08-choose-p2-execution-policy.md).
Integration base689f8962; new owned worktree
/home/chris/.local/share/boardstudio/worktrees/p2-lifecycle-r1-20261001, branch
prototype/boardstudio-p2-lifecycle-r1. Checkout authorization/fixture commit
e5383450 precedes the assigned writer. Original exhausted P2 worktree remains.

Before executable edits, copy the full existing Reviung41 project fixture;
105292 bytes, SHA b577dd2009fffbf00489cc8d0f2ccc088861f62a7f30af42470d35c11534bdae,
existing unlocked switch matrix/main-right-keys/r0c0. No document fields are
removed. Initial metadata preparation assumed bare SW references and failed;
administrative repair1 uses the actual definition kind. Its partial static-copy
commit is preserved; authority is completed before writer assignment. This does
not consume the renewed implementation repair budget or conceal a failed gate.
New finite limits are two implementation and two administrative repairs.

Fresh read-only Luna/medium smoke verifies actual clean base/worktree, installed
Codex0.159.3, Dioxus0.7.10, Rust1.98, pnpm12.6, wasm-pack0.15, agent-browser0.38.1
and Chromium153. Required commands and exact unapplied raw patch/four-file hashes
are available. Unsupported git apply --stdin fails read-only; the justified stdin
hyphen fallback applicability check passes. DB and turn_context independently
confirm Luna/medium. Results are preserved before supported archival with DB proof.
One coding writer requests Luna/high Fast/priority; independent DB/turn_context
confirm Luna/high. Current config requests Fast; runtime served tier is absent,
not inferred from agent self-identification or configuration. No model/config/
permission change, paid external service or protected-worktree write occurs.

The exact approved formatting patch is applied; renderer fmt passes. Required
native strict Clippy fails with14 lib/17 test renderer errors, including existing
assets/wrl/geometry/dead-code patterns; WASM strict Clippy fails with29 lib/33 test
renderer errors. Vendor warnings are separate. This blocks
acceptance and is not silently suppressed or fixed outside the exact-format
allowance. [Worker evidence](../../.scratch/prototypes/p2-lifecycle/evidence/renewal-worker/)
retains commands/output. The approved interim-demo clause allows safe independent
P2 implementation/build/runtime work to continue, clearly incomplete until every
original gate is met. No executable candidate acceptance/integration is claimed.
Root alone updates canonical status and integration; writer touches only its
assigned paths and commits coherent verified increments. Final candidate/review/
integration results and all failures/unperformed gates will be recorded here.

Administrative repair2: the writer's standalone prototype `cargo fmt --all`
also formatted three core path-dependency files outside its ownership. The
writer reported the command and preserved both the exact patch and complete
post-format file bytes in the worker evidence directory before reverse-checking
and reversing only its own accidental formatting. All three files then matched
HEAD; no unrelated changes were reset or discarded. The administrative budget
is now2/2; implementation repairs remain0/2 at this checkpoint. Later formatter
checks must target owned prototype sources without recursively formatting
providers. The protected assessment's31 recorded files, HEAD and historical
dirty status remain unchanged in the coordinator's read-only preservation audit.

Implementation repair1: Chromium rejected a truncated packaged Dioxus WASM
module; coordinator Node validation independently confirmed the same invalid
2,281,472-byte staged artifact. Upstream Rust and direct optimizer artifacts
validated. The host's /tmp tmpfs was nearly full, while the owned home filesystem
had adequate capacity. An unchanged locked Dioxus build using a task-local
TMPDIR produced valid7,448,875-byte WASM without pin, optimizer-flag or gate
changes. Failed output remains preserved. The explicit staging script supplies
stable sibling worker/renderer URLs alongside Dioxus's hashed release assets.

The interim subpath release at http://127.0.0.1:4392/boardstudio/ runs. In its
independent task-owned Chromium session the coordinator inspected actual visible
board pixels, committed a real mouse drag and asserted exact visible target
restoration through one Undo and Redo. The browser fetched the repaired WASM;
initial errors were empty. [Interim coordinator evidence](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/root-interim-browser.json)
records the artifact and limits. This is not the final reviewed candidate.

Implementation repair2: live DPR-only emulation left the mounted canvas at DPR1,
while remounting at DPR2 sized it correctly. The scoped owned resolution-query
notification and teardown path now passes native browser notification checks;
locked r2 releases serve root4394 and subpath4393. The implementation budget is now2/2; another
diagnosed failed acceptance gate cannot authorize a third repair. Required
baseline provider lints and all unperformed checks remain explicit; no acceptance
or executable integration is inferred from this working interim build.

Coordinator checks the r2 root release in a fresh owned Chromium session: visible
green board pixels, native DPR1→2 on the same canvas at unchanged CSS dimensions,
and repeated mount/unmount with instrumented actual listeners, ResizeObserver,
RAF and generated renderer dispose/free. Four normal unmounts return tracked
resources to zero. Actual WEBGL_lose_context reports explicit failure and stops
frames; the fifth unmount cleans resources and a new mount succeeds. These are
scoped lifetime assertions, not a whole-process GPU-memory or full-editor claim.
[Independent checkpoint](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/root-browser-checkpoint.json)
retains commands and limitations. A reused session navigating from4392 to4394
failed initialization and requested renderer JS from the previous4392 origin;
the fresh session fetched the correct4394 assets. The discrepancy remains
unexplained and is not erased by the successful fresh-session result.

Full reference app tests now pass81 suites/469 tests after copying the exact
accepted CAD artifact into the new worktree's ignored output. The frontend build,
contracts/tests, repository and boundary checks pass. Full native core tests and
renderer26 tests pass. Read-only core fmt still fails on three unchanged provider
files; core strict Clippy fails69 library/76 library-test diagnostics. The exact
four-file renderer-format allowance does not authorize those provider repairs.
Provider gates remain blocking. Remaining paired reference, accessibility,
resource and final exact-candidate review evidence is still being collected;
this checkpoint does not mark P2 complete or integrate its executable code.

The r2 root coordinator session also passes trusted Tab reachability to the
named SVG surface, a keyboard move followed by exact one-step Undo, and a real
mouse drag followed by exact one-step Undo/Redo. The coordinator independently
reruns the locked native prototype suite:7 tests pass. [Scoped probe record](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/root-browser-probe.json)
retains the full command/output ledger. Worker axe reports html-has-lang at both
prefixes; contrast and screen-reader evidence are incomplete. This newly found
accessibility failure remains blocking with no third implementation repair.

**Additional verified behavior blocker:** at1280×577/DPR1, the copied target's
pointerdown at(393,442) captures the SVG coordinate frame before focusing it.
Focus scrolls the page139px and moves the SVG top273.125→134.125; subsequent
move/up at(423,466) use the shifted frame. The actual public worker commits
pose(147.285,-51.02)→(160.4281293,-122.4876778), revision3→4, moving the target
offboard. The corresponding reference remains unscrolled. Different fitted
view scales mean identical CSS deltas alone do not establish geometric parity;
the captured-frame discontinuity is the directly observed defect.
[Exact coordinate/request/pose evidence](../../.scratch/prototypes/p2-lifecycle/evidence/renewal-worker/p2-offboard-focus-repro.json)
and screenshots are retained. The coordinator separately observes focus-induced
scroll116px at1280×600; its mutated overlapping fixture did not reproduce this
exact target move, so that observation is only corroborating frame evidence.
The earlier1280×720 trusted drag/Undo/Redo pass remains scoped to that viewport.
No source repair follows the exhausted2/2 implementation budget.

A controlled delay of the actual generated renderer module import, followed by
panel unmount before the Promise resolves, makes zero WebGL allocations and
leaves zero tracked listeners/observers/RAF; stale status is not published and
a later fresh mount succeeds. [Coordinator late-import/focus evidence](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/root-late-import-and-focus.json)
records this test and its instrumentation. It does not waive the confirmed drag,
accessibility or provider gate failures. The source candidate stays isolated;
only accurate status/evidence may enter the dedicated migration branch.

The writer freezes owned prototype/evidence source at
`b2cf39db55db7826368031e61bb1b033bb652800`; its index is released. Results are
preserved before supported archival; current read-only DB confirms Luna/high
and archived1. [Final worker ledger](../../.scratch/prototypes/p2-lifecycle/evidence/renewal-worker/validation-ledger.json)
retains actual locked Dioxus build commands, both prefix stages, native334 core
passes/6 existing ignored, renderer26, prototype7, app469, contract/repository/
boundary results and blocked checks. Existing reference browser cancellation/
history tests pass4/4 on the final valid task-temp run at4399; focused part drag/
Undo and keyboard nudge/Undo pass2/2 at4398, retries0/one worker. No provider
source or assertion was changed. Axe raw execution/report was not retained;
that gate is unverified despite the reported violation, and contrast/screen-
reader/focus-equivalence evidence remains incomplete. The coordinator observes
the empty html language attribute directly. No blanket browser acceptance follows.

[Frozen source hashes](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/frozen-source.json)
and [owned server/restart inventory](../../.scratch/prototypes/p2-lifecycle/evidence/renewal-worker/owned-servers.json)
make the incomplete release reviewable. The coordinator independently reruns the
frozen locked native probe:7/7 pass. All31 protected historical file hashes,
HEAD and dirty status match the original preservation record. Fresh Standards
and Spec reviews follow against current integration base689f8962; no executable
candidate integration or task completion is authorized while these gates fail.

The status handoff is prepared separately from689f8962 on
docs/boardstudio-p2-handoff. [Handoff scope](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/handoff-scope.json)
contains documentation, the immutable copied fixture and selected recorded
evidence. It excludes executable Rust/JS/Python, Cargo manifests/locks, WASM and
the renderer formatting changes. Those remain solely in preserved source
`b2cf39db`, full review candidate`57bd6ece`, under
/home/chris/.local/share/boardstudio/worktrees/p2-lifecycle-r1-20261001.
The formatter and build pass claims above describe that owned prototype
candidate; provider sources on the migration branch remain at the prior accepted
base. The copied evidence subset is not a standalone rebuildable prototype.

Final independent reviews completed read-only against source/report candidate
`57bd6eceed3bce9ad9b854121e93d8adc22f7bf8` and integration base689f8962. The
Spec review confirms the short-viewport focus-scroll coordinate defect and rejects
executable acceptance. It also found the inherited prototype README still says
the predecessor run is unstarted and points to its old authority path. That
README records historical predecessor status and was preserved; this renewal
RUN/task status is the current authority. The Standards review found no hard
standards violation, identified only an optional inspection/readability note
about the1,088line web module, and withdrew an initial pointer-capture finding
after confirming existing lost-pointer-capture cancellation. Both reports are
[retained here](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/).
Review completion does not satisfy the failed provider, drag or accessibility
gates. The code candidate is not integrated.

The status/documentation/evidence handoff was committed as
`23b9044a65e3e7cef9d07d46fc15999e946bb44e` and fast-forwarded into the dedicated
`migration/boardstudio-m1` branch. This integration contains no executable P2
source or dependency-manifest changes. The integrated revision passed
`git diff --check` and `pnpm run check:repo` (7/7 repository-check tests; 553
authored modules, 369 production-reachable). The copied Reviung41 fixture SHA-256
was verified against the original. The executable task status remains blocked
and its integration field remains null.

## P2 lifecycle continuation — 2026-10-01

The user's later instruction to continue with parallel implementation and unlimited repair iterations supersedes the finite repair-budget statements in the earlier historical checkpoints above. Work continues only on the isolated branch `prototype/p2-lifecycle-followup-20261001`; no production promotion, push, or main-branch merge occurred. The approved source candidate and current run status are recorded in [task-status.json](../../.scratch/prototypes/p2-lifecycle/evidence/renewal/task-status.json).

The integrated candidate is `feeff1cf9d3ddf1fe76b67403c0125aa7753feaa`, based on `83562529d1b91ca5035a9147cf1db1af12e8a3fc`. A fresh exact-candidate review found no concrete correctness issues. It confirmed the renderer/core lint changes preserve branch and iteration behavior and did not remove matrix projection tests. The short-viewport regression now passes at 1280×577: pointer down does not scroll or shift the SVG frame, the drag commits through the public CoreEngine, the part remains on the board, SVG focus is retained, and the document language is `en`. A locked Dioxus release bundle was rebuilt after the Rust changes and tested in Chromium using Playwright against the staged root route. `/boardstudio/`, full axe, contrast, screen-reader, and keyboard-focus equivalence checks remain unverified.

The full core native tests, renderer 26 tests, prototype 7 tests, native renderer strict Clippy, locked WASM check/build, and Dioxus release build pass. Core strict Clippy now has only three `large_enum_variant` findings on public `EditOperation`, `CoreReply`, and `ArtifactReply`; fixing those requires a public Rust API change, so they remain visible. Renderer strict WASM Clippy has one `arc_with_non_send_sync` diagnostic because three-d 0.19.0's required `from_gl_context` signature accepts `Arc<Context>`. No warning was suppressed or dependency changed. These lint findings and the incomplete accessibility evidence keep broader P2 acceptance open. The exact browser result and final bundle staging record are in [followup-browser-verification.json](../../.scratch/prototypes/p2-lifecycle/evidence/renewal-worker/followup-browser-verification.json).


## Active approved continuation

The user explicitly requested implementation of the remaining-migration plan on 2026-10-01. [The current run](continuation-run.json) supersedes historical finite budgets and stale status text. A persistent isolated branch combines preserved handoff `a15dc95c` and prototype `7ade8731`; original checkouts and stashes are unchanged. Public enum boxing and one scoped three-d Arc lint exception are approved, with wire/file compatibility required. P2 completion and P3 feasibility proceed independently; production adoption remains dependent on evidence. The prior short-viewport browser regression passed at both root and `/boardstudio/`, while exact provider-artifact provenance and remaining accessibility/lifecycle gates still require completion.


### P2 acceptance on the persistent continuation

P2 is accepted at reviewed executable source `e5d4e74868486e052c7a74ab50c5540481136220`.
The approved ABI refactor (`7be0baa2`, integrated as `f7f6787f`) boxes large payloads
without changing JSON/files/generated TypeScript. Strict native/WASM checks pass;
the single scoped Arc exception documents the pinned three-d 0.19 API.
Provider artifacts were rebuilt from recorded source `ff9b7b0e`; final4 Dioxus
root/subpath bundles use those exact verified assets and source `e5d4e748`.
The provenance now includes embedded provider inputs, not only Rust source files.

Browser evidence exposed two further defects and drove focused repairs: cached
renderer programs retained their context after disposal, and relative dynamic
imports could retain an old origin across same-tab deployment changes. Explicit
program-cache teardown plus departing-context release and absolute current-origin
resource URLs pass the regression checks. Both final deployment suites have no
runtime errors or failed requests. Keyboard Undo/Redo, focus, compact and short
viewports, same-canvas DPR, four teardown cycles, context loss, and cancelled late
imports pass. The unchanged reference app passes 469 tests, its build, and six
affected browser interaction tests. Contract and native/WASM boundary checks pass.

[Gate commands, logs and limitations](../../.scratch/prototypes/p2-lifecycle/evidence/continuation/gates.json)
and [fresh independent reviews](../../.scratch/prototypes/p2-lifecycle/evidence/continuation/review-final.json)
are retained. Axe has zero violations and one incomplete check for overlapping
SVG labels; computed contrast is at least 7.41:1, without a speech/readability
claim. The accepted P2 charter does not require a manual speech run. Startup,
size and resource observations do not substitute invented thresholds for absent
budgets. Earlier failures remain preserved as historical evidence.

P3 continues independently with copied data and isolated storage. No production
session/web application, full M1 workflow, full-range CAD integer parity, or
newer-dev provider reconciliation is claimed. Original checkouts/stashes remain
preserved. Repair caps remain removed.


### P3 acceptance and resumable handoff

The approved P2/P3 task set is complete. P3 source `efa2bc4a` was merged with
current integration base `dfbfcb16` at `9efd6711`, then validated with the approved
boxed Rust contracts. Final evidence candidate `5f22e2e11138d8c496a00457bb668ce0960d9a55`
was reviewed and fast-forwarded into `migration/boardstudio-m1-continuation`
without conflicts or executable changes. Both independent Luna/high reviews
have zero blocking findings; Standards retains one optional redundant test-helper
parameter note. No different model/provider or permission escalation was used.

Real IndexedDB evidence shows request success followed by transaction abort,
rollback to revision 3, and retry of the same retained snapshot reaching revision 4
with one engine commit and one Undo entry. All four assets and the copied document
survive archive/reload. The coordinator's actual `app/src/storage.ts` exchange
passes Rust archive → reference import/save/load/archive → Rust reopen, including
byte hashes, Uint8Array representation, active preference and invalid-import
preservation. This does not claim the complete saved-document corpus or UI parity.

Rust service-worker policy runs through a generated synchronous initializer.
Feature-gating page/archive dependencies out of the worker gives a 60,828-byte
policy WASM and an 81,334-byte initializer. The browser proves root/subpath cache
separation, ignoring unrelated cached responses, root-v1→v2 update with subpath
preservation, cached offline navigation and explicit missing-asset failure. A
fresh context with no worker/cache fails offline honestly. Hex-encoded scope keys
fix a collision found during review. Earlier failed TLA and oversized-loader
experiments remain preserved; no general browser-size threshold is inferred.
Forced worker-process termination before cached reopen remains untested.

[Accepted commands and integration evidence](../../.scratch/prototypes/p3-reference-check/evidence/acceptance.json)
and [independent reviews](../../.scratch/prototypes/p3-reference-check/evidence/review-final.json)
are the portable handoff. Native tests, fmt, strict native/page-WASM/worker-WASM
Clippy and locked release builds pass on the exact reviewed source. The
coordinator verified source/asset hashes and repeated the real browser matrix
and reference compatibility checks in the resulting integration checkout.
The original checkout remains `dev@5a472a94` with its unrelated `.scratch/` and
two stashes; no push, deployment, main merge, worktree deletion or automatic
prototype promotion occurred.

Resume from the persistent continuation and [canonical task state](continuation-run.json),
not the vanished `/tmp` checkout or historical blocked checkpoints. There are no
remaining tasks in this bounded P2/P3 run. The next bounded production slice must
reconcile newer encoder/VIK/module/provider changes from `dev`, then deliver the
accepted complete M1 session/web workflow using REVIUNG41 and Sofle. Full-range
CAD revision compatibility remains an explicit gate before complete integer
parity is claimed. Full application parity and React retirement remain later
capability milestones; detailed whole-migration tasks have not been generated.

## Remaining M1 production workflow — 2026-10-01

The user requested to-spec planning of all remaining M1 documentation followed by implementation. [M1-r1 specification](../../.scratch/m1-production/spec.md), [bounded plan and documentation deliverables](../../.scratch/m1-production/PLAN.md), and [current production-slice state](m1-production-run.json) govern this new task set. The completed P2/P3 run stays complete. Newer provider input is main-checkout dev `5a472a94`, fetched into the separate migration repository under a pinned reference. Integration proceeds on `codex/m1-production-20261001` in the persistent continuation; original checkout/stashes and historical evidence stay preserved. No cutover/push/deployment/main merge follows from M1 implementation.

### Approved transparent Rust boxing extension

On 2026-10-01 the user explicitly answered “Approve transparent boxing” for
`ArtifactReply::PreparePreview.result`, `ArtifactReply::ImportModuleBoard.result`,
and the `EditOperation::SetMountedModule` instance/definition/host-connector
payloads. Ticket 01 may implement these `Box<T>` Rust API changes while preserving
JSON, saved files and generated TypeScript. This bounded approval extends the
earlier enum-boxing decision; affected native/WASM lint, tests and contract
checks remain required before integration.

### Production composition checkpoint

Headless session implementation `f2494e57` is integrated at `3f405fcd` with eight
public native tests, fmt and strict Clippy passing. Browser host `303f27e0` is
integrated via reviewed merge candidate `e13a5fb3` at `4a9ca209`. Candidate
presentation/runtime now compiles and passes strict page WASM Clippy at
`6e740a79`; this is source verification only. Real integrated browser durable
recovery, archive, offline, CAD/renderer, STEP, presentation and performance
gates remain open. CAD/STEP worker and offline policy development proceed
independently; their acceptance still follows the task graph.

### CAD and offline implementation checkpoint

Integration `e96cd223` contains the reconciled provider/boxing changes, scoped
browser host, Rust offline policy and CAD worker. The public application suite
passes eleven tests, including save-abort recovery and active-board snapping;
five CAD tests cover snapshot identity, safe-range rejection and payload checks.
Page and CAD-worker WASM library checks pass. Offline minimal-shell browser
proof covers root/subpath separation, offline reopen, update and missing assets;
full application offline acceptance is still open.

Consumer `f5e287b9` adds captured generation, independent STEP worker/export
lifetimes, scope/token checks before delivery, case settings and renderer camera
controls. Numeric/pointer UI repairs and browser storage/archive proofs are in
progress. The full page check currently waits for the interaction worker's
`import_file` adapter. The canvas nominal PCB reference does not claim generator
execution or populated PCB parity. No production ticket is closed by this
checkpoint; final release, exact geometry/readback, resource, accessibility,
performance and Standards/Spec review gates remain open.

### Full-fixture development verification checkpoint

Presentation source is frozen at `23c67f8a`. Browser evidence records numeric
preview/Escape/Enter, saved-library refresh, keyboard list entry/navigation,
pointer threshold/final sample/cancellation, Undo/Redo, Shift range, Ctrl toggle,
Alt snap bypass and a scrollable Sofle layout at 920×500. These results use the
recorded development page and are not complete release/offline acceptance.

Actual IndexedDB abort/retry, all REVIUNG41 and Sofle asset hashes, reload, invalid
archive preservation and Rust → reference storage/archive → Rust exchange pass.
Nine case projections match the actual TypeScript reference, including flipped
instances. Both fixtures pass staged worker preview/exact and independent STEP
readback; max-safe and wide-revision behavior is explicit. The final complete
root/subpath release, Runtime stale/cancel guards, renderer resource lifecycle,
accessibility, applicable performance budgets and independent Standards/Spec
review remain required. No production ticket is closed by this checkpoint.

### Production source and final candidate checkpoint

The production tree at `2040e23b` includes the reviewed export-registration
cleanup and public CAD reply correction. Exact assembly output legitimately
omits bounds; STEP readback continues to require valid bounds. Thirteen session
and twelve web native tests, formatting, application strict Clippy and strict
all-target page/CAD-worker WASM Clippy pass. Independent Standards and Spec
reviews report no blocking findings; the final source-check record confirms no
production-source difference from that reviewed revision.

The real editor on focused `2040e23b` generates exact geometry for REVIUNG41
and Sofle Left/Right. DPR2 resize, scope invalidation/remount, cancellation and
immediate retry pass. Context loss stops rendering; a fresh remount recovers.
This focused artifact deliberately omits offline policy and is not complete
release acceptance. The uniquely staged final build
`m1-release-20261001-bc1c6b52` is in progress; current-release offline, file/URL
lifetimes and eligible performance checks follow it. Earlier complete releases
and failed attempts remain retained. Relevant screen-reader testing is blocked
because no screen reader is installed. No missing gate is reported as passed.

See [source checks](../../.scratch/m1-production/evidence/integration/final-source-checks/record.json),
[editor renderer evidence](../../.scratch/m1-production/evidence/renderer/focused-2040e23b/README.md),
[reviews](../../.scratch/m1-production/evidence/review/standards-2040e23b.md),
[Spec review](../../.scratch/m1-production/evidence/review/spec-2040e23b.md) and
[performance protocol](../../.scratch/m1-production/evidence/performance/README.md).

### Fresh-prefix build correction

The complete `bc1c6b52` build exited zero and every captured hash matched, but
its inventory exposed three Dioxus page WASM binaries retained from prior
builds. The actual staged-output regression fails with the expected obsolete
asset assertion. This artifact is retained as component evidence; it fails the
reproducible release/cache-inventory gate.

Build correction `fe2ada03` preserves each previous Dioxus public directory in
the new uniquely named build output, then builds each prefix into a fresh
public directory. Nothing is deleted or discarded. Runtime/provider source
remains `2040e23b`; independent Standards/Spec review of the script correction
and complete rebuild `m1-release-20261001-fe2ada03` are in progress. All earlier
artifacts, successful commands and failed assertions remain retained.

The corrected `fe2ada03` complete build passed all 18 commands. Source and
staged hashes match; the actual regression is green at both prefixes, with
one current page WASM and 47 staged files each. Standards and Spec reviews
report no blocking findings.

A separate real UI check found that export-only CAD replies omit `bodies`,
while the page decoder calls `Array::from` on that absent field and traps.
Raw JavaScript worker/readback proof did not exercise this Rust decoder.
The public UI failure is retained; a separate implementer is building the
correct public-browser regression and repair. STEP acceptance and final
release/performance remain open until the corrected page is verified.

### STEP decoder repair integrated

Public `CadWorker::request` browser regression was red on the old decoder and
green after `5a0972a2`. Omitted body meshes now decode as empty; a supplied
non-array returns an error through the request channel. Preview/Exact payload
validation remains strict. The regression seam is compiled only with
`test-harness`, which is excluded from production feature selections. Native
web tests (12), fmt, strict affected WASM Clippy and the focused page build pass.

Independent public UI verification downloads the same 10,334,256-byte Sofle STEP
(SHA-256 `582f343c9a9eabfedab5a8f69c2dc078c5ef31c0ade008d9de989a8d53b15195`)
as the independent worker/readback proof. The blob URL is revoked after about
one second, with no retained anchor or browser errors. Standards and Spec
reviews have no blocking findings. Independent merger integrates the exact
candidate at `8f509433`. The final complete build
`m1-release-20261001-8f509433` passed all 18 commands, 925 source hashes and
both 47-file prefix inventories with one current page WASM each. Final root
and subpath offline/storage/update checks pass. Both fixtures generated and
downloaded STEP files with URL revocation; changing scope suppressed stale
delivery. Quiet-host performance and complete resource/semantic checks remain
in progress. Relevant screen-reader testing remains blocked.

See [final release browser evidence](../../.scratch/m1-production/evidence/browser-storage/release-8f509433-final-qa.md),
[build provenance](../../.scratch/m1-production/evidence/integration/release-8f509433-provenance.json),
[current acceptance](../../.scratch/m1-production/ACCEPTANCE.md) and
[editable handoff](../../.scratch/m1-production/HANDOFF.md). Earlier checkpoint
statements retain their contemporaneous failures and pending gates.


### Pointer allocation regression and focused repair

Two complete public pointer sessions on `8f509433` fail 100/200-key caps:
135.3/495.8 ms and 134.4/500.6 ms against unchanged 50/100 ms limits.
The stopped paired run is ineligible because its reference import navigation
raced React hydration; the public readiness adapter was repaired separately.
All failures and partial sessions remain retained.

The presentation repeatedly deep-cloned complete range and keyboard item lists
for every part. Reviewed commits `9a5b2098` and `e7d29ce6` share those immutable
lists, preserving ordered selection and Arrow/Home/End/Space/Enter behavior.
The first fix improves timings but still fails; the combined fix passes one
focused session at 18.6/24.1/33.3 ms for 30/100/200 keys, with 100 measured
samples each. This is not five-session or complete-release acceptance.
Formatting, twelve native web tests, strict WASM page lint, release compilation
and public selection/keyboard checks pass; independent Standards/Spec reviews
have no findings. The independent merger integrates the exact source at
`b9748745`; unique complete rebuild `m1-release-20261001-b9748745` passed all
18 commands and source/asset verification. Its bounded root/subpath smoke passes.
Two attempted paired runs stopped on harness availability/readiness failures;
partial candidate cap passes do not establish the required five-session aggregate.

Same-input default REVIUNG export through the actual TypeScript service yields
the same five bodies and measured geometry as M1. STEP serialization differs
in 434 direction/point coefficients (maximum numeric delta about 1e-11);
independent BRep volume differs by about 5.82e-11 mm³. Exact-byte parity is not
claimed. The nondefault physical-instance diagnostic and all raw readbacks are
retained separately. Scoped worker/observer/listener/RAF/URL observations pass
for tested transitions; root-close and late-import races remain narrower open
checks. Full M1 acceptance and actual screen-reader testing remain open.


### Final story audit: startup restoration and instance navigation

Public browser checks against `b9748745` found that saved Sofle copies retain
the scoped active-project preference, but online and offline reloads show only
the library. The editor also exposes only a Board selector despite saved
physical instances. [The public red record](../../.scratch/m1-production/evidence/integration/startup-restore-instance-red-b9748745.md)
retains the exact assets, saved document, preference, accessibility snapshots
and screenshots. Stories 4 and 5 remain under implementation: restoration will
reuse the existing scoped preference and sequenced saved-open path; an
accessible Physical instance selector will use the existing Navigate event,
including an explicit canonical-board choice. No schema or public API change
is needed. Earlier bounded smoke did not cover these two requirements.


### Unchanged reference performance gates

The [exact reference run](../../.scratch/m1-production/evidence/performance/runs/reference-gates-20261002T002536Z/assessment.md)
completed serial UI/live, CAD and frozen comparison commands, recording the
`b9748745` candidate provenance as context. UI/live exits 1 on existing frozen
worker/main-thread and numeric/Undo/gasket timing comparisons. CAD generation
exits 0 after five sessions; all per-row completion/paint/RSS comparisons pass.
Its frozen comparator exits 1 because OS and core/renderer WASM identities
differ from the baseline, so baseline-comparable acceptance is ineligible.
All nine pinned inputs remain byte-identical. This reference-only run does not
substitute for final public M1 pointer timing. No budget or baseline changed.


The final story repair is integrated at `a49bb798` after independent Standards
and Spec review. Fourteen public application tests, twelve web tests, formatting,
strict application/page lint and focused release compilation pass.
[Focused root-only browser QA](../../.scratch/m1-production/evidence/integration/startup-restore-instance-focused-b50ddbdd-qa/record.json)
passes active restoration, stale-preference recovery, board-filtered physical
options, canonical reset, keyboard selection and compact layouts; axe reports
zero violations with SVG contrast incomplete. The focused artifact has no
offline policy. A fresh maintained root/subpath build is running; actual offline
restoration, delayed IDB/slow-worker startup and physical-case races remain open.


### Final maintained story-completion release

`m1-release-20261002-a49bb798` passes all 18 build commands, 925 source hashes,
and both 47-file inventories with one current page WASM per prefix. The
[exact-source command ledger](../../.scratch/m1-production/evidence/integration/final-story-source-checks/record.json)
retains fresh application14/web12, fmt, strict lint, repository/contracts and
boundary checks, all exiting zero; prior transient check reports are not reused
as raw logs. The owned editable demo is [root](http://127.0.0.1:34285/) and
[/boardstudio/](http://127.0.0.1:34285/boardstudio/).

[Final browser QA](../../.scratch/m1-production/evidence/integration/release-a49bb798-qa/README.md)
passes both-prefix online/offline active restoration, preference handling,
same-origin storage isolation, actual delayed saved-read supersession and slow
Core initialization. Canonical and configured Left physical case generation
succeed; navigation during delayed CAD initialization terminates the old worker
and prevents stale geometry. Actual app-tab closure removes Core/CAD browser
targets and reopening restores saved work. This observes platform teardown,
not Rust drop execution or GPU memory. Sofle Right PCB reports its unready input.
All five fresh paired public pointer sessions pass the unchanged 33/50/100 ms
caps at 30/100/200 keys, with median candidate p95 18.8/24.4/33.1 ms and
100 measured samples per scenario. [Raw paired results](../../.scratch/m1-production/evidence/performance/runs/paired-pointer-a49bb798-20261002-retry1/summary.json)
retain every sample and endpoint/environment eligibility. Ancillary startup
visibility observations are incomplete and establish no startup timing gate. Required screen-reader
interaction remains blocked; frozen reference timing failures/ineligible CAD and
material/GPU attribution limits remain in the acceptance ledger. No M1 closure,
production cutover, push, deployment, main merge or React retirement is claimed.


### Final physical STEP delivery and acceptance frontier

The final a49 release passes configured Sofle Left PCB / left half STEP delivery
on an owned online root origin. Its 6,417,795-byte download, URL revocation and
physical-to-canonical navigation while the actual CAD worker WASM is held are
recorded in [the changed-path check](../../.scratch/m1-production/evidence/integration/release-a49bb798-qa/physical-left-step-acceptance.md).
Releasing unchanged bytes produces no stale download. This check uses no service
worker and makes no offline or material-oracle claim. Older canonical REVIUNG/Sofle
STEP/readback evidence retains its 8f/b974 artifact scope.

Implementation and the documentation plan are retained on
`codex/m1-production-20261001`; production source remains `a49bb798`.
Ticket 06 remains open for actual screen-reader interaction, retained frozen
UI/live failures, environment-ineligible frozen CAD comparison, and required
material/direct resource attribution. The [handoff](../../.scratch/m1-production/HANDOFF.md#remaining-acceptance-work)
records the continuation work. Original tracked checkout and stash identities
remain preserved; unrelated untracked content was not hashed.

The user also requires continuous architectural/design/theoretical/quality takeaways. The [post-port refactoring register](POST-PORT-REFACTOR.md) is updated at every workflow handoff and consolidated for the later refactoring phase; it does not broaden this frontend port or waive current gates.

## Frontend model allocation — 2026-10-02

User preference now routes bounded implementation and evidence work to Luna
low/medium/high, with Astra high/xhigh reserved for independent reviews and bug
fixing. A Luna Medium readiness audit and Astra High policy review informed the
[packet/agent policy](../../.scratch/dioxus-frontend-v1/AGENT-ROUTING.md) and
[all-task routing](../../.scratch/dioxus-frontend-v1/agent-policy.json). The normal
team is two feature authors plus a verifier, rotating capacity into review.
The 62 task IDs, graph, acceptance gates and current executable remain unchanged.
Priority is configured on the host; effective child tiers were not independently
observed and the spawn interface has no tier parameter. No host configuration
change or feature implementation was performed in this policy review.


## Frontend parallel implementation — 2026-10-02

The user authorized implementation and automatic next-ticket creation through
agents. [Current execution](../../.scratch/dioxus-frontend-tranche-1/execution.json)
and [authority](../../.scratch/dioxus-frontend-tranche-1/AUTHORITY.md) record the
published first twelve children of four parents; all62 remaining work packages
stay in the task graph. INT.1's direct private Library/Objects/Inspector extraction
is underway in an isolated worker after independent contract review. A parallel
agent prepares Parts/PCB/Keymap starter tickets for source-checked publication.
The current baseline public Layout regression passes; source extraction,
new UI features and parent acceptance need their own integrated evidence.


### INT.1 accepted and next frontier published

Private Library/Objects/Inspector extraction is integrated at `598b2c02`. The [acceptance record](../../.scratch/dioxus-frontend-tranche-1/evidence/int1-integrated/record.json) retains independent Standards/Spec reviews (zero findings), 14 native page tests, strict native/WASM checks, fresh root/subpath page/offline build provenance, public layers/keycaps/footprints regression, and matched Inspector numeric lifecycle traces. The editable fresh demo is [root](http://127.0.0.1:34645/) and [subpath](http://127.0.0.1:34645/boardstudio/).

Two Luna High authors prepare saved-keyboard cards and desktop panel modes in isolated worktrees. Agents automatically generated and Astra reviewed six further children: [Parts catalogue](../../.scratch/dioxus-parts-catalogue/README.md), [PCB view](../../.scratch/dioxus-pcb-view/README.md), and [Keymap layers](../../.scratch/dioxus-keymap-layers/README.md). These refine three more parents; all 62 parent IDs and original joins remain. Feature-specific contracts remain required. No new refactoring takeaway observed; source reconciliation reinforces RF-009. Full frontend v1, actual AT, retained M1 gates and production cutover remain open.


### Cards, panels and tree implementation frontier

Private saved-keyboard cards and desktop panel composition are integrated at `2030c9a3`. The fresh root/subpath page and offline build, 14 native tests, four standalone panel-policy tests and strict native/WASM checks pass. [Independent review and build record](../../.scratch/dioxus-frontend-tranche-1/evidence/cards-panels-2030c9a3/record.json) retains three open findings: responsive panel tracks, compact-to-desktop auto-hide, and unnecessary document copying. This candidate is not accepted. Astra handles the behavioral fixes after public red regressions; Luna handles immutable library ownership.

The tree hierarchy is committed in its worker at `33177652`; private shared selection/context and scoped pointer integration are underway. Public tree acceptance remains open. Agents review further Keycaps/BND.1 children for automatic publication. All 62 parent milestones and their original acceptance joins remain. Saved-library malformed-record listing and confirmed post-submit open supersession are still unresolved; no public API change or production adoption is implied.


Agents automatically published independently reviewed [Keycaps physical 2D](../../.scratch/dioxus-keycaps-projection/README.md) and [private keycap CAD bridge proof](../../.scratch/dioxus-keycap-cad-bridge/README.md) tickets. There are now 20 bounded tickets, refining the unchanged 62-parent graph. Dispatch still requires concrete private seams; publication does not imply implementation or parent acceptance. No new refactoring takeaway observed during this source/dependency review.


### Desktop panel repairs verified

Source `47cf655b` fixes the interactive-panel inert attribute, responsive grid overrides, and compact-to-desktop auto-hide timer. The [exact build, independent source reviews and public evidence](../../.scratch/dioxus-frontend-tranche-1/evidence/panels-fixed-47cf655b/record.json) pass both red/green browser loops, independent panel modes and hover/focus, reload/workspace preference retention, malformed/unavailable optional storage, camera/revision neutrality, mounted long-content retention, themes/keyboard and axe (zero violations in tested states). Actual AT remains separately required; no AT pass or full F2.3 closure is claimed. The fresh editable candidate is [root](http://127.0.0.1:34651/) and [subpath](http://127.0.0.1:34651/boardstudio/).

Tree integration `547b0762` is source-reviewed and strict WASM checks pass in its worker; merged public scope/gesture tests remain pending. Private Parts catalogue source is under review/correction. The exact tolerant-listing public API patch is reviewed but unapplied, with an explicit user decision pending; frontend work continues independently.
