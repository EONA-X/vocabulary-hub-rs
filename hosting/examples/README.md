# Hosting examples

Worked examples of hosting patterns that have no generic equivalent: they
encode a specific deployment's choices (a particular dev/preview domain
layout, a particular Hub UI frontend build), not something every
instantiation of this vocabulary hub needs. `hosting/router.htaccess` is the
only `.htaccess` this project ships as generic, ready-to-use content; the
files here are references to adapt, not to deploy unmodified.

Two substitution points, documented in both files as plain-text tokens:

- `{{SITE_BASE_HOST}}` -- the authored-content host, as a bare hostname
  (mirrors `HubConfig.site_base`).
- `{{VENDORED_HOST}}` -- the catalog/vendored host, as a bare hostname
  (mirrors `HubConfig.catalog_iri` / `HubConfig.docs_base`).

Fill both in (a simple find-and-replace) before deploying from an
instantiation that follows this pattern.

## The pattern: previewing every published host under one dev domain

`dev/.htaccess` (plus `dev/robots.txt`) previews, on a single dev/preview
domain, both the Vocabulary Hub UI frontend (`hub-ui.htaccess`, at the dev
domain's root) and the vocabularies of every production host (under their
production paths, reached through `hosting/router.htaccess`). It is not a
publication: it keeps the dev domain out of search results and names each
response's production twin as canonical.

`scripts/examples/dev-root.sh` assembles a dev domain's root `.htaccess` from
`dev/.htaccess` + `hosting/router.htaccess` + `hub-ui.htaccess`, in that
order, matching `hub-ui.htaccess`'s own header comment.
