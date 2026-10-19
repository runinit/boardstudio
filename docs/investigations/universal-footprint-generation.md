# Universal footprint generation

Inspected revision: `f209af58b` (2026-10-18).
Status: code and contracts surveyed; proposed redesign only. No generator behavior
or saved-document format has changed.

## Executive summary

The current system already has a common pipeline for the *outputs* of built-in
generators, but not a common representation for their *inputs or behavior*. Each
of the 36 built-ins has a `GeneratorSpec`, a parameter list and a handwritten
Rust body that constructs KiCad S-expression text. The shared registry validates
parameters, supplies render context, parses that text, extracts geometry and
terminals, and supports preview/export. This is a sound provider seam, but it
does not make a new footprint work automatically: adding one still means writing
and registering a body and metadata.

There are two different meanings of “footprint goes in, generator works”:

1. **A static KiCad footprint goes in and works as a footprint.** This is already
   supported: Parts imports `.kicad_mod`, retains the source, projects geometry
   for preview, and exports the retained source. It is not a generator and should
   not be converted into one. This path has no per-footprint Rust exception.
2. **A new parametric footprint/generator goes in and becomes editable and
   regenerable.** This cannot be inferred from arbitrary KiCad source. The source
   does not say which dimensions, pad numbers, layers, labels, nets, or geometry
   should vary together. The engine needs an explicit program describing those
   relationships; arbitrary source inference is not the goal.

The requirement is stronger than “data-driven for ordinary footprints”: **all 36
built-in parametric generators must use one universal parametric engine**. A
per-footprint handwritten body is not an acceptable permanent escape hatch. Keep
the static import path distinct, but make every built-in's behavior data in a
shared declarative representation interpreted by the same engine. The engine may
have reusable domain operations (for patterns like pin banks, reversible
footprints, and route expansion); a footprint selects and configures those
operations, it does not implement them in its own Rust function.

This is achievable for this closed, known set: every existing body is finite Rust
code and can be represented by a bounded program/data language. It does **not**
mean inferring useful parameters from arbitrary geometry or executing arbitrary
external code. New generator behavior must use the same documented language and
operation library.

## Current flow and ownership

### Built-in parametric generators

`footprints/src/registry.rs` owns the common contract: source/version lookup,
typed parameters, defaults, net resolution, side/part pose, and body invocation.
Each entry in `footprints/src/generators/{ceoloide,infused_kim}` declares its own
`GeneratorSpec` and `body`. The body returns S-expression text; the shared
`sexpr` parser turns it into forms. `geometry.rs` extracts pads and a courtyard;
`definition.rs` normalizes pads, terminal mappings and envelopes; `models.rs`
handles model bindings; Core prepares preview/export jobs and calls the provider.

The existing abstraction therefore centralizes *plumbing*, not footprint
semantics. The per-footprint surface includes source ID, display name, kind,
parameter schema/defaults, optional matrix/keycap metadata, licensing, and a
function pointer. The function pointer is the irreducible custom behavior today.

### Imported and authored footprints

`core/src/artifact/source.rs` imports a KiCad footprint, keeps the original source
as authoritative, and derives preview geometry. Export patches/uses that retained
source. `compile_authored` handles authored geometry without a generator. Export
planning distinguishes generated, imported, and authored definitions. This
separation is valuable: source preservation avoids lossy round-trips and arbitrary
imported geometry does not need a synthetic parameter schema.

### Existing constraints that matter to redesign

- Generated pad count/order is identity-sensitive: normalization reuses saved pad
  IDs/nets by index. A template revision that reorders or removes pads can corrupt
  electrical identity without an explicit migration.
- Generator parameters are persisted and typed; Core validates edits and saved
  documents. The common schema is a public contract, not an internal convenience.
- Matrix terminals are discovered by rendering marker nets, so generated output
  must preserve net-to-pad behavior, not just visual geometry.
- Side, mirroring, pose, models, export and physical qualification all cross the
  generator seam. An output that renders is not automatically fabrication-ready.
- Goldens capture 820 cases across the built-ins; existing qualification tests
  and independent KiCad checks provide additional evidence. Preserve these before
  replacing implementations.

## Why per-footprint exceptions arise

The 36 bodies are not 36 instances of one obvious shape. Some are simple fixed
footprints with text/label settings; others change pad populations, generate
arrays, route or fill zones, draw logos, use local nets, select 3D models, or
encode domain-specific switch/controller wiring. A grep finds roughly 509
`if`/`match`/loop lines across the modules; that is a rough inventory, not a
complexity metric. The engine needs a small expression/template core plus a
finite library of reusable footprint operations:

