#!/usr/bin/env sh
# Runs INSIDE an httpd:2.4 container (an instantiation's own `hosting` CI job,
# or locally:
#   docker run --rm -v "$PWD:/w" -w /w httpd:2.4 sh scripts/hosting-acceptance.sh <site-dir>
# ) and checks <site-dir>, the build output (one tree per production host the
# build published to, named after that host: <site-dir>/<host>/...), in the
# layouts it is served from. However many host trees <site-dir> contains:
#
#   bare.test       every tree merged as the web root (what check-conneg.sh
#                   assumes)
#   dev.test        a stand-in dev-preview domain: a stand-in Hub UI build
#                   (and its docs/) at the root, root .htaccess =
#                   examples/dev/.htaccess + hosting/router.htaccess +
#                   hosting/examples/hub-ui.htaccess (scripts/examples/dev-root.sh),
#                   and every tree merged under voc/
#   <host>.test     one per host tree: a stand-in Hub UI at the root,
#                   hosting/router.htaccess + hosting/examples/hub-ui.htaccess, and
#                   that host's tree under voc/ -- the layout every
#                   published host gets here. An instantiation that fronts a
#                   given host with its own CMS instead (installing just
#                   hosting/router.htaccess above that CMS's own rules) is
#                   its own deployment concern, outside this generic check.
#
# dev and <host>.test share every path; only the root .htaccess and
# robots.txt differ. All but bare.test are served over HTTPS (self-signed).
set -eu
site=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")/.." && pwd)
fail=0
ok() { echo "ok   $*"; }
ko() { echo "FAIL $*"; fail=1; }

apt-get update -qq >/dev/null && apt-get install -qq -y curl openssl >/dev/null

# --- layouts -----------------------------------------------------------------
rm -rf /srv/bare /srv/dev
merged() { mkdir -p "$1"; for tree in "$site"/*/; do cp -r "$tree". "$1"/; done; }
merged /srv/bare

mkdir -p /srv/dev && merged /srv/dev/voc
sh "$here"/scripts/examples/dev-root.sh /srv/dev
printf 'hub ui dev build\n' > /srv/dev/index.html
mkdir -p /srv/dev/catalogue && printf 'hub ui catalogue\n' > /srv/dev/catalogue/index.html
mkdir -p /srv/dev/docs && printf '[]\n' > /srv/dev/docs/catalog.jsonld

# One production-style layout per host tree the build actually produced
# (whatever their number or names): the Hub UI at the root, that host's own
# tree under voc/.
hosts=
for tree in "$site"/*/; do
  host=$(basename "$tree")
  hosts="$hosts $host"
  root=/srv/$host
  rm -rf "$root"
  mkdir -p "$root/voc"
  cp -r "$tree." "$root/voc/"
  cat "$here"/hosting/router.htaccess "$here"/hosting/examples/hub-ui.htaccess > "$root/.htaccess"
  printf 'hub ui\n' > "$root/index.html"
done
hosts=${hosts# }
[ -n "$hosts" ] || { echo "FAIL $site publishes no host tree: nothing to check"; exit 1; }

# A version an earlier build published and this build no longer knows about:
# a deploy never deletes it, so it must still negotiate.
roots="/srv/dev"
for host in $hosts; do roots="$roots /srv/$host"; done
for root in $roots; do
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
CONF
test_hosts="bare.test dev.test"
for host in $hosts; do
  test_hosts="$test_hosts $host.test"
  cat >> "$conf" <<CONF
<VirtualHost *:443>
  ServerName $host.test
  DocumentRoot /srv/$host
  SSLEngine on
  SSLCertificateFile /tmp/c.pem
  SSLCertificateKeyFile /tmp/k.pem
</VirtualHost>
CONF
done
echo "127.0.0.1 $test_hosts" >> /etc/hosts
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

  prd=$host.test
  echo "-- prd ($prd)"
  routed "$prd" "$slug" "$version"
  got=$(hdr X-Robots-Tag "https://$prd/$slug/$version")
  [ -z "$got" ] && ok "prd is indexable" || ko "prd sends $got"
  for other_host in $hosts; do
    [ "$other_host" != "$host" ] || continue
    other=$other_host.test
    got=$(status -H 'Accept: text/turtle' "https://$other/$slug/$version")
    [ "${got%% *}" = 404 ] && ok "not on the other host ($other)" || ko "$other/$slug/$version -> $got (want 404: published on $host only)"
  done
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
prd_hosts=
for host in $hosts; do prd_hosts="$prd_hosts $host.test"; done
for h in dev.test $prd_hosts; do
  got=$(status "https://$h/ontology/no-such-slug/v1.0.0")
  [ "${got%% *}" = 404 ] && ok "$h: an unknown slug is a 404" || ko "$h/ontology/no-such-slug/v1.0.0 -> $got (want 404)"
done
# The Hub UI routes client-side: a page path loads the app shell, its data
# and the vocabularies never do.
for h in dev.test $prd_hosts; do
  shell=$(cat "/srv/${h%.test}/index.html")
  for path in "/ontologies?ontology=netex" "/crosswalks?crosswalk=crosswalk-netex-datex" /ontologies; do
    got="$(curl -sk "https://$h$path") $(status "https://$h$path")"
    [ "$got" = "$shell 200 " ] && ok "$h$path loads the Hub UI shell" || ko "$h$path -> '$got' (want the Hub UI shell, 200)"
  done
  got=$(status "https://$h/docs/missing.json")
  [ "${got%% *}" = 404 ] && ok "$h: a missing docs/ file is a 404" || ko "$h/docs/missing.json -> $got (want 404)"
done
got="$(curl -sk https://dev.test/docs/catalog.jsonld) $(curl -sk -o /dev/null -w '%{http_code} %{content_type}' https://dev.test/docs/catalog.jsonld)"
[ "$got" = "[] 200 application/ld+json; charset=utf-8" ] && ok "dev serves the Hub UI data as JSON-LD" || ko "dev /docs/catalog.jsonld -> '$got'"
got=$(curl -sk https://dev.test/docs/)
case "$got" in *"Index of"*) ko "dev lists directories (/docs/)" ;; *) ok "dev lists no directories" ;; esac

exit $fail
