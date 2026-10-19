# atopile vs kicad-footprint-generator as a model for one footprint engine

Date: 2026-10-19. BoardStudio revision: `a516109d7`. This note follows on from
`docs/research/universal-parametric-footprint-engine.md` and
`docs/investigations/universal-footprint-generation.md`. It does not change
code, and cargo was not run. Both external repos were shallow-cloned and read,
and none of their code was executed.

Pinned sources (all line references below are at these commits):

- **KFG** = kicad-footprint-generator `a903a695bf4e69905e9e56548f44dd46cdc1130e` (2026-10-10).
  Base URL: `https://gitlab.com/kicad/libraries/kicad-footprint-generator/-/blob/a903a695bf4e69905e9e56548f44dd46cdc1130e/`
- **ATO** = atopile `619eda7f777558a3e500dbad9cc2941712881495` (2026-03-11, `main` HEAD at clone time).
  Base URL: `https://github.com/atopile/atopile/blob/619eda7f777558a3e500dbad9cc2941712881495/`

Paths below are relative to those bases.

## 0. Answer in brief

- **atopile has no parametric footprint generator.** It gets footprints in two
  ways. It imports them from JLC/LCSC through the third-party `easyeda2kicad`,
  or it references existing `.kicad_mod` files. It then works on them in a typed
  KiCad s-expression model (Zig, exposed to Python). That model is useful prior
  art for a typed KiCad tree. It is not prior art for a parameter engine.
- **KFG is the only real parametric-engine precedent of the two.** It uses YAML
  data, one script per family, a shared node tree (`KicadModTree`) and a single
  serializer. Two of its design choices conflict with our contracts: the
  serializer re-sorts pads, and it formats floats with its own rule.
- **Recommendation:** borrow ideas only, from both. Write the engine in Rust so
  that it emits our existing `sexpr::Expr` tree directly. Neither project should
  become a dependency or be ported (§5).

## 1. Repo facts settled

**820 golden cases: confirmed.** `footprints/tests/golden/manifest.json` has
`totals.cases = 820` across `generators = 36`. The per-generator `cases` fields
in the manifest also sum to 820. Counting the case entries in the 36 files under
`footprints/tests/golden/generators/*.json` gives 820 too. The totals also list
`errors: 99`, by family `side-rotation 360, boolean-flip 151, marker-nets 75,
default 72, demo-part 62, extra 43, coercion 23, parameter-merge 22,
demo-definition 12`. The goldens compare exact serialized text for each form:
`footprints/tests/golden_forms.rs:63-73` serializes the recorded `Expr`s and
asserts string equality. So form order, atom spelling and quoting are all part
of the contract.

**Our tree types that the engine would emit into:**

| Item | Location | Role |
| --- | --- | --- |
| `enum Expr { Atom(String), Str(String), List(Vec<Expr>) }` | `footprints/src/sexpr.rs:6-11` | The KiCad form tree. It is untyped (no per-token schema). `Atom` keeps the exact number text, and atom vs quoted string is preserved |
| `Expr::atom`, `Expr::string` | `sexpr.rs:14-20` | Constructors |
| `child`, `children` | `sexpr.rs:48-54` | Lookups that downstream geometry/definition code uses |
| `parse_forms` | `sexpr.rs:163` | Text → `Vec<Expr>`. This is the reparse step we want to remove from rendering |
| `serialize` | `sexpr.rs:203-212` | `Expr` → text (JSON quoting for strings) |
| `type Body = fn(&RenderContext) -> Result<String>` | `footprints/src/registry.rs:16` | The per-footprint text seam to replace |
| `render(...) -> Result<Vec<Expr>>` | `registry.rs:157-163` | Public output is already `Vec<Expr>` |
| `let text = (spec.body)(&context)?; … parse_forms(&text)` | `registry.rs:236-240` | The render-then-reparse step |
| `export_forms(forms: Vec<Expr>, …)` | `footprints/src/export.rs:67` | Export already takes forms |
| `js_number`, `js_to_fixed` | `footprints/src/number.rs:9`, `:58` | The number-to-text rules the goldens depend on |

"Build the KiCad form tree directly" means this concretely: the engine's output
builder returns `Vec<Expr>` and creates number atoms with
`Expr::Atom(js_number(v))` (or `js_to_fixed` where a body uses it now). The
`Program` type then replaces `Body` at `registry.rs:16`, and lines 236-240 become
`program.run(&context)` with no `parse_forms`. Nothing downstream of `render`
changes, because it already consumes `Vec<Expr>`. `parse_forms` stays in use for
imported `.kicad_mod` and for the golden tests. A new typed per-token model
(like KicadModTree's or atopile's) is **not** required, because `Expr` already
holds everything the goldens check. Adding one would put a second
formatting/ordering authority in front of the goldens.

