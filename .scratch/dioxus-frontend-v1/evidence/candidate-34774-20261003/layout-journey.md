# Layout-ready paired browser journey receipt

Status: **unqualified / unexecuted**. This bounded pass did not establish journey outcomes; it is not Layout parent acceptance.

Pair: React reference `http://127.0.0.1:5175/`, source `5a472a9426e6e38993361da402cd4ec730feb369`; Dioxus candidate `http://127.0.0.1:34774/`, build `frontend-source-batch-20261003-5`, source `5c8161cf80d2c93c6214cb7caed6dffc4463f64d` (run record `current_progress.served_candidate`). Named isolated sessions were `layoutready-react-20261003`, `layoutready-dioxus-20261003`, and `layoutready-dioxus-outline-20261003`; viewport 1440×900, default light theme. Opened the built-in Sofle v2 demo (58 keys, two boards) in React. React baseline is captured in `reference-sofle-layout.png`; candidate open-keyboard baseline is `candidate-open-keyboard.png`.

| Journey | Result | Evidence / limit |
|---|---|---|
| Select a key through the object tree; inspect Properties and Relations; make one key-size/splay edit; Undo; save and reopen | **Unexecuted** | React Sofle canvas baseline captured. Candidate `Start Sofle v2` click returned in the isolated candidate session. A candidate screenshot was then requested, but the enclosing `exec_command` yielded after 10 seconds. A later lightweight title/URL read also yielded. The wrapper printed only command output and discarded each returned `session_id`; I did not retrieve either eventual result. Selection, edit, Undo, save, and reopen outcomes are unknown. |
| Copy/edit generated Outline; operate one perimeter control; Undo; save and reopen | **Unexecuted** | Candidate open-keyboard baseline only. No outline interaction or persistence result was established. |

The browser calls used the agent-browser managed headless Chromium session and its documented 25,000 ms default action timeout. The 10,000 ms `exec_command` yield is not a timeout or failure; because its session ID was discarded, this receipt cannot claim the browser or renderer hung. No new browser retries were made after recognizing the lost IDs.

Cleanup was attempted with `agent-browser close` for the three named sessions under 3-second shell limits: one call printed `Browser closed`; the other calls produced no output before the shell limits. Which sessions remain open is unverified.

No application source, build, or tests were changed or run. Source joins and Layout criteria remain unassessed.
