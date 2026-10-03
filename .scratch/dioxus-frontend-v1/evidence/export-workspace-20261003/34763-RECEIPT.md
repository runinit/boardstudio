# Export workspace bounded browser receipt

- Candidate: `http://127.0.0.1:34763/` (also served at `/boardstudio/`)
- Immutable source: `99ec041a2895e5ab23880be25501beb9360487db`
- Candidate provenance: `d3c03d6db107ab7a55753b5798d99444f7d0843540ef96530c00d9f44296fe08` (as reported by root)
- Candidate qualification reported by root: 8 fresh + 22 inherited checks; strict page check PASS; 1371 source inputs and 145 assets with zero route mismatch.
- Fixture: `/home/chris/.local/share/boardstudio/retained-tmp/20261002/keycaps-findings-f6c4-c6.boardstudio`
- Fixture SHA-256: `9027125846d2878176f127ec39b60c9ab605a0eb2a4b9019e10dbba846fe87a6`
- Viewport: 1280 x 577. Dioxus used the named `keycaps-gap-react-f7d2bd82afec` browser session; the persisted fixture was explicitly reopened after the prior cleanup had ended sessions.
- Pinned React reference: `http://127.0.0.1:5173/`, source `5a472a9426e6e38993361da402cd4ec730feb369`. The retained pinned-React Export reference capture was reused; React was not replayed for unchanged behavior.

## Observed behavior

With the fixture open, the Layout export route displayed the selected board as “Left PCB” and exposed the seven required design-output rows plus the optional Generated mechanical row. The portable-copy control and Review wiring / Review case links were present. SVG and DXF were enabled; ZMK active configuration was available. KiCad board, draft KiCad board, and KiCad footprints were disabled with readiness guidance. Authored Case STEP and Generated mechanical were shown as “Needs work” for this fixture. This is a bounded readiness observation, not a claim that all export providers are implemented.

SVG and DXF were each activated from the UI and downloaded successfully:

| Format | Captured file | Bytes | SHA-256 | Signature / media type |
| --- | --- | ---: | --- | --- |
| SVG | `dioxus-export.svg` | 2,150 | `920aee5f749dc53d9efd318b23de0ef0e41360df67ffa6ef1cd6e2b32e6fad81` | Starts with an `<svg` document; `image/svg+xml` |
| DXF | `dioxus-export.dxf` | 2,676 | `50e12044a2b5fe481b06769ed1d92c72be88022df22ee437ac886ecbf846696c` | AutoCAD DXF 2000 text; `application/dxf` |

Starting from Keycaps, the Export route showed “Back to Keycaps”. The explicit Back action, Escape, and header Export toggle each returned to Keycaps, preserving the selected Left PCB and its Keycaps context. The retained final-state screenshot is [34763-export-back-keycaps.png](34763-export-back-keycaps.png); the Layout-entry screenshot is [34763-export.png](34763-export.png). The downloaded [SVG](34763-outline.svg) and [DXF](34763-outline.dxf) are also retained beside this receipt.

## Scope limits

This receipt covers route composition/readiness, real SVG and DXF downloads, and return behavior only. It does not qualify the portable-copy download, Review-link destinations, ZMK download, authored/generated STEP in this fixture, KiCad providers, or complete export readiness. The existing ordinary authored Case STEP path applies when no generated mechanical configuration is present; it does not establish generated-stack authored-only STEP or a complete mechanical ZIP. F8.2–F8.6 remain open according to their parent criteria.
