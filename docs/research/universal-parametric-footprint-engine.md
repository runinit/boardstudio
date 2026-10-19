# Universal parametric footprint engine: primary-source check

Date: 2026-10-19. Repo revision: `ce27ff126`. This note checks and strengthens
`docs/investigations/universal-footprint-generation.md` (the "draft"). It does not
replace it. No code was changed and cargo was not run.

## 1. The draft's claims against the repo

| Draft claim | Verdict | Source |
| --- | --- | --- |
| There are 36 built-ins, each with its own `body` fn | Confirmed. `SPECS` lists 24 ceoloide and 12 infused-kim entries | `footprints/src/generators/mod.rs:15-52` |
| The per-footprint seam is a function pointer that returns text | Confirmed: `pub type Body = fn(&RenderContext) -> Result<String>`, and `GeneratorSpec.body` | `footprints/src/registry.rs:16`, `registry.rs:33-45` |
| Text is parsed back into forms after the body runs | Confirmed: `let text = (spec.body)(&context)?; ... parse_forms(&text)` | `registry.rs:236-238` |
| Parameter typing and defaults are already shared | Confirmed: `ParamKind {Number, Boolean, String, Net, Array}` and the resolution loop | `footprints/src/params.rs:8-15`, `registry.rs:171-213` |
| Pad identity is reused by index | Confirmed | `footprints/src/definition.rs:80-101` |
| Terminals are found by rendering marker nets | Confirmed: `__boardstudio_terminal_<name>__` | `definition.rs:175-200` |
| 820 golden cases | Not confirmed in this pass. The goldens are one JSON file per generator (36 files) plus `manifest.json` with `totals`; the count was not summed | `footprints/tests/golden/generators/`, `footprints/tests/golden/manifest.json` |

Ergogen's own pipeline has the same split. Parameters are declared as data,
with types `string|number|boolean|array|object|net|anchor`. The footprint `body`
is arbitrary JS that returns a KiCad string
(`ergogen/ergogen` `src/pcbs.js` ~L93-119, https://github.com/ergogen/ergogen/blob/master/src/pcbs.js).
BoardStudio's `RenderContext` helpers (`xy/at/isxy/iaxy/esxy/eaxy/local_net`,
`footprints/src/context.rs:195-247`) port that API directly (`pcbs.js` L131-156:
`internal_xyfunc`, `external_xyfunc`, `local_net`). The local-net prefix is the
one difference: upstream uses `${component_ref}_${suffix}` (`pcbs.js` ~L153), and
BoardStudio uses a stable-ID-based name (`context.rs:237-246`). That is the
reason the draft wants nets to be an engine operation and not per-footprint
code.

Ergogen also has a data-level reversibility hook. A `params.mirror` object
overrides params on mirrored points (`pcbs.js` L50-55). The engine should keep
this kind of override as plain data.

## 2. Shared operations and the generators they cover

These counts come from grepping each generator file for `"reversible"`,
`local_net`, model params (`3dmodel`/`model_side`/`infused_model`), `points_text`
and top-level `for` loops (files under `footprints/src/generators/*/`).

