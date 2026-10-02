# Open supersession: public diagnosis

Date: 2026-10-02. Read-only source investigation at migration worktree HEAD 5e5d3370. Running candidate at http://127.0.0.1:34645/ is the coordinator's fresh 598b2c artifact. Dedicated browser session frontend-contract-race. No repository edits/builds. Used diagnosing-bugs and agent-browser workflows. Browser fixture writes are isolated to this session's boardstudio-m1-root database.

## Confirmed red-capable loop

Command (already executed):

```sh
python3 /tmp/frontend-run/open-supersession-repro.py --hold-reply
```

Result: exit 1 in approximately 0.7 seconds, `FAIL: superseded A adopted after newer B was unavailable`.

Exact retained trace: `/tmp/frontend-run/open-supersession-reply-result.json`. Companion outbound-delay trace: `/tmp/frontend-run/open-supersession-result.json`. Script is `/tmp/frontend-run/open-supersession-repro.py`.

The script uses the real application public project-menu buttons and real BrowserStore, Runtime, Session, core worker and persistence path. It starts with the actual REVIUNG41 fixture P, seeds two identified saved copies A/B in isolated IndexedDB, reloads, then deletes B after its card has been listed (normal unavailable-on-open situation). It introduces only a worker transport scheduling gate, not an alternate Session or fake worker reply. A public click enters Runtime.open_saved -> Session.submit -> worker Open. With --hold-reply, the worker actually executes A; its real reply is delayed before the existing CoreWorker.onmessage handler runs. Public click B completes its actual provider load as unavailable. Releasing the unchanged A reply then makes A current and persists its active-project ID.

Before: current P/REVIUNG41, revision 3, active-project P.
A pending: P remains visible; actual A core request m1-2 has executed and its reply is held.
B result: provider get(race-b) observed, current P visible, exact unavailable message shown.
After release A: current Race A, revision 3, durable active-project race-a, Saved locally.
Expected: latest B intent supersedes A; when B is unavailable, P remains usable/current and obsolete A cannot adopt.

The alternate command without --hold-reply holds the outbound request AFTER Session submission but before core execution. It went red repeatedly (three recorded executions while tightening). The inbound reply version separately proves that merely avoiding sending stale requests cannot solve the complete problem.

Control command already executed:

```sh
python3 /tmp/frontend-run/open-supersession-repro.py --no-B-control
```

Exit 0: when B intent is removed, A opens normally. Retained control: `/tmp/frontend-run/open-supersession-control.json`. This controls for the transport gate causing an ordinary open failure. Minimal logical scenario is P -> pending A -> unavailable B -> release A; no edit, workspace navigation, extra worker, or mock Session is needed. Full fixture geometry is simply the available accepted fixture, not a prerequisite claimed by the diagnosis.

One initial selector attempt failed at the harness level (`:text-is` unsupported), fixed by semantic role/name clicks. One attempted capture-event listener could not delay the earlier installed onmessage handler and therefore did not reproduce the intended schedule; it was replaced by wrapping/restoring that exact handler. Those harness failures are not product failures and do not count as red evidence.

## Ranked hypotheses and distinguishing evidence

Before further probes, the following were stated as alternatives: (1) latest-open identity ends at provider loading and never reaches Session acceptance; (2) B's apparent failure is not a real superseding Runtime intent; (3) only the visible name is stale, while accepted/durable state is correct.

Probe: trace provider get IDs without changing their results; gate the real A reply; observe both visible current project and durable active preference. get(race-a), A request, get(race-b), unavailable message, and stale durable race-a eliminate (2)/(3). Source confirms (1): Runtime.begin_open increments a local sequence; open_saved checks it before submit; Session has no sequence/intent from B when B cannot load a document. Runtime.run forwards A's completion, Session accepts a matching request/executor reply, persists it and adopts it. This is a confirmed public defect at the tested boundary, not merely a source risk.

Relevant source: runtime.rs:689–696,764–795 and 188–213; application/src/session.rs:912–927,1174–1263,1292 onward. CoreWorker.onmessage/requests are in web/src/host/core_client.rs; worker dispatch actually mutates its single CoreEngine before replying (web/src/core_worker.rs:33,74–76).

## Repair constraints and next bounded decision

No repair applied or declared designed. The failing public loop is available BEFORE a fix. Private Runtime attempt bookkeeping alone is insufficient after a worker has already changed its authoritative state: dropping the reply strands Session.active_core, CoreFailed forces RecoveryRequired, and synthetic CoreReply::Error makes the accepted snapshot disagree with the mutated worker. Reopening a copy of P destroys worker undo history and is not equivalent restoration. Ignoring/disabling B changes the required latest-intent behavior.

A correct bounded repair needs an explicit Session/core open-intent adoption/cancellation agreement retaining the prior engine/document/history until the chosen open can be adopted, settling cancelled callers exactly once and respecting durable save ordering. First evaluate the smallest additive contract on existing Session/core boundaries; any public API addition goes into an UNAPPLIED reviewable proposal/patch and requires explicit approval. Do not broaden this into a framework/provider/domain rewrite. The tolerant listing API is a separate bounded decision documented in tolerant-listing-api-proposal.md.

Not yet exercised: B returns a valid document; core failure; A queued behind a previous save; B begins during A persistence; prior P has undo history/selection; startup/fixture/import vs saved-open supersession. These remain acceptance requirements, not inferred passes. For the save-pending case, define the durable commit/adoption linearization explicitly rather than treating a changed UI guard as cancellation of a transaction that already committed. A repair must rerun the exact public loop and affected safety variants, then receive a different independent reviewer.

RF: update the source-risk finding to confirmed for post-submit/core-reply supersession. Design lesson: cancellation must be carried to the actual adoption owner, with engine state and caller settlement included; current Runtime/provider-local sequence cannot authorize late Session persistence. Current correctness work remains blocking, not deferred to post-port refactoring.

## Cleanup/provenance

Instrumentation is browser-only and removed by page reload/session close; no production source contains it. Scripts and traces are retained in /tmp/frontend-run as explicitly marked diagnostic artifacts. No user project database or another agent's browser session was touched. The browser gate transports actual request/reply payloads unchanged and captures only IDs/status/current-name needed for diagnosis. No credentials were collected.
