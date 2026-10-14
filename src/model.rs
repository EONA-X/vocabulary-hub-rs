//! Shared types: a published vocabulary version, the EU asset-classification
//! table every [`crate::discoverer::Discoverer`] classifies against, and what
//! [`crate::discoverer::Discoverer::discover`] returns.

use oxrdf::Triple;

/// One version of a vocabulary, ready to be published: its RDF graph plus
/// everything [`crate::site::write_site`] and [`crate::catalog::write_catalog`]
/// need to place it.
#[derive(Debug, Clone)]
pub struct Vocabulary {
  /// The host tree it publishes under, e.g. `https://vocabulary-hub.example/`.
  pub base: String,
  /// One of [`ASSET_TYPES`]' names, e.g. `"ontology"`.
  pub kind: String,
  pub slug: String,
  /// The version directory's own name, e.g. `"v0.0.1"`.
  pub version: String,
  /// `<slug>/<version>`, the directory it was read from.
  pub dir_name: String,
  /// The IRI its terms are minted under (its `owl:Ontology` or
  /// `skos:ConceptScheme` root), fragment included.
  pub namespace: String,
  pub prefixes: Vec<(String, String)>,
  pub triples: Vec<Triple>,
}

/// A `<slug>/<version>` directory a [`Discoverer`](crate::discoverer::Discoverer)
/// found but did not publish, and why.
#[derive(Debug, Clone)]
pub struct Skipped {
  pub dir_name: String,
  pub reason: String,
}

/// What [`Discoverer::discover`](crate::discoverer::Discoverer::discover) returns.
#[derive(Debug, Clone, Default)]
pub struct Discovery {
  pub published: Vec<Vocabulary>,
  pub skipped: Vec<Skipped>,
}

/// EU Publications Office asset-classification concepts
/// (`http://publications.europa.eu/resource/authority/asset-classification/`)
/// covered by each published asset type, in publication order.
pub const ASSET_TYPES: [(&str, &[&str]); 5] = [
  ("ontology", &["c_89b4bdb7"]),
  ("shape", &["c_b37963b3", "c_3948c2ed"]),
  ("crosswalk", &["c_bba2bb35"]),
  (
    "vocabulary",
    &[
      "c_64714767",
      "c_a7773248",
      "c_5796a20b",
      "c_ebfb658e",
      "c_25e514f4",
      "c_d3bf7907",
      "c_ecacbeba",
      "c_b0bfba6e",
    ],
  ),
  ("codelist", &["c_cdd11291", "c_5b130cc6"]),
];

/// `(ontology|shape|crosswalk|vocabulary|codelist)`, an alternation matching
/// the path segment the host's router dispatches each asset type's slugs to.
pub fn asset_type_alternation() -> String {
  format!("({})", ASSET_TYPES.iter().map(|(name, _)| *name).collect::<Vec<_>>().join("|"))
}
