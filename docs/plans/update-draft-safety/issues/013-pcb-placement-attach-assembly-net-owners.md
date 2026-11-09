# 013: PCB placement, attach, assembly and new-net owners

Status: resolved
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the PCB cluster: mounted-module
placement, module-attach options, the assembly draft form, and the new-net
name form. The assembly form is the multi-admission exemplar — Save assembly,
Place on board and Apply to matrix are separate admission keys over one draft.
Attach options and new-net are judged against their mount-time starting
values, including the new-net draft's survival of part switching and collapse.

## Acceptance criteria

- [x] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [x] Assembly drafts are judged per admission key: Save, Place and Matrix each clear only their own key; submitting one never clears a draft belonging to another, and an older settlement never clears newer input.
- [x] Mount-snapshot owners (attach options, new-net name) defer when their entered values differ from their mount-time starting values and clear on revert or resolution.
- [x] The new-net draft keeps blocking while its owner stays mounted across part switches and collapse, and retires on workspace switch or unmount.
- [x] Each deferral message names its editing area and per-key remedy; resolution happens only through that form's own controls.
- [x] Hidden or keep-mounted instances keep blocking; unmount or scope change retires them.
- [x] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above per owner and add none. The shared gate, epoch and lifetime mechanics are proven at
the 008 and 011 seams, so each row proves owner wiring only. Tier `mounted` (Update Now seam);
rows P1–P7 GREEN (owner and timing evidence below).

- P1 Deferral — per owner (mounted-module placement, attach options, assembly draft, new-net name):
  a real valid unapplied draft, or a realistic invalid unsubmitted draft → Update Now from the toast
  and from the header → the update defers and the draft is untouched; the message names the editing
  area and per-key remedy. Oracle: deferral state, message against the settled per-(owner, key)
  copy, unchanged draft. Draft kind and entry point are table cases.
- P2 Assembly admission keys — tree and origin values unresolved → submit one key. Track each
  outcome separately: Save landing re-baselines the saved tree but leaves origin deferring with
  the Place-or-close remedy; successful Place's existing owner close retires its entries, so the
  oracle is no deferral from that retired owner, not other keys still deferring. Matrix leaves the
  owner and origin mounted; the Place-only origin continues to defer after Matrix lands. Submission
  memory remains per key; one key in flight never exempts another key's unresolved draft.
- P3 Assembly older settlement — Save and Matrix in flight with different submitted values, newer
  typing while the owner stays mounted, then either older settlement → the newer input still
  defers. Oracle: deferral state after each settlement. Place's existing owner retirement is the
  distinct P2 lifetime case, not an assertion that the editor remains mounted.
- P4 Mount-snapshot owners (attach options, new-net name) — entered values differ from the
  mount-time starting values → defers; revert to them or resolve through the form's own controls →
  clears. User-approved correction: changed attach options still defer during reachable
  pre-admission preparation; separately, actual connector loading with unchanged defaults
  remains safe and locks the controls until preparation finishes. Changed options during
  connector loading are unreachable: loading requires the mount defaults and locks controls.
  Oracles: deferral present then absent; loading state and control lockout.
- P5 New-net lifetime — name typed → switch part, collapse → still defers; workspace switch or
  unmount → no deferral. Oracle: deferral state at each step in one scenario.
- P6 Retention — an owner hidden but mounted → Update Now → still defers; unmount or scope change →
  no deferral from the retired entry. Oracle: deferral present, then absent.
- P7 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.

## Answer and completion evidence

- Initial assembly baseline 5/5 and expected missing-deferral RED:
  `.evidence/update-draft-safety/013-assembly-{baseline,red}.log`.
- Owner RED/GREEN and subsequent lifetime fixes: `013-resume-*.log` and
  `013-recovery-*.log` under `.evidence/update-draft-safety/`.
