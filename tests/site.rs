use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::Triple;
use vocabulary_hub_build::{SITE_BASE, Vocabulary, discover, site::write_site};

const NS: &str = "https://eona-x.eu/vocabulary/odrl-profile/v0.0.1#";

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
  fs::create_dir_all(root.join("odrl-profile/v0.0.1")).unwrap();
  fs::write(root.join("odrl-profile/v0.0.1/ontology.ttl"), profile()).unwrap();
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

  let expected: Vec<PathBuf> = [
    "eona-x.eu/vocabulary/odrl-profile/.htaccess",
    "eona-x.eu/vocabulary/odrl-profile/assets/components.css",
    "eona-x.eu/vocabulary/odrl-profile/assets/ontology.css",
    "eona-x.eu/vocabulary/odrl-profile/assets/tokens.css",
    "eona-x.eu/vocabulary/odrl-profile/v0.0.1.html",
    "eona-x.eu/vocabulary/odrl-profile/v0.0.1.jsonld",
    "eona-x.eu/vocabulary/odrl-profile/v0.0.1.ttl",
  ]
  .map(PathBuf::from)
  .into();
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

  let ttl = fs::read(out.path().join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.ttl")).unwrap();
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

  let jsonld = fs::read(out.path().join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.jsonld")).unwrap();
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

  write_site(out.path(), &published(src.path())).unwrap();

  let html = fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.html")).unwrap();
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

  let html = fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.html")).unwrap();
  assert!(html.contains("<title>Eona-X ODRL Profile"), "{html}");
  assert!(html.contains(r#"href="v0.0.1.ttl""#));
  assert!(html.contains(r#"href="v0.0.1.jsonld""#));
  assert!(html.contains("A profile for &lt;policies&gt; &amp; claims."), "{html}");
  assert!(!html.contains("<policies>"));
}

#[test]
fn the_htaccess_negotiates_any_version_present_on_disk_and_points_the_bare_slug_at_the_latest() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  let htaccess = fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/.htaccess")).unwrap();
  assert!(htaccess.contains("RewriteBase /vocabulary/odrl-profile/"), "{htaccess}");
  // Upstream only keeps the latest version, so a later build will not know
  // about v0.0.1 — yet its IRIs must keep resolving once deployed. The rules
  // key on the files being there, never on a list of versions.
  assert!(htaccess.contains("RewriteCond %{REQUEST_FILENAME}.ttl -f"), "{htaccess}");
  assert!(!htaccess.contains(r"v0\.0\.1"), "{htaccess}");
  // Absolute, with the request's own host: a relative target lets Apache build
  // the URL itself, and behind OVH's TLS front it adds ":443".
  assert!(
    htaccess.contains("RewriteRule ^$ https://%{HTTP_HOST}/vocabulary/odrl-profile/v0.0.1 [R=302,L]"),
    "{htaccess}"
  );
  assert!(htaccess.contains("AddType text/turtle .ttl"), "{htaccess}");
  assert!(htaccess.contains("AddType application/ld+json .jsonld"), "{htaccess}");
}

#[test]
fn the_htaccess_answers_only_under_its_published_path_even_behind_a_router() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  // On eona-x.eu the slug sits under voc/ behind the apex router; a direct
  // /voc/vocabulary/odrl-profile/... request (no internal redirect yet) must not be a
  // second public path for the same document.
  let htaccess = fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/.htaccess")).unwrap();
  assert!(
    htaccess.contains("RewriteCond %{ENV:REDIRECT_STATUS} ^$\nRewriteCond %{REQUEST_URI} !^/vocabulary/odrl-profile(/|$)\nRewriteRule ^ - [R=404,L]"),
    "{htaccess}"
  );
}

#[test]
fn the_html_names_the_published_iri_as_canonical_wherever_it_is_served() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &published(src.path())).unwrap();

  let html = fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.html")).unwrap();
  assert!(
    html.contains(r#"<link rel="canonical" href="https://eona-x.eu/vocabulary/odrl-profile/v0.0.1">"#),
    "{html}"
  );
}

#[test]
fn a_build_that_publishes_nothing_still_produces_its_output_directory() {
  let out = tempfile::tempdir().unwrap();
  let site = out.path().join("public");

  let written = write_site(&site, &[]).unwrap();

  // Until upstream mints a vocabulary under eona-x.eu this is the normal
  // case, and CI steps after the build expect the directory to exist.
  assert!(written.is_empty());
  assert!(site.is_dir());
}

fn html_page(out: &Path) -> String {
  let src = tempfile::tempdir().unwrap();
  write_site(out, &published(src.path())).unwrap();
  fs::read_to_string(out.join("eona-x.eu/vocabulary/odrl-profile/v0.0.1.html")).unwrap()
}

#[test]
fn the_html_page_is_styled_by_the_eona_x_ui_toolkit_stylesheets_shipped_next_to_it() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());

  for (file, content) in [
    ("tokens.css", eona_ui_toolkit::TOKENS_CSS),
    ("components.css", eona_ui_toolkit::COMPONENTS_CSS),
    ("ontology.css", eona_ui_toolkit::ONTOLOGY_CSS),
  ] {
    assert!(
      html.contains(&format!(r#"<link rel="stylesheet" href="assets/{file}">"#)),
      "{file} not linked:\n{html}"
    );
    assert_eq!(
      fs::read_to_string(out.path().join("eona-x.eu/vocabulary/odrl-profile/assets").join(file)).unwrap(),
      content,
      "{file}"
    );
  }
}

#[test]
fn the_html_page_uses_the_default_eona_x_theme() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());

  // The toolkit's light EONA-X palette is the default; dark is an opt-in attribute.
  assert!(html.contains(r#"<html lang="en">"#), "{html}");
  assert!(!html.contains("data-theme"), "{html}");
}

#[test]
fn the_html_page_carries_the_eona_x_mark_linking_to_eona_x_eu() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());

  let svg_head = &eona_ui_toolkit::LOGO_SVG[eona_ui_toolkit::LOGO_SVG.find("<svg").unwrap()..][..40];
  assert!(html.contains(svg_head), "the toolkit's logo is not inlined:\n{html}");
  assert!(html.contains(r#"href="https://eona-x.eu/""#), "{html}");
}

#[test]
fn terms_are_rendered_with_the_toolkit_ontology_components() {
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
fn the_page_cancels_the_toolkit_space_reserved_for_the_portal_fixed_header() {
  let out = tempfile::tempdir().unwrap();
  let html = html_page(out.path());

  // components.css reserves `html > body { padding-top }` for SiteHeader, a
  // fixed bar this page does not use; left alone it opens a blank band.
  assert!(eona_ui_toolkit::COMPONENTS_CSS.contains("html > body { padding-top:"));
  assert!(html.contains("html>body{padding-top:0}"), "{html}");
}

const VENDORED_NS: &str = "https://vocabulary.eona-x.eu/ontology/battery/v1.0.0#";

/// An Eona-X representation of someone else's standard: an external
/// `dcterms:creator` puts it on vocabulary.eona-x.eu.
fn vendored(root: &Path) {
  let dir = root.join("battery/v1.0.0");
  fs::create_dir_all(&dir).unwrap();
  fs::write(
    dir.join("ontology.ttl"),
    format!(
      r#"@prefix bat: <{VENDORED_NS}> .
@prefix owl: <http://www.w3.org/2002/07/owl#> .
@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .
bat: a owl:Ontology ; owl:versionInfo "1.0.0" ; rdfs:label "Battery"@en .
bat:Cell a owl:Class ; rdfs:label "Cell"@en ; rdfs:isDefinedBy bat: .
"#
    ),
  )
  .unwrap();
  fs::write(
    dir.join("metadata.ttl"),
    r#"@prefix dcterms: <http://purl.org/dc/terms/> .
@prefix foaf: <http://xmlns.com/foaf/0.1/> .
<> dcterms:creator [ a foaf:Organization ; foaf:name "A standards body" ] .
"#,
  )
  .unwrap();
}

fn both_hosts(root: &Path) -> Vec<Vocabulary> {
  vendored(root);
  published(root)
}

#[test]
fn each_host_gets_its_own_tree_named_after_it() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  let written = write_site(out.path(), &both_hosts(src.path())).unwrap();

  // One tree per production host, each uploaded to that host's voc/ folder
  // (dev-voc merges them, their paths never collide).
  for p in [
    "eona-x.eu/vocabulary/odrl-profile/v0.0.1.ttl",
    "vocabulary.eona-x.eu/ontology/battery/v1.0.0.ttl",
    "vocabulary.eona-x.eu/ontology/battery/v1.0.0.jsonld",
    "vocabulary.eona-x.eu/ontology/battery/v1.0.0.html",
    "vocabulary.eona-x.eu/ontology/battery/.htaccess",
    "vocabulary.eona-x.eu/ontology/battery/assets/tokens.css",
  ] {
    assert!(written.contains(&PathBuf::from(p)), "{p} not written: {written:?}");
    assert!(out.path().join(p).is_file(), "{p} missing");
  }
  assert!(!out.path().join("eona-x.eu/ontology/battery").exists());
  assert!(!out.path().join("vocabulary.eona-x.eu/vocabulary/odrl-profile").exists());
}

#[test]
fn a_vendored_vocabulary_names_its_own_host_as_canonical() {
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &both_hosts(src.path())).unwrap();

  let html = fs::read_to_string(out.path().join("vocabulary.eona-x.eu/ontology/battery/v1.0.0.html")).unwrap();
  assert!(
    html.contains(r#"<link rel="canonical" href="https://vocabulary.eona-x.eu/ontology/battery/v1.0.0">"#),
    "{html}"
  );
  let ttl = fs::read_to_string(out.path().join("vocabulary.eona-x.eu/ontology/battery/v1.0.0.ttl")).unwrap();
  assert!(ttl.contains(&format!("<{VENDORED_NS}>")), "{ttl}");
}

#[test]
fn each_slug_htaccess_tells_the_dev_overlay_which_production_host_is_canonical() {
  // dev-voc serves both hosts' paths side by side; the Link rel=canonical its
  // root .htaccess sends names the host each slug belongs to.
  let src = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();

  write_site(out.path(), &both_hosts(src.path())).unwrap();

  for (slug, host) in [
    ("eona-x.eu/vocabulary/odrl-profile", "eona-x.eu"),
    ("vocabulary.eona-x.eu/ontology/battery", "vocabulary.eona-x.eu"),
  ] {
    let htaccess = fs::read_to_string(out.path().join(slug).join(".htaccess")).unwrap();
    assert!(htaccess.contains(&format!("RewriteRule ^ - [E=CANONICAL_HOST:{host}]")), "{htaccess}");
    // Before the content-negotiation rules, which end the pass.
    let set = htaccess.find("E=CANONICAL_HOST").unwrap();
    assert!(set < htaccess.find("HTTP_ACCEPT").unwrap(), "{htaccess}");
  }
}

#[test]
fn one_asset_type_and_slug_on_both_hosts_is_refused() {
  // The paths would collide on dev-voc; upstream's gate refuses it too.
  let a = tempfile::tempdir().unwrap();
  let b = tempfile::tempdir().unwrap();
  let out = tempfile::tempdir().unwrap();
  let mut vocabularies = published(a.path());
  let dir = b.path().join("odrl-profile/v0.0.2");
  fs::create_dir_all(&dir).unwrap();
  fs::write(
    dir.join("ontology.ttl"),
    profile()
      .replace(
        "https://eona-x.eu/vocabulary/odrl-profile/v0.0.1#",
        "https://vocabulary.eona-x.eu/vocabulary/odrl-profile/v0.0.2#",
      )
      .replace(r#""0.0.1""#, r#""0.0.2""#),
  )
  .unwrap();
  fs::write(
    dir.join("metadata.ttl"),
    "@prefix dcterms: <http://purl.org/dc/terms/> .\n<> dcterms:creator \"Someone else\" .\n",
  )
  .unwrap();
  vocabularies.extend(discover(b.path(), SITE_BASE).unwrap().published);
  assert_eq!(vocabularies.len(), 2);

  let err = write_site(out.path(), &vocabularies).unwrap_err();

  assert!(err.to_string().contains("vocabulary/odrl-profile"), "{err}");
}
