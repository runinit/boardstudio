# Chromiumoxide migration feasibility (research)

Planning-only research for [Chromiumoxide browser checks](../plans/chromiumoxide-browser-checks/map.md),
answering issues
[001](../plans/chromiumoxide-browser-checks/issues/001-protocol-compatibility.md) and
[002](../plans/chromiumoxide-browser-checks/issues/002-launch-contract.md).
Issue [003](../plans/chromiumoxide-browser-checks/issues/003-parity-cutover.md) (grilling/cutover)
is intentionally untouched; user decisions stay pending. No production edits, no builds, no pushes.

Destination (user-confirmed, unchanged): replace **only** the Node/Playwright real-page checks
(drag smoke, CAD runtime capture, Graphite acceptance) with a Rust chromiumoxide harness.
The `wasm-bindgen-test`/ChromeDriver runner, all assertions, receipts and CI verdicts are preserved.
Full Node removal is explicitly out of scope (the tooling step keeps Node).

Conventions used below: **proven** = observed in primary-source output quoted here;
**proposed** = concrete mapping derived from primary sources but not yet run;
**unrun** = explicitly not executed in this turn and not claimed.

## 1. Protocol compatibility (issue 001)

### 1.1 What the archived spike actually recorded — proven

Source: branch `prototype/chromiumoxide-spike`,
`scripts/browser-harness-prototype/{RESULTS.md,Cargo.toml,src/main.rs}` (commit `4b18cb5d1`).

- Pinned dependency: `chromiumoxide = "=0.9.1"` (`Cargo.toml`), so
  `chromiumoxide_cdp 0.9.1`, whose `CURRENT_REVISION` is `r1566079`
  (spike `Q1` line; upstream `chromiumoxide_cdp/src/lib.rs`:
  `pub const CURRENT_REVISION: Revision = Revision(1566079)`).
- Both runs (system `Chrome/155.0.8059.79` and CfT `Chrome/156.0.8078.47`,
  CDP `protocol=1.3`) completed every functional step and printed
  `page errors: []` with `Q1 handler/protocol errors: 19`.
- All 19 errors have the same shape (truncated at 400 chars by the spike itself):
  `HANDLER ERROR: data did not match any variant of untagged enum Message:
  {"method":"Network.requestWillBeSentExtraInfo","params":{"requestId":"1171863.3",
  "associatedCookies":[],"headers":{"Accept":"*/*", ...`.
- Functional results were complete despite the 19 errors: role+name lookups
  (`Rows`/`Columns` sliders, `Fit board` exactness, body-vs-SVG scoping),
  trusted Ctrl and non-Ctrl drags (`trusted:true`, `pointerCaptureMidDrag:true`,
  correct offset/stagger state transitions). Nothing the spike consumed failed.

### 1.2 Mechanism — proven from upstream source

- `src/conn.rs` (`Connection::poll_next`): any WebSocket text message that fails
  `serde_json::from_str::<Message<T>>` becomes `CdpError::InvalidMessage(raw, err)`.
  `Message` is an untagged enum, hence the `data did not match any variant` text.
- `src/handler/mod.rs` (handler poll loop): `Err(CdpError::InvalidMessage(..))` is
  **dropped with a `tracing::warn!`** when `HandlerConfig.ignore_invalid_messages`
  is true, and only returned to the handler consumer otherwise.
- `src/browser/config.rs`: the builder default is `ignore_invalid_events: true`
  (mapped to `ignore_invalid_messages: true` in `build()`); the spike is what opted
  out via `.surface_invalid_messages()`. Doc comment on that builder method:
  "The browser handler will return `CdpError::InvalidMessage` if a received message
  cannot be parsed."
- Command responses travel as `Message::Response` and are matched to pending calls
  in `on_response`; an unparseable **unsolicited event** is a different message and
  cannot fail a pending command. The spike's `page.execute(...)` calls all succeeded.

Classification: the 19 errors are **unsolicited-event parse failures** surfaced by an
explicit opt-in, not command failures and not subscribed-event loss. The harness
never subscribed to the Network domain; it consumed Accessibility/Input/DOM/Runtime
commands plus one `Runtime.exceptionThrown` listener, all of which worked.

### 1.3 Exact failing field — unrecoverable from the archive (evidence gap)

The spike formats errors as `format!("{err}: {}", &raw[..raw.len().min(400)])`
(`src/main.rs`), and both `RESULTS.md` transcripts end mid-`headers`. The Serde
`err` half (which variant failed and why) was never recorded. Therefore **the exact
schema mismatch cannot be determined from archived evidence**; any claim naming a
specific field would be speculation.