- Additional P1–P6 proofs are GREEN proof gaps, not fabricated product REDs:
  `013-proof-completion-focused-green.log`, `013-proof-completion-pcb-green.log`,
  and the assembly segment of `013-proof-completion-parts-green.log`. That last
  log also includes an earlier failed attachment test-authoring run, not a
  product regression; the corrected attachment run is in the focused GREEN log.
- Real connector-loading proof and temporary asset staging: `013-p4-loading-*.log`,
  `013-{red,tooling-green,browser-green,guards-green}.log`. Coordinator reran
  refusal guards and the current full Parts browser subset:
  `013-coordinator-runner-guards.log`, `013-coordinator-parts-latest.log`.
- Coordinator independently read the full owned diff and reran the stale connector,
  stale/failed new-net, staging refusal and replacement guards, plus the origin
  preparation regression. Final Parts and PCB browser subsets: 106/106 and 64/64,
  `013-coordinator-browser-final.log`; guards: `013-coordinator-{attach-reject,net-reject,runner-guards,origin-preparation}.log`.
- Origin preparation was an additional genuine RED (`013-origin-preparation-red.log`):
  preparation had incorrectly exempted an unadmitted draft. The fix is GREEN
  (`013-origin-preparation-green.log`) and the coordinator reran it independently.
- Initial review attempts were PARTIAL. Fresh bounded full-source Spec and Standards
  reviews PASS at assembly blob `7aee1b9a` and the pinned companion blobs; fresh
  Spec and Standards delta reviews PASS for `7aee1b9a..48e22b5a`, with no Required
  findings. Review checkpoints: `review-013-checkpoint`,
  `review-013-standards/checkpoint`, `review-013-delta/checkpoint`, and
  `review-013-assembly-editor-delta`. Optional test-selector and stub suggestions
  do not expand scope. The unrelated externally modified coding standards and
  research file are not part of this ticket.
- P1–P6: mounted owner tests and actual Place/Matrix/attach/new-net lifecycle in
  `013-proof-completion-{parts-green,focused-green,pcb-green}.log`, plus the real
  loading case and origin regression above. P7: per-owner initial REDs in
  `013-{assembly-red,resume-attach-red,resume-module-red,resume-net-red}.log`,
  followed by GREENs in `013-{recovery-assembly-final2,recovery-attach-final2,recovery-module-final,recovery-wiring-final2}.log`;
  later already-passing cases are proof-gap GREEN, not manufactured REDs.
- Current base before ticket commit: `f3aa548bb7dfc64ba1e3e036cd59d2467b0bf47d`.
  Full project gate, packaged Layout leg, mutations and final integrated review
  remain 015 scope; no 019 umbrella closure is claimed. Next: 014.

## User decision

The original question incorrectly assumed Apply to matrix consumes the X/Y
origin. Source inspection establishes that those fields are passed only to
Place on selected board; Matrix receives the assembly definition, not origin.
After that correction, the user approved retaining existing product behavior:
protect X/Y as Place-only draft parameters. Matrix submission never resolves or
exempts them; they remain unresolved until Place, revert, or owner close. Matrix
resolves only the assembly changes it actually applies, subject to per-key
safety. No Matrix-origin baseline amendment or geometry change is authorized.
Normal save and Update Now safety still apply; durability must be verified.

## Decision required before implementation (historical)

The original acceptance criteria above remain unchanged. P2 cannot yet claim permanent origin
resolution after Matrix: [004's Pair-only amendment](004-choose-draft-safety-interface.md#later-user-amendment-sticky-pair-action-parameter-2026-11-07)
grants no other action parameter a successful-submit baseline. Matrix leaves the origin mounted
and unchanged, with only its mount-default baseline, so it re-raises after the in-flight exemption
ends. Before 013 starts, the user must reconcile the per-key resolution criterion with that rule:
retain this re-deferral, or explicitly amend the origin baseline contract. No amendment or editor
behavior change is authorized by this proof map. The successful Place close remains the existing
owner-retirement case. This question does not block the independent A/B/C preparation or 012.
