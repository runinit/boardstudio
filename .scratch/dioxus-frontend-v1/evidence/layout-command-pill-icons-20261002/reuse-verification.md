# Independent page-only reuse build verification

This receipt records a read-only verification of the real page-only reuse build for the Layout command-pill source change. It checks build lineage, maintained-source inventory, staged route file maps, and reused-provider bytes. It does not claim that this isolated branch ran the build or performed the public browser smoke.

## Build identity

- Integration checkout: `/home/chris/.local/share/boardstudio/worktrees/migration-m1-continuation-20261001`
- Candidate: `web/target/builds/frontend-command-icons-reuse-20261002`
- Candidate source commit: `8cfd6bb79e9e10b788e007fd428145b1e37095d1`
- Candidate `provenance.json` SHA256: `af1da7332faad3b4f44f5b67e489155a2e0c2609dd398c2066fe64a6af83689e`
- Candidate status and mode: `complete`, `page-only-provider-reuse`
- Full baseline: `web/target/builds/frontend-workbench-parity-joined-20261002`
- Baseline source commit: `9d34f3672f4f05cb3771bc5cd358508b13e9c31f`
- Baseline `provenance.json` SHA256 recorded by candidate: `4bc856174b33998c5cf99f4c05a8ba35ad7ab4ce7705bfece6510cdd4467544e`

## Checks performed

I compared the candidate’s 22 inherited command rows against the full-build provenance and confirmed their exact label/order, argv, cwd, environment, exit status, log location, and stored log SHA256. Every inherited command exited 0, and each retained log hash matched. The full baseline identity, provenance digest, and route path/hash maps also matched.

I checked the actual eight-command reuse sequence. The rows matched the expected generator, fresh root/subpath page, offline-worker, and embed commands, including argv, cwd, environment, log existence, and successful exit status. All eight logs were present and exited 0. This was a real reuse run with eight fresh commands; the 22 full-build commands are inherited lineage and were not represented as rerun.

The candidate maintained-source map contained 1328 paths and exactly matched a fresh inventory of the checkout at candidate HEAD. Its path set matched the full baseline. The only three content changes were `web/assets/m1.css`, `web/src/presentation/objects/layout_toolbar.rs`, and `web/src/presentation/objects/layout_transform_toolbar.rs`; all three were in the explicit page-only allowlist. No source path or content drift was found.

For both `/` and `/boardstudio/`, the actual staged site path sets and SHA256 maps matched provenance exactly and contained 145 files. Each route’s 126 reused-provider paths and hashes exactly matched the corresponding provider subset from the full baseline. The receipt’s reused-provider maps matched those same 126 entries. Current tool observations matched the full baseline version receipts.

| Sequence | Commands | Recorded first start to last finish |
| --- | ---: | ---: |
| Full baseline | 22 | 471.61 s |
| Page-only reuse | 8 | 104.66 s |

The command-span comparison is about 4.51× shorter for reuse (366.95 seconds less elapsed span). It excludes build setup outside the recorded command sequence.

## Public smoke boundary

After this verification, root reported a separate public smoke on the completed candidate for both routes: controlled offline mode, Saved Rev 9, 24 SVG paths, Snap close/focus behavior, and compact 375px/390px layouts without document overflow. Root associated that smoke with metadata commit `34614d1a`. That browser interaction is root-reported here and was not part of my independent receipt verification.
