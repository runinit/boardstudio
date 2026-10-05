# F9.7 adopted-path local journey

Ran the bounded journey against the coordinator's actual default `pnpm start` instance at `http://127.0.0.1:4173/` (build `start-f4107630-20261005T181117Z-3535061`, source commit `f41076301c592cf7145fbd2125bf7de00e4e7b72`) in isolated agent-browser session `f97-adoption-20261005`. The input was the F96 saved copy `bootstrap-reference-final-20261005/reference-resaved.boardstudio`, SHA-256 `58c0d6303c08dafe4f2259b297d78c619f8c9469163575ff4dcea53c95cf2342`.

The public M1 root worker `/service-worker.js` was active with no waiting/installing successor. The journey imported the archive, renamed `Sofle v2` to `Sofle v2 F9.7 adoption` via the Project name control and Enter/blur, reached saved state, reloaded and read the renamed project, then selected Layout, PCB, Keymap, Keycaps, Case, and Parts via their real tabs. It opened the actual project menu and verified the portable-copy button was enabled and hit-testable before exporting.

The portable copy is `run-start-f4107630/renamed-project-copy.boardstudio`, SHA-256 `937d1fddac0a6c6066a25efdaf607aae378d3d2b87026ba8c9563fe8db2ad861`. It preserves all non-name/revision ProjectDoc fields and all six asset paths/hashes; name is the requested rename and revision is 24 (from 23). Final reload reads the renamed project as saved under active ID `m1-sofle-v2-copy`, still controlled by `/service-worker.js` with no waiting/installing worker. `agent-browser errors` returned an empty list and console messages were empty.

Raw commands and phase observations: `run-start-f4107630/browser-steps.json` and `run-start-f4107630/report.json`. No screenshot was taken. This covers the requested local default adopted path; it is not a hosted production deployment check.
