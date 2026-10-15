# vocabulary-hub-rs

**[Live demo](https://eona-x.github.io/vocabulary-hub-rs/)** — built by
[`examples/demo.rs`](examples/demo.rs) from the sample vocabularies under
[`examples/vocabularies/`](examples/vocabularies/), redeployed on every push
to `main` by [`.github/workflows/pages.yml`](.github/workflows/pages.yml).

A small, host-agnostic Rust static-site generator for publishing RDF
vocabularies (ontologies, SHACL/application-profile shapes, crosswalks,
controlled vocabularies, code lists) as a content-negotiated site: every
version of every asset is served as Turtle, JSON-LD, and a human-readable
HTML page, routed through a plain Apache `.htaccess` convention. It also
builds a DCAT catalog of everything it finds, for a catalog browser or any
other consumer to fetch at runtime.

It has no opinion about who you are or what your vocabularies look like
beyond a plain directory convention: `<slug>/<version>/` directories, each
with a graph file. Branding, hosting, and publication rules are all
injected by the caller — this crate ships sane, clearly-labelled defaults so
`cargo run` works out of the box, and a path to replace each of them when it
isn't enough.

## What it does

Given a directory of `<slug>/<version>/` vocabularies, the build:

1. **Discovers** which `<slug>/<version>` directories are published, and
   from which graph file, via a [`Discoverer`](#the-discoverer-trait).
2. **Writes a site** (`site::write_site`): for each published asset version,
   `{asset-type}/{slug}/{version}.ttl`, `.jsonld`, and `.html`, plus one
   `.htaccess` per `{asset-type}/{slug}/` directory that content-negotiates
   on the `Accept` header (JSON-LD, then Turtle, then HTML as the fallback —
   `*/*` included, so a browser following a link lands on the human page).
   Terms are hash IRIs (`{asset-type}/{slug}/{version}#Term`), so one
   document answers for every term in that version.
3. **Optionally writes a DCAT catalog** (`catalog::write_catalog`): a
   `docs/` tree — per-slug manifests, every version's graph in several
   formats, and `catalog.ttl` / `catalog.jsonld` with DCAT versioning —
   minted under `HubConfig::catalog_iri`, for a catalog UI to consume.

Routing is deliberately simple: mod_rewrite rather than Apache's
`mod_negotiation` type maps, because every Apache-style static host enables
mod_rewrite. The trade-off is that `q`-values aren't weighed — the first
matching media type wins, in a fixed order — which is enough for RDF clients
(which always name a real RDF media type) and humans (who get HTML from
everything else).

## Quickstart

```sh
cargo run -- <vocabularies-dir> <out-dir> [<docs-dir>]
```

- `<vocabularies-dir>`: a checkout laid out as `<slug>/<version>/`
  directories (see `tests/fixtures/vocabularies/` for minimal examples).
  `DefaultDiscoverer` (the one `cargo run` wires up) publishes any such
  directory containing an `ontology.ttl` or `alignment.ttl`.
- `<out-dir>`: receives the published site, one tree per host the
  discovered assets publish to.
- `<docs-dir>` (optional): if given, also writes the catalog data tree
  described above. Set `BUILT_AT` in the environment to stamp it with a
  real build timestamp (CI can pass the commit timestamp); it defaults to
  `"local build"`.

To check the result serves correctly under a real Apache, see
`scripts/serve-local.sh` and `scripts/check-conneg.sh`, and
`scripts/hosting-acceptance.sh` for the full `.htaccess` layout under a
stock `httpd` container.

## `HubConfig`

[`config::HubConfig`](src/config.rs) is pure data — everything a build needs
that isn't the vocabularies themselves. It does not hold a `Discoverer`; the
caller owns one alongside a `HubConfig` and passes both into `write_site`
and `write_catalog` explicitly. `HubConfig::default()` returns a generic,
obviously-fake configuration (`https://vocabulary-hub.example/`, RFC 2606)
so the crate is useful out of the box without masquerading as a real site.

| Field | Type | Configures |
|---|---|---|
| `site_base` | `String` | Default/primary content host base URL. |
| `stylesheets` | `[(&'static str, &'static str); 3]` | `(filename, CSS content)` pairs linked by every vocabulary HTML page. |
| `brand_name` | `String` | Header mark's accessible label. |
| `brand_link` | `String` | URL the header mark and footer link point to. |
| `brand_link_label` | `String` | Visible text of the footer's home link. |
| `brand_mark_svg` | `&'static str` | Inline SVG markup for the header logo mark. |
| `footer_text` | `String` | Footer text. |
| `catalog_iri` | `String` | Base IRI minting the DCAT catalog and every asset/version IRI. |
| `docs_base` | `String` | Base URL where the `docs/` catalog data tree is served (used in `landingPage`, `thumbnail`, `downloadURL`). |
| `catalog_title` | `String` | DCAT catalog `dct:title`. |
| `catalog_description` | `String` | DCAT catalog `dct:description`. |

## The `Discoverer` trait

[`discoverer::Discoverer`](src/discoverer.rs) says which `<slug>/<version>`
directories under a checkout are published, and from which file — nothing
else about the build depends on how that decision is made. This is the seam
a real deployment usually needs to customize: a different directory layout,
a split across multiple hosts, a gate on some upstream CI check or
front-matter field, vendoring rules distinguishing "we wrote this" from
"this is our copy of someone else's standard".

```rust
pub trait Discoverer {
  fn discover(&self, dir: &Path, default_base: &str) -> Result<Discovery, String>;
  fn version_dirs(&self, dir: &Path) -> Result<Vec<String>, String>;
  fn graph_file(&self, dir: &Path) -> Result<String, String>;
}
```

[`default_discovery::DefaultDiscoverer`](src/default_discovery.rs) is the
bundled, ready-to-use implementation: every `<slug>/<version>` with an
`ontology.ttl` or `alignment.ttl` is published under the single
`default_base` given, with no host-splitting and no vendoring rules. Its
asset type comes from an optional `metadata.ttl`'s `dcat:type`; absent that,
it defaults to `"ontology"`. It's enough for a single-repository, single-host
publication straight out of the box (it's what `cargo run`/`main.rs` wires
up), and a readable reference for writing your own against a real content
repository with its own layout and gating rules.

## Theming

[`theme::default`](src/theme/default.rs) ships three plain CSS files
(`tokens.css`, `components.css`, `ontology.css`) — design tokens, generic
page/component styles, and vocabulary-page-specific styles — just enough
valid CSS to render a readable page. They're wired into
`HubConfig::default()`'s `stylesheets` field and nothing else; swapping
theme/branding is purely a matter of building a `HubConfig` whose
`stylesheets`, `brand_name`, `brand_link`, `brand_link_label`,
`brand_mark_svg`, and `footer_text` point at your own content instead of
calling `HubConfig::default()`.

## Instantiating this for your own organization

Using this crate for a real deployment means driving the library directly
(`vocabulary_hub_rs::{catalog, config, discoverer, site}`, as `main.rs`
does) with your own values in place of the defaults, rather than forking
it. Four things typically change:

1. **Your own `HubConfig`.** Build one with your organization's
   `site_base`, `catalog_iri`, `docs_base`, brand fields, and catalog
   title/description, instead of `HubConfig::default()`.
2. **Your own `Discoverer`**, if `DefaultDiscoverer`'s single-host,
   no-vendoring-rules behavior isn't enough — e.g. you publish to more than
   one host, need a vendored-vs-authored split, or want to gate publication
   on a field your own content repository's quality-check tooling sets.
   Implement the three `Discoverer` methods against your own layout and
   pass it to `write_site`/`write_catalog` in place of `DefaultDiscoverer`.
3. **Your own theme CSS.** Write your own `(filename, CSS)` pairs (or
   however many stylesheets you need) and set `HubConfig.stylesheets` to
   them; `theme::default` is a placeholder, not something to extend.
4. **Your own hosting glue.** `hosting/` and `.gitlab-ci.yml` in this
   generic repository are templates for an instantiation to copy and adapt,
   not something this repository's own CI demonstrates end-to-end. Two
   placeholder tokens are the documented substitution points:
   - `{{SITE_BASE_HOST}}` — the authored-content host as a bare hostname
     (mirrors `HubConfig.site_base`).
   - `{{VENDORED_HOST}}` — the catalog/vendored host as a bare hostname
     (mirrors `HubConfig.catalog_iri` / `docs_base`).

   These are plain-text substitution points in the Apache `.htaccess`
   templates under `hosting/`: your deployment pipeline replaces them with
   real hostnames before installing the file. `.gitlab-ci.yml` should use
   GitLab CI's own variable interpolation instead of these tokens: declare
   `SITE_BASE_HOST`, `VENDORED_HOST` (and optionally `DEV_HOST`) in a
   top-level `variables:` block, and reference them as `$SITE_BASE_HOST`,
   `$VENDORED_HOST`, `$DEV_HOST` in `environment:url:` fields and script
   lines, rather than hand-editing hostnames into the pipeline.

## Layout

```
src/
  config.rs                    HubConfig
  discoverer.rs                 the Discoverer trait
  default_discovery.rs           DefaultDiscoverer
  theme.rs, theme/                bundled placeholder CSS
  site.rs, htaccess.rs, html.rs     site build (Turtle/JSON-LD/HTML + .htaccess)
  catalog.rs                        DCAT catalog build
  model.rs, turtle.rs                shared types and Turtle parsing
  main.rs                             CLI wiring HubConfig::default() + DefaultDiscoverer
hosting/                      .htaccess templates, placeholder-driven
scripts/                      local serve / conneg / hosting-acceptance checks
tests/                        integration tests + minimal vocabulary fixtures
```

## License

Apache License 2.0 — see [LICENSE](LICENSE).
