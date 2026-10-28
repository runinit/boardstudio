#!/usr/bin/env bash
# Install the shared Chrome for Testing pair at the requested drag-test path.
set -euo pipefail

destination=${1:?usage: install-drag-chrome.sh DESTINATION}
script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
export BOARDSTUDIO_CFT_DIR="$destination"
exec "$script_dir/install-chrome-for-testing.sh"
