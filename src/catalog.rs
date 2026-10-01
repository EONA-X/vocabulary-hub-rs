//! The Vocabulary Hub catalog data (the UI's `docs/` tree).

use std::path::{Path, PathBuf};

pub const CATALOG_IRI: &str = "https://vocabulary.eona-x.eu/catalog";
pub const DOCS_BASE: &str = "https://vocabulary.eona-x.eu/docs";

/// Writes the catalog data for the vocabularies at `vocabularies` into `out`.
pub fn write_catalog(_vocabularies: &Path, _out: &Path, _built_at: &str) -> Result<Vec<PathBuf>, String> {
  todo!()
}