Plausibility context only (not a diagnosis): the upstream PDL
(`chromiumoxide_cdp/pdl/domains/Network.pdl`) marks
`event requestWillBeSentExtraInfo` **experimental**, with `associatedCookies`,
`headers`, experimental `connectTiming`, optional `clientSecurityState`,
`siteHasCookieInOtherPartition`, `appliedNetworkConditionsId`. Experimental events
drift across Chrome releases, so a Chrome-155/156 payload vs an r1566079 snapshot
mismatch is unsurprising. The separately indexed upstream report of the same
`untagged enum Message` failure class (0.7.0 vs Chromium 140, camelCase/snake_case
field drift) shows the failure class recurs when the PDL snapshot lags Chrome —
but that report concerns a different event and version, and is cited only as prior
art for the failure class, not as this spike's cause.

### 1.4 Version and support facts — proven

- `chromiumoxide 0.9.1` (2026-02-25) is the newest release: crates.io API reports
  `max_version`, `newest_version` and `default_version` all `0.9.1`. There is no
  newer pin to move to; its MSRV is Rust 1.85 / edition 2024, and this machine has
  `rustc 1.98.0` (read-only version probe, no compilation run).
- Local Chrome state re-verified this turn (third corroboration of the earlier
  diagnosis): `Chromium 156.0.8078.47` and `ChromeDriver 156.0.8078.47` agree, so
  version skew is still rejected as a cause. The `~/.config/chromium` dead
  profile-sync-daemon symlink observed in the diagnosis is gone (the path is now a
  listable directory); a full chromium launch was deliberately not re-run here
  (build/lane contention guardrail), so launch health under the default env is
  **unrun**, while the override mechanism itself stays proven by the archived probe.
- The 19-error observation is dated 2026-10-20 against Chrome 155/156. Per the
  ticket's "don't assume historic errors still reproduce", the count is archived
  evidence, not a current claim.

### 1.5 Safe error policy — proposed

- Ship the harness with the **default** (`ignore_invalid_messages = true`); never
  call `.surface_invalid_messages()` in production paths. Unsolicited experimental
  events then degrade to `tracing::warn!` + single-event loss, exactly as today
  Playwright tolerates protocol additions.
- Do not swallow meaningful errors: command failures (`page.execute` returning
  `Err`, response `error` payloads) stay fatal; the `Runtime.exceptionThrown`
  subscription stays fatal (it feeds the existing "page errors fail the run"
  assertions); add a harness-level counter of `InvalidMessage` occurrences to the
  run log so drift is visible without failing green runs.
- Pin `chromiumoxide = "=0.9.1"` together with the CI Chrome major (016 story 22
  already requires this); any Chrome bump re-runs the red probe below before the
  pin moves.
- Go/no-go for protocol: **GO** if the probe shows zero command/subscription
  failures with invalids confined to unsubscribed experimental Network events;
  **NO-GO** if any subscribed-domain event or command (Page, Runtime, DOM,
  Input, Accessibility, Emulation, or Network once the capture consumes it) fails
  to parse or resolve.

### 1.6 Minimal red-capable probe (unrun; recommended next)

A small standalone binary (own manifest/lockfile, like the spike) that launches
with a process-owned temp profile, `goto`s the built page, subscribes exactly the
domains the harness will consume, and records the **full** raw payload plus Serde
error for every `InvalidMessage` (no truncation), alongside a command-error count.
Run against current Chromium 156 and any candidate CI Chrome. Needs the build lock
and a built site; deliberately not executed in this turn.

## 2. Launch/discovery/profile contract (issue 002)

### 2.1 Current real-page consumers — proven (source inventory)

| Consumer | Launcher / gate | Launch shape |
|---|---|---|
| Drag smoke (`scripts/test-drag-interactions.cjs`, `playwright-core`) | `scripts/run-drag-browser.py`, preflight `browser_tools.locate()` in `scripts/check.py` | `chromium.launch({ executablePath: CHROME_BIN \|\| '/usr/bin/chromium', args: ['--no-sandbox','--use-angle=swiftshader','--enable-unsafe-swiftshader'] })`; `browser.newPage({ viewport: 1513x1043 })` plus three extra `browser.newContext({ viewport })` for collision/qualification scenarios; `pageerror` listener fails the run; receipt carries `browser: browser.version()` |
| CAD capture (`cad/scripts/measure-cad-runtime.cjs`, `playwright-core`) | `cad/scripts/measure-cad.py` via `browser_tools.locate()` | `chromium.launch({ executablePath: CHROME_BIN, ...same flags })` (no fallback); `newContext({ viewport, serviceWorkers: allow/block })`; CDP `Network.enable` + `Network.setCacheDisabled({ cacheDisabled: !warmStartup })`; `context.on('response')` with `fromServiceWorker()`; `pageerror` fails the observation; `browser` event logged with version/executable/flags/viewport/cache/SW state |
| Graphite acceptance (`scripts/test-graphite-feedback.cjs`, full `playwright`) | manual against a running dev server | `chromium.launch({ executablePath: CHROME_BIN \|\| '/usr/bin/chromium', ...same flags })`; `newContext({ colorScheme: 'dark', viewport: 1513x1043 })`; mid-run `setViewportSize` (1513/1024/390); `BOARDSTUDIO_SCREENSHOTS` PNG capture; `pageerror` fails the run |

