use std::fs;
use std::path::Path;

use vocabulary_hub_build::{SITE_BASE, discover};

fn vocab(root: &Path, dir: &str, ontology_ttl: &str) {
  fs::create_dir_all(root.join(dir)).unwrap();
  fs::write(root.join(dir).join("ontology.ttl"), ontology_ttl).unwrap();
}

fn ontology(namespace: &str, version_info: &str) -> String {
  format!(
    r#"@prefix ex: <{namespace}> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
ex: a owl:Ontology ; owl:versionInfo "{version_info}" ; rdfs:label "Example"@en .
ex:Thing rdfs:label "Thing"@en ; rdfs:isDefinedBy ex: .
"#
  )
}

#[test]
fn a_vocabulary_minted_under_the_site_base_is_published_at_its_slug_and_version() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-odrl-profile", &ontology("https://eona-x.eu/odrl-profile/v0.0.1#", "0.0.1"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert_eq!(found.published.len(), 1);
  let v = &found.published[0];
  assert_eq!(v.dir_name, "eonax-odrl-profile");
  assert_eq!(v.namespace, "https://eona-x.eu/odrl-profile/v0.0.1#");
  assert_eq!(v.slug, "odrl-profile");
  assert_eq!(v.version, "v0.0.1");
  assert_eq!(v.triples.len(), 5);
  assert!(v.prefixes.iter().any(|(p, iri)| p == "ex" && iri == "https://eona-x.eu/odrl-profile/v0.0.1#"));
}

#[test]
fn a_vocabulary_still_minted_elsewhere_is_skipped_with_its_namespace_as_the_reason() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-credentials", &ontology("https://w3id.org/eonax/credentials/", "0.1.0"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert!(found.published.is_empty());
  assert_eq!(found.skipped.len(), 1);
  assert_eq!(found.skipped[0].dir_name, "eonax-credentials");
  assert!(found.skipped[0].reason.contains("https://w3id.org/eonax/credentials/"), "{}", found.skipped[0].reason);
}

#[test]
fn only_eonax_directories_are_candidates() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "odrl22", &ontology("https://eona-x.eu/odrl22/v2.2#", "2.2"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert!(found.published.is_empty());
  assert!(found.skipped.is_empty());
}

#[test]
fn a_version_segment_that_disagrees_with_owl_version_info_fails_the_build() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-odrl-profile", &ontology("https://eona-x.eu/odrl-profile/v0.0.2#", "0.0.1"));

  let err = discover(root.path(), SITE_BASE).err().expect("mismatched version must not publish");

  assert!(err.contains("v0.0.2") && err.contains("0.0.1"), "{err}");
}

#[test]
fn published_vocabularies_come_out_in_directory_name_order() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-b", &ontology("https://eona-x.eu/b/v1.0.0#", "1.0.0"));
  vocab(root.path(), "eonax-a", &ontology("https://eona-x.eu/a/v1.0.0#", "1.0.0"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  let slugs: Vec<_> = found.published.iter().map(|v| v.slug.as_str()).collect();
  assert_eq!(slugs, ["a", "b"]);
}
