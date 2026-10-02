# Proposed F3.3 child graph: Transform, Align and command pill

This is a draft for the integration coordinator. It does not edit `.scratch/dioxus-frontend-v1/workflows/F3.json`, root task files, RUN or the RF register.

| Proposed child | Scope | Capability start | Acceptance joins |
|---|---|---|---|
| F3.3a, existing issue 05 | Persistent Select preference and standalone-part Snap controls, as already reviewed; matrix/member free-drag remains under RF-005/F3.3. | Live T1-10 context/selection path, Session gesture path and normalizer. | Canonical F3.3 parent join plus child-specific paired UI evidence. |
| F3.3b, draft issue 07 | Matrix/cell/row/column transform fields and working shortcuts to those fields. Stagger/Splay/Origin pointer tools and Pick origin are deferred pending reviewed cancel-safe preview capability. No standalone-part rotation until a matching edit authority exists. | Selected matrix/row/column/key context, accepted current matrix scene, existing SetMatrix and SetMatrixSplay commit path. | Canonical F3.3 acceptance join to F3.2; child public edit/history evidence. |
| F3.3c, draft issue 08 | Same-board independent reference picker and six one-shot alignment commands; private source-backed envelope bounds and exact guards. No Relationships item. | Current selected IDs/active-board membership plus SetMatrix/MoveParts edit paths; reviewer-approved use of the private envelope policy. | Canonical F3.3 acceptance join; paired live-reference, geometry, guard and history evidence. |
| F3.3d, draft issue 09 | Root-owned composition of working Select/Transform/Align/Snap leaves into one command pill; separate view controls. | F3.3a/b/c included leaf actions have real handlers and shared root lifetime callbacks; deferred Transform pointer tools are not represented as working leaves. | Canonical F3.3 joins plus integrated command-surface evidence; F3.5 joins only when Relationships is later added. |

F3.3 stays `start_after=[F3.1]`, `acceptance_after=[F3.2]`; none of these drafts makes a whole F3.1/F3.2 acceptance an additional start gate. Children do not set canonical graph edges. Root decides when/if to publish IDs and exact graph records after source and standards review. The composition-only child does not start while its leaves are missing or represent behavior with disabled/decorative controls.

## Existing boundaries to preserve

- Existing issue 05/F3.3a owns Select/Snap. This proposal neither duplicates its selection preference nor broadens its standalone-only Snap mutation proof.
- The earlier matrix inspector child owns structural name/rows/columns/pitch and its currently scoped actions. F3.3b adds transform properties only; do not claim whole matrix authoring acceptance.
- Matrix/member free-drag local-coordinate parity is already source-confirmed under RF-005 and remains open in F3.3. Stagger's explicit local offset operation must not be used to claim every cell/member drag is fixed. The pointer Stagger/Splay/Origin commands are not start-ready until a safe preview/cancel capability is proven.
- Relationships/constraints inspector navigation is F3.5. The Align menu omits Relationships until that genuine route is mounted; constraints remain under F3.3's existing overall scope.
- Standalone rotation has no current `MoveParts` operation. Any new or changed Core/API operation requires separate authorization and review; these drafts do not propose one.
