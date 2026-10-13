# 155: Admit hosted additions into durable half correspondences

Status: ready-for-agent
Type: build
Blocked by: [153](153-validate-independent-exceptions-with-matrix-owners.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Related: [154](154-reserve-staged-circuit-member-references.md), [111](111-integrate-mirrored-half-reconciliation.md), [112](112-inverse-map-hosted-electrical-edits.md), [146](146-mirror-side-circuit-embeds-land-as-typed.md)

## Source-proven preparation gap

154's reference fix allows the circuit embed, but a subsequent source member MoveParts updates
only the source: requested(23,11), mirror remains(180,7) rather than(177,11). AddCircuit and
AddModule stage their target objects and private replay provenance without admitting their
selected identity maps into durable halves.correspondences. Later inverse/forward lookup uses
only those durable links.111's member_poses wiring handles initial placement, not admission.
Evidence: main `.evidence/ticket154/pose-delta/prepared.txt` and `member-move-audit.md`.

112 already requires linked circuit member MoveParts/RemoveParts; no new product policy is needed.
This is separate private preparation before activation.154 stays incomplete until this dependency
is verified; its WIP reference fix and failing pose regression are preserved separately.

## Scope

Compose the already selected, authenticated hosted AddModule/AddCircuit maps into the same staged
pair record before the existing final validation/publication boundary. Reuse existing identity,
namespace, ownership, transition and projection checks; do not build another admission engine.
Admit only existing kinds: Module; EmbeddedCircuit; member Part; newly owned private Net.
Joined host Net links must reuse the exact existing correspondence and reject conflicting maps.
Definitions retain their existing namespace/Part.definition_id mapping: there is no Definition
correspondence kind, schema addition or helper visibility grant. Preserve references, geometry,
Local/Omitted state, unrelated pairs/objects and authenticated replay behavior.

Exclusive source scope: `core/src/mirrored_half/reconcile.rs` and its established private inline
tests. Root assigns further paths if proven necessary. Keep154 allocator edits and111 member pose
activation in their existing worktrees; no model/ops/runtime/config/baseline/golden edits.

## Acceptance

- [ ] Red-first private AddModule and AddCircuit probes expose missing durable links.
- [ ] Successful staged admission commits exact selected owner/member/private-net records once;
      joined Net and definition identities retain existing rules; unrelated records remain unchanged.
- [ ] Valid replay/reconciliation is idempotent; incomplete/ambiguous/conflicting/wrong-board
      maps refuse atomically using existing validation.
- [ ] Subsequent source and mirror member/module edits use the admitted links; save/reopen
      retains them, and supported removal does not leave stale records.
- [ ] Independent Standards/Spec review, meaningful admission mutations and required checks pass.

Root owns full checks/baseline/local integration. No push.155 can be verified on153's main source
with directly selected private transition maps; it does not depend on154's allocator fix.
After155 integrates,154's preserved prepared-path pose regression must pass before154 closes.
