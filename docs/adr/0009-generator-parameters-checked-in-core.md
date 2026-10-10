---
status: accepted
---

# Core checks generator parameter types and names

Generator parameters are declared with a kind (number, boolean, text, net or list), and
render refuses a value of the wrong kind. Until now Core did not check them: a wrong type
surfaced only when a footprint rendered, and a name no generator declares was silently
ignored. [ADR-0004](0004-document-format-version-and-id-rename.md) migrates saved numeric
text to numbers, and what it leaves behind (unconvertible values, names no generator
declares, text for a number or a list) is for Core to report.

Core now runs one check, built on the declared kind, over every saved parameter map: a
definition's own parameters, a part's own parameters and an assembly member's parameters.
Each map is checked where it is saved, so a value is judged once. A `null` value is absent,
as it is at render.

| Boundary | Other wrong type | Name no generator declares | Text where a number or a list is declared |
| --- | --- | --- | --- |
| Open | refused | located warning | located warning |
| ReplaceDocument | refused | located warning | located warning |
| Edit (commit and preview) | refused | refused | refused |

On an edit, Core judges only the values the edit introduces or changes. A legacy warning
therefore does not block an unrelated edit. A name that no generator declares, or text for a
number or a list, that the document already holds for the same generator (the name, and for
text the value) is not introduced by copying it to another part, because the Parts editor
keeps names it does not show and the user could not remove the name first. An edit that
writes a different new name or value is refused. Open and ReplaceDocument judge the whole
document, so a wrong type other than that text cannot reach a scene. ReplaceDocument must
accept everything Open accepts, because the page replaces the whole document for many
settings.

Warnings are findings from `validate` with scope Pcb and an ID of the form
`generator-parameter:<owner>:<name>:<kind>`, targeting the definition, the part or the
assembly and member. Any other wrong type is refused before a scene exists, so it never
becomes a finding. The check uses the same `ParamKind::accepts`
as render, so a value Core accepts also renders.

## Why these choices

- **Open warns on unknown names.** Saved and third-party documents can hold names that no
  generator declares. Refusing the whole project would lose the user's work for a value
  render already ignores.
- **Open warns on text for a number or a list.** Version 4 keeps number text it cannot convert
  (`"two"`) and non-empty list text (`"[1,2,3]"`) as text, because the older steps did, so
  saved data may hold either. The part then fails at render until the user replaces the value.
  Refusing Open would leave a project that cannot be opened to repair it. The inventory found
  none in the shipped data (see
  [ticket 04](../plans/generator-parameters/issues/04-validate-generator-parameters-in-core.md)),
  but that does not show older saves are free of it, so it warns rather than refuses.
- **Edits refuse everything they write.** A new bad value should not enter the document; the
  refusal names the generator and the parameter.

## Consequences

- Any other wrong type (a boolean where a number is declared, a number where text is) is a
  refusal on Open. Migration does not produce one, and the shipped projects hold none.
- Changing a matrix key to a different generator drops the saved parameters the new generator
  rejects (the old generator's terminal nets, for example). Without this, the edit would be
  refused for values render already ignored.
- The Parts editor coerces form text before it submits (`web/crates/parts/src/parts/generator_settings.rs`).
  Its output is tested against this check; text that does not parse is refused in the form
  and never reaches Core.
- The check walks the saved parameter maps on each edit and preview, before and after. On the
  demo projects it takes 0.01 to 0.03 ms (release build), against 3 to 7 ms for a move.
