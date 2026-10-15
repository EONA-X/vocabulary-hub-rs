//! A hub's catalog data (a UI's `docs/` tree), built from a vocabularies
//! checkout as it is: every `<slug>/v<version>/` with a `metadata.ttl`.
//! Fixtures modelled on eona-vocabulary-services' own test data
//! (`vocab-alpha`, `vocab-beta`).

use std::collections::HashSet;
use std::fs;
use std::path::Path;

use serde_json::{Value, json};
use vocabulary_hub_rs::catalog::write_catalog;
use vocabulary_hub_rs::config::HubConfig;
use vocabulary_hub_rs::default_discovery::DefaultDiscoverer;

const BUILT_AT: &str = "2026-10-01 12:00 UTC";
const DCT: &str = "http://purl.org/dc/terms/";
const DCAT: &str = "http://www.w3.org/ns/dcat#";
const FOAF: &str = "http://xmlns.com/foaf/0.1/";
const ASSETTYPE: &str = "http://publications.europa.eu/resource/authority/asset-classification/";

const SITE_BASE: &str = "https://hub.example/";
const CATALOG_IRI: &str = "https://catalog.example/catalog";
const DOCS_BASE: &str = "https://catalog.example/docs";

/// A `HubConfig` fixture distinct enough from the crate's own defaults that
/// tests asserting on its fields can't be satisfied by accident.
fn config() -> HubConfig {
  HubConfig {
    site_base: SITE_BASE.to_string(),
    catalog_iri: CATALOG_IRI.to_string(),
    docs_base: DOCS_BASE.to_string(),
    catalog_title: "Example Vocabulary Hub".to_string(),
    catalog_description: "The ontologies, shapes, vocabularies, code lists and crosswalks of the example data space.".to_string(),
    ..HubConfig::default()
  }
}

fn write(root: &Path, file: &str, content: &str) {
  let path = root.join(file);
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(path, content).unwrap();
}

const PREFIXES: &str = r#"@prefix dcat: <http://www.w3.org/ns/dcat#> .
@prefix adms: <http://www.w3.org/ns/adms#> .
@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
@prefix schema: <http://schema.org/> .
@prefix vann: <http://purl.org/vocab/vann/> .
@prefix assettype: <http://publications.europa.eu/resource/authority/asset-classification/> .
@prefix status: <http://publications.europa.eu/resource/authority/concept-status/> .
"#;

/// A `metadata.ttl` as upstream writes it (metadata profile 1.3.0).
fn metadata(title: &str, version: &str, asset_type: &str, status: &str, extra: &str) -> String {
  format!(
    "{PREFIXES}<> a dcat:Dataset, adms:Asset ;\n  dcat:version \"{version}\" ;\n  dcterms:title \"{title}\"@en ;\n  dcterms:description \"About {title}.\"@en ;\n  dcat:type assettype:{asset_type} ;\n  adms:status status:{status} .\n{extra}\n"
  )
}

fn alpha_ontology(version: &str) -> String {
  format!(
    r#"@prefix al: <https://example.org/alpha#> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
<https://example.org/alpha#> a owl:Ontology ; owl:versionInfo "{version}" ; rdfs:label "Alpha"@en .
al:Thing a owl:Class ; rdfs:label "Thing"@en, "Chose"@fr ; rdfs:comment "A thing." .
al:size a owl:DatatypeProperty ; rdfs:domain al:Thing ; rdfs:range <http://www.w3.org/2001/XMLSchema#integer> .
"#
  )
}

