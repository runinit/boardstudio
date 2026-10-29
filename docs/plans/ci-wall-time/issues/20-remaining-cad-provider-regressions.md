# 20: Remaining CAD provider regressions

Status: resolved
Type: task
Parent: [CI wall time map](../map.md)
Related: [Face-completeness replacement](18-face-completeness-replacement.md)

## Question

What replacement approach preserves the seven other packaged-provider regressions' cases,
assertions, package freshness, failure propagation and relevant runtime behavior without Node?

## Work and completion

- Inventory the original scripts and validate the approach against one regression and the actual
  package. Identify where shared or detached buffers and process-memory diagnostics need distinct
  treatment. Compare named cases and failure paths with that original.
- Record the approach or a blocker, then create appropriately sized follow-up tickets for the
  remaining replacements and wire them as blockers of gate verification. Keep each original
  required until its own replacement is validated. No push, merge or hosted CI.
- Establish and validate the collision/shared batched runner foundation and its interface for the
  downstream ports; reconcile the detailed planner contract before resolving this foundation.
  Record focused positive and fail-closed negative evidence and independent re-review, without
  treating the direct single-suite wrapper as the selected gate cutover.

## Answer

Resolved on 2026-10-29 **only for the local seven-regression replacement decision** at
`research/chromiumoxide-web-split@3d816250c5e5658f74d19800a83904c12f0ca517`. The seven provider
scripts are replaced by the registered browser batch. The handoff reports an independent Opus
final recheck found no remaining concrete blockers to this local decision; its
[thread](t3-thread://v2/40166353-280d-4eee-9e39-26bdec8fc383/thread:delegated-task:command%3Amcp%3Afa5e3878-1f9d-4860-9c1d-0bc0fe2cffa0%3Adelegate-task%3A5dec029b-f074-4384-b7b4-c9bef9cef4b9)
was unavailable to this documentation pass. The earlier WIP and pending-review statements below
are historical. See [20a](20a-step-provider-ports.md#answer),
[20b](20b-qualified-analysis-port.md#answer), and [20c](20c-shared-batch-cutover.md#answer).
Complete browser and full gate verification, hosted-action audit and the cold 10-minute target
remain open in [22](22-node-free-gate-verification.md) and
[23](23-migrated-gate-performance-baseline.md).

## Comments

- 2026-10-29: User reprioritized planning and removing all seven provider regressions now; ticket
  19 was a scheduling preference, not a technical blocker, and remains open. Claimed for a local
  collision-analysis pilot on `research/chromiumoxide-web-split@0f5d8ae1`. Keep all original Node
  checks and gate invocations until replacements are validated and independently reviewed; test
  adaptations must preserve meaningful cases and failure handling.

### Collision pilot — 2026-10-29

The user subsequently clarified that safe removal of unnecessary or redundant behavior, including
optional modes, selectors and disk rereads, is authorized without another approval. Assess and
record substantive impacts and preserve meaningful regression detection and safety; incidental
Node formatting need not be retained. No such old behavior has been removed in this pilot.

Sources: `research/chromiumoxide-web-split@0f5d8ae188752065622ed2cd7e3b367fde9640f2` plus the
uncommitted pilot files `cad/scripts/collision-analysis-web.py`,
`cad/scripts/test-collision-analysis-web.py`, and
`tools/browser-harness/src/bin/face_completeness_web.rs`. The original
`cad/wasm/tests/collision-analysis.cjs`, `cad/scripts/cad_build.py` provider list and all gate
invocations are unchanged. The pilot maps the original body, changes its four fixture reads to
on-demand browser reads with SHA-256 transport checks, and reuses the face driver's current strict
CDP lifecycle. The driver accepts the explicit `collision-analysis main` identity without changing
the default face identity. It is opt-in and **not a registered replacement**.

Named-case inventory: the original is one unnamed sequential script, represented as
`collision-analysis/main`, not 66 independently named tests. It contains 66 static assertion
sites, including six `assert.throws` sites (loops execute sites repeatedly). Mapped coverage includes
the contract example and generated bodies, option rejections and invalid deflections, holed plate,
refined PCB, typed arrays and handles, transformed separation at 80 mm, rejected operand metadata,
separated/touching/crossing/nested/open cubes, degenerate triangles, tetra STEP options and handle
releases. The mapping test requires the same ordered assertion tokens and checks source boundaries;
this is an inventory, not an independent proof of semantic equivalence.

Focused local evidence in the implementation checkout's `.evidence/`:

- `collision20-driver-final-test.log`: 3/3 driver unit tests; `collision20-driver-final-build.log`:
  locked driver build. `collision20-final-controls.log`: 4/4 Python cases, including real-browser
  assertion helper controls, the original collision body, and invalid-result rejection.
- `collision20-browser.log`: real Chrome 156 positive on the freshly built WEB package;
  `collision20-negative.log`: the integrated pilot exits 1 with `actual: 80`, `expected: 81`.
  The negative used a read-only bind mount of a single temporary modified script, not an edit to
  the tracked source. `collision20-node-reference.log`: the original script separately passed on
  the same package under Node **outside the gate**, for comparison only.
- `collision20-face-regression.log`: 13 face tests passed and one Node-only comparison was skipped
  with the updated driver. `collision20-package-build.log` records the first failed build attempt:
  `--mode` appeared after `--locked` and was passed to Cargo. The corrected
  `collision20-package-build-retry.log` records the successful isolated build.

Builds used the shared `~/.cache/tmp/boardstudio-build.lock`, an initial process/resource check,
offline Cargo, Node masked, and fresh private targets under
`/home/chris/.cache/tmp/opencode/collision20-7O5DkqIi/`, never the repository's targets. The
`wasm-pack build wasm --target web --out-dir "$T/pkg" --out-name boardstudio_cad_wasm --no-opt
--release --mode no-install --locked` build used cached wasm-bindgen 0.2.129 and a read-only checkout.
The browser runs used bubblewrap with the network unshared, `/usr/bin/node` bound to `/dev/null`,
the new package read-only bind-mounted over `cad/wasm/pkg`, and a privately generated current
input stamp bind-mounted over the checkout stamp. That stamp checks inputs, not package-byte
authentication. The Rust driver was built with `cargo build --manifest-path
tools/browser-harness/Cargo.toml --locked --bin face-completeness-web` in a separate private target.
The Chrome driver uses `--no-sandbox`; bubblewrap provides the recorded external isolation.

Private `strace -f -e trace=execve,execveat` logs are in that directory as `positive.exec.trace`,
`negative.exec.trace`, and `final-controls.exec.trace`: the positive launches Python, the driver,
Chrome/Crashpad and Chrome's `/proc/self/exe`; no Node/npm/npx executable basename was seen. This
does not exclude renamed or embedded runtimes, and builds were not traced. The shared handler
reported its already-classified post-shutdown WebSocket reset in successful runs; other CDP errors
remain fail-closed. No full gate or hosted runtime audit is claimed.

Remaining before review/cutover: independently assess the pilot's restricted `deepEqual` and
`throws` semantics against actual uses and failure cases, fixture-read/output error propagation,
asynchronous completion boundary and the shared driver's changed argument contract. The browser
and Node transport/order differ (async fetch and synchronous output forwarding versus filesystem
and console), and the browser's failure diagnostics need not be identical; none is yet shown safe
for every simultaneous failure. Corrupt-package/missing-fixture, output-transport and detached-error
controls have not been separately run through the collision pilot. The six other originals remain
required and unported here. The old node-removal five STEP ports are reference material only: their
handler drains errors broadly and must not be transplanted. The source inventory from the read-only
planner is: repair accounting 45, zero repair 53, projected line 16, planar seam 18, complete
surface preview 40, collision 66, qualified analysis 99 static assertion sites; qualified analysis
also needs shared/detached buffer and process-memory diagnostic assessment, and zero repair needs
offset-view/immutability assessment. Its proposal to delete manual legacy mode, selectors and disk
rereads was not an implemented decision; evaluate each impact under the clarified authorization.
The planner's source is <t3-thread://v2/40166353-280d-4eee-9e39-26bdec8fc383/thread:delegated-task:command%3Amcp%3Afa5e3878-1f9d-4860-9c1d-0bc0fe2cffa0%3Adelegate-task%3A71c27373-2994-4226-981d-56c6af702cbb>.

Working-tree status at handoff (2026-10-29): implementation branch `research/chromiumoxide-web-split`
at `0f5d8ae1` has the three pilot files listed above (one modified, two untracked). Plan branch
`ci-wall-time-plan` at `66649bfc` has the scoped ticket, map and wiki index modified; its pre-existing
modified `AGENTS.md` and untracked `docs/research/2026-10-23-model-routing-and-allowances.md`,
`memory/2026-10-19.md`, `mutants.out/{caught.txt,debug.log,lock.json,log/baseline.log,missed.txt,mutants.json,timeout.txt,unviable.txt}`
and `opencode.jsonc` were preserved. No commit, push, merge or hosted CI was performed. Ticket 20
remains claimed and unresolved pending independent review and the remaining work.

### Collision review follow-up — 2026-10-29

An independent combined spec/standards review found the pilot's actual assertion shapes covered,
but blocked gate registration on the untrusted development driver override and missing collision-specific
failure controls. It also requested negative tests for the shared driver's argument contract. It did
not require incidental Node assertion compatibility or manual behavior solely for parity. The review
is at <t3-thread://v2/40166353-280d-4eee-9e39-26bdec8fc383/thread:delegated-task:command%3Amcp%3Afa5e3878-1f9d-4860-9c1d-0bc0fe2cffa0%3Adelegate-task%3Ae70fc995-4891-4463-9116-9dc9054a3efc>.

Uncommitted follow-up adds `cad/scripts/collision-analysis-gate.py` and its hermetic tests. The
wrapper selects the current-checkout locked Cargo artifact, overrides the pilot's environment driver,
runs the positive and five required control groups, and rejects skips. Collision-specific controls
cover all four missing fixtures and corrupt-WASM ordering, rejected output and broken stdout,
synchronized detached exception/rejection before completion, invalid results, and helper behavior.
The Rust driver now tests its accepted and rejected identity arguments. Hermetic gate tests also
cover build/missing-driver failure and a spoofed override; runner tests reject nonzero, malformed,
empty and `ok: 1` driver results.

Evidence in the implementation checkout: `.evidence/collision20-review-gate-hermetic.log` and
`collision20-review-runner-hermetic.log` each pass 3/3; `collision20-review-driver-tests.log` passes
4/4 in the private driver target; `collision20-review-gate-browser.log` passes the positive and 5/5
control groups with the earlier fresh package and matching stamp, private target, offline network,
Node masked, and `CAD_COLLISION_WEB_DRIVER=/bin/true` spoofed. The same gate fails on the bind-mounted
80-to-81 mutation with actual 80/expected 81 in `collision20-review-gate-mutation.log`. The successful
gate trace (`collision20-review-gate-browser.strace`) has 171 exec attempts and no Node/npm/npx/
Playwright basename; this cannot exclude renamed or embedded runtimes. The package stamp does not
authenticate package bytes. `cargo fmt --check` passes. No full browser or gate run is claimed.

The wrapper is **not registered**, and all seven Node checks remain required. The new wrapper and
controls need independent re-review, and a registration must verify the actual scheduler path,
lock inheritance, receipt classification, and mutation failure before removal of the original gate
invocation. No claim is made about errors scheduled after the driver's completion boundary. No
commit, push, merge, hosted CI or performance result was produced.

### Shared-batch direction and exception control — 2026-10-29

The user prioritizes removing the seven CAD Node invocations, with parallel implementation after
the common foundation, and permits removing or updating nonessential regressions and redundant
behavior while retaining meaningful failure detection. Record material coverage and behavior
changes; do not silently discard safety checks. Ergogen [19](19-node-free-ergogen-oracle.md) is
not a technical prerequisite for this CAD work, though it remains relevant to the full gate.

The collision pilot is the foundation (A). The intended architecture is a shared, batched browser
runner for the CAD regressions, rather than registering the direct single-suite gate wrapper WIP.
That wrapper and pilot are evidence and reference material, not the selected cutover. Establish the
shared runner contract and fail-closed controls first; then the five STEP ports (B,
[20a](20a-step-provider-ports.md)) and qualified analysis (C,
[20b](20b-qualified-analysis-port.md)) can proceed in parallel; shared integration and cutover (E,
[20c](20c-shared-batch-cutover.md)) follows both. Ticket 20 remains claimed and unresolved until
its foundation is ready; these dependencies do not claim any implementation is complete.

The implementation checkout's `.evidence/collision20-throws-review-gate.log` records the positive
and 6/6 collision control groups passing, including a negative control that replaces `assert.throws`
with a no-op and verifies that its callback not running is rejected. Controls also exercise missing
fixtures/corrupt WASM ordering, output transport, detached errors before completion and invalid
results. This is focused pilot evidence, not independent re-review, batched-runner validation,
registration, a complete browser step, a full gate, a hosted-action audit or a timing result.

Keep both vendor assets at their existing paths and do not edit them during the parallel work:
`ergogen/library/vendor/kicad/3d_models/Panasonic_EVQPUL_EVQPUC.step` and
`ergogen/library/vendor/infused-kim/3d_models/SMD_0805_Capacitor.step`. Coordinate ownership of
shared runner and integration files rather than editing them concurrently. The requested detailed
planner thread was unavailable to this documentation pass; reconcile its full contract before
implementation or ticket closure rather than treating this summary as a substitute.

### Seven-suite cutover WIP and fresh evidence — 2026-10-29

This section records the earlier WIP handoff; the final local decision is in [Answer](#answer).

The later uncommitted WIP in the implementation checkout
`/home/chris/.local/share/opencode/worktree/3ca6f9/chromiumoxide-web-split-experiment`
on `research/chromiumoxide-web-split@0f5d8ae188752065622ed2cd7e3b367fde9640f2` installs
seven `.mjs` provider suites and removes their seven `.cjs` scripts, registering a batched provider
command with a trusted driver and controls. `load-provider.cjs` and the face script remain. This
supersedes the pilot-only state described above, not the need for validation or independent
recheck. Independent Opus review of the final cutover found no proven code defect but requested
fresh evidence; the evidence below still awaits that recheck. Ticket 20 remains claimed, and
[20a](20a-step-provider-ports.md), [20b](20b-qualified-analysis-port.md),
[20c](20c-shared-batch-cutover.md), and [22](22-node-free-gate-verification.md) remain unresolved.

The previously approved ports have material adaptations: the optional planar legacy comparison
was removed; the qualified triangle/exponent filter was removed so both run; Node process-memory
diagnostics became browser WASM/heap diagnostics, while `retainedStorage` assertions remain; and a
redundant Panasonic disk reread was removed. Zero-repair offset sentinel and strict refusal checks,
qualified isolation/shared-buffer checks and the pinned vendor path were strengthened. See the
port tickets for targeted evidence. These are not claims of line-for-line parity.

Evidence below is local and gitignored in the implementation checkout's `.evidence/`; it may not
travel with the branch. The first private build failed trying to install wasm-bindgen
(`t20-final-private-wasm-build.log`). The normal build script succeeded on retry
(`t20-final-wasm-retry.log`), reusing compilation from the earlier failed build, not a cold build.
The handoff reports `--no-opt`, cached wasm-bindgen 0.2.129, a private package/target/stamp,
Node masked, offline execution and the shared build lock; the build log itself omits the command,
mounts and version. The handoff reports that private stamp `2de4f5ca...` matched the computed
current input digest; it does not authenticate output bytes. Fresh hermetic logs
`t20-final-retry-{provider-hermetic,provider-gate,cad-build,face-gate,face-hermetic,scheduler,receipt}.log`
respectively report 6, 5, 16, 7, 7, 40 and 38 passing tests.

`t20-final-scheduler-green.log` records the actual checked-out registered provider command via a
partial `check.run_plan`: all seven suites and four controls passed. The handoff reports an
inherited build lock, a spoofed driver `/bin/true` overridden, Node masked, network isolated,
Chrome for Testing 156 and the fresh private package and matching stamp. `t20-final-scheduler-mutations.log` and
`t20-final-scheduler-mutants/` record `scripts/mutation-check.py`'s baseline and three killed
mutations for their intended actual/expected assertions: collision distance 80→81, zero deflection
.1→.2 and planar face ID 75→76; other suites continued. These are targeted test mutations, not
provider mutation coverage.

These focused results are not a complete browser step, full gate, Node basename or runtime
independence proof, hosted-action audit or 10-minute result. A stamp verifies build inputs, not
package bytes; the shared checkout package remains stale. The three remaining Node tooling `.cjs`
tests, `node --version` and hosted-action runtimes remain unaudited. Full-gate obligations stay in
[22](22-node-free-gate-verification.md); the later local decision in [Answer](#answer) does not
constitute full-gate acceptance.
