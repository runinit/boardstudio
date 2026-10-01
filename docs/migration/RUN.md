# BoardStudio milestone continuation

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

## Current task set

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
After that bounded approval, apply the exact patch in the task-owned worktree,
run all affected renderer checks, then resume the
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
