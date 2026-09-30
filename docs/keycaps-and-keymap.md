# Keycaps and keymap

Open **Keymap**, choose a profile for each matrix, and select a switch on the
layout to assign its ZMK binding. Its legend follows that binding unless an
explicit legend or blank override is saved. Board colors provide defaults;
individual keys can override the cap color, profile, row, size in units, and
socket. Changes participate in normal project saving, Undo, Redo, and archives.

Matrix profiles include Cherry, OEM, DCS, DSA, SA, Hi-Pro, G20, and Choc. Their
parameters are independently designed nominal presets, including sculpted rows,
taper, dish, and wall thickness. They do not reproduce manufacturer dimensions.
Sockets include MX, Choc v1, Choc v2, and Alps. Known switch families are inferred;
ambiguous or custom definitions require an explicit socket. A mismatched known
switch family produces an error.

The **3D assembly** view generates hollow caps, sockets, and recessed letter
inlays in Rust using the existing Cadrum/OCCT backend. Colors belong to individual
assembly bodies. CAD templates are cached independently of placement and color,
and preview work runs in the CAD worker with cooperative cancellation between
batches. **Export keycap STEP** includes caps and their separate letter inlays.
Wide caps currently have central sockets; stabilizer sockets are not generated.

Clearance findings compare rotated envelopes through the full switch travel,
including neighbor caps and current prepared case walls, openings, cavities, and
support solids. This is a conservative interference screen, not an exact Boolean
collision or a physical fit certification. Stale case geometry is identified;
update the Case preview to check the current revision. Switch housing details,
stabilizers, printing tolerances, and material deformation require separate fit
validation.

Configure the controller and electrical scan in **PCB**, then use **Export ZMK
source** in Keymap. The ZIP includes resolved scan pins, the keymap, shield
configuration, a pinned ZMK manifest, and `build-local.sh`. Extract it into a
fresh directory, install the ZMK local toolchain, then run:

```sh
sh build-local.sh
```

The script initializes its own west workspace, updates dependencies, exports
Zephyr, and builds the board or both split halves. Repeated runs reuse that
workspace. It refuses an enclosing west workspace or a different local manifest.
See [ZMK's local build guide](https://zmk.dev/docs/development/local-toolchain/build-flash)
for toolchain installation. Tests validate exported arguments and script behavior;
a complete firmware compilation still requires the external toolchain.

## Implementation provenance

The public [KeyV2 project overview](https://github.com/rsheldiii/Keyv2) was used to
identify useful concepts such as profiles, stems, units, and legends. No upstream
OpenSCAD implementation, dimensional tables, or generated geometry was copied.
The domain resolver, CAD construction, and interfaces were authored independently
in this repository. This is a Board Studio implementation of the requested
workflow, not a claim of complete KeyV2 feature parity.

Letter outlines use bundled DejaVu Sans with `ttf-parser` 0.25.1. The font's license
and attribution are shipped alongside the font in `cad/wasm/assets` and in the
web application's public notices. Unsupported glyphs produce a visible CAD error.

See [Development worktrees](development-worktrees.md) for preserved experiments
and unfinished mechanical work relocated before this feature was implemented.

## Verification

The implementation passed 299 core tests, 23 native CAD tests, 49 CAD/WASM
checks, 26 renderer tests, and 442 app tests. The affected browser tests cover
keymap editing and persistence, mobile input bounds, generated caps and legends,
STEP and firmware downloads, existing assembly controls, electrical export,
and workflow navigation (17 browser tests). Repository hygiene, generated-contract checks,
contract runtime imports, and native/WASM boundaries also passed.

The local firmware build script was executed against a west test double to
check command arguments, repeat runs, workspace isolation, and paths containing
spaces. This does not constitute a full ZMK toolchain compilation or physical
keycap fit verification.
