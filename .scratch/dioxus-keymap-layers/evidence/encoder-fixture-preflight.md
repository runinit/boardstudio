# Real encoder fixture preflight

Date: 2026-10-02. This is a read-only source/archive inspection for later public UI verification. No Cargo/build was run, no project/archive was edited, and no browser fixture was created. The findings below are expected Core behavior derived from exact source and archive contents; they are not a Dioxus browser observation or a runtime call to `describe`.

## Smallest genuine layered fixture

Use `evidence/binding-editor-reference/imported-layered-sofle.boardstudio` from the retained evidence tree as the smallest genuine archive found that combines actual physical rotary parts with a nontrivial public Keymap fixture. Its size is 1,585,829 bytes and SHA-256 is `5b17071a819e4cfa28531913685e8fedf84cbdd361cefccb3b16befd6c0776df`. Its public React reference provenance is recorded in `evidence/binding-editor-reference/README.md`: the archive was already UI-produced, then imported into the isolated React reference session; the source session used pin `5a472a9426e6e38993361da402cd4ec730feb369`. The archive document is `m1-sofle-v2-copy`, “Sofle v2”, revision 9, split topology, with Main and Function layers and no existing encoder sensor bindings.

For a single-board encoder interaction, select its genuine Left PCB/left instance in the public UI and use `left/SW25`; the archive retains both actual halves and must not be rewritten to remove the other one. The physical rotary `Part`s are:

| Board | Physical part / reference | Expected Core encoder sensor ID | Expected push key ID | Actual electrical assignments |
|---|---|---|---|---|
| Left | `left/SW25` / `left-SW25` | `left/SW25` | `left/SW25/push` | encoder-a P1, encoder-b P0, encoder-push P2 |
| Right | `right/SW25` / `right-SW25` | `right/SW25` | `right/SW25/push` | encoder-a P2, encoder-b P3, encoder-push P4 |

Each part uses definition `sofle/rotary_encoder_ec11_ec12`, generator `ceoloide/rotary_encoder_ec11_ec12`, and source terminals `A`, `C`, `B`, `S1`, `S2`; `S1`/`S2` are the actual matrix terminals. The part belongs to only its corresponding board and is not a matrix cell. `inputProfile` is absent, and there is no `pressScanMode` override or `include_momentary_switch_pads=false` setting.

## Source-derived Core projection (not executed)

The inspected integration tree is at HEAD `e2a84d8b00418eab4d1ec47e0bf5b6957da9ffe3`. SHA-256 values of the relevant files at that tree are:

- `core/src/electrical_peripherals.rs`: `2c1a15492d0ec9d02e2a86fffc2f94160d3d431062f1fdba5ece493954bf34eb`
- `core/src/inputs.rs`: `c12ceb0c4a71ea9d0eb432a55fd376da18bcad4672bdec48bdabaac98f74cefd`
- `core/src/keymap.rs`: `a2a0764e90efeacf74482f7a399350914e9f049511af6b4e0478ec1e9ba5538b`
- `core/src/modules/electrical.rs`: `ae9e0dc6f32f729e76a3ec37a9ac2dc9685cddb3ee0ece5faa3aed6522b53653`
- `core/src/modules.rs`: `079ffa2bd4297bc91d012c8b9054685f1fae4fdd02ce6d4fb09a0cd06dd20a72`

For each physical part, `inputs::profile` recognizes the exact generator source and supplies the legacy EC11 profile (`a=A`, `b=C`, `common=B`, `steps=80`, `triggersPerRotation=20`, driver `ec11`). It obtains press contacts from the definition's `matrixTerminals` (`S1`/`S2`). Because each physical part is outside matrices, `matrix_member` is false and the default scan mode is Direct. `electrical_peripherals::describe` therefore derives the push IDs by appending `/push`, reports the rotary A/B roles, and includes S1 as `encoder-push` plus S2 fixed to GND. The typed encoder sensor key itself remains the unsuffixed physical part ID. These follow directly from `inputs.rs` 57–112 and `electrical_peripherals.rs` 63–111.

