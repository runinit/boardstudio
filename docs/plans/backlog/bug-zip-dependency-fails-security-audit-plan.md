---
title: 'Fix the zip dependency audit failure'
type: bugfix
ticket: '5'
created: '2026-11-17'
status: built
baseline_revision: '4dc2cfcde38e7fd869fc1787390ef5ea038abb3a'
route: oneshot
route_source: auto
risk: low
review: quick
review_source: pinned
lenses_ran: [quick]
review_loop_iteration: 0
context: []
---

<frozen-after-approval reason="human-owned intent">

## Intent

The repository security gate blocks both hosted CI trials on RUSTSEC-2026-0548. Resolve the affected workspace dependency, preserve native and WASM archive compatibility, and deliver a draft PR against the common base. Keep existing audit policy, trial PRs and the CI progress epic outside this change.

</frozen-after-approval>

## Implementation Notes

- One-shot route for the exact version pin and its lockfile resolution. Core owns the pin and uses the existing deflate backend.
- The hosted audit and local baseline both exited 1 on zip 9.0.1. The other four tracked lockfiles contain no zip 9.0.1.
- Ticket 5 passed independent BMad ticket validation before implementation.
- Gortex rejected writes after the branch switch. Used the repository's native-write fallback after retrying.

- Cargo resolved zip 9.0.5 after the exact pin changed. The workspace lock entry changed only version and checksum. The enabled deflate features and all other tracked lockfile entries matched the base.
- The first legacy archive invocation named the module as a test target. Cargo rejected that invocation, and the corrected `integration` target with the module filter ran four tests.

## Plan Change Log

## Review Triage Log

- Quick independent review completed with no findings.

## Verification

- `python3 scripts/check.py security` must exit 0 with no RUSTSEC-2026-0548 finding.
- `cargo test --locked -p boardstudio_core --lib archive::` must pass the archive unit tests.
- `cargo test --locked -p boardstudio_core --test integration legacy_archive_migration::` must pass the legacy archive integration tests.
- `cargo check --locked -p boardstudio_core --target wasm32-unknown-unknown` must pass.
- Compare the draft branch to `46d758d3fe8fc049084c87f077083dbe3eae137b` and verify the diff is limited to the fix and its BMad records.

**Local results**

- Baseline security gate exited 1 on zip 9.0.1 and RUSTSEC-2026-0548. The patched gate exited 0, with 6 audit-script tests passing and 1,527 advisories loaded from database revision `233155b4025b854c68a4b199c0e32d7fd53291a2`.
- The patched audit reported 0 blocking findings for 434 workspace, 186 Monstertruck and 27 vendored cgmath dependencies. Allowed warnings remained 1, 4 and 0 respectively.
- Core archive unit tests passed 24, with 1 existing ignored child probe. Legacy archive integration passed 4. The locked Core WASM check passed.
- The shared-base diff contains the Core manifest, root lockfile, and this ticket and plan.
