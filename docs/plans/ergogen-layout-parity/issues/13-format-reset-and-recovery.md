# 13: Decide the format reset and recovery behaviour

Status: resolved
Type: grilling
Blocked by: [04](04-layout-kernel-order-of-operations.md), [07](07-matrix-anchors.md), [11](11-mirrored-halves-rules.md)
Map: [map.md](../map.md) · Decision: [ADR-0036](../../../adr/0036-matrices-follow-ergogen-point-layout.md)

## Question

Graduated from the map's Format reset fog after the layout and exception decisions
settled. ADR-0036 already chooses a format bump, refusal of older documents and no
migration. Decide the user-visible and storage-lifecycle details without reopening that
choice:

- the new format identity and its checks across local storage and portable archives;
- the message and available recovery actions when an older project is refused;
- whether/how old stored projects are retained, exported, explicitly cleared or replaced;
- how reset/new-project actions treat current drafts, pending edits, Undo/Redo and
  retained browser data, including cancellation and storage failures;
- messages for archive import and incompatible document versions, keeping refusal
  distinct from corruption or successful conversion.

Read the current storage/import/version paths before proposing policy. Do not infer
permission for automatic data deletion from the no-migration decision. This ticket
decides the policy with the user; it does not perform a reset or remove stored projects.

## Answer

Decided with the user on 2026-10-23 in two rounds; all eight recommendations were
accepted. This is the planned layout-parity contract, not a claim that the current
version-6 implementation already enforces it. No stored data is reset by this ticket.

### Version identity and no-migration gate

| Identifier | Layout-parity decision |
| --- | --- |
| Document `formatVersion` | **7**, the new schema for the agreed layout changes. |
| Document/envelope `format` | Keep **`boardstudio/v2`**; it is not the numeric schema version. |
| IndexedDB database version | Keep **1**; no database-schema reset is needed for this decision. |
| Optional `archive.json` metadata | Keep its existing purpose; do not invent a second project schema version there. |

Use controlled, consistent validation at browser list/load, archive import and Core
Open, including paths that can otherwise deserialize directly. Inspect the version
before legacy migration or schema-dependent interpretation and before replacement
side effects. Accept the new document only after normal schema/integrity validation.

- Version 7 enters normal current-schema validation.
- Older versions, including unversioned legacy input, are unsupported. Do not migrate,
  stamp a new version, inject defaults as a conversion, or normalize and save them.
- A newer version requires a compatible newer build.
- A malformed version field or invalid document/package fails validation. An
  unrecognized format identifier is unsupported; do not infer a conversion from it.
- Bundled examples and fixtures are explicitly rebuilt under
  [ticket 14](14-starters-and-demos.md), not upgraded through a runtime legacy route.

Changing only `CURRENT_VERSION` is not sufficient. The current migration loop would
reach `unreachable!` for version 6 after a constant-only bump; adding a normal 6→7
step would still migrate older inputs. The implementation must replace reachable
automatic-migration behaviour on these entry paths with the controlled refusal gate.

### Preserve saved data and distinguish errors

Keep incompatible saved projects and their assets unchanged and listed as
**Unsupported format**. Healthy records remain available in the same project list.
Listing or attempting to open an unsupported record must not rewrite it, claim it was
converted, or trigger automatic storage/asset reclamation. Accept retained disk usage
until explicit deletion, preserving ADR-0019's conservative asset-safety rules.

Messages distinguish unsupported versions from corruption. Include the found and
supported version when known. For example:

- Older: **This project uses format 6. This build requires format 7. No conversion is
  available. Your saved data has not been changed.**
- Newer: **This project requires format 8. This build supports format 7. Open it with a
  compatible newer build.**
- Invalid: **Project data failed validation**, followed by the relevant reason, not a
  suggestion to repair the document by changing its version tag.

Unversioned legacy input gets an explicit legacy/unsupported explanation, not a
guessed new version. Package integrity checks, limits, asset hashes and path validation
remain required; a version refusal is not a statement that an old package is otherwise
valid. Do not offer a version-tag-only repair, legacy editor or automatic converter.

An incompatible saved record offers existing actions:

- **Download raw saved data**, labelled **JSON only**, not a complete project-and-assets
  archive. Retrieve the stored JSON without migration; do not pretend this reconstructs
  an archive or embeds its assets.
- **New project**, creating fresh identities without overwriting the old record.
- **Delete**, with explicit per-project confirmation.

Do not promise an unavailable legacy build or add a legacy archive exporter.

### Import refusal and explicit deletion

An incompatible or invalid archive rejected before Open leaves the original file,
active accepted document, drafts, Undo/Redo and stored projects/assets unchanged.
Validate the candidate and package before dispatching replacement work; do not leave
partially imported projects or assets. Persistence failure after a valid candidate
has entered replacement is the retained-result case below, not a successful import
or a claim that the replacement was published.

**Start fresh** means create a new project, not wipe browser data. Add no automatic
or global factory-reset action. Delete targets one named project, is cancelable, and
uses a comparable storage token so another tab's changed record cannot be deleted
under stale confirmation. A changed record requires refreshed observation/confirmation,
not a retry that silently bypasses the guard. Unrelated projects and potentially live
assets remain protected. Failed deletion stays visible and retryable, never falsely
reported as success.

### Drafts, pending work, publication and history

Validate a proposed saved project or import before abandoning the current editor's
work. Ask before discarding uncommitted drafts. A canceled switch does not discard
current work; already-owned edits/saves may continue their normal settlement.

