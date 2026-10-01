#!/usr/bin/env sh
# Acceptance check for one published vocabulary version, against any base URL:
# the acceptance layouts (scripts/hosting-acceptance.sh) before merging,
# dev-voc.eona-x.eu / eona-x.eu after a deploy. Asserts what a client of the
# hash IRI https://eona-x.eu/{asset-type}/{slug}/{version}#Term actually gets
# from {base}: per Accept header, the status, the Content-Type AND a body of
# that format describing that namespace; and that the version-less paths
# redirect to a version on the same host (no ":443" an Apache behind a TLS
# front adds to URLs it builds itself).
#
#   scripts/check-conneg.sh <base-url> <asset-type>/<slug> <version>
#   scripts/check-conneg.sh https://eona-x.eu/ vocabulary/odrl-profile v0.0.1
set -eu
base=${1%/}; slug=$2; version=$3
# CURL_OPTS=-k for a self-signed test certificate.
curl() { command curl ${CURL_OPTS:-} "$@"; }
url="$base/$slug/$version"
host=${base#*://}; host=${host%%/*}
# The published IRIs are always on eona-x.eu, whichever host serves them.
ns="https://eona-x.eu/$slug/$version#"
body=$(mktemp); trap 'rm -f "$body"' EXIT
fail=0
ok() { echo "ok   $*"; }
ko() { echo "FAIL $*"; fail=1; }

expect() { # <accept> <expected content-type> <expected body: turtle|jsonld|html>
  got=$(curl -s -o "$body" -w '%{http_code} %{content_type}' -H "Accept: $1" "$url")
  case "$got" in
    "200 $2"*) ;;
    *) ko "Accept: $1 -> $got (want 200 $2)"; return ;;
  esac
  case "$3" in
    turtle)
      if grep -q '^@prefix ' "$body" && grep -qF "<$ns>" "$body" && ! grep -qi '<!doctype' "$body"; then
        ok "Accept: $1 -> $got, Turtle declaring <$ns>"
      else ko "Accept: $1 -> $got, but the body is not Turtle declaring <$ns>: $(head -c 120 "$body")"; fi ;;
    jsonld)
      if [ "$(tr -d ' \t\r\n' < "$body" | head -c 1)" = "{" ] && grep -qF '"@context"' "$body" && grep -qF "$ns" "$body"; then
        ok "Accept: $1 -> $got, a JSON-LD object with @context and $ns"
      else ko "Accept: $1 -> $got, but the body is not JSON-LD with @context and $ns: $(head -c 120 "$body")"; fi ;;
    html)
      if grep -qi '^<!doctype html>' "$body" && grep -qF "<link rel=\"canonical\" href=\"https://eona-x.eu/$slug/$version\">" "$body"; then
        ok "Accept: $1 -> $got, an HTML page with rel=canonical https://eona-x.eu/$slug/$version"
      else ko "Accept: $1 -> $got, but the body is not the HTML page: $(head -c 120 "$body")"; fi ;;
  esac
}

expect 'text/turtle' 'text/turtle' turtle
expect 'application/ld+json' 'application/ld+json' jsonld
expect 'application/json' 'application/ld+json' jsonld
expect 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8' 'text/html' html
expect '*/*' 'text/html' html

vary=$(curl -s -o /dev/null -D - -H 'Accept: text/turtle' "$url" | tr -d '\r' | grep -i '^vary:' || true)
case "$vary" in
  *[Aa]ccept*) ok "$vary" ;;
  *) ko "no 'Vary: Accept' on the negotiated response" ;;
esac

redirect() { # <path> <status> <location suffix pattern>
  got=$(curl -s -o /dev/null -w '%{http_code} %{redirect_url}' "$base$1")
  case "$got" in
    *:443/*) ko "$1 -> $got (explicit :443 in the redirect)" ;;
    "$2 "*"://$host"$3) ok "$1 -> $got" ;;
    *) ko "$1 -> $got (want $2 to <scheme>://$host$3)" ;;
  esac
}
redirect "/$slug/" 302 "/$slug/v*"
redirect "/$slug" 301 "/$slug/"

exit $fail
