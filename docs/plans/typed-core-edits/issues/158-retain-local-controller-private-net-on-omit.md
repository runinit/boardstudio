# 158: Retain a Local controller own-pin net through Omit

Status: resolved
Type: build
Blocked by: [157](157-classify-matrix-membership-by-identity.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [147](147-mirror-keycap-settings-and-plain-nets.md), [111](111-integrate-mirrored-half-reconciliation.md)

## Observed regression

The public111 Local-controller fixture establishes an accepted Local controller with its own
mirror-authored pin on a corresponded net. Omitting the source net's last non-controller member
removes the mirror net despite that surviving Local pin. The public assertion fails before
controller Rejoin, so this is Omit loss, not the accepted linked-controller pin-loss behavior.
Evidence: main `.evidence/ticket157/activation-proof/public-after-oracle-fix.log/json` and
`.evidence/ticket111/integration/net-lifecycle-diagnosis.md`.

## Existing contract and scope

147 says Local parts retain their owned pins; ADR0010 retains a net while mirror-authored data
uses it. capture_omitted_copper currently excludes the mirror controller unconditionally from
still_used. Count its surviving pin as use when its Part association is Local, retaining the
existing linked-controller exclusion. Preserve the private-net predicate, circuit-use guard,
owner/board/correspondence validation and copper capture/restore semantics.

Own only lifecycle.rs and existing lifecycle/tests.rs. No visibility, schema, production routing,
new ownership/net policy or electrical resolver changes. Associated Local circuit owner/member
projection is a distinct subsequent preparation ticket, not part of this predicate correction.

## Acceptance

- [x] A compiled private regression fails for Local controller net/pin/board registration loss
      during Omit before the fix, using a valid accepted fixture.
- [x] Local controller own pin retains its mirror net and registration through Omit, repeated
      preparation/Open; controller Rejoin retains existing source-authoritative behavior.
- [x] Linked controller pin remains excluded, while another surviving mirror-owned pin or circuit
      reference continues retaining its net. Malformed ownership is still refused atomically.
- [x] The original public111 Local-controller Preview/Commit test passes unchanged, including
      accepted Preview isolation and existing Rejoin pin-loss assertions.
- [x] Independent Standards/Spec review, meaningful compiled mutations, affected/full checks and
      native-baseline additions pass; root performs local merge and post-merge checks. No push.

## Comments

Claimed after157 fully integrated and post-merge repo/test/lint passed1847 native tests; dev
bookkeeping e03a3be0. Root owns integration/baseline/fullchecks. No human policy decision pending.

## Answer

The existing private omission guard now counts a surviving mirror-controller pin as use when
that controller's Part association is Local. Linked controller exclusion, source-private-net
classification, circuit-use checks and copper handling remain unchanged. No schema, visibility
or production route changed.

The new valid-fixture regression compiled and failed at the missing mirror net before the fix.
It now retains the Local pin, net and board registration through Omit, serialization, repeated
preparation and Open. Duplicate controller ownership is refused atomically. The full private
lifecycle module passes62/62 and Core test-target Clippy passes. Existing linked-controller and
circuit-owner controls remain green; this does not claim every Local-controller/circuit-use
cross-combination was newly tested.

Both independent review axes accept the bounded implementation. Two compiled mutations were
killed (exclude Local controller; retain linked controller), none survived or failed compilation,
and all source bytes restored exactly. Full repo/test/lint passes with1,849 native tests;
baseline adds exactly two names and removes none.

Source6cc41c24 was integrated into the isolated111 draft. Its unchanged public controller test
passes, proving Local-pin retention through Omit, accepted Preview isolation and existing
source-authoritative controller Rejoin pin loss. Full public111 is61/62; the associated Local
circuit payload projection failure remains open for separate159 preparation.

Evidence: main `.evidence/ticket158/` (review, private RED, root-focused-checks, mutation-run,
full-checks, native-baseline-delta and activation-proof). Source `614e3aa3` merged locally into
`dev` at `2cb402e2`, with identical candidate and merge trees. Post-merge repository, test and
lint checks pass with all 1,849 baselined native tests executed; exact HEAD and commands are in
`post-merge-checks.json/log`. Ticket 111 activation and parent 19 remain open. No push.