fn fixture(root: &Path) {
  // Two versions of an upstream-credited ontology, with its upstream body and logo.
  let upstream = "<> dcterms:creator [ a foaf:Agent ; foaf:name \"Alpha Standards Body\" ] .\n<> schema:logo \"alpha-logo.png\" .\n[] vann:preferredNamespacePrefix \"al\" ; vann:preferredNamespaceUri \"https://example.org/alpha#\" .";
  for version in ["2.0.0", "2.1.0"] {
    write(
      root,
      &format!("vocab-alpha/v{version}/metadata.ttl"),
      &metadata("Vocab Alpha", version, "c_89b4bdb7", "CURRENT", upstream),
    );
    write(root, &format!("vocab-alpha/v{version}/ontology.ttl"), &alpha_ontology(version));
  }
  // A shapes graph: no root (so the hub's own Discoverer never publishes
  // it), no creator.
  write(
    root,
    "vocab-beta/v1.0.0/metadata.ttl",
    &metadata("Vocab Beta", "1.0.0", "c_3948c2ed", "DRAFT", ""),
  );
  write(
    root,
    "vocab-beta/v1.0.0/ontology.ttl",
    "@prefix sh: <http://www.w3.org/ns/shacl#> .\n<https://example.org/beta/S> a sh:NodeShape .\n",
  );
  // ont-a has a single root, so the hub's own Discoverer publishes it under SITE_BASE.
  write(root, "ont-a/v1.0.0/metadata.ttl", &metadata("ont-a", "1.0.0", "c_89b4bdb7", "CURRENT", ""));
  write(
    root,
    "ont-a/v1.0.0/ontology.ttl",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<https://example.org/ont-a#> a owl:Ontology ; owl:versionInfo \"1.0.0\" .\n<https://example.org/ont-a#X> a owl:Class .\n",
  );
  write(root, "ont-b/v1.0.0/metadata.ttl", &metadata("ont-b", "1.0.0", "c_cdd11291", "CURRENT", ""));
  write(
    root,
    "ont-b/v1.0.0/ontology.ttl",
    "@prefix skos: <http://www.w3.org/2004/02/skos/core#> .\n<https://example.org/ont-b#> a skos:ConceptScheme .\n<https://example.org/ont-b#X> a skos:Concept .\n",
  );
  write(root, "xwalk/v0.1.0/metadata.ttl", &metadata("A to B", "0.1.0", "c_bba2bb35", "CURRENT", ""));
  write(
    root,
    "xwalk/v0.1.0/alignment.ttl",
    r#"@prefix void: <http://rdfs.org/ns/void#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
<https://example.org/x/a-b> a void:Linkset ; rdfs:seeAlso <https://example.org/ont-b#X>, <https://example.org/ont-a#X> .
<https://example.org/x/m1> a <http://www.w3.org/1999/02/22-rdf-syntax-ns#Statement> ;
  <http://www.w3.org/1999/02/22-rdf-syntax-ns#subject> <https://example.org/ont-a#X> ;
  <http://www.w3.org/1999/02/22-rdf-syntax-ns#predicate> skos:exactMatch ;
  <http://www.w3.org/1999/02/22-rdf-syntax-ns#object> <https://example.org/ont-b#X> .
"#,
  );
  // A dependency closure without metadata.ttl, the assets, and the tools.
  write(
    root,
    "closure/v1.0.0/ontology.ttl",
    "<https://example.org/c/C> a <http://www.w3.org/2002/07/owl#Class> .\n",
  );
  write(root, "assets/alpha-logo.png", "png");
  write(root, "assets/owl-illustration.svg", "<svg/>");
  write(root, "tools/vocabulary-check/Cargo.toml", "[package]");
}

fn build(root: &Path, out: &Path) {
  fixture(root);
  write_catalog(root, out, BUILT_AT, &config(), &DefaultDiscoverer).unwrap();
}

fn json(out: &Path, file: &str) -> Value {
  serde_json::from_str(&fs::read_to_string(out.join(file)).unwrap()).unwrap()
}

fn node<'a>(nodes: &'a Value, id: &str) -> &'a Value {
  let matching: Vec<&Value> = nodes.as_array().unwrap().iter().filter(|n| n["@id"] == id).collect();
  assert_eq!(matching.len(), 1, "exactly one node {id}");
  matching[0]
}

fn ids(values: &Value) -> Vec<String> {
  values
    .as_array()
    .map(|a| a.iter().filter_map(|v| v["@id"].as_str().map(str::to_string)).collect())
    .unwrap_or_default()
}