## 2. KFG: how it works

- **Data + family script + tree.** There are 211 YAML files under `data/` and
  539 Python files under `src/generators/` (`find` counts). For example,
  `data/package/gullwing/soic.yaml` is consumed by
  `src/generators/package/gullwing/footprint.py`, which builds a `Footprint`
  tree from `KicadModTree` (`footprint.py` L29-31).
- **Tree.** `KicadModTree/nodes/Node.py` L282 (`class Node`) has
  `translate/rotate` (L386-441) and `flatten` (L457). The base nodes are in
  `KicadModTree/nodes/base/` (`Pad`, `Line`, `Arc`, `Model`, `Zone`, `Text`,
  `Group`, …). Specialized nodes (`PadArray`, `ExposedPad`, `Translation`,
  `Rotation`, `ChamferedPadGrid`, …) are in `KicadModTree/nodes/specialized/`.
  `Translation`/`Rotation` are container nodes that transform their children
  when the tree is flattened (`Translation.py` L25-77).
- **Serialization.** `KicadModTree/KicadFileHandler.py` L119-139 flattens the
  tree and then **sorts every non-property node into "KiCad native order"**
  (`other_nodes.sort(key=…)`), and L141-218 writes the header and the nodes.
- **Pad order is not preserved.** The pad sort key is (pad number with a natural
  sort, x, y, size, shape) (`KicadModTree/serializer.py` L296-340, "Approximate
  sorting order from FOOTPRINT::cmp_pads"). The order in which pads were emitted
  is discarded. Pad identity in KiCad libraries is the **pad number**, not
  position in the file.
- **Number formatting.** `serializer.py` L485-498: `("%f" % n).rstrip("0").rstrip(".")`
  rounds to 6 decimal places and maps `-0` → `0`. This is a different rule from
  our JS `String(n)` (`number.rs:9`).
- **Expressions in data.** `$(...)` strings in YAML are evaluated with `asteval`,
  a restricted Python interpreter
  (`src/generators/tools/footprint/declarative_def_tools/ast_evaluator.py` L20, L45-66).
  It is used per variant at `footprint.py` L173-174.
- **Overrides / exceptions.** Named `*_overwrite` keys
  (`footprint.py` L191-219). The typed pad override record is `pad_overrides`
  (`declarative_def_tools/pad_overrides.py`: `move`, `at`, `size_increase`, …).
  YAML inheritance uses `dict_inherit` (`src/kilibs/util/dict_tools.py` L49).
- **Arrays.** `PadArray(pincount, spacing, initial, increment, deleted_pins,
  hidden_pins, pad_overrides …)` (`KicadModTree/nodes/specialized/PadArray.py`
  L45-153, override application L382).
- **Conditionals.** These are plain Python `if` in the family scripts, e.g. the
  IPC class chosen by pitch at `footprint.py` L176-182 and the EP branches at
  L191 and L290-330. They are not data.
- **3D models.** `Footprint.add_standard_3d_model_to_footprint(lib, name)` uses a
  global path convention (`KicadModTree/nodes/Footprint.py` L250-267). The
  `Model` node serializes offset/scale/rotate (`serializer.py` L1305-1321).
- **Front/back, reversible.** There is no reversible concept: `grep -i reversible`
  over `KicadModTree/` and `src/generators/` finds nothing. Library footprints
  are front-side (`KicadFileHandler.py` L149 always writes `(layer "F.Cu")`), and
  flipping is left to KiCad at placement. `Pad` has only an `x_mirror/y_mirror`
  constructor option (`nodes/base/Pad.py` L332-333, L524).
- **License.** Mixed. The repo default is GPL-3.0-or-later, and new
  contributions must be GPL-3.0-or-later. Some files are MIT or LGPL-2+ according
  to their headers (`README.md` L31-47, `LICENSE.GPL`, `LICENSE.LGPL`,
  `LICENSE.MIT`). `KicadModTree/nodes/Node.py` L1-14 has a GPL-3 header.

## 3. atopile: how it works

- **Where footprints come from:**
  1. JLC/LCSC parts through the third-party `easyeda2kicad` package
     (`pyproject.toml` L30 `atopile-easyeda2kicad>=0.9.7`). In
     `src/faebryk/libs/picker/lcsc.py` L20-27 (imports) and L216-233
     (`EasyEDAFootprint.from_api`), easyeda2kicad's exporter **writes v5
     footprint text to a file**. atopile then parses it with
     `kicad.loads(kicad.footprint_v5.FootprintFile, …)` and upgrades it with
     `kicad.convert` (`src/faebryk/libs/kicad/fileformats.py` L333-360).
     So atopile also uses the text-then-reparse pattern, at the import boundary.
  2. An existing KiCad library footprint file: trait
     `has_associated_kicad_library_footprint`
     (`src/faebryk/library/KiCadFootprints.py` L136-282).
- **No parametric generation.** The `package = "0402"` field is a **picker
  constraint**, not a geometry generator. It maps to the `has_package_requirements`
  trait over the `SMDSize` enum (`src/atopile/compiler/overrides.py` L44-112,
  `src/faebryk/libs/smd.py`). A search for footprint-generator code
  (`grep -rlni "footprint.*generat"`) finds only the transformer and parts search.
  `kicad.pcb.Pad(...)` is constructed only to copy library pads into a board
  (`src/faebryk/exporters/pcb/kicad/transformer.py` L1509-1550,
  `src/atopile/server/domains/parts_search.py` L370-395).
- **Typed KiCad model.** This is written in Zig and exposed to Python as
  `pyzig_sexp`. Raw parsing is in
  `src/faebryk/core/zig/src/sexp/{tokenizer,ast,structure}.zig`, and the typed
  structs are in `src/faebryk/core/zig/src/sexp/kicad/{footprint,pcb}.zig`.
  `footprint.Footprint` (`kicad/footprint.zig` L13-59) holds **separate lists
  for each kind**: `fp_lines`, `fp_arcs`, `fp_texts`, `pads`, `models`, …
  (multidict fields). The encoder writes fields in a compile-time sorted field
  order (`structure.zig` L1028-1069, L1484-1514), so lines, texts and pads that
  were interleaved in the source come out grouped by kind.
- **Number formatting.** `round(v * 10e6) / 10e6` then `{d}`
  (`structure.zig` L996-1006 and L1436-1446). The comment says "6 decimal
  places", but `10e6` is 1e7, so in practice it rounds to 7 decimals. There are
  special cases for three tokens that need fixed `.6` formatting.
- **Pad/net identity.** A pad is matched **by name/number**. Pad names come from
  copper pads (`KiCadFootprints.py` L190-202). Binding builds
  `pcb_pad_by_name` and matches each logical pad by `pad_name` or `pad_number`
  (`transformer.py` L340-360). Nets are then written onto every PCB pad with
  that name (`transformer.py` L2150-2175). Footprints on the board are
  identified by an `atopile_address` property (`transformer.py` L310-315).
- **Layout reuse.** `src/faebryk/exporters/pcb/layout/layout_sync.py` keys
  footprints by `atopile_address` (L97-99). It maps nets between a sub-layout PCB
  and the top PCB by counting matched pads (`_generate_net_map`, L157+), and
  copies footprints, routes and other items with an offset (L231-345).
- **Front/back.** This happens at placement, not in the footprint. It mirrors
  coordinates and swaps `F.*`↔`B.*` (`transformer.py` L1069-1071, L1599-1654,
  L1721+). Footprints themselves are not reversible.
- **3D models.** These are taken from EasyEDA, with an offset fix-up
  (`lcsc.py` L173-192, L375-386, L459) and a `.wrl` suffix workaround (L228-231).
- **License.** MIT (`LICENSE` L1-3).

## 4. Side-by-side

| Concern | BoardStudio today | KFG | atopile |
| --- | --- | --- | --- |
| Parametric generation | 36 Rust bodies returning text (`registry.rs:16`) | Yes: YAML + family script + `KicadModTree` | **None.** It imports EasyEDA footprints or references `.kicad_mod` |
| Output representation | Text → `parse_forms` → `Vec<Expr>` (`registry.rs:236-240`) | Typed Python node tree → one serializer | Typed Zig structs. Import goes text (easyeda2kicad) → parse |
| Emission order | Body order, compared exactly by goldens | **Re-sorted** into KiCad native order (`KicadFileHandler.py` L131-134) | **Grouped by kind** (multidict field order, `structure.zig` L1066+) |
| Pad identity | **Index in emission order** (`definition.rs:80-101`) | Pad number. Sort is by number, then position (`serializer.py` L326-340) | Pad name/number (`transformer.py` L351-360) |
| Number formatting | JS `String(n)` (`number.rs:9`), exact text in goldens | `%f`, strip zeros, 6 dp, `-0`→`0` (`serializer.py` L485-498) | Round to 1e-7, `{d}` (`structure.zig` L996-1006) |
| Reversible / front-back | Emitted inside the footprint. Two vocabularies (see the earlier note §2) | Not supported. Footprints are always `F.Cu` | Flipped at board placement, not in the footprint |
| Conditional content (boolean combos) | Rust `if` per body | Python `if` per family script, plus `$(…)` asteval expressions in YAML | n/a |
| Arrays / repeats | Rust loops | `PadArray` (count, spacing, increment, deleted/hidden pins, overrides) | n/a |
| Exceptions / overrides | Code | Named `*_overwrite` keys, `pad_overrides`, `dict_inherit` | n/a (a per-part offset fix-up for imports) |
| 3D model refs | `util.rs:79-133` (`model_side`, `infused_model`, the /25.4 quirk) | `add_standard_3d_model_to_footprint` (global path convention) | Taken from EasyEDA plus fix-ups |
| Nets in footprints | Marker nets, `local_net` | None (library footprints have no nets) | Assigned to board pads by name |
| Layout reuse | n/a | n/a | `layout_sync.py` via `atopile_address` |
| Language / runtime | Rust → native + WASM | Python 3 + asteval | Python + Zig extension module |
| License | — | GPL-3.0-or-later default, some MIT/LGPL files | MIT |
| Use for us | — | **Ideas**: family operations, `PadArray` parameters, named overrides, inheritance | **Ideas**: one typed KiCad model shared by import and export; identity by stable address |

## 5. Feasibility for our Rust/WASM stack

- **Depend: no for both.** KFG is Python and needs `asteval`. atopile's core
  needs a native Zig/Python extension. Neither can run in our WASM target
  without a Python runtime, and the trust boundary that the Rust cutover removed
  would come back (draft "Decision"). KFG's GPL-3 default would also bind
  anything linked to it.
- **Port: no.** Porting KicadModTree would bring in two behaviors that break our
  goldens and identity: the native-order re-sort, which changes pad order and so
  breaks index identity in `definition.rs:80-101`, and the `%f` float rule. We
  would have to disable both. What is left is a node tree that is less precise
  than `Expr` for our purpose. atopile's model groups forms by kind, which would
  break exact form-order goldens for the same reason.
- **Reuse ideas: yes.** All of these are written fresh in Rust, so there is no
  license impact:
  1. Family operations with array parameters shaped like KFG's `PadArray`
     (`pincount`, `spacing`, `initial`, `increment`, `deleted_pins`, per-pad
     overrides). This fits our `pad_bank` and `pin_rows` operations.
  2. Named override keys and typed per-pad overrides as data, not code hooks
     (`*_overwrite`, `pad_overrides`). This confirms recommendation 3 in the
     earlier note.
  3. A bounded expression language in data. KFG shows the need (`$(…)`), but it
     uses an embedded Python interpreter. We should keep our planned typed,
     non-Turing-complete expression nodes.
  4. From atopile: a single tree type for both import and generated output. We
     already have this (`Expr` serves `parse_forms` for imports and `render`
     output). Flipping at placement time is also not our model: our built-ins
     are reversible inside the footprint, so the engine needs `side_compose`.

## 6. Recommendation

Follow **KFG's architecture** (data + a small set of family operations + named
overrides + one tree builder). Do **not** adopt its serializer semantics. Keep
**our own `Expr`** as the tree, so the Rust engine emits `Vec<Expr>` directly:

1. Replace `Body` (`registry.rs:16`) with `Program`, and replace the body call
   plus `parse_forms` (`registry.rs:236-240`) with a direct `Vec<Expr>` build.
2. Make number formatting an output-builder rule using `js_number`/`js_to_fixed`
   (`number.rs:9,58`) when atoms are created. Never re-format at serialize time.
3. Make emission order the program order. **No sorting.** Pad index identity
   (`definition.rs:80-101`) and exact-text goldens (`golden_forms.rs:63-73`)
   both depend on it.
4. Treat atopile only as confirmation of two things: a typed KiCad tree is the
   right boundary, and name-based pad identity is the common industry choice.
   Changing our index-based pad identity would be a separate, reviewed
   document migration and is out of scope here.

Gate: the 820 golden cases (36 generators) must still pass byte-for-byte for
every form after each generator migrates.

## Not checked

- KFG's `KicadModTree` unit-test expectations and generator CI outputs.
- Whether atopile's decoder keeps unknown tokens on round-trip.
- easyeda2kicad's own source (it is a separate repository).
- Whether atopile's documentation mentions footprint generation that is planned
  but not yet in code. Only source at the pinned commit was read.
