# T1-02 saved keyboard cards: source contract

Prepared 2026-10-02 in `/home/chris/.local/share/boardstudio/worktrees/frontend-library-20261002`, branch `codex/frontend-library-20261002`, baseline `598b2c026941130f9b95ff4fee57353f8eefcb0d`. The worktree was clean during investigation. No application source was changed and no browser/build/test was run. INT.1 acceptance was reported by the coordinator; implementation still awaits the coordinator/Astra contract clearance.

## React behavior and visible card projection

- Reference component: `app/src/ui/ProjectLibrary.tsx`. Listing runs `listProjects()` on mount and when `document?.id` or retry counter changes. It reports loading, failed with “Try again”, and ready. It has no special empty copy when the list is empty. A failed request preserves the prior `projects` array; list retry increments the attempt counter.
- Input records are complete `ProjectDoc`s from the current accepted document plus the read-only browser list. The current document is placed first; saved records with its ID are removed; the rest sort by `projectName(a).localeCompare(projectName(b))`. Name is a nonblank `name.trim()` or `Untitled keyboard`. The name fallback is for presentation only.
- Preview derives only switch parts: resolve `part.definitionId` against definitions where `kind === 'switch'`; size precedence is `part.keycap`, definition keycap, then 18×18; SVG coordinates are `(pose.at.x, -pose.at.y)` and angle is `-pose.rotation`. Rotated rectangle extents determine a padded SVG viewBox. Card summary uses switch-key count and `boards.length` (“Single board” for 0 or 1, otherwise “N boards”). No switch parts displays “No keys placed”. A per-record exception displays “Preview unavailable” plus “Open to check this keyboard”; the card remains openable.
- The card is one keyboard-open button named `Open {name}`. Current card additionally has `aria-current="true"`, an accent outline and “Current” marker. React styles are in `app/src/ui/project-library.css`; relevant reference sizing is a 108px desktop preview and two cards per row below 540px. At ≤820px search/button controls increase to 44px. Light/dark values use existing `--wb-*` tokens.
- Project menu and entry both render the same `ProjectLibrary`; project menu closes before calling `onOpen`. Opening is the only saved-card action that changes active project/session. Deletion, import, demos and search are outside T1-02.

`LibraryWorkspace.tsx` is the separate 3D parts-library viewer (`app/src/ui/LibraryWorkspace.tsx:18`); it is not involved in saved keyboard summaries or open behavior.

## Exact Rust/provider inputs and existing actions

- The page has one `Rc<Runtime>` and one `Session`; `web/src/presentation.rs::App` subscribes once and puts that runtime in Dioxus context. Both `LibraryLanding` and the existing `<details class="m1-project-menu">` render the same `Library {}` component. The current component also consumes the shared version signal.
- Current record source: `runtime.model().accepted.as_ref().map(|s| s.document.clone())`; `ReadModel.accepted` is an `AcceptedSnapshot` whose document is `Arc<ProjectDoc>`. This is a presentation snapshot, not a writable domain store.
- Saved records: `runtime.store.list_documents().await -> Result<Vec<boardstudio_core::model::ProjectDoc>, PersistError>` (`web/src/host/storage.rs`). It reads all values from the isolated M1 IndexedDB projects object store and deserializes them; the call does not mutate Session, ProjectDoc, history, selection, or storage. `BrowserStore` is scoped to `boardstudio-m1-root` or `boardstudio-m1-boardstudio`. React remains on `boardstudio-v2`; do not bridge these stores.
- Existing open action: `runtime.open_saved(id: String) -> ()` (`web/src/runtime.rs`). It synchronously advances a checked `open_sequence`, asynchronously loads that ID through the same `BrowserStore`, ignores a load result/error whose sequence is stale, then submits `Event::Open` (or existing `Event::RecoverWithDocument` while lifecycle is `RecoveryRequired`). Missing/error results are reported through `runtime.report`. Accepted state/history remains Session-owned. There is no new facade, schema, or domain/store API needed for T1-02.
- Existing `presentation/library.rs` currently lists only `(id, name)`, refreshes from an effect on accepted `(id,name)`, and calls `runtime.open_saved`. Its task has no request generation/unmount check; list errors only call global `runtime.report`, with no local loading/retry state. T1-02's private presentation state/projection can consume the existing store and current snapshot and keep retry/loading/error local.