fn downloads(dir: &str, name: &str) -> Value {
  json!({"ttl": format!("{dir}/{name}.ttl"), "jsonld": format!("{dir}/{name}.jsonld"), "owl": format!("{dir}/{name}.owl"), "nt": format!("{dir}/{name}.nt")})
}

#[test]
fn ontologies_json_lists_the_latest_version_of_every_asset_but_the_crosswalks_in_slug_order() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  let manifest = json(out.path(), "ontologies.json");

  assert_eq!(manifest["generatedAt"], BUILT_AT);
  let slugs: Vec<&str> = manifest["ontologies"].as_array().unwrap().iter().map(|e| e["slug"].as_str().unwrap()).collect();
  assert_eq!(slugs, ["ont-a", "ont-b", "vocab-alpha", "vocab-beta"]);

  let alpha = &manifest["ontologies"][2];
  assert_eq!(alpha["title"], "Vocab Alpha");
  assert_eq!(alpha["description"], "About Vocab Alpha.");
  assert_eq!(alpha["jsonld"], "vocab-alpha/v2.1.0/ontology.jsonld");
  assert_eq!(alpha["namespace"], "https://example.org/alpha#");
  assert_eq!(alpha["prefix"], "al");
  assert_eq!(alpha["version"], "2.1.0");
  assert_eq!(alpha["upstream"], json!({"name": "Alpha Standards Body", "logo": "alpha-logo.png"}));
  assert_eq!(alpha["kind"], "ontology");
  assert_eq!(alpha["downloads"], downloads("vocab-alpha/v2.1.0", "ontology"));

  let b = &manifest["ontologies"][1];
  assert_eq!(b["kind"], "codelist");
  assert_eq!(b["namespace"], "https://example.org/ont-b#");
  assert_eq!(b["jsonld"], "ont-b/v1.0.0/ontology.jsonld");

  // No root: no namespace (null, not "" — "" would prefix every IRI), and the
  // slug as prefix.
  let beta = &manifest["ontologies"][3];
  assert_eq!(beta["kind"], "shape");
  assert_eq!(beta["namespace"], Value::Null);
  assert_eq!(beta["prefix"], "vocab-beta");
  assert_eq!(beta["version"], "1.0.0");
  assert_eq!(beta["upstream"], Value::Null);
}

#[test]
fn alignments_json_lists_the_crosswalks_with_their_sides_in_authored_order() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  let manifest = json(out.path(), "alignments.json");

  assert_eq!(manifest["generatedAt"], BUILT_AT);
  assert_eq!(
    manifest["alignments"],
    json!([{
      "slug": "xwalk",
      "title": "A to B",
      "description": "About A to B.",
      "kind": "crosswalk",
      "version": "0.1.0",
      "jsonld": "xwalk/v0.1.0/alignment.jsonld",
      "sides": ["ont-b", "ont-a"],
      "downloads": downloads("xwalk/v0.1.0", "alignment")
    }])
  );
}

#[test]
fn a_crosswalk_side_goes_to_the_ontology_whose_namespace_is_the_longest_match_not_a_mere_string_prefix() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  for (slug, iri) in [("a", "https://ex.org/onto"), ("b", "https://ex.org/onto-ext")] {
    write(
      src.path(),
      &format!("{slug}/v1.0.0/metadata.ttl"),
      &metadata(slug, "1.0.0", "c_89b4bdb7", "CURRENT", ""),
    );
    write(
      src.path(),
      &format!("{slug}/v1.0.0/ontology.ttl"),
      &format!("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<{iri}> a owl:Ontology .\n"),
    );
  }
  write(src.path(), "x/v1.0.0/metadata.ttl", &metadata("X", "1.0.0", "c_bba2bb35", "CURRENT", ""));
  write(
    src.path(),
    "x/v1.0.0/alignment.ttl",
    "@prefix void: <http://rdfs.org/ns/void#> .\n@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .\n<https://ex.org/x#ls> a void:Linkset ; rdfs:seeAlso <https://ex.org/onto-ext> .\n",
  );
  write_catalog(src.path(), out.path(), BUILT_AT, &config(), &DefaultDiscoverer).unwrap();

  let manifest = json(out.path(), "alignments.json");

  assert_eq!(manifest["alignments"][0]["sides"], json!(["b"]));
}