| Operation (engine-owned) | Generators that need it | Evidence |
| --- | --- | --- |
| **Literal form tree + text/label nodes** | All 36 | every body emits `(footprint ...)`/`(fp_text ...)` text |
| **Side select + reversible duplication**: emit a group on F, B, or both, and drop single-side silk when reversible | ceoloide: battery ×2, diode, both displays, LED, both MCUs, power/reset switches ×3, encoder, Choc, KS27, MX, TRRS, logo, text (17) | `switch_mx.rs:680-729` is the canonical shape: `if reversible \|\| side=="F" {front}`, `if reversible \|\| side=="B" {back}`. Upstream `ceoloide/ergogen-footprints` `switch_mx.js` L22-33, 132 |
| **Second reversibility convention** (`reverse` + `mirror` + `swap_pad_direction`) | infused-kim: `smd_0805`, `pads`, `nice_view`, connectors, switches | `infused_kim/smd_0805.rs:19,272-279`. Upstream `infused-kim/kb_ergogen_fp` `smd_0805.js` L7-16, L70 |
| **Mirror-aware vs mirror-resistant placement** (`isxy/iaxy/esxy/eaxy`) | All placed generators | `context.rs:204-233`, mirroring `pcbs.js` L134-150 |
| **Pin-row bank**: table of pin names × rows, optionally inverted, with per-row local nets and jumpers | `mcu_nice_nano`, `mcu_supermini_nrf52840`, and in a reduced form `display_nice_view`, `display_ssd1306`, `infused_kim/nice_view` | `mcu_nice_nano.rs:105` (`PIN_NAMES: [[&str;2];12]`), `:199-207` (12-row loop), `:560-627` (socket rows), `:640` (`invert_pins`) |
| **Local (jumper) nets** | battery_jst, both displays, both MCUs, infused nice_view (6) | `local_net` grep counts; `context.rs:237` |
| **Model node with side/offset defaults** | 26 of 36 | `util.rs:79-133` (`model_side`, `infused_model`). The infused-kim family divides offsets by 25.4 (`util.rs:121`), so the unit convention must be data |
| **Indexed pad bank** (N pads, layout, per-pad label/net) | `infused_kim/pads`, `smd_0805` | `pads.rs`, `smd_0805.rs:235-279`; upstream ergogen `src/footprints/pad.js` `layout(toggle, side)` |
| **Point-list polygon/zone** | `utility_filled_zone`, `utility_keepout_zone` | `util.rs:9-27` (`points_text`, already shared) |
| **Route-string parser → segments/vias** | `utility_router` | `utility_router.rs:55-95` (`parse_position`, `traces`) |
| **Static vector art + scale** | `utility_logo`, `icon_bat` | file sizes 146/88 lines, no loops |
| **Hotswap/solder switch variants + reversible route stitching** | MX, Choc, KS27 | `switch_mx.rs:723-729` (`hotswap_routes_same_side` / `_unplated`) |

The table supports the draft's coverage map. One correction: the draft does not
mention that **two incompatible reversibility vocabularies** exist (ceoloide
`reversible` vs infused-kim `reverse/mirror/swap_pad_direction`). Both parameter
sets are persisted, so the engine needs one side-composition operation that each
generator configures with its own parameter names. A shared param name would
break saved documents.

## 3. Prior art

### KiCad footprint format

