# Editor workspace ownership — research brief

Approved: 2026-11-15. Validation: normal. File budget: 15 files per question. History budget: 10 commits/investigations total. Read code only; do not run builds or tests.

Repository snapshot: ad9db473012a4a4201217bd5094a4f24e7801932 (`ad9db4730`). Branch: `docs/bmad-ticket-trees` (metadata only; not research evidence). Uncommitted changes are excluded.

## Questions

1. How does production composition work? (how/where) Where are Runtime construction, subscription, reconciliation, providers and App/Editor teardown owned?
2. Where does workspace orchestration actually live? (where/how) For Layout, PCB, Parts, Case, Keymap, Keycaps, setup guide and camera, who owns controllers, projections, admitted actions, surface construction and resources?
3. How do cross-workspace workflows preserve ownership? (how/what touches) Trace shared plan delivery, physical setup, placement, findings, deferred focus and canvas transactions, including cancellation and stale-result rejection.
4. How is that behavior tested? (how/where) Identify composition, mounted and packaged coverage, distinguishing test assertions from executed evidence.
5. Why are the current boundaries and lifetimes this way? (why) Inspect relevant history for partial extractions and deliberate lifetime choices. Date two comparable controller-ownership patterns found by analyzers.

## Research restrictions

Exclude `docs/plans/`, `_bmad/`, `_bmad-output/`, `.agents/`, `.evidence/`, `.scratch/` and other planning/ticket folders from research evidence and searches. Existing plans and the requested outcome are not evidence of code behavior. Code documentation may explain the code; no proposed future work is implementation evidence.

Initial access plan used the exact committed Gortex view. That view remained building; the user subsequently explicitly authorized read-only `git show`, `git grep` and Git history at `ad9db473012a4a4201217bd5094a4f24e7801932`. This approved Git snapshot route is authoritative for researchers. Do not silently use working-copy source. Current research-artifact documentation writes still use guarded Gortex tools. No mutations, configuration changes, staging or commits by researchers. Return relevance-filtered digests, with citations `path:line (symbol) @ ad9db4730`, mark load-bearing claims and name uncertainty. No recommendations.

Never conclude from training data alone. Training knowledge proposes hypotheses only; conclusions require evidence retrieved this run. The research firewall: project context shapes questions, not answers. Researchers receive only neutral brief questions and source pointers; no legacy tickets or desired backlog changes.

## Output boundary

The factual report contains no backlog recommendations. A separate coordinator-owned proposal may compare verified findings with frozen legacy requirements and overlapping plans, explicitly distinguishing source requirements, code facts and proposals. No migration, implementation or commit is authorized.
