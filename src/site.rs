use std::path::{Path, PathBuf};

use crate::Vocabulary;

/// Writes every published representation of `vocabularies` under `out`,
/// returning the written paths relative to `out`.
pub fn write_site(_out: &Path, _vocabularies: &[Vocabulary]) -> std::io::Result<Vec<PathBuf>> {
  todo!()
}
