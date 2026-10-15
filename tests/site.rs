use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::Triple;
use vocabulary_hub_rs::config::HubConfig;
use vocabulary_hub_rs::default_components::DefaultComponents;
use vocabulary_hub_rs::default_discovery::DefaultDiscoverer;
use vocabulary_hub_rs::discoverer::Discoverer;
use vocabulary_hub_rs::model::Vocabulary;
use vocabulary_hub_rs::site::write_site;

const BASE: &str = "https://example.test/";
const NS: &str = "https://example.test/vocabulary/example-profile/v0.0.1#";

/// A `HubConfig` fixture distinct enough from the crate's own defaults that
/// tests asserting on its fields can't be satisfied by accident.
fn config() -> HubConfig {
  HubConfig {
    brand_name: "Example Hub".to_string(),
    brand_link: BASE.to_string(),
    brand_link_label: "example.test".to_string(),
    footer_text: "Example Vocabulary Hub".to_string(),
    ..HubConfig::default()
  }
}

fn profile() -> String {
  format!(
    r#"@prefix ex-policy: <{NS}> .
@prefix odrl: <http://www.w3.org/ns/odrl/2/> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
@prefix dct: <http://purl.org/dc/terms/> .
ex-policy: a owl:Ontology ; owl:versionInfo "0.0.1" ;
  dct:title "Example ODRL Profile"@en ;
  dct:description "A profile for <policies> & claims."@en .
ex-policy:GenericClaim a odrl:LeftOperand, skos:Concept ;
  rdfs:isDefinedBy ex-policy: ;
  rdfs:label "Generic Claim"@en ;
  skos:definition "Addresses a claim inside a credential."@en .
ex-policy:leftOperands a skos:Collection ;
  skos:prefLabel "Example constraint left operands"@en ;
  skos:member ex-policy:GenericClaim, odrl:eq .
"#
  )
}

/// `dcat:type` covering `"vocabulary"` in [`vocabulary_hub_rs::model::ASSET_TYPES`].
const METADATA: &str = r#"@prefix dcat: <http://www.w3.org/ns/dcat#> .
@prefix assettype: <http://publications.europa.eu/resource/authority/asset-classification/> .
<> dcat:type assettype:c_64714767 .
"#;

fn published(root: &Path) -> Vec<Vocabulary> {
  fs::create_dir_all(root.join("example-profile/v0.0.1")).unwrap();
  fs::write(root.join("example-profile/v0.0.1/ontology.ttl"), profile()).unwrap();
  fs::write(root.join("example-profile/v0.0.1/metadata.ttl"), METADATA).unwrap();
  DefaultDiscoverer.discover(root, BASE).unwrap().published
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

  let mut written = write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();
  written.sort();

  let expected: Vec<PathBuf> = [
    "example.test/vocabulary/assets/.htaccess",
    "example.test/vocabulary/assets/components.css",
    "example.test/vocabulary/assets/ontology.css",
    "example.test/vocabulary/assets/tokens.css",
    "example.test/vocabulary/example-profile/.htaccess",
    "example.test/vocabulary/example-profile/v0.0.1.html",
    "example.test/vocabulary/example-profile/v0.0.1.jsonld",
    "example.test/vocabulary/example-profile/v0.0.1.ttl",
  ]
  .map(PathBuf::from)
  .into();
  assert_eq!(written, expected);
  for p in &expected {
    assert!(out.path().join(p).is_file(), "{} missing", p.display());
  }
}

