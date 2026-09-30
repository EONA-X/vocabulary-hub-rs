use std::fs;
use std::path::Path;

use vocabulary_hub_build::{SITE_BASE, discover};

fn vocab(root: &Path, dir: &str, ontology_ttl: &str) {
  fs::create_dir_all(root.join(dir)).unwrap();
  fs::write(root.join(dir).join("ontology.ttl"), ontology_ttl).unwrap();
}

/// The DCAT/ADMS self-description upstream ships next to each ontology, with
/// its EU asset-classification `dcat:type`.
fn metadata(root: &Path, dir: &str, asset_type: &str) {
  fs::write(
    root.join(dir).join("metadata.ttl"),
    format!(
      r#"@prefix dcat: <http://www.w3.org/ns/dcat#> .
@prefix assettype: <http://publications.europa.eu/resource/authority/asset-classification/> .
<> a dcat:Dataset ; dcat:type assettype:{asset_type} .
"#
    ),
  )
  .unwrap();
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

fn concept_scheme(namespace: &str, version_info: &str) -> String {
  format!(
    r#"@prefix ex: <{namespace}> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
ex: a skos:ConceptScheme ; owl:versionInfo "{version_info}" ; skos:prefLabel "Example codes"@en .
ex:A a skos:Concept ; skos:inScheme ex: ; skos:prefLabel "A"@en .
"#
  )
}

#[test]
fn a_vocabulary_minted_under_the_site_base_is_published_at_its_asset_class_slug_and_version() {
  let root = tempfile::tempdir().unwrap();
  vocab(
    root.path(),
    "eonax-odrl-profile",
    &ontology("https://eona-x.eu/c_89b4bdb7/odrl-profile/v0.0.1#", "0.0.1"),
  );

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert_eq!(found.published.len(), 1);
  let v = &found.published[0];
  assert_eq!(v.dir_name, "eonax-odrl-profile");
  assert_eq!(v.namespace, "https://eona-x.eu/c_89b4bdb7/odrl-profile/v0.0.1#");
  assert_eq!(v.kind, "c_89b4bdb7");
  assert_eq!(v.slug, "odrl-profile");
  assert_eq!(v.version, "v0.0.1");
  assert_eq!(v.triples.len(), 5);
  assert!(
    v.prefixes
      .iter()
      .any(|(p, iri)| p == "ex" && iri == "https://eona-x.eu/c_89b4bdb7/odrl-profile/v0.0.1#")
  );
}

#[test]
fn the_asset_class_segment_is_an_eu_asset_classification_concept_code() {
  let root = tempfile::tempdir().unwrap();
  // Formal ontology, Markup schema, Alignment, Thesaurus, Code list.
  for code in ["c_89b4bdb7", "c_3948c2ed", "c_bba2bb35", "c_a7773248", "c_cdd11291"] {
    vocab(
      root.path(),
      &format!("eonax-{code}"),
      &ontology(&format!("https://eona-x.eu/{code}/x/v1.0.0#"), "1.0.0"),
    );
  }

  let found = discover(root.path(), SITE_BASE).unwrap();

  let mut codes: Vec<_> = found.published.iter().map(|v| v.kind.as_str()).collect();
  codes.sort();
  assert_eq!(codes, ["c_3948c2ed", "c_89b4bdb7", "c_a7773248", "c_bba2bb35", "c_cdd11291"]);
}

#[test]
fn a_code_list_rooted_in_a_skos_concept_scheme_is_published() {
  let root = tempfile::tempdir().unwrap();
  vocab(
    root.path(),
    "eonax-roles",
    &concept_scheme("https://eona-x.eu/c_cdd11291/roles/v1.0.0#", "1.0.0"),
  );

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert_eq!(found.published.len(), 1);
  assert_eq!(found.published[0].kind, "c_cdd11291");
  assert_eq!(found.published[0].slug, "roles");
}

#[test]
fn a_vocabulary_still_minted_elsewhere_is_skipped_with_its_namespace_as_the_reason() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-credentials", &ontology("https://w3id.org/eonax/credentials/", "0.1.0"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert!(found.published.is_empty());
  assert_eq!(found.skipped.len(), 1);
  assert_eq!(found.skipped[0].dir_name, "eonax-credentials");
  assert!(
    found.skipped[0].reason.contains("https://w3id.org/eonax/credentials/"),
    "{}",
    found.skipped[0].reason
  );
}

