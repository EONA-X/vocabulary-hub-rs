use std::path::Path;

use oxrdf::Triple;

/// One publishable vocabulary version.
pub struct Vocabulary {
  /// Source directory name in eona-vocabularies-reference, e.g. `eonax-odrl-profile`.
  pub dir_name: String,
  /// The ontology IRI, which is also the term namespace, e.g. `https://eona-x.eu/odrl-profile/v0.0.1#`.
  pub namespace: String,
  /// First path segment under the site base, e.g. `odrl-profile`.
  pub slug: String,
  /// Second path segment, e.g. `v0.0.1`.
  pub version: String,
  pub triples: Vec<Triple>,
  /// The source file's own `@prefix` table, reused when re-serializing.
  pub prefixes: Vec<(String, String)>,
}

/// A candidate directory that is deliberately not published, and why.
pub struct Skipped {
  pub dir_name: String,
  pub reason: String,
}

pub struct Discovery {
  pub published: Vec<Vocabulary>,
  pub skipped: Vec<Skipped>,
}

/// Scans `vocabularies_root` (a checkout of eona-vocabularies-reference) for
/// `eonax-*` directories and splits them into publishable and skipped.
pub fn discover(_vocabularies_root: &Path, _site_base: &str) -> Result<Discovery, String> {
  todo!()
}
