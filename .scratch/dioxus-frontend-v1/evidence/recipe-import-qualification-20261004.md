# F4.6-C02 recipe import qualification — 2026-10-04

Candidate attribution: `frontend-parts-library-repairs-20261004`, source `3a412978cee729a24d37e7e55e7c3664f1187fb2`, before the34822server was replaced by the preset candidate.

## Result

Paired public UI import, save, reload, and reopen succeeded on candidate
`http://127.0.0.1:34822/` and reference `http://127.0.0.1:5175/`. The saved source
recipes were preserved. Each project received a duplicate named
`Import qualification20261004` before import.

The first candidate navigation used the documented `/boardstudio/` subpath and
showed the welcome screen with zero saved keyboards. The coordinator identified
the exact candidate root route; navigating there restored `Grouped journey Sofle
20261004` and its `Grouped recipe final 20261004`. No fixture was recreated.

## Import and persistence

- Fixture: existing valid file
  `ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step`, 33,744
  bytes.
- Supported browser upload flow: start `tab.playwright.waitForEvent("filechooser")`,
  click the visible `Import model` button, then call `chooser.setFiles([absolutePath])`.
  The chooser reported single-file selection. No alternate upload route was used.
- In both frontends the import added a third custom model on the duplicated MX
  member and selected `SMD_0805_Capacitor.step`. The candidate selected asset value
  persisted as `model-asset-24`; the reference selected value persisted as
  `62812dc9-acfb-432f-8fe6-aeb23a8de89d`.
- Both editors reported `Assembly saved. Existing placements are unchanged.` and
  `Saved locally`. After full-page reload and reopening the duplicated recipe, the
  imported asset remained selected. Existing Model 1 and Model 2 remained selected
  as `SW_Hotswap_Kailh_MX.stp` and `SW_Cherry_MX_PCB.stp`, respectively, with their
  original transforms intact. Model 3 retained offset `(0,0,0)`, rotation `(0,0,0)`,
  and scale `(1,1,1)`.

## Preview and limits

- Candidate isolated Parts sample preview was ready before save and reported
  `3D preview ready` after reload/reopen.
- The reference editor exposed the complete PCB assembly preview rather than an
  isolated draft-preview region. It settled at `3 / 3 models · 1.6 mm PCB` after
  reload. This qualifies the reference scene settling, not isolated draft preview.
- Malformed-file rejection, non-finite pose/model fields, member-ID errors, missing
  model references and file-size-limit behavior were not exercised. No source-tree
  or library-asset changes, builds, tests, commits, or screenshots were made; the
  project-local imported assets are the intended browser output. Mobile checks
  remain deferred; Case-local export and preset Customize3Dassembly are outside
  this packet.

## Existing validation coverage map (read-only reconciliation)

- **Paired public behavior already observed:** the consolidated 34822 receipt records
  empty-name and empty-member Save rejections (`Name the assembly and add at least
  one member`), rejection of first-member X=4 for matrix Apply, and rejection of
  Model 1 Scale X=0 while retaining 1. This receipt adds successful valid-file
  import, isolated candidate preview readiness, and Save/reload/reopen. The earlier
  `parts-assembly-34762/RESULTS.md` changed-only preview journey reuses earlier import
  evidence; it adds no import-error or assembly-validation branch.
- **Owner guards, with no direct assembly-editor test found:**
  `assembly_editor.rs::saved_document` checks blank name/zero members, unique
  nonblank member IDs, finite member pose, finite model offset/rotation/scale,
  positive scale and asset existence (lines 1863–1925). `MemberPatch::apply` keeps
  the previous pose/vector for unparseable/nonfinite values and nonpositive scale
  (lines 1570–1658). `assembly_presets.rs::matrix_with_assembly` and
  `document_with_assembly` repeat member identity/transform guards at matrix/board
  placement boundaries (lines 295–358 and 474–540). The only mounted
  `assembly_editor.rs` test found,
  `persisted_member_selection_survives_dynamic_options_mount`, covers retained
  select values, not error handling.
- **Importer owner guards and native helper tests:**
  `model_asset_import.rs::read_model_file` accepts known filename extensions,
  rejects zero or >32 MiB files, reads bytes, and verifies SHA-256; there is no
  mounted test of this browser `File` path. `model_delivery.rs` native tests
  `format_selection_is_case_insensitive_and_rejects_other_extensions`,
  `verified_bytes_reject_wrong_digest_and_bounds`, and
  `mesh_validation_requires_complete_finite_matching_buffers` cover extension
  parsing, digest/empty-byte rejection, and decoded mesh shape/finiteness. Despite
  its name, the verified-bytes test does not exercise the >32 MiB case; none tests
  STEP parser rejection or browser file-read/store failure.
- **Core boundary:** `core/src/model.rs::AssemblyDefinition` and `AssemblyMember`
  are serializable data structures, with no saved-recipe validator. Core matrix
  geometry validation is a separate domain and does not test editor/import errors.

Smallest remaining public branch is optional malformed-file behavior (if the current
import/preview UI exposes a stable rejection); no public route can author malformed
member IDs, so those source guards should remain identified as owner-level only.
