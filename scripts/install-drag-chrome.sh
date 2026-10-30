#!/usr/bin/env bash
# Chrome for Testing paired with tools/browser-harness's chromiumoxide 0.9.1.
set -euo pipefail

version=145.0.7622.0
destination=${1:?usage: install-drag-chrome.sh DESTINATION}
[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || { echo 'Chrome for Testing requires Linux x64' >&2; exit 1; }
mkdir -p "$destination"
stage=$(mktemp -d "$destination/.chrome-install.XXXXXXXX")
installing=0
cleanup() {
  local recovery_failed=0
  if (( installing )); then
    for name in chrome chromedriver; do
      if [[ -e "$stage/installed-$name" ]]; then rm -rf -- "$destination/$name-linux64" || recovery_failed=1; fi
      if [[ -e "$stage/backup-$name" || -L "$stage/backup-$name" ]]; then
        mv -T -- "$stage/backup-$name" "$destination/$name-linux64" || recovery_failed=1
      fi
    done
  fi
  if (( recovery_failed )); then
    echo "Installation rollback failed; backups remain in $stage" >&2
  else
    rm -rf -- "$stage"
  fi
}
trap cleanup EXIT
for name in chrome chromedriver; do
  case "$name" in
    chrome) checksum=e601cb8f9f38a726ebeeb6c4dac42340368757364046e8b6cb0d7d167c2e9a51 ;;
    chromedriver) checksum=92133923c8b36b14712ff8dc1e8cb9130bd278a71693816065c8f25ffe107ac5 ;;
  esac
  archive="$stage/$name.zip"
  curl --proto '=https' --tlsv1.2 -fsSL \
    "https://storage.googleapis.com/chrome-for-testing-public/$version/linux64/$name-linux64.zip" -o "$archive"
  echo "$checksum  $archive" | sha256sum -c -
  unzip -q "$archive" -d "$stage"
  actual=$("$stage/$name-linux64/$name" --version)
  if [[ $name == chromedriver ]]; then actual_version=$(awk '{print $2}' <<< "$actual"); else actual_version=$(awk '{print $NF}' <<< "$actual"); fi
  [[ "$actual_version" == "$version" ]] || { echo "Unexpected $name: $actual" >&2; exit 1; }
  echo "$actual"
done
installing=1
for name in chrome chromedriver; do
  if [[ -e "$destination/$name-linux64" || -L "$destination/$name-linux64" ]]; then
    mv -T -- "$destination/$name-linux64" "$stage/backup-$name"
  fi
  touch "$stage/installed-$name"
  mv -T -- "$stage/$name-linux64" "$destination/$name-linux64"
done
installing=0
