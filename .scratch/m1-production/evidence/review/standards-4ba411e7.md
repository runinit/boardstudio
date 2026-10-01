# Standards review — `4ba411e7`

Compared `97a779a60f06f6ee109ecace17d06fa28e625eee...4ba411e7` in `application`, `web` and `scripts`; only `application/src/session.rs` and `application/tests/durable_session.rs` changed.

## Actionable hard breaches

None. On `Completion::ExportFailed`, [session.rs](/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001/application/src/session.rs:886) now removes the failed operation from the session's active export registry before settling it. This matches export ownership: registry membership is what authorizes future host work through `export_is_current`, and prevents a later reopen from emitting cancellation for work already failed. The added regression test exercises that lifecycle. No public types, method signatures, field visibility, or serialization were changed.

## Carried judgment-call smells

- The new `web/src/lib.rs` exports a broad surface of host/CAD/offline/renderer modules. The binary/example have concrete cross-crate uses and the package is unpublished, so this remains a nonblocking API-surface question.
- `web/src/runtime.rs` remains a long coordinator spanning session effects and browser workflows. Its ownership is accepted for the root runtime; consider splitting only around a distinct lifecycle owner.

No source mutation or build/check was performed for this review.
