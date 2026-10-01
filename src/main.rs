//! `vocabulary-hub-build <vocabularies-dir> <out-dir>`
//!
//! `<vocabularies-dir>` is a checkout of eona-vocabularies-reference (the
//! `vocabularies/` submodule of content.vocabulary-hub.eona-x.eu);
//! `<out-dir>` receives one tree per production host (`eona-x.eu/`,
//! `vocabulary.eona-x.eu/`), each uploaded to that host's `voc/` folder.

use std::path::PathBuf;
use std::process::ExitCode;

use vocabulary_hub_build::{SITE_BASE, discover, site::write_site};

fn main() -> ExitCode {
  let args: Vec<PathBuf> = std::env::args_os().skip(1).map(PathBuf::from).collect();
  let [vocabularies, out] = args.as_slice() else {
    eprintln!("usage: vocabulary-hub-build <vocabularies-dir> <out-dir>");
    return ExitCode::from(2);
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
    Ok(written) => {
      println!("wrote {} files under {}", written.len(), out.display());
      ExitCode::SUCCESS
    }
    Err(e) => {
      eprintln!("error: writing {}: {e}", out.display());
      ExitCode::FAILURE
    }
  }
}
