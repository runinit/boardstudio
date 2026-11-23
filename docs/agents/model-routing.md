# Model routing

The user's direct requests take precedence over these defaults. Select from the available catalog; do not switch providers without the user requesting it. If a requested selection is unavailable, report that rather than silently substituting another provider.

- **Default service tier only:** Never pass `serviceTier: "priority"` or `fastMode: true` unless explicitly requested by the user.
- **Tiered reasoning by complexity:** Use `low` for mechanical cleanups, dead-code/vendor deletion, and simple test ports; `medium` for standard multi-file refactors and routine reviews; `high` for deep architectural cutovers. Reserve `xhigh`/`max` for genuinely stuck or high-risk design problems.
- **Batch merges & selective reviews:** Saturate 3–4 file-disjoint lanes, merge completed waves in one pass (`cargo fmt --all --check && python3 scripts/check.py repo tooling`), and reserve independent review delegates for high-risk architectural cutovers rather than routine slices.

| Work | Preferred route |
| --- | --- |
| Deep architectural cutovers & complex multi-stage planning | **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "max"`/`"xhigh"`) or **pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`); **Codex** Sol 6.2 (`medium`/`high`) or **pi** `meta/muse-wonder-full-1.0-contributor` (`thinking: "xhigh"`) as secondary; **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`/`high`) as implementation fallback if primary usage runs out. |
| Standard multi-file refactors & native test conversions | **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "xhigh"`) or **pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`); **Codex** Terra 6.1 (`medium`) / Sol 6.2 (`medium`) as secondary; **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`) as implementation fallback if usage runs out. |
| Mechanical cleanups, dead-code/vendor deletion & research collection | **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "medium"`/`"high"`) or **Codex** Terra 6.1 (`low`); **Antigravity** Hydrogen (`gemini-4-hydrogen`, `low`/`medium`) or Neon (`low`) as fallback. |
| Read-only architectural, diff, and invariant reviews *(reviews only — reserve Claude here)* | **Claude** Opus 5.5 (`medium`/`high`) or Sonnet 5.5 (`medium`) for read-only reviews only; **Antigravity** Neon (`high`) for wave-end gate audits. |

Collect research at low effort, then route synthesis according to its complexity. A long command alone does not justify an expensive model; route diagnosis by the complexity of the problem.

Coordinate delegated implementation with pointers. Specify the exact owned files, expected regression, passing behavior, and concrete done condition or blocker in the implementation handoff. A handoff is complete with a verified change and checks, a reproducible failure and a concrete materially different next experiment, or the exact blocker. After two inconclusive attempts, restate the user goal and blocker, then escalate the evidence and a materially different next experiment. Respect the user's provider choices when escalating.
