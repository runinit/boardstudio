# ZMK keymap editing and export reference

Verified against official documentation and observable Studio interfaces on 2026-09-30. Scope: local BoardStudio editing and ZMK source export. Runtime USB/BLE connection is deferred. Implementation should be independently designed around BoardStudio's Rust domain model and existing UI conventions; no Studio implementation source was inspected or copied.

## Binding and layer contract

A `.keymap` is preprocessed devicetree. Include `<behaviors.dtsi>` and `<dt-bindings/zmk/keys.h>`, then emit a root node containing a keymap with `compatible = "zmk,keymap"`. Each layer needs one binding per physical key position; `display-name` supplies a human label. Optional `sensor-bindings` describes sensors separately. Layer indices start at zero and follow declaration order; higher active indices take priority regardless of activation order. Export bindings in the board's physical key order, rather than display or object-map order. [Official keymap structure](https://zmk.dev/docs/keymaps).

Recommended domain choice: store stable layer IDs and resolve numeric indices during export. Reordering then preserves intended layer references. Reject deleting referenced layers or require explicit reassignment. Layer operations include `&mo index` (momentary), `&to index` (exclusive except base), `&tog index` (toggle), and sticky layer `&sl index`. [Layer behaviors](https://zmk.dev/docs/keymaps/behaviors/layers), [behavior catalog](https://zmk.dev/docs/keymaps/behaviors).

## Key presses and dual-role keys

`&kp A` sends a keycode. Modifier functions wrap a keycode, e.g. `&kp LC(RA(B))`; available functions are `LS`, `LC`, `LA`, `LG`, `RS`, `RC`, `RA`, `RG`. A modifier key itself is also a valid keycode. A modified key press does not need a macro. Prefer a structured keycode plus modifier list over unrestricted DTS strings; validate aliases against the selected firmware version. [Modifier documentation](https://zmk.dev/docs/keymaps/modifiers).

`&mt LSHIFT A` holds the first keycode and taps the second. `&lt 1 A` holds a layer and taps a keycode. Stock mod-tap uses hold-preferred resolution; stock layer-tap uses tap-preferred. Timing/flavor changes belong to behavior definitions or node overrides, not additional positional parameters. Per-key timing requires distinct custom hold-tap instances. Custom hold-taps forward one parameter to each child; children requiring more parameters need a wrapping macro. [Hold-tap documentation](https://zmk.dev/docs/keymaps/behaviors/hold-tap).

## Macros

Emit a labeled macro node before binding its label, e.g. `bs_macro_1: bs_macro_1 { compatible = "zmk,behavior-macro"; #binding-cells = <0>; wait-ms = <40>; tap-ms = <40>; bindings = <&kp A &kp B>; };` under the root's `macros` node. A key uses `&bs_macro_1`.

Represent tap/press/release, waits, and pause-until-release as ordered steps. Export mode controls (`&macro_tap`, `&macro_press`, `&macro_release`), timing controls, and `&macro_pause_for_release` faithfully; press-only steps must have deliberate release handling. Parameterized variants require matching compatible strings and binding-cell counts (one or two), plus explicit parameter-forwarding controls. The default behavior queue has 64 events; taps consume two. Devicetree limits bindings to 256. Warn for long macros, and document required queue configuration rather than promising unrestricted sequences. [Macro specification](https://zmk.dev/docs/keymaps/behaviors/macros).

## Encoders

Rotation is a sensor binding, while an encoder push button is an ordinary matrix key. `sensor-bindings = <&inc_dec_kp C_VOL_UP C_VOL_DN>;` uses clockwise first, counterclockwise second. Multiple entries follow hardware sensor order. [Encoder behavior](https://zmk.dev/docs/features/encoders).

For arbitrary actions, generate a labeled `zmk,behavior-sensor-rotate` node with `#sensor-binding-cells = <0>` and `bindings = <&some_cw_action>, <&some_ccw_action>;`, then reference it from each layer's `sensor-bindings`. The variable variant uses `zmk,behavior-sensor-rotate-var`, two sensor binding cells, and single-parameter children. [Sensor rotation contract](https://zmk.dev/docs/keymaps/behaviors/sensor-rotate).

Keymap output alone cannot create hardware support. EC11 nodes need A/B GPIOs and `steps`; the board needs an ordered `zmk,keymap-sensors` sensor list, enabled sensor status, and encoder configuration. Typical `.conf` settings are `CONFIG_EC11=y` and `CONFIG_EC11_TRIGGER_GLOBAL_THREAD=y`. Preserve existing board declarations or flag missing physical configuration instead of inventing GPIOs. [Encoder configuration](https://zmk.dev/docs/config/encoders), [hardware integration](https://zmk.dev/docs/hardware-integration/encoders).

## Extensibility and compatibility

The official catalog also includes transparent/none, sticky keys, key toggle, caps word, repeat, mouse, Bluetooth/output, lighting, reset, and power behaviors, plus user-defined tap dances and mod morphs. Reserve an explicit custom behavior reference with typed parameters and a declaration/firmware dependency; a raw label alone does not define a behavior. Validate arity and references. Claims of complete ZMK support additionally require combos, conditional layers, input processors, custom definitions, and firmware-dependent configuration. [Behavior catalog](https://zmk.dev/docs/keymaps/behaviors).

Studio runtime cannot define arbitrary new behaviors or add layers beyond firmware capacity; these limitations do not prevent local devicetree generation. Existing Studio-persisted mappings may override newly flashed `.keymap` defaults until Restore Stock Settings is used. Label this release as source editing/export, not live synchronization. [Studio capabilities and persistence](https://zmk.dev/docs/features/studio).

## Clean-room UX observations

[DYA Studio](https://studio.dya.cormoran.works/keymap) was inspected in its public demo: named layer buttons with reorder/rename/add/delete, selectable physical key geometry, a behavior picker with key press/layer-tap/mod-tap/none/transparent shortcuts, eight modifier toggles, searchable keycodes, and a separate rotary configuration with directional assignments. Apply these interaction principles using BoardStudio's own component styling and Rust contracts. Do not copy layout/assets or assume its device protocol belongs in this release.

[ZMK Studio](https://zmk.studio/) exposed a welcome and USB connection button without connected hardware. No hidden editor behavior was inferred. Its official docs establish layer naming and assignment of existing user-defined/predefined behaviors; firmware property editing and encoder assignment have different support levels from local source generation. [Official Studio capabilities](https://zmk.dev/docs/features/studio).

## Validation boundaries

Use approved public Rust edit/export APIs to test persistence, invalid keycodes/arity, modifier composition, macro references, layer reorder/delete semantics, physical binding order, sensor count/order, directional action export, and deterministic declaration labels. Browser workflows should verify selection, behavior-specific parameters, layer changes, macro creation, encoder directions, and export feedback. An actual ZMK build against a pinned board/firmware target is stronger evidence than string assertions; absent that build, report source validation rather than compiled firmware compatibility. HID delivery timing and encoder electrical correctness need real firmware/device testing.

## Pinned ZMK v0.3.0 verification

The current documentation above is supplemented by the export target's tagged sources. In v0.3.0 the fixed sensor behavior has `compatible = "zmk,behavior-sensor-rotate"`, `#sensor-binding-cells = <0>`, complete directional `bindings`, and optional `tap-ms` (default 5). The variable form has `compatible = "zmk,behavior-sensor-rotate-var"`, `#sensor-binding-cells = <2>`, and two behavior phandles. The built-in `inc_dec_kp` is a variable instance with two `kp` children. These are **sensor** cells, not ordinary `#binding-cells`. [Fixed schema](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/dts/bindings/behaviors/zmk,behavior-sensor-rotate.yaml), [variable schema](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/dts/bindings/behaviors/zmk,behavior-sensor-rotate-var.yaml), [built-in declaration](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/dts/behaviors/sensor_rotate_key_press.dtsi).

Register the global sensor order once in common board definitions:

```dts
sensors: sensors {
    compatible = "zmk,keymap-sensors";
    sensors = <&left_encoder &right_encoder>;
    triggers-per-rotation = <20>;
};
```

The registration is discovered through the first `zmk,keymap-sensors` instance; no `chosen` entry is needed. Firmware derives each sensor index directly from this property. On a split, preserve this **same complete order on both halves**, with both nodes disabled in common definitions and only the physically local node enabled in each half's overlay. Remote disabled entries remain in the list; initialization skips missing local devices without compacting their indices. The stock Kyria demonstrates this pattern. Values for `steps`, triggers, and GPIOs are hardware-specific. [Registration schema](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/dts/bindings/zmk,keymap-sensors.yaml), [index macros](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/include/zmk/sensors.h), [sensor initialization](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/src/sensors.c), [common Kyria declarations](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/boards/shields/kyria/kyria_common.dtsi), [left overlay](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/boards/shields/kyria/kyria_left.overlay), [right overlay](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/boards/shields/kyria/kyria_right.overlay).

### Explicit wait versus delay setting

In v0.3.0, `&macro_wait_time 100` only changes the persistent delay setting for subsequent normal behavior events. It does not queue a wait immediately, and alone at the end of a macro adds no delay. Tap mode enqueues a press followed by `tap-ms`, then release followed by `wait-ms`; press/release modes enqueue their single event followed by `wait-ms`. [Macro implementation](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/src/behaviors/behavior_macro.c).

Recommended lowering for a UI step named “Wait 100 ms,” assuming prior tap mode and default inter-action delay 40:

```dts
<&macro_wait_time 100>,
<&macro_press &none>,
<&macro_wait_time 40>,
<&macro_tap>
```

This is an implementation recommendation derived from the tagged sources: the `none` press queues a no-op event followed by the requested delay, then restores delay and mode. Restore the actual previous mode and delay, rather than always the example defaults. It consumes one queue event and avoids introducing an extra tap-time delay. Queue processing invokes each event and waits afterward; `none` press/release produce no key output. [Queue implementation](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/src/behavior_queue.c), [none behavior](https://github.com/zmkfirmware/zmk/blob/v0.3.0/app/src/behaviors/behavior_none.c).