#[test]
fn every_version_is_published_in_four_formats_that_all_carry_the_source_graph() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  for (dir, source, name) in [
    ("vocab-alpha/v2.0.0", "ontology.ttl", "ontology"),
    ("vocab-alpha/v2.1.0", "ontology.ttl", "ontology"),
    ("ont-b/v1.0.0", "ontology.ttl", "ontology"),
    ("xwalk/v0.1.0", "alignment.ttl", "alignment"),
  ] {
    let source = graph_of_turtle(&fs::read(src.path().join(dir).join(source)).unwrap());
    let base = out.path().join(dir);
    let file = |ext: &str| fs::read(base.join(format!("{name}.{ext}"))).unwrap();
    let ttl = graph_of_turtle(&file("ttl"));
    let nt: HashSet<String> = oxttl::NTriplesParser::new()
      .for_reader(file("nt").as_slice())
      .map(|t| canonical(&t.unwrap()))
      .collect();
    let owl: HashSet<String> = oxrdfxml::RdfXmlParser::new()
      .for_reader(file("owl").as_slice())
      .map(|t| canonical(&t.unwrap()))
      .collect();
    let jsonld = graph_of_jsonld(&file("jsonld"));
    for (format, g) in [("ttl", &ttl), ("nt", &nt), ("owl", &owl), ("jsonld", &jsonld)] {
      assert_eq!(g, &source, "{dir}/{name}.{format}");
    }
  }
}

#[test]
fn the_json_ld_is_expanded_one_node_per_subject_as_the_ui_parses_it() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  let nodes = json(out.path(), "vocab-alpha/v2.1.0/ontology.jsonld");

  let thing = node(&nodes, "https://example.org/alpha#Thing");
  assert_eq!(thing["@type"], json!(["http://www.w3.org/2002/07/owl#Class"]));
  let labels = thing["http://www.w3.org/2000/01/rdf-schema#label"].as_array().unwrap();
  assert!(labels.contains(&json!({"@value": "Thing", "@language": "en"})), "{labels:?}");
  assert!(labels.contains(&json!({"@value": "Chose", "@language": "fr"})), "{labels:?}");
  assert_eq!(thing["http://www.w3.org/2000/01/rdf-schema#comment"], json!([{"@value": "A thing."}]));
  let size = node(&nodes, "https://example.org/alpha#size");
  assert_eq!(
    size["http://www.w3.org/2000/01/rdf-schema#domain"],
    json!([{"@id": "https://example.org/alpha#Thing"}])
  );
  assert_eq!(
    size["http://www.w3.org/2000/01/rdf-schema#range"],
    json!([{"@id": "http://www.w3.org/2001/XMLSchema#integer"}])
  );
  let version = &node(&nodes, "https://example.org/alpha#")["http://www.w3.org/2002/07/owl#versionInfo"];
  assert_eq!(version, &json!([{"@value": "2.1.0"}]));
}

#[test]
fn the_catalog_is_a_dcat_catalog_under_the_configured_catalog_iri() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  let nodes = json(out.path(), "catalog.jsonld");

  let catalog = node(&nodes, CATALOG_IRI);
  assert_eq!(catalog["@type"], json!([format!("{DCAT}Catalog")]));
  assert_eq!(catalog[&format!("{DCT}modified")], json!([{"@value": BUILT_AT}]));
  assert_eq!(
    catalog[&format!("{DCT}title")],
    json!([{"@value": "Example Vocabulary Hub", "@language": "en"}])
  );
  assert_eq!(
    ids(&catalog[&format!("{DCAT}dataset")]),
    ["ont-a", "ont-b", "vocab-alpha", "vocab-beta", "xwalk"].map(|s| format!("{CATALOG_IRI}/{s}")),
    "vocabularies in slug order, then crosswalks"
  );
  let ttl = fs::read_to_string(out.path().join("catalog.ttl")).unwrap();
  assert!(ttl.contains(CATALOG_IRI));
  assert_eq!(
    graph_of_turtle(ttl.as_bytes()).len(),
    graph_of_jsonld(&fs::read(out.path().join("catalog.jsonld")).unwrap()).len()
  );
}

