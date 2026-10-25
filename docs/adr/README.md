# Architecture decisions

Accepted decisions, oldest first. Read one when you change the area in its last column. A new ADR
gets the next number and a row here; `scripts/check-adr-index.py` (part of
`python3 scripts/check.py repo`) fails for an ADR that is not listed.

| ADR | Decision | Read when you change |
|---|---|---|
| [0001](0001-fixed-edited-outlines.md) | An edited board outline is a separate copy and stays fixed when components move. | outline edits |
| [0002](0002-linked-outline-refinements.md) | An accepted outline refinement stays linked to its components; Freeze makes a fixed copy. | outline suggestions and refinements |
| [0003](0003-rust-footprint-generators-in-core.md) | Footprint generators are Rust inside Core; the JavaScript is gone. | footprint generators |
| [0004](0004-document-format-version-and-id-rename.md) | Persisted documents carry a format version; migrations run at Core's load boundary and a build refuses a newer version. | the saved document shape |
| [0005](0005-resolve-queued-edits-at-execution.md) | A commit is an intent; Core resolves each queued edit against the accepted document when it runs. | edit queuing and resolvers |
| [0006](0006-replace-document-for-import-and-recovery.md) | Whole-document replacement is reserved for import and recovery. | `ReplaceDocument` or an edit resolver |
| [0007](0007-matrix-gestures-edit-named-parameters.md) | Matrix gestures edit named placement parameters through Core. | matrix placement, stagger, splay |
| [0008](0008-linked-halves-stay-linked.md) | A mechanical pair keeps everything linked; exceptions are explicit. | mechanical features of a pair |
| [0009](0009-generator-parameters-checked-in-core.md) | Core checks generator parameter types and names. | generator parameters |
| [0010](0010-mirrored-half-is-a-core-maintained-reflection.md) | A mirrored half is a Core-maintained reflection with explicit exceptions. | mirrored halves, correspondences |
| [0011](0011-part-offset-is-its-own-constraint-kind.md) | A part-axes offset is its own constraint kind, not a property on Offset. | placement constraints |
| [0012](0012-bounded-outline-validation.md) | Outline validity uses exact 0.001 mm contacts and a 1024-point budget per persisted path. | outline path validation |
| [0013](0013-save-before-publication-and-explicit-recovery.md) | Publish the accepted document only after saving; retry retained results without replaying edits and recover explicitly after unknown outcomes. | saving, accepted state and recovery |
| [0014](0014-document-authority-and-captured-lifetimes.md) | Keep one writable document authority and operation-specific lifetimes; preview recovery respects automatic-preview mode with bounded retry. | application ownership, scope changes, asynchronous results and preview recovery |
| [0015](0015-exact-generated-cad-and-mesh-preview-scope.md) | Generate exact case/PCB output from accepted inputs; keep keycap and vendor preview meshes distinct from manufacturing authority. | CAD construction, STEP export and preview geometry |
| [0016](0016-export-ownership-is-separate-from-preview-currency.md) | Keep exports independent of disposable previews; distinguish active-work ownership from final delivery checks. | export capture, cancellation and delivery |
| [0017](0017-forgiving-vendor-previews-with-preserved-source.md) | Allow repaired and partial vendor previews with explicit diagnostics; preserve originals and do not infer whole-model clearance from incomplete geometry. | vendor STEP import, repair admission and preview errors |
| [0018](0018-distinguish-collision-observations-from-clearance-guarantees.md) | Show approximate collision findings with limitations; reserve clearance conclusions for supported accuracy. | collision checks, measurements and uncertainty reporting |
| [0019](0019-reject-stale-saves-and-reclaim-assets-conservatively.md) | Reject stale multi-tab saves and defer automatic asset cleanup when references or editor lifetimes are uncertain. | save conflicts, recovery and asset reclamation |
| [0020](0020-validate-portable-project-packages-in-core.md) | Validate portable project packages and asset integrity in Core under bounded resource use. | project archive import, export and integrity checks |
| [0021](0021-test-real-workflows-natively-and-browser-behavior-mounted.md) | Test real Session/Core workflows natively with shared controls; retain targeted mounted browser coverage. | workflow test adapters, test doubles and browser coverage |
| [0022](0022-static-local-first-rust-application-with-worker-execution.md) | Keep core workflows local in a static Rust-owned browser app; isolate heavy work and preserve active offline clients during updates. | hosting, execution placement and offline updates |
| [0023](0023-distinguish-mounted-daughterboards-from-embedded-circuits.md) | Keep mounted daughterboard circuitry separate from host-owned embedded copies and require explicit electrical joins. | module mounting, circuit embedding and host electrical ownership |
| [0024](0024-share-reversible-pcb-routing-not-half-firmware-configuration.md) | Share reversible PCB routing while resolving each half's firmware key positions and MCU mapping from its actual assembly and jumpers. | reversible boards, jumper configurations and per-half firmware |
| [0025](0025-delete-a-removed-halfs-exclusive-pcb-design.md) | Delete a removed half's exclusive PCB design and accessories in one undoable edit; preserve shared and surviving data. | topology reduction, PCB deletion and shared-design ownership |
| [0026](0026-decouple-firmware-role-from-physical-side.md) | Allow either physical half to be central where supported; keep firmware role independent of left/right and keymap position. | split firmware roles, target selection and side ordering |
| [0027](0027-save-incomplete-split-designs-with-output-specific-readiness.md) | Allow unfinished split designs to save and remain editable; gate only outputs requiring missing hardware. | incomplete hardware, saving and output readiness |
| [0028](0028-convert-reversibility-explicitly-without-merging-designs.md) | Convert reversibility explicitly: require prior half deletion before sharing a design; create an unrouted mirrored design when separating it. | reversibility controls, design conversion and routing preservation |
| [0030](0030-export-firmware-as-an-editable-repository-template.md) | Export an editable firmware repository template; disclose unfinished optional peripherals without blocking an otherwise valid handoff. | firmware package contents, build ownership and export expectations |
| [0031](0031-preserve-pcb-handoff-protection-across-undo.md) | Preserve established PCB handoff pin-assignment protection across Undo/Redo; require explicit reviewed remapping. | PCB handoff protection, pin remapping and edit history |
| [0032](0032-recreated-inputs-start-unassigned.md) | Recreated inputs do not inherit removed inputs' bindings, even when coordinates or IDs are reused. | key deletion/recreation, matrix shrink/regrow and binding cleanup |
| [0033](0033-include-license-information-in-exported-packages.md) | Include `license.md` in exported packages so known licence and attribution information travels with their contents. | package export/import, attribution and source notices |
| [0034](0034-matrices-follow-ergogen-point-layout.md) | Matrices follow ergogen's point-layout model with deliberate exceptions; saved projects are not migrated. | matrix layout parameters, ergogen parity, format version |
| [0035](0035-split-half-frame-and-stored-gap.md) | Split halves share one stored gap and each half keeps its own frame; legacy boards open unchanged and adopt on first edit. | mirrored pair links, split gap, half-frame origins |

## Deprecated records

| ADR | Disposition | Read when you change |
|---|---|---|
| [0029](0029-constrain-geometry-scripts-and-apply-results-atomically.md) | User-authored geometry scripting is out of product scope; the earlier feature premise was incorrect. Existing code safety remains required pending separately scoped removal. | scripting code, its UI and saved-script compatibility |

## Notes

- Number 0003 was used twice. The first ADR 0003, "Rust application ownership and browser trial
  boundaries" (accepted 2026-10-01), was removed with the migration machinery in `3cdeb2ac2`
  and the number was reassigned on 2026-10-05. Read the original with
  `git show 3cdeb2ac2^:docs/adr/0003-rust-application-ownership.md`. Numbers are not reused.
