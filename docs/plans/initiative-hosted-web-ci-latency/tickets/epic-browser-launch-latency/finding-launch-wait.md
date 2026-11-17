# Finding: the ~300 s crate-launch wait (spike 1.1)

Ticket: 1.1, epic-browser-launch-latency. Source-reading only; no builds, tests,
browser runs, CI reruns, or experiments were run for this finding.
Baseline measurements: `docs/plans/research-web-ci-latency/digests/measurements-logs.md`
(run 44018165934 attempts A1/A2 at `46d758d`); local anchors re-checked at HEAD
`e66cfcff21073dec7760c030bf246956865dcbbe` (no diff since `46d758d` in
`scripts/run-wasm-tests.py`, `scripts/check.py`,
`scripts/build-patched-wasm-bindgen.py`, `scripts/wasm_bindgen_tools.py`,
`.github/workflows/check.yaml`, `.cargo/config.toml`).

Claim labels: **demonstrated-from-source** (the cited code does this),
**hypothesis** (fits source + measurements, not shown by them), **unknown**.

## Verdict

**Experiment specified.** Source settles where the wait can and cannot live, that
`pageLoadStrategy: eager` is applied but cannot shorten this class of wait, that
the `tiny_http` patched pair addresses the one stall mechanism the source
contains, and what distinguishes the fast four. Source does **not** settle which
request stalls in a hosted launch or which event releases it at 285–322 s, and
the timing arithmetic contradicts both clean 300 s expiries as the usual
release (§5). Promoting the research hypothesis ("waited out a ~300 s wait in
the runner/WebDriver session, eager did not prevent it") to a cause would
overstate the evidence. §7 specifies the one instrumented experiment that
settles trigger and release; it needs separate user approval because it builds
and runs browsers.

| Research open question | Answer | Label |
| --- | --- | --- |
| 1. Where inside a slow launch? | Most likely inside the runner's single WebDriver Navigate (`goto`) call, before any test output is read. Source bounds make Navigate the only *single* runner phase that can hold 285–322 s, but stacked sub-bound waits, the unbounded bindgen phase, and pre-runner `wasm-pack`/cargo time in the measured launch wall are not excluded by source alone (§2). | hypothesis (elimination from source bounds plus measurements); phase bounds themselves demonstrated-from-source |
| 2. Which wait is ~300 s; does `eager` apply? | Three distinct 300 s constants exist (§3, §5). `eager` is transmitted, parsed, and applied, but only moves the Navigate endpoint to `DOMContentLoaded`, which a stalled module-graph fetch still blocks (§3). Which constant, if any, ends a hosted slow launch is unsettled (§5). | demonstrated-from-source (eager path); unknown (release) |
| 3. Patched pair? | The patch fixes a real lost-queue stranding in `tiny_http`'s connection pool, the only indefinite server-side stall in the source (§4). Relevant as candidate fix; not proven to be the hosted trigger. Its origin commit records no rationale. | demonstrated-from-source (mechanism); hypothesis (hosted trigger); unknown (author intent) |
| 4. Why are four never slow? | They are the four launches whose crates do not depend on `dioxus-web` on wasm32, so their generated pages have no Dioxus JS snippet imports in the module graph (§6). | demonstrated-from-source (dependency difference); hypothesis (page-graph consequence, generated output not inspected) |

## 1. Exact sources read

| Artifact | Exactness anchor |
| --- | --- |
| `wasm-bindgen-cli` 0.2.129 crate (contains `wasm-bindgen-test-runner`) | sha256 `5fd044ed…03168c`, matching `scripts/build-patched-wasm-bindgen.py:28 (CRATES)`; `.cargo_vcs_info.json` git sha1 `165586f854c852bcbc4d14d8a75a0a0e0d0abc12`, `path_in_vcs: crates/cli` |
| `tiny_http` 0.12.0 crate | sha256 `389915df…cdc82`, matching the same `CRATES` table and the lock entry the patch script rewrites |
| `rouille` 3.6.2 crate | sha256 `3716fbf5…fcc921`, matching `Cargo.lock` inside the `wasm-bindgen-cli` 0.2.129 crate (`rouille 3.6.2`, `tiny_http 0.12.0`, `ureq 3.4.2`) |
| Chromium / ChromeDriver 156.0.8078.73 | `scripts/chrome-for-testing.json` pins `156.0.8078.73` with `chromedriver_sha256`; the workflow asserts both binaries report that version. Source read at Chromium tag `156.0.8078.73` (`chrome/test/chromedriver/…`, `net/socket/client_socket_pool.cc`, Blink parser/document) |
| Dioxus 0.7.10 (`dioxus-web`, `dioxus-interpreter-js`) | pinned `=0.7.10` in the web crates' `Cargo.toml`s (§6). The interpreter actually built is **not** the crates.io source: root `Cargo.toml:31-33` patches it to `web/vendor/dioxus-interpreter-js-0.7.10`, whose `PROVENANCE.md:10-12` records differences from the published source in `src/unified_bindings.rs`, `src/ts/core.ts`, and the regenerated `src/js/core.js`. Interpreter anchors below cite that vendored tree; `dioxus-web` anchors cite the published 0.7.10 crate (`dioxus-web` is not patched) |

## 2. Anatomy of one launch: every wait and its bound

**Demonstrated-from-source.** The runner (crate path
`src/wasm_bindgen_test_runner.rs`, `rmain`) reads the wasm file, then sets the
whole timeout budget in one place:

```rust
// src/wasm_bindgen_test_runner.rs:314-321
let test = timeout_from_env("WASM_BINDGEN_TEST_TIMEOUT", 20);
// Browser launch and page load follow the drivers' own default limits, but
// never undercut a raised test timeout.
let timeouts = headless::Timeouts {
    driver: timeout_from_env("WASM_BINDGEN_TEST_DRIVER_TIMEOUT", 5),
    startup: Duration::from_secs(60).max(test),
    page_load: Duration::from_secs(300).max(test),
    test,
};
```

BoardStudio sets `WASM_BINDGEN_TEST_TIMEOUT=120`, and nothing in the repo or
workflow sets `WASM_BINDGEN_TEST_DRIVER_TIMEOUT`, so locally: `test=120`,
`startup=120`, `page_load=300`, `driver=5`. Phases in `headless::run`
(`src/wasm_bindgen_test_runner/headless.rs`):

| Phase | Bound | On expiry |
| --- | --- | --- |
| Bindgen generation into tmpdir, before the server starts (`wasm_bindgen_test_runner.rs:326-386`, `Bindgen::generate`) | **none** | — (CPU work only) |
| Driver spawn / port bind loop (`headless.rs:83-140`) | `driver` = 5 s total, 10 s per attempt | `bail!` → launch fails |
| `POST /session` (`headless.rs:166`) | `startup` = 120 s client timeout | error via `?` → fails |
| `GET /session/{id}` for the browser name (`headless.rs:168-170`, `740-746`) | 120 s | **error swallowed** (`debug!` log, returns `None`) |
| `POST /session/{id}/url` — Navigate (`headless.rs:197`, `Client::goto:673-687`) | `page_load` = 300 s **client-side** ureq `timeout_global` (`with_timeout:852-853`) | error via `?` → fails (but see the legacy-response hole, §5) |
| Poll `#output` every 100 ms for `test result: ` (`headless.rs:215-249`) | `test` = 120 s from loop start; each poll's `/execute/sync` budget is the remainder | falls out of the loop, then `bail!("some tests failed")` if no `test result: ok` (`headless.rs:297-325`) |
| Window close on drop | 5 s (`CLEANUP_TIMEOUT:509`) | warning only |

Every HTTP call goes through `Client::doit:805-826`, which applies
`with_timeout(...)` and bails on a non-200 HTTP status. Two consequences:

1. **Demonstrated-from-source** (the bounds), **hypothesis** (the location).
   A passing launch cannot contain an expired runner timeout (the swallowed
   browser-name GET is the sole exception, capped at 120 s). Whatever consumed
   285–322 s ended *before* its bound. Navigate is the only *single* runner
   phase whose bound reaches that range, and the post-Navigate poll phase is
   capped at ~120 s and fails closed without output. That does not by itself
   prove location: several sub-bound phases could stack (e.g. session
   creation and the swallowed GET inside their 120 s budgets), bindgen is
   unbounded (below), and the measured quantity being localized is the whole
   `wasm-pack test` launch wall minus in-page test time
   (`scripts/check.py:132-150 (browser)`), including pre-runner time this
   table does not bound (the research measured per-launch cargo steps at
   0.15–0.68 s, a measurement, not a source bound). "The wait is in Navigate"
   is therefore an elimination over source bounds *plus* measurements — the
   leading hypothesis §7 timestamps, not a demonstrated location.
2. **Demonstrated-from-source.** The runner reads no test output until Navigate
   returns (the poll loop starts after `goto`), and its human phase markers
   (`Visiting …`, `Waiting for test to finish…`) go through
   `Shell::status`, which is a no-op off a TTY
   (`src/wasm_bindgen_test_runner/shell.rs:16-19`) — why the hosted logs cannot
   localize the wait inside a launch.

**Hypothesis (exclusion, not proof).** The bindgen phase is formally unbounded
and could in principle absorb time. It does not fit the measurements: split
launches of one binary (`ui-shared` 94/2 tests, `layout` 281/2) carry the same
~300 s, the slow set flips between attempts (`parts` 73.5→336.8 s wall,
`keycaps` 300.2→20.5 s), and no launch in either attempt lands between 47 s and
285 s of overhead — a timed wait's signature, not CPU work's. The experiment
in §7 timestamps this phase anyway.

## 3. `pageLoadStrategy: eager` — applied, and not able to shorten this wait

**Demonstrated-from-source (end to end).**

- BoardStudio generates `{ "goog:chromeOptions": {…}, "pageLoadStrategy":
  "eager" }` in `scripts/run-wasm-tests.py:512-518
  (desktop_webdriver_config)` and the `browser` step's environment carries it
  to every crate launch (`scripts/check.py:1231-1237 (run_plan)`,
  `scripts/check.py:1269-1278 (run_step)`).
- The runner reads that file into a capabilities map
  (`headless.rs:150-162`) and for Chrome sends it in the **legacy** envelope
  `{"desiredCapabilities": cap, "requiredCapabilities": {}}`
  (`headless.rs:47-52 (LegacyNewSessionParameters)`, `headless.rs:603-625
  (Client::new_session)`).
- ChromeDriver 156 classifies that as legacy mode
  (`GetW3CSetting`, `session_commands.cc:151-154`: no `capabilities` key plus
  a `desiredCapabilities` key ⇒ `w3c_compliant=false`) but still calls
  `Capabilities::Parse` (`session_commands.cc:492-505`), whose parser map
  registers `pageLoadStrategy` unconditionally (`capabilities.cc:1370`), and
  `ParsePageLoadStrategy:692-700` accepts `eager` (default is `kNormal`,
  `capabilities.cc:1328`).
- The strategy reaches the tracker on the local-desktop path these sessions
  use: `LaunchDesktopChrome` (`chrome_launcher.cc:464`) passes
  `capabilities.page_load_strategy` to `ChromeDesktopImpl`
  (`chrome_launcher.cc:775`) → `WebViewImpl::CreatePageLoadStrategy`
  (`web_view_impl.cc:577-578`, `NavigationTracker(…, /*is_eager=*/true)`) →
  pending navigation ends on `Page.domContentEventFired`
  (`navigation_tracker.cc:281`) / `readyState == "interactive"`
  (`navigation_tracker.cc:416-417`), instead of `load`/`"complete"`.

So the legacy envelope does **not** drop the strategy. What `eager` cannot do
is return Navigate before `DOMContentLoaded`, and this test page holds
`DOMContentLoaded` hostage to its module graph:

- The served page ends with `<script src='run.js' type=module></script>`
  (`server.rs:693-696`, template `index-headless.html:49`), and `run.js`
  statically imports the generated glue (`server.rs:398-411`,
  `from './{module}'`), which in turn statically imports any JS snippets the
  test binary's `#[wasm_bindgen]` externs declare.
- Module scripts are deferred scripts; Blink runs deferred scripts before
  finishing parsing, and only then dispatches `DOMContentLoaded`
  (`HTMLDocumentParser::AttemptToRunDeferredScriptsAndEnd`,
  `html_document_parser.cc:1072-1079`, returns without `end()` until deferred
  scripts execute; `Document::FinishedParsing`, `document.cc:8040-8054`,
  dispatches the event ChromeDriver's tracker listens for). A snippet fetch
  that never answers therefore blocks `DOMContentLoaded` exactly as it blocks
  `load`.

**Conclusion (demonstrated-from-source).** `bf6ba7288`'s model — "a page slow
to reach `load` stalled launches; eager returns at DOMContentLoaded, leaving
the wait under the test timeout" — is wrong for a stall inside the module
graph: eager applies to this Navigate, and the stall sits in front of the
earlier endpoint too, in front of the 120 s test budget as well (that budget
starts only after Navigate returns, §2). Eager should stay or go on other
grounds; it is not a lever on this wait.

## 4. The `tiny_http` patched pair: relevant mechanism, unproven trigger

**Demonstrated-from-source (server stack).** The runner's HTTP server is
`rouille::Server::new` (`server.rs:8,674`). Rouille 3.6.2 builds
`tiny_http::Server::http` (`rouille/src/lib.rs:364`) and defaults to a
thread-per-request executor (`Executor::default`, `lib.rs:316-322`;
`Server::process:549-552`), so Rouille's own dispatch is not the bottleneck.
The bottleneck candidates live in `tiny_http`:

- Its accept thread dispatches **one task per TCP connection** to a `TaskPool`,
  and the task iterates every request on that connection
  (`tiny_http/src/lib.rs:288-339`).
- Iterating a connection blocks reading the next request
  (`ClientConnection::next`, `tiny_http/src/client.rs:172-187`); the crate sets
  no socket read timeout (no `set_read_timeout` anywhere in `tiny_http/src`),
  and an HTTP/1.1 connection stays open unless the client sent
  `Connection: close`/upgrade or HTTP/1.0 (`client.rs:251-258`). Chrome's
  keep-alive connections therefore each occupy a pool worker, blocked on a
  socket read, for as long as Chrome holds them pooled.
- The pool pre-spawns `MIN_THREADS = 4` workers
  (`tiny_http/src/util/task_pool.rs:30,60-62`). Its spawn decision is the line
  BoardStudio's patch rewrites:

```rust
// tiny_http/src/util/task_pool.rs:69-77 (TaskPool::spawn), unpatched
let mut queue = self.sharing.todo.lock().unwrap();
if self.sharing.waiting_tasks.load(Ordering::Acquire) == 0 {
    self.add_thread(Some(code));
} else {
    queue.push_back(code);
    self.sharing.condvar.notify_one();
}
```

Workers pop **one** task per wakeup, register as `waiting_tasks` only while
about to wait, and wait indefinitely while `active <= MIN_THREADS`, else on a
5 s timeout (`task_pool.rs:96-126`).

**Demonstrated-from-source (the code can strand a connection; an interleaving
exists).** With 4 idle workers, five connections accepted faster than the
workers are scheduled (the hosted case: four launches × Chrome + runner on
four CPUs): the first four `spawn`s queue a task and `notify_one` each —
`waiting_tasks` still reads 4 because no worker has run yet. The fifth `spawn`
sees `waiting_tasks != 0`, queues its connection, and its `notify_one` finds
no sleeping worker left (all four already signalled): the notification is
lost and no new thread is created. Each worker then pops one connection and
blocks in its keep-alive read. The fifth connection's request bytes are never
read — Chrome waits for a response that can only come when one of the four
running connections *ends*. This is an indefinite stall of one HTTP request
to the test server, triggered purely by connection-arrival timing.

**Demonstrated-from-source (the patch).** `scripts/build-patched-wasm-bindgen.py:33-34`
rewrites the condition to `waiting_tasks <= queue.len()`: spawn a fresh thread
whenever every idle worker already has a queued task claimed for it. In the
interleaving above the fifth spawn creates a thread instead of queueing, so
this stranding class cannot form. The pair is used only when
`BOARDSTUDIO_WASM_BINDGEN_DIR` is set (`scripts/wasm_bindgen_tools.py:9-15
(configure)`); the workflow never sets it, and neither hosted log prints
`wasm-bindgen pair:` (measurements digest), so A1/A2 ran the unpatched pool.

**Unknown (origin).** `git show 9ba41e4a6` — the commit that added the patch
script, its test, and `wasm_bindgen_tools.py` — is a WIP checkpoint ("WIP:
checkpoint browser build and Node-free tooling work") with no rationale in the
message or the script. Why the author chose this condition is not recorded;
the analysis above stands on the code, not on recovered intent.

**Hypothesis (hosted trigger).** That this stranding is what stalled the nine
slow launches per attempt — one module-graph request on a stranded fifth+
connection, holding `DOMContentLoaded` (§3), released when some pooled
connection eventually closed — fits every measurement (bimodality,
flipped slow sets, fast four, §6). It is not demonstrated: nothing in the
hosted logs records a stranded connection, and the release event is
unidentified (§5).

**Relevance verdict.** Relevant, precisely scoped: the patched pair is the
candidate fix for the only indefinite stall in the server source. This spike
neither adopts nor deletes it; adoption evidence is the experiment's job (§7).

## 5. Three 300 s constants, and why none is demonstrated as the release

**Demonstrated-from-source.** "300 s" is three different timers that happen to
share a value:

1. The runner's client-side Navigate budget,
   `page_load: Duration::from_secs(300).max(test)` (§2), with the comment
   "Browser launch and page load follow the drivers' own default limits"
   (`wasm_bindgen_test_runner.rs:315-316`).
2. ChromeDriver's default page-load timeout,
   `Session::kDefaultPageLoadTimeout = base::Seconds(300)` ("The default
   timeout values came from W3C spec", `session.cc:106-109`;
   `capabilities.h:126-128`). Navigate uses it via `ExecuteGet` +
   `WaitForPendingNavigations` (`window_commands.cc:728-736,776-785,805-812`);
   on expiry ChromeDriver sends `Page.stopLoading`, waits up to 10 s
   (`kWaitForNavigationStopSeconds`, `web_view_impl.cc:63`) for the stop, and
   still returns `kTimeout` (`web_view_impl.cc:1678-1698`).
3. Chrome's used-idle-socket lifetime,
   `g_used_idle_socket_timeout_s = 300` ("The maximum duration, in seconds, to
   keep used idle persistent sockets alive", `net/socket/client_socket_pool.cc:41-42`)
   — the timer that would eventually close a pooled keep-alive connection and
   free a `tiny_http` worker in the §4 interleaving.

**Demonstrated-from-source (a masking hole).** A ChromeDriver `kTimeout`
would not necessarily fail the launch. In legacy mode ChromeDriver answers
**HTTP 200 even for errors**, encoding the failure only as `"status": 21`
(`kTimeout = 21`, `status.h:23`) in the JSON body
(`HttpHandler::PrepareLegacyResponse`, `server/http_handler.cc:1409-1447`;
the W3C twin maps `kTimeout` to HTTP 500, `http_handler.cc:1528-1531`). The
runner's `doit` checks only the HTTP status (`headless.rs:818-820`), and
`goto` deserializes the body into an empty `Response {}`
(`headless.rs:673-687`), discarding the body's `status`. So a timed-out
Navigate can be recorded by the runner as success, after which polling begins
on a page whose load was stopped. Whether this fired in A1/A2 is **unknown** —
the response body was not retained (it only reaches the log via a `debug!`
line, `headless.rs:824`, which the hosted run did not enable).

**Unknown (the release), with source arithmetic against both expiries.**
A passing slow launch's Navigate returned successfully to the runner *and*
the page then ran its tests. Neither clean expiry explains that across the
eighteen slow launches:

- If release were ChromeDriver's expiry (2) rescued by the masking hole,
  Navigate alone would take ≥300 s of wall before any test ran — but
  `library` A2 overhead is 285.3 s and `keymap` A2 is 285.0 s, and overhead
  includes every phase before Navigate. ChromeDriver's clock also starts
  after the runner's ureq clock (request in flight first), and its
  stop-cleanup runs after expiry, so its response would normally arrive
  after the runner's own 300 s client deadline — a client timeout, i.e. a
  failed launch, not eighteen passes.
- If release were idle-socket expiry (3) freeing a §4 worker exactly 300 s
  after a socket went idle inside the same Navigate, Navigate would again
  exceed 300 s from its start and hit one of the two deadlines above.
- What the arithmetic does say: the stalled request was answered (or the page
  otherwise reached `DOMContentLoaded`) *before* both 300 s Navigate
  deadlines, typically after a wait of roughly 240–300 s (slow overhead minus
  the 11–47 s fast-launch overhead band). **Which event, on which clock,
  releases it in that window is not identifiable from source and the retained
  logs.** Candidates the source cannot rank: a pooled connection closing for
  a Chrome-internal reason before its idle deadline (socket-pool pressure,
  request reprioritization), CPU-starved timer/event delivery on the 4-CPU
  runner shifting effective deadlines, or a different request altogether
  stalling at a different point than §4's interleaving assumes.

This is the point at which source stops. It is also why the research
hypothesis stays a hypothesis: its "waited out a ~300 s wait" does not survive
contact with the three separate clocks as a demonstrated cause, and even
the wait's *location* (Navigate, in front of `DOMContentLoaded`) is, per §2,
an elimination-grade hypothesis rather than a demonstration.

## 6. The fast four: no `dioxus-web` JS in the page

**Demonstrated-from-source (dependency difference).** All twelve crates use
the same `run_in_browser` mode, the same generated config (§3), and the same
page template. Exactly the fast four lack the Dioxus web renderer on wasm32:

| Crate | wasm32 `dioxus` features | `dioxus-web` |
| --- | --- | --- |
| `host` (`web/crates/host/Cargo.toml:18,24`) | no Dioxus at all | no |
| `runtime` (`web/crates/runtime/Cargo.toml:16,24`) | no Dioxus at all | no |
| `catalogue` (`web/crates/catalogue/Cargo.toml:9,15`) | no Dioxus, no direct `wasm-bindgen` | no |
| `ui-model` (`web/crates/ui-model/Cargo.toml:17`) | `["minimal"]` only (all targets) | no |
| the other eight: `case:26-27`, `keycaps:22-23`, `keymap:22-23`, `layout:28-29`, `library:23-24`, `parts:26-27`, `pcb:25-26`, `ui-shared:24-25` | `["minimal", "web", "mounted"]` | **yes** (`["mounted"]`) |

`dioxus-web` 0.7.10 depends on `dioxus-interpreter-js` with features
`["minimal_bindings", "webonly"]` requested unconditionally in its
dependency table (`dioxus-web/Cargo.toml:116-121`), and the vendored
interpreter (§1) gates its three wasm-bindgen JS-module externs exactly
behind those features: `web/vendor/dioxus-interpreter-js-0.7.10/src/lib.rs:28`
(`#[cfg(all(feature = "minimal_bindings", feature = "webonly"))]`), externs
at `:35,41,56` (`module = "/src/js/set_attribute.js"`, `hydrate.js`,
`patch_console.js`). Two oft-quoted Dioxus snippet sources do **not** apply
under the manifests traced here and are not part of this claim:
`dioxus-web/src/document.rs`'s `eval.js` externs compile only under its
`document` feature (`dioxus-web/src/lib.rs:21-22`), and its
`src/lib.rs:102` `inline_js` sits in a `#[cfg(feature = "hydrate")]` block
(`:96`); neither the direct `dioxus-web` dependencies
(`default-features = false, features = ["mounted"]`, table above —
`mounted` enables only `web-sys` items, `dioxus-web/Cargo.toml:67-72`) nor
the facade chains (`dioxus` features `web:153-160` = `dep:dioxus-web`,
`mounted:126` = `dioxus-web?/mounted`, `minimal:115-121`) enable `document`
or `hydrate`. A full resolved-feature proof over every crate in the unit
graph was not performed (no builds); §7 reads the actual requests off a run. For the `--target web` output the
runner generates (`b.web(true)`, `wasm_bindgen_test_runner.rs:340-347`;
`generate` at `:375`), bindgen writes every such extern's JS into the output
directory as a snippet (`wasm-bindgen-cli-support 0.2.129`, crate sha256
`a0724352…cc65a` per the cli `Cargo.lock`: `src/lib.rs:623-632`
(`local_module_name`/`inline_js_module_name` → `./snippets/…`), `src/lib.rs:825-842`
(write snippets into `out_dir`)) and the glue module statically imports them
from there; the runner serves that whole tmpdir (`server.rs:740-820 (try_asset)`).

**Hypothesis (page consequence; generated glue not inspected — no builds in
this spike).** The eight eligible crates' pages therefore fetch a *fan-out* of
snippet modules when the glue module loads — several requests Chrome issues
in parallel on separate connections — while the fast four's pages fetch a
short sequential chain (`run.js` → glue → `_bg.wasm` fetch), reusing one or
two connections. Only the fan-out can present the server with the >4
near-simultaneous connections §4's stranding requires, and only while CPU
contention delays worker scheduling; that is the shape of "sometimes slow,
flips between attempts" for `parts`/`keycaps`, and of never-slow for the
four. Note every launch dev-depends on `web/crates/test-floor`, which depends
on all twelve crates, and it exists precisely so that every launch resolves
*one equal feature set* (`web/crates/test-floor/Cargo.toml:8-10`;
`scripts/check.py:96-105 (BROWSER_PREBUILD)` compiles all launch binaries in
one cargo invocation). Feature unification therefore cuts against reading
resolved features off any one manifest: what §6 traced is that no traced
chain turns on `document`/`hydrate`, while the interpreter externs ride on
`dioxus-web`'s unconditional dependency features. Whether those externs are
*reachable* (hence present as imports) in any given compiled test binary —
fast four included, given test-floor dev-dependencies — is not established
by manifests at all; it is §7's net-log that enumerates the real requests,
and §8 leaves it unknown until then.

Ruled out as the common factor: the compile-time Tailwind crates
(`CSS_TEST_CRATES`, `scripts/check.py:109`) are only four of the eight; the
per-crate `webdriver.json` files exist only for `layout`/`ui-shared` and are
not read on the check.py path (the generated config wins via
`WASM_BINDGEN_TEST_WEBDRIVER_JSON`, §3); test counts (2 vs 281) do not track
slowness in either direction.

## 7. The one discriminating experiment (needs approval)

**Approval needed:** explicit user approval to build an instrumented runner,
run headless Chrome, and spend **up to three** hosted (or hosted-equivalent
4-CPU) batches, stopping at the first that meets the stopping condition
below. This spike ran none of that. The experiment applies **no fix** — in
particular it does not use the patched pair — so its result stays readable as
evidence about the current code.

**Design (one instrumented batch).** Re-run the existing 14-launch browser
batch (`check.py browser`, unchanged crates/filters/counts) under hosted-like
contention (4 CPUs, default `BOARDSTUDIO_CHECK_JOBS`), with the exact
0.2.129 runner rebuilt with **logging only** added, and Chrome/ChromeDriver
logs captured per launch:

1. Runner phase timestamps (monotonic, per launch): bindgen start/end, server
   spawn, driver spawn, `POST /session` sent/returned, Navigate sent/returned
   **with the raw response body logged** (the legacy `"status"` field, §5),
   first poll, `test result:` seen.
2. Server connection log: in `TaskPool::spawn`, per accepted connection —
   timestamp, `waiting_tasks`, `queue.len()`, branch taken (new thread vs
   queued) — plus task start/end and per-request received/responded in the
   Rouille handler. This is the direct observable of §4's stranding.
3. Browser network log: one Chrome `--log-net-log=<path>` **per launch**,
   giving per-request socket assignment, send, first byte, complete, and
   socket close/reuse with timestamps; plus ChromeDriver verbose logging
   for `Page.stopLoading`/timeout events. The per-launch path cannot come
   from BoardStudio's config — `scripts/check.py:1231-1237` builds one
   generated config/environment for the whole plan and
   `scripts/run-wasm-tests.py:512-518` writes a single shared
   `webdriver.json`, while up to four launches run at once on a 4-CPU runner
   (`check_jobs`, `scripts/check.py:279-284`), so a shared `--log-net-log`
   path would collide across simultaneous Chromes. Instead the instrumented
   runner itself appends a unique argument per session — it already mutates
   `goog:chromeOptions.args` when building the Chrome session
   (`Client::new_session`, `headless.rs:603-618`) — e.g.
   `--log-net-log=/…/net-log-<runner-pid>-<server-port>.json`, in a directory
   passed by one new environment variable, and logs the chosen path beside
   its phase timestamps.

**Hypotheses and the observations that settle them** (read off one slow
launch's three aligned logs):

| # | Hypothesis | Settling observation |
| --- | --- | --- |
| H1 | §4 stranding: a module-graph request sits on a queued, never-started `tiny_http` connection | Server log shows a connection accepted and **queued (not new-thread) with no task-start** while Chrome's net-log shows its request sent and unanswered; its task-start coincides with another connection task's end (socket close in net-log), its response precedes `DOMContentLoaded`/Navigate return by seconds, and Navigate's body is `"status": 0`. The specific close event adjacent to task-start names the release. |
| H2 | ChromeDriver page-load expiry releases the wait (masked by the legacy hole, §5) | ChromeDriver verbose log shows the pending-navigation wait expiring and `Page.stopLoading` firing at ~300 s, and the instrumented runner records *either* a Navigate response body with `"status": 21` accepted as success (the §5 masking) *or* its own ureq client timeout — per §5's clock arithmetic the ChromeDriver response normally arrives after the runner's deadline, so the body may never be seen and the runner-side log alone cannot settle this row; the ChromeDriver-side log is the settling record. Meaning if settled: the release was ChromeDriver expiry **of a `DOMContentLoaded`-blocking stall** (under eager, per §3, `load`-only resources cannot produce this `kTimeout`), so H2 is a release-mechanism variant of H1-class stalls, not an eager/strategy fix surface; and a launch released this way should then fail in the poll phase unless its module graph had in fact completed, which the logs must also show. |
| H3 | The wait is not in Navigate at all (bindgen/CPU) | Phase timestamps place ≥240 s outside the Navigate window while server and net-logs show every request answered in seconds. Refutes §2's hypothesis-grade exclusion and redirects the epic. |
| H4 | Release is Chrome idle-socket expiry (constant 3) | H1's pattern, with the freeing socket shown in net-log as closed by the client ~300 s after its last response and no earlier close reason; if instead the freeing close is earlier and reason-coded (pool pressure etc.), H4 is refuted and the net-log names the actual reason. |

H1 observed once also demonstrates the patched pair's relevance from evidence
(the patch removes exactly the queued-branch condition logged in step 2),
which is the input the deferred fix entries wait on. Snippet reachability
(§6's hypothesis) is read off the same run's net-log request lists per crate
— no separate build inspection needed.

**Stopping condition.** Stop at the first batch that yields at least one
launch with ≥240 s overhead and complete tri-log capture: read the verdict
off that launch per the table. If three full batches produce no slow launch,
stop and report inconclusive (contention not reproduced) rather than
rerunning further; do not change timeouts/concurrency mid-experiment. All
instrumentation is removed or kept behind the experiment branch; nothing from
it merges as a "fix".

## 8. What remains unknown; the next decision

Unknown: which request stalls in a hosted slow launch; which event releases
it at ~240–300 s; whether the legacy masking hole (§5) ever fired; the patched
pair author's original intent; snippet reachability in compiled fast-four
binaries (expected none, unverified without a build).

The user's next decision: **approve or decline the §7 instrumented experiment**
(one approval covering the instrumented build, browser runs, and up to three
hosted batches stopping at the first conclusive one, log-only). Approving it is the only path this finding offers to a
demonstrated trigger and release; declining leaves the epic with §4's
candidate mechanism as a hypothesis, in which case re-slicing should record
that any patched-pair adoption would be a bet, not a proven fix. No fix,
timeout, or concurrency change is recommended from source alone.
