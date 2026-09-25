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

## Status

**Scaffold only — no build implementation yet.** There is no generator, no static site, no
publish pipeline here yet; those are intentionally left for follow-up work (see the tracking
issue backlog#677 and the ODRL-profile-specific backlog#654 /
[eona-vocabulary-services#1](https://gitlab.eona-x.org/eona-x/semantic-hub/eona-vocabulary-services/-/issues/1)
for what lands here first).

## Related

- Content repo: [`content.vocabulary-hub.eona-x.eu`](https://gitlab.eona-x.org/eona-x/semantic-hub/content.vocabulary-hub.eona-x.eu)
- Authoring/runtime: [`eona-vocabulary-services`](https://gitlab.eona-x.org/eona-x/semantic-hub/eona-vocabulary-services)
- Equivalent pattern elsewhere: [`developer.eona-x.eu`](https://gitlab.eona-x.org/eona-x/web/developer.eona-x.eu) / `content.developer.eona-x.eu`
