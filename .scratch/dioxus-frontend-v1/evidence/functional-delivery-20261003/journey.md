# Functional delivery batch — 2026-10-03

The user prioritized working workflows over exact visual layout. No parent is accepted by this bounded batch. Candidate 34775 is packaged source `2caace52ee2bd9a813030586bf88a1de7aa88a0c`; TypeScript reference 5175 is `5a472a9426e6e38993361da402cd4ec730feb369`. Source-only follow-ups are not attributed to that immutable package.

## Export and outline characterization (coordinator)

Fresh Sofle: SVG, DXF, KiCad footprints, ZMK firmware and embedded-model portable project all downloaded. ZIP CRC validation passed; footprints contains 22 entries, firmware 15, portable project 8 (project/archive metadata plus six model assets). This establishes delivery/content structure, not downstream manufacturing or firmware validation.

Matched settled matrix key width 1 → 1.25 → 1 in both applications produced byte-identical exported SVGs, SHA-256 `c34c198fa3e2c4342668b274c8d8385aceedd12b89c73a1b8db9f60181f1f83e`. Pristine candidate min X is -3.82; both after the roundtrip are -3.97. The earlier coordinate mismatch compared different edit histories, and does not establish a migration regression. The roundtrip normalization itself remains a post-port characterization question. An initial rapid automation attempt did not await the first accepted update and remained at width 1.25; its output is retained and excluded from the settled comparison.

Both coordinator-owned browser sessions closed. Download files remain at `/home/chris/.local/share/boardstudio/retained-tmp/20261003/functional-exports`; hashes:

- `footprints.zip`: 1584592 bytes, SHA-256 `4d0417f389f61352dcee19eacf3e4341b0664be5b1a99d996b6edf45b28edb83`.
- `outline-settled-roundtrip.svg`: 2150 bytes, SHA-256 `c34c198fa3e2c4342668b274c8d8385aceedd12b89c73a1b8db9f60181f1f83e`.
- `outline-size-roundtrip.svg`: 2103 bytes, SHA-256 `f13a41bccceb9907bb0b46b1ce7eb869a0cb0b27a4b3409457aedc50b39be2ec`.
- `outline.dxf`: 2676 bytes, SHA-256 `50e12044a2b5fe481b06769ed1d92c72be88022df22ee437ac886ecbf846696c`.
- `outline.svg`: 2150 bytes, SHA-256 `920aee5f749dc53d9efd318b23de0ef0e41360df67ffa6ef1cd6e2b32e6fad81`.
- `react-outline-settled-roundtrip.svg`: 2150 bytes, SHA-256 `c34c198fa3e2c4342668b274c8d8385aceedd12b89c73a1b8db9f60181f1f83e`.
- `sofle.boardstudio`: 1585781 bytes, SHA-256 `f6d09570c89bd2f9034030cde7b5c161e5031747b1289d674f58fc5a187ca356`.
- `zmk.zip`: 12039 bytes, SHA-256 `71a87134efc1a380ff87c414960f2ae0ba1b6263b9b006ba4808a93ae0c938db`.

## Parallel functional journeys

- Parts/Project author: create keyboard, import the above archive, place MX Hotswap assembly (70 → 72 parts), switch project/reopen with matrix retained; import absolute-path SW_MX_1u footprint and reopen with entry retained. Same footprint imported in TypeScript. Initial relative-path NotReadableError was a browser-tool path error; no product repair warranted. Owned sessions closed.
- Case author: generated stack, Wireless → Wired physical setup, Exploded/Section/Fit and explicit Update preview worked. Mechanical package downloaded with assembly/per-part STEP, STL, DXF, SVG, fabrication and fit metadata. Unsupported thickness surfaced profile validation; restoring 1.5 mm restored exact five-layer preview. No new defect established; owned session closed.
- Keycaps verifier: paired same-fixture SW1 3D preview and keycap STEP export passed; files are byte-identical, 6,246,504 bytes, SHA-256 `8ba23e9753f07e3a74e869a5f7eedbdd67b7d7b8500f4b76cf10e179f4ce52e0`. Evidence is in adjacent `keycaps-f6c5-functional-20261003/`. Browser error lists empty; owned sessions closed.
- Keymap and PCB diagnostic conclusions, implementation fixes and new candidate checks are recorded below when settled.

## Implemented selection correction

Objects Matrix/Row/Column/Key/Component selections now update the existing selection-kind signal that drives canvas interaction mode. Board/Outline/OutlineVersion/Bridge/MountedModule/LayoutGroup leave it unchanged. The focused WASM regression failed before the mapping (Matrix returned None, expected Some(Matrix)) and passed after (1 passed, 0 failed, 180 filtered). The author ran formatting and targeted diff checks. No second selection authority was introduced. Existing outline Done and shared Snap settings already function in source; cosmetic control relocation is deferred.
