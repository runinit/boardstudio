# 001: Chrome protocol compatibility and event failures

Status: resolved
Type: research
Blocked by: none
Parent: [Chromiumoxide browser checks](../map.md)

## Question

Why did the archived chromiumoxide spike record 19 protocol-decoding errors on Chrome
155/156, and what supported version or minimal integration approach makes the required
DevTools interactions reliable without suppressing failures?

Use the archived spike and primary upstream sources. Separate malformed/unrecognised
unsolicited events from command failures and event loss affecting our checks. Identify the
smallest safe red-capable compatibility probe, required event domains, dependency/version
choices, upgrade risks and a concrete go/no-go recommendation. No production edits or builds
that contend with active lanes; collect bounded runtime evidence only when safe.

## Claim

Owned by this Wayfinder session; delegated research is shared with the browser-launch question.

## Answer

The 19 archived spike errors are unsolicited `Network.requestWillBeSentExtraInfo`
event parse failures surfaced only because the spike opted into
`.surface_invalid_messages()`; the default harness config drops such events with a
warning and every command the spike consumed (Accessibility/Input/DOM/Runtime,
role lookup, trusted drags, `page errors: []`) succeeded. They are event loss in an
unsubscribed experimental domain, not command failures. The exact field mismatch is
unrecoverable from the archive (spike truncated raw payloads to 400 chars; Serde
cause never recorded) — see the recommended full-payload red probe. `0.9.1`
(`CURRENT_REVISION r1566079`) is the newest upstream release; local Chrome 156
match re-verified. Proposed policy: pin `=0.9.1` with CI Chrome, ship the default
ignore with an `InvalidMessage` counter in the run log, keep command/`pageerror`
failures fatal. Go/no-go and probe spec in §1.5–1.6 of the report.

Full findings:
[chromiumoxide-migration-feasibility.md](../../../research/chromiumoxide-migration-feasibility.md)
(§1). Archived on local branch `research/chromiumoxide-feasibility` at
`74a623030a2e174ab5fb2e393800728af5ea838f` (not for merge, do not push).
