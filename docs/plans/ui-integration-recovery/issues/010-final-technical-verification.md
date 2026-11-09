# 010: Complete final technical verification and independent review

Status: resolved
Type: task
Blocked by: [009](009-verify-reload-persistence.md), [022](022-restore-collision-provider-browser-proof.md)
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [Migration gate record](../../rust-ui-build/issues/026-merge-gate.md)

## What to build

Establish technical acceptance of the combined recovery candidate: complete
final-source checks and independent spec+standards review of the reconciled
migration, including inherited integration behavior. Deliver a precisely
identified checked candidate for populated-workspace human review in 011.

## Acceptance criteria

- [x] All listed blockers are resolved with their required evidence. 003 is `wontfix` (not applicable)
  and 008 is covered through 009. Do not infer completion from source
  presence, diagnosis alone or a ticket title/status without its acceptance.
- [x] Run the full repository default check, including the full browser gate,
  against the final source. Record exact invocation, source/build/browser
  identity and all terminal results; selected-crate and quick plans are not
  substitutes. A completed full run need not be duplicated without a reason.
- [x] The real drag/save/Undo and reload scenarios run against the combined
  candidate, with persistence/scene assertions and receipt checks intact.
  Retained older or individually focused runs do not establish this result.
- [x] Complete the existing accessibility floor and strict old-CSS check. State
  what they establish and any unrun broader accessibility validation.
- [x] Obtain independent spec and standards review through the repository's
  two-axis review workflow. Cover the reconciled migration candidate relative
  to the integrated behavioral reference, not merely the last repair diff.
- [x] Resolve or explicitly disposition review findings and recheck corrections.
  Outstanding acceptance blockers keep this ticket open. New unrelated work is
  separately scoped rather than absorbed as unlimited final-gate cleanup.
- [x] Ensure evidence still applies to the delivered source after corrections.
  Record which checks were rerun and why any retained evidence remains valid;
  do not silently promote results from a different candidate.
- [x] Reconcile the migration/recovery records with the actual gate and review
  results (A8), preserving historical receipts and keeping human acceptance open.
  Do not change or close the parent specification.

## Verification and completion

The required checks are the full default/browser plan, the accessibility floor,
the strict old-CSS check and independent spec+standards review. Document their
commands, results and source identities in the ticket's completion record, with
long logs kept in the repository's evidence area. No merge, push or live-preview
replacement is authorized by this gate. Product acceptance still requires 011.

## Comments

- 2026-11-06: The user approved 022 as an explicit blocker for this gate. 008 and
  009 are resolved within their bounded scopes, but the complete smoke runner
  still fails at Case mounting style and has no success receipt.
- 2026-11-06: Published with direct dependencies only. This ticket separates
  combined technical evidence from the user's populated-workspace acceptance.
- 2026-11-06: The user cancelled 004–007; they are no longer dependencies.
  Cancellation does not establish their behaviors or waive outstanding review
  findings. Reconcile final acceptance scope with that decision when running
  the independent review and human acceptance.

## Completion record (coordinator, 2026-11-08)

Source HEAD `18d4d7f535dca096a811c9e71bbf823f475485fc`; Status `resolved`.
Acceptance readiness: **READY for 011**, under the explicit coverage and risk
acceptances below, not product acceptance. Newly executed: full default gate
(including drag/reload and accessibility floor), strict old-CSS check and both
review axes. Retained: 008/009/022 Answers and receipts, draft-safety and
019/020/021 reviewed closures; the rerun-versus-retained table below explains why.
Spec remains **PARTIAL** (coverage-capped); Standards' required R1 is resolved.
No mandatory technical blocker remains under the coordinator's dispositions.
Optional debt and residual risks are retained explicitly. Next: **011 human
acceptance only**; no parent-001 closure, merge, push or live-preview replacement.

Documentation-only reconciliation checks: `python3 scripts/check-doc-links.py`
PASS (719 Markdown files; `.evidence/ui-integration-recovery/010-doc-reconciliation-links.log`),
`git diff --check` PASS, full edited-file/diff read and three-file ownership check;
001 is unchanged and 011's acceptance criteria/status are untouched. This docs
pass reran no product gate and changed no code; it records the coordinator's
fresh gate and review evidence above.

## Answer

### Candidate and fresh technical evidence

The gate-time identity record is
`.evidence/ui-integration-recovery/010-identities.txt`: HEAD
`18d4d7f535dca096a811c9e71bbf823f475485fc`, tracked-diff SHA-256
`1bc8b72402bb56cbf5f53a22705be753dcd8e327cc437016fdbcb122155c35e0`.
It explicitly records the dirty external-audit `AGENTS.md` / `CODING_STANDARDS.md`
and untracked `.gortex.yaml`, `.gortex/`, `docs/agents/ai-documentation.md` and
`docs/research/2026-11-07-ai-agent-test-economy.md`. These are excluded from
candidate implementation claims; the external work is documentation/retrieval
audit material, not a recovery repair.

