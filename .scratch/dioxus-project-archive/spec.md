# F2.2 capability spec — bundle-aware portable project archive

**Status:** Draft for independent Spec and Standards review. This narrows an existing F2.2 provider gap; it does not change the 62-task parent graph or parent acceptance.

## Problem Statement

The Rust/Dioxus application can package the current accepted document and its project-owned local assets through the existing Core archive worker. It cannot yet produce the TypeScript-compatible portable archive when a project references bundled Ergogen model files that are not already stored as project assets. The Dioxus packer also does not receive or record the shared `embedUsedModels` choice. A missing model or asset must fail the operation instead of yielding an archive that opens with broken references.

## Solution

Provide one private, snapshot-bound project-archive packing capability that accepts the current accepted document/scope and the `embedUsedModels` value. It keeps all project-owned local assets in the archive, resolves referenced bundled model assets only when the option is enabled, and sends the augmented packed document and verified bytes through the existing archive provider. Its output is a downloadable project copy whose contents match the pinned React behavior without changing the live accepted document, saved document, or asset store.

F2 owns whole-project portable-copy packing. F8 owns the Export-route checkbox and its own export-row behavior. The Project-menu issue consumes the same private pack semantics and same preference source, but its menu action remains a separate child. Do not change parent graph edges, add an archive engine, widen a public API, or change archive/project schemas.

## User Stories

1. As a designer, I want my project-local assets included in a portable copy regardless of the bundled-model option, so that files I added remain available after import.
2. As a designer, I want referenced bundled Ergogen models included when the shared option is enabled, so that the copied project renders and exports like the source project on another browser.
3. As a designer, I want only bundled models referenced by the current project included, so that unused library content does not bloat my archive.
4. As a designer, I want the option disabled to omit only bundled model copies while preserving document-owned assets, so that the shared Export preference has predictable meaning.
5. As a designer, I want unavailable model bytes, a mismatch against an independently declared expected digest (when the input provides one), or missing local assets to stop the download with a useful error, so that I do not trust an incomplete archive.
6. As a designer, I want packaging to use the accepted project snapshot and reject stale completion, so that a download cannot silently represent an older or different project.
7. As a designer, I want the packed archive to describe the bundled assets it contains without adding those assets to my live project, so that creating a backup never changes the keyboard I am editing.
8. As a designer, I want the Project menu and Export route to use one setting and one packaging behavior, so that the same preference has the same result at either entrypoint.

## Implementation Decisions

- Use the existing accepted snapshot and scope guards, BrowserStore local-asset reads, `PackProject` archive request/reply, and browser delivery lifecycle. Capability 01 adds a private `embed_used_models: bool` pack input and tests both values; no existing Rust composition input is assumed. The visible F8 checkbox and paired entrypoint behavior are later integration acceptance, not a prerequisite for adapter work.
- Keep archive packing private to the frontend/runtime boundary. Pass the embedding choice into the operation as an ordinary setting; preserve F8 ownership of the visible Export-route option and F2 ownership of Project-menu portable-copy integration.
- Compute the same used-bundled-model closure as the React packer, including generator-derived models for used Ergogen instances with effective parameters, definition models, module definitions and circuit definitions, assembly members and overrides, and board references.
- Resolve only recognized bundled Ergogen IDs through provider ticket 02, which defines a private catalogue identity/metadata/raw-byte seam backed by source model assets packaged as static application files. Ticket 01 consumes that provider and owns used-model closure, its own private boolean input, packed-document construction, and archive invocation; it does not create a second provider or use runtime network URLs. Compute SHA-256 from supplied bytes for the packed asset record/archive path and retain source ID, filename, media type, and computed digest; React's bundled catalogue has no independent expected digest. Rely on existing Core archive validation to verify path/payload digest consistency. Runtime operation must work offline after the application assets are installed, and archive packing must not route bytes through the renderer's stricter model-content validator.
- Build a packed-document value that adds asset records for newly embedded bundled models. Never mutate the accepted snapshot, browser project store, or asset store as a side effect of exporting.
- Keep document-local assets in both option states. `embedUsedModels=false` excludes only bundled catalogue bytes not already present in `document.assets`; preserve React's behavior for model assets already owned by the document.
- Include the existing archive metadata value for `embedUsedModels`; preserve current archive limits and validation. Treat model bytes as opaque, as the React/Core pack path does: do not reject empty or otherwise content-unrecognized model bytes beyond existing provider availability/read, digest, path, and size checks. Do not modify the archive transport/schema to add the behavior.
- Reuse ordinary error reporting and action retry. Do not add progress/cancel UI absent from React. Internal cancellation and stale-scope rejection remain required.
- The TypeScript route keeps its existing `${project.name}.boardstudio` filename. Project-menu and Export menu/control placement remain owned by issues 16 and F8.1 respectively.
- Preserve the existing F2.2 start and INT.2 acceptance joins. This capability ticket is blocked by provider ticket 02 and requires the existing accepted-snapshot, BrowserStore asset, archive-worker, and guarded-delivery inputs, which are callable today. It creates/tests its private `embed_used_models` bool input itself; it does not wait for the F8 checkbox. It does not mark or waive F2.1/F2.2/F8 acceptance.

