# 118: Sofle adaptation hardware contract

Status: ready-for-agent  
Type: research  
Blocked by: none

## Contract

The user has delegated hardware selection and additions for demonstrated footprint gaps. Reuse existing catalogue entries, generator parameters, and assemblies first; add an asset only when a gap is established. Keep every measured Sofle v2, RGB, and Choc key position, tilt, and width fixed. Use the existing case-generator plate, gasket, and tolerance behavior; screws and spacers are allowed. Do not add a user BOM or routine selection-approval gate.

The current architecture is provisional and needs engineering revisions. The independent reviewer returned **NEEDS REVISION**. Isolated catalogue qualification may proceed; this verdict does not clear the full design. The hardware candidate audit contains the detailed selection, evidence, and open gates: [Sofle adaptation hardware candidates and case-generator evidence](../audit/sofle-adaptation-hardware-candidates.md).

## Acceptance criteria

- Record selected versus qualified items and the evidence for each; do not claim source qualification or complete physical/electrical fit without evidence.
- Close or explicitly assign the UART powered-off/hot-plug, lighting branch fault protection, full-white current/thermal, far-end voltage, encoder/matrix-current, and installed-geometry revisions in the linked audit.
- Verify the target firmware revision, pin mux, central/peripheral builds, and bench behavior. Record the exact harness and stack; do not assume BLE support.
- Preserve the measured key geometry and saved width manifest. Reuse existing footprints, generator parameters, and assemblies before adding assets.
- Audit the exact generator/test path and define a bounded file lease before dispatching unit 119. Keep 119 provisional until that audit; do not invent its file ownership or imply the other broad 119–127 tasks are ready.
- Keep this design open and unqualified until the engineering evidence is recorded. Documentation completion alone does not resolve it.

## Review boundary

The independent review addressed the candidate design and identified required revisions; it did not physically qualify components or clear the whole design. Unit 119's earliest independent scope is only the existing Kailh CPG151101S11-16 MX hot-swap socket against official KHPS220643 Rev A. It has no dependency on 19, 20, 44, or recipe/golden work.

The subsequent read-only audit identified the existing generator and public test pattern. [Ticket 119](119-mx-hotswap-socket-qualification.md) now defines the proposed isolated test/evidence scope; the generator remains read-only because no defect is established. Root's exact implementation lease and command handoff remain required. This source-path audit does not close any system electrical or installed-fit gate in 118.

## References

See the linked candidate audit for primary manufacturer and upstream firmware references and the provisional inventory.
