# Current-candidate axe entrypoint investigation (F5.8 / F3.7)

## Existing retained harness

The retained F1 scan was captured by `.scratch/dioxus-frontend-v1/evidence/capture-matrix.py`. Its invocation shape is:

```sh
python3 .scratch/dioxus-frontend-v1/evidence/capture-matrix.py <agent-browser-session> <url> <output-directory>
```

The script calls `agent-browser --session SESSION ... --json`; the actual axe action is `agent-browser ... a11y --json`. The JSON reports axe-core **4.12.1**, violations, passes, and incomplete nodes. The `url` argument is currently unused: the script changes viewport/theme and captures the already-open tab but does not navigate or validate origin. The retained F1 `visual-final/browser-steps.json` records origin `http://127.0.0.1:34233/`; retained F3a `layout-layers/axe-light.json` records `http://127.0.0.1:34643/`. Both show zero violations and one incomplete color-contrast rule over SVG text; they are historical snapshots, not current PCB or integrated Layout results.

The related `.scratch/dioxus-frontend-v1/evidence/layout-layers/browser-qa.py <session> <url> <output-directory>` explicitly opens the URL and exercises F3a public layer behavior, but it does **not** call `a11y`. Neither Python file is a headless Playwright test or current-candidate server.

## Can the repository headless runner target c1d3:34825?

No existing repository Playwright entrypoint targets an already-served candidate without changing/replacing its web-server configuration. `app/playwright.config.ts` reads `BOARDSTUDIO_TEST_PORT` (default 4328), sets `baseURL` to that port, and always starts `vite preview` with `reuseExistingServer: false`; `app/playwright.pages.config.ts` inherits that server and appends `/boardstudio/`. Setting the port to 34825 would try to start a new Vite preview on the occupied candidate port, rather than attach to the c1d3 candidate. The app Playwright suites also contain no axe scan. Rebuilding TS/reference is unnecessary for direct axe on the candidate, but no in-repo headless test command provides that direct-target path.

## Safe next use and limits

The existing axe mechanism can scan c1d3 only if a separate authorized session is already navigated to the exact served candidate URL and the session is controlled under the browser policy. The parent’s restriction says the current browser tab must be operated only through the native browser client, so do not aim `agent-browser` or Playwright/CDP at that tab. Root can capture axe through an approved isolated/native-browser session, preserving URL, candidate source `c1d3`, workspace/view (PCB populated for F5.8; Layout for F3.7), viewport/theme, and raw violations/incomplete output. Do not count old zero-violation scans as qualification for either current route, and do not interpret axe’s incomplete SVG contrast result as a pass or screen-reader evidence.