#[test]
fn two_slugs_of_the_same_asset_type_share_one_copy_of_each_stylesheet() {
  // A real deployment ships ~18 slugs of one asset type; each must not carry
  // its own private copy of the same bytes under a different URL.
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  for slug in ["example-profile", "other-profile"] {
    fs::create_dir_all(src.path().join(slug).join("v0.0.1")).unwrap();
    fs::write(src.path().join(slug).join("v0.0.1/ontology.ttl"), profile()).unwrap();
    fs::write(src.path().join(slug).join("v0.0.1/metadata.ttl"), METADATA).unwrap();
  }
  let vocabularies = DefaultDiscoverer.discover(src.path(), BASE).unwrap().published;
  let slugs: Vec<&str> = vocabularies.iter().map(|v| v.slug.as_str()).collect();
  assert_eq!(vocabularies.len(), 2, "{slugs:?}");

  let written = write_site(out.path(), &vocabularies, &config(), &DefaultComponents).unwrap();

  let css_writes = written.iter().filter(|p| p.extension().is_some_and(|e| e == "css")).count();
  assert_eq!(
    css_writes,
    config().stylesheets.len(),
    "expected one copy of each stylesheet, not one per slug: {written:?}"
  );
  for (file, content) in config().stylesheets {
    assert_eq!(
      fs::read_to_string(out.path().join("example.test/vocabulary/assets").join(file)).unwrap(),
      content,
      "{file}"
    );
  }
  assert!(
    !out.path().join("example.test/vocabulary/example-profile/assets").exists(),
    "example-profile must not carry its own copy of the stylesheets"
  );
  assert!(
    !out.path().join("example.test/vocabulary/other-profile/assets").exists(),
    "other-profile must not carry its own copy of the stylesheets"
  );
}

#[test]
fn the_turtle_is_the_whole_document_and_keeps_the_source_prefixes() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let vocabularies = published(src.path());

  write_site(out.path(), &vocabularies, &config(), &DefaultComponents).unwrap();

  let ttl = fs::read(out.path().join("example.test/vocabulary/example-profile/v0.0.1.ttl")).unwrap();
  let reparsed = oxttl::TurtleParser::new().for_reader(ttl.as_slice()).collect::<Result<Vec<_>, _>>().unwrap();
  assert_eq!(triple_set(reparsed), triple_set(vocabularies[0].triples.clone()));
  assert!(String::from_utf8(ttl).unwrap().contains(&format!("@prefix ex-policy: <{NS}>")));
}

#[test]
fn the_json_ld_is_the_same_graph() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let vocabularies = published(src.path());

  write_site(out.path(), &vocabularies, &config(), &DefaultComponents).unwrap();

  let jsonld = fs::read(out.path().join("example.test/vocabulary/example-profile/v0.0.1.jsonld")).unwrap();
  let reparsed: Vec<Triple> = oxjsonld::JsonLdParser::new()
    .for_reader(jsonld.as_slice())
    .map(|q| Triple::from(q.unwrap()))
    .collect();
  assert_eq!(triple_set(reparsed), triple_set(vocabularies[0].triples.clone()));
}

#[test]
fn the_html_gives_every_term_an_anchor_named_after_its_fragment() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let html = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap();
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

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let html = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap();
  assert!(html.contains("<title>Example ODRL Profile"), "{html}");
  assert!(html.contains(r#"href="v0.0.1.ttl""#));
  assert!(html.contains(r#"href="v0.0.1.jsonld""#));
  assert!(html.contains("A profile for &lt;policies&gt; &amp; claims."), "{html}");
  assert!(!html.contains("<policies>"));
}

#[test]
fn the_htaccess_negotiates_any_version_present_on_disk_and_points_the_bare_slug_at_the_latest() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let htaccess = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/.htaccess")).unwrap();
  assert!(htaccess.contains("RewriteBase /vocabulary/example-profile/"), "{htaccess}");
  // Upstream only keeps the latest version, so a later build will not know
  // about v0.0.1 — yet its IRIs must keep resolving once deployed. The rules
  // key on the files being there, never on a list of versions.
  assert!(htaccess.contains("RewriteCond %{REQUEST_FILENAME}.ttl -f"), "{htaccess}");
  assert!(!htaccess.contains(r"v0\.0\.1"), "{htaccess}");
  // Absolute, with the request's own host: a relative target lets Apache build
  // the URL itself, and behind a TLS front it may add ":443".
  assert!(
    htaccess.contains("RewriteRule ^$ https://%{HTTP_HOST}/vocabulary/example-profile/v0.0.1 [R=302,L]"),
    "{htaccess}"
  );
  assert!(htaccess.contains("AddType text/turtle .ttl"), "{htaccess}");
  assert!(htaccess.contains("AddType application/ld+json .jsonld"), "{htaccess}");
}

#[test]
fn the_htaccess_answers_only_under_its_published_path_even_behind_a_router() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let htaccess = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/.htaccess")).unwrap();
  assert!(
    htaccess.contains("RewriteCond %{ENV:REDIRECT_STATUS} ^$\nRewriteCond %{REQUEST_URI} !^/vocabulary/example-profile(/|$)\nRewriteRule ^ - [R=404,L]"),
    "{htaccess}"
  );
}

#[test]
fn the_htaccess_compresses_every_representation_it_serves() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  // No .css lives under a slug directory any more (the stylesheets moved to
  // the shared assets/ one level up), so this slug's own .htaccess no longer
  // needs to carry text/css.
  let htaccess = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/.htaccess")).unwrap();
  assert!(
    htaccess.contains("AddOutputFilterByType DEFLATE text/html text/turtle application/ld+json"),
    "{htaccess}"
  );
  assert!(!htaccess.contains("text/css"), "{htaccess}");
}

