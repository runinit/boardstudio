# Golden baseline for the Rust footprint generators

Recorded from the JavaScript provider (`ergogen/src`, `kicad/src/ergogen.ts`,
`scripts/web/preview-generator-worker.ts`, the 36 generators in
`ergogen/generated/catalogue.mjs`) at commit `851875a0e`, as step 1 of
[the plan](../../../docs/investigations/footprint-generators-rust.md). The
JavaScript is deleted in step 6, after which these files cannot be regenerated;
treat them as read-only inputs. Expected changes (listed deviations, the
`ergogen:` → `generator:` / `bundled-model:` rename) are applied by hand or by a
one-off rewrite script, never by re-recording.

## Files

| Path | Contents |
| --- | --- |
| `generators/<namespace>__<name>.json` | One file per generator: `source`, `parameters` (provider `parameters()`), `catalogue` (its `catalogue()` definition) and `cases`, one JSON object per line |
| `provider.json` | Provider-level behavior: `isErgogen`, unknown generator / unsupported version / missing generator errors, `parseForms` inputs and results, `modelAssetIdsForPaths` |
| `worker.json` | 28 preview-worker request/reply pairs: net allocation, unresolved model paths, job failures, and every envelope validation message |
| `numeric_vectors.json` | JavaScript number formatting and parsing, `Math.sin`/`cos`/`hypot` and `encodeURIComponent` results recorded from V8 |
| `context_vectors.json` | The render context (`p.at`, `isxy`, `esxy`, nets, local nets, side, parameter merging) recorded through a synthetic generator |
| `manifest.json` | Counts per generator and family, demo-project coverage, source hashes, Node version |
The temporary Node recorders were deleted after the Rust cutover. Their source
and the original generator implementations remain available in Git at `851875a0e`.

## Case record

`{ id, family, input, output }`. `input` is the definition parameter overrides
and, for placed cases, the part (`pose`, `side`, `generatorParameters`) with the
reserved net table (`GND` = 1, next index 2). Every output is
`{"ok": value}` or `{"error": message}`; error text is the baseline wording for
typed generator errors.

| Output | Provider call |
| --- | --- |
| `forms` | `render` → `serialize` per form; `.nets` is the allocator snapshot (absent when standalone) |
| `geometry` | `geometry(render(...))`: pads, courtyard, `courtyardFallback` |
| `normalized` | `normalizeDefinition` (definition-level cases only): pads, courtyard, keycap, terminals, envelope source and notice |
| `modelAssetIds`, `modelBindings` | Same-named provider functions |
| `export` | `exportErgogenForms`: `footprints` and `objects` text after arc upgrade, model-path rewrite and `module` → `footprint`; plus the `modelPaths` supplied and net snapshot |

Standalone cases have no part and no allocator (net index 0, as standalone
export does). Forms are serialized S-expressions with numbers as the generator
printed them, so numbers compare as exact text.

## Input matrix

| Family | Cases per generator | Inputs |
| --- | --- | --- |
| `default` | 2 | Definition defaults standalone; placed at origin |
| `boolean-flip` | one per boolean parameter | Each boolean flipped at definition level, with saved pad IDs and net IDs so index-matching is recorded |
| `side-rotation` | 10 | Part side front/back × `side` parameter F/B × rotation 0/90/37 at (12.5, -7.25); mismatched pairs; `side` unset |
| `marker-nets` | 3 (generators with net parameters) | Distinct net names, the reserved name `GND`, empty names. Terminal marker discovery is recorded in `normalized.terminals` |
| `parameter-merge` | 1 | Part parameters overriding definition parameters |
| `coercion` | 1 | Every number parameter supplied as its default in string form |
| `extra` | per generator | Utility routes, zones and text; validation errors; model path forms; keycap envelopes |
| `demo-definition`, `demo-part` | per demo | Deduplicated generator definitions and placed parts from every bundled demo |

Demo coverage is the starter, Sofle v2/RGB/Choc, the 15 measured keyboards,
the VIK module review and the committed REVIUNG41 archive. They were built by
the production builders and a native Core process, then deduplicated by
generator, parameters, side and parameter shape; the first instance and the
first rotated instance of each are kept.

## Known limits

- No generator declares an `anchor` parameter, so anchors are exercised only
  through the part pose (`p.point`, `p.at`, `p.xy`) in the rotation cases.
- The `utility_router` `error-bad-position` message is V8's JSON parse error,
  recorded under Node v26.10.0, and is not a stable wording to match.
- Demo documents depend on Core at the recorded commit; re-recording at any
  other revision would change them.

## Regenerating (until step 6)

```sh
cargo build --manifest-path core/Cargo.toml --example core_request
node --no-warnings --import ./footprints/tests/golden/harness/register.mjs \
  footprints/tests/golden/harness/record.mjs
node --no-warnings --import ./footprints/tests/golden/harness/register.mjs \
  footprints/tests/golden/harness/record_context.mjs
```

Node 24 or later strips the TypeScript types; no `pnpm install` is needed.
Output was verified identical across two consecutive runs.
