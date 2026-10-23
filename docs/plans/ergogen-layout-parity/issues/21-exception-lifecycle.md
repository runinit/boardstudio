# 21: Preserve Local/Omitted state through resize, detach, Rejoin and removal

Status: ready-for-agent
Type: build
Blocked by: [20](20-reflect-linked-layouts.md)
Parent: [10](10-slice-the-build.md) · Spec: [spec.md](../spec.md)
Map: [map.md](../map.md)

## Outcome

Close the recorded lifecycle gaps through the actual prepared route, not merely the
direct helper tests. Preserve explicit ownership, geometry and source identity in one
atomic transaction; confirmation is not permission for silent extra exceptions.

## First RED / GREEN slice

Fail a public source resize with a Local partner matrix: source grows, Local dimensions
and poses stay independent, with valid correspondences. Fix that route before adding
the Omitted-partner case; do not bypass production preparation in the test.

## Subsequent acceptance

- Local keys retain world poses through shared changes using only independent offset/
  turn compensation; their Shift/Bend remain shared. Explicit pose edits remain local.
- Make Local on an anchored matrix detaches/keeps pose; its future layout stays own.
  Omitted items retain live logical poses; Local→Omitted uses current shared layout
  rather than freezing former Local placement. Preserve explicit child exceptions.
- Rejoin adopts current source dimensions/layout/anchor; missing anchors or invalid
  graph refuse atomically without changing a Local parent or unrelated state.
- Shared/as-removal Detach preserves both poses. If another Local exception is needed,
  expose the proposed changes for explicit confirmation before commit; cancel changes
  nothing. Revalidate the confirmed intent against current accepted state.
- Source removal retains a complete Local matrix/group, not loose parts; retire only
  dead correspondence, preserve surviving ownership/anchors and unrelated links. No
  source replacement is silently joined. Omitted records retire without resurrection.
- Cover same-board and native systems, source/follower edits, frame/Follow restrictions,
  child exceptions, identity remaps, preview/cancel, one Undo and exact Redo.

## Checks

Primary source: `core/src/mirrored_half/lifecycle.rs`, reconciliation membership kernel
and same-board layout lifecycle. Regressions must enter Core/Session production
preparation; cite the helper/production discrepancy from [decision 11](11-mirrored-halves-rules.md#answer) but do not treat
those helper tests as proof. Run focused lifecycle/mirror tests, then quick/native
checks with name retention. No private adapter may become a second authority.
