# Renewed documentation audit — final `a49bb798` checkpoint

Read-only review of the live integration worktree at HEAD
`0d9e22ae2f973c3a516a40670c7944441eaa5b30`; the canonical docs below were
modified but uncommitted during review. The report reviews their worktree
contents, not just HEAD. Their hashes at review time were:

| File | SHA-256 |
| --- | --- |
| `.scratch/m1-production/ACCEPTANCE.md` | `024d3aa298a07bee14010bbce5722f838ec1bff218cda05aaa5daa535894041c` |
| `.scratch/m1-production/BUILD.md` | `82bb8da0aea04285ace316afad8684cb97fdd00666ad14a1e563e1fecec5734b` |
| `.scratch/m1-production/HANDOFF.md` | `d65e1e8a2bed57427edeb0130dbe39c33a548179c31e77ccde62e01723b87f96` |
| `.scratch/m1-production/issues/06-acceptance.md` | `0ae591b3d0c14f2cdbf82526281d31bab5faf750b78eecac3b689ce00e0b868f` |
| `docs/migration/RUN.md` | `91cf5f2e28318f14188e3b403469e315fe265cc7bfd89246dac5310707fdbfc2` |
| `docs/migration/m1-production-run.json` | `42189027a221764bc8d1dd463e94dac34cd12dbbc348a3f25a6b4c531ade3bc5` |
| `TODO.md` | `ea7b5e4cc0db8f510e57ee54caa434beb9693403ad4ab4b2041fc06754c90d0b` |

Evidence cross-check used final build provenance and source-check records, final
browser QA README/record, the two exact-source review reports, current paired
pointer records, and the unchanged reference-gate assessment. No source,
canonical doc or browser/build/test state was changed.

## Findings

1. **One acceptance checklist item appears stale.**
   `.scratch/m1-production/issues/06-acceptance.md:15` leaves “Independent
   Standards and Spec reviews have no blocking findings on the final integrated
   candidate” unchecked. The linked current acceptance text says both reviews
   have no findings. Standards reviewed candidate `e1e8606`, Spec reviewed source
   `b50ddbdd`, and the exact-source ledger identifies production source
   `a49bb798`; `git diff e1e8606..a49bb798` has no production-file changes in
   `application`, `web` or `core`. Either mark this item checked and identify the
   source-equivalent review records, or explain what final-candidate review is
   still pending. This does not justify closing ticket 06: the screen-reader,
   performance and resource/material gates remain open.

2. **Ticket 05's “final release” wording needs an artifact ID.**
   `docs/migration/m1-production-run.json:35` says final-release STEP downloads,
   URL cleanup and stale-scope suppression pass, while the current `a49bb798` QA
   README covers startup, case and tab-teardown flows but does not report a STEP
   download scenario. The retained detailed export record is for
   `8f509433`; CAD/export source files are unchanged through `a49bb798`, but
   `web/src/presentation.rs` did change. Clarify that the claim refers to
   `8f509433` and note source reuse, or add current-release STEP evidence if the
   intent is to claim a fresh `a49` browser pass. The current wording is
   ambiguous rather than demonstrably false.

3. **Pointer status and blockers are represented conservatively.**
   The current ledger, RUN, BUILD, HANDOFF and ticket 06 call five-session
   timing in progress; parent coordination reports sessions 1/2 pass without a
   final aggregate. They do not call it accepted. The explicit blockers also
   match the evidence: actual screen-reader work is unavailable; reference
   UI/live comparisons fail and the frozen CAD comparison is ineligible;
   direct Rust `use_drop`/GPU accounting and full STEP/renderer material
   attribution are unperformed. Final QA carefully limits its worker-close
   result to browser target teardown, labels injected storage errors as not a
   real storage-denial test, and records the Sofle Right unready input.

The historical 8f table in `ACCEPTANCE.md` retains its 13-test count under an
explicit 8f heading; the current a49 section and JSON say application 14/web 12.
Those scopes agree. The final QA root/subpath, preference, delayed-open,
slow-Core, case-cancellation and teardown claims are supported by
`release-a49bb798-qa/README.md` and its raw scenario files. A local Markdown file
target check over spec, plan, acceptance, handoff, build, ticket 06, RUN, current
JSON and TODO found zero missing paths; this does not check external URLs or
heading anchors.

## Acceptance state

M1 remains open. The report does not alter canonical state or authorize closure.
