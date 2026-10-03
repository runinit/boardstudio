# Consolidated module attachment review — 2026-10-03

Verdict: source and supplied focused test/package gates pass after the catalogue-settlement repair. The repaired fresh attachment, accepted navigation, connector reuse, history and archive-reopen journey passes. No remaining actionable source finding was identified. F5.5-C03 remains **implemented**, because the full accepted profile/model/asset reuse finish condition is not yet qualified. No parent acceptance is implied.

## Scope and source identity

Reviewed `05f96412..a0bd90ca` and the corrective catalogue projection through final source `733c1da2abede39a617d2eca2e42d9bd437cea41`. The batch adds the private Parts module attachment child, forwards its callback through the Parts workspace, and admits navigation in the root only after the matching accepted attachment. It uses the existing bundled module/connector definitions and `SetMountedModule`; it changes no public Core API, wire contract or saved-document format.

This is the single consolidated attachment review. The reviewer read focused source and retained evidence, ran no application tests/builds/browser sessions, and wrote only this artifact. The prior controls review remains the historical verdict for its earlier source; this report owns the subsequent attachment changes and repaired journey.

## Source findings and repair

The attachment owner captures project/session/board scope, exact Parts definition selection and selection generation, accepted token and revision. It rechecks these before submission after connector loading. Pending controls prevent draft changes; keyed ownership and alive/unmount checks prevent an abandoned owner from navigating or reporting success. Root navigation separately checks Parts workspace, current scope/generation/selection, accepted token/revision and the exact mounted identity on the expected board before opening its PCB Inspector.

Fresh module identity allocation checks both mounted module IDs and part IDs, including the automatic connector ID. Existing connector reuse requires membership on the selected board and a host-role definition. Automatic creation passes the pinned horizontal connector definition through the same Core transaction as the exact selected module definition. Core rejects incompatible definition identity collisions rather than replacing independent project snapshots. No whole-document replacement or unrelated placement mutation is added.

One actionable ownership finding was confirmed on the initial published candidate: an accepted token update invalidated the request-owned module catalogue view, temporarily unmounted the attachment owner, and suppressed its success/navigation callback. Browser export proved two complete module/connector pairs after two user attachment attempts; it did not establish partial persistence or a double dispatch from one click. The first success had no visible completion before the second attempt.

The repair extracts production `catalogue_view`. Matching request results retain their prior behavior. While a resource result is missing/stale, an already loaded immutable bundled catalogue is merged with **only the current accepted request’s project overrides**. It does not reuse the stale result’s project entries. Inactive catalogues remain empty and initial loading without a cache remains pending. This preserves the settling owner across its own accepted edit while dropping prior-document overrides after a switch. The corrective source has no further actionable finding.

## Supplied tests and package evidence

The production catalogue refresh regression failed before the correction at `accepted token refresh must not unmount the selected module owner`, then one WASM Chrome test passed. It also checks current accepted overrides, removal of a previous document’s overrides on switch, initial no-cache loading and inactive state. Exact red/green logs and hashes are retained in the [module receipt](modules/vik-splitter-gnd-join-ad26b07a.md). Compiler/package gates were executed by the root/authors; this review did not rerun them or earlier unrelated checks.

| Published candidate | Source | Package result |
| --- | --- | --- |
| `frontend-module-attachment-20261003`, port 34781 | `a0bd90ca579e0139acf5d63c251bc8a8290276d5` | 126.26576 seconds; initial settlement failure reproduced publicly |
| `frontend-module-attachment-repair-20261003`, port 34782 | `733c1da2abede39a617d2eca2e42d9bd437cea41` | 130.45538 seconds; repaired journey passes |

The [initial proof](../frontend-module-attachment-20261003/package-proof.json) and [repair proof](../frontend-module-attachment-repair-20261003/package-proof.json) each report 8 fresh and 22 inherited commands, zero source/asset mismatches and release warnings, and HTTP 200 with isolation headers at root and `/boardstudio/`. Each proof names its exact source commit. Later evidence/criterion edits do not alter packaged app assets. Package identity and functional qualification remain distinct.

## Browser qualification and criterion limits

The [final appended receipt](modules/vik-splitter-gnd-join-ad26b07a.md) imports the retained fixture, creates an empty Board 2, selects Haptic DRV2605L `3V3 pullups, JP1 bridged`, and performs exactly one attachment with the default automatic connector. The repaired candidate opens PCB on Board 2 with the correct mounted Inspector and selected variant. Public exports show exactly one module and one J_VIK1 connector with matching generated identities.

Repeat Save reports success and retains those identities and contents. It creates an ordinary history entry: the first Undo removes that save entry while retaining the module/connector; the next Undo removes the attachment and both objects. Redo restores the same IDs. Four public archives establish the respective states. A second owned browser session imports the final archive, selects the actual module SVG with focus/Enter, and uses `Edit selected mounted placement` to reopen the same Board 2 placement and J_VIK1. Both sessions were closed.

This qualifies the previously missing fresh attachment extension and supersedes the initial accepted-navigation failure. The prior paired TS fresh Haptic route, circuit join/removal and constituent placement evidence is reused; it is not presented as a new complete replay. Existing foreign-connector rejection evidence remains valid. Pending selection/scope switching, connector-load failure and rejected persistence/recovery branches were not executed by this final green journey; source guards do not substitute for those browser receipts.

The cited F4 receipts at 34764/34765 establish exact variant/readiness/source attribution and source-board/component preview, with explicit limits. Neither exercises an accepted profile edit or model/asset change flowing back into the integrated placement. Therefore F5.5-C03 stays implemented with that residual, even though the fresh attachment extension passes. F5.5-C02 also retains its broader unqualified action/scope branches; no full criterion or F5.5 parent is closed from these narrower receipts.

Removing F4.7 from the **criterion start blockers** is consistent with the existing definition/profile/readiness and edit capabilities. The original parent `acceptance_after F4.7` remains intact; ownership of source authoring stays in F4 and placement stays in F5. Current records preserve 10 accepted parents of 62, 52 open parents and accepted history. The separately qualified F5.2-C03 Apply history receipt brings the current verified-criterion count to 35; this source review makes no additional claim about that independent journey.

The later Case baseline receipt reproduces the same terminal/net preview error before any reassociation on the unchanged fixture. That resolves the earlier reassociation-regression uncertainty without qualifying Case preview or the broader F7 joins. No preview/provider completion or release readiness is inferred here.