**Post-gate identity note (coordinator):** the ONLY candidate-tree change during
or after the gate before this record was the 010 claim line plus these three
documentation files (010, the map and 011). No production/test source changed.
Retaining the gate evidence is justified on that documentation-only basis, not
by silently substituting another candidate. The drag receipt identified in the
gate log binds the source HEAD and source/build hashes:
`web/target/test-results/drag-browser/20261109T013611Z-d885f979c31c/receipt.json`
(source SHA-256 `400f6cd3045d57cec809ef095bf167e7b409129507f7265ba2d531da85b60331`,
build SHA-256 `3f28aaf6cb72fd88c2f97a5ad019dfcca19b4a0d9bdcfaa43824bc95c96f650b`).
The gate's `BOARDSTUDIO_DRAG_BINDING` names the packaged site
`web/target/site/site-subpath/boardstudio`, SHA-256
`8a0606dae06b8fa732efa2e06794ff67afa4d145b4d4989cf14053e15d5cc561`.
Browser identity in the gate log: **Chrome-for-Testing 156.0.8078.73** (matching
ChromeDriver and drag receipt).

Fresh full default invocation (serialized build lock):

```sh
nohup bash -c 'exec 9>"$HOME/.cache/tmp/boardstudio-build.lock"; flock 9; BOARDSTUDIO_BUILD_LOCK_FD=9 python3 scripts/check.py' > .evidence/ui-integration-recovery/010-full-gate.log 2>&1 &
```

`010-identities.txt` transcribed the same lock path with `~` shorthand.

`.evidence/ui-integration-recovery/010-full-gate.log` ends **ALL STEPS PASSED**:
repo, tooling, build, lint, test and browser; **442.9s** total. Its browser gate
includes **PASS drag browser smoke: 17 checks** with intact receipt validation:
real drag/save/cancel/exact Undo, saved-edit reload and saved-Undo reload with
exact document/scene, same-project and saved-revision assertions, plus the
collision/provider scenarios. The historical complete-runner failure recorded
in 008/009 is superseded by this fresh combined pass, not erased.

