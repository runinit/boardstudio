# Evidence provenance

This folder retains the exploratory public UI evidence and source audit requested at `/tmp/frontend-parity-reset-20261002/case/`.

- Dioxus integration source: `89b1de8a28fdf02db91d972c90a69235bfbbbffb`.
- React reference source: `5a472a9426e6e38993361da402cd4ec730feb369`.
- React origin `http://127.0.0.1:5175/`; Dioxus origin `http://127.0.0.1:34687/`.
- Agent-browser sessions: `case-parity-react-2318ea1a1376` and `case-parity-dioxus-2318ea1a1376`, viewport 1280×577.
- React entered the public REVIUNG41 demo; Dioxus used the public saved REVIUNG41 copy. Document payload equality was not established. Do not treat these captures as acceptance-grade paired fixture proof.
- The candidate profile actions advanced its isolated local document to revision 6 by disabling the mechanical stack, adding one authored Case body and adding one mount. This profile is not an untouched baseline.
- The page error checks were empty. Candidate `/provenance.json` returned 404, so no served provenance manifest is asserted.
- `AUDIT.md` records observable outcomes, source hashes, and untested acceptance gates. `ACTION-TRAIL.md` records visible actions. `source-hashes.sha256` pins inspected source files. `SHA256SUMS.final.txt` pins the retained `/tmp` packet files. No build binaries, Cargo targets or generated assets are included.

This is source/public UI exploration only: no Cargo, source implementation or build command was run for this audit.
