#!/usr/bin/env bash
# Chrome for Testing pinned on its own schedule (decoupled from any
# webdriver client crate). Version and hashes come from the sibling
# chrome-for-testing.json pin; this script contains no version literal.
set -euo pipefail

[[ $(uname -s) == Linux && $(uname -m) == x86_64 ]] || { echo 'Chrome for Testing requires Linux x64' >&2; exit 1; }

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null && pwd)
pin_json="$script_dir/chrome-for-testing.json"
[[ -f $pin_json ]] || { echo "Missing pin file: $pin_json" >&2; exit 1; }
command -v python3 >/dev/null || { echo 'install-chrome-for-testing.sh requires python3 to read the pin JSON' >&2; exit 1; }

version=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["version"])' "$pin_json")
chrome_sha256=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["chrome_sha256"])' "$pin_json")
chromedriver_sha256=$(python3 -c 'import json,sys; print(json.load(open(sys.argv[1]))["chromedriver_sha256"])' "$pin_json")
[[ -n $version && -n $chrome_sha256 && -n $chromedriver_sha256 ]] || { echo "Invalid pin file: $pin_json" >&2; exit 1; }

destination=${BOARDSTUDIO_CFT_DIR:-$HOME/.cache/boardstudio/chrome-for-testing/$version}
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
    chrome) checksum=$chrome_sha256 ;;
    chromedriver) checksum=$chromedriver_sha256 ;;
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
echo "$destination/chrome-linux64/chrome"
echo "$destination/chromedriver-linux64/chromedriver"
