# Project context

Shared design context for UX work on BoardStudio. [PRODUCT.md](../../PRODUCT.md)
owns product scope, users and principles; this note records what PRODUCT.md
leaves open: sessions, business stance, success criteria, evidence and the
assumptions still to test. Recorded 2026-10-07 from the maintainer's answers.

Each item is marked **Confirmed** (stated by the maintainer) or **Assumption**
(inferred; validate before relying on it for a decision that is costly to reverse).

## Users and sessions

**Confirmed.** Designers arrive in the sessions below. Importing outside work
(KLE, existing KiCad boards) is not a current priority: no import source beyond
the bundled Ergogen generators needs design attention for now.

| Session | What the designer needs most | Main failure risk |
| --- | --- | --- |
| New board from scratch | A fast path from starter layout through PCB and Case to Export | Not knowing what remains before fabrication |
| Revising a saved project | Seeing what a change affects; stale exports rejected clearly | A change that silently leaves an old export or case result in place |
| Quick exploration | Low commitment; trying ideas without setup | Exploration state mistaken for a design ready to fabricate |

**Assumption.** Until there is evidence, *new board from scratch* is the main path
for the first-fabricated-board goal, and *revising a saved project* comes second.
Quick exploration must work but does not set the default layout of the
workbench. Do not merge the sessions into one composite persona: their failure
risks differ.

**Assumption.** Users are experienced keyboard builders who already know KiCad,
footprints and fabrication houses, and spend real money on each board and case
order. They tolerate dense expert controls but not ambiguity about readiness.

## Product and business

- **Confirmed.** Free now; a paid tier may come later. Paid features, including
  any ordering flow, are out of scope until the app itself works end to end.
- **Confirmed.** Solo maintainer, who holds all decision authority. No deadline.
- **Confirmed.** The first fabrication target is JLCPCB, which covers PCB, CNC
  and 3D printing. Cases start as CNC aluminium and CNC plastic; the first
  builds are an aluminium case with a PCB plate, and a 3D-printed case with a
  3D-printed or PCB plate.
- **Confirmed.** PCB routing will be built into the app. Until it is, routing
  happens in KiCad.
- **Confirmed.** Manufacturing checks (tolerances, minimum features, plate and
  switch fit) are built into the app from the manufacturer's published
  requirements.
- **Assumption.** Published requirements say what a manufacturer accepts, not
  how a finished keyboard fits. Physical builds of the combinations above should
  calibrate the built-in checks.

## Ethical stance

**Confirmed.** These stay free and local whatever paid tier exists:

1. Opening and saving your own project files. A lapsed or absent subscription
   never locks a project.
2. The core exports needed to fabricate (KiCad, outlines, case geometry,
   firmware handoff).
3. Offline, local-first work with no required account or cloud sync.
4. No usage telemetry by default. Any analytics are opt-in.

**Confirmed.** Every pattern in the Intent anti-pattern catalog is rejected. The
risks most specific to BoardStudio are:

- **Overclaiming readiness.** Calling a preview, a syntax check or an exact
  geometry result "ready" when it does not establish fabrication readiness.
  PRODUCT.md principle 5 is the rule, and this is the highest-cost harm because
  it leads to fabrication orders that fail.
- **Paywall nagging** once a paid tier exists: upgrade prompts inside core
  workflows or on export. Paid features can be described where relevant, never
  placed in the path of free work.
- **Lock-in by format.** A paid feature must not change the project format in a
  way the free tier cannot open.
## Platform and access

**Confirmed.** Desktop-first. Full editing on desktop. Narrow screens support
review and light edits. WCAG 2.2 AA throughout, including keyboard operation of
spatial editing and non-repetitive status announcements.

## Success criteria

**Confirmed goals** for the next few months:

1. **First board fabricated.** A real board and case are ordered from BoardStudio
   exports and fit when assembled.
2. **No misleading readiness.** No designer mistakes a preview or partial check
   for fabrication approval.

Telemetry is off by default, so these are measured directly rather than from
analytics:

| Goal | Signal | How it is checked |
| --- | --- | --- |
| First board fabricated | JLCPCB accepts the PCB, CNC and 3D-print files without hand repair; case parts fit | Fabrication log per board: what was edited outside BoardStudio, what JLCPCB's review flagged, what did not fit, and why |
| No misleading readiness | Designers describe each state's meaning correctly | Observed sessions: ask "is this ready to fabricate?" at Case and Export, then compare the answer with the true state |
| No misleading readiness | Stale or incomplete results never reach a file | Tests that exporters reject stale revisions (already a product requirement) |

## Evidence

**Confirmed.** All evidence today comes from the maintainer's own projects. There
has been no outside testing.

This means every claim about "users" above is a hypothesis drawn from one expert.
Expect gaps in places such as onboarding, terminology and import expectations,
where the maintainer's familiarity hides friction.

**Next evidence step (assumption about approach).** Five observed sessions with
outside keyboard designers, at least one per session type, think-aloud, focused
on the Case and Export readiness states. That is enough to catch most severe
usability issues; it is not enough to size preferences.

## Readiness ladder

**Assumption.** Readiness wording should name the real check each stage passed,
not a general "ready". These stages are distinct and must not be merged:

| Stage | Established by | Says nothing about |
| --- | --- | --- |
| Preview available | BoardStudio generated geometry for the current revision | Validity, clearance, manufacturability |
| Export valid | Exporter accepted the current, non-stale revision | Manufacturer acceptance, fit |
| Manufacturer review passed | JLCPCB's own file review for PCB, CNC or 3D print | Physical fit, electrical function |
| Fit and function verified | An assembled keyboard, checked by a person | — |

BoardStudio can establish the first two by itself. The third comes from JLCPCB,
and the fourth only from hardware.

## Open questions

None outstanding as of 2026-10-07. Re-open this list when routing or a paid
tier is specified.
