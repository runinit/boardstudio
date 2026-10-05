# F9.6 copied-data same-scope cutover rehearsal

This bounded rehearsal uses a fresh named `agent-browser` session, one stable origin (`http://127.0.0.1:34831/`), and a local proxy that switches upstream bytes between pinned reference `http://127.0.0.1:5175/` and immutable candidate `http://127.0.0.1:34830/` (`f956c0dfe9ec942cb79b9905565e9c404fb96732`, build `frontend-layout-pointer-cache-20261005`). It does not clear localStorage, IndexedDB, CacheStorage, or registrations. It does not use the selected in-app browser or a user profile.

The copied input is `reference-joined-input.boardstudio`, SHA-256 `58c0d6303c08dafe4f2259b297d78c619f8c9469163575ff4dcea53c95cf2342`. Run the bounded script with a free local port and fresh output directory:

```sh
python3 .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/rehearsal.py 34831 .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/run-<id>
```

The proxy requests identity-encoded response entities and closes each downstream response so the pinned reference's chunked responses remain correctly framed. It preserves upstream entity bodies and response content types. A request trace identifies the active upstream for every request.

## Observed result

`run-f956-20261005-08/report.json` records successful reference worker registration/control at the staging origin, public `.boardstudio` import, public “Save project copy” download, and reload/readback of the same project identity in React storage. The saved archive has the same SHA as the copied input and preserves all six asset entries.

After switching only the proxy target to the candidate, one reload remained under the React shell and its controller at `/sw.js`. The candidate returned 404 for `/sw.js`; the candidate `/service-worker.js` was never requested. React cache `boardstudio-v2-1b6b10523d1a` and IndexedDB `boardstudio-v2` remained present. No candidate import, rollback, or re-adoption was attempted after this deterministic shell-transition failure. This is a same-scope service-worker cutover blocker; it does not contradict the prior archive round-trip compatibility evidence and does not establish direct IndexedDB migration, which is not supplied.

The final browser session and proxy are held for review. Session: `f96-cutover-164703-3448630`; proxy port `34831`; script process can be released by creating `run-f956-20261005-08/release-session`. Do not repeat the archive journey to reproduce the already captured failure.

`run-f956-20261005-01` through `-07` preserve bounded setup attempts. Those records are harness/fixture failures before an upstream transition; they are not product failures or RED evidence. Run `-08` is the sole completed data journey and transition observation.
# Repaired-candidate continuation (prepared, not yet run)

The old failed `34831` session/profile remains preserved and is not reused for
the final run. First create a fresh reference-controlled profile on staging
origin `34836`. This imports the retained archive through React's public Open
project input, waits for “Saved locally,” saves a public copy, reloads, and
requires the ordinary `/sw.js` worker to be active with no waiting successor.
It does not close that browser session or clear storage/workers.

```sh
python3 .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/bootstrap_reference.py \
  --reference-url http://127.0.0.1:5175/ \
  --origin-port 34836 \
  --input-archive .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/run-f956-20261005-08/reference-resaved.boardstudio \
  --output-dir .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/bootstrap-reference-20261005
```

After candidate `34834` and the copied React rollback artifact `34835` are
published, read the generated session name from `bootstrap-reference-20261005/owner.json`,
create that bootstrap output's `release-session` file to release only port
`34836`, then continue the same browser session at the same origin:

```sh
python3 .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/continue_rehearsal.py \
  --candidate-url http://127.0.0.1:34834/ \
  --reference-url http://127.0.0.1:34835/ \
  --origin-port 34836 \
  --session SESSION_FROM_OWNER_JSON \
  --rollback-archive .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/bootstrap-reference-20261005/reference-resaved.boardstudio \
  --output-dir .scratch/dioxus-frontend-v1/evidence/f96-adoption-20261005/rehearsal/run-worker-handoff-final
```

The owner writes its PID and exact endpoints to `owner.json`, records the
first failed boundary without retrying, and remains alive until the
`release-session` file is created inside the output directory. Each transition
must reach its exact family worker (`/service-worker.js` for M1, `/sw.js` for
React) as the active root registration with no installing or waiting successor
before the next direction. The continuation requires the copied React package
with the worker handoff overlay, not raw `5175`. After each public Save-copy action, the script waits
for the durable “Saved locally” state before reload. It records full project
field diffs and per-asset payload hashes; numeric document differences use the
accepted F94 tolerance of absolute `1e-8` and relative `1e-12`. Do not use this
continuation with a different browser session or a new origin.
