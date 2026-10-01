use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rdf_sitegen::{Prefixes, to_jsonld, to_turtle, write_if_changed};

use crate::{Vocabulary, htaccess, html};

/// Writes every published representation of `vocabularies` under `out`, one
/// tree per production host (`eona-x.eu/`, `vocabulary.eona-x.eu/`), returning
/// the written paths relative to `out`. In each `{host}/` tree:
///
/// - `{asset-type}/{slug}/{version}.ttl` and `.jsonld` — the whole ontology document.
///   Terms are hash IRIs (`{asset-type}/{slug}/{version}#Term`), so the client strips the
///   fragment before it asks and one document answers for every term.
/// - `{asset-type}/{slug}/{version}.html` — the human page, one anchor per term, so the
///   same `#Term` lands on that term's section in a browser.
/// - `{asset-type}/{slug}/.htaccess` — content negotiation for this slug only. It is kept
///   out of the site root because eona-x.eu serves other things too, and a
///   root `.htaccess` would be shared with them.
pub fn write_site(out: &Path, vocabularies: &[Vocabulary]) -> std::io::Result<Vec<PathBuf>> {
  // Exists even when nothing is published — the normal case until upstream
  // mints a vocabulary under the site base.
  std::fs::create_dir_all(out)?;
  let mut written = Vec::new();
  // {asset-type}/{slug} -> (host, versions)
  let mut versions_by_slug: BTreeMap<String, (&str, Vec<&str>)> = BTreeMap::new();

  for v in vocabularies {
    let host = host(&v.base);
    let slug = format!("{}/{}", v.kind, v.slug);
    let entry = versions_by_slug.entry(slug.clone()).or_insert((host, Vec::new()));
    if entry.0 != host {
      // dev-voc serves both hosts' trees side by side: the paths would collide.
      return Err(std::io::Error::other(format!("{slug} is published on both {} and {host}", entry.0)));
    }
    entry.1.push(&v.version);
    let prefix_refs: Vec<(&str, &str)> = v.prefixes.iter().map(|(p, iri)| (p.as_str(), iri.as_str())).collect();
    let prefixes = Prefixes(&prefix_refs);
    let base = PathBuf::from(host).join(&v.kind).join(&v.slug).join(&v.version);
    for (ext, bytes) in [
      ("ttl", to_turtle(&v.triples, &prefixes)),
      ("jsonld", to_jsonld(&v.triples, &prefixes)),
      ("html", html::render(v).into_bytes()),
    ] {
      // `with_extension` would eat `v0.0.1`'s last dot-segment.
      let rel = PathBuf::from(format!("{}.{ext}", base.display()));
      write_if_changed(&out.join(&rel), &bytes)?;
      written.push(rel);
    }
  }

  for (slug, (host, versions)) in versions_by_slug {
    let dir = PathBuf::from(host).join(&slug);
    // The EONA-X UI toolkit's stylesheets, once per slug, shared by its versions'
    // HTML pages (linked as `assets/<file>`).
    for (file, css) in html::STYLESHEETS {
      let rel = dir.join("assets").join(file);
      write_if_changed(&out.join(&rel), css.as_bytes())?;
      written.push(rel);
    }
    let rel = dir.join(".htaccess");
    write_if_changed(&out.join(&rel), htaccess::render(host, &slug, &versions).as_bytes())?;
    written.push(rel);
  }
  Ok(written)
}

/// `https://vocabulary.eona-x.eu/` -> `vocabulary.eona-x.eu`
fn host(base: &str) -> &str {
  base.split_once("://").map_or(base, |(_, rest)| rest).trim_end_matches('/')
}
