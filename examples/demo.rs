//! A worked example of coding a Vocabulary Hub: builds a demo site and
//! catalog from the real, sample vocabularies under `examples/vocabularies/`
//! (the same sample data eona-vocabulary-ui develops against), using the
//! crate's defaults (`DefaultDiscoverer`, `DefaultComponents`) plus a custom
//! `HubConfig`. This is what `.github/workflows/pages.yml` runs to publish
//! <https://eona-x.github.io/vocabulary-hub-rs/>.
//!
//! `write_site`/`write_catalog` are this crate's whole job; the index page
//! below is not — a deployment brings its own, as eona-vocabulary-ui does.
//!
//! Run: `cargo run --release --example demo -- <out-dir>`

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use vocabulary_hub_rs::catalog::write_catalog;
use vocabulary_hub_rs::config::HubConfig;
use vocabulary_hub_rs::default_components::DefaultComponents;
use vocabulary_hub_rs::default_discovery::DefaultDiscoverer;
use vocabulary_hub_rs::discoverer::Discoverer;
use vocabulary_hub_rs::model::Vocabulary;
use vocabulary_hub_rs::site::write_site;

const SITE_BASE: &str = "https://eona-x.github.io/vocabulary-hub-rs/";
const SITE_HOST_PATH: &str = "eona-x.github.io/vocabulary-hub-rs";

fn main() -> ExitCode {
  let Some(out) = std::env::args_os().nth(1).map(PathBuf::from) else {
    eprintln!("usage: demo <out-dir>");
    return ExitCode::from(2);
  };
  let vocabularies_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/vocabularies");
  let site_root = out.join(SITE_HOST_PATH);

  let config = HubConfig {
    site_base: SITE_BASE.to_string(),
    brand_name: "Vocabulary Hub RS".to_string(),
    brand_link: SITE_BASE.to_string(),
    brand_link_label: "eona-x.github.io/vocabulary-hub-rs".to_string(),
    footer_text: "vocabulary-hub-rs — built from this repo's examples/demo.rs".to_string(),
    catalog_title: "Vocabulary Hub RS — demo catalog".to_string(),
    catalog_description: "A demo catalog published by examples/demo.rs from a handful of real sample vocabularies.".to_string(),
    ..HubConfig::default()
  };
  let discoverer = DefaultDiscoverer;
  let components = DefaultComponents;

  let found = match discoverer.discover(&vocabularies_dir, &config.site_base) {
    Ok(found) => found,
    Err(e) => {
      eprintln!("error: {e}");
      return ExitCode::FAILURE;
    }
  };
  for s in &found.skipped {
    println!("skip    {}: {}", s.dir_name, s.reason);
  }
  for v in &found.published {
    println!("publish {} -> {}{}/{}/{}", v.dir_name, v.base, v.kind, v.slug, v.version);
  }

  match write_site(&out, &found.published, &config, &components) {
    Ok(written) => println!("wrote {} site files under {}", written.len(), site_root.display()),
    Err(e) => {
      eprintln!("error: writing {}: {e}", site_root.display());
      return ExitCode::FAILURE;
    }
  }

  let docs = site_root.join("docs");
  let built_at = std::env::var("BUILT_AT").unwrap_or_else(|_| "local build".into());
  match write_catalog(&vocabularies_dir, &docs, &built_at, &config, &discoverer) {
    Ok(written) => println!("wrote {} catalog files under {}", written.len(), docs.display()),
    Err(e) => {
      eprintln!("error: catalog: {e}");
      return ExitCode::FAILURE;
    }
  }

  if let Err(e) = write_index(&site_root, &found.published) {
    eprintln!("error: writing index.html: {e}");
    return ExitCode::FAILURE;
  }
  println!("wrote {}", site_root.join("index.html").display());
  ExitCode::SUCCESS
}

/// A landing page is not this crate's job (see the module doc) — this is
/// just enough of one to make the demo's root navigable, grouped by asset
/// type the way the catalog itself is.
fn write_index(site_root: &Path, published: &[Vocabulary]) -> std::io::Result<()> {
  let mut by_kind: Vec<(&str, Vec<&Vocabulary>)> = Vec::new();
  for v in published {
    match by_kind.iter_mut().find(|(k, _)| *k == v.kind) {
      Some((_, vs)) => vs.push(v),
      None => by_kind.push((&v.kind, vec![v])),
    }
  }
  by_kind.sort_by_key(|(k, _)| *k);

  let mut sections = String::new();
  for (kind, vocabularies) in &by_kind {
    sections.push_str(&format!("<h2>{}</h2>\n<ul>\n", esc(kind)));
    for v in vocabularies {
      let href = format!("{}/{}/{}.html", esc(kind), esc(&v.slug), esc(&v.version));
      sections.push_str(&format!("<li><a href=\"{href}\">{}</a></li>\n", esc(&v.slug)));
    }
    sections.push_str("</ul>\n");
  }

  let html = format!(
    r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Vocabulary Hub RS — demo</title>
</head>
<body style="max-width:40rem;margin:2rem auto;padding:0 16px;font-family:system-ui,sans-serif">
<h1>Vocabulary Hub RS — demo</h1>
<p>Built by <a href="https://github.com/EONA-X/vocabulary-hub-rs/blob/main/examples/demo.rs">examples/demo.rs</a>
from the sample vocabularies under
<a href="https://github.com/EONA-X/vocabulary-hub-rs/tree/main/examples/vocabularies">examples/vocabularies/</a>.
Also published as a <a href="docs/catalog.jsonld">DCAT catalog</a>
(<a href="docs/ontologies.json">ontologies.json</a>, <a href="docs/alignments.json">alignments.json</a>).</p>
{sections}
</body>
</html>
"#
  );
  std::fs::write(site_root.join("index.html"), html)
}

fn esc(s: &str) -> String {
  s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
