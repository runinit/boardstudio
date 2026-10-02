# Binding controller Standards review

Reviewed commit `cd79d9e0664c651ee0bcfff51f791d3b0be56602` in the Keymap worker and released memo correction, controller SHA-256 `42d12a9bef6cb4cc3859f560a53f60c7df778ce9d850cbc1c15a9b3b9c934785`. Commit controller blob: `04e832a479d4559e5250fb1ec4359c6705413b08`; editor blob: `d8ade37b72541f786611258cc1771f9c73697fc0`.

No remaining material Standards finding in the corrected source. The original per-render duplicate projection allocated layer/macro labels twice, contrary to CONSTRAINTS.md:182–187. The correction memoizes one immutable Rc projection by source Scope/token/revision, Runtime version, view, active layer and selected key, and reuses it for feedback. This clears the source-copy finding without asserting measured performance.

The Editor-owned sequence survives child remounts. Display projection remains available during Busy/Unsaved states while edit admission requires fresh Ready/Saved state and current scope/generation. Pending feedback stays correlated with the admitted request across accepted-token advancement. Fresh field-specific merge preserves sibling binding values; matching terminal outcomes, selected-key/layer membership and full owner identity control settlement. No public visibility or API expansion appears in this slice.

RF takeaway: preserve the existing private Runtime outcome seam and accepted Rc view owner; no new refactoring issue established. Root must mount the hook unconditionally, pass live admission and accepted source, and preserve the child draft key. Compiler, native protocol checks and public failure/retry/Undo/remount behavior remain open. Source-only review; no Cargo, browser run or source modification performed.
