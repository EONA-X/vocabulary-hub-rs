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
writes the tree to upload to the web root's `voc/` folder:

```
public/<asset-type>/<slug>/<version>.ttl      whole ontology / concept scheme
public/<asset-type>/<slug>/<version>.jsonld   same graph
public/<asset-type>/<slug>/<version>.html     human page, one #anchor per term
public/<asset-type>/<slug>/.htaccess          Accept-header negotiation for this slug only
```

**IRI convention: `https://eona-x.eu/<asset-type>/<slug>/<version>#<term>`**, e.g.
`https://eona-x.eu/vocabulary/odrl-profile/v0.0.1#GenericClaim`. `<asset-type>` is one of five
names, each covering concepts of the EU
[asset-classification](http://publications.europa.eu/resource/dataset/asset-classification)
authority table (`ASSET_TYPES` in `src/discover.rs`):

| `<asset-type>` | EU asset-classification concepts it covers |
|---|---|
| `ontology` | Formal ontology |
| `shape` | Application profile, Markup schema |
| `crosswalk` | Alignment |
| `vocabulary` | Terminology, Thesaurus, Dictionary, Glossary, Lexicon, Synonym ring, Folksonomy, Categorisation |
| `codelist` | Code list, Authority file |

A vocabulary is published **only when its root resource** (`owl:Ontology`, or
`skos:ConceptScheme` for code lists and thesauri) **has that IRI**, and the build fails if:

- the path under `https://eona-x.eu/` is not `<asset-type>/<slug>/<version>#`, or `<asset-type>`
  is not one of the five;
- `metadata.ttl` declares a `dcat:type` the asset type does not cover;
- `<version>` is not `v` + `owl:versionInfo`.

Vocabularies still minted under `w3id.org/eonax/...` are listed as skipped: at the current pin
that is all three, until upstream moves them under `https://eona-x.eu/`.

Negotiation (mod_rewrite, first match wins, `q`-values not weighed): `application/ld+json` or
`application/json` → JSON-LD, `text/turtle` → Turtle, anything else including `*/*` → HTML.
`<asset-type>/<slug>/` redirects (302) to the latest version. The `.htaccess` sits in each slug directory,
not at the root, because the root is shared with the rest of eona-x.eu.

```sh
git clone --recurse-submodules <this repo>
cargo run -- content/vocabularies public
scripts/serve-local.sh public                 # stock httpd:2.4 with AllowOverride All, :18088
scripts/check-conneg.sh http://localhost:18088/ vocabulary/odrl-profile v0.0.1
# all three hosting layouts, as CI runs them:
docker run --rm -v "$PWD:/w" -w /w httpd:2.4 sh scripts/hosting-acceptance.sh <site-dir>
```

## Updating the content

`content/` pins content.vocabulary-hub.eona-x.eu. To publish what is on its `main`, run the
manual **`bump:content`** job of a `main` pipeline: it moves `content/` to that latest `main`,
commits and pushes the bump if anything changed (listing the incoming commits), and the push
starts the pipeline that rebuilds and redeploys dev. It pushes with the CI job token as the user
who runs it, so it needs a Maintainer and the project's *CI/CD → Job token permissions → Allow
Git push requests to the repository*.

## Hosting and deploy

Two environments with the **same paths**; only the web root around `voc/` differs. Checked by
`scripts/hosting-acceptance.sh` in CI (stock `httpd:2.4`), together with the bare build output:

| GitLab environment | URL | `OVH_SFTP_DIR` | Web root around `voc/` | Deploy |
|---|---|---|---|---|
| `dev` | https://dev-voc.eona-x.eu/&lt;asset-type&gt;/&lt;slug&gt;/ | `voc` (account rooted at `dev-voc/`) | the Vocabulary Hub UI dev build (prd: https://vocabulary.eona-x.eu/); root `.htaccess` = `hosting/dev/.htaccess` + `hosting/router.htaccess`, plus `hosting/dev/robots.txt` | `deploy:dev`, every `main` pipeline, `mirror` |
| `prd` | https://eona-x.eu/&lt;asset-type&gt;/&lt;slug&gt;/ | `<WP_ROOT>/voc` | WordPress; `hosting/router.htaccess` above `# BEGIN WordPress` | `deploy:prd`, manual after dev, `additive` (a published version is never deleted) |

The dev root files give dev-voc `robots.txt` `Disallow: /`, `X-Robots-Tag: noindex`, HTTP →
HTTPS, and on every response a `rel="canonical"` Link to its production twin:
`https://eona-x.eu<path>` under an asset type (`/vocabulary/…`, `/ontology/…`, …),
`https://vocabulary.eona-x.eu<path>` elsewhere (the Hub UI). The Hub UI must therefore never use
one of the five asset-type names as a top-level path.
In dev, the root files are deployed by CI too (`scripts/dev-root.sh`, added without deleting
anything, so the Hub UI build next to them is safe); in prd, the router block is installed once by
hand above WordPress's. Each environment has its own SFTP account, rooted at its web root.

`scripts/deploy-sftp.sh` uploads with lftp over SFTP to an SFTP-only OVH account, with the
cluster's host key pinned. `OVH_SFTP_USER`, `OVH_SFTP_PASSWORD` and `OVH_SFTP_DIR` are scoped per
GitLab environment. `OVH_SFTP_DIR` is required, must be a plain relative path, and must name a
`voc` folder; the account root (`.`) is accepted only for `additive` uploads, which never delete. Each deploy job
ends with `scripts/smoke-live.sh` against the live URL.

Accounts, domains, the root files and the CI variables are set up by hand, step by step, per the
**eona-x.eu OVH web hosting deployment guide** in
[`eona-architecture-decision-records`](https://gitlab.eona-x.org/eona-x/eona-architecture-decision-records)
(`docs/eona-x-eu-ovh-web-hosting-deployment-guide.md`), which also records why (ADR
`20260929-deploy-eona-x-eu-sites-to-ovh-web-hosting-over-sftp`).

## Related

- Content repo: [`content.vocabulary-hub.eona-x.eu`](https://gitlab.eona-x.org/eona-x/semantic-hub/content.vocabulary-hub.eona-x.eu)
- Authoring/runtime: [`eona-vocabulary-services`](https://gitlab.eona-x.org/eona-x/semantic-hub/eona-vocabulary-services)
- Equivalent pattern elsewhere: [`developer.eona-x.eu`](https://gitlab.eona-x.org/eona-x/web/developer.eona-x.eu) / `content.developer.eona-x.eu`
