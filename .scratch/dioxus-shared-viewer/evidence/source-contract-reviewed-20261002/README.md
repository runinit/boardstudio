# F7.1 source-contract evidence

This packet preserves the corrected F7.1 contract and its independent final
standards review. The review found both identity/ordering and conditional Case
overlay findings resolved, and cleared the source contract for bounded private
F7.3 authoring. It explicitly does **not** close F7.1/F7.3 acceptance or any
parent acceptance gate.

## Exact reviewed artifacts

`reviewed-contract.md` and `reviewed-dispatch.md` are byte-for-byte snapshots of
the reviewed Git blobs. Their Git blob IDs and SHA-256 digests are in
`HASH-MANIFEST.txt`. `final-standards-review.md` is the retained reviewer report
from `/tmp/frontend-run/f7-common-viewer-final-standards-review.md`.

The reviewer recorded no source edits, compiler run, or browser run. The
contract's source inspection basis is continuation commit
`1574d15766d6abe9c02ba1efa89b3ec09ebd5bc6`; its pinned React comparison is
`5a472a9426e6e38993361da402cd4ec730feb369`. `HASH-MANIFEST.txt` records the
Git blob IDs for the inspected source files at the continuation commit. Those
hashes identify the reviewed source snapshot; they are not a claim about later
worktree edits.

The source evidence supports a private page-binary host/adapter placement,
operation mapping, consumer input projections, the distinction between
application scope, viewer/projection generation, and monotonic renderer scene
sequence, and the corrected React Case-overlay conditions. It does not
demonstrate that the page binary compiles or reaches the chosen private host.

## Remaining acceptance evidence

F7.3 still needs actual page-binary build and strict checks; integrated proof of
the mapped scene, state, camera, pick, handle, decode, lifecycle, failure and
scope-switch paths; and paired behavior for all five consumers. Resize/DPR,
cleanup, initialization/render/context-loss behavior, 2D fallback and provider
retry remain runtime checks. The review says performance, accessibility and
actual assistive-technology gates remain unproven. INT.2 and BND.1 remain
acceptance joins. This source packet supplies none of that integrated or
browser evidence and does not change the existing RF ledger.

