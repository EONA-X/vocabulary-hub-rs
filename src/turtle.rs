//! Turtle parsing shared by [`crate::default_discovery`] (classifying and
//! loading each vocabulary version) and [`crate::catalog`] (reading the same
//! files again for the DCAT catalog's own metadata).

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use oxrdf::{BlankNode, NamedOrBlankNode, Term, Triple};
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
  Ok((relabel_blank_nodes(triples), prefixes))
}

/// The parser gives every blank node a random id, which would make each build
/// of unchanged input emit different files. Relabel them `b0`, `b1`, ... in
/// first-seen order.
fn relabel_blank_nodes(triples: Vec<Triple>) -> Vec<Triple> {
  let mut labels: HashMap<BlankNode, BlankNode> = HashMap::new();
  let mut relabel = |b: BlankNode| {
    let next = labels.len();
    labels.entry(b).or_insert_with(|| BlankNode::new_unchecked(format!("b{next}"))).clone()
  };
  triples
    .into_iter()
    .map(|t| Triple {
      subject: match t.subject {
        NamedOrBlankNode::BlankNode(b) => relabel(b).into(),
        other => other,
      },
      predicate: t.predicate,
      object: match t.object {
        Term::BlankNode(b) => relabel(b).into(),
        other => other,
      },
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  const BASE: &str = "https://example.test/";

  fn parse(ttl: &str) -> Vec<Triple> {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ontology.ttl");
    fs::write(&path, ttl).unwrap();
    parse_turtle(&path, BASE).unwrap().0
  }

  const WITH_BLANK_NODES: &str = r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<#A> rdfs:subClassOf [ a owl:Restriction ; owl:onProperty <#p> ] .
<#B> rdfs:subClassOf _:x . _:x a owl:Restriction .
"#;

  #[test]
  fn parsing_the_same_file_twice_yields_identical_blank_node_labels() {
    assert_eq!(parse(WITH_BLANK_NODES), parse(WITH_BLANK_NODES));
  }

  #[test]
  fn blank_nodes_stay_distinct_and_keep_their_links_after_relabelling() {
    let triples = parse(WITH_BLANK_NODES);
    let ids: std::collections::BTreeSet<String> = triples
      .iter()
      .filter_map(|t| match &t.subject {
        oxrdf::NamedOrBlankNode::BlankNode(b) => Some(b.as_str().to_string()),
        _ => None,
      })
      .collect();
    assert_eq!(ids.len(), 2);
    for t in triples.iter().filter(|t| matches!(t.object, oxrdf::Term::BlankNode(_))) {
      let oxrdf::Term::BlankNode(o) = &t.object else { unreachable!() };
      assert!(ids.contains(o.as_str()));
    }
  }
}
