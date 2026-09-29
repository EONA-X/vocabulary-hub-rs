#!/usr/bin/env sh
# Post-deploy smoke test: every version <site-dir> publishes must negotiate at
# <base-url>. A build that publishes nothing checks nothing.
#   scripts/smoke-live.sh https://eona-x.eu/ public
set -eu
base=$1; site=$2; fail=0
for ttl in "$site"/*/*.ttl; do
  [ -e "$ttl" ] || continue
  sh "$(dirname "$0")/check-conneg.sh" "$base" "$(basename "$(dirname "$ttl")")" "$(basename "$ttl" .ttl)" || fail=1
done
exit $fail
