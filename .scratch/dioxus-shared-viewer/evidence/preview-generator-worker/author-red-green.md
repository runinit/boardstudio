# Preview generator worker: reserved-name red/green record

Worker commit: `f6dd310bbf449b64f165780b1a00429d9d4e9d50`.

The repeated-reserved-name regression was added before removing the worker's duplicate-name rejection. Its first targeted run failed with the expected boundary error: `Reserved net table contains duplicates`. This demonstrated that the worker rejected a Core-valid ordered reserved-net prefix before generator output. I then removed only reserved-name uniqueness/nonempty restrictions, retaining safe integer and unique-index validation. The final regression uses the checked-in `ceoloide/battery_connector_jst_ph_2` generator with two distinct reserved indices named `SHARED`; it asserts both reserved entries remain in order and the generated pad references index 1, matching the existing first-match allocator.

Final targeted command: `node --test scripts/web/preview-generator-worker.test.mjs` — 6 passed, 0 failed. `git diff --check` passed before the author commit. The final test suite also covers relative-only packaged imports, no React/Core initialization, ordered jobs and per-job net snapshots, safe unresolved-model path mapping, unsafe revision/index rejection, and correlated bounded errors.

The raw failing command output and pre-fix temporary test version were not retained as files; the failure description above records the observed terminal result and does not claim a preserved log artifact. Independent source reviews and the root's integration 6/6 test result are retained separately in this evidence directory.

Final source hashes:

- `scripts/web/preview-generator-worker.ts`: `6bae601ac2f452c542de9fe91b31a1c07a5ec58bad59efe4813889b43f8a03f6`
- `scripts/web/build-preview-generator.mjs`: `3896278f0022672fc98ed9d6e3c42a5effdfc227c6dfb011e63f3ac642af754e`
- `scripts/web/preview-generator-worker.test.mjs`: `cd6a791d2320eaea9d33995e93ffa843ac04ecc5a20f1bc954897b7572d61961`

No Cargo, browser, packaging integration, or public acceptance claim is recorded here. Runtime/Prepare-Finish integration, page registration, offline packaging, and public model-delivery checks remain open.
