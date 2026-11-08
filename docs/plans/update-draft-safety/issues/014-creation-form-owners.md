# 014: Creation-form owners

Status: resolved
Type: build
Parent: [Draft-safe update adoption spec](../spec.md)
Blocked by: [Draft-owner registry and Geometry scripts tracer at both Update Now checkpoints](008-registry-and-geometry-tracer.md)

## What to build

Registry adapters and per-key deferral copy for the creation forms that have
no accepted value to project — the new matrix form, the mirrored-pair form,
and existing-half setup. Their entered values are judged against the snapshot
of the form's starting values taken at mount, so a half-filled creation
defers Update Now like any other unresolved draft, including input the form
itself would silently reject on submit.

## Acceptance criteria

- [x] Each form defers Update Now when its entered values differ from its mount-time starting values, and clears on revert to those values or on resolution through the form's own controls.
- [x] Silently-rejected invalid input (values the submit path would return without effect) still defers; protection never depends on a submission attempt or an invalid-field marker.
- [x] Each deferral message names its editing area and remedy.
- [x] A form that unmounts without submitting retires its registration and never blocks a later update.
- [x] Hidden or keep-mounted instances keep blocking.
- [x] Each adapter change carries its own RED/GREEN evidence through the mounted seam; focused checks pass and evidence logs follow the plan's conventions.

## Answer and completion record

- Matrix isolated slider/preset genuine pre-fix RED: `014-matrix-red-slider.log`,
  `014-matrix-red-preset-settled.log`; focused GREEN `014-matrix-final-focused.log`.
- Pair and existing-half focused GREEN: `014-pair-focused-final.log`,
  `014-half-focused-final.log`. Their pristine pre-implementation RED remains a
  proof gap because the initial fixtures required correction; later disconnected-
  adapter controls RED in `014-red3-adapter-absent-*.log` are not represented as
  pre-edit RED. The user explicitly approved these later adapter-disconnected
  failure controls as a narrow substitute for the missing pre-implementation REDs,
  subject to independent review that each control removed only its adapter. This
  does not waive GREEN tests, reviews, or other gates.
- Coordinator independently reran invalid-submit guards:
  `014-coordinator-invalid-guards.log` (2/2). Initial full Layout browser subset
  failed on a foreign invalid Rows field left by an earlier test; document-wide
  oracle and exact fixture failure: `014-layout-document-oracle-red.log`. Fixture
  cleanup guard RED and GREEN: `014-matrix-fixture-cleanup-guard-red.log`,
  `014-layout-matrix-fixture-cleanup-green.log`. Coordinator reran the guard and
  Layout browser subset: `014-coordinator-fixture-guard.log`,
  `014-coordinator-layout-green.log`.
- Real held Create uncovered missing same-key exemption for Matrix and existing
  half; RED/GREEN: `014-inflight-matrix-{red,green}.log`,
  `014-inflight-half-real-{red,green}.log`. Pair Preview is not admission; its
  placement unmounts the form, and Create failure remount had reset the baseline:
  `014-inflight-pair-{probe,green}.log`. Coordinator reran real pending and pair
  failure guards and full Layout subset: `014-coordinator-inflight-guards.log`,
  `014-coordinator-pair-failure.log`, `014-coordinator-layout-final.log`.
  The delegate reports newer typing during pending is unreachable because controls
  are disabled or absent; independent review must verify this classification.
- Coordinator completed the full owned diff read and all guard reruns above.
  Final Layout run passed both launch groups (266 + 2 = 268 tests) and test-CSS.
  Fresh full-source Standards PASS (`014-standards-review-fc2ffb4/checkpoint`);
  prior full-source Spec coverage plus final evidence recheck Spec PASS/READY
  (`014-spec-final-2026-11-08`). No Required findings remain. Earlier PARTIAL
  reports remain historical and are not relabeled.
- Independently verified substitute-control construction and exact restoration:
  `014-control-{pair,half}-{delta,red,green,hashes}.log`; only the respective
  registration was disconnected. The user-approved P5 substitution is satisfied.
- Pending input classifications are structural: Matrix/existing-half controls
  are disabled; Pair form is absent in Placement. Pair Preview alone is not
  admission. Failure remount retains the original owner baseline. Spec and
  Standards verified these boundaries; no forced-input proof is claimed.
- Base before this ticket commit: `b99997d467220f2154454f650f89f6f5378eebc1`.
  External standards/research and the separate Gortex-audit files are excluded
  from this ticket commit. Full project gate, mutation and packaged Layout leg
  remain 015 scope. Next: 015; no 019 umbrella closure.

## Proof map

Follows the [proof map protocol](../../../agents/issue-tracker.md#proof-map); the rows restate the
criteria above per form (new matrix, mirrored-pair, existing-half) and add none. The shared gate and
lifetime mechanics are proven at the 008 and 011 seams, so each row proves form wiring only. Tier
`mounted` (Update Now seam); P1/P3/P4 GREEN, P2 GREEN for pair/half and approved structural N/A for matrix, P5 GREEN under the explicit evidence substitution above.

- P1 Mount baseline — form freshly mounted and untouched → Update Now → no deferral; an entered value
  differs from the mount-time starting values → defers; revert to them or resolve through the form's
  own controls → clears. Oracle: Update Now defers or proceeds; the message names the editing area
  and remedy. One scenario per form.
- P2 Silently invalid input — a value the form's submit path returns on without effect, entered with
  no submit attempt and no invalid-field marker → Update Now → still defers (and still defers after a
  submit attempt that returns without effect). Oracle: deferral state with no invalid marker
  present, so the marker query cannot be the cause. The invalid value is chosen from each form's
  own submit path.
  User-approved matrix exception: P2 is structurally N/A for the New matrix
  form. `MatrixSetup` silently rejects only unparseable row/column strings;
  its production `MatrixDimensionSliders` writers round/clamp and format
  integer strings, with no text entry. The remaining two forms still require P2.
- P3 Unmount without submit — value entered → form unmounted unsubmitted → later Update Now → not
  deferred by the retired registration. Oracle: update proceeds.
- P4 Retention — form hidden but mounted → Update Now → still defers. Oracle: deferral present.
- P5 Adapter RED/GREEN — each adapter's rows run RED then GREEN through the mounted seam. A row the
  existing product already passes is a `proof gap` recorded GREEN, not a fabricated RED.
