# F7.1 contract re-review

Reviewed updated issue blob `865c3cf0b2bee3b4cd1050a0032078bd7d5a1999` and DISPATCH blob `330f860d8f94b5bfa862255d37383dba95c8fbed` in the integration worktree. Reused the prior renderer/host/consumer source inspection; no compiler, browser or source edits.

**Both prior contract findings are resolved. The contract is ready for bounded private F7.3 authoring.**

The contract explicitly separates application Scope, viewer-instance/projection generation and a strictly increasing renderer scene sequence. It covers same-scope remounts and Parts definition/companion/placement/side/rotation changes, identifies reused sample IDs/revisions, and requires current-owner checks for results, status, picks and gestures. Accepted/false-stale/error propagation is explicit, including the existing host's discarded boolean limitation.

Layout, Keymap and Keycaps now retain available conditional Case overlays when the Case and canonical documents are the same; mechanical assembly additionally requires generatedCase. A separate physical Case projection cannot contribute its overlays to the canonical scene. The Case consumer retains matching document/scene/instance identity.

Private page-binary inclusion of the existing host source and private presentation adapter remain the selected placement; no public facade, API, provider, format or visibility expansion is authorized. Actual page-binary and library checks must still establish reachability and strict-check compatibility. F7.3 must demonstrate scoped scene ordering, all mapped renderer operations, mount/unmount cleanup, errors/context loss, provider retry, 2D fallback, and paired five-consumer public behavior. Performance, accessibility and actual AT gates remain unproven. INT.2/BND.1 remain acceptance joins, not added start barriers.

No new refactoring takeaway observed beyond existing RF-002/RF-012. This clears the bounded source contract only; it does not certify implementation or close F7.1/F7.3 acceptance by itself.