Prerequisite plumbing, all proven from source:

- `scripts/browser_tools.py::locate()`: Node must exist; `node -e "require('playwright-core')"`
  must exit 0 (else `MissingBrowserTool` naming `playwright-core`/`NODE_PATH`); Chrome is
  `CHROME_BIN` else first of `google-chrome`/`chromium`/`chromium-browser` on PATH.
- CI (`.github/actions/web-tools/action.yml`) installs `playwright-core@1.62.1` under
  `$RUNNER_TEMP/web-tools/node` and exports `NODE_PATH`; the full `playwright` package
  Graphite requires has **no pinned provenance in CI** (unpinned-prerequisite gap the
  migration closes by removing the npm dependency entirely).
- `run-drag-browser.py` sets `CHROME_BIN`, a shallow `TMPDIR=~/.cache/tmp` (comment:
  deep paths exceed Unix socket limits), preview URL and receipt env vars, and
  `verify_receipt` enforces checksum, run/head/source/build identity, the 10 check
  names and timing fields. `measure-cad.py` passes only `CHROME_BIN` (no `TMPDIR`).
- Retained WASM path (must not change): `scripts/run-wasm-tests.py` (chromedriver
  discovery, `CHROME_BINARIES`, `desktop_webdriver_config` fallback),
  per-crate `webdriver.json`, `wasm-pack test --headless --chrome` launches in
  `scripts/check.py::browser()`. Committed `web/crates/layout/webdriver.json` is
  pin-less/portable (`git show HEAD:...`); the absolute personal `binary` pin exists
  only in this worktree's dirty lane state — left untouched, still lane-local.

### 2.2 Diagnosis reuse with freshness notes

- Dead `~/.config/chromium` psd symlink causing startup crashes: mechanism proven by
  the archived probe (clean `XDG_CONFIG_HOME` turned the same binary green); the
  symlink itself is gone today (path listable), so the trigger is stale but the
  defense (process-owned profile/config, §2.3) stays warranted. Unrun this turn:
  a full default-env chromium launch.
- `NODE_PATH`/`playwright-core` prelaunch failures: source unchanged
  (`browser_tools.py`, `test-browser-tools.py` untouched), claim stands without rerun.
- Version match 156 corroborated a third time this turn (§1.4).
- Drag Undo assertion failure: separate app bug, retained failure at cutover —
  restated, not re-investigated.

### 2.3 chromiumoxide launch mapping — proven from upstream source

(`src/browser/config.rs`, `src/detection.rs`, `src/browser/argument.rs`, all `main`.)

- Executable: `.chrome_executable(path)`; without it upstream auto-detects via
  `CHROME` env var then `chrome`/`chrome-browser`/`google-chrome-stable`/
  `chromium`/`chromium-browser`/`msedge…` — a different order and set than
  `browser_tools` (which honours `CHROME_BIN`, not `CHROME`, and never Edge).
  The harness must therefore implement its own discovery preserving the
  `CHROME_BIN`-first order and the `MissingBrowserTool` messages, and pass the
  result explicitly. Proposed and uncontroversial.
- Profile/config isolation: `.user_data_dir(path)` maps to `--user-data-dir`;
  without it upstream falls back to a shared `chromiumoxide-runner` temp dir.
  Per-process env overrides go through `.env(key, val)` / `.envs(..)`
  (`process_envs`, passed to the child command which otherwise inherits). The
  spike used a PID-scoped temp profile plus `remove_dir_all` on exit; the
  crashpad path that defeated even explicit profiles under the dead psd mount is
  controlled via `XDG_CONFIG_HOME`, so the contract is: process-owned profile dir
  under the existing shallow `~/.cache/tmp`-style `TMPDIR` **plus** a
  process-local `XDG_CONFIG_HOME` override for the child. Proposed; mechanism proven.