`KeymapChange::Encoder` accepts only an ID equal to a `describe` item with `kind == "encoder"`; otherwise it returns `Unknown physical encoder` (`core/src/keymap.rs` 410–421). The ordinary push binding path separately validates the `/push` ID against a reported encoder/press `pressKeyId` (`keymap.rs` 331–365). This distinguishes `left/SW25`/`right/SW25` sensor keys from `left/SW25/push`/`right/SW25/push` push-key bindings.

## Attached VIK encoder row boundary

The checked-in real module catalogue includes `ec11-evqwgd001`, definition ID `vik:ec11-evqwgd001:pcb-ec11-evqwgd001-ec11-evqwgd001`. In `app/src/modules/imported-modules.json`, the row has a rotary profile `{a: gpio1, b: gpio2, common: gnd, driver: ec11}` and required signals `gnd`, `v3v3`, `rgb`, `v5`, `gpio1`, `gpio2`. Its source is VIK repo revision `cd5d16e4cd9137a229fc673412a89d75f4e64553`, path `pcb/ec11-evqwgd001/ec11-evqwgd001.kicad_pcb`, SHA-256 `2b22c0f0b2b99206ac25029b8acd05c75146d9246cb8e3d9ceec5deacae4e033`, license `CERN-OHL-S-2.0`, upstream status Complete. The module README says this board does not support the encoder click, so this row has no push-key ID.

However, no retained `.boardstudio` or project JSON in the inspected `dioxus-keymap-layers/evidence` or `dioxus-case-workspace/evidence` contains an attached module row: the genuine Sofle/REVIUNG project documents inspected have empty/missing `modules` and `moduleDefinitions`. In Core, `modules::host_requirements` folds connected modules into a host connector requirement of kind `vik`, with `rotary=None` and `pressKeyId=None`; it emits the VIK GPIO functions but does not turn this catalogue row into an `encoder` `PeripheralRequirement` (`core/src/modules/electrical.rs` 561–633). Consequently the Core `KeymapChange::Encoder` guard would reject a mounted VIK module ID as `Unknown physical encoder` today. That is source-backed boundary analysis only; no attached VIK row was created or passed through the public UI in this preflight.

The app firmware handoff has a separate module-encoder branch for exact catalogue row `ec11-evqwgd001` (`app/src/firmwareHandoff.ts` 60–70, 90–96). That branch is not evidence of Core `describe` output or of public Dioxus behavior. Its source hash at the inspected integration tree is `8ea6032bc58ae252eeecb5d02a2c39c33cfb0375843f576ed722ed6f16ce7e5a`.

Relevant catalogue-data hashes at the same integration tree:

- `app/src/modules/imported-modules.json`: `dce6dba69eb7f8d672f7ba499442c9e86f656a473016be56943425915d9724b8`
- `app/src/modules/import-manifest.json`: `04c8b5cc5fc3c110d2e63b084ad2563c2b426b3b2a488b198e0d63ba079c3821`

The slightly larger `reference-layered-bindings.boardstudio` (1,585,933 bytes, SHA-256 `c9aa6a4e387fc206fd3a4a0e944b140f095ba8774a8ab5bdc93a99d74e19c9ea`) is also a genuine React public archive and adds a two-step macro plus Function-layer bindings. Use it only if the next UI case needs macro coverage. The separate Case reference `sofle-left-configured.boardstudio` (SHA-256 `889c89a177b3398aa4adc8687a331a8128481eb3d0f10c3d1b38a4a825bc36bd`) contains the same physical encoders but no Keymap layers. No matching physical rotary encoder was found in the retained REVIUNG archives searched.

## Search and limits

I scanned the retained `.boardstudio` archives and project JSON under the two requested evidence roots for the exact rotary generator/profile and for non-empty `modules` / `moduleDefinitions`. The layered Sofle archive above is the smallest suitable physical-encoder-plus-layers fixture found. A genuine attached VIK encoder archive is absent; do not represent one as already tested. No source, catalogue, archive, build output, or browser state was changed.