#[test]
fn each_asset_is_a_dataset_carrying_its_latest_metadata_and_the_computed_links() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());
  let nodes = json(out.path(), "catalog.jsonld");

  let alpha = node(&nodes, &format!("{CATALOG_IRI}/vocab-alpha"));
  let types: HashSet<&str> = alpha["@type"].as_array().unwrap().iter().map(|t| t.as_str().unwrap()).collect();
  assert_eq!(types, HashSet::from([&*format!("{DCAT}Dataset"), "http://www.w3.org/ns/adms#Asset"]));
  assert_eq!(alpha[&format!("{DCT}identifier")], json!([{"@value": "vocab-alpha"}]));
  assert_eq!(alpha[&format!("{DCT}title")], json!([{"@value": "Vocab Alpha", "@language": "en"}]));
  assert_eq!(ids(&alpha[&format!("{DCAT}type")]), [format!("{ASSETTYPE}c_89b4bdb7")]);
  assert_eq!(
    ids(&alpha["http://www.w3.org/ns/adms#status"]),
    ["http://publications.europa.eu/resource/authority/concept-status/CURRENT"]
  );
  assert_eq!(ids(&alpha[&format!("{DCT}conformsTo")]), ["https://example.org/alpha#"]);
  assert_eq!(alpha[&format!("{DCAT}version")], json!([{"@value": "2.1.0"}]));
  assert_eq!(
    ids(&alpha[&format!("{DCAT}landingPage")]),
    [format!("{DOCS_BASE}/ontologies?ontology=vocab-alpha")]
  );
  assert_eq!(ids(&alpha[&format!("{FOAF}thumbnail")]), [format!("{DOCS_BASE}/assets/alpha-logo.png")]);
  let creator = ids(&alpha[&format!("{DCT}creator")]);
  assert_eq!(node(&nodes, &creator[0])[&format!("{FOAF}name")], json!([{"@value": "Alpha Standards Body"}]));
  // Metadata about the source files is not catalog metadata.
  assert!(alpha.get("http://schema.org/logo").is_none());
  assert!(
    nodes
      .as_array()
      .unwrap()
      .iter()
      .all(|n| n.get("http://purl.org/vocab/vann/preferredNamespacePrefix").is_none())
  );

  let distributions = ids(&alpha[&format!("{DCAT}distribution")]);
  assert_eq!(distributions.len(), 4);
  let mut formats = HashSet::new();
  for d in &distributions {
    let d = node(&nodes, d);
    assert_eq!(d["@type"], json!([format!("{DCAT}Distribution")]));
    assert_eq!(d[&format!("{DCAT}mediaType")].as_array().unwrap().len(), 1);
    let url = &ids(&d[&format!("{DCAT}downloadURL")])[0];
    assert!(url.starts_with(&format!("{DOCS_BASE}/vocab-alpha/v2.1.0/ontology.")), "{url}");
    assert!(d[&format!("{DCT}title")][0]["@value"].as_str().unwrap().starts_with("Vocab Alpha 2.1.0 ("));
    formats.insert(ids(&d[&format!("{DCT}format")])[0].clone());
  }
  assert_eq!(
    formats,
    ["RDF_TURTLE", "JSON_LD", "OWL", "RDF_N_TRIPLES"]
      .map(|f| format!("http://publications.europa.eu/resource/authority/file-type/{f}"))
      .into_iter()
      .collect()
  );

  // No creator: the default illustration; no root: no conformsTo.
  let beta = node(&nodes, &format!("{CATALOG_IRI}/vocab-beta"));
  assert_eq!(ids(&beta[&format!("{FOAF}thumbnail")]), [format!("{DOCS_BASE}/assets/owl-illustration.svg")]);
  assert!(beta.get(format!("{DCT}creator")).is_none() && beta.get(format!("{DCT}conformsTo")).is_none());

  // A crosswalk lands on the crosswalk view.
  let xwalk = node(&nodes, &format!("{CATALOG_IRI}/xwalk"));
  assert_eq!(ids(&xwalk[&format!("{DCAT}landingPage")]), [format!("{DOCS_BASE}/crosswalks?crosswalk=xwalk")]);
  assert_eq!(ids(&xwalk[&format!("{DCAT}type")]), [format!("{ASSETTYPE}c_bba2bb35")]);
}

