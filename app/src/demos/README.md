# Sofle demo keyboards

Open **Project → Demo keyboard** and choose **Sofle v2**, **Sofle RGB**, or
**Sofle Choc**. Each opens a fresh, locally saved project. Use the Board selector
in Objects to switch between the two independently editable PCBs. Keys are
physical row/column matrices, with separate thumb groups and attached diode/LED
assemblies. Peripherals and live generated outlines are editable. Save project copy creates a normal `.boardstudio` archive.

These are library-based equivalents, not imported/routed copies of the original
PCBs. They use only existing bundled generators; no footprint library changes
are required.

| Detail | Reconstruction |
| --- | --- |
| Key positions, stagger, thumb angles | Measured separately from each upstream PCB; mirrored for the left half |
| Exterior outline | Live part envelope with the normal 4 mm margin and corner controls |
| v2 keys | 29 MX hotswap switches + diodes per half |
| RGB keys | 29 MX hotswap switches + diodes + MINI-E LEDs per half, plus 7 additional LEDs |
| Choc keys | 29 Choc V1 hotswap switches + diodes + MINI-E LEDs per half |
| Controller | Bundled nice!nano footprint with extra pins enabled for peripheral pin capacity |
| Encoder | Bundled EC11/EC12, including push switch |
| OLED | Bundled SSD1306, with origin adjusted to align the header |
| Split connector | Bundled PJ-320A; placement adjusted for its different body |
| Reset / mounting | Bundled THT reset, rotated to fit, and 2.2 mm NPTH holes |
| Wiring | Board Studio's reviewed matrix/peripheral resolver; wired UART split with local power per half |

The original Pro Micro, MJ-4PP-9, reversible pads/jumpers, legacy LED footprints,
auxiliary headers, graphics, and copper routing are not copied. The two PCB designs
are distinct instead of one reversible design. Upstream RGB board-level LED slots
are not reused: the replacement LED footprints emit their own cutouts. Keycaps
use the existing presets' sizes. The RGB additional LEDs use the bundled MINI-E
in standard mounting instead of the upstream packages. Firmware output targets
the supported nice!nano platform, not the original Sofle QMK target.

Tests exercise the real Rust/WASM core: opening, two-board net isolation,
electrical resolution, KiCad generation and parsing, split firmware generation,
matrix editing, undo, and pad/hole bounds containment. Browser tests open every
demo through the menu, select either PCB, and reload saved projects.

These are unrouted design/test projects. Successful net planning and export are
not manufacturing validation: routing, clearance checks, component fit, RGB
power/logic-level design, firmware builds, and hardware testing remain necessary.

## Provenance and regeneration

