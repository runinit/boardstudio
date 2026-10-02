# Standards review request — Layout Transform / Align child boundaries

Please review the same bounded proposal as the [Spec request](spec-review-request.md), focusing on:

- feature-local ownership: leaf form/menu receives read-only accepted projection and emits typed intents; root retains Runtime, selection, workspace/scope/presentation generation, edit admission, callbacks and gesture lifetime;
- existing Core/Session operations and no public/member visibility, schema, new event, duplicate document/selection store or UI-calculated fake geometry;
- current Scope, board, target, token/revision and linked-counterpart revalidation before each edit; previews/cancel/final pointer sample and exact one-step history;
- whether the missing standalone rotation operation and missing Relations route are honestly excluded, with no inert controls;
- whether F3.3a and RF-005 source findings remain intact, with no broad refactor or unreviewed new IDs;
- whether command-pill composition is correctly delayed until Select/Snap/Transform/Align are real, preserving the separate view group, keyboard/focus and compact layout;
- whether the private align envelope policy can use the accepted courtyard/keycap envelope source with unsupported geometry rejected, and whether this should remain an implementation review gate.

No implementation or build is included. The paired evidence only establishes the current visual control mismatch; fresh Sofle menu captures establish labels/scope response but not geometry edits or acceptance.
