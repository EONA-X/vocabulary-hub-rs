#!/usr/bin/env sh
# Post-deploy smoke test: every version a host tree of the build publishes
# must negotiate at <base-url>. The tree's name is the production host minting
# its IRIs (public/eona-x.eu, public/vocabulary.eona-x.eu). A tree that
# publishes nothing checks nothing.
#   scripts/smoke-live.sh https://eona-x.eu/ public/eona-x.eu
#   scripts/smoke-live.sh https://dev-voc.eona-x.eu/ public/vocabulary.eona-x.eu
set -eu
base=$1; tree=${2%/}; fail=0
publication="https://$(basename "$tree")/"
# <tree>/<asset-type>/<slug>/<version>.ttl
for ttl in "$tree"/*/*/*.ttl; do
  [ -e "$ttl" ] || continue
  slug_path=${ttl#"$tree"/}; slug_path=${slug_path%/*}
  sh "$(dirname "$0")/check-conneg.sh" "$base" "$slug_path" "$(basename "$ttl" .ttl)" "$publication" || fail=1
done
exit $fail
