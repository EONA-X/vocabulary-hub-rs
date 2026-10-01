#!/usr/bin/env sh
# Runs INSIDE an httpd:2.4 container (CI's `hosting` job, or locally:
#   docker run --rm -v "$PWD:/w" -w /w httpd:2.4 sh scripts/hosting-acceptance.sh <site-dir>
# ) and checks <site-dir>, the build output (one tree per production host:
# eona-x.eu/, vocabulary.eona-x.eu/), in the layouts it is served from:
#
#   bare.test     both trees merged as the web root (what check-conneg.sh assumes)
#   dev.test      dev-voc.eona-x.eu: a stand-in Hub UI dev build at the root,
#                 root .htaccess = hosting/dev/.htaccess + hosting/router.htaccess,
#                 hosting/dev/robots.txt, and both trees merged under voc/
#   prd.test      eona-x.eu: a WordPress-like web root with hosting/router.htaccess
#                 above WordPress's block, and the eona-x.eu tree under voc/
#   prd-voc.test  vocabulary.eona-x.eu: a stand-in Hub UI at the root,
#                 hosting/router.htaccess, and the vocabulary.eona-x.eu tree under voc/
#
# dev and prd share every path; only the root .htaccess and robots.txt differ.
# All but bare.test are served over HTTPS (self-signed), as on OVH.
set -eu
site=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
fail=0
ok() { echo "ok   $*"; }
ko() { echo "FAIL $*"; fail=1; }

apt-get update -qq >/dev/null && apt-get install -qq -y curl openssl >/dev/null

# --- layouts -----------------------------------------------------------------
rm -rf /srv/bare /srv/dev /srv/prd /srv/prd-voc
merged() { mkdir -p "$1"; for tree in "$site"/*/; do cp -r "$tree". "$1"/; done; }
merged /srv/bare

mkdir -p /srv/dev && merged /srv/dev/voc
sh "$here"/scripts/dev-root.sh /srv/dev
printf 'hub ui dev build\n' > /srv/dev/index.html
mkdir -p /srv/dev/catalogue && printf 'hub ui catalogue\n' > /srv/dev/catalogue/index.html

mkdir -p /srv/prd /srv/prd/voc
[ ! -d "$site/eona-x.eu" ] || cp -r "$site/eona-x.eu/." /srv/prd/voc/
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

mkdir -p /srv/prd-voc/voc
[ ! -d "$site/vocabulary.eona-x.eu" ] || cp -r "$site/vocabulary.eona-x.eu/." /srv/prd-voc/voc/
cp "$here"/hosting/router.htaccess /srv/prd-voc/.htaccess
printf 'hub ui\n' > /srv/prd-voc/index.html

# A version an earlier build published and this build no longer knows about:
# the prd deploy never deletes it, so it must still negotiate.
for root in /srv/dev /srv/prd /srv/prd-voc; do
  for d in "$root"/voc/*/*/; do
    ttl=$(ls "$d"*.ttl 2>/dev/null | head -1) || true
    [ -n "$ttl" ] || continue
    # As an earlier build would have written it: same document, its own version
    # in every IRI, so the body checks hold it to its own namespace.
    v=$(basename "$ttl" .ttl)
    for ext in ttl jsonld html; do sed "s#/$v#/v0.0.0-earlier#g" "${ttl%.ttl}.$ext" > "${d}v0.0.0-earlier.$ext"; done
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
<VirtualHost *:443>
  ServerName prd-voc.test
  DocumentRoot /srv/prd-voc
  SSLEngine on
  SSLCertificateFile /tmp/c.pem
  SSLCertificateKeyFile /tmp/k.pem
</VirtualHost>
CONF
echo '127.0.0.1 bare.test dev.test prd.test prd-voc.test' >> /etc/hosts
httpd -t 2>&1 | grep -v AH00558 || true
httpd -k start
until curl -s -o /dev/null http://bare.test/; do sleep 0.2; done

status() { curl -sk -o /dev/null -w '%{http_code} %{redirect_url}' "$@"; }
hdr() { name=$1; shift; curl -sk -o /dev/null -D - "$@" | tr -d '\r' | grep -i "^$name:" || true; }

# <host>/<asset-type>/<slug>/<version>
versions=$(cd "$site" && ls */*/*/*.ttl 2>/dev/null | sed 's/\.ttl$//') || true
[ -n "$versions" ] || { echo "FAIL $site publishes nothing: nothing to check"; exit 1; }

routed() { # <host> <slug> <version>: what dev and prd share
  h=$1; slug=$2; version=$3
  CURL_OPTS=-k sh "$here/scripts/check-conneg.sh" "https://$h/" "$slug" "$version" "$publication" || fail=1
  if out=$(CURL_OPTS=-k sh "$here/scripts/check-conneg.sh" "https://$h/" "$slug" v0.0.0-earlier "$publication"); then
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
  got=$(status "https://$h/voc/${slug%%/*}/")
  [ "${got%% *}" = 404 ] && ok "/voc/<asset-type>/ is not a public path" || ko "/voc/${slug%%/*}/ -> $got (want 404)"
  got=$(status "https://$h/${slug%%/*}/")
  [ "${got%% *}" = 404 ] && ok "/<asset-type>/ has no index (404)" || ko "/${slug%%/*}/ -> $got (want 404)"
}

for hsv in $versions; do
  host=${hsv%%/*}; sv=${hsv#*/}
  slug=${sv%/*}; version=${sv##*/}   # slug = <asset-type>/<slug>
  publication="https://$host/"
  echo "== $host: $slug/$version"

  echo "-- bare"
  sh "$here/scripts/check-conneg.sh" http://bare.test/ "$slug" "$version" "$publication" || fail=1

  echo "-- dev"
  routed dev.test "$slug" "$version"
  got=$(status "http://dev.test/$slug/$version")
  [ "$got" = "301 https://dev.test/$slug/$version" ] && ok "http -> $got" || ko "http://dev.test/$slug/$version -> $got (want 301 to https)"
  got=$(hdr X-Robots-Tag -H 'Accept: text/turtle' "https://dev.test/$slug/$version")
  case "$got" in *noindex*) ok "$got" ;; *) ko "no X-Robots-Tag noindex on $slug/$version" ;; esac
  got=$(hdr Link "https://dev.test/$slug/$version")
  case "$got" in *"<https://$host/$slug/$version>; rel=\"canonical\""*) ok "$got" ;; *) ko "canonical Link on $slug/$version: '$got' (want https://$host/$slug/$version)" ;; esac

  case "$host" in
    eona-x.eu) prd=prd.test; other=prd-voc.test ;;
    vocabulary.eona-x.eu) prd=prd-voc.test; other=prd.test ;;
    *) ko "unknown host tree $host"; continue ;;
  esac
  echo "-- prd ($prd)"
  routed "$prd" "$slug" "$version"
  got=$(hdr X-Robots-Tag "https://$prd/$slug/$version")
  [ -z "$got" ] && ok "prd is indexable" || ko "prd sends $got"
  got=$(status -H 'Accept: text/turtle' "https://$other/$slug/$version")
  [ "${got%% *}" = 404 ] && ok "not on the other host ($other)" || ko "$other/$slug/$version -> $got (want 404: published on $host only)"
