#!/usr/bin/env sh
# WORKED EXAMPLE (see hosting/examples/README.md). Writes a dev-preview
# domain's root files into <out-dir>: .htaccess (the dev overlay, the router
# block, then the Hub UI block) and robots.txt. One source for both an
# instantiation's own dev-deploy CI job and scripts/hosting-acceptance.sh, so
# what CI tests is what it deploys. hosting/examples/dev/.htaccess and
# hosting/examples/hub-ui.htaccess carry unfilled {{SITE_BASE_HOST}} /
# {{VENDORED_HOST}} tokens: fill them in before deploying the output of this
# script.
#   scripts/examples/dev-root.sh <out-dir>
set -eu
here=$(cd "$(dirname "$0")/../.." && pwd)
mkdir -p "$1"
cat "$here/hosting/examples/dev/.htaccess" "$here/hosting/router.htaccess" "$here/hosting/examples/hub-ui.htaccess" > "$1/.htaccess"
cp "$here/hosting/examples/dev/robots.txt" "$1/robots.txt"
