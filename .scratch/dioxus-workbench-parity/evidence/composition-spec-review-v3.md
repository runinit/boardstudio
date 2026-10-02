# Composition v3 amendment and replacement ticket: Spec review

**V3 amendment clear; no blocking content finding.** Reviewed contract SHA-256 `c6550ab551734340be7c58683494925f4087941b9f598f0fd10131396a2f395f` as an exact diff against cleared v2 `a7e584a7bdc1e0129d23c72614e903eadd838c520c0d028b27e0ce50a6300d48`.

A workspace-tagged per-surface input enum is now the sole dispatch discriminator, eliminating disagreement with a separate string. The revised ownership text preserves existing root and feature-local lifecycle/Runtime behavior, including CasePanel, while adding no new composition state, hooks or authority. PCB/Keycaps accurately retain no Inspector wrapper. These are bounded clarifications; the v2 Spec clearance and its implementation/public-evidence obligations continue to apply.

Also reviewed replacement ticket SHA-256 `11c6dc607d8058ff4b5f2dbfb767aa193aa67745bce4d04421edca3f561a9356`. Its scope, dispatch hold, disjoint leaf ownership, behavior-preserving public characterization, source-appropriate selection transitions, root verification, mandatory ledger handoff and no-parent-closure limits align with Q4/Q5/Q6 and accepted INT.1. No material content findings.

**Publication provenance:** this ticket currently references retained v2. Before publishing/dispatching against v3, retain the exact v3 proposal and update the ticket link/hash so both review axes and implementation refer to the same final contract revision. This is provenance alignment, not a request to redesign the seam.

No code, builds, publication or central-document edits performed. Only this independent review artifact was written.