#[test]
fn every_version_is_a_dcat_version_of_its_asset() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());
  let nodes = json(out.path(), "catalog.jsonld");
  let asset = format!("{CATALOG_IRI}/vocab-alpha");
  let (v200, v210) = (format!("{asset}/v2.0.0"), format!("{asset}/v2.1.0"));

  let alpha = node(&nodes, &asset);
  assert_eq!(ids(&alpha[&format!("{DCAT}hasVersion")]), [v200.clone(), v210.clone()]);
  assert_eq!(ids(&alpha[&format!("{DCAT}hasCurrentVersion")]), std::slice::from_ref(&v210));

  let latest = node(&nodes, &v210);
  assert_eq!(latest["@type"], json!([format!("{DCAT}Dataset")]));
  assert_eq!(latest[&format!("{DCAT}version")], json!([{"@value": "2.1.0"}]));
  assert_eq!(ids(&latest[&format!("{DCT}isVersionOf")]), std::slice::from_ref(&asset));
  assert_eq!(ids(&latest[&format!("{DCAT}previousVersion")]), std::slice::from_ref(&v200));
  assert_eq!(ids(&latest[&format!("{DCAT}distribution")]), ids(&alpha[&format!("{DCAT}distribution")]));

  let earlier = node(&nodes, &v200);
  assert!(earlier.get(format!("{DCAT}previousVersion")).is_none());
  let urls: Vec<String> = ids(&earlier[&format!("{DCAT}distribution")])
    .iter()
    .flat_map(|d| ids(&node(&nodes, d)[&format!("{DCAT}downloadURL")]))
    .collect();
  assert_eq!(urls.len(), 4);
  assert!(urls.iter().all(|u| u.starts_with(&format!("{DOCS_BASE}/vocab-alpha/v2.0.0/"))), "{urls:?}");
}

#[test]
fn a_version_the_hub_publishes_links_its_dereferenceable_iri() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());
  let nodes = json(out.path(), "catalog.jsonld");

  // ont-a has a single root, so the hub's own Discoverer publishes it under
  // SITE_BASE and the catalog links that dereferenceable IRI.
  let published = node(&nodes, &format!("{CATALOG_IRI}/ont-a/v1.0.0"));
  assert_eq!(ids(&published[&format!("{FOAF}page")]), [format!("{SITE_BASE}ontology/ont-a/v1.0.0")]);
  // vocab-beta has no owl:Ontology/skos:ConceptScheme root, so the hub's own
  // Discoverer never publishes it: no dereferenceable IRI to link.
  let elsewhere = node(&nodes, &format!("{CATALOG_IRI}/vocab-beta/v1.0.0"));
  assert!(elsewhere.get(format!("{FOAF}page")).is_none());
}

