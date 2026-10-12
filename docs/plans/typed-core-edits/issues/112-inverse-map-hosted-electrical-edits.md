# 112: Inverse-map hosted and electrical edits

Status: resolved
Type: build
Blocked by: [110](110-inverse-map-mirrored-half-edits.md)
Parent: [19](19-reconcile-mirrored-halves.md)
Decision: [ADR-0010](../../../adr/0010-mirrored-half-is-a-core-maintained-reflection.md)

## Request

Prepare private translation for the hosted/electrical families inventoried in 110.
Do not activate it in Core execution; 111 joins all verified preparation.

## Scope and evidence

`EditOperation`/`lib.rs::apply` at `2478fb151` supply these exact families:
`SetMountedModule`, `RemoveMountedModule`, `EmbedModuleCircuit`, `RemoveEmbeddedCircuit`,
`SetModuleDefinition`, `SetNet`, `SetInputScanMode`, `SetWiringMode`, `SetKeycapBoard`,
`SetMatrixKeycaps`, `SetKeycapKey`. Shared definitions execute once without inverse-transforming
definition geometry. Keycap scalar/legend payloads remain unchanged; owner IDs map.

- Inverse-map module/circuit pose, owner instance/board, connector/upstream/bus/joined-net
  references using 107's ownership. Keep hosted creation and namespace lifecycle atomic.
- Map net/part/board ownership and net-pin part IDs; pad IDs remain unchanged. Use established
  side-name derivation. Scan/wiring modes have no geometry transform.
- Canonicalize source-side additions before derivation allocates counterparts. Preserve
  locked-member/collision refusal and module connector/joined-host-net independence.
- Local/omitted targets use 109's rules. No independent-part companion-mode semantics.
- `MoveParts` and `RemoveParts` whose ids are parts owned by a linked embedded circuit: 110
  refuses them (they are circuit members, not independent parts). Translate them with the
  circuit's namespace and member map, atomically with the circuit's own edit.
- Maintain the exhaustive coverage ledger in 110: translated/delegated/deliberately unchanged/
  unsupported with reason. Unsupported acceptance-bearing behavior remains blocked.

## File ownership

Exclusive for this slice: private translation modules under `core/src/mirrored_half/` or
`core/src/mirrored_half.rs`, `core/src/mechanical/halves.rs` only for association lookup,
and established mirrored-half test files. Hosted/electrical implementation modules and
`core/src/lib.rs` are read-only; request exact additional ownership before helper edits.
No payload/API/visibility widening and no production hook.

## Test-first and acceptance

- [x] Direct translation red/green cases cover every listed family and meaningful
      translated/rotated pose and connected-reference examples from both halves.
- [x] Shared definitions and scalar/keycap payloads execute once with unchanged values.
- [x] Module connection and circuit namespace/join transformations are atomic; local
      targets and unrelated connector/host-net ownership are preserved.
- [x] Inventory has no unexplained pass-through; composition calls no production path.

Independent verifier: mutate inverse module pose, circuit join/reference translation,
net pin identity and keycap owner mapping separately. If hosted allocation cannot fit
one session, report the precise remaining family to root before further dispatch.

## Outcome (2026-10-12)

Implemented in the private child module `core/src/mirrored_half/inverse/hosted.rs` (tests in
`inverse/hosted_tests.rs`), routed from `canonicalize`'s former ticket-112 arm. No call site, no
visibility or payload widening; 111 activates, 115 composes. `linked_part` now delegates circuit
members to `hosted::member_is_linked`, which replaces 110's refusal.

Ledger (no unsupported entry remains for this inventory):

- **Translated:** `SetMountedModule`, `RemoveMountedModule`, `EmbedModuleCircuit`,
  `RemoveEmbeddedCircuit`, `SetNet`, `SetInputScanMode`, `SetWiringMode`, `SetKeycapBoard`,
  `SetMatrixKeycaps`, `SetKeycapKey`, and `MoveParts`/`RemoveParts` on linked circuit members.
- **Deliberately unchanged:** `SetModuleDefinition` (shared definition, executes once).