## Testing Decisions

- Treat the visible project-archive action and archive round-trip as the highest existing behavior seam. Compare the same accepted fixture, option value, scope, bytes, ZIP paths, embedded project asset records, and restored project on pinned React and Dioxus.
- Test both preference values: local project assets are present in both; only the React-compatible bundled model closure is conditionally added; unused library entries are absent; existing project-owned asset records are not duplicated. React unconditionally includes recognized `definition.models` references from every document definition, even with no placed part; preserve that positive behavior.
- Include fixtures for a used Ergogen part with effective instance overrides, an unplaced definition with a recognized `definition.models` entry (included), an unused library model and a no-part Ergogen definition with no static model references (omitted), module/circuit models, assembly-member models/overrides, and board-reference models.
- Assert a missing packaged provider or local asset, byte-read failure, stale snapshot, or a mismatch against an independently declared expected digest (where the input contract supplies one) prevents delivery and preserves the live project/document/assets. For bundled catalogue bytes, compute the digest used in the asset record/path and let Core verify the archive path/payload match; do not invent a catalogue expected digest. Verify offline operation with shipped assets. Ensure archive packing bypasses renderer-only empty/format/size validation while retaining Core's archive size/path/hash limits. Do not introduce model-format/content validation; preserve existing opaque-byte archive behavior.
- Inspect the extracted archive in an isolated browser profile and compare the restored packed document and asset bytes/identities. Verify the live pre-export document and store remain unchanged.
- Retain applicable archive/asset contract tests, paired desktop/compact and light/dark browser captures, keyboard/focus checks, and no browser page errors. The Project-menu child separately compares its click path and settings integration.
- Existing prior art includes Rust archive pack/unpack and reference storage/exporter tests. Do not close F2.2, F8, or F9 from private unit tests alone.

## Out of Scope

- Adding the Project-menu `Save project copy` control/action, which remains issue 16.
- Adding or repositioning the Export-route `Embed used models` checkbox, which remains F8.1/F8.2 ownership.
- New Rust public API, transport or document schema, archive format, backend algorithm, browser store, user asset mutation, archive engine, or runtime network download.
- Export readiness for PCB, firmware, CAD, Case or other outputs.
- Closing any parent task or changing existing F2/F8 parent graph edges, INT.2 joins, blocker history, or acceptance gates.

## Further Notes

### Source and capability audit

The issue16 contract is SHA-256 `4391010a63ea561416dd71c038e3e6960f5bcc106b5353344d5ffe84fe45203d`; its independent Spec and Standards reviews were clear at that exact hash before this capability split. Current F2.2 parent SHA-256 is `c344151f4c4b69bb305032a01e9fc3d1096ba076ac408fca074524891590709d`.

Current Rust `Runtime::pack_archive` already consumes an `AcceptedSnapshot` and `Scope`, loads every existing project asset by SHA, sends `pack-project` to the Core worker, and rejects canceled/stale completion before delivery. The app-level export path uses the same guarded download map and currently hardcodes `keyboard.boardstudio`. The Project-menu is a mounted dropdown containing the library, but no portable-copy action is mounted there. The current Rust page has no `embedUsedModels` state/control.

The capability gap is specifically model closure/bytes and option propagation. The Rust archive contract already accepts optional archive metadata and verifies every referenced asset entry. The Rust page's model-delivery selection distinguishes missing document assets and missing bundled providers; it does not currently supply an archive byte provider for the Ergogen model catalogue. The React reference uses its bundled-model map and bytes, `modelAssetIds`, `packProject`, and the shared `embedUsedModels` state. F2.2 and F8 specifications already assign the portable pack/control to F2 and the Export checkbox/route to F8. No new RF identity is proposed; preserve existing RF-001/RF-002/RF-009 observations and record no new refactoring takeaway unless implementation yields new evidence.

The separately reviewed issue15 focus discrepancy remains documented in its packet and does not affect this archive capability. Existing F2.2→INT.2 and F8.6→F2.2 graph joins remain unchanged.