#[test]
fn versions_are_ordered_numerically_so_v1_10_0_is_newer_than_v1_9_0() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  for version in ["1.9.0", "1.10.0"] {
    write(
      src.path(),
      &format!("a/v{version}/metadata.ttl"),
      &metadata("A", version, "c_89b4bdb7", "CURRENT", ""),
    );
    write(
      src.path(),
      &format!("a/v{version}/ontology.ttl"),
      "<https://ex.org/a#> a <http://www.w3.org/2002/07/owl#Ontology> .\n",
    );
  }
  write_catalog(src.path(), out.path(), BUILT_AT, &config(), &DefaultDiscoverer).unwrap();

  let manifest = json(out.path(), "ontologies.json");
  assert_eq!(manifest["ontologies"][0]["version"], "1.10.0");
  assert_eq!(manifest["ontologies"][0]["downloads"], downloads("a/v1.10.0", "ontology"));

  let nodes = json(out.path(), "catalog.jsonld");
  let asset = node(&nodes, &format!("{CATALOG_IRI}/a"));
  assert_eq!(ids(&asset[&format!("{DCAT}hasCurrentVersion")]), [format!("{CATALOG_IRI}/a/v1.10.0")]);
  let newer = node(&nodes, &format!("{CATALOG_IRI}/a/v1.10.0"));
  assert_eq!(ids(&newer[&format!("{DCAT}previousVersion")]), [format!("{CATALOG_IRI}/a/v1.9.0")]);
  let older = node(&nodes, &format!("{CATALOG_IRI}/a/v1.9.0"));
  assert!(older.get(format!("{DCAT}previousVersion")).is_none());
}

#[test]
fn the_assets_are_copied_and_nothing_else_is_written() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  build(src.path(), out.path());

  assert_eq!(fs::read_to_string(out.path().join("assets/alpha-logo.png")).unwrap(), "png");
  // No landing page, no per-slug redirects: the UI is the root.
  assert!(!out.path().join("index.html").exists());
  assert!(!out.path().join("vocab-alpha/index.html").exists());
  assert!(!out.path().join("closure").exists(), "a directory without metadata.ttl is not published");
  assert!(!out.path().join("tools").exists());
}

fn graph_of_turtle(data: &[u8]) -> HashSet<String> {
  oxttl::TurtleParser::new().for_reader(data).map(|t| canonical(&t.unwrap())).collect()
}

fn graph_of_jsonld(data: &[u8]) -> HashSet<String> {
  oxjsonld::JsonLdParser::new()
    .for_reader(data)
    .map(|q| canonical(&oxrdf::Triple::from(q.unwrap())))
    .collect()
}

/// A triple as N-Triples with blank nodes renamed away, so graphs from
/// different serializations compare equal when only labels differ.
fn canonical(t: &oxrdf::Triple) -> String {
  let term = |s: String| if s.starts_with("_:") { "_:b".to_string() } else { s };
  format!("{} {} {}", term(t.subject.to_string()), t.predicate, term(t.object.to_string()))
}

#[test]
fn a_version_dir_with_metadata_but_no_graph_file_is_skipped_like_discovery_does() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  fixture(src.path());
  // Work in progress: metadata only, no ontology.ttl or alignment.ttl yet.
  write(src.path(), "wip/v0.1.0/metadata.ttl", &metadata("Wip", "0.1.0", "c_89b4bdb7", "DRAFT", ""));

  let result = write_catalog(src.path(), out.path(), BUILT_AT, &config(), &DefaultDiscoverer);

  assert!(result.is_ok(), "expected the catalog to be written, got {result:?}");
  let manifest = json(out.path(), "ontologies.json");
  let slugs: Vec<&str> = manifest["ontologies"].as_array().unwrap().iter().map(|o| o["slug"].as_str().unwrap()).collect();
  assert!(!slugs.contains(&"wip"), "an unpublished version is not listed: {slugs:?}");
  assert!(slugs.contains(&"vocab-alpha"));
  assert!(!out.path().join("wip").exists());
}

#[test]
fn a_missing_vocabularies_dir_is_an_error_that_leaves_the_existing_catalog_untouched() {
  let tmp = tempfile::tempdir().unwrap();
  let docs = tmp.path().join("docs");
  write(&docs, "ontologies.json", "previously published");

  let result = write_catalog(&tmp.path().join("does-not-exist"), &docs, BUILT_AT, &config(), &DefaultDiscoverer);

  assert!(result.is_err(), "expected an error, got {result:?}");
  assert_eq!(fs::read_to_string(docs.join("ontologies.json")).unwrap(), "previously published");
}

