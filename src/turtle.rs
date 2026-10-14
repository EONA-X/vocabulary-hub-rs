//! Turtle parsing shared by [`crate::default_discovery`] (classifying and
//! loading each vocabulary version) and [`crate::catalog`] (reading the same
//! files again for the DCAT catalog's own metadata).

use std::fs;
use std::path::Path;

use oxrdf::Triple;
use oxttl::TurtleParser;

/// A graph and its `@prefix` table.
pub type Parsed = (Vec<Triple>, Vec<(String, String)>);

/// Parses the Turtle file at `path`, resolving relative IRIs (including the
/// bare `<>` a file commonly uses for "this document") against `base`.
pub fn parse_turtle(path: &Path, base: &str) -> Result<Parsed, String> {
  let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
  let mut parser = TurtleParser::new().with_base_iri(base).map_err(|e| format!("{base}: {e}"))?.for_slice(&data);
  let mut triples = Vec::new();
  for t in parser.by_ref() {
    triples.push(t.map_err(|e| format!("{}: {e}", path.display()))?);
  }
  let prefixes = parser.prefixes().map(|(p, iri)| (p.to_string(), iri.to_string())).collect();
  Ok((triples, prefixes))
}
