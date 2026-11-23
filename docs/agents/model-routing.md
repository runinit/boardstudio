# Model routing

The user's direct requests take precedence over these defaults. Select from the available catalog; do not switch providers without the user requesting it. If a requested selection is unavailable, report that rather than silently substituting another provider.

- **Default service tier only:** Never pass `serviceTier: "priority"` or `fastMode: true` unless explicitly requested by the user.
- **Tiered reasoning by complexity:** Use `low` for mechanical cleanups, dead-code/vendor deletion, and simple test ports; `medium` for standard multi-file refactors and routine reviews; `high` for deep architectural cutovers. Reserve `xhigh`/`max` for genuinely stuck or high-risk design problems.
- **Batch merges & selective reviews:** Saturate 3–4 file-disjoint lanes, merge completed waves in one pass (`cargo fmt --all --check && python3 scripts/check.py repo tooling`), and reserve independent review delegates for high-risk architectural cutovers rather than routine slices.

| Work | Preferred route |
| --- | --- |
| Deep architectural cutovers & complex multi-stage planning | **Codex** Sol 6.2 (`medium`/`high`) first; **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`/`high`) as implementation fallback if Codex usage runs out. |
| Standard multi-file refactors & native test conversions | **Codex** Terra 6.1 (`medium`) or Sol 6.2 (`medium`) first; **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`) as implementation fallback if Codex usage runs out (or Spark 1.3 Contributor via Muse/Pi if enabled). |
| Mechanical cleanups, dead-code/vendor deletion & research collection | **Codex** Terra 6.1 (`low`) first; **Antigravity** Hydrogen (`gemini-4-hydrogen`, `low`/`medium`) or Neon (`low`) as fallback. |
| Read-only architectural, diff, and invariant reviews *(reviews only — reserve Claude here)* | **Claude** Opus 5.5 (`medium`/`high`) or Sonnet 5.5 (`medium`) for read-only reviews only; **Antigravity** Neon (`high`) for wave-end gate audits. |

Collect research at low effort, then route synthesis according to its complexity. A long command alone does not justify an expensive model; route diagnosis by the complexity of the problem.

Coordinate delegated implementation with pointers. Specify the exact owned files, expected regression, passing behavior, and concrete done condition or blocker in the implementation handoff. A handoff is complete with a verified change and checks, a reproducible failure and a concrete materially different next experiment, or the exact blocker. After two inconclusive attempts, restate the user goal and blocker, then escalate the evidence and a materially different next experiment. Respect the user's provider choices when escalating.