| Behavior | Shared representation | Where differences live |
| --- | --- | --- |
| Fixed KiCad forms | Literal form-tree nodes | Data, never footprint-specific code |
| Parameter use | Typed expression nodes (literal, parameter, arithmetic, comparison, string/number formatting) | Add a generic expression primitive only when needed |
| Optional/repeated forms | `if` and bounded `map/repeat` nodes | Repeats have explicit finite bounds |
| Coordinate placement | Shared pose/side transform operations | No handwritten coordinate transform in a footprint definition |
| Nets and matrix terminals | Explicit parameter-to-pad bindings plus local-net operation | Topology rules are reusable engine operations |
| 3D models | Shared model node with declared defaults and side policy | Datum/default facts remain per-footprint data |
| Dynamic pad banks / switches | Reusable operations parameterized by pad tables, geometry and electrical mappings | Add an operation only when it has a stable reusable contract |
| Router, zones, logo paths | Reusable parsers/geometry emitters, fed by typed data | Do not embed a unique Rust body in one footprint |
| Kind, name, license, qualification | Data metadata | Never infer source facts from geometry |

Copying parameter tables is not a universal solution: similar names do not imply
the same defaults, pad order, electrical contract, or physical dimensions. Keep
those per-footprint facts as data. “No footprint-specific code” does not mean
“no footprint-specific data.”

## Proposed module design

Keep one public operation for the parametric path:

```text
render(generator reference, definition parameters, placed-part context)
    -> rendered footprint forms + diagnostics
```

Internally, replace the registered entry's body function with a `Program`:

```text
GeneratorSpec
  = metadata + parameter declarations + Program

Program
  = ordered list of operations over typed values and a footprint output builder
```

The program is a small, versioned declarative language. Its core is a typed
expression tree and ordered form construction: literals; parameter reads;
checked arithmetic/comparisons; conditional branches; bounded iteration over
arrays/counts; KiCad form nodes; validated string/number serialization; pose and
side transforms; net references; and model placement. Expressions return typed
values, not interpolated source fragments. The language is non-Turing-complete,
deterministic, bounded, and cannot call Rust, JavaScript, or the host.

Above this core, expose reusable domain operations for patterns that should not
be duplicated as giant trees: e.g. parametric pad arrays, mirrored/reversible
front/back construction, MCU pin rows and traces, switch/socket patterns, route
string parsing and segment emission, polygon/zone output, and model placement
defaults. These operations are part of the engine, not generator callbacks. Each
operation has typed inputs, validation, output semantics, and direct tests; a
generator's `Program` invokes/configures them with its own data. Keep operations
only where at least one real generator needs the semantics, and factor further
when multiple generators share them.

Represent large fixed pad/pin maps, logo paths, land patterns, names, defaults,
and model datum offsets as data tables. This is the right home for differences
between MX/Choc/KS27, connector variants, MCU pinouts and hardware dimensions.
Preserve their declared order exactly. New source generators are added by
providing metadata, parameter declarations, tables and a `Program`; they do not
add a per-footprint Rust function. No `Custom(function)` variant remains in the
target design.

The engine should produce a shared `FootprintOutput` (KiCad forms plus any
declared semantic metadata/diagnostics); a serializer emits text only at the
KiCad export edge. That removes the stringly-typed seam and makes programs
testable against forms. Translating bodies mechanically into another string
template would preserve the current weakness rather than deliver the redesign.

Static KiCad imports stay on the source-preserving path. Shared parsing and
projection utilities may be consolidated if an exact same-purpose duplication is
found, but imported source must remain authoritative and must not silently become
a generator.

## Coverage map and migration sequence

The inventory suggests the known set can fit one bounded engine, but requires
more than text substitution; migration and golden comparison must prove that:

