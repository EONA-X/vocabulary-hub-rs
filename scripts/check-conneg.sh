#!/usr/bin/env sh
# Acceptance check for one published vocabulary version, against any base URL:
# a local Apache (scripts/serve-local.sh) before merging, eona-x.eu after a
# deploy. Asserts what a client of the hash IRI {base}{slug}/{version}#Term
# actually gets, per Accept header, and that the bare slug points at a version.
#
#   scripts/check-conneg.sh <base-url> <slug> <version>
#   scripts/check-conneg.sh https://eona-x.eu/ odrl-profile v0.0.1
set -eu
base=${1%/}; slug=$2; version=$3
# CURL_OPTS=-k for a self-signed test certificate.
curl() { command curl ${CURL_OPTS:-} "$@"; }
url="$base/$slug/$version"
fail=0

expect() { # <accept> <expected content-type prefix>
  got=$(curl -s -o /dev/null -w '%{http_code} %{content_type}' -H "Accept: $1" "$url")
  case "$got" in
    "200 $2"*) echo "ok   Accept: $1 -> $got" ;;
    *) echo "FAIL Accept: $1 -> $got (want 200 $2)"; fail=1 ;;
  esac
}

expect 'text/turtle' 'text/turtle'
expect 'application/ld+json' 'application/ld+json'
expect 'application/json' 'application/ld+json'
expect 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8' 'text/html'
expect '*/*' 'text/html'

vary=$(curl -s -o /dev/null -D - -H 'Accept: text/turtle' "$url" | tr -d '\r' | grep -i '^vary:' || true)
case "$vary" in
  *[Aa]ccept*) echo "ok   $vary" ;;
  *) echo "FAIL no 'Vary: Accept' on the negotiated response"; fail=1 ;;
esac

loc=$(curl -s -o /dev/null -w '%{http_code} %{redirect_url}' "$base/$slug/")
case "$loc" in
  "302 "*"/$slug/"*) echo "ok   $slug/ -> $loc" ;;
  *) echo "FAIL $slug/ -> $loc (want 302 to a version)"; fail=1 ;;
esac

exit $fail