Do not cancel an owned save or silently drop queued edits to switch. Wait for
settlement. Known save failure retains the exact computed result for save-only retry;
unknown executor outcome requires explicit recovery. Neither retry nor switching
bypasses the stale-tab checks in ADR-0019. Publish the replacement only after its
storage transaction succeeds, following ADR-0013, and do not replay the Core edit
when retrying persistence.

A successful New/Open/reopen starts a fresh Editor session: empty Undo/Redo, fresh
identity state, and retired gesture/job/export work. There is no cross-format Undo or
history migration. Merely listing an incompatible record, rejecting an input or
canceling a switch must not clear current history/session state. This is not a promise
of Undo survival across executor failure or explicit reopen/recovery.

The format change does not authorize forced worker activation, closing other editors,
or clearing their offline assets. ADR-0022's active-editor update policy remains intact.

### Required implementation acceptance

Exercise real browser-storage, archive and Core/Session entry paths, not only a new
version helper, covering:

- Valid version-7 round trips; refusal of each older version, missing version, newer
  versions, malformed fields and unsupported format identifiers, without panic,
  migration, version stamping or persisted input changes.
- Direct deserialization/Open routes cannot bypass refusal or reset history before
  validation. Listing mixed healthy/unsupported records preserves both and writes none.
- Raw JSON download preserves the stored representation's data without conversion and
  is not presented as an asset-complete archive.
- Unsupported/invalid import leaves current work, history and storage unchanged, with
  no partial asset/project writes; normal package-integrity checks remain enforced.
- New project uses fresh identities; per-project delete cancellation, guarded stale-tab
  conflict, retryable failure and asset retention protect unrelated/live data.
- Pending saves/edits, draft-discard confirmation, switch cancellation and successful
  session replacement follow the agreed ordering. Failed valid-replacement persistence
  retains the exact result, does not publish success, and retries storage without Core
  replay; unknown outcomes use explicit recovery.
- Only successful replacement resets session/history. Error text/actions distinguish
  older/newer/invalid input without a converter, tag-repair or global wipe path.

Verification for this decision was source/test inventory and focused source inspection.
No builds, application runtime tests, browser actions or storage changes were performed.
The current migration loop and ordinary New/Open draft-confirmation gap are explicit
implementation risks, not delivered support.

## Comments

### 2026-10-23 — current-path inventory

- The document's numeric `formatVersion` is currently 6 (`migrate::CURRENT_VERSION`);
  `boardstudio/v2` is a separate document/envelope identifier. IndexedDB's version 1
  and the optional `archive.json` metadata have different purposes.
- Browser list/load (`storage::decode_document`) and archive unpack currently run
  ordered migrations before typed document use. Missing `formatVersion` means legacy
  version 1. Listing/loading does not itself write migrations back; a later guarded
  save can persist them.
- Merely bumping `CURRENT_VERSION` to 7 is unsafe: the migration loop has steps only
  through 5→6 and would reach `unreachable!` for 6. Adding a normal 6→7 step would
  still migrate old inputs, contrary to ADR-0036. The implementation needs controlled
  no-migration validation on every load/import/Open route, not just a constant change.
- Unreadable saved records already remain listed, with raw saved-JSON download and
  guarded confirmed deletion. Raw JSON is not a complete project-and-assets archive.
- Successful Core Open clears Undo/Redo and seeds new identity state. Ordinary New/Open
  currently lacks the recovery path's explicit pending-work discard confirmation.
  Existing save-only retry, stale-tab conflict and conservative asset-retention rules
  in ADRs 0013/0019 remain applicable.

This is source/test inventory, not execution evidence. No builds, runtime tests,
browser actions, data resets or storage writes were performed.

### 2026-10-23 — round 1 accepted

The user answered “Y to all” to these three recommendations:

1. Preserve incompatible saved projects and their assets unchanged, listed as
   Unsupported format. Do not automatically wipe storage, rewrite version tags or
   reclaim their assets. New projects get fresh identities. Retained disk usage is
   accepted until explicit deletion; this does not add a legacy editor or converter.
2. An incompatible archive import changes nothing: do not replace the active project,
   discard drafts/Undo or leave partially imported projects/assets. The original
   archive remains untouched.
3. Distinguish older unsupported format/no conversion, newer format/compatible newer
   build required, and malformed or damaged data/validation failed. Do not label an
   unsupported version corrupt or offer a version-tag-only “repair”.

### 2026-10-23 — round 2 accepted

The user answered “Y to all” to these five recommendations:

4. Use document version 7 with the existing `boardstudio/v2` format identifier,
   IndexedDB version 1 and archive metadata. Every entry path refuses older/newer
   input without migration; missing versions are legacy/unsupported and malformed
   versions fail validation. Rebuild bundled data explicitly.
5. Reuse JSON-only raw download, fresh New project and confirmed Delete for unsupported
   saved records; add no converter, legacy archive exporter or unavailable-build promise.
6. Start fresh creates a new project, not a browser-data reset. Delete one named project
   with cancelation, stale-record guards, retryable failure and unrelated/live-asset safety.
7. Validate before abandoning current work, confirm draft discard, and do not cancel
   owned saves or silently discard queued edits. Wait for settlement and preserve the
   existing save-only retry/explicit recovery flow.
8. Successful replacement starts a fresh editor session/history; detection, refusal and
   cancellation do not clear current history. Publish only after persistence succeeds,
   without promising Undo survival across executor failure or reopen.

All branches are settled. The ticket is resolved as a design decision, not an
implementation or permission to reset stored data.
