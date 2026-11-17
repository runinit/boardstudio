| Component | Failure | Effect | Detection | Prevention |
|---|---|---|---|---|
| Cache key | Omits a build input or includes unrelated workspace changes | Incompatible reuse or unnecessary rebuilds | Independent input-change and unrelated-change fixtures | Define and test the complete input set |
| Compatibility check | Runs after restore or accepts an incompatible lock | Browser verification may use incompatible tools | Prospective cache hit with a mismatched lock | Validate before cache lookup |
| Restored pair | One executable is missing, broken, or wrong version | Consumer fails or uses the wrong tool | Damage each executable independently | Verify both executables before use |
| Producer overlap | Asset is consumed before generation completes | Packaged checks fail or use stale assets | Identify first consumers and monitor missing assets | Overlap only a producer with a proven dependency boundary |
| Matched experiment | Cache state, target layout, or coverage differs | Timing comparison or recommendation is misleading | Record conditions and exact test outcomes | Match conditions and retain isolation and gate inventory |
| Recommendation | Apparent gain is within run-to-run variation | A more complex schedule provides no reliable benefit | Reverse-order matched pair and variation comparison | Retain sequential schedule when evidence is inconclusive |

## Findings

1. **Story, requirement 2:** “Relevant Cargo configuration” leaves the cache input boundary undefined, making completeness difficult to verify. Require the maintainer to enumerate the actual build inputs and document why excluded workspace inputs cannot affect the pair.
2. **Spike, approach 4 and acceptance criterion 2:** The reverse-order pair is optional, but adoption requires a repeatable gain larger than observed variation. Require the reverse-order pair whenever adoption is recommended; otherwise close with the sequential schedule.
3. **Spike, approach 2:** It says to schedule the cache story first but fixes the experiment to baseline `a6cfa5183`, leaving the experiment’s revision ambiguous after that story lands. Specify the post-story local revision for both arms and retain `a6cfa5183` as the original baseline reference.
