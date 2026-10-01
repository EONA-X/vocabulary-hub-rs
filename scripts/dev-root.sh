#!/usr/bin/env sh
# Writes dev-voc.eona-x.eu's root files into <out-dir>: .htaccess (the dev
# overlay, then the router block) and robots.txt. One source for both the
# deploy:dev job and scripts/hosting-acceptance.sh, so what CI tests is what it
# deploys.
#   scripts/dev-root.sh <out-dir>
set -eu
here=$(cd "$(dirname "$0")/.." && pwd)
mkdir -p "$1"
cat "$here/hosting/dev/.htaccess" "$here/hosting/router.htaccess" > "$1/.htaccess"
cp "$here/hosting/dev/robots.txt" "$1/robots.txt"