#[test]
fn only_eonax_directories_are_candidates() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "odrl22", &ontology("https://eona-x.eu/c_89b4bdb7/odrl22/v2.2#", "2.2"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert!(found.published.is_empty());
  assert!(found.skipped.is_empty());
}

#[test]
fn a_version_segment_that_disagrees_with_owl_version_info_fails_the_build() {
  let root = tempfile::tempdir().unwrap();
  vocab(
    root.path(),
    "eonax-odrl-profile",
    &ontology("https://eona-x.eu/c_89b4bdb7/odrl-profile/v0.0.2#", "0.0.1"),
  );

  let err = discover(root.path(), SITE_BASE).err().expect("mismatched version must not publish");

  assert!(err.contains("v0.0.2") && err.contains("0.0.1"), "{err}");
}

#[test]
fn the_former_slug_version_form_without_an_asset_class_fails_the_build() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-odrl-profile", &ontology("https://eona-x.eu/odrl-profile/v0.0.1#", "0.0.1"));

  let err = discover(root.path(), SITE_BASE)
    .err()
    .expect("an IRI under the site base without an asset class must not publish");

  assert!(err.contains("<asset-class>/<slug>/<version>"), "{err}");
}

#[test]
fn a_code_outside_the_eu_asset_classification_fails_the_build() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-x", &ontology("https://eona-x.eu/c_00000000/x/v1.0.0#", "1.0.0"));

  let err = discover(root.path(), SITE_BASE)
    .err()
    .expect("a code outside the EU asset-classification table must not publish");

  assert!(err.contains("c_00000000") && err.contains("asset-classification"), "{err}");
}

#[test]
fn the_segment_must_be_the_eu_asset_classification_declared_in_metadata() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-roles", &ontology("https://eona-x.eu/c_cdd11291/roles/v1.0.0#", "1.0.0"));
  metadata(root.path(), "eonax-roles", "c_89b4bdb7"); // Formal ontology

  let err = discover(root.path(), SITE_BASE)
    .err()
    .expect("a Code list IRI on a Formal ontology must not publish");

  assert!(err.contains("c_cdd11291") && err.contains("c_89b4bdb7"), "{err}");
}

#[test]
fn a_segment_that_is_the_declared_asset_classification_is_published() {
  let root = tempfile::tempdir().unwrap();
  vocab(
    root.path(),
    "eonax-odrl-profile",
    &ontology("https://eona-x.eu/c_89b4bdb7/odrl-profile/v0.0.1#", "0.0.1"),
  );
  metadata(root.path(), "eonax-odrl-profile", "c_89b4bdb7"); // Formal ontology
  vocab(
    root.path(),
    "eonax-metadata-profile",
    &ontology("https://eona-x.eu/c_3948c2ed/metadata-profile/v1.0.0#", "1.0.0"),
  );
  metadata(root.path(), "eonax-metadata-profile", "c_3948c2ed"); // Markup schema

  let found = discover(root.path(), SITE_BASE).unwrap();

  assert_eq!(found.published.len(), 2);
}

#[test]
fn published_vocabularies_come_out_in_directory_name_order() {
  let root = tempfile::tempdir().unwrap();
  vocab(root.path(), "eonax-b", &ontology("https://eona-x.eu/c_89b4bdb7/b/v1.0.0#", "1.0.0"));
  vocab(root.path(), "eonax-a", &ontology("https://eona-x.eu/c_89b4bdb7/a/v1.0.0#", "1.0.0"));

  let found = discover(root.path(), SITE_BASE).unwrap();

  let slugs: Vec<_> = found.published.iter().map(|v| v.slug.as_str()).collect();
  assert_eq!(slugs, ["a", "b"]);
}
