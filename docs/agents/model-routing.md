# Model routing

The user's direct requests take precedence over these defaults. Select from the available catalog; do not switch providers without the user requesting it. If a requested selection is unavailable, report that rather than silently substituting another provider.

- **Default service tier only:** Never pass `serviceTier: "priority"` or `fastMode: true` unless explicitly requested by the user.
- **Tiered reasoning by complexity:** Use **Pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`) as the primary implementation route. When using secondary/fallback routes, use `low` for mechanical cleanups, dead-code/vendor deletion, and simple test ports; `medium` for standard multi-file refactors and routine reviews; `high` for deep architectural cutovers; and reserve `xhigh`/`max` for genuinely stuck or high-risk design problems.
- **Batch merges & selective reviews:** Saturate 3–4 file-disjoint lanes, merge completed waves in one pass (`cargo fmt --all --check && python3 scripts/check.py repo tooling`), and reserve independent **Claude** review delegates (`claude-opus-5-5` / `claude-sonnet-5-5`) strictly for read-only reviews.

| Work | Preferred route |
| --- | --- |
| Deep architectural cutovers & complex multi-stage planning | **Pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`); **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "xhigh"`/`"max"`), **Codex** Sol 6.2 (`medium`/`high`), or **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`/`high`) only when explicitly requested or primary usage is unavailable. |
| Standard multi-file refactors & native test conversions | **Pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`); **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "xhigh"`), **Codex** Terra 6.1 / Sol 6.2 (`medium`), or **Antigravity** Hydrogen (`gemini-4-hydrogen`, `medium`) only when explicitly requested or primary usage is unavailable. |
| Mechanical cleanups, dead-code/vendor deletion & research collection | **Pi** `meta/muse-spark-1.3-contributor` (`thinking: "xhigh"`); **Muse** Spark 1.3 (`muse-spark-1.3`, `reasoningEffort: "medium"`/`"high"`), **Codex** Terra 6.1 (`low`), or **Antigravity** Hydrogen (`gemini-4-hydrogen`, `low`/`medium`) / Neon (`low`) only as fallback. |
| Read-only architectural, diff, and invariant reviews *(reviews only — reserve Claude here)* | **Claude** Opus 5.5 (`medium`/`high`) or Sonnet 5.5 (`medium`) for read-only reviews only; optional **Antigravity** Neon (`high`) wave-end gate audits only when explicitly requested by the user. |

Collect research at low effort, then route synthesis according to its complexity. A long command alone does not justify an expensive model; route diagnosis by the complexity of the problem.

Coordinate delegated implementation with pointers. Specify the exact owned files, expected regression, passing behavior, and concrete done condition or blocker in the implementation handoff. A handoff is complete with a verified change and checks, a reproducible failure and a concrete materially different next experiment, or the exact blocker. After two inconclusive attempts, restate the user goal and blocker, then escalate the evidence and a materially different next experiment. Respect the user's provider choices when escalating.
