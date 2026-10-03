# BoardStudio ZMK handoff

- Target: ZMK v0.3.0
- Controller: `ceoloide/mcu_nice_nano`
- Matrix: 5 rows x 6 columns
- Diodes: `col2row`
- Split transport: `wired-uart`

Unassigned positions use `&none`; edit bindings in BoardStudio or the generated keymap before building. The included workflow uses ZMK v0.3.0. For a local build, install the ZMK v0.3.0 toolchain and run `sh build-local.sh` from the package root. Builds use isolated directories for each shield. Verify the PCB jumper recipe and local routing obligations before assembly.
