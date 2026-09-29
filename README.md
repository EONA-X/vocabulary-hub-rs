# vocabulary-hub.eona-x.eu

Site/publishing project for the **Vocabulary Hub's** presence on
[https://eona-x.eu/](https://eona-x.eu/).

This is where the Vocabulary Hub's eona-x.eu-facing pages get **generated/published from**
(the app/build side). Its content — the ontology/vocabulary files it publishes — lives in the
sibling repo [`content.vocabulary-hub.eona-x.eu`](https://gitlab.eona-x.org/eona-x/semantic-hub/content.vocabulary-hub.eona-x.eu),
mirroring the existing `developer.eona-x.eu` / `content.developer.eona-x.eu` split.

## What this is (and isn't)

- **Is**: the eona-x.eu publishing surface for the Vocabulary Hub — the project that will,
  eventually, take the vocabulary snapshot in `content.vocabulary-hub.eona-x.eu` and build/serve
  it at `https://eona-x.eu/<slug>/<version>` (per the publication scheme decided in
  [backlog#677](https://gitlab.eona-x.org/eona-x/backlog/-/issues/677)).
- **Isn't**: a replacement for [`eona-vocabulary-services`](https://gitlab.eona-x.org/eona-x/semantic-hub/eona-vocabulary-services),
  which remains the authoring/CI pipeline and runtime services (Oxigraph, Prez, eovoc-ui) for
  the Vocabulary Hub. This project doesn't run those services or author vocabularies — it only
  publishes a subset of their output onto eona-x.eu.

## Build

`vocabulary-hub-build` (Rust, on [rdf-sitegen](https://github.com/ds-labs-org/ds-rdf-sitegen-rs))
reads `content/vocabularies/eonax-*/ontology.ttl` — `content/` is the
`content.vocabulary-hub.eona-x.eu` submodule, which pins `eona-vocabularies-reference` — and
writes the tree to upload to the eona-x.eu web root:

```
public/<slug>/<version>.ttl      whole ontology document
public/<slug>/<version>.jsonld   same graph
public/<slug>/<version>.html     human page, one #anchor per term
public/<slug>/.htaccess          Accept-header negotiation for this slug only
```

A vocabulary is published **only when its `owl:Ontology` IRI is
`https://eona-x.eu/<slug>/<version>#`** (backlog#677), and the build fails if `<version>`
disagrees with `owl:versionInfo`. Vocabularies still minted under `w3id.org/eonax/...` are
listed as skipped: at the current pin that is all three, until
[eona-vocabularies-reference#4](https://github.com/EONA-X/eona-vocabularies-reference/pull/4)
merges and the pin is bumped.

Negotiation (mod_rewrite, first match wins, `q`-values not weighed): `application/ld+json` or
`application/json` → JSON-LD, `text/turtle` → Turtle, anything else including `*/*` → HTML.
`<slug>/` redirects (302) to the latest version. The `.htaccess` sits in each slug directory,
not at the root, because the root is shared with the rest of eona-x.eu.

```sh
git clone --recurse-submodules <this repo>
cargo run -- content/vocabularies public
scripts/serve-local.sh public                 # stock httpd:2.4 with AllowOverride All, :18088
scripts/check-conneg.sh http://localhost:18088/ odrl-profile v0.0.1
```

CI runs fmt/clippy/tests, the build, then `check-conneg.sh` against `httpd:2.4` for every
published version. **There is no deploy job yet**; `check-conneg.sh https://eona-x.eu/ …` is
the post-deploy smoke test.

## Related

- Content repo: [`content.vocabulary-hub.eona-x.eu`](https://gitlab.eona-x.org/eona-x/semantic-hub/content.vocabulary-hub.eona-x.eu)
- Authoring/runtime: [`eona-vocabulary-services`](https://gitlab.eona-x.org/eona-x/semantic-hub/eona-vocabulary-services)
- Equivalent pattern elsewhere: [`developer.eona-x.eu`](https://gitlab.eona-x.org/eona-x/web/developer.eona-x.eu) / `content.developer.eona-x.eu`
