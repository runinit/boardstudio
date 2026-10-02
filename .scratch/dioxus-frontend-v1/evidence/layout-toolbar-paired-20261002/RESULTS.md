# Paired Layout toolbar and Case readiness check

**Date:** 2026-10-02  
**Dioxus candidate:** `http://127.0.0.1:34696/`  
**TypeScript reference:** `http://127.0.0.1:5175/`  
**Fixture:** same imported layered Sofle archive in isolated sessions, SHA-256 `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`.

## Layout toolbar inventory

Both captures show Left PCB with Column 1 selected (four grouped objects / eight keys) and Snap settings open. The visible contour and selected-key geometry align. The Dioxus menu exposes Off, ⅛u, ¼u, ½u, 1u, 1 mm, 0.5 mm, and 0.1 mm increments, plus Geometry snap, Envelope gap, and Snap gap; these are the same snap choices inspected in the reference.

The remaining parity gaps are visible: TypeScript combines Select, Transform, Align, and Snap into one floating command pill and keeps the workbench view switch under it. Dioxus presently renders Select and Snap separately, shows an additional selection-context chip, and lacks Transform/Align controls. Its left rail also includes Physical instance and Group objects filters that are absent from the reference. This capture is a control/visual inventory, not a gesture edit or a claim that F3.3 is complete.

## Case readiness regression

On the same archive, Dioxus now shows Generate case disabled while the selected physical scope has no matching readiness row. The readiness predicate is private to the page, mirrors the lower-level preparation precondition, and has truth-table coverage; `preparation_request` remains the final admission authority. The separate baseline Case viewer gap (TypeScript displays the 90/90 physical model assembly after adding the default plate while Dioxus remains blank) remains open under RF-003/F7.3.

## Captures

| File | SHA-256 |
| --- | --- |
| `typescript-layout-column-snap-open.png` | `3ba98b128c5f683ab60a66eff781020f193e6a66210fbc7125d6d977f27311f2` |
| `dioxus-layout-column-snap-open.png` | `077c423e58ee01bb755c8eb40fafd20b4d9a1cfba0535e2e03e1a7f7c79c3af9` |
| `dioxus-case-readiness.png` | `7d47c641d85d25dd35b18e37475b69b7d0dfea5e6049796b2d958c88540bdb12` |

No edit, Undo, or save/reopen journey was performed in this pass. Full F3 and all parent acceptance remain open.
