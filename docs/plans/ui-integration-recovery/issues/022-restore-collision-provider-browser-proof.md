# 022: Restore collision/provider browser proof

Status: resolved
Type: build
Blocked by: none
Parent: [001: Recovery specification](001-ui-integration-recovery-spec.md)
Related: [Plan map](../map.md), [008 results](008-restore-drag-save-undo-proof.md#answer), [009 results](009-verify-reload-persistence.md#answer), [010 final gate](010-final-technical-verification.md)

## What to build

Restore the required packaged-app collision/provider smoke workflow against the
current shared controls. Complete the scenarios left unreached after the Case
mounting-style control failure and obtain a valid success receipt without
weakening geometry, qualification, uncertainty or provenance assertions.

008 and 009 already prove their bounded drag/save/Undo/reload outcomes; their
complete runner still fails because Case mounting style is no longer a native
select. Preserve those outcomes while repairing the remaining harness boundary.

## Acceptance criteria

- [x] Reproduce the recorded native-select failure on an identified packaged
  build using the supported runner. Distinguish harness assumptions from actual
  product defects; diagnose any newly exposed failure before prescribing a fix.
- [x] Drive the real accessible Case mounting-style control and any other stale
  control or save-status observations required by this same gate. Required
  preconditions fail explicitly; no direct fixture rewrite substitutes for the
  user interaction under test.
- [x] Complete the default collision PCB pair, Case viewer with overflowing
  findings, qualified collision product fixture and unavailable-PCB uncertainty
  scenario, retaining the existing meaningful assertions. Record terminal
  results for each scenario rather than treating an unreached check as passed.
- [x] Keep the existing trusted drag, cancellation, durable save, exact Undo and
  reload checks green. Preserve document/scene equality, revision checks,
  qualification semantics and uncertainty handling; do not loosen tolerances or
  remove checks to obtain a passing run.
- [x] Demonstrate RED/GREEN for the restored harness behavior and run focused
  harness/consumer controls. Preserve failure behavior: a failed or incomplete
  scenario must produce no success receipt.
- [x] Run the complete supported smoke workflow to success and validate its
  receipt against the actual source, packaged bytes and browser identity. Keep
  the complete smoke result distinct from 010's broader full default gate.
- [x] Update the relevant evidence and historical gate records with exact
  commands, tested identities, results and remaining limits, preserving earlier
  failures as history. Use a dedicated browser profile/origin, not the user's
  live preview.

## Verification and scope

Use the existing supported Rust browser harness and runner, existing receipt
validators and repository mutation runner where mutation controls are needed.
Run applicable formatting, lint and harness tests, followed by the complete
packaged smoke workflow. Obtain independent spec and standards review before
closure.

Unrelated opt-in scripts and wholesale browser-tooling migration are outside
this slice. Change another consumer only when evidence shows it is required by
this gate. If a product defect or additional independent repair exceeds this
bounded workflow, record a separate blocker instead of expanding silently.
Ticket 021's save-Toast policy and 019's update-adoption behavior are separate.

## Comments

- 2026-11-06: The user approved this single-ticket scope and making it a blocker
  for 010. No implementation or new browser run was performed during publication.

## Implementation checkpoint (2026-11-06)

**Implementation complete; independent spec and standards review pending at this checkpoint.**
This is not ticket closure or permission to enter the separate review/QA phase.
022 still blocks 010. No product source, 019/021 behavior or 010 final gate was
changed or run. Initial checkout was clean at the HEAD below; implementation
owns only `tools/browser-harness/src/bin/drag_smoke.rs` plus this ticket, its map
row and the evidence addendum in the historical 026 gate record.

### Diagnosis and RED/GREEN

All packaged runs used `python3 scripts/run-drag-browser.py`, the same checked
package, isolated per-run profiles and ephemeral localhost origins, never the
user's preview. Logs below are local, Git-ignored, under
`.evidence/ui-integration-recovery/`; run logs and receipts are under
`web/target/test-results/drag-browser/<run-id>/`.

| Run | Terminal result | Evidence log |
| --- | --- | --- |
| `20261106T232726Z-a838d90aed2c` | **RED**, original harness: `combobox Case mounting style is not a native select`; no receipt | `022-red-packaged.log` |
| `20261106T232914Z-289810fb8598` | Real Select interaction passes; **FAIL** at trusted Check collisions hit test, covered by the setup guide; no receipt | `022-green-packaged.log` |
| `20261106T233219Z-dd1737ad8ce7` | Required Hide guide interaction passes; **FAIL** waiting for findings through removed `.m1-case-collision-checks`; no receipt | `022-green-packaged2.log` |
| `20261106T233551Z-d4b09efd6e18` | **PASS**, 17 checks, receipt validated; before final test-fixture sensitivity and alert-wait corrections | `022-green-packaged3.log`, `022-validate-receipt.log` |
| `20261106T233935Z-abbc5bec9637` | **GREEN**, final source, 17 checks and validated receipt | `022-packaged-final.log`, `022-validate-final-receipt.log` |

- The shared Select exposes a **button** named Case mounting style, a named
  **listbox**, and **options**, not a native select. The harness now uses trusted
  clicks on that button and the exact option scoped to that listbox, then waits
  for the displayed selection and closed trigger. Missing controls fail; no
  synthetic input/change or fixture rewrite replaces this interaction.
- A separate `agent-browser` probe reproduced the guide covering collision
  actions, including after Open full case settings. The real Hide guide action
  made them reachable. Both collision/qualification setup paths now require
  that dismissal, as the drag setup already did. Probe session `bs022`, dedicated
  origin `http://127.0.0.1:41922/boardstudio/`, was closed and server stopped;
  final mounted snapshot: `022-guide-probe.log`. This was a workflow-precondition
  omission, not evidence requiring a product repair.
- Removed panel/provenance classes are replaced by shared `data-slot` Collapsibles
  anchored to Collision check / Witness identity; the container is the named Case
  assembly region. Shared Alert observations use `[role="alert"]`, no longer
  only `p[role="alert"]`, including the completion wait's error branch.
- Pair counts, geometry bounds, document/scene equality, revisions, precision
  intervals, qualification/refinement/uncertainty semantics, provenance content,
  cancellation and receipt validators are unchanged. Existing qualification
  fixture construction is unchanged; no other consumer needed modification.

### Final identities and terminal scenarios

- HEAD: `366d52100d9dbf6686a9cd9dfdf02f0262a13eed`.
- Tested working-tree source identity:
  `282ada9a521305691b0985800dc9d5ebdbf265c011e7bca4c78b781ed150b4d1`.
  This was compared with a fresh `source_identity()` **before** the subsequent
  documentation-only checkpoint edits. Final harness Git blob:
  `40fa2ddab94cdc6c6723dcb8fd4e097d3f5a3e76`.
- Package: `python3 scripts/check.py build` passed, 61.1 s (`022-build.log`);
  no application source changed afterward. Runner's packaged-file digest:
  `d994f02459ec9c21a6d575bfcb652e89178123b3d18c3bfefa703eafc82b4b8f`.
  The separate serving-site identity in `BOARDSTUDIO_DRAG_BINDING` is
  `68357744b7c0f5a8c0ac2a1a8088c0641b66cf1aae967b34c861e8415d496771`.
- Browser: Chromium `156.0.8078.79`, chromiumoxide `0.9.1`, viewport `1513 × 1043`.
  Runner executable `/home/chris/.local/bin/google-chrome` resolves to
  `/usr/bin/chromium`; its `--version` agrees with the receipt. Loaded document
  byte observation: `document_body.matches = true`.
- Final receipt: `20261106T233935Z-abbc5bec9637/receipt.json`, checksum
  `de087f9fdfc276f87852f2d950f5d6bed0accb0cd5401d53c62fe56898f904d4`;
  23:39:36.311Z–23:40:37.144Z. The supported runner's existing `verify_receipt`
  passed, then passed again against freshly recomputed HEAD/diff and packaged
  bytes; supplemental checks verified browser version, viewport and document
  bytes. All three failed packaged runs were checked to have **no receipt**.

| Scenario | Final terminal result |
| --- | --- |
| Default collision PCB pair | PASS: 78 findings, 4 observed PCB pairs, 0 alerts |
| Case viewer with overflowing findings | PASS: scroll/client heights 9859/276, overflow-y auto, viewer 861 × 386; unchanged viewport/container bounds pass |
| Qualified product fixture | PASS: all 21 distinct default and tight pairs; 4 supported Clear results; provenance/source interval, real refinement/preparation and tight uncertainty/refusal assertions pass; canceled run publishes no old findings |
| Unavailable physical PCB | PASS: 1 unavailable alert; all 4 PCB pairs remain uncertain without mesh observation; supported Clear pair retained |
| Existing drag/save/cancel/Undo/reload | PASS: Part/Column/Stagger/Ctrl Offset/Matrix accepted changes and exact Undo; Stagger cancel leaves exact document/scene/revision; saved edit and saved Undo reload at revisions 13/15 with exact identity/document/scene and mounted re-save proof |

### Focused checks and rerun commands

```sh
cargo fmt --manifest-path tools/browser-harness/Cargo.toml --check
cargo clippy --manifest-path tools/browser-harness/Cargo.toml --locked --all-targets --all-features -- -D warnings
cargo test --manifest-path tools/browser-harness/Cargo.toml --locked
CHROME_BIN=/usr/bin/chromium cargo test --manifest-path tools/browser-harness/Cargo.toml --locked --bin drag-smoke mounting_style_uses_trusted_scoped_accessible_options -- --ignored --nocapture
python3 scripts/test-drag-receipt-bytes.py
python3 scripts/test-browser-tools.py
CHROME_BIN=/usr/bin/chromium python3 scripts/test-drag-browser-consumer-controls.py
python3 scripts/run-drag-browser.py
python3 scripts/check-doc-links.py
git diff --check
```

- Final fmt/Clippy/all harness tests passed: **51 passed, 3 ignored**
  (`022-harness-final.log`); the new ignored browser test was run explicitly and
  passed. It first failed with the native-select error in 0.29 s
  (`022-red-control.log`), then passed with trusted input, an out-of-scope option
  decoy, missing trigger and missing option controls (`022-green-control.log`
  and final mutation baseline). An earlier compile-only attempt referenced a
  private cleanup field; the test uses the existing public cleanup method.
- Receipt byte/tamper control: 1 passed (`022-receipt-controls.log`); browser-tool
  controls: 13 passed (`022-browser-tools.log`). Final real-consumer controls:
  **10 passed**, every intentional failure exits nonzero with no receipt
  (`022-consumer-final.log`). An initial invocation without the documented
  `CHROME_BIN` failed browser provisioning; the corrected commands above pass.
- Repository mutation runner: **2 killed, 0 survived, 0 compile failures**;
  `restored_exact = true` for both (`022-mutation-run3.log`, `022-mutations3/`).
  Removing option scoping fails with `ambiguous role lookup: option "Rigid mount"
  matched 2 nodes`; omitting the option click fails waiting for the selected
  Rigid mount. The first unscoped mutant survived because an invalid standalone
  ARIA option was ignored by accessibility; the final decoy is inside its own
  named listbox. That fixture correction first exposed its own unscoped handler
  query, which was corrected before the final passing baseline. Earlier results
  remain in `022-mutations/` and `022-mutations2/`, not reported as final kills.

To regenerate the negative-path spec outside `.evidence/` and rerun:

```sh
python3 - <<'PY'
import json
from pathlib import Path
old = 'driver::click_role_timeout(page, Some(*list), "option", option_label, timeout).await?;'
variants = [('unscoped-option', old.replace('Some(*list)', 'None')),
            ('skip-option', '// Mutation: skip the required option interaction.')]
Path('/tmp/boardstudio-022-mutations.json').write_text(json.dumps([
    {'name': '022-' + name, 'description': name, 'edits': [{
        'file': 'tools/browser-harness/src/bin/drag_smoke.rs',
        'old': old, 'new': new, 'count': 1}]} for name, new in variants]))
PY
python3 scripts/mutation-check.py /tmp/boardstudio-022-mutations.json --command 'env CHROME_BIN=/usr/bin/chromium cargo test --manifest-path tools/browser-harness/Cargo.toml --locked --bin drag-smoke mounting_style_uses_trusted_scoped_accessible_options -- --ignored --nocapture' --evidence-dir .evidence/ui-integration-recovery/022-mutations-rerun --timeout 90
```

The full smoke's runner performs receipt validation against the source/build
identity it just tested. For a fresh-source recheck after these documentation
edits, rerun that supported command rather than comparing this historical
receipt's diff digest to the newly changed documentation.

### Remaining boundary

- [x] Independent spec and standards review completed after explicit approval;
  see the review checkpoint below. Ticket closure and separate QA/final gates
  remain distinct decisions.
- 010's broader full default gate and 011's human acceptance remain open. No
  full native/application/mounted suite, Chrome-pin qualification, update
  adoption (019), or save-Toast policy (021) acceptance is claimed here.
- The post-boundary WebSocket reset is classified by the existing shutdown
  protocol as expected after clean child exit on successful runs; no error
  handling was suppressed. Logs/receipts are local artifacts, not committed
  source copies. No push, merge or commit was performed.

## Independent review checkpoint (2026-11-07)

The user explicitly approved `/code-review` using Sol with `xhigh` thinking on
Pi. Two separate fresh-context reviewers used `pi`,
`openai-codex/gpt-6.1-sol`, `thinking: xhigh`, one per axis. The fixed point was
`366d52100d9dbf6686a9cd9dfdf02f0262a13eed`; the reviewed command was
`git diff 366d52100d9dbf6686a9cd9dfdf02f0262a13eed --` because all four changed
files were uncommitted and the commit list since that base was empty.

### Standards

**0 required findings; 2 optional, low-severity heuristics:**

- Possible Duplicated Code at `drag_smoke.rs:1844,1857,2116,2126,2381`: repeated
  collision-panel/provenance locators encode a shared DOM contract. Consider
  selector constants while leaving scenario assertions inline.
- Possible Mysterious Name at `drag_smoke.rs:2419`: `select_combobox_option`
  now drives the shared Select, not the separate Combobox. Consider a
  Select-specific name.

Neither is a documented-standard breach or a blocker. No required findings in
the documentation changes. These optional suggestions were left unchanged in
this review-only phase, preserving the tested harness identity and bounded diff.
Report: `.evidence/ui-integration-recovery/022-review-standards.log`.

### Spec

**0 findings:** no missing/partial requirements, unasked scope expansion or
incorrect implementation identified. The reviewer traced the interaction and
observations against the current shared Select/Collapsible/Alert/Case code,
trusted driver and unchanged semantic/receipt assertions, rather than relying
only on the implementation checkpoint.

Independent reviewer reruns passed:

- The explicit Chromium Select test listed above: 1 passed, including trusted
  scoped selection and missing trigger/option rejection.
- The real-consumer controls listed above: 10 passed; every intentional failure
  produced no success receipt.
- `python3 scripts/test-drag-receipt-bytes.py`: 1 passed.
- Receipt audit against freshly recomputed package/index bytes, actual browser
  version and viewport, retained historical source identity and matching current
  harness Git blob; all three failed packaged runs still have no receipt.
- `git diff --check`; Standards also reran documentation links (691 files).

The complete build/smoke and final mutation logs were inspected, not independently
reissued by the delegates. Report, exact commands and output:
`.evidence/ui-integration-recovery/022-review-spec.log`.

After both reviews finished, the coordinator independently reran the repository
mutation command above with evidence directory
`.evidence/ui-integration-recovery/022-review-coordinator-mutations`:
**2 killed, 0 survived, 0 compile failures**, by the intended ambiguous-option
lookup and selected-value timeout; both restored exactly. Run log:
`022-review-coordinator-mutations.log`. The harness Git blob is still
`40fa2ddab94cdc6c6723dcb8fd4e097d3f5a3e76`; neither delegate edited tracked
files, and the coordinator changed only this review record and the 022 map row.

**Review complete, not ticket closure.** 022 remains `ready-for-human` pending
the closure decision and still blocks 010. No separate QA, full default gate,
019/021 work, human acceptance, commit, push or merge was performed in this phase.

## Answer

Closed on the user's explicit approval after independent spec and standards
review. The restored packaged workflow passes all 17 checks with a validated
source/build/browser-bound receipt, preserving drag/save/cancellation/exact
Undo/reload, collision geometry, qualification, uncertainty and provenance
assertions. Independent review found 0 required findings and 2 optional,
nonblocking standards suggestions; rejection controls and both coordinator
mutation reruns passed as recorded above.

022 no longer blocks 010. This closure does not close or execute 010's broader
final gate, 011's human acceptance, or the separate 019/021 work. No commit,
push or merge is authorized or performed by this closure.
