# Authored Case preview and STEP — candidate 34814

Candidate `frontend-case-geometry-admission-20261004`, source `f551e4e0` (clean build inputs),
served at `http://127.0.0.1:34814/` and `/boardstudio/`; package proof published via
`migration-deliver.py publish-candidate` (COOP/COEP verified). Full build required the
unsandboxed `wasm-pack` step (`cargo install wasm-bindgen` temp dir is read-only in the sandbox).

Journey (Sofle v2 demo copy, built-in browser):
1. Case workbench → Left case assembly → New case body. A default Plate saved
   ("Left PCB plate", PLATE, 3 mm); **Update preview became enabled** and "Exact case geometry ready"
   was shown. This was the 34812 failure (button stayed disabled).
2. Export workspace for Left PCB: Case STEP enabled. Export produced blob `model/step`,
   816,239 bytes, download name `Sofle v2-Left_PCB-case.step` (captured in-page; the pane does not
   expose the downloads folder, file content was not inspected).
3. Right case assembly (unconfigured): Update preview disabled, "Generate a case from the saved keyboard".
   Export workspace for Right PCB: Case STEP shows "Add and generate case geometry in Case. Needs work"
   with no Export button, so Left's output is not offered for Right.
4. Return to Left: Export workspace again offers Case STEP.

Limits: cancel/retry/failed generation states were not re-exercised (retained 34775 control/status
evidence). Generated mechanical ZIP not rerun (34810 verified). Layout Pick-origin result carried
forward from 34812 (`web/src/presentation.rs` unchanged).