done

echo "-- sites around the vocabularies"
# Crawlers must be allowed to fetch dev pages, or they never see the noindex.
got=$(curl -sk https://dev.test/robots.txt)
case "$got" in *"Disallow: /"*) ko "dev robots.txt disallows crawling: the noindex header would never be read" ;; *"Allow: /"*) ok "dev robots.txt lets crawlers read the noindex" ;; *) ko "dev robots.txt: '$got'" ;; esac
for path in / /robots.txt /does-not-exist; do
  got=$(hdr X-Robots-Tag "https://dev.test$path")
  case "$got" in *noindex*) ok "noindex on dev $path" ;; *) ko "no X-Robots-Tag noindex on dev $path" ;; esac
done
got=$(curl -sk https://dev.test/)
[ "$got" = "hub ui dev build" ] && ok "dev root still serves the Hub UI dev build" || ko "dev / -> '$got'"
for path in / /catalogue/; do
  got=$(hdr Link "https://dev.test$path")
  case "$got" in *"<https://vocabulary.eona-x.eu$path>; rel=\"canonical\""*) ok "Hub UI canonical: $got" ;; *) ko "dev $path canonical Link: '$got' (want vocabulary.eona-x.eu)" ;; esac
done
for h in prd.test prd-voc.test dev.test; do
  got=$(status "https://$h/ontology/no-such-slug/v1.0.0")
  [ "${got%% *}" = 404 ] && ok "$h: an unknown slug is a 404" || ko "$h/ontology/no-such-slug/v1.0.0 -> $got (want 404)"
done
got=$(curl -sk https://prd.test/some/wordpress/page)
[ "$got" = "wordpress front controller" ] && ok "other paths still reach WordPress" || ko "WordPress path -> '$got'"

exit $fail
