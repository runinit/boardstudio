# Draft-safe update adoption — implementation execution

Source: [spec](spec.md) and [implementation transfer](issues/006-agree-verification-and-implementation-transfer.md#answer).

## Authorization and boundaries

The user authorized continuing through tickets 008–015 with fresh implementation
agents and independent reviews. Report RED, mounted GREEN and packaged GREEN;
continuation is authorized, not waiver of tests, reviews or incomplete evidence.
The later user-approved Pair-only baseline amendment is recorded in
[004](issues/004-choose-draft-safety-interface.md). No other editor Apply/Save
policy change, Geometry typed-draft conversion or Class B expansion is authorized.
Keep the parent adoption umbrella open; worker/cache/Toast concerns remain separate.

Work in the shared `fix/ui-integration-recovery` checkout. Checkpoint base:
`398c7a45c`. Preserve overlapping work. Use native Gortex for source evidence and
mutations, with no CLI/source fallback, tracking or daemon changes. The user
permits up to three bounded attempts for transient routing failures, separated
by independent permissible work, not busy-polling. Report a persistent blocker.

## Delivery order: prove the tracer before expanding coverage

1. **Finish only the five identified 011 corrections:** sticky Pair success and
   failure semantics, newer constraint kind/section preservation, newer input
   across Remove landing, live accepted projection at each poll, and correct
   owner/target rebinding. Require focused RED/GREEN, coordinator failure-path
   reruns and fresh Spec/Standards review. Do not add unrelated form features,
   broad rewrites or speculative abstractions to this correction.
2. **010 is next, before starting 012–014.** Reuse the existing packaged driver
   and diagnostic adapter. First prove the real Geometry tracer: an unapplied
   draft prevents reload; Cancel preserves it; Apply plus acknowledged durable
   save allows actual B adoption with the intended saved project/value/revision.
   Complete the existing 010 matrix and the specified Layout follow-up; do not
   substitute mounted tests or labels for real packaged evidence.
3. **Stop expansion if that proof fails.** Fix the demonstrated integration gap
   and rerun the same assertion before copying the adapter pattern to more owners.
   A test count or number of completed tickets is not the delivery checkpoint.
4. **Then finish the approved remaining owners:** 012 Parts, 013 PCB, 014 creation
   forms. Their fourteen-owner coverage obligation is not dropped. Escalate any
   need for broad form rewrites or a new product policy before implementing it.
5. **015 closes verification, not the parent umbrella:** coverage/copy audit,
   mutation controls, full integrated diff read, coordinator-owned full build/gate
   receipt and final combined Spec/Standards review.

Gortex investigations were read-only operational work and are complete. Do not
turn this feature into a Gortex repair project. Required correctness and documented
standards findings remain gates; optional review cleanup is not extra scope.

## Frontier and ownership

| Ticket | State | Next acceptance evidence / owner |
| --- | --- | --- |
| [008 Registry and Geometry tracer](issues/008-registry-and-geometry-tracer.md) | Resolved | Reviewed corrections; coordinator Geometry 6/6, helpers 16/16, notifications 30/30; composite 010 accepted |
| [009 Persistence adapter](issues/009-diagnostic-gated-persistence-adapter.md) | Resolved | Reviewed corrections; coordinator adapter 10/10, Runtime 1/1, tooling 9/9; real packaged integration and cleanup in 010 |
| [011 Layout owners](issues/011-layout-explicit-submit-owners.md) | Resolved | Spec/Standards PASS at `16b85737`; coordinator complete diffs and 106/106; packaged Layout proof in 010 |
| [010 Packaged A→B proof](issues/010-packaged-a-to-b-proof.md) | Resolved; composite checkpoint accepted | Complete code reviews through `c5a876ec`, seven behavioral criteria and final owned profile/database/output cleanup pass |
| [012 Parts owners](issues/012-parts-single-draft-owners.md) | Resolved 2026-11-08 | Reviewed + coordinator-accepted; 013 decision next, then 014 |
| [013 PCB owners](issues/013-pcb-placement-attach-assembly-net-owners.md) | Resolved 2026-11-08 | Full-source Spec/Standards and origin-fix delta reviews PASS; coordinator Parts 106/106, PCB 64/64; 014 next |
| [014 Creation forms](issues/014-creation-form-owners.md) | Resolved 2026-11-08 | Spec PASS/READY, full-source Standards PASS; coordinator Layout 268/268 plus CSS; 015 next |
| [015 Coverage and closure](issues/015-coverage-mutation-and-closure.md) | Not started | Coordinator owns full build/gate; all specified evidence and reviews remain required |

## Verification-reliability preparation

User-approved A→B→C, finishing before 012 starts. It improves how remaining tickets are proved and
reviewed; it is not Parts scope and changes no product, acceptance threshold or dependency.
The canonical protocol (proof map, review reports, completion record) is in
[issue tracker](../../agents/issue-tracker.md#building-and-verifying-a-ticket).

- **A, protocol and proof maps:** implemented in `3c1ff8c`, independently reviewed with full
  seven-file coverage. Standards found one required status-label correction; Spec found one
  required assembly-settlement correction. Both are corrected in the follow-up: 015 uses
  `planned`; 013 distinguishes Place retirement from mounted Matrix settlement and explicitly
  blocks on the unresolved origin-baseline decision. Coordinator diff and documentation checks
  pass; A is complete. The reviews' original conditional/changes-required verdicts are retained,
  not restated as new reviewer PASSes. Review tasks: Standards `768d3dca`, Spec `a5503305`.
- **B, browser lifecycle:** code/unit checkpoint `6764109e` accepted. Independent Standards
  (`51139f25`) and Spec (`1f11c52f`) PASS, no Critical/Required findings; both read all four changed
  files to EOF and the full diff. Earlier partial reviews are not counted as full certification.
  Coordinator verified 16 lifecycle tests, 64 check-tool tests, the complete tooling gate and
  718-file documentation-link check. Additional RED/GREEN guards cover absent ownership,
  retained-profile rollover, failed close status, unknown process state, removal failure and
  profile paths containing spaces. Evidence under `.evidence/update-draft-safety/`:
  `abc-b-coordinator-red.log`, `abc-b-coordinator-red-io-path.log`,
  `abc-b-coordinator-green.log`, `abc-b-coordinator-test-check.log`,
  `abc-b-coordinator-tooling.log`, `abc-b-coordinator-doclinks.log`.
  Optional suggestions are not acceptance blockers. B's code/unit checkpoint deferred real-browser
  acceptance to the integrated C smoke below, which now discharges that lifecycle gate.
- **C, generated-output cleanup:** code/unit checkpoint `5ef7b778` accepted. Independent Standards
  (`2e9a7f5c`) and Spec (`da05402b`) PASS, no Critical/Required findings; both read all three changed
  files to EOF and the full diff. Fresh command-owned workspaces validate existing, symlinked,
  unusable and input-overlapping paths before writes. Normal/caught-failure cleanup waits for
  browser/profile/server safety; uncertain cleanup retains paths and fails. `--keep-artifacts` is diagnostic-only, never acceptance
  (exit 3 without failures). Geometry exports now share session ownership and retention; coordinator
  RED/GREEN corrected their premature deletion. Coordinator verified 31 focused tests, seven named
  refusal/failure/retention guards and the full tooling gate. Evidence under
  `.evidence/update-draft-safety/`: `abc-c-coordinator-focused.log`, `abc-c-coordinator-guards.log`,
  `abc-c-coordinator-export-red.log`, `abc-c-coordinator-export-green.log`,
  `abc-c-coordinator-tooling.log`, `abc-c-coordinator-doclinks.log`,
  `abc-c-coordinator-lifecycle.log`.
  The single integrated real-browser smoke passed using existing `site-019-fix` read-only as input:
  main created/stamped its A/B workspace, Chrome ran on the exact private profile before close,
  normal project import created IndexedDB, normal export observed the imported project, and one
  confirmed close removed profile/database/fixture/export. Repeated cleanup issued no second close;
  main removed its generated workspace and released port 4746. No rebuild, input deletion, cache
  clearing or worker unregister was used. This lifecycle proof is not a new product matrix or any
  015/019 closure. No invocation-owned smoke outputs remain. Build-phase owners still attest
  diagnostic-source cleanup separately; this smoke created no source input requiring cleanup.
  Optional review suggestions are deferred without changing acceptance criteria. The Spec reviewer
  did not load the review guides or inspect logs; coordinator guide reads and executed evidence
  are recorded separately, not attributed to that reviewer. Runtime workspace-creation I/O failure
  can reset the serve log after validation; no general zero-write runtime-error guarantee is claimed.
  Coordinator acceptance: A/B/C preparation complete; source checks and smoke are reused unchanged.
- **012:** next, not started. The user approved a new-thread transfer for the remaining 012–015
  implementation. 013 still requires its explicit human origin-baseline decision; no policy is waived.

This sequencing checkpoint does not change ticket scope or waive formal dependencies.
The reviewed 008 implementation can feed dependent work while 010 discharges its
explicit composite acceptance debt. No partial owner coverage is final acceptance.

## Delegation and verification discipline

- Fresh agent per ticket. Well-defined spec/ticket implementation: Pi
  `meta/muse-spark-1.3-contributor`, `thinking=max`. Independent reviews remain
  on Pi Contributor at `thinking=medium` under this effort's existing routing.
  Consequential issues: fresh Pi `openai-codex/gpt-6.1-sol`, normally
  `thinking=high`, with failure evidence and a materially different next experiment.
- One source/focused-build owner at a time; builds serialized per target.
  Read-only reviews may overlap disjoint work. No duplicate blocked implementations.
- This record is coordinator-owned. Delegates edit only their explicitly assigned
  sources and ticket progress; virtual notes supplement the self-contained transfer.
- At each completion: coordinator reads the full diff and evidence, independently
  reruns rejecting guardrails, and starts fresh read-only Spec and Standards
  reviews pinned to the completed change. Required findings are fixed; no premature
  resolved status. Optional suggestions do not expand the task.
- Proof maps, review reports and completion records follow the
  [canonical protocol](../../agents/issue-tracker.md#building-and-verifying-a-ticket).
- Full-suite/full-build runs belong to the coordinator at 015, not concurrent
  delegates. Preserve all required checks and approvals; report blocked gates.
- Evidence logs only under `.evidence/`. No source copies, custom manifests or
  target directories there. Disposable fixtures/profiles/databases only.

## Verified evidence to date

- 008: `10df36141`, corrected by `02ec0b636`; coordinator read both complete
  diffs and reran helper **16/16**, Geometry **4/4**, notifications **30/30**.
  Same-key pending exemption, terminal-record regressions and real Back/unrelated
  submission controls verified. JSON representation and existing failure policy
  unchanged. Both review axes permit dependent implementation; trusted activation
  remains 010 debt. Logs: `008-coordinator-sol-{ui,geometry,notifications}.log`.
- 009: `86bd4620b`, corrected by `764a4dd54`; deadline-based entry observation
  and awaited cleanup replace timing guesses/scheduled-only deletion. Fresh reviews
  found both required corrections resolved; exact structural-view limitations are
  recorded in the ticket. Independent adapter **10/10**, Runtime held-save **1/1**;
  original build-tooling **9/9**. Logs: `009-coordinator-sol-{gate,runtime}.log`
  and `009-coordinator-tooling.log`.
- 011: original `043787ba5`, Pair RED `94b9acf`, corrective `16b85737`.
  Coordinator read the complete original and corrective diffs and independently
  reran **106/106** focused browser tests, including rejection and binding paths.
  Fresh Spec and Standards reviews PASS with no required changes. The test-economy
  review found distinct regression purposes; the display/static-owner/live-projector
  hosts serve different coverage. The small canonical-literal extraction suggestion
  is optional, not extra implementation scope. Contract-attachment freshness and
  final mutation/full-gate closure are not certified by these focused checks.
  Logs: `011-approved-*.log`, `011-coordinator-corrected-browser.log`.
- 010: `c899b0f69` + Layout `99eafb644` + Geometry Retry `be39738b` now have
  fresh Spec/Standards PASS (Standards discloses partial driver coverage), full
  coordinator combined-diff read, independent Geometry **6/6**, helpers **16/16**,
  notifications **30/30**, and **five packaged legs / zero failures** from corrected
  diagnostic assets. Both literal Retry paths adopt B directly and reopen exact r3;
  Layout reopens exact r1/value. Logs: `008-coordinator-retry-*.log`,
  `010-coordinator-retry.log`.
- Coordinator then closed two verifier gaps without changing the app: actual
  header-Space activation and exact reopened project/source/revision for tracer
  and hold (reuse the existing Retry export oracle). **Four affected existing
  legs / zero failures**, `010-coordinator-oracles.log`; wrong project, revision
  and source are rejected by `010-coordinator-export-oracle.log`. This follow-up
  is included in the complete final rechecks below; 010 records reproducible runs.
- Fresh consequential Sol/high adjudication confirmed the takeover timing gap;
  that agent was blocked before editing by three native `view_building` impacts.
  Coordinator's later native edit strengthened the **existing single leg**:
  held r3 → Update Now deferral → newer input while held → r3 landing preserves
  newer input → Geometry-specific deferral again → resolve r4 → B/exact reopen.
  This is waiting-intent proof, not a claim of a post-permit worker-probe race.
  Takeover + Layout **2/2** and inherited/negative controls **6/6** pass on the
  corrected diagnostic app (`010-coordinator-takeover.log`,
  `010-coordinator-controls.log`). Shared owned-profile cleanup rejects unconfirmed
  close/live Chrome and both explicit profiles/IndexedDB were removed; default
  startup compatibility is checked (`010-coordinator-profile-controls.log`).
- Complete combined rechecks through `c5a876ec`: Standards task
  `e7e3846c-37d3-47ed-b122-ae6d86ac2a77` **PASS, no Required code changes**;
  fresh Spec completeness task `6ec0a0e3-cf9e-44d7-b03c-b9dce5603cc4`
  **code correct, checkpoint BLOCKED on mandatory cleanup**. Initial Spec task
  `606c173e-4055-4dcc-bf78-89eb3064527a` disclosed partial driver coverage and is
  not used as complete certification. Both final axes cover all changed source;
  no new production correction or optional cleanup is requested.
- Final coordinator cleanup discharges that sole checkpoint blocker:
  `010-cleanup-lifecycle.log` matches a private CLI CDP endpoint to its temporary
  profile, observes real app IndexedDB and exact live Chrome processes, then proves
  close removes both processes and disk state. Two failed ownership-instrumentation
  setups are retained, not behavioral RED. `010-cleanup-attestation.log` proves
  33 recorded temporary roots absent, zero default/copied-profile residuals in
  both temp roots, no active sessions, and deletion of 15 exact log-attributed,
  ignored diagnostic outputs after no-use checks. No unidentified/live state was
  deleted; immutable 002 and Cargo/provider caches were retained. Independent
  postcheck confirms all 15 absent and port 4744 free.
- **010 composite accepted; 008/009/011 closure bookkeeping complete.** Each
  owning ticket has its acceptance mapping and final Answer; the historical
  conditional review snapshot remains truthful. 012 is next, not yet started.
  Optimized validation, all-14 coverage and 015 integrated gate/mutations/final
  review remain due; deleted disposable packages must be rebuilt for those runs.
  No final all-owner acceptance or parent umbrella closure is claimed.
- **012 resolved 2026-11-08.** Generator (Apply+Upload), module profile
  (save+save-rotary) and fit profile (Save) adapters with per-key copy;
  25 mounted tests GREEN, full parts suite 97/97 GREEN, fresh
  Spec/Standards PASS with full four-file coverage and no Required
  findings. Two user-approved failure preserves (generator Failed arm +
  2 oracle corrections; module snapshot-restore, zero corrections) and
  three evidenced N/A classifications (staged-shape, fit invalid, fit
  P3(b)/(d)) all held under Spec scrutiny. Full project gate remains
  015 scope per plan. Next: 013 human origin-baseline decision, then
  014; see the 012 completion record.
