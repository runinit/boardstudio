# 003: Give Keymap complete workspace ownership

Status: ready-for-agent
Type: build
Blocked by: [002: Production composition test seam](002-production-composition-test-seam.md)
Parent: [001: Editor workspace ownership specification](001-editor-workspace-ownership.md)
Related: [Plan and verification contract](../map.md), [ownership research](../../../research/editor-workspace-extraction.md)

## What to build

Keep layer editing, key and encoder bindings, macros and firmware export working as
before while making Keymap own its complete page-shell orchestration. This is the
first bounded workspace slice and the interface/lifetime pilot for later extractions.

## Scope and constraints

- Move source/projection assembly, layer/encoder/binding/macro controller hooks,
  admitted selection/layer/export actions and construction of navigation, toolbar,
  canvas and Inspector inputs into the existing Keymap workspace module.
- Keep hooks unconditional in Editor scope and preserve lower-crate provider ancestry,
  mounted editor keys, binding lifetimes and existing async execution primitives.
- Consume the existing shared PCB resolution signal. Expose only genuinely shared
  results, such as view/contours needed by camera coordination, not individual
  controllers for Editor to reconstruct the former Inspector.
- Keep shared instance selection, canvas/view settings and firmware ownership rules
  unchanged. No universal workspace interface or mega-context.

## Acceptance criteria

- [ ] Editor no longer constructs Keymap controller internals or the workspace's large Inspector/navigation inputs. The private interface hides meaningful wiring rather than merely moving markup.
- [ ] Real mounted layer creation/rename/selection, key binding, encoder binding and macro edits produce the same accepted values, pending/error behavior and Undo boundaries. Use an eligible encoder/PCB-plan fixture, not compile-only encoder coverage.
- [ ] Firmware-export readiness and action admission use the current board and original captured-source checks; stale or incomplete inputs cannot trigger an export.
- [ ] Gate a field edit, navigate away and back through production controls, then deliver success and failure. Session work settles once, retired observations do not revive and a newer mounted draft/binding cannot receive an old result.
- [ ] A board or replacement-project transition rejects stale Keymap controls even when object IDs are reused; selection and scope changes come from production reconciliation.
- [ ] Shared camera projections remain available, and unrelated accepted changes update clean fields without overwriting dirty drafts.
- [ ] Keymap controllers remain Editor-lifetime while visible editor children retain their existing conditional lifetimes; no new provider or per-workspace subscriber is introduced.
- [ ] The resulting private interface and any shared outputs are documented in the ticket outcome so later slices can consume them without depending on Keymap's controller details.

## Verification

Run documented quick checks, page-only WASM compilation, the page-bin browser suite
including new composition cases, and the existing Keymap mounted/queued-edit tests.
Retain lower-level controller coverage; do not duplicate every field at App level.
Apply the plan's shared verification and lifetime contract.
