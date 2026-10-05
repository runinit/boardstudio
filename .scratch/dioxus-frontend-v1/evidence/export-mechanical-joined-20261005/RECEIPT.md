# Authored and generated mechanical output qualification

Candidate38641dd1353bcfb6230d52c1d5b84c7ad7908e7a (frontend-routed-case-cancellation-20261005,34823) versus pinned TypeScript5a472a9426e6e38993361da402cd4ec730feb369 (5175). All downloads came from the public main Export workspace. Case-local Export geometry is a separate retained F7.6 proof.

## Valid generated package

Both imported the same retained f82-c01-left-plate-mechanical-ready.boardstudio SHA6773e7222eb5def8c4d2b7e2b9152da9d731e1b1383437943324241be94ac0c6, revision5, selected Left physical assembly, waited for current geometry, then opened Export. Both generated actions downloaded real ZIPs (candidate-generated.zip/reference-generated.zip).20matching paths: assembly STEP, four part STEP/STL pairs, four SVG/DXF outline pairs, fabrication instructions, critical-fit SVG and assembly JSON. ZIP integrity and all STEP signatures pass. Entire parsed assembly JSON equal atrevision5. Byte-level entries differ; STEP Cartesian-point multisets agree at1e-8mm, all numeric SVG/DXF/specification contents agree at10significantdigits after blank-line normalization. STL triangle counts, unique vertices at1e-6mm, surface areas and signed volumes agree to recorded precision, while triangle decompositions differ. No raw ZIP or full topology identity claim. generated-comparison.json and generated-geometry-comparison.json retain exact limits.

## FR-4 branch

Switching the Sofle plate process to PCB FR-4 properly blocks29sharp MX cutouts in both apps; fr4-ready.json preserves actual blockers, no output claimed. Instead imported explicit synthetic fr4-provider-fixture.boardstudio, derived from the existing real Core mechanical_plate test: empty40×30board, circular radius7opening at20/15 approximated64points, rigidFR4plate1.5mm,2.2mmNPTHmount at4/4; physical primary owner added for the public route. This is a QA geometry fixture, not a measured hardware or fabrication approval claim. Exact fixture hash/basis are in fr4-fixture.json.

Both public Case routes admitted the fixture and produced current geometry; both actual generated ZIPs downloaded successfully (candidate-fr4-generated.zip/reference-fr4-generated.zip).19matching entries include plate-kicad/mechanical-plate.kicad_pcb, .kicad_pro and plate fabrication instructions alongside STEP/STL/outlines/specification files. Complete assembly JSON equal at observedrevision1. FR4board output byte-identical SHA84910304353fd5c63453a5bf194028a0fce2c80ea45036442b9fef1d4f14dce6, thickness1.5mm,68closed-cutsegments, NPTHpresent, no nets/traces/vias. Actual manifest in fr4-artifact-comparison.json. KiCad CLI/CAM was not run; this frontend path proof does not claim fabrication qualification.

## Authored STEP routing

On the valid generated-config fixture, candidate enabled Authored Case STEP and downloaded Sofle v2-Left_PCB-case.step,816239bytes. Reference row remained disabled with its known prior Add and generate case geometry in Case limitation. Reimported the unchanged initial case-generation-export input into both apps and added default authored Plate through public New case body without configuring any mechanical stack. Candidate generated exact geometry and main Export enabled Case STEP; reference stayed Waiting to update preview even after its Update preview action, leaving the row disabled. This is documented reference behavior, not a paired-success claim.

Candidate authored-only action downloaded Case generation export20261005-Left_PCB-case.step. Actual files with/without generated configuration are byte-identical, SHAe292d44c9e19f848fbc27e3643415788b97e03488164779e393b9145184458f3,816239bytes, ISO-10303-21 header/end and2583Cartesianpoints. authored-artifact-comparison.json joins prior34814published model/step MIME and Left→Rightdisabled→Leftenabled scope evidence. The generated package never substitutes for authored STEP. Independent review still required; pending-delivery owner/currentness proof must be joined explicitly, not inferred from these successful downloads.

## Other rejected input retained

The separate gasket-unlinkr14fixture reaches generated preview but both apps reject package hardware screw:closure:left-keys-layout:0 not linked to a generated mounting feature. gasket-package-rejected.json retains that true rejection. It does not invalidate the successful gasket pointer/accepted-document proof and was not relabeled a successful export.
