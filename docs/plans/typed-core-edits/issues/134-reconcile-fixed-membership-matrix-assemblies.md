# 134: Reconcile fixed-membership matrix assemblies

Status: resolved
Type: build
Blocked by: [105](105-mirrored-half-derivation-kernel.md), [132](132-reconcile-fixed-membership-matrix-geometry.md)

Parent: [106](106-reconcile-mirrored-matrices-layouts-parts.md), within [19](19-reconcile-mirrored-halves.md). This isolated child does not wait for 106 or 19 to resolve. B completed the root-granted Luna implementation and fresh, different Sol 6.1 xhigh verification cycle. Bounded acceptance and the full integration cohort passed; source and mutation leases are released.

## Scope

In core/src/mirrored_half/reconcile.rs only, reconcile the existing fixed membership of two already-materialized, nonempty matrix assemblies through the private derivation seam from 105. Preserve both assemblies and their dependencies, every member's effective placement mode and tweak, and existing materialized Some(LiteralMirror) fallback. This is not allocation, API or visibility widening, topology change, deletion, schema or activation work, and does not repair a missing source reference.

The accepted independently hand-calculated oracle is retained in the readiness packet and independent review linked below. It covers the 2×3 keys and eight companions, 28 ownership entries, the retained dependencies on both nonempty assemblies, and exactly these changed IDs (sorted mirror-matrix last):

- matrix/mirror-matrix/r0c1
- matrix/mirror-matrix/r0c1/diode
- matrix/mirror-matrix/r0c1/fallback
- matrix/mirror-matrix/r0c2
- matrix/mirror-matrix/r0c2/diode
- matrix/mirror-matrix/r1c0
- matrix/mirror-matrix/r1c0/diode
- matrix/mirror-matrix/r1c2
- matrix/mirror-matrix/r1c2/diode
- matrix/mirror-matrix/r1c2/stab
- mirror-matrix

Use the packet's literal key/companion positions, headings, axis and rotated transforms, per-item settings and tolerance. Tests must assert literal expected poses and roles independently of producer output; preserve distinct IDs/nets and references. Require exact whole-document repeat equality and empty changed IDs on repeat.

## Acceptance

Validate the fixture and freshly bind the graph before source edits. Demonstrate a compiling, intended regression RED, then GREEN. Cover missing, mismatched, duplicate, reversed and touching associations; malformed or missing owners including the second retained child; inconsistent fixed assemblies; non-finite values; unsupported local/override/variant/independent-solver cases; and the existing 132 rejection coverage. Verify each retained diode's nonempty Pad.id-based net, constraint, global-outline and board-registration references.

Fresh independent verification must make meaningful temporary mutations to mode/fallback, stabilizer heading and second retained-child ownership; each must fail at an intended assertion, then be restored byte-for-byte with hashes recorded. Run focused, affected-crate, lint, repo and typecheck gates as applicable. Parent106/19 remain incomplete and unactivated; root owns integration, final full batch, goldens and index refresh. No hardware acceptance is implied.

## Evidence

- Root-approved planning packet: /home/chris/.cache/boardstudio-baselines/ticket106-planning/fixed-membership-assembly-readiness-20261011.md, SHA256 63d5126fcb854a27321defecd3dbeeee0c385c1c55c87d303e6feea1154f6f68.
- Independent scope/oracle review: /home/chris/.cache/boardstudio-baselines/ticket106-planning/independent-assembly-scope-oracle-review-20261011.md, SHA256 d3e881c695572063b2ea9420fc6ef5eab3d328e2f642f18d4f2def1dab4bdc53.
- B planning gate: PASS. Implementation, fresh independent verification and the full integration cohort are complete; source and mutation leases are released.

## Outcome (2026-10-11)

Fresh independent Sol6.1 xhigh PASS for bounded existing fixed-membership matrix assemblies. Sole source core/src/mirrored_half/reconcile.rs, final SHA256 253a6db355e731e1c11801763cd1c1916023237e012e04ff3db754bf05376fb5. All seven prior test bodies are preserved. Verifier strengthened vacuous non-pose proof with distinct valid nondefault sentinels and added the small pre-setter refusal for unsupported authored/constrained linked keys, keeping retained constraints valid.

The accepted fixture proves per-item mode/tweak/fallback, stabiliser heading,28 owners, both retained nonempty dependency closures, literal poses, exact11 changed IDs, final materialization and whole-document repeat equality. Final focused9/9 PASS; repo/lint/test/typecheck PASS91.7s; diff check PASS. Three compiling intended failures cover fallback mode, stabiliser heading and second-retained-child ownership. The last rejects at the staged-owner guard before dependency assertions. Each exact restore/touch/hash was followed by9/9 GREEN. Probes ran on strengthened pre-key-guard7c5f9b95... source; unchanged producer sites and positive fixture were bound to final253a6db3... with final focused/affected GREEN. No mutant remains. Parent106/19 stay OPEN; private reconciliation is unactivated.

Authority: /home/chris/.cache/boardstudio-baselines/ticket134/independent-fresh/independent-assessment.md. Root source commit `bcdef94c`, merged with `--no-ff`. One full default cohort check passed at integration `6eeef607adc186b162e38ff22ac54521de42d809` in 510.0 s (repo 0.1, tooling 7.5, lint 10.1, build 142.4, test 70.3, browser 279.5); all 70 presentation tests passed. Log: `/tmp/boardstudio-cohort-133-134.log`.
