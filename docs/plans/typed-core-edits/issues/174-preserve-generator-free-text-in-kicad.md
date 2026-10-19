# 174: Preserve quoted generator labels in valid KiCad exports

Status: ready-for-agent
Type: build
Priority: P1
Blocked by: None (can start immediately)
Related: [Input-boundary audit research](../../../research/audit-input-boundaries-2026-10-18.md)

## What to build

Users can enter ordinary quotes, backslashes and Unicode in supported generator
text/label fields and get the same text in a valid KiCad board. The preview and
manufacturing export must agree; requiring users to pre-escape their saved text
is not an acceptable fix.

The audit entered a label containing an inch mark. Packaged Core returned a
successful board export whose text was split into a shortened string and an
extra token. KiCad 10 accepted the plain-text control and rejected the quoted
label. Both text generators and active labels in the pad-row and SMD generators
reproduced the loss.

## Acceptance criteria

- [ ] First reproduce exact text loss and invalid KiCad export with the inch-mark
  case. Prove the corrected parsed text equals the user's original value.
- [ ] Quote/escape literal text once at emission using the existing serialized
  string representation. Inventory affected generator free-text interpolations,
  including active labels and any affected font/reference literals; apply the
  same correction without escaping atom selectors or route syntax as strings.
- [ ] Cover both text generators and active front/back/reversible labels in pad
  and SMD output. Native cases include quotes, backslashes, Unicode and supported
  whitespace; literal escape-looking text must not silently change meaning.
- [ ] Saved parameter values stay literal and compatible, with no double escaping
  through rendering, normalization, preview or export. Unrelated nets, model
  references, numeric geometry and ordinary generator golden cases stay intact.
- [ ] Fresh packaged Core exports containing the affected text pass a real KiCad
  load check, alongside a plain-text control. Assert exact emitted label values
  as well as parser acceptance so silently deleting the quote cannot pass.
- [ ] Run affected generator/Core export checks. Keep physical hardware
  qualification and non-text compatibility decisions unchanged; do not refresh
  broad goldens to hide unintended output changes.

## Comments

Created from the user's audit findings after primary-source research. Independent
of 175's PCB import identity correction. Use a narrow literal-escaping correction,
not a new generator language or wholesale rewrite of generator templates.

### Reverified after the footprint-engine merge (2026-10-19)

Merged `footprint-engine` at `2fe19cf42` into `dev`. The new `Body::Forms`
path currently ports only `ceoloide/mounting_hole_npth`; both text generators
and the affected pad/SMD generators still use `Body::Text`.

A temporary integration probe exercised `bundled().render` for
`ceoloide/utility_text` and `infused-kim/text`, asserting that the emitted
`gr_text` string equals the literal saved `text` parameter. Both plain controls
(`Board 12 inch`) passed; both inch-mark cases (`Board 12" inch`) failed with
actual `Str("Board 12")`. Command:
`cargo test -p boardstudio-footprints --locked --test ticket_174_probe`.
Log: `.evidence/174-after-footprint-engine.log` (local, gitignored).
The temporary probe was removed after diagnosis; no production fix was made.

174 remains needed. This is fresh generator-boundary evidence, not a fresh
packaged Core export or real KiCad load check. The new structured-output path
is available to the implementation; preserve the acceptance criteria above.

The merged footprint crate's full suite passed with
`cargo test -p boardstudio-footprints --locked`
(`.evidence/footprint-engine-merged-tests.log`). Separate Standards and Spec
reviews of `b4d788c7c...f57e3a4e1` reported no findings for the narrow merge;
neither treated this first structured-output port as a fix for 174.
