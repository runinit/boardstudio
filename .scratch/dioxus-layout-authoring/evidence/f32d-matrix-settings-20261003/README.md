# F3.2d matrix configuration reference pin

Pinned React source: `5a472a9426e6e38993361da402cd4ec730feb369`, served from the public React route at `http://127.0.0.1:5173/`.

Fixture: layered Sofle archive `layered-sofle-export.boardstudio`, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

Using a named isolated browser session (`issue04-reference-eeb225b34b3e`), I imported the fixture, selected the `keys` matrix in the Left PCB tree, and expanded the matrix inspector's Key assembly, Keycap spacing, and Matrix actions sections. The mounted controls were:

- Layout name, rows, columns, pitch X/Y, and position/orientation controls.
- Assembly preset (`MX Solder`, `MX Hotswap`, `Choc V1 Solder`, `Choc V1 Hotswap`, `MX RGB`, `Choc V1 RGB`, `MX Hotswap RGB`, `Choc V1 Hotswap RGB`), switch orientation, Update assembly preset, and Duplicate design as variant.
- Switch footprint, diode direction, Edge gap X/Y, and a preview computed as `max(0, pitch - edge gap)` per axis.
- Delete matrix.
- Add object offers Add row and Add column for the selected matrix.

The screenshot `react-matrix-settings.png` captures these expanded sections. No project edit was submitted; the accepted document remained unchanged. Current structural Add/Delete/variant behavior will be verified on the integrated Dioxus candidate by the owning helper.
