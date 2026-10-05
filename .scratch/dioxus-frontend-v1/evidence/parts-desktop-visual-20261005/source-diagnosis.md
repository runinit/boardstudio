# Bundled MX initial 2D preview: source diagnosis

Read-only diagnosis, 2026-10-05. Public reproducer and actual DOM are retained in `RECEIPT.md`, paired light/dark screenshots, and `layer-side-observations.json`. Candidate is published38641dd1; reference is pinned5a472a9. No test, compiler, product source or browser internal-state execution was performed for this note.

## Supported cause

The selected generator has no explicit saved side at initial selection, but the form displays the schema default B. Dioxus previews the accepted catalogue geometry until an actual generator input event. The reference instead prepares a complete transient default-expanded generator draft on selection. This difference explains initial candidate F.Cu/F.SilkS and opposite pad order while both forms say Back.

Exact source chain:

- `ergogen/library/switch_mx.js:134` declares default `side: 'B'`. `ergogen/src/index.ts::catalogue` creates definitions with an empty generator parameter map, then `normalizeDefinition` renders that map.
- In the shared retained provider, `ergogen/src/index.ts::render` first fills defaults but then explicitly assigns `p.side = inputs.side ?? (mirrored ? 'B' : 'F')`. A library definition with no part and no explicit side renders F. This provider behavior is byte-identical in the pinned reference; changing the shared engine is neither necessary nor proposed.
- Dioxus `parts/generator_settings.rs::parameters` displays saved value or schema default. Its selection-identity effect clears `edits` and the draft store; only `change_parameter` calls `prepare_generator_candidate` and publishes normalized transient geometry. `parts.rs::PartsWorkspace` uses that transient definition only once Ready; otherwise the original catalogue geometry reaches `PartsPreviewPanel`.
- Pinned TS `app/src/ui/usePartsEditing.ts:51–58` fills its transient edits map from all schema defaults and overlays saved values on selection. `generatorDraft` applies that map and normalizes the initial preview. Explicit B reaches render before any user input. There is no need to attribute the difference to a compiled KiCad fallback.

## Rejected competing explanations

The initial cached-KiCad-source hypothesis is inconsistent with both the source and actual DOM. `catalogue()`/`normalizeDefinition()` do not attach `kicadSource` to bundled MX. Dioxus `source_backed_preview` emits no generator drawings and its outline is Courtyard, whereas the observed canvas advertises SilkS and Keycap. The existing warning is the catalogue's generated-envelope diagnostic, not evidence that the KiCad branch was selected. All layer buttons were enabled in both apps, ruling out a simple hidden-layer preference. Solid versus dashed Keycap is separately explained by Dioxus `.m1-keycap-overlay rect` CSS; it does not establish wrong geometry.

## Minimal public falsifier and repair boundary

On the same candidate selection, change **Board side / side** to Front and then Back; wait for the Ready/unapplied preview after each, without Apply. Prediction: Back now renders B.Cu/B.SilkS and pad1 left/pad2 right, matching the reference; the accepted project revision stays unchanged. Reselecting the bundled component should restore the original omission if no initial-default effect exists. If this prediction fails, retain exact public draft/status evidence before any patch.

A justified product repair would prepare the initial transient generator candidate from schema defaults plus saved parameters through the existing normalizer and current-owner guards. Preserve accepted document and bundled catalogue identities; do not rewrite shared provider defaults or merely force B in the SVG. The regression must mount the actual settings/preview consumer with packaged MX and prove B layers/pad order immediately after selection without a change event, plus no accepted edit. A pure `apply_input` test cannot catch the missing initial call because that helper already expands defaults correctly. This is a proposed owning RED, not an executed test or implementation lease.

The independent clipped Key assemblies list is documented by `candidate-assembly-ancestors.json` and bounds: 273px content inside ~40.93px details, leaving ~2px of the first choice. Root owns that CSS repair; this note does not conflate clipping with generator orientation.