#[test]
fn a_published_version_carries_the_same_term_iris_in_the_catalog_as_on_the_site() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  write(src.path(), "rel/v1/metadata.ttl", &metadata("Rel", "1", "c_89b4bdb7", "CURRENT", ""));
  write(
    src.path(),
    "rel/v1/ontology.ttl",
    "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#> a owl:Ontology .\n<#Thing> a owl:Class .\n",
  );
  write_catalog(src.path(), out.path(), BUILT_AT, &config(), &DefaultDiscoverer).unwrap();

  // The site mints it under `{SITE_BASE}ontology/rel/v1`; the catalog's own
  // copy of the graph must not re-mint it under the catalog IRI.
  let nt = fs::read_to_string(out.path().join("rel/v1/ontology.nt")).unwrap();
  assert!(nt.contains(&format!("<{SITE_BASE}ontology/rel/v1#Thing>")), "{nt}");
  assert!(!nt.contains(CATALOG_IRI), "{nt}");
}

#[test]
fn blank_nodes_authored_with_the_same_label_in_two_metadata_files_stay_two_nodes_in_the_catalog() {
  let (src, out) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
  for slug in ["org-a", "org-b"] {
    let extra = format!("<> dcterms:creator _:org .\n_:org foaf:name \"Org of {slug}\" .");
    write(
      src.path(),
      &format!("{slug}/v1/metadata.ttl"),
      &metadata(slug, "1", "c_89b4bdb7", "CURRENT", &extra),
    );
    write(
      src.path(),
      &format!("{slug}/v1/ontology.ttl"),
      &format!("@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<https://example.org/{slug}#> a owl:Ontology .\n"),
    );
  }
  write_catalog(src.path(), out.path(), BUILT_AT, &config(), &DefaultDiscoverer).unwrap();

  let triples: Vec<oxrdf::Triple> = oxttl::TurtleParser::new()
    .for_reader(fs::read(out.path().join("catalog.ttl")).unwrap().as_slice())
    .map(|t| t.unwrap())
    .collect();
  let creators: HashSet<String> = triples
    .iter()
    .filter(|t| t.predicate.as_str() == format!("{DCT}creator"))
    .map(|t| t.object.to_string())
    .collect();
  assert_eq!(creators.len(), 2, "each asset keeps its own creator node: {creators:?}");
  for creator in &creators {
    let names = triples
      .iter()
      .filter(|t| t.subject.to_string() == *creator && t.predicate.as_str() == format!("{FOAF}name"))
      .count();
    assert_eq!(names, 1, "{creator} carries exactly one name");
  }
}

#[test]
fn a_site_base_without_a_trailing_slash_still_yields_well_formed_published_iris() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  fixture(src.path());
  let mut config = config();
  config.site_base = "https://hub.example".to_string();
  write_catalog(src.path(), out.path(), BUILT_AT, &config, &DefaultDiscoverer).unwrap();

  let catalog = fs::read_to_string(out.path().join("catalog.ttl")).unwrap();
  assert!(
    catalog.contains("foaf:page <https://hub.example/"),
    "published pages live under the host: {catalog}"
  );
  assert!(
    !catalog.contains("hub.exampleontology") && !catalog.contains("hub.examplecodelist"),
    "no IRI glues the kind onto the host: {catalog}"
  );
}

#[test]
fn a_catalog_iri_with_a_trailing_slash_does_not_mint_double_slash_iris() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  fixture(src.path());
  let mut config = config();
  config.catalog_iri = format!("{CATALOG_IRI}/");
  write_catalog(src.path(), out.path(), BUILT_AT, &config, &DefaultDiscoverer).unwrap();

  let catalog = fs::read_to_string(out.path().join("catalog.ttl")).unwrap();
  assert!(!catalog.contains("catalog//"), "no doubled slash in minted IRIs: {catalog}");
}