- Flags: `.no_sandbox()` adds `--no-sandbox` + `--disable-setuid-sandbox`;
  SwiftShader flags pass through `.args([...])` as the spike did;
  `DEFAULT_ARGS` already includes `--disable-dev-shm-usage` (matching the lane's
  webdriver args), `--no-first-run`, `--disable-extensions` handling.
  Headless defaults to old `--headless` (`HeadlessMode::True`); the spike ran the
  default successfully — keep it unless a capability needs `--headless=new`.
- Viewport: `.viewport(..)` defaults to 800x600; the spike set 1513x1043 plus
  `.window_size(1513, 1043)`, matching all three consumers' viewports. Proposed:
  same values, owned by the shared launcher (016 story 16).
- Timeouts/diagnostics: `LAUNCH_TIMEOUT` 20s default (`.launch_timeout(..)`
  adjustable); launch failures surface `LaunchExit`/`LaunchTimeout` with captured
  stderr — wire these into the existing `MissingBrowserTool`-style messages and
  keep logging the browser version string (spike proved `browser.version()` works;
  the drag receipt and capture `browser` event both require it).
- Cleanup: close the `Browser` (child process) and remove the temp profile, as the
  spike did; no persistent configuration changes, no unrelated mounts touched.

### 2.4 Requirements split (new harness vs retained WASM runner)

- New harness owns: `CHROME_BIN`-first discovery, Cargo-only prerequisites (drop
  Node/`playwright-core` installs and `NODE_PATH` from the web-tools action),
  process-owned profile + child `XDG_CONFIG_HOME`/`TMPDIR`, shared flags/viewport,
  version logging, child cleanup. It must keep every `BOARDSTUDIO_*` env var,
  receipt shape/verification, CSV/events-log columns and exit codes byte-compatible.
- Retained runner keeps: chromedriver discovery, `CHROME_BINARIES`,
  `desktop_webdriver_config`, per-crate `webdriver.json`, and the
  `wasm-pack test --headless --chrome` invocations — no shared-launch ownership
  with the harness beyond documenting that both need a matching Chrome.

## 3. Parity-risk inventory (input to issue 003 — no decisions taken)

| Current Playwright behaviour | Harness mapping | Standing |
|---|---|---|
| `getByRole` role + exact accessible name (buttons, sliders, Undo) | `Accessibility.queryAXTree` with `backend_node_id` scope | Proven by spike, incl. prefix-exactness (`Fit`→0) and subtree scoping (body vs SVG canvas) |
| `getByLabel`, `getByRole('option')`, `filter(hasText)`, `isVisible`, `inputValue`/`fill`/`selectOption`/`check`/`press` with auto-wait | hand-rolled query + wait-until-true + `Input.dispatch*` | Proposed; auto-wait/actionability semantics are the main fidelity risk |
| Trusted drags (`mouse.move` steps, `down`/`up`, `keyboard.down/up('Control')`, `Escape` cancel, pointer capture, modifier-aware handles) | `Input.dispatchMouseEvent`/`dispatchKeyEvent` with modifier bitmask, in-page capture listeners | Proven for trust/capture/ctrl/state transitions; `Escape`-cancel and Undo sequencing stay script-level assertions, unchanged |
| `pageerror` fails the run (all three checks) | `Runtime.exceptionThrown` listener | Proven subscribed; unhandled-rejection equivalence to Playwright's `pageerror` is unrun — verify before cutover, do not weaken the assertion |
| Capture: `serviceWorkers: block/allow`, `Network.setCacheDisabled`, `response` + `fromServiceWorker()` resource log | `NetworkManager` (`set_cache_enabled`), explicit `Network.setCacheDisabled`, network event listeners; SW block via CDP `Network.setBypassServiceWorker` | Proposed; resource/status/SW-origin columns must compare field-for-field against `events.log` |
| Graphite: dark `colorScheme`, mid-run viewport resizes, `BOARDSTUDIO_SCREENSHOTS` PNGs | Emulation domain (`setEmulatedMedia`, device metrics) + `Page.captureScreenshot` | Proposed; toolbar-centering assertions depend on exact viewport emulation |
| Multiple `browser.newContext` per run (collision + qualification fixtures) | browser contexts / incognito targets per scenario | Proposed; isolation equivalence (IndexedDB `boardstudio-m1-boardstudio` per context) must be demonstrated, not assumed |
| `browser.version()`, receipt checksum/identity/timings, `verify_receipt` | `Browser::version()` (proven by spike) + unchanged launcher verification | Proven, keep byte-compatible |
| Tooling-step protocol self-check of the in-page script under Node `vm` | standalone script file kept runnable without a browser | Unchanged by design (016); Node stays a tooling dependency |

