#!/bin/sh
# Runs the model-lookup browser pilot or its tests with only loopback networking, a cleared
# environment, Node masked and a private writable temp dir. Chrome has no sandbox of its own
# and the host filesystem stays readable. Use the matching system Chromium and ChromeDriver.
# Usage: MODEL_LOOKUP_TMP_PARENT=/absolute/private/parent sh model-lookup-pilot.sh pilot|tests [pilot.py args]
set -eu
test "$#" -ge 1 || { echo 'expected pilot|tests' >&2; exit 2; }
test -n "${MODEL_LOOKUP_TMP_PARENT:-}" || { echo 'set MODEL_LOOKUP_TMP_PARENT to a trusted absolute directory' >&2; exit 2; }
case "$MODEL_LOOKUP_TMP_PARENT" in /*) ;; *) echo 'parent path must be absolute' >&2; exit 2;; esac
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
case "$1" in pilot) script=model-lookup-pilot.py;; tests) script=test-model-lookup-pilot.py;; *) echo 'expected pilot or tests' >&2; exit 2;; esac
shift
run=$(mktemp -d "$MODEL_LOOKUP_TMP_PARENT/ml-XXXXXXXX")
trap 'rm -rf -- "$run"' EXIT HUP INT TERM
unshare --user --map-root-user --net /bin/sh -eu -c '
  ip link set lo up
  test "$(ip -o link show | cut -d: -f2 | tr -d " ")" = lo
  test -z "$(ip route)"
  test -z "$(ip -6 route)"
  run=$1; script=$2; shift 2
  exec bwrap --ro-bind / / --dev /dev --proc /proc --bind "$run" "$run" \
    --ro-bind /dev/null /usr/bin/node --clearenv --setenv PATH /usr/bin:/bin \
    --setenv HOME "$run" --setenv TMPDIR "$run" --setenv XDG_CONFIG_HOME "$run" \
    --setenv XDG_CACHE_HOME "$run" --setenv PYTHONDONTWRITEBYTECODE 1 --setenv CHROME_BIN /usr/bin/chromium \
    --setenv CHROMEDRIVER /usr/bin/chromedriver \
    /bin/sh -eu -c '\''test ! -x /usr/bin/node; exec python3 -EsB "$@"'\'' sh "$script" "$@"
' sh "$run" "$dir/$script" "$@"
