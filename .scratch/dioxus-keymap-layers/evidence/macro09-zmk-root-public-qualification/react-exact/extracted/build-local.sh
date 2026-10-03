#!/bin/sh
set -eu
package_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$package_dir"
command -v west >/dev/null 2>&1 || { echo 'Install the ZMK v0.3.0 local toolchain and west first.' >&2; exit 1; }
if [ ! -d .west ]; then
  if west topdir >/dev/null 2>&1; then
    echo 'Extract this package outside an existing west workspace.' >&2; exit 1
  fi
  west init -l config
else
  manifest_dir=$(west config manifest.path)
  [ "$manifest_dir" = config ] || { echo 'This workspace uses a different manifest.' >&2; exit 1; }
fi
west update
west zephyr-export
west build -s zmk/app -d build/boardstudio_left -b nice_nano_v2 -- -DSHIELD=boardstudio_left -DZMK_CONFIG="$package_dir/config"
west build -s zmk/app -d build/boardstudio_right -b nice_nano_v2 -- -DSHIELD=boardstudio_right -DZMK_CONFIG="$package_dir/config"