Existing failing app assertion (Part-drag Undo) is a retained failure: it must fail
identically under the new harness, never be deleted or redefined by the migration.

## 4. Recommended probes and go/no-go (for the grilling ticket)

1. **P1 — full-payload protocol probe** (§1.6): needs build lock + built site; red =
   any subscribed/command failure, green = invalids confined to unsubscribed
   experimental Network events. Settles 001's remaining field-level unknown.
2. **P2 — launch-contract probe**: `locate()` parity (missing-piece messages),
   process-owned profile + child `XDG_CONFIG_HOME` under the dead-mount simulation,
   shallow-`TMPDIR` socket behaviour, child cleanup, version logging. Mostly
   runnable without the build lock (no compilation); the dead-mount simulation is
   bounded and kills only owned processes.
3. **P3 — subscribed-domain smoke**: role lookup + trusted drag + `exceptionThrown`
   against the built page on current Chromium 156 (build lock + site build).
   Reuses the spike's approach; do not repeat the whole suite.

Cutover gates (proposed for 003 to settle): P1–P3 green on the same source/build
for old and new runners; receipts carry identical check names and verdicts;
`verify_receipt`, CSV/events-log columns and exit codes unchanged; web-tools action
drops `playwright-core` only after the harness lane is green; the Undo failure (if
still present) appears in both receipts.

## 5. Suggested grilling questions for issue 003

- Which ISO dates pin `chromiumoxide = "=0.9.1"` against which CI Chrome major, and
  who re-runs P1 on a Chrome bump?
- Where does the `InvalidMessage` counter live (run log vs receipt) and does it
  ever fail a run?
- Who owns `CHROME_BIN` discovery error text after `browser_tools.locate()` loses
  its Node half, and what happens to `test-browser-tools.py`?
- Is `Network.setBypassServiceWorker` an acceptable replacement for Playwright's
  `serviceWorkers: 'block'`, verified by which events.log diff?
- How is per-context IndexedDB isolation demonstrated for the three extra drag
  contexts before the old scripts retire?
- What proves unhandled-promise rejections still fail the run under
  `Runtime.exceptionThrown`?

## Sources

Upstream (primary, fetched 2026-10-26):
`https://github.com/mattsse/chromiumoxide` —
`src/browser/config.rs` (builder defaults, `surface_invalid_messages`,
`user_data_dir`, `.env`, `DEFAULT_ARGS`, launch env/stderr),
`src/browser/argument.rs`, `src/detection.rs` (auto-detect order),
`src/conn.rs` (`InvalidMessage` origin), `src/handler/mod.rs` (`ignore_invalid_messages`
branch, `HandlerConfig` defaults), `src/handler/network.rs` (`NetworkManager`,
`set_cache_enabled`), `chromiumoxide_cdp/src/lib.rs` (`CURRENT_REVISION = 1566079`),
`chromiumoxide_cdp/pdl/domains/Network.pdl` (`requestWillBeSentExtraInfo` experimental
schema); `https://crates.io/api/v1/crates/chromiumoxide` (0.9.1 newest, 2026-02-25,
Rust 1.85); `https://docs.rs/.../EventRequestWillBeSentExtraInfo.html` (generated shape).

Repo (all paths relative to checkout root):
`scripts/browser-harness-prototype/` on `prototype/chromiumoxide-spike`
(`RESULTS.md`, `Cargo.toml`, `src/main.rs`, commit `4b18cb5d1`);
`scripts/test-drag-interactions.cjs`, `scripts/test-graphite-feedback.cjs`,
`scripts/qualification-page-fixture.cjs`, `cad/scripts/measure-cad-runtime.cjs`,
`cad/scripts/measure-cad.py`, `scripts/run-drag-browser.py`,
`scripts/browser_tools.py`, `scripts/test-browser-tools.py`, `scripts/check.py`,
`scripts/run-wasm-tests.py`, `.github/actions/web-tools/action.yml`, `README.md`
(Browser tests), `docs/plans/cad-deepening/issues/016-rust-browser-harness.md`,
`docs/research/chromedriver-test-failures.md` (untouched),
`.evidence/chromedriver-diagnosis/probe-results.md`.

Local runtime facts established this turn (no builds, no owned-process residue):
`Chromium 156.0.8078.47` / `ChromeDriver 156.0.8078.47` agreement;
`~/.config/chromium` now a listable directory (psd symlink gone);
`rustc 1.98.0`; committed `web/crates/layout/webdriver.json` pin-less vs dirty
lane-local pin (preserved untouched).
