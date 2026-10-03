# Build ZMK locally

Install the ZMK v0.3.0 local toolchain (west, CMake, Ninja, the Zephyr SDK, and Python dependencies). Extract this package into an empty directory, then run `sh build-local.sh`. The script initializes an isolated west workspace, fetches the pinned manifest, and builds each shield into a separate directory. Firmware outputs are under `build/<shield>/zephyr/zmk.uf2`. Existing workspaces with a different manifest are rejected. See `config/boards/shields/boardstudio/README.md` for hardware and wiring details.
