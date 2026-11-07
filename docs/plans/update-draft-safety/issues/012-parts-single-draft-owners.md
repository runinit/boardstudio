# 012: Parts single-draft owners

Status: ready-for-agent
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the Parts single-draft
editors sharing the one-Save/Apply pattern: generator parameters (including
its Upload admission key), the module profile editor, and the mechanical fit
profile editor. Their unapplied Draft values defer Update Now until resolved
through each editor's own controls.

## Acceptance criteria

- [ ] Each owner defers Update Now with a real valid unapplied draft and with a realistic invalid unsubmitted draft, at both entry points, via the mounted Update Now seam.
- [ ] Generator parameters are judged per admission key: submitting one key (Apply or Upload) does not clear an unresolved draft belonging to the other.
- [ ] Each deferral message names its editing area and per-key remedy; resolution happens only through that editor's own controls.
- [ ] Per-key behaviour follows the contract: landing updates the baseline, failure/refusal never clears, revert-to-baseline clears, newer typing re-raises, older settlement never clears newer input.
- [ ] Hidden or keep-mounted instances keep blocking; unmount or owner change retires them.
- [ ] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map). Starts only after the
approved verification-reliability preparation finishes
([execution](../execution.md#verification-reliability-preparation)). The rows restate the criteria
above per owner and add none. The shared gate, epoch and lifetime mechanics are proven at the 008
and 011 seams, so each row proves owner wiring only. Owners: generator parameters (Apply and Upload
keys), module profile, mechanical fit profile. Tier `mounted` (Update Now seam); every row
`planned`.

- P1 Deferral — a real valid unapplied draft, or a realistic invalid unsubmitted draft the owner's
  own submit would not accept → Update Now from the toast and from the header → the update defers
  and the draft value is untouched; the message names the editing area and the per-key remedy. Only
  the editor's own controls resolve it. Oracle: deferral state, message against the settled
  per-(owner, key) copy, unchanged draft. One scenario per owner; draft kind and entry point are
  table cases.
- P2 Generator admission keys — drafts unresolved under both Apply and Upload → submit one key and
  land it → the other key still defers with its own remedy. Oracle: the remaining deferral names the
  unsubmitted key only. Both directions (Apply→Upload, Upload→Apply).
- P3 Per-key lifecycle wiring — submission in flight → landing, failure or refusal, revert to
  baseline, newer typing before the older settlement → landing re-baselines, failure or refusal keeps
  deferring, revert clears, newer typing re-raises and the older settlement never clears it. Oracle:
  deferral state after each outcome. Owner specifics: module profile's one Save click admits two
  collections (profile and rotary), and either older settlement must leave newer input deferring;
  text an owner parses before submit compares by the value it would submit.
- P4 Retention — owner hidden but mounted → Update Now → still defers; owner unmounted or
  replaced → no deferral from the retired entry. Oracle: deferral present, then absent.
- P5 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.
