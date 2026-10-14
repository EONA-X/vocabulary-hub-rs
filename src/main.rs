//! `vocabulary-hub-rs <vocabularies-dir> <out-dir> [<docs-dir>]`
//!
//! `<vocabularies-dir>` is a checkout of versioned RDF vocabularies, laid out
//! as `<slug>/<version>/` directories; `<out-dir>` receives the published
//! site, one tree per host vocabularies publish to; `<docs-dir>`, if given,
//! a hub UI's catalog data (its `docs/`), stamped with `$BUILT_AT` (CI passes
//! the commit timestamp).
//!
//! This binary wires the crate's defaults together: [`HubConfig::default`]
//! and [`DefaultDiscoverer`]. A deployment with its own branding or
//! publication rules builds its own `HubConfig` and/or `Discoverer` and
//! calls the library directly instead.

use std::path::PathBuf;
use std::process::ExitCode;

use vocabulary_hub_rs::catalog::write_catalog;
use vocabulary_hub_rs::config::HubConfig;
use vocabulary_hub_rs::default_discovery::DefaultDiscoverer;
use vocabulary_hub_rs::discoverer::Discoverer;
use vocabulary_hub_rs::site::write_site;

fn main() -> ExitCode {
  let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
  let (vocabularies, out, docs) = match args.as_slice() {
    [vocabularies, out] => (vocabularies, out, None),
    [vocabularies, out, docs] => (vocabularies, out, Some(docs)),
    _ => {
      eprintln!("usage: vocabulary-hub-rs <vocabularies-dir> <out-dir> [<docs-dir>]");
      return ExitCode::from(2);
    }
  };

  let config = HubConfig::default();
  let discoverer = DefaultDiscoverer;

  let found = match discoverer.discover(vocabularies, &config.site_base) {
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
  match write_site(out, &found.published, &config) {
    Ok(written) => println!("wrote {} files under {}", written.len(), out.display()),
    Err(e) => {
      eprintln!("error: writing {}: {e}", out.display());
      return ExitCode::FAILURE;
    }
  }
  if let Some(docs) = docs {
    let built_at = std::env::var("BUILT_AT").unwrap_or_else(|_| "local build".into());
    match write_catalog(vocabularies, docs, &built_at, &config, &discoverer) {
      Ok(written) => println!("wrote {} catalog files under {}", written.len(), docs.display()),
      Err(e) => {
        eprintln!("error: catalog: {e}");
        return ExitCode::FAILURE;
      }
    }
  }
  ExitCode::SUCCESS
}
