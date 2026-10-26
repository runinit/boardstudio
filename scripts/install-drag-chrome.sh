#!/usr/bin/env bash
# Chrome for Testing paired with tools/browser-harness's chromiumoxide 0.9.1.
set -euo pipefail

version=145.0.7622.0
destination=${1:?usage: install-drag-chrome.sh DESTINATION}
mkdir -p "$destination"
archive=$(mktemp)
trap 'rm -f "$archive"' EXIT
curl --proto '=https' --tlsv1.2 -fsSL \
  "https://storage.googleapis.com/chrome-for-testing-public/$version/linux64/chrome-linux64.zip" -o "$archive"
echo "e601cb8f9f38a726ebeeb6c4dac42340368757364046e8b6cb0d7d167c2e9a51  $archive" | sha256sum -c -
unzip -q -o "$archive" -d "$destination"
actual=$("$destination/chrome-linux64/chrome" --version)
[[ "$(awk '{print $NF}' <<< "$actual")" == "$version" ]] || { echo "Unexpected browser: $actual" >&2; exit 1; }
echo "$actual (chromiumoxide 0.9.1)"
