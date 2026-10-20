# 016: Real-browser checks drive Chrome from Rust

Status: ready-for-agent
Type: spec
Blocked by: none
Parent: [CAD deepening](../map.md)
Related: [015: Browser check runs on the development machine](015-browser-check-runs-locally.md) (superseded)

## Problem Statement

The real-page browser checks are the only part of the toolchain that needs Node Playwright.
The full check's browser step stops on a development machine unless `playwright-core` is on
`NODE_PATH`. CI installs that package separately just for this, and the CAD runtime benchmark
and the Graphite acceptance script need it too, one of them as the full `playwright` package.
A Rust project ends up depending on an npm package, a Node runtime and an extra CI install step
for three scripts, and a developer can't run the full check without setting all of that up.

## Solution

Replace Node Playwright with one small Rust browser harness built on
[chromiumoxide](https://lib.rs/crates/chromiumoxide), which drives Chrome over the DevTools
protocol. The drag smoke, the CAD runtime capture and the Graphite acceptance become binaries
of that harness. They keep their current inputs, outputs and verdicts, so their launchers and
receipts don't change shape. A developer needs only Cargo and Chrome to run `check.py browser`,
and CI stops installing Playwright.

## User Stories

1. As a developer, I want `check.py browser` to run with only Cargo and Chrome installed, so that the full check works on a fresh machine.
2. As a developer, I want the check to tell me when Chrome is missing and how to point `CHROME_BIN` at it, so that I can fix the one remaining prerequisite.
3. As a developer, I want no `NODE_PATH` or npm setup for browser checks, so that local and CI setup match.
4. As a CI maintainer, I want the web-tools action to stop installing `playwright-core`, so that CI has one fewer pinned download and cache entry.
5. As a reviewer, I want the drag smoke's success receipt to keep its fields and verification, so that receipts written before and after the port compare directly.
6. As a reviewer, I want the drag smoke to keep proving trusted pointer drags over Part, Column, Stagger and Ctrl+Offset, so that live preview, pointer capture, saved document state and exact Undo stay covered.
7. As a reviewer, I want case **Collision check** findings in the smoke to count only raw surface observations between a case body and the PCB, as today, so that an unavailable PCB never passes.
8. As a developer, I want page errors collected and failing the run, so that a script error in the page is not hidden behind a passing interaction.
9. As a developer, I want failures to name the check that failed and leave the same failure log, so that I can tell what broke without rerunning.
10. As a CAD performance investigator, I want the runtime capture to accept the same arguments `measure-cad.py` passes today, so that the benchmark workflow is unchanged.
11. As a CAD performance investigator, I want the same CSV columns, events log and exit codes from the capture, so that earlier results stay comparable.
12. As a CAD performance investigator, I want cold runs to disable the HTTP cache and block service workers, and warm startup to allow both, so that the measured conditions are unchanged.
13. As a CAD performance investigator, I want every network response recorded with its status and service-worker origin, as today, so that resource evidence stays complete.
14. As a tooling maintainer, I want the capture's protocol self-check to keep running in the tooling step without a browser, so that protocol regressions are caught cheaply.
15. As a designer, I want the Graphite acceptance to keep its PASS/FAIL lines, dark colour scheme, viewport and optional screenshots, so that feedback rounds compare with earlier ones.
16. As a script author, I want one place that launches Chrome with the project's flags and viewport, so that the three checks don't drift apart.
17. As a script author, I want to find elements by role and accessible name, so that the checks keep addressing the page the way users and assistive technology do.
18. As a script author, I want a wait-until-true helper with a timeout, so that checks wait on page state rather than fixed sleeps.
19. As a script author, I want trusted mouse and keyboard input, so that pointer capture and modifier drags behave as they do for a person.
20. As a maintainer, I want the harness's dependencies kept out of the app's workspace lockfile, so that browser tooling never changes what the app builds against.
21. As a maintainer, I want the harness linted and formatted by the check, so that it doesn't rot outside the gate.
22. As a maintainer, I want the harness pinned to a chromiumoxide version that works with CI's Chrome, so that a Chrome update failing the protocol is caught in one place.

## Implementation Decisions

- **One harness crate** with its own manifest and lockfile, excluded from the main workspace,
  in the same way the CAD worker is. It uses chromiumoxide on the tokio runtime.
- **A shared library module** that the three binaries use:
  - launch Chrome with the existing flags (`--no-sandbox`, SwiftShader ANGLE), a 1513×1043
    viewport, and an optional dark colour scheme;
  - collect page errors;
  - evaluate script in the page;
  - find elements by role and accessible name, exact or not;
  - wait for a page condition with a timeout;
  - send trusted mouse moves, down and up events, key presses and held modifiers through the
    DevTools input domain;
  - take screenshots.

  Each helper exists because one of the current scripts uses it. Nothing more is added.
- **Drag smoke:** a binary of the harness replaces the Node script. It keeps the environment
  contract: the preview URL, receipt path, and source HEAD, source hash and build hash. It
  writes the same receipt, and the launcher's receipt verification is unchanged. The launcher
  builds the harness with `--locked` and runs it instead of Node.
- **Prerequisite probe:** browser tool location checks for Chrome (`CHROME_BIN`, then the usual
  executable names) and Cargo. It no longer looks for Node or `playwright-core`. Its messages
  say how to supply what is missing.
- **CAD runtime capture:**
  - A binary of the harness takes the same positional arguments and writes the same metrics
    CSV and events log, with the same exit codes.
  - Cache, service-worker and response capture go through the DevTools network domain, as
    today.
  - The JavaScript that runs inside the page moves into a standalone script file that the
    harness loads. The tooling step's protocol self-check keeps exercising that file in Node's
    `vm`, with no browser.
  - `measure-cad.py` launches the binary.
  - Node therefore stays a tooling-test dependency, and Playwright does not.
- **Graphite acceptance:** a binary of the harness, with the same PASS/FAIL output, dark
  scheme, viewport and `BOARDSTUDIO_SCREENSHOTS` behaviour. It is still a manual check against
  a running dev server. The check's list of scripts it deliberately doesn't run names the new
  entry point.
- **CI:** the web-tools action drops the `playwright-core` install and `NODE_PATH`. The browser
  job builds the harness, and its build output is cached alongside the other Rust targets.
- **Lint:** the check's lint step formats and Clippy-checks the harness through its manifest.
- **README:** the browser prerequisites become Chrome (or `CHROME_BIN`) and Cargo.
- **Protocol compatibility:** pin chromiumoxide. If CI's Chrome is newer than the protocol the
  pinned version generates, record which DevTools calls the harness uses and confirm each one
  works on that Chrome.

## Testing Decisions

- Test external behaviour at the highest seams that already exist:
  - the browser step's drag smoke and its receipt verification;
  - the capture's protocol self-check, which runs the in-page script under controlled workers;
  - the prerequisite probe's unit tests, rewritten for Chrome and Cargo.
- Harness helpers are tested through the binaries that use them. Add a helper test only where a
  binary can't show a failure clearly, such as role-name matching. Such a test runs against
  headless Chrome on a local static page, inside the browser step only, never in native
  `cargo test`.
- Equivalence: run the old and new drag smoke on the same build at least once, and confirm the
  receipts carry the same checks and verdicts. Record this in the ticket comments.
- Prior art:
  - the browser-tools unit tests, which mock tool discovery;
  - the capture protocol self-check;
  - the drag launcher's receipt verification;
  - the check's tests asserting which commands each step runs.

## Out of Scope

- Changing what any of the three checks asserts.
- The `wasm-pack test --headless --chrome` crate tests. They use wasm-bindgen's own runner.
- Removing Node from the repo: the tooling step still runs Node tests.
- Firefox or WebKit.

## Further Notes

- Runs that build the web page or the harness may need the sandbox lifted. Inside it,
  `wasm-pack` cannot install `wasm-bindgen`.
- Suggested slicing for `/to-tickets`:
  1. the harness library plus the drag smoke, the probe, CI and the README, which is enough to
     unblock the full check;
  2. the CAD runtime capture;
  3. the Graphite acceptance.

  Slices 2 and 3 are blocked by slice 1.
