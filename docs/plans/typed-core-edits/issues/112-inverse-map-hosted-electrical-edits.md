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

## Execution and verification gates

This is a child of [19](19-reconcile-mirrored-halves.md); resolving it does not resolve 19.
The entire 19 execution batch runs **alone**: no independent implementation or heavy checks.
Root must first finish 37, integrate/sync the verified prerequisite lanes, complete the batch
check, refresh the bound lane index, and explicitly grant this slice and its exclusive files.
Use `/home/chris/01_Projects/ts-boardstudio2/.worktrees/lane-mirror-core`, branch `lane/mirror-core`;
report path/branch/status and inspect overlaps before editing. Every shell call has that
absolute `workdir`; commands run one at a time in the foreground.

Read `docs/agents/agent-workflow.md`, `verification-process.md`, `gitnexus-index.md` and
parent 19. Luna 6 high implements test-first and records `implemented (awaiting verification)`;
a fresh Sol 6.1 xhigh independently verifies/mutation-checks, then alone may set `resolved`.
Run bound GitNexus impact before every symbol edit; list callers, processes and risk.
`apply`/`layout::resolve` are known CRITICAL paths: warn before editing. UNKNOWN, stale or
missing answers need source confirmation and honest limitation reporting, not an all-clear.
Use `boardstudio-lane-mirror-core`; root alone refreshes. Run complete `detect_changes` before
any commit; truncated/partial evidence is not clean.

Request every test/build/browser/performance command slot from root before running it.
Affected checks: `repo lint test typecheck`, plus native/WASM boundary parity when its
behavior/transcript changes; targeted red/green and verifier mutations also need slots.
No full check per slice, no `release`, no background checks; Cargo uses 12 jobs.
Maps, ADRs, CONTEXT.md, scripts/check.py and generated goldens remain root-owned.
No merges/push/PR, stash/reset/discard or broad staging; explicit paths only.
Commit only after root permits it, with the actual-author trailer. Outcome records exact
commands/results/timings, mutation failures/restoration and every unrun or blocked gate.
