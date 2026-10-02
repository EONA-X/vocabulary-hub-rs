//! `vocabulary-hub-build <vocabularies-dir> <out-dir> [<docs-dir>]`
//!
//! `<vocabularies-dir>` is a checkout of eona-vocabularies-reference (the
//! `vocabularies/` submodule of content.vocabulary-hub.eona-x.eu);
//! `<out-dir>` receives one tree per production host (`eona-x.eu/`,
//! `vocabulary.eona-x.eu/`), each uploaded to that host's `voc/` folder;
//! `<docs-dir>`, if given, the Vocabulary Hub UI's catalog data (its `docs/`),
//! stamped with `$BUILT_AT` (CI passes the commit timestamp).

use std::path::PathBuf;
use std::process::ExitCode;

use vocabulary_hub_build::{SITE_BASE, catalog::write_catalog, discover, site::write_site};

fn main() -> ExitCode {
  let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
  let (vocabularies, out, docs) = match args.as_slice() {
    [vocabularies, out] => (vocabularies, out, None),
    [vocabularies, out, docs] => (vocabularies, out, Some(docs)),
    _ => {
      eprintln!("usage: vocabulary-hub-build <vocabularies-dir> <out-dir> [<docs-dir>]");
      return ExitCode::from(2);
    }
  };
  let found = match discover(vocabularies, SITE_BASE) {
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
  match write_site(out, &found.published) {
    Ok(written) => println!("wrote {} files under {}", written.len(), out.display()),
    Err(e) => {
      eprintln!("error: writing {}: {e}", out.display());
      return ExitCode::FAILURE;
    }
  }
  if let Some(docs) = docs {
    let built_at = std::env::var("BUILT_AT").unwrap_or_else(|_| "local build".into());
    match write_catalog(vocabularies, docs, &built_at) {
      Ok(written) => println!("wrote {} catalog files under {}", written.len(), docs.display()),
      Err(e) => {
        eprintln!("error: catalog: {e}");
        return ExitCode::FAILURE;
      }
    }
  }
  ExitCode::SUCCESS
}