## Async/lifecycle audit and integration gap

List state must be request-scoped in the component: a request started for an old accepted identity or an earlier retry must not replace the newest result, and an unmounted library must not publish its completion. The current effect has no such guard.

`open_saved` already rejects late IndexedDB load results/errors after another `begin_open()`. Its guard ends immediately before `submit(Event::Open/RecoverWithDocument)`. `Session::submit` queues opens, sets `Lifecycle::Opening`, and its core response is asynchronous. If open A has passed the load guard and been submitted, then open B begins while A's core request is in flight, A's result can still be accepted before B's load finishes; if B is unavailable/fails, the stale A may become the active document. This is a concrete gap beyond the safe late-storage-load case. The `Session` event contract has no cancellation/supersession event, and the assigned private library module cannot fix an already-submitted open without a runtime/session integration change.

Coordinator review needed before implementation: decide whether T1-02 is accepted with the existing guard interpreted as protecting late provider-load results, or route the post-submit supersession gap to a separately owned Runtime/Session patch. Do not widen the public API or edit `runtime.rs`/`application` from this packet. If a patch is authorized, it needs a regression oracle for A submitted → B supersedes before A core completion → B load fails/late-completes, with active identity remaining consistent with the accepted latest-intent policy.

## Candidate private implementation seam

Keep the implementation in `web/src/presentation/library.rs` and private children under `web/src/presentation/library/`. Project immutable `ProjectDoc` inputs to private card data; make rendering pure over that projection. Local list status/retry and a monotonically increasing request identity live with the mounted library component. All card-open callbacks call the already-present `runtime.open_saved(id)` after the same project-menu close behavior; list/retry/cardy display do not submit Session events.

No global CSS or module registration should be required: `presentation.rs` already declares `mod library;` and mounts `Library {}` from both existing locations. If visual parity requires CSS outside this private component, send the coordinator the narrowly scoped `web/assets/m1.css` patch for review; do not edit coordinator-owned CSS. Shared theme, entry/menu placement and global status remain coordinator-owned.

## Relevant requirements read

- `CONSTRAINTS.md`, ticket `.scratch/dioxus-frontend-tranche-1/issues/02-saved-keyboards.md`, tranche `ACCEPTANCE.md` / `AUTHORITY.md` / `SOURCE-NOTES.md`, and `.scratch/dioxus-frontend-v1/AGENT-ROUTING.md`.
- `/home/chris/.agents/skills/tdd/SKILL.md`: agreed public seam is the integrated Dioxus library + existing BrowserStore/Session action; do not create implementation-coupled tests or new APIs.
- `/home/chris/.agents/skills/implement-spec/SKILL.md`: implement the authorized ticket in its own worktree, then integrate/review through the coordinator. The slot policy and task packet prohibit additional agents here.
- Domain references: `CONTEXT.md`, `docs/agents/domain.md`, `docs/architecture.md`, `docs/adr/0003-rust-application-ownership.md`, `SPEC-host-platform.md`. No `CONTEXT-MAP.md` or `GLOSSARY.md` exists at the worktree root.

## Damaged-preview versus list-deserialization boundary

The ticket wording needs two distinct cases in review evidence:

1. A **typed `ProjectDoc`** returned by `BrowserStore.list_documents()` whose switch geometry cannot produce a safe preview can be isolated in the private card projection and shown with that card's “Preview unavailable” fallback. An explicit finite/positive geometry check may be needed because non-finite `f64` values can propagate to invalid SVG coordinates without throwing; treat this as preview-only, preserve the saved document, and do not block its open action.
2. A **stored value that cannot deserialize as `ProjectDoc`** never reaches card projection. `BrowserStore::list_documents` maps each `JsValue` through `serde_wasm_bindgen::from_value` and collects a `Result<Vec<ProjectDoc>, PersistError>`; one failure rejects the whole listing. `BrowserStore` has no per-record result/skip contract. Isolating this case would require changing the provider boundary, which T1-02 explicitly may not do. Keep it represented as list failure/retry under the existing API and ask Astra/root whether the ticket's “one preview failure” refers only to case 1; do not add a public method, weaken stored-document validation, or create another store.