| Family / generators | Engine capability required | Data that remains specific |
| --- | --- | --- |
| `mounting_hole_*`, `mounting_hole`, basic utilities (`text`, `point_debugger`, `icon_bat`) | Scalar expressions, optional nodes, standard graphics/text/forms | Defaults, geometry, labels, layers |
| `utility_logo` | Static vector path data, scale/side transform | Logo points and artwork metadata |
| `utility_filled_zone`, `utility_keepout_zone` | Validated polygon/zone node emission | User points, zone rules and flags |
| `utility_router` | Bounded route parser and trace/via emitter | Route strings and widths |
| `pads`, `smd_0805` | Bounded indexed pad-bank generation, layout/labels/net binding | Pad-count schema, per-pin labels and nets |
| `mcu_nice_nano`, `mcu_supermini_nrf52840` | Repeated pin rows, pin-label table, shared trace pattern/local-net logic | Pin tables, extra-pin row, model assets |
| MX, Choc, KS27/KS33 switches; reset/power switches; diode and LED | Reversible/front-back composition, conditional pad groups, switch/land-pattern/trace operations | Physical dimensions, land patterns, pad numbers and connection maps |
| Displays, connectors, encoder, trackpoint, Nice!View, TRRS | Standard forms plus repeat/mirror/side/model operations; reusable board/pin patterns | Board outlines, pinouts, mounting dimensions, model datum/defaults |

This map is a coverage checklist, not permission to leave an unlisted custom body.
Every current branch, loop, validation, coordinate formula, and emitted form must
be represented by core language nodes, reusable engine operations, or data.

1. **Lock the contracts.** Keep generator goldens, parameter lint, pad identity,
   terminal/net tests, model tests, and independent KiCad export validation as
   the acceptance baseline. Add a test that exercises a `Program` through the
   same registry, normalization, preview and export seam.
2. **Build the typed output and expression core.** Include parameter/type
   validation, expression error paths, safe serialization, deterministic limits,
   and exact-form tests. Avoid parsing generated strings back into forms.
3. **Add the reusable operation set from the coverage map.** For each operation,
   write its contract and tests, then prove it against real generators. An
   operation used by one generator is acceptable if it is a generic domain
   concept (e.g. logo path scaling), but it must not contain that generator's
   source ID or hardcoded data.
4. **Migrate all 36 entries in behavioral slices.** Compare form trees with
   existing goldens and exercise type errors, defaults, marker nets, front/back
   placement and model paths. Keep old implementations temporarily as
   differential oracles; delete them when each slice matches.
5. **Measure leverage before closure.** Count deleted handwritten behavior,
   declarative data, reusable operation complexity, interpreter size, and code
   exceptions. The target is no generator-specific executable logic and one
   behavior contract to maintain.
6. **Require zero per-footprint bodies at closure.** Inventory every
   `GeneratorSpec` and prove each is a `Program`; remove the body-function
   registration route so a new generator cannot bypass the engine.
7. **Only then decide whether to broaden inputs.** If importing non-KiCad
   libraries is needed, add a format adapter that produces retained source or the
   same normalized representation. That is separate from this redesign.

## Acceptance criteria for a useful redesign

- All 36 built-ins are data `Program`s executed by the same engine; no
  per-footprint Rust body/function callback remains.
- Adding a new parametric footprint requires metadata, parameter declarations
  and program data. New engine operations require a generic contract and tests;
  they cannot contain per-footprint identity checks.
- Existing 36-generator outputs, parameter behavior, pad order/identity,
  terminals, model bindings and exports remain equivalent unless a separately
  reviewed migration explicitly changes them.
- A static imported `.kicad_mod` remains source-preserving, editable as an
  imported definition, and exportable without being labeled parametric.
- Invalid inputs fail with useful diagnostics; unknown parameters do not silently
  gain new meaning; template expansion is bounded and cannot execute code.
- No claim of universal fabrication readiness follows from parsing/rendering.
  Provenance, licensing, electrical correctness, mechanical fit and output gates
  remain independent evidence.

## Decision and next implementation step

The requested scope is all built-in parametric generators using the shared
program engine. Static import is already largely present; external generator
ecosystems need a specific source format and trust/licensing contract. A
universal executor for arbitrary generator code is not recommended: it would
restore a runtime and trust boundary that the Rust cutover intentionally removed.

The implementation should begin with the typed output/expression core and a
representative simple footprint, then grow operations against the full coverage
map. The old bodies are useful differential references, not fallback
implementations to retain at completion.

No generator implementation was changed as part of this design update. The
first implementation slice is the typed output/expression core; generator
migration should follow only after its shape is accepted against the coverage
map and golden baseline. Main design risk is overbuilding a DSL that merely
recreates Rust as a harder-to-read language; mitigate it with the coverage map,
small typed primitives, operation-level tests, and measured deletion of bodies.
