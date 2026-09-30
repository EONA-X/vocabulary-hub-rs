#!/usr/bin/env sh
# Runs INSIDE an httpd:2.4 container (CI's `hosting` job, or locally:
#   docker run --rm -v "$PWD:/w" -w /w httpd:2.4 sh scripts/hosting-acceptance.sh <site-dir>
# ) and checks <site-dir> in the three layouts it is served from:
#
#   bare.test  the build output as the web root (what check-conneg.sh assumes)
#   dev.test   dev-voc.eona-x.eu: a stand-in eona-x.eu dev site at the root,
#              root .htaccess = hosting/dev/.htaccess + hosting/router.htaccess,
#              hosting/dev/robots.txt, and the build output under voc/
#   prd.test   eona-x.eu: a WordPress-like web root with hosting/router.htaccess
#              above WordPress's block, and the build output under voc/
#
# dev and prd share every path; only the root .htaccess and robots.txt differ.
# dev.test and prd.test are served over HTTPS (self-signed), as on OVH.
set -eu
site=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
fail=0
ok() { echo "ok   $*"; }
ko() { echo "FAIL $*"; fail=1; }

apt-get update -qq >/dev/null && apt-get install -qq -y curl openssl >/dev/null

# --- layouts -----------------------------------------------------------------
rm -rf /srv/bare /srv/dev /srv/prd
cp -r "$site" /srv/bare

mkdir -p /srv/dev && cp -r "$site" /srv/dev/voc
cat "$here"/hosting/dev/.htaccess "$here"/hosting/router.htaccess > /srv/dev/.htaccess
cp "$here"/hosting/dev/robots.txt /srv/dev/
printf 'eona-x.eu dev site\n' > /srv/dev/index.html

mkdir -p /srv/prd && cp -r "$site" /srv/prd/voc
printf 'wordpress front controller\n' > /srv/prd/index.php
{ cat "$here"/hosting/router.htaccess; cat <<'WP'

# BEGIN WordPress
<IfModule mod_rewrite.c>
RewriteEngine On
RewriteBase /
RewriteRule ^index\.php$ - [L]
RewriteCond %{REQUEST_FILENAME} !-f
RewriteCond %{REQUEST_FILENAME} !-d
RewriteRule . /index.php [L]
</IfModule>
# END WordPress
WP
} > /srv/prd/.htaccess

# A version an earlier build published and this build no longer knows about:
# the prd deploy never deletes it, so it must still negotiate.
for root in /srv/dev /srv/prd; do
  for d in "$root"/voc/*/; do
    ttl=$(ls "$d"*.ttl 2>/dev/null | head -1) || true
    [ -n "$ttl" ] || continue
    for ext in ttl jsonld html; do cp "${ttl%.ttl}.$ext" "${d}v0.0.0-earlier.$ext"; done
  done
done

# --- httpd -------------------------------------------------------------------
conf=/usr/local/apache2/conf/httpd.conf
sed -i -e 's/^#\(LoadModule \(rewrite\|ssl\|socache_shmcb\|headers\)_module\)/\1/' "$conf"
openssl req -x509 -newkey rsa:2048 -nodes -days 1 -subj /CN=test -keyout /tmp/k.pem -out /tmp/c.pem 2>/dev/null
cat >> "$conf" <<'CONF'
Listen 443
ServerName localhost
<Directory /srv>
  AllowOverride All
  Require all granted
</Directory>
<VirtualHost *:80>
  ServerName bare.test
  DocumentRoot /srv/bare
</VirtualHost>
<VirtualHost *:80>
  ServerName dev.test
  DocumentRoot /srv/dev
</VirtualHost>
<VirtualHost *:443>
  ServerName dev.test
  DocumentRoot /srv/dev
  SSLEngine on
  SSLCertificateFile /tmp/c.pem
  SSLCertificateKeyFile /tmp/k.pem
</VirtualHost>
<VirtualHost *:443>
  ServerName prd.test
  DocumentRoot /srv/prd
  SSLEngine on
  SSLCertificateFile /tmp/c.pem
  SSLCertificateKeyFile /tmp/k.pem
</VirtualHost>
CONF
echo '127.0.0.1 bare.test dev.test prd.test' >> /etc/hosts
httpd -t 2>&1 | grep -v AH00558 || true
httpd -k start
until curl -s -o /dev/null http://bare.test/; do sleep 0.2; done

status() { curl -sk -o /dev/null -w '%{http_code} %{redirect_url}' "$@"; }
hdr() { name=$1; shift; curl -sk -o /dev/null -D - "$@" | tr -d '\r' | grep -i "^$name:" || true; }

versions=$(cd "$site" && ls */*.ttl 2>/dev/null | sed 's/\.ttl$//') || true
[ -n "$versions" ] || { echo "FAIL $site publishes nothing: nothing to check"; exit 1; }