Measurements: [josefadamcik/SofleKeyboard](https://github.com/josefadamcik/SofleKeyboard),
commit `fb294f7c58d0f91c379a9ef339159d4d056643e9`.
`sofle-layouts.json` records the source PCB paths and SHA-256 digests.
Sofle by Josef Adamcik; RGB contributed by Dane Evans. See
[SOFLE-LICENSE](SOFLE-LICENSE) for upstream MIT attribution, including antecedent
keyboard designs. Bundled footprint licenses remain in `ergogen/library`.

From the repository root, with an upstream checkout at the pinned commit:

```sh
node scripts/extract-sofle-layouts.mjs /path/to/SofleKeyboard
pnpm --dir app exec vitest run src/demos/sofle.test.ts
pnpm --dir app build
pnpm --dir app exec playwright test sofle-demos.spec.ts
```

The extractor only reads measurements and verifies closed outlines. It never
imports footprints, routing, or executable code from the upstream checkout.

## Additional keyboard adaptations

The same menu also offers Corne (42), Lily58 (58), Ferris Sweep (34), Chocofi
(36), REVIUNG41 (41), TOTEM (38), KLOR (42), Cantor (42), GH60 ANSI (61),
Discipline ANSI (68), Mysterium ANSI (87), Voyager97 (103-key selected layout),
Voyager104 ANSI (104), Plaid (48), and Lumberjack (60).

`keyboard-layouts.json` retains measured switch centers, angles, source references,
PCB hashes, and exact upstream revisions. Alternate overlapping switch positions
are excluded. Voyager97 is the upstream model name: this selected layout includes
103 keys, including its extra function/navigation positions. Discipline's two
right-of-space modifiers use 1u keycaps to match their measured 19.05 mm spacing.

These additional demos reconstruct **key layouts**, with live part-envelope PCB
outlines and a separate controller area on the right. They do not reproduce
source outlines, case compatibility, mounting patterns, reversible construction,
on-board MCU circuitry, protection circuitry, or source routing. Every PCB uses
the existing nice!nano module footprint, a reset switch, and a diode matrix.
Split demos add the existing TRRS connector; their two designs are independently
editable. Sweep and Cantor therefore use diodes rather than their original direct
scan circuits. Split power is local to each half. Original OLEDs, encoders, LEDs,
haptics, buzzers, batteries, and power switches are omitted from these additional
layouts; the detailed Sofle demos above retain their specified peripherals.

Balanced electrical row/column assignments fit the existing controller's GPIO
budget independently of the visible matrix groups. Finger columns, main rows,
thumbs, function keys, navigation, arrows and numpads use physical groups. Column
stagger/splay and residual cell offsets preserve the measured positions. Wide MX keys carry the approved 2u or 6.25u
stabilizer as an attached assembly member, so moving the matrix also moves its
stabilizers. Standard 2u stabilizer spacing also serves 2.25u and 2.75u keys.
All other footprints come from the bundled generator catalogue.

Upstream license texts are retained in `licenses/`; see each layout's
`licenseFile`. In particular, Discipline retains its upstream noncommercial
terms. The inspected GH60 repository has no root license: its entry records only
standard ANSI switch positions and does not copy its PCB artwork or circuitry.
These measurements do not change the licenses of their source projects.

To regenerate, use checkouts matching the revisions in the measurement file:

```sh
node scripts/extract-keyboard-layouts.mjs /path/to/checkouts
pnpm --dir app exec vitest run src/demos/keyboards.test.ts src/parts/catalogue.test.ts
pnpm --dir app build
pnpm --dir app exec playwright test keyboard-demos.spec.ts
```

Checkout directory names are specified in the extractor. Corne uses the
`corne-cherry-v3.0.1` revision (`corne-v3` directory), not the different v4 layout.
The [reviewed parts import process](../parts/README.md) documents how the two
approved stabilizers are converted without importing upstream switch libraries.

## Editing outlines and presets

New templates use the normal automatic envelope settings, so key movement,
resizing, deletion and component placement regenerate the perimeter. Existing
saved demos are not migrated automatically. Open a new demo for the physical
groups and live outlines.

**Add object → Board outline…** exposes additions, cutouts and manual connections.
Draw points, then press Enter. Edit their coordinates or drag the canvas handles;
use the 1, 0.5 or 0.1 mm grid, Alt to move freely, and arrow keys to nudge. Midpoint
handles insert vertices; Delete removes a focused point while keeping the minimum
three polygon or two connection points. Escape cancels a drag without an undo step.

Additions and cutouts are fixed by default. A shape attachment makes them follow
a component; attaching/detaching preserves the current shape. Connection endpoints
within 10 mm of a component attach automatically and can be changed in the editor.
Other connection points stay fixed unless explicitly attached. Deleting an anchor
keeps its attached controls at their last position. Missing attachments in loaded
files appear in Findings. Connections use a positive width and join islands before
the automatic nearest bridges are calculated. Cutouts are applied afterwards.
The resulting core contours drive the canvas and SVG, DXF and PCB exports.

Preset changes preserve authored keycap sizes and extra assembly members, including
stabilizers. MX stabilizers block incompatible Choc presets until removed. New keys
inherit the diode/LED preset without copying another key's stabilizer. No other
components have been added beyond the two approved stabilizer footprints.