#[test]
fn the_shared_assets_htaccess_compresses_the_stylesheets() {
  // Moving the stylesheets out of the slug directory means they no longer
  // cascade from that slug's own .htaccess (AllowOverride only looks down a
  // tree, never sideways or up): the shared assets/ needs its own coverage.
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let htaccess = fs::read_to_string(out.path().join("example.test/vocabulary/assets/.htaccess")).unwrap();
  assert!(htaccess.contains("AddOutputFilterByType DEFLATE text/css"), "{htaccess}");
}

#[test]
fn the_html_names_the_published_iri_as_canonical_wherever_it_is_served() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path()), &config(), &DefaultComponents).unwrap();

  let html = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap();
  assert!(
    html.contains(r#"<link rel="canonical" href="https://example.test/vocabulary/example-profile/v0.0.1">"#),
    "{html}"
  );
}

#[test]
fn a_build_that_publishes_nothing_still_produces_its_output_directory() {
  let out = tempfile::tempdir().unwrap();
  let site = out.path().join("public");

  let written = write_site(&site, &[], &config(), &DefaultComponents).unwrap();

  assert!(written.is_empty());
  assert!(site.is_dir());
}

fn html_page(out: &Path) -> String {
  let src = tempfile::tempdir().unwrap();
  write_site(out, &published(src.path()), &config(), &DefaultComponents).unwrap();
  fs::read_to_string(out.join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap()
}

#[test]
fn the_html_page_is_styled_by_the_configured_stylesheets_shared_by_its_asset_type() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());
  let cfg = config();

  for (file, content) in cfg.stylesheets {
    assert!(
      html.contains(&format!(r#"<link rel="stylesheet" href="../assets/{file}">"#)),
      "{file} not linked:\n{html}"
    );
    assert_eq!(
      fs::read_to_string(out.path().join("example.test/vocabulary/assets").join(file)).unwrap(),
      content,
      "{file}"
    );
  }
}

#[test]
fn the_html_page_appends_extra_page_css_after_the_built_in_layout_rules() {
  let out = tempfile::tempdir().unwrap();
  let src = tempfile::tempdir().unwrap();
  let config = HubConfig {
    extra_page_css: "html>body{padding-top:0}",
    ..config()
  };
  write_site(out.path(), &published(src.path()), &config, &DefaultComponents).unwrap();
  let html = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap();
  assert!(html.contains("</style>"), "{html}");
  let before_close = html.split("</style>").next().unwrap();
  assert!(before_close.ends_with("html>body{padding-top:0}"), "extra_page_css not appended last:\n{html}");
}

#[test]
fn the_html_page_carries_the_brand_mark_linking_to_the_brand_link() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());
  let cfg = config();

  assert!(html.contains(cfg.brand_mark_svg), "the brand mark is not inlined:\n{html}");
  assert!(html.contains(&format!(r#"href="{}""#, cfg.brand_link)), "{html}");
  assert!(html.contains(&format!(r#"aria-label="{}""#, cfg.brand_name)), "{html}");
}

#[test]
fn the_footer_carries_the_configured_text_and_home_link() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());
  let cfg = config();

  assert!(html.contains(&cfg.footer_text), "{html}");
  assert!(html.contains(&cfg.brand_link_label), "{html}");
}

#[test]
fn terms_are_rendered_with_the_local_ontology_components() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());

  // SectionHead opens the overview and the terms.
  assert!(html.contains(r#"class="section-head""#), "{html}");
  // OntoBadge for each rdf:type: odrl:LeftOperand has no TermKind of its own.
  assert!(html.contains(r#"class="badge badge--default""#) && html.contains("odrl:LeftOperand"), "{html}");
  // OntoAnnotation for each annotation, with the literal's language as a badge.
  assert!(html.contains(r#"<dt class="annotation__label">Definition</dt>"#), "{html}");
  assert!(
    html.contains(r#"<span class="annotation__text">Addresses a claim inside a credential.</span>"#),
    "{html}"
  );
  assert!(html.contains(">en</span>"), "{html}");
  // SimpleFooter closes the page.
  assert!(html.contains(r#"class="icons-footer""#), "{html}");
}

#[test]
fn one_asset_type_and_slug_on_both_hosts_is_refused() {
  // write_site's own invariant, independent of any Discoverer: the same
  // {kind}/{slug} cannot be split across two hosts, or their paths would
  // collide once served side by side.
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let mut vocabularies = published(src.path());
  let mut elsewhere = vocabularies[0].clone();
  elsewhere.base = "https://other.example/".to_string();
  elsewhere.version = "v0.0.2".to_string();
  vocabularies.push(elsewhere);

  let err = write_site(out.path(), &vocabularies, &config(), &DefaultComponents).unwrap_err();

  assert!(err.to_string().contains("vocabulary/example-profile"), "{err}");
}

#[tokio::test(flavor = "current_thread")]
async fn write_site_works_when_called_from_inside_a_tokio_runtime() {
  let config = HubConfig::default();
  let found = DefaultDiscoverer.discover(Path::new("tests/fixtures/vocabularies"), &config.site_base).unwrap();
  let out = tempfile::tempdir().unwrap();
  write_site(out.path(), &found.published, &config, &DefaultComponents).unwrap();
}

#[test]
fn a_member_iri_with_a_script_scheme_is_shown_as_text_and_never_linked() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let ttl = format!(
    r#"@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix skos: <http://www.w3.org/2004/02/skos/core#> .
<{NS}> a owl:Ontology .
<{NS}A> a owl:Class ; skos:member <javascript:alert(3)>, <data:text/html,hi>, <http://www.w3.org/ns/odrl/2/eq> .
"#
  );
  fs::create_dir_all(src.path().join("example-profile/v0.0.1")).unwrap();
  fs::write(src.path().join("example-profile/v0.0.1/ontology.ttl"), ttl).unwrap();
  fs::write(src.path().join("example-profile/v0.0.1/metadata.ttl"), METADATA).unwrap();
  let vocabularies = DefaultDiscoverer.discover(src.path(), BASE).unwrap().published;

  write_site(out.path(), &vocabularies, &config(), &DefaultComponents).unwrap();

  let html = fs::read_to_string(out.path().join("example.test/vocabulary/example-profile/v0.0.1.html")).unwrap();
  assert!(!html.contains(r#"href="javascript:"#), "{html}");
  assert!(!html.contains(r#"href="data:"#), "{html}");
  assert!(html.contains("javascript:alert(3)"), "{html}");
  assert!(html.contains(r#"href="http://www.w3.org/ns/odrl/2/eq""#), "{html}");
}
