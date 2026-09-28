# Gasket-case hardware preset research

Research date: 2026-09-28. This note bounds the first hardware catalog for the gasket-case redesign. Values below are copied or converted from manufacturer material; they are not a substitute for fit checks in the target material and process.

## Agreed catalog scope

Initial presets cover **M2 and M2.5**, defaulting to **M2**, with **Torx and hex**
drives. M1.5 and M3 are excluded for now. Advanced preferences permit custom screw
sizes. Historical M3 measurements below are reference data, not initial presets;
M1.5 is no longer a release research requirement. Drive type and external head
profile are distinct properties. Torx candidates are recorded separately below.

## Findings

### Heat-installed inserts for printed cases

[SPIROL's Inserts for Plastics design guide](https://www.spirol.com/library/main_catalogs/SPIROL-Inserts-for-Plastics-us.pdf) is the primary dimensional source. Its straight-hole Series 19/20 table gives the following metric mappings (the guide also gives inch equivalents, tolerances, and a recommended hole diameter):

| Thread | Pitch | Overknurl diameter A (short / long) | Pilot diameter P | Length L (short / long) | Recommended hole D | Status |
|---|---:|---:|---:|---:|---:|---|
| M2 | 0.4 mm | 3.58 / 3.63 mm | 3.12 mm | 3.18 / 3.99 mm | 3.20 mm | verified SPIROL family |
| M2.5 | 0.45 mm | 4.62 / 4.75 mm | 3.91 mm | 3.56 / 5.74 mm | 3.99 mm | verified SPIROL family |
| M3 | 0.5 mm | 4.62 / 4.75 mm | 3.91 mm | 3.56 / 5.74 mm | 3.99 mm | verified SPIROL family |

The table is a shared dimensional row for M2.5 and M3 in this insert series; the thread pitch is different and must remain a separate parameter. SPIROL says the hole must be deeper than the insert length and that correct hole size and sufficient surrounding material are critical ([installation guidance](https://www.spirol.com/resources/white-papers/how-to-design-the-proper-hole-for-heat-ultrasonic-inserts/)). Its guidance describes a general starting point of roughly two to three times the insert diameter for the **boss diameter** (the surrounding boss, not a required uniform wall thickness), with the relative multiplier decreasing as insert diameter increases. The software should therefore derive pilot-hole depth and boss/edge clearance from the selected insert, while leaving process-specific clearance as an explicit margin.

[ruthex's official assortment listing](https://www.ruthex.de/en/products/ruthex-komplett-set-m2-m3-m4-m5-sortimentskasten-m2-5-m6-m8-standard-gewindeeinsatz-m3s-m4s-m5s-short-1-4-zoll-gewindebuchsen-lotspitzen-set) confirms the product family. A [ruthex-authored RX-series data sheet](https://www.igo3d.com/mediafiles/Sonstiges/Ruthex/ruthex_Datenblatt_RX-Serie.pdf), dated 2022-08-15 and hosted by a distributor, supplies these dimensions (the drawing's labels should be checked when implementing):

| Product identifier | Thread | d1 / d2 | Length | Wmin | d3 | Status |
|---|---|---:|---:|---:|---:|---|
| GE-M2x04-001 / RX-M2x4 | M2 | 3.6 / 3.1 mm | 4.0 mm | 1.3 mm | 3.2 mm | manufacturer data sheet |
| GE-M25x57-001 / RX-M2,5x5,7 | M2.5 | 4.6 / 3.9 mm | 5.7 mm | 1.6 mm | 4.0 mm | manufacturer data sheet |
| GE-M3Sx40-002 / RX-M3Sx4,0 | M3 | 4.6 / 3.9 mm | 4.0 mm | 1.6 mm | 4.0 mm | manufacturer data sheet |
| GE-M3x57-001 / RX-M3x5,7 | M3 | 4.6 / 3.9 mm | 5.7 mm | 1.6 mm | 4.0 mm | manufacturer data sheet |

Use these ruthex values only with the selected ruthex product; do not mix them with SPIROL Series 19/20 geometry. The product listing confirms consumer availability, while the data sheet supplies the dimensional source.

**M1.5:** neither SPIROL's metric heat/ultrasonic catalog nor the ruthex family found here offers an M1.5 insert. Do not substitute M1.6 or infer a hole size. M1.5 remains an allowed screw-thread value only if a future insert supplier drawing is added; printed-case export should report “no verified insert preset” for M1.5.

### Inserts for machined metal cases

For machined solid tops, investigate Böllhoff [HELICOIL Plus](https://media.boellhoff.com/files/pdf1/0100-helicoil-plus-en.pdf), pages 17–18. These require tapped holding threads, not smooth heat-set pockets. The catalog supplies these Free Running candidates; material selection remains part of the order code:

| Inner thread | Pitch | Nominal lengths: 1d / 1.5d / 2d | Recommended drill B | Example 1.5d item |
|---|---:|---:|---:|---|
| M2 | 0.40 mm | 2 / 3 / 4 mm | 2.10 mm | 4130 002 0003 |
| M2.5 | 0.45 mm | 2.5 / 3.75 / 5 mm | 2.60 mm | 4130 025 0375 |
| M3 | 0.50 mm | 3 / 4.5 / 6 mm | 3.20 mm | 4130 003 0045 |

The catalog distinguishes full holding-thread length, drilling depth, installation setback, and maximum screw penetration with the tang retained. Its blind-hole depth rule is `t1 = t2 + e1`; series production adds at least one pitch to `t1` and `t2`. Preserve these separate constraints and the closed roof. Exact tap/runout, tang handling, host material, and installation access still need resolution before export-ready presets. Thread requirements belong in manufacturing annotations; detailed helical CAD is not required by this proposal.

Sheet-metal self-clinching nuts are excluded from this proposal because compatibility with this blind-roof construction was not established. M1.5 remains unresolved.

### Screws

[Westfield Fasteners' DIN 912 / ISO 4762 A2 stainless tables](https://www.westfieldfasteners.co.uk/Datasheets/ScrewBolt_SHCap_M.pdf) provide a concrete common screw family for the first presets:

| Thread | Pitch | Maximum head diameter | Head height | Hex socket | Catalog-listed lengths | Example identifier |
|---|---:|---:|---:|---:|---|---|
| M2 | 0.40 mm | 3.8 mm | 2.0 mm | 1.5 mm | 4, 5, 6, 8, 10, 12, 14, 16, 18, 20 mm | M2 x 6: [WF2329](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-SHCap-M2.html) |
| M2.5 | 0.45 mm | 4.5 mm | 2.5 mm | 2.0 mm | 4, 5, 6, 8, 10, 12, 14, 16, 18, 20 mm | M2.5 x 6: [WF2335](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-SHCap-M2.5.html) |
| M3 | 0.50 mm | 5.5 mm | 3.0 mm | 2.5 mm | 4, 5, 6, 8, 10, 12, 14, 16, 18, 20 mm | [product table](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-SHCap-M3.html) |

These are catalog-listed lengths, not a live inventory claim. The generator can use this bounded 4–20 mm set as a proposed starter list while retaining supplier, material, and finish as fields. No usable primary-source M1.5 screw-and-insert pairing was verified in this research; keep M1.5 unresolved and never derive it from M1.6.

### Torx candidates

Westfield lists A2 stainless Torx pan-head products in [M2](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-TXPan-M2.html) and [M2.5](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-TXPan-M2.5.html):

| Thread | Pitch | Listed head diameter | Listed head height | Drive | Example 6 mm item |
|---|---:|---:|---:|---|---|
| M2 | 0.40 mm | 4.0 mm | 1.6 mm | T6 | WF26460 |
| M2.5 | 0.45 mm | 5.0 mm | 2.0 mm | T8 | WF26471 |

Both catalog pages list the proposed bounded lengths 4, 5, 6, 8, 10, 12, 14, 16, 18, and 20 mm. These are pan heads, distinct from the hex socket-cap candidates above.

[A+D's reference sheet](https://www.aanddfasteners.co.uk/uploads/media/wvGo3n-media-SpecificationforMetricTorxScrew.pdf) separates DIN 7985 and ISO 14583 maximum envelopes: M2 heights 1.72/1.60 mm, and M2.5 heights 2.12/2.10 mm, respectively. Westfield's listed M2.5 height is 2.0 mm, not an explicit maximum. Keep supplier nominal dimensions, maximum envelopes, and fit allowances distinct; resolve the selected product tolerance before declaring a head-seat fit verified. Do not substitute one vendor's dimensions as another's guaranteed limits.

## Bounded starter preset proposal

| Case process | Threads | Insert candidates | Screw candidates | Remaining work |
|---|---|---|---|---|
| Printed | M2, M2.5 | ruthex RX-M2x4 and RX-M2,5x5,7 | Hex: Westfield A2 ISO 4762/DIN 912; Torx: Westfield pan-head candidates above | installation depth and tolerance policy |
| Machined metal | M2, M2.5 | Böllhoff HELICOIL Plus blind-hole candidates | Hex: Westfield A2 ISO 4762/DIN 912; Torx: Westfield pan-head candidates above | full machining and installation requirements |

Custom screw-and-insert combinations are approved in Advanced preferences, including
thread diameter/pitch, screw length/head dimensions and matching insert dimensions. Unknown foam
compression limits permit export with a warning, as agreed; that does not supply
missing hardware geometry.

## Open verification tasks

1. Confirm the selected ruthex drawing labels, blind-hole depth, installation allowance, and material-dependent fit policy. Preserve the dated supplier snapshot rather than mixing brands.
2. Confirm the proposed head profiles and resolve product-specific maximum envelopes/tolerances for the Torx candidates.
3. Complete the selected HELICOIL machining callouts and installation clearances for the blind top, including minimum roof and tang handling.
4. Compare complete resolved case sizes for the candidate head variants; prioritize assembled outer height over width and depth, as confirmed in Q23. Prefer a no-boss fit with an available longer screw.

## Slim-profile follow-up

The user prioritizes a couple of slim head variants with Torx and hex drives.
The standard socket-cap/pan-head proposal above is retained as comparison data,
not the selected default. These are documented alternatives, **not identifications
of the two AliExpress products**:

| Candidate | M2 head diameter / height | M2.5 head diameter / height | Evidence and limitation |
|---|---|---|---|
| NBK SLH low-head hex | 3.8 / 1.3 mm | 4.5 / 1.6 mm | [Manufacturer table](https://www.nbk1560.com/en-US/products/specialscrew/nedzicom/socketheadcapscrew/SLH/) lists 5, 6, 8, 10, 12 mm lengths for both; these short variants may not reach the top insert |
| Aspen ISO 14580 low-head Torx | 3.80 max / 1.55 max mm for the sourced M2 × 12 drawing | 1.85 mm reference height; final product envelope pending | [M2 × 12 product drawing](https://www.aspenfasteners.com/content/2D_PDF/product51/ME492-204X12.PDF), [family reference](https://www.aspenfasteners.com/content/pdf/Metric_ISO_14580_spec.pdf); T6/T8 drives; verify each chosen length |

NBK values are product-table dimensions; obtain tolerances before treating them as
maximum envelopes. Aspen's summary table reports M2 DK 3.62 mm while its product
drawing gives 3.80 mm with -0.18 tolerance: do not treat the summary value as the
maximum head diameter. A product drawing should control the seat envelope.

Recommendation pending the requested listing data: low-head hex and ISO 14580
Torx are the two slim families to evaluate first. The shortest head is not
necessarily the best assembly fit if the available shank cannot reach the insert.
Select against head recess, actual grip distance, available length, engagement,
and supporting material together.

### Screw reach and insert length

[Westfield's standard M2 socket-cap catalog](https://www.westfieldfasteners.co.uk/A2-ScrewBolt-SHCap-M2.html)
lists 22, 25 and 30 mm screws. The earlier 20 mm bound was a proposed catalog
subset, not a universal limit. This does not establish those lengths for slim
variants.

Geometric derivation for the bottom-up closure: required usable screw reach is
at least `G + E`, where `G` runs from the head bearing plane to the first usable
insert thread, and `E` is engagement. Include thread lead/runout and tolerances.
Increasing insert length with its entrance fixed does not shorten `G`. Bringing
that entrance downward can shorten the grip, but needs a supported top boss and
tray clearance; an unsupported protruding insert is not an equivalent solution.
Downward closure bosses were approved in Q17, subject to clearances and unchanged
rigid stops. Q18 prefers an available longer screw with no downward boss; Q19
preserves infeasible selections and reports conflicts with suggested alternatives. A longer insert alone is not
a reason to claim that a 20 mm screw fits.

### Supplied listings: retrieval boundary

- [AliExpress item 1005006974121879](https://www.aliexpress.com/item/1005006974121879.html)
- [AliExpress item 1005012735799741](https://www.aliexpress.com/item/1005012735799741.html)

Both web-reader requests failed. The first direct browser navigation was rejected
by site-safety policy, and no workaround was attempted. No titles, dimensions,
selected variants, insert lengths, or seller specifications from these listings
were verified. Comparing these exact products requires user-supplied listing
text or dimension-chart images; the recommendations above concern independently
sourced products only.

## HZYUEGOU countersunk hex family: user-supplied evidence

User-supplied [AliExpress item 1005012862187959](https://www.aliexpress.com/item/1005012862187959.html)
and two screenshots establish a seller-described flat countersunk hex-socket
family. The seller labels it 304/A2 stainless, with A2-70 on the product image;
these are seller claims, not material certification.

- [First screenshot](gasket-hardware-hzyuegou-m16.png): M1.6 and 8 mm are selected;
  it is not evidence of M2 length availability.
- [M2 screenshot](gasket-hardware-hzyuegou-m2.png): M2 is selected; visible
  solid-bordered length options are 3, 4, 5, 6, 8, 10, 12, 14, 16, 18, 20, 22,
  25, 30, 35, and 40 mm. Options from 45 mm upward are dashed. The caption says
  “Length: 30mm” while the highlighted tile appears to be 16 mm, so record the
  visible option set without claiming a uniquely verified selected variant.

Include this specific family in the candidate catalog without a 20 mm cap.
M2.5 is shown as an available thread option, but its length matrix remains
unverified. M1.6 and other advertised threads remain outside the agreed starter
catalog. The screenshots do not establish head diameter/depth, countersink angle,
pitch, hex size, thread coverage, dimensional tolerances or current stock.

**Fit assessment:** this is a useful flush-head alternative when a correctly
sized countersink leaves enough material in the bottom closure land. It can avoid
needing a short-screw accommodation if one of its longer variants meets the actual
stack and engagement constraints. Exact fit is pending dimensions; no longer
insert or downward boss is implied solely by the selected thread size.

Flat countersunk screws are specified by overall length, unlike cap/pan screws
measured under the head; see [Accu's explicit overall-length product specification](https://www.accu.co.uk/countersunk-socket-head-screws/153236-SSK-M3-8-A2-BL).
This source establishes the convention, not the AliExpress product dimensions.
Store the datum per family and compute the installed tip, usable thread and
engagement from it. A countersunk and cap screw with the same nominal length are
not interchangeable inputs to an under-head grip calculation.

## CM ultra-thin wafer-head hex: user-supplied evidence

[Saved screenshot](gasket-hardware-wafer-hex-m2.png). The seller title describes
CM hex-socket ultra-thin, super-flat wafer-head screws in 304 stainless. The item
URL is not visible, so this evidence is not assigned to either earlier AliExpress
link. The material description is a seller claim.

- Selected variant: **M2, 100 pieces, ×25 mm**; caption and selected tile agree.
- Solid-bordered lengths: **2, 3, 4, 5, 6, 8, 10, 12, 14, 16, 18, 20, 25 mm**.
- Dashed lengths: **22, 30, 35, 40, 45, 50 mm**. Do not include these as confirmed
  choices or interpolate them into the observed list.
- M2.5 is an offered thread option, but its length combinations are not shown.
- The follow-up [dimension chart](gasket-hardware-wafer-hex-dimensions.png)
  supplies pitch, head limits and hex width as transcribed below. Thread coverage,
  length tolerance and the meaning of the length datum are not yet established.

**Fit assessment:** include as a thin, non-countersunk alternative alongside the
HZYUEGOU countersunk hex and low-head Torx candidates. A shallow flat-bottom recess
may accommodate this head if enough bearing material remains. Use the follow-up chart's maximum
head dimensions plus process allowances. Under-head transition and tool access
still need to be accounted for. M2 ×25 is evidence
of a longer offered wafer-head variant, not proof of engagement in every case.
Use the family-specific length datum; the supplied chart appears to include the
head in `L`, so the ordinary under-head convention must not be assumed. No insert
extension is implied.

### Wafer-head dimension chart

Source: [user-supplied chart](gasket-hardware-wafer-hex-dimensions.png), interpreted
in the context of the preceding wafer-head listing. It has no visible product ID.
All dimensions below are millimetres; these are seller specifications, not
independent measurements.

| Thread | Pitch P | Head diameter dk min–max | Head thickness k min–max | Hex width s |
|---|---:|---:|---:|---:|
| M2 | 0.40 | 3.9–4.1 | 0.8–1.0 | 1.2 |
| M2.5 | 0.45 | 5.0–5.2 | 0.8–1.0 | 1.5 |

For head clearance use dk max and k max, plus process/fit allowances. Neither
4.1 × 1.0 nor 5.2 × 1.0 is an automatically validated pocket size. Preserve the
minimum limits for tolerance checks rather than averaging the limits.

The `L` dimension's left extension appears aligned with the outer head face.
That suggests overall length, in tension with the expected under-head convention
for this shape. Record the ambiguity. If overall, under-head reach is `L - k`:
25 mm yields 24.0–24.2 mm using only the supplied head tolerance. Length tolerance,
thread lead/runout and usable engagement must still be included. Do not finalize
engagement from an assumed 25 mm under-head reach.

The M8 row has a reversed dk max/min pair (15.5/16.1), an apparent chart error.
M8 is outside the agreed catalog; do not silently correct it or import that row.
The M2/M2.5 ranges themselves are ordered consistently.

## Selection decisions from Q18–Q20

Prefer an available longer screw that achieves valid engagement without a downward
closure boss. Boss extension is a fallback, not a way to optimize for the shortest
screw. If selected hardware cannot fit, report the conflict and suggest alternatives
without silently replacing it. The initial head variant should yield the smaller
complete case, rather than favoring a particular drive or the thinnest head alone.
Q21 specifies overall outer width, depth and assembled height, not a volume score.
Q23 prioritizes minimum assembled height over width/depth reductions, using the
complete feasible stack rather than head thickness alone. Q22 preserves
selected hardware during later edits and produces conflicts with suggestions;
there is no ongoing automatic hardware reselection. Parametric case regeneration
for the selected hardware remains enabled.

### Q24: automatic case growth

The generator automatically grows case height when the selected hardware needs
more material or clearance, choosing the minimum feasible height. Routine sizing
should be hands-off. Preserve hardware selections and pinned positions while
resolving geometry; report conflicts and suggest alternatives only when the
constraints cannot be satisfied. This refines Q22: it prohibits silent hardware
replacement, not automatic resizing. Existing travel and material constraints
remain in force.
