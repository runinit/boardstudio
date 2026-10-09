---
status: accepted
---

# Core checks generator parameter types and names

Generator parameters are declared with a kind (number, boolean, text, net or list), and
render refuses a value of the wrong kind. Until now Core did not check them: a wrong type
surfaced only when a footprint rendered, and a name no generator declares was silently
ignored. [ADR-0004](0004-document-format-version-and-id-rename.md) migrates saved numeric
text to numbers, and what it leaves behind (unconvertible values, names no generator
declares, non-empty list text) is for Core to report.

Core now runs one check, built on the declared kind, over every saved parameter map: a
definition's own parameters, a part's own parameters and an assembly member's parameters.
Each map is checked where it is saved, so a value is judged once. A `null` value is absent,
as it is at render.

| Boundary | Wrong type | Name no generator declares | Non-empty list text for a list |
| --- | --- | --- | --- |
| Open | refused | located warning | located warning |
| ReplaceDocument | refused | located warning | located warning |
| Edit (commit and preview) | refused | refused | refused |

On an edit, Core judges only the values the edit introduces or changes. A legacy warning
therefore does not block an unrelated edit, and an edit that writes a value over a legacy
one is judged on the new value. Open and ReplaceDocument judge the whole document, so a
wrong type cannot reach a scene.

Warnings are findings from `validate` with scope Pcb and an ID of the form
`generator-parameter:<owner>:<name>:<kind>`, targeting the definition, the part or the
assembly and member. Wrong types are errors. The check uses the same `ParamKind::accepts`
as render, so a value Core accepts also renders.

## Why these choices

- **Open warns on unknown names.** Saved and third-party documents can hold names that no
  generator declares. Refusing the whole project would lose the user's work for a value
  render already ignores.
- **Open warns on non-empty list text.** Version 4 keeps `"[1,2,3]"` as text, because the
  older steps did, so saved data may hold it. A list parameter then fails at render until the
  user replaces it. The inventory found none in the shipped data (see
  [ticket 04](../plans/generator-parameters/issues/04-validate-generator-parameters-in-core.md)),
  but that does not show older saves are free of it, so it warns rather than refuses.
- **Edits refuse everything they write.** A new bad value should not enter the document; the
  refusal names the generator and the parameter.

## Consequences

- A wrong type that is not a number or a list (for example `"two"` for `hole_size`) is a
  refusal on Open. Migration keeps such values, so an old project holding one will not open
  until it is repaired. The shipped projects hold none.
- Copying a part that carries a legacy undeclared name is refused, because the copy writes the
  name. The user can remove the name first.
- The Parts editor coerces form text before it submits (`web/crates/parts/src/parts/generator_settings.rs`).
  Its output is tested against this check; text that does not parse is refused in the form
  and never reaches Core.