routed() { # <host> <slug> <version>: what dev and prd share
  h=$1; slug=$2; version=$3
  CURL_OPTS=-k sh "$here/scripts/check-conneg.sh" "https://$h/" "$slug" "$version" || fail=1
  if out=$(CURL_OPTS=-k sh "$here/scripts/check-conneg.sh" "https://$h/" "$slug" v0.0.0-earlier); then
    ok "an earlier, no longer built version still negotiates"
  else
    echo "$out"; ko "earlier version v0.0.0-earlier lost"
  fi
  got=$(status "https://$h/$slug")
  [ "$got" = "301 https://$h/$slug/" ] && ok "/$slug -> $got" || ko "/$slug -> $got (want 301 to /$slug/)"
  got=$(status "https://$h/$slug/")
  case "$got" in "302 https://$h/$slug/v"*) ok "/$slug/ -> $got" ;; *) ko "/$slug/ -> $got (want 302 to /$slug/<version>, never /voc/)" ;; esac
  got=$(status "https://$h/voc/$slug/$version.ttl")
  [ "${got%% *}" = 404 ] && ok "/voc/$slug/ is not a public path" || ko "/voc/$slug/$version.ttl -> $got (want 404)"
  got=$(status "https://$h/voc/")
  [ "${got%% *}" = 404 ] && ok "/voc/ is not a public path" || ko "/voc/ -> $got (want 404)"
}

for sv in $versions; do
  slug=${sv%%/*}; version=${sv#*/}
  echo "== $slug/$version"

  echo "-- bare"
  sh "$here/scripts/check-conneg.sh" http://bare.test/ "$slug" "$version" || fail=1

  echo "-- dev"
  routed dev.test "$slug" "$version"
  got=$(status "http://dev.test/$slug/$version")
  [ "$got" = "301 https://dev.test/$slug/$version" ] && ok "http -> $got" || ko "http://dev.test/$slug/$version -> $got (want 301 to https)"
  got=$(hdr X-Robots-Tag -H 'Accept: text/turtle' "https://dev.test/$slug/$version")
  case "$got" in *noindex*) ok "$got" ;; *) ko "no X-Robots-Tag noindex on $slug/$version" ;; esac
  got=$(hdr Link "https://dev.test/$slug/$version")
  case "$got" in *"<https://eona-x.eu/$slug/$version>; rel=\"canonical\""*) ok "$got" ;; *) ko "canonical Link on $slug/$version: '$got'" ;; esac

  echo "-- prd"
  routed prd.test "$slug" "$version"
  got=$(hdr X-Robots-Tag "https://prd.test/$slug/$version")
  [ -z "$got" ] && ok "prd is indexable" || ko "prd sends $got"
done

echo "-- sites around the vocabularies"
got=$(curl -sk https://dev.test/robots.txt | grep -c '^Disallow: /$' || true)
[ "$got" = 1 ] && ok "dev robots.txt disallows everything" || ko "dev robots.txt does not disallow /"
got=$(curl -sk https://dev.test/)
[ "$got" = "eona-x.eu dev site" ] && ok "dev root still serves the eona-x.eu dev site" || ko "dev / -> '$got'"
got=$(hdr Link https://dev.test/)
case "$got" in *"<https://eona-x.eu/>; rel=\"canonical\""*) ok "dev site canonical: $got" ;; *) ko "dev / canonical Link: '$got'" ;; esac
got=$(curl -sk https://prd.test/some/wordpress/page)
[ "$got" = "wordpress front controller" ] && ok "other paths still reach WordPress" || ko "WordPress path -> '$got'"

exit $fail
