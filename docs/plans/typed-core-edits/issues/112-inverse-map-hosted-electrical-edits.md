# 112: Inverse-map hosted and electrical edits

Status: ready-for-agent
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

- [ ] Direct translation red/green cases cover every listed family and meaningful
      translated/rotated pose and connected-reference examples from both halves.
- [ ] Shared definitions and scalar/keycap payloads execute once with unchanged values.
- [ ] Module connection and circuit namespace/join transformations are atomic; local
      targets and unrelated connector/host-net ownership are preserved.
- [ ] Inventory has no unexplained pass-through; composition calls no production path.

Independent verifier: mutate inverse module pose, circuit join/reference translation,
net pin identity and keycap owner mapping separately. If hosted allocation cannot fit
one session, report the precise remaining family to root before further dispatch.