The official spec gives the header tokens `footprint "NAME"`, `version` and
`generator`. It defers everything else to the common board definitions
(https://dev-docs.kicad.org/en/file-formats/sexpr-footprint/ and
https://dev-docs.kicad.org/en/file-formats/sexpr-intro/index.html#_footprint).
A typed form-builder output is the right target: the format is a tree, and
string concatenation is only an implementation accident here
(`registry.rs:236-238`).

### kicad-footprint-generator (KiCad Library Team)

https://gitlab.com/kicad/libraries/kicad-footprint-generator (default branch `main`)

- **Data and code are split, and the code is per family, not per footprint.**
  `data/<family>/*.yaml` hold the dimensions (e.g. `data/package/gullwing/soic.yaml`).
  Each family has one script (`src/generators/package/gullwing/footprint.py`,
  `spec.py`). Hundreds of SOIC/SOP variants are YAML entries with
  `nominal`/`tolerance` dimensions (`soic.yaml` L42-66) and a shared
  `default_parameters` block (`soic.yaml` L34-36).
- **Escape hatches are named data keys, not code.** `footprint.py` ~L200-219
  reads `pad_size_y_overwrite`, `pad_to_pad_min_x_overwrite` and others from the
  YAML to override computed values.
- **IPC-7351 is a shared calculator.** `ipc_gull_wing(...)` in
  `src/generators/tools/footprint/ipc_pad_size_calculators.py` (L102-138)
  computes `Zmax = Lmin + 2JT + sqrt(CL^2+F^2+P^2)` and rounds it to a base. The
  family script picks an IPC class by pitch (`footprint.py` L178-189: `ipc_spec_gw_small_pitch`
  / `_large_pitch`, density from config) and takes courtyard offsets from it (L390).
- **Output is a tree library** (`KicadModTree`: `Footprint`, `PadArray`,
  `ExposedPad`; `footprint.py` L29-31), not text templates.

Lesson for BoardStudio: the KiCad team did not reach "data only". They reached
**a small set of family operations + data + named overrides**, and that matches
the draft's "reusable domain operations" layer. Keyboard footprints take
datasheet dimensions as given, so IPC derivation does not apply to the current
36 generators. It would only matter if an SMD-package family operation is added
later.

### Horizon EDA

Horizon packages carry a `parameter_set` and a `parameter_program`
(`horizon-eda/horizon` `src/pool/package.hpp` L109-111). The program is a tiny
stack language: tokens `INT|CMD|STR|UUID` (`src/parameter/program.hpp` L34-36),
commands `+ - * / dup dupc swap +xy -xy chs dump get-parameter`
(`src/parameter/program.cpp` L131-162), plus domain commands such as
`set-polygon` (`src/parameter/program_polygon.cpp`). It is a working example of
the draft's "non-Turing-complete, bounded language" with a small domain-command
set. It only adjusts geometry that already exists. It has no conditionals or
repetition, so it cannot express side selection or pin banks. BoardStudio needs
`if` and bounded `repeat` on top of that model.

### atopile

Not checked (only the repo root listing was fetched). No claim is made.

## 4. Where data-only falls short

1. **Conditional composition over many booleans.** For example, MX has 41 params
   (`switch_mx.rs:15`) and its emission is gated by combinations of these
   (`switch_mx.rs:680-729`). This needs `if` nodes over boolean expressions.
   Tables alone cannot express it.
2. **Pin-label truth tables.** Nice!Nano silk visibility depends on
   `reversible`, `invert_pins`, `extra`, `only_required` and the side
   (`mcu_nice_nano.rs:414-509`). Encode it as one predicate expression per label
   group, not as precomputed tables. Otherwise the table grows with every flag.
3. **Parsing user strings** (`utility_router.rs:55`) is engine code. The
   footprint data only names the parameter.
4. **Legacy numeric quirks.** Output uses `js_number` (`footprints/src/number.rs`)
   and the infused model offsets divide by 25.4 (`util.rs:121`). Golden equality
   depends on reproducing these, so they must be engine serialization rules,
   selectable by data.
5. **Pad order is identity** (`definition.rs:80-101`). A declarative rewrite must
   keep the emission order of pads exactly, not only the same set.

## 5. Recommendations

1. Keep the draft's direction: `Program` replaces `Body` (`registry.rs:16,44`).
   Plain data alone is not enough (see §4).
2. Make the operation set match the evidence in §2: `side_compose` (configurable
   param names, which covers both vocabularies), `place` (mirror-aware/resistant),
   `pin_rows` (table + invert + local nets), `pad_bank`, `polygon` (already
   `points_text`), `route_string`, `model` (with unit convention), `text`. That is
   about eight operations. Follow KiCad's family-script granularity, not one
   operation per footprint.
3. Copy KiCad's **named-override** pattern (`*_overwrite`) for per-footprint
   deviations. Do not add a code hook.
4. Copy Ergogen's `params.mirror` override object (`pcbs.js` L50-55) as data
   instead of mirror branches inside programs.
5. Emit `Expr` trees directly (KicadModTree precedent) and drop
   `parse_forms(text)` from `render`. Keep golden comparison at the form level
   (`footprints/tests/golden_forms.rs`).
6. Migrate in the order of operation reuse: utilities → pad banks → side_compose
   switches → pin_rows MCUs/displays. Run the old `body` as a differential oracle
   until each golden matches. This agrees with the draft's §migration.
