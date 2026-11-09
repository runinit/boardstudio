---
status: accepted
---

# Validate portable project packages in Core

Portable project archives bundle the document and its packaged assets, with content-addressed asset identities verified before accepting the package. Core owns shared validation of paths, duplicate entries, required assets and hashes, plus compressed, decompressed and entry-count limits. Reject corrupt packages rather than silently accepting missing or substituted asset contents. The user confirmed this portability and integrity boundary on 2026-10-19, choosing a structured, validated package over arbitrary ZIP interpretation.

Specific numerical limits remain implementation/reference details, not permanent architectural commitments; their original selection rationale was not recovered. Format handling follows [ADR-0004](0004-document-format-version-and-id-rename.md), qualified by [ADR-0036](0036-matrices-follow-ergogen-point-layout.md)'s no-migration reset and [ticket 13's planned version-7 gate](../plans/ergogen-layout-parity/issues/13-format-reset-and-recovery.md#answer). Package integrity is distinct from [ADR-0017](0017-forgiving-vendor-previews-with-preserved-source.md)'s forgiving interpretation of intact vendor model bytes: a model may yield an imperfect preview without making a corrupt project archive acceptable. Generic artifact ZIP exports are a separate path and do not inherit every project-package limit.

Evidence: [Core archive service and regression tests](../../core/src/archive.rs), particularly `unpack_verifies_missing_assets_and_hashes_before_any_output`, `actual_read_limits_stop_at_the_first_excess_byte` and `generic_exports_preserve_paths_and_have_no_project_numeric_limits`. This confirmation records the policy now; it does not invent historical rationale for the envelope or cap values.