The accessibility floor runs inside that gate. The shared-control browser
results and final mounted-browser results (223 executed, 0 failed) establish the
role-and-name, keyboard and focus behaviors required by
[accessibility floor §3](../../rust-ui-migration/accessibility-floor.md#3-the-floor),
including popup roles/names, navigation, Escape/focus return, focus containment
and reachable controls. This is bounded floor evidence, not broad accessibility
certification. **UNRUN / not claimed** under
[floor §1](../../rust-ui-migration/accessibility-floor.md#1-method-and-limits):
screen-reader validation, other browsers, touch, broader in-app combinations and
an axe rerun. The historical lab's other unmeasured sequences are not promoted
to new measurements here.

Fresh strict old-CSS receipt:
`.evidence/ui-integration-recovery/010-old-css-strict.log` — **0** `.board-*` /
`.m1-*` selectors and **0** `--wb-*` variables remain (**0/0**).

### Rerun versus retained evidence (criterion 7)

| Evidence | Disposition | Applicability to delivered candidate |
| --- | --- | --- |
| Full default gate | Fresh, `010-full-gate.log` | Combined source at the identity above; only documentation changes afterward. |
| Real drag/save/Undo and both reloads | Fresh inside that gate, 17 checks and bound receipt | Replaces reliance on individually focused historical runs. |
| Accessibility floor | Fresh inside that gate | Floor §3 only; broader validation remains unrun as stated above. |
| Strict old-CSS | Fresh, `010-old-css-strict.log` | 0/0 on this candidate. |
| Spec and Standards reviews | Fresh, checkpoints below | Integrated-reference cross-checks plus the explicitly accepted aggregate coverage; no full-diff line-read claim. |
| [008](008-restore-drag-save-undo-proof.md#answer), [009](009-verify-reload-persistence.md#answer), [022](022-restore-collision-provider-browser-proof.md#answer) Answers and receipts | Retained | Bounded repair/proof history; unchanged relevant source since their gates or superseded by this full combined gate. 009 and 022 are resolved; 008 is covered through 009. |
| [Draft-safety reviewed closure](../../update-draft-safety/issues/015-coverage-mutation-and-closure.md#answer), [019](019-adopt-build-safely.md#answer), [020](020-split-firmware-either-half-central.md#answer), [021](021-repeated-save-toasts.md#answer) | Retained | Reviewed closures and delta rechecks remain applicable to unchanged relevant source; this gate supersedes older full-gate limitations, not their scope/risk caveats. |

003 remains `wontfix` / not applicable following 002's withdrawn MCU premise.
The user-cancelled 004–007 are not blockers and are not restored behaviors.

### Independent review and explicit coverage disposition

Spec checkpoint: `review/spec-010`. Verdict **PARTIAL**:

| Audit item | Disposition |
| --- | --- |
| A1 | Premise withdrawn by the user; no reproduced MCU defect or new repair claimed. |
| A2/A3/A6/A7 | Cancelled-accepted (user cancellations 004–007). Restoration is **ABSENT**: the Relations tab remains, scope-popup duplication remains, disclosure motion is unproven, and 17 chevron selectors remain. |
| A4 | Recovered for the surviving repairs, with their behavioral proof; not evidence for cancelled repairs. |
| A5 | Recovered technically by the fresh complete gate and bound drag/reload receipt. |
| A8 | Reconciled by this record and the map/011 handoff, with historical evidence preserved. |

Cancellations alone yield **PASS-with-accepted-cancellations**; they do not cause
the PARTIAL verdict. **PARTIAL is coverage-capped and is not upgraded here.**

Standards checkpoint:
`/01a1bdfa-320c-76ec-b9f3-1f6d4fcb65e7/notes/review/standards-010`.
Review verdict **CHANGES REQUIRED → required finding resolved** by the R1
coordinator action below; optional findings are explicitly accepted, not fixed.

**COVERAGE DISPOSITION (coordinator, explicit):** neither axis line-read the full
`510861b11..18d4d7f535dca096a811c9e71bbf823f475485fc` (445-path) diff. Coverage
is the aggregate of the per-ticket combined reviews (008–022 chain, including
015/019/020/021 and delta rechecks), both axes' sampled cross-checks and the full
gate's behavioral evidence. The remaining unread source is **accepted on that
basis**, not described as read. The Spec verdict stays **PARTIAL** on record.

### Findings and retained optional debt

**Standards R1 — evidence hygiene: RESOLVED by coordinator relocation,
2026-11-08.** `.evidence/018-green-notice{,-run2}.png`
were moved to `web/target/test-results/retained-evidence/` under their existing
basenames; `.evidence/ui-integration-recovery/021-packaged/export-results.png`
was moved there as `021-export-results.png`.
`009-mutations-spec.json` was an **INPUT spec**, moved from
`.evidence/ui-integration-recovery/` to
`~/.cache/tmp/boardstudio-009-mutations-spec.json`. Mutation **RESULTS** JSONs
remain as machine logs. Historical citations were true at their time and are
preserved rather than rewritten as if those original receipts never existed.

All items below are **RECORDED AS ACCEPTED OPTIONAL DEBT**, not fixed in 010:

- O1: opener-test duplication — retain existing proof; avoid final-verification churn.
- O2: worker spelling test — retain existing proof; avoid final-verification churn.
- O3: verification server log-handle `ResourceWarning` — accepted cleanup debt, no gate failure.
- O4: stale migration-era comments in `runtime.rs` / host `offline.rs` — documentation debt, not a claimed repair.
- Spec optional: `collapsible.rs:11` comment — retained comment debt; no motion-restoration claim.

### 011 coordinator handoff

Candidate: `18d4d7f535dca096a811c9e71bbf823f475485fc`, with only the 010 claim
and these documentation edits after the gate; use the source/build/browser/site
identities above, not an assumed clean tree or another preview.

Scope truth for human review:

- Expect retained **Relations tabs**, **duplicate scope popups**, **abrupt disclosures**
  (restored disclosure motion is unproven) and **unrepaired chevrons**. The 004–007
  restorations are absent by user cancellation, not delivered repairs.
- The MCU defect premise was withdrawn; no MCU repair or fresh original-symptom reproduction is claimed.
- Class B draft safety remains revocable risk acceptance; worker-restart and navigating-client/cache races remain residual risks, not proven safe.
- Firmware is **not hardware-validated** by these gates.
- Successful autosave feedback is **header-only**, not repeated success Toasts.

[011's surfaces × Themes/Appearances × widths matrix](011-populated-workspace-acceptance.md#acceptance-criteria)
is **THE human gate**: Layout, Case, Parts, PCB, Keymap, Keycaps, Library, project
menu and Export; Original/shadcn × Light/Dark × 1440×900, 1024×900, 390×844, with
real populated projects and the user's decision. Automated gates do not close
011 or parent 001, authorize merge/push, or replace the user's live preview.
