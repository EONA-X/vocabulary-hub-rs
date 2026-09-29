#!/usr/bin/env sh
# Serves <site-dir> with a stock Apache 2.4 honouring .htaccess, as the static
# host does, on http://localhost:<port>/ (default 18088). Stop with
# `docker rm -f vocabulary-hub-local`.
set -eu
site=$(cd "$1" && pwd); port=${2:-18088}
docker rm -f vocabulary-hub-local >/dev/null 2>&1 || true
docker run -d --name vocabulary-hub-local -p "$port:80" -v "$site:/usr/local/apache2/htdocs:ro" httpd:2.4 sh -c '
  conf=/usr/local/apache2/conf/httpd.conf
  sed -i -e "s/^#\(LoadModule rewrite_module\)/\1/" -e "s/AllowOverride None/AllowOverride All/" "$conf"
  exec httpd-foreground' >/dev/null
until curl -s -o /dev/null "http://localhost:$port/"; do sleep 0.2; done
echo "serving $site on http://localhost:$port/"
