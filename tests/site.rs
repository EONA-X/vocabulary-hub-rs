use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::Triple;
use vocabulary_hub_build::{SITE_BASE, Vocabulary, discover, site::write_site};

const NS: &str = "https://eona-x.eu/odrl-profile/v0.0.1#";

fn profile() -> String {
  format!(
    r#"@prefix eox-policy: <{NS}> .
@prefix odrl: <http://www.w3.org/ns/odrl/2/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix dct: <http://purl.org/dc/terms/> .
eox-policy: a owl:Ontology ; owl:versionInfo "0.0.1" ;
  dct:title "Eona-X ODRL Profile"@en ;
  dct:description "A profile for <policies> & claims."@en .
eox-policy:GenericClaim a odrl:LeftOperand, skos:Concept ;
  rdfs:isDefinedBy eox-policy: ;
  rdfs:label "Generic Claim"@en ;
  skos:definition "Addresses a claim inside a credential."@en .
eox-policy:leftOperands a skos:Collection ;
  skos:prefLabel "Eona-X constraint left operands"@en ;
  skos:member eox-policy:GenericClaim, odrl:eq .
"#
  )
}

fn published(root: &Path) -> Vec<Vocabulary> {
  fs::create_dir_all(root.join("eonax-odrl-profile")).unwrap();
  fs::write(root.join("eonax-odrl-profile/ontology.ttl"), profile()).unwrap();
  discover(root, SITE_BASE).unwrap().published
}

fn triple_set(triples: impl IntoIterator<Item = Triple>) -> HashSet<String> {
  // Blank-node labels differ across a round trip; this profile has none, so
  // the N-Triples form of each triple is a faithful identity.
  triples.into_iter().map(|t| t.to_string()).collect()
}

#[test]
fn each_version_is_written_as_turtle_json_ld_and_html_next_to_its_slug_htaccess() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  let mut written = write_site(out.path(), &published(src.path())).unwrap();
  written.sort();

  let expected: Vec<PathBuf> =
    ["odrl-profile/.htaccess", "odrl-profile/v0.0.1.html", "odrl-profile/v0.0.1.jsonld", "odrl-profile/v0.0.1.ttl"].map(PathBuf::from).into();
  assert_eq!(written, expected);
  for p in &expected {
    assert!(out.path().join(p).is_file(), "{} missing", p.display());
  }
}

#[test]
fn the_turtle_is_the_whole_document_and_keeps_the_source_prefixes() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let vocabularies = published(src.path());

  write_site(out.path(), &vocabularies).unwrap();

  let ttl = fs::read(out.path().join("odrl-profile/v0.0.1.ttl")).unwrap();
  let reparsed = oxttl::TurtleParser::new().for_reader(ttl.as_slice()).collect::<Result<Vec<_>, _>>().unwrap();
  assert_eq!(triple_set(reparsed), triple_set(vocabularies[0].triples.clone()));
  assert!(String::from_utf8(ttl).unwrap().contains(&format!("@prefix eox-policy: <{NS}>")));
}

#[test]
fn the_json_ld_is_the_same_graph() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let vocabularies = published(src.path());

  write_site(out.path(), &vocabularies).unwrap();

  let jsonld = fs::read(out.path().join("odrl-profile/v0.0.1.jsonld")).unwrap();
  let reparsed: Vec<Triple> =
    oxjsonld::JsonLdParser::new().for_reader(jsonld.as_slice()).map(|q| Triple::from(q.unwrap())).collect();
  assert_eq!(triple_set(reparsed), triple_set(vocabularies[0].triples.clone()));
}

#[test]
fn the_html_gives_every_term_an_anchor_named_after_its_fragment() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  let html = fs::read_to_string(out.path().join("odrl-profile/v0.0.1.html")).unwrap();
  assert!(html.contains(r#"id="GenericClaim""#), "{html}");
  assert!(html.contains(r#"id="leftOperands""#), "{html}");
  assert!(html.contains("Generic Claim"));
  assert!(html.contains("Addresses a claim inside a credential."));
  // Members inside the vocabulary link in-page; outside ones link out.
  assert!(html.contains(r##"href="#GenericClaim""##), "{html}");
  assert!(html.contains(r#"href="http://www.w3.org/ns/odrl/2/eq""#), "{html}");
}

#[test]
fn the_html_links_its_machine_readable_twins_and_escapes_literals() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  let html = fs::read_to_string(out.path().join("odrl-profile/v0.0.1.html")).unwrap();
  assert!(html.contains("<title>Eona-X ODRL Profile"), "{html}");
  assert!(html.contains(r#"href="v0.0.1.ttl""#));
  assert!(html.contains(r#"href="v0.0.1.jsonld""#));
  assert!(html.contains("A profile for &lt;policies&gt; &amp; claims."), "{html}");
  assert!(!html.contains("<policies>"));
}

#[test]
fn the_htaccess_negotiates_only_its_own_slug_and_points_the_bare_slug_at_the_latest_version() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  let htaccess = fs::read_to_string(out.path().join("odrl-profile/.htaccess")).unwrap();
  assert!(htaccess.contains("RewriteBase /odrl-profile/"), "{htaccess}");
  assert!(htaccess.contains(r"^(v0\.0\.1)$"), "{htaccess}");
  assert!(htaccess.contains("^$ v0.0.1 [R=302,L]"), "{htaccess}");
  assert!(htaccess.contains("AddType text/turtle .ttl"), "{htaccess}");
  assert!(htaccess.contains("AddType application/ld+json .jsonld"), "{htaccess}");
}