Judgement calls:

- Module and circuit poses use the exact inverse of the independent-part rule (position by
  `pair::inverse`, heading `inverse_angle + 180`). A circuit's member geometry is not reflected
  by this: members are re-derived forward as independent parts.
- A value the mirror shows unchanged keeps the source's own spelling (connector, bus, upstream,
  pose, net name), so an unrelated edit never rewrites it. Bus ids invert by the board-token
  swap first, then by stripping the `<mirror>/` prefix, each verified by the forward rule.
- `SetNet` is applied as a pin delta on the source net. Pins on mirror-only parts (no source
  counterpart) cannot travel through a source edit, so a request that changes them is refused;
  source-only pins are kept. Net names pass verbatim, as the hosted forward projection copies them.
- A new module or circuit on a mirror board is a source addition with a planned `ModuleAddition`/
  `CircuitAddition` (identity, member, reference, definition and net maps in `Canonical::hosted`);
  a generated host connector is a `PartAddition`. Plan references equal what a real mirror embed
  allocates (tested against `modules::embed`). A joined host net with no source is refused.
- `RemoveEmbeddedCircuit` refuses a locked mirror member, so the lock cannot be bypassed.
- Local targets run as written; Omitted are refused. Keycap board ids map by the pair's board
  ids with no exception overlay (keycap settings have no exception kind of their own).

Findings for 111/115:

- Reconciliation does not project keycap settings after creation, nor non-circuit nets: a
  translated keycap or plain-net edit changes the source only. 115 must decide whether the
  mirror shows it.
- `SetNet` execution registers a new net on the document's first board only; unchanged here.

Evidence: 20 mutations (pose position/heading/instance pin, circuit join/member/net map, source
id, namespace, connector, net pin/pad identity, mirror-only pins, source-only pins, keycap owner
x3 and payload, scan/wiring owner, shared definition): all killed (one survivor, circuit
namespace, was killed after the test was tightened). Logs in `.evidence/ticket112/`. Tests were
written after the code except the ledger change; red-first was not observed for this slice.

### Review round (2026-10-12), red first

Each item below had a failing test before its fix (the five tests failed, then went green):

- **Omitted connector.** A mirror module whose connector is Omitted shows `connection: null`
  (forward clears it). An edit that leaves it null keeps the source's connection; a mirror that
  shows a connection and then drops it still disconnects the source.
- **Generated host connector.** An explicit `<module id>/vik-host-connector` is staged as empty
  while the module is translated, so it is planned (`PartAddition`) like the empty-id case and the
  source connection names `<source id>/vik-host-connector`. An empty bus id stays empty.
  `mirror_reference` is checked against what `modules::set` allocates on the mirror board.
- **Circuit placement (choice: refuse).** The forward rule reflects circuit members as independent
  parts (position mirrored, heading `180 - r`). Embedding at the inverse pose lands a member as
  typed only when it sits on the circuit's x axis and is unturned; otherwise it lands at
  `(x, -y)` with a different heading, and a source circuit that lands it as typed would need a
  reflected module definition (shared definitions are not transformed). A mirror embed with a
  member off the axis or turned is therefore refused, with a message to embed on the source half.
  This is an ADR-0010 consequence, not a 112 defect. Accepted embeds are tested to land each member
  where the forward rule puts it, for both sides.
- **Net pins and circuit exceptions.** A pin on a member of a Local or Omitted circuit is the
  mirror's own pin (`member_state`), so it can no longer be mapped onto the source net.
- **Standards.** The ledger test uses linked requests and asserts each is rewritten. Omitted
  connector, rotated-map source-half embed and `J_VIK` reference cases are added. The dispatch has
  no wildcard arm. `SetKeycapBoard` does not consult the ElectricalBoard exception: keycap
  settings are keyed by board id and have no exception kind; that exception governs wiring only.

Mutation round 2: 27 mutations (the 20 above plus connection retention, generated-connector
staging, placement x2, circuit-exception pins, connector reference, Omitted circuit member), all
killed; logs `.evidence/ticket112/mutation-check-round2.log`.
