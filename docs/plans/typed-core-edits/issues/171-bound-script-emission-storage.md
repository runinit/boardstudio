# 171: Reject oversized geometry-script output before copying it

Status: resolved
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Geometry scripts stop as soon as their shared object-count or combined emitted
string-byte budget would be exceeded. Opening or applying an oversized script
must fail cleanly instead of collecting large amounts of output that will later
be discarded. Normal scripts retain their current output and stable identities.

The audit's 4,149-byte script repeatedly emitted four 4,096-byte string fields.
Packaged WASM linear memory grew from 1,835,008 to 185,532,416 bytes before the
operation limit rejected evaluation. This measures linear memory growth, not
allocator usage or process peak memory.

## Acceptance criteria

- [x] Reproduce the delayed emission-count/byte rejection before the fix. Cover
  both part and rectangle emitters through real script evaluation.
- [x] Before copying emitted strings or appending an object, check one shared
  emission-count budget and one finite, documented aggregate UTF-8 byte budget.
  Count every copied string field and use checked arithmetic for the accounting.
- [x] Part and rectangle emissions share the budgets, including mixed calls.
  Exact-boundary output succeeds and the first over-budget emission is refused;
  a large first emission cannot allocate its copied output before preflight.
- [x] A script cannot catch the budget error and publish a successful truncated
  result or resume copying output. Budget exhaustion stops evaluation and remains
  a failed script operation, with no partial document changes accepted.
- [x] Existing source, operation, string, array, map and recursion limits stay in
  force. Normal emitted IDs, geometry, poses and validation remain compatible.
- [x] Native regression checks cover count, aggregate bytes, UTF-8 accounting,
  mixed emitters, attempted error recovery and unchanged accepted document after
  failed application. Fresh packaged Core WASM evidence exercises the public
  open/apply boundary and shows bounded refusal before the prior growth pattern.
  Record actual measurements without claiming allocator/RSS proof.

## Comments

Created from the user's audit findings after primary-source research. Independent
of the other four active tickets. Bound the existing emission collector; do not add a
replacement scripting service or change unrelated script language behavior.

## Outcome

Implemented in `core/src/script.rs` on branch `dev`. `collect` now charges one
shared `EmissionBudget` before it copies any string field or appends an
emission: `MAX_EMISSIONS` (the existing 10_000 object-count limit, now checked
before the push) and a new finite, documented aggregate
`MAX_EMISSION_STRING_BYTES = 1024 * 1024` UTF-8-byte ceiling. Every copied string
field is counted with `checked_add`. Both the `part` (group, local, definition,
reference) and `rect` (group, local, operation) callbacks draw on the same
budget. Exhaustion is sticky and reported as Rhai's non-catchable
`ErrorTerminated`, and `collect` re-checks the exhausted flag after evaluation,
so a `try/catch` cannot swallow it or publish a truncated result. Source,
operation, per-string, array, map and recursion limits are unchanged; the
previous post-evaluation `MAX_EMISSIONS` check is now redundant and removed.

Native regression tests (`core/src/script.rs` `mod tests`, run by
`cargo test -p boardstudio_core --locked --lib`) cover: exact aggregate-byte
boundary succeeds; first over-byte emission refused; UTF-8 byte (not character)
accounting; mixed `part`/`rect` sharing; attempted `try/catch` recovery stays
failed; count boundary (10_000 allowed, 10_001 refused, catch-attempt refused);
and unchanged accepted document after a failed `apply_scripts`. Existing
`core_engine::scripts_emit_stable_ids_and_keep_visual_pose`,
`core_engine::bad_script_is_atomic_and_limited` and all 825 lib tests pass.

Fresh packaged Core WASM (`wasm-pack build web --target web --features
core-worker --dev`, the production core-worker package) drove the public
`CoreEngine.request` open and `replace-document` apply boundary in Node. Before
the fix, the audit pattern (`while true { part(s,s,s,s,0,0,0); }` with a
4_096-byte field) grew linear memory 183_762_944 bytes on open and 183_697_408
bytes on apply before the operation limit rejected it; the rectangle loop grew
124_452_864 bytes. After the fix the same scripts fail with `Script emission
budget exceeded` after exactly 1_048_576 bytes of growth for both emitters and
1_572_864 bytes for the large-first-emission/try-catch cases, and the accepted
document is unchanged. Normal scripts still emit `script/s/main/k1` and
`script/s/main/board` through both boundaries. These are WASM linear-memory
observations for these probes, not RSS or an allocator-wide peak.

Evidence: `.evidence/ticket-171/` (`red-native.log`, `green-native.log`,
`core-suite.log`, `clippy-fmt.log`, `probe.mjs`, `packaged-prefix.json`,
`packaged-fixed.json`). Not checked here: the integrated full `check.py` cohort
(parent-owned).
