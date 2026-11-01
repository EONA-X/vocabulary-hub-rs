//! The Vocabulary Hub catalog data: a `docs/` tree a hub UI can fetch at
//! runtime, built from a vocabularies checkout as it is. Every
//! `<slug>/v<version>/` with a `metadata.ttl` is an asset version; the newest
//! version of each slug is the one the UI lists.
//!
//! - `ontologies.json`, `alignments.json`: the UI's manifests, latest versions only.
//! - `<slug>/v<version>/{ontology|alignment}.{ttl,jsonld,owl,nt}`: every version's
//!   graph (the file its `metadata.ttl` declares), JSON-LD expanded as the UI parses it.
//! - `catalog.jsonld`, `catalog.ttl`: a DCAT catalog of the assets, with DCAT
//!   versioning, minted under [`HubConfig::catalog_iri`].
//! - `assets/`: logos, copied as they are (symlinks inside the checkout are ignored).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use rdf_sitegen::{Prefixes, to_turtle, write_if_changed};
use serde_json::{Map, Value, json};

use crate::config::HubConfig;
use crate::discoverer::Discoverer;
use crate::model::ASSET_TYPES;
use crate::turtle::{is_regular_file, parse_turtle};

const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const XSD_STRING: &str = "http://www.w3.org/2001/XMLSchema#string";
const RDF_LANG_STRING: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#langString";
const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const OWL_VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
const SKOS_CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";
const RDFS_SEE_ALSO: &str = "http://www.w3.org/2000/01/rdf-schema#seeAlso";
const VOID_LINKSET: &str = "http://rdfs.org/ns/void#Linkset";
const VANN_PREFIX: &str = "http://purl.org/vocab/vann/preferredNamespacePrefix";
const VANN_URI: &str = "http://purl.org/vocab/vann/preferredNamespaceUri";
const DCAT: &str = "http://www.w3.org/ns/dcat#";
const DCT: &str = "http://purl.org/dc/terms/";
const FOAF: &str = "http://xmlns.com/foaf/0.1/";
const SCHEMA_LOGO: [&str; 2] = ["http://schema.org/logo", "https://schema.org/logo"];
const FILE_TYPE: &str = "http://publications.europa.eu/resource/authority/file-type/";
const DEFAULT_THUMBNAIL: &str = "owl-illustration.svg";

const XSD: &str = "http://www.w3.org/2001/XMLSchema#";
/// The IANA media-type registry, the range of `dcat:mediaType` (`dct:MediaType`).
const IANA_MEDIA_TYPE: &str = "http://www.iana.org/assignments/media-types/";

/// Whether `s` is an `xsd:dateTime` lexical form: `YYYY-MM-DDThh:mm:ss[.fff](Z|±hh:mm)`.
fn is_xsd_date_time(s: &str) -> bool {
  let b = s.as_bytes();
  let digits = |r: std::ops::Range<usize>| b.get(r).is_some_and(|d| d.iter().all(u8::is_ascii_digit));
  let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().unwrap_or(u32::MAX);
  if !(s.is_ascii() && b.len() >= 19 && digits(0..4) && digits(5..7) && digits(8..10) && digits(11..13) && digits(14..16) && digits(17..19)) {
    return false;
  }
  if (b[4], b[7], b[10], b[13], b[16]) != (b'-', b'-', b'T', b':', b':') {
    return false;
  }
  if !(1..=12).contains(&num(5..7)) || !(1..=31).contains(&num(8..10)) || num(11..13) > 24 || num(14..16) > 59 || num(17..19) > 59 {
    return false;
  }
  let mut rest = &s[19..];
  if let Some(frac) = rest.strip_prefix('.') {
    let n = frac.bytes().take_while(u8::is_ascii_digit).count();
    if n == 0 {
      return false;
    }
    rest = &frac[n..];
  }
  match rest.as_bytes() {
    b"Z" => true,
    [b'+' | b'-', h1, h2, b':', m1, m2] => {
      [h1, h2, m1, m2].iter().all(|c| c.is_ascii_digit()) && rest[1..3].parse::<u32>().unwrap() <= 14 && rest[4..6].parse::<u32>().unwrap() <= 59
    }
    _ => false,
  }
}

/// The four published formats: extension, label, media type, EU file type.
const FORMATS: [(&str, &str, &str, &str); 4] = [
  ("ttl", "Turtle", "text/turtle", "RDF_TURTLE"),
  ("jsonld", "JSON-LD", "application/ld+json", "JSON_LD"),
  ("owl", "RDF/XML", "application/rdf+xml", "OWL"),
  ("nt", "N-Triples", "application/n-triples", "RDF_N_TRIPLES"),
];

/// One `<slug>/v<version>/` with its metadata and graph.
struct AssetVersion {
  /// The base IRI minting this and every other asset's IRIs.
  catalog_iri: String,
  slug: String,
  /// `v<version>`, the directory name.
  dir: String,
  version: String,
  title: Option<String>,
  description: Option<String>,
  /// The asset type covering its `dcat:type`, e.g. `ontology`.
  kind: Option<&'static str>,
  /// Only ever an absolute IRI (validated), else the single root ontology.
  namespace: Option<String>,
  prefix: Option<String>,
  creator: Option<String>,
  /// Only ever a plain file name (validated by `safe_logo`).
  logo: Option<String>,
  /// The metadata triples about the asset (`<>`, resolved to the asset's
  /// catalog IRI) and the blank nodes they reach, minus what only describes
  /// the source files.
  metadata: Vec<Triple>,
  graph: Vec<Triple>,
  prefixes: Vec<(String, String)>,
  /// The dereferenceable IRI the hub publishes it at, if it does.
  published: Option<String>,
}

impl AssetVersion {
  fn is_crosswalk(&self) -> bool {
    self.kind == Some("crosswalk")
  }

  /// `ontology` or `alignment`: the published graph's file name.
  fn name(&self) -> &'static str {
    if self.is_crosswalk() { "alignment" } else { "ontology" }
  }

  /// `<slug>/v<version>/<name>.<ext>`, relative to the docs tree.
  fn file(&self, ext: &str) -> String {
    format!("{}/{}/{}.{ext}", self.slug, self.dir, self.name())
  }

  fn downloads(&self) -> Value {
    json!({"ttl": self.file("ttl"), "jsonld": self.file("jsonld"), "owl": self.file("owl"), "nt": self.file("nt")})
  }

  fn asset_iri(&self) -> String {
    format!("{}/{}", self.catalog_iri, self.slug)
  }

  fn version_iri(&self) -> String {
    format!("{}/{}/{}", self.catalog_iri, self.slug, self.dir)
  }
}

/// `v1.0.0-rc.1` as its dot-separated core segments and its pre-release suffix.
fn split_version(v: &str) -> (Vec<String>, Option<&str>) {
  let v = v.trim_start_matches('v');
  let (core, pre) = v.split_once('-').map_or((v, None), |(core, pre)| (core, Some(pre)));
  (core.split('.').map(str::to_string).collect(), pre)
}

/// Orders two `v<version>` directory names by version: dot-separated numeric
/// segments compare as integers, anything else falls back to string order. A
/// pre-release suffix (`-rc.1`) ranks below its release.
fn version_order(a: &str, b: &str) -> std::cmp::Ordering {
  let ((a_segments, a_pre), (b_segments, b_pre)) = (split_version(a), split_version(b));
  for (x, y) in a_segments.iter().zip(&b_segments) {
    let ordering = match (x.parse::<u64>(), y.parse::<u64>()) {
      (Ok(x), Ok(y)) => x.cmp(&y),
      _ => x.cmp(y),
    };
    if ordering.is_ne() {
      return ordering;
    }
  }
  a_segments
    .len()
    .cmp(&b_segments.len())
    .then_with(|| match (a_pre, b_pre) {
      (None, Some(_)) => std::cmp::Ordering::Greater,
      (Some(_), None) => std::cmp::Ordering::Less,
      (a_pre, b_pre) => a_pre.cmp(&b_pre),
    })
    .then_with(|| a.cmp(b))
}

/// Writes the catalog data for the vocabularies at `vocabularies` into `out`,
/// using `config`'s branding, IRIs and `discoverer` to tell which
/// `<slug>/<version>` directories the hub itself publishes (and at which
/// IRI). Returns the written paths relative to `out`.
///
/// A version whose metadata or graph does not parse, or whose name is not an
/// IRI segment, is logged and skipped like discovery skips it; a missing
/// vocabularies directory is still an error.
pub fn write_catalog(vocabularies: &Path, out: &Path, built_at: &str, config: &HubConfig, discoverer: &dyn Discoverer) -> Result<Vec<PathBuf>, String> {
  // Catalog IRIs are joined with `/`: a trailing slash would mint `//` IRIs.
  let config = &HubConfig {
    catalog_iri: config.catalog_iri.trim_end_matches('/').to_string(),
    ..config.clone()
  };
  // The IRI each published version is served at, and its graph as the site
  // parsed it: the catalog reuses that graph so both carry the same term IRIs.
  let mut published: HashMap<String, Published> = discoverer
    .discover(vocabularies, &config.site_base)?
    .published
    .into_iter()
    .map(|v| {
      let page = format!("{}{}/{}/{}", v.base, v.kind, v.slug, v.version);
      (v.dir_name, (page, v.triples, v.prefixes))
    })
    .collect();

  // slug -> its versions, oldest first.
  let mut assets: BTreeMap<String, Vec<AssetVersion>> = BTreeMap::new();
  for dir_name in discoverer.version_dirs(vocabularies)? {
    let dir = vocabularies.join(&dir_name);
    // `version_dirs` yields each dir once; a duplicate from a custom Discoverer just re-parses.
    // Like discovery: a version without a graph file is not published yet.
    if !is_regular_file(&dir.join("metadata.ttl")) || discoverer.graph_file(&dir).is_err() {
      continue;
    }
    match read_version(&dir, &dir_name, published.remove(&dir_name), discoverer, &config.catalog_iri) {
      Ok(v) => assets.entry(v.slug.clone()).or_default().push(v),
      Err(reason) => {
        eprintln!("skip    {dir_name}: {reason}");
        continue;
      }
    }
  }

  // The discoverer promises no order, and directory names sort lexicographically
  // (v1.10.0 before v1.9.0): order each slug's versions by version number.
  for versions in assets.values_mut() {
    versions.sort_by(|a, b| version_order(&a.dir, &b.dir));
  }

  let mut written = Vec::new();
  let mut write = |rel: String, bytes: &[u8]| -> Result<(), String> {
    write_if_changed(&out.join(&rel), bytes).map_err(|e| format!("writing {rel}: {e}"))?;
    written.push(PathBuf::from(rel));
    Ok(())
  };

  for versions in assets.values() {
    for v in versions {
      let prefixes: Vec<(&str, &str)> = v.prefixes.iter().map(|(p, i)| (p.as_str(), i.as_str())).collect();
      write(v.file("ttl"), &to_turtle(&v.graph, &Prefixes(&prefixes)))?;
      write(v.file("jsonld"), &expanded_jsonld(&v.graph))?;
      write(v.file("owl"), &rdf_xml(&v.graph).map_err(|e| format!("{}: RDF/XML: {e}", v.file("owl")))?)?;
      write(v.file("nt"), &n_triples(&v.graph))?;
    }
  }

  let latest: Vec<&AssetVersion> = assets.values().filter_map(|versions| versions.last()).collect();
  let namespaces: Vec<(&str, &str)> = latest
    .iter()
    .filter(|v| !v.is_crosswalk())
    .filter_map(|v| Some((v.namespace.as_deref()?, v.slug.as_str())))
    .collect();

  let ontologies: Vec<Value> = latest
    .iter()
    .filter(|v| !v.is_crosswalk())
    .map(|v| {
      json!({
        "slug": v.slug,
        "title": v.title.as_deref().unwrap_or(&v.slug),
        "description": v.description,
        "kind": v.kind,
        "version": v.version,
        "jsonld": v.file("jsonld"),
        "namespace": v.namespace,
        "prefix": v.prefix.as_deref().unwrap_or(&v.slug),
        "upstream": v.creator.as_ref().map(|name| json!({"name": name, "logo": v.logo})),
        "downloads": v.downloads(),
      })
    })
    .collect();
  let alignments: Vec<Value> = latest
    .iter()
    .filter(|v| v.is_crosswalk())
    .map(|v| {
      json!({
        "slug": v.slug,
        "title": v.title.as_deref().unwrap_or(&v.slug),
        "description": v.description,
        "kind": v.kind,
        "version": v.version,
        "jsonld": v.file("jsonld"),
        "sides": sides(&v.graph, &namespaces),
        "downloads": v.downloads(),
      })
    })
    .collect();
  write("ontologies.json".into(), &pretty(&json!({"generatedAt": built_at, "ontologies": ontologies})))?;
  write("alignments.json".into(), &pretty(&json!({"generatedAt": built_at, "alignments": alignments})))?;

  let catalog = catalog_graph(&assets, built_at, config);
  write("catalog.jsonld".into(), &expanded_jsonld(&catalog))?;
  let prefixes = [("dcat", DCAT), ("dcterms", DCT), ("foaf", FOAF), ("adms", "http://www.w3.org/ns/adms#")];
  write("catalog.ttl".into(), &to_turtle(&catalog, &Prefixes(&prefixes)))?;

  let assets_dir = vocabularies.join("assets");
  if assets_dir.is_dir() {
    for entry in fs::read_dir(&assets_dir).map_err(|e| format!("{}: {e}", assets_dir.display()))? {
      let entry = entry.map_err(|e| e.to_string())?;
      let path = entry.path();
      let is_file = entry.file_type().map_err(|e| format!("{}: {e}", path.display()))?.is_file();
      if is_file {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        write(format!("assets/{name}"), &fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)?;
      }
    }
  }
  Ok(written)
}

/// A published version's page IRI, triples and prefixes, as the discoverer parsed them.
type Published = (String, Vec<Triple>, Vec<(String, String)>);

fn read_version(dir: &Path, dir_name: &str, published: Option<Published>, discoverer: &dyn Discoverer, catalog_iri: &str) -> Result<AssetVersion, String> {
  let (slug, version_dir) = dir_name.split_once('/').ok_or_else(|| format!("{dir_name}: not <slug>/<version>"))?;
  let asset_iri = format!("{catalog_iri}/{slug}");
  let (meta, _) = parse_turtle(&dir.join("metadata.ttl"), &asset_iri)?;
  let graph_file = discoverer.graph_file(dir)?;
  let (published, graph, prefixes) = match published {
    Some((page, graph, prefixes)) => (Some(page), graph, prefixes),
    None => {
      let (graph, prefixes) = parse_turtle(&dir.join(&graph_file), &format!("{asset_iri}/{version_dir}/"))?;
      (None, graph, prefixes)
    }
  };

  let subject = NamedOrBlankNode::NamedNode(NamedNode::new_unchecked(&asset_iri));
  let literal = |p: &str| objects(&meta, &subject, p).into_iter().find_map(literal_value);

  let codes: Vec<&str> = objects(&meta, &subject, &format!("{DCAT}type"))
    .into_iter()
    .filter_map(|o| match o {
      Term::NamedNode(n) => n.as_str().rsplit('/').next(),
      _ => None,
    })
    .collect();
  let kind = ASSET_TYPES
    .iter()
    .find(|(_, covered)| codes.iter().any(|c| covered.contains(c)))
    .map(|(name, _)| *name);
  let creator = objects(&meta, &subject, &format!("{DCT}creator")).into_iter().find_map(|agent| {
    let agent = match agent {
      Term::NamedNode(n) => NamedOrBlankNode::NamedNode(n.clone()),
      Term::BlankNode(b) => NamedOrBlankNode::BlankNode(b.clone()),
      Term::Literal(l) => return Some(l.value().to_string()),
      #[allow(unreachable_patterns)]
      _ => return None,
    };
    objects(&meta, &agent, &format!("{FOAF}name")).into_iter().find_map(literal_value)
  });
  let vann = |p: &str| {
    meta
      .iter()
      .chain(&graph)
      .filter(|t| t.predicate.as_str() == p)
      .find_map(|t| literal_value(&t.object))
  };
  let roots = roots(&graph);
  let namespace = vann(VANN_URI)
    .filter(|ns| NamedNode::new(ns.as_str()).is_ok())
    .or_else(|| (roots.len() == 1).then(|| roots[0].clone()));
  let version = literal(&format!("{DCAT}version"))
    .or_else(|| {
      roots.first().and_then(|r| {
        graph
          .iter()
          .find(|t| t.predicate.as_str() == OWL_VERSION_INFO && matches!(&t.subject, NamedOrBlankNode::NamedNode(s) if s.as_str() == r.as_str()))
          .and_then(|t| literal_value(&t.object))
      })
    })
    .unwrap_or_else(|| version_dir.trim_start_matches('v').to_string());

  // What only describes the source files: its graph file, logo, namespace
  // prefix, and its version (computed below, per version).
  let dropped = [
    format!("{DCAT}distribution"),
    format!("{DCAT}version"),
    SCHEMA_LOGO[0].into(),
    SCHEMA_LOGO[1].into(),
  ];
  let mut metadata: Vec<Triple> = meta
    .iter()
    .filter(|t| t.subject == subject && !dropped.iter().any(|d| d == t.predicate.as_str()))
    .cloned()
    .collect();
  let mut by_blank: HashMap<&BlankNode, Vec<&Triple>> = HashMap::new();
  for t in &meta {
    if let NamedOrBlankNode::BlankNode(b) = &t.subject {
      by_blank.entry(b).or_default().push(t);
    }
  }
  let mut reached: HashSet<BlankNode> = HashSet::new();
  let mut frontier: Vec<BlankNode> = metadata
    .iter()
    .filter_map(|t| if let Term::BlankNode(b) = &t.object { Some(b.clone()) } else { None })
    .collect();
  while let Some(b) = frontier.pop() {
    if !reached.insert(b.clone()) {
      continue;
    }
    for t in by_blank.get(&b).into_iter().flatten() {
      metadata.push((*t).clone());
      if let Term::BlankNode(o) = &t.object {
        frontier.push(o.clone());
      }
    }
  }

  Ok(AssetVersion {
    catalog_iri: catalog_iri.to_string(),
    slug: slug.to_string(),
    dir: version_dir.to_string(),
    version,
    title: literal(&format!("{DCT}title")),
    description: literal(&format!("{DCT}description")),
    kind,
    namespace,
    prefix: vann(VANN_PREFIX),
    creator,
    logo: SCHEMA_LOGO.iter().find_map(|p| literal(p)).and_then(|l| safe_logo(&l)),
    metadata,
    graph,
    prefixes,
    published,
  })
}

/// The DCAT catalog: the catalog, one dataset per asset (its latest metadata),
/// one dataset per version, and each version's four distributions.
fn catalog_graph(assets: &BTreeMap<String, Vec<AssetVersion>>, built_at: &str, config: &HubConfig) -> Vec<Triple> {
  let mut g = Graph::default();
  let catalog = config.catalog_iri.as_str();
  g.iri(catalog, RDF_TYPE, &format!("{DCAT}Catalog"));
  g.lang(catalog, &format!("{DCT}title"), &config.catalog_title);
  g.lang(catalog, &format!("{DCT}description"), &config.catalog_description);
  if is_xsd_date_time(built_at) {
    g.push(
      catalog,
      &format!("{DCT}modified"),
      Literal::new_typed_literal(built_at, NamedNode::new_unchecked(format!("{XSD}dateTime"))).into(),
    );
  }
  // Vocabularies in slug order, then crosswalks.
  let (crosswalks, others): (Vec<_>, Vec<_>) = assets.values().filter_map(|versions| versions.last()).partition(|v| v.is_crosswalk());
  for v in others.iter().chain(&crosswalks) {
    g.iri(catalog, &format!("{DCAT}dataset"), &v.asset_iri());
  }

  // Blank nodes get catalog-wide labels `m0`, `m1`, ... in asset order, so
  // unchanged input rebuilds to identical files.
  let mut next_blank_node = 0;
  for versions in assets.values() {
    let Some(latest) = versions.last() else { continue };
    let asset = latest.asset_iri();
    // Standardize the blank nodes apart: labels are only unique per file.
    let mut fresh: HashMap<BlankNode, BlankNode> = HashMap::new();
    let mut rename = |b: &BlankNode| {
      fresh
        .entry(b.clone())
        .or_insert_with(|| {
          next_blank_node += 1;
          BlankNode::new_unchecked(format!("m{}", next_blank_node - 1))
        })
        .clone()
    };
    for t in &latest.metadata {
      let subject = match &t.subject {
        NamedOrBlankNode::BlankNode(b) => NamedOrBlankNode::BlankNode(rename(b)),
        other => other.clone(),
      };
      let object = match &t.object {
        Term::BlankNode(b) => Term::BlankNode(rename(b)),
        other => other.clone(),
      };
      g.triples.push(Triple::new(subject, t.predicate.clone(), object));
    }
    g.plain(&asset, &format!("{DCT}identifier"), &latest.slug);
    g.plain(&asset, &format!("{DCAT}version"), &latest.version);
    if let Some(ns) = &latest.namespace {
      g.iri(&asset, &format!("{DCT}conformsTo"), ns);
    }
    let docs_base = &config.docs_base;
    let landing = if latest.is_crosswalk() {
      format!("{docs_base}/crosswalks?crosswalk={}", latest.slug)
    } else {
      format!("{docs_base}/ontologies?ontology={}", latest.slug)
    };
    g.iri(&asset, &format!("{DCAT}landingPage"), &landing);
    let thumbnail = latest.logo.as_deref().unwrap_or(DEFAULT_THUMBNAIL);
    g.iri(&asset, &format!("{FOAF}thumbnail"), &format!("{docs_base}/assets/{thumbnail}"));
    for (ext, ..) in FORMATS {
      g.iri(&asset, &format!("{DCAT}distribution"), &format!("{}/{ext}", latest.version_iri()));
    }
    for v in versions {
      g.iri(&asset, &format!("{DCAT}hasVersion"), &v.version_iri());
    }
    g.iri(&asset, &format!("{DCAT}hasCurrentVersion"), &latest.version_iri());

    let mut previous: Option<&AssetVersion> = None;
    for v in versions {
      let version = v.version_iri();
      g.iri(&version, RDF_TYPE, &format!("{DCAT}Dataset"));
      g.plain(&version, &format!("{DCAT}version"), &v.version);
      g.iri(&version, &format!("{DCT}isVersionOf"), &asset);
      if let Some(p) = previous {
        g.iri(&version, &format!("{DCAT}previousVersion"), &p.version_iri());
      }
      if let Some(page) = &v.published {
        g.iri(&version, &format!("{FOAF}page"), page);
      }
      let title = v.title.as_deref().unwrap_or(&v.slug);
      for (ext, label, media_type, file_type) in FORMATS {
        let distribution = format!("{version}/{ext}");
        g.iri(&version, &format!("{DCAT}distribution"), &distribution);
        g.iri(&distribution, RDF_TYPE, &format!("{DCAT}Distribution"));
        g.lang(&distribution, &format!("{DCT}title"), &format!("{title} {} ({label})", v.version));
        g.iri(&distribution, &format!("{DCAT}downloadURL"), &format!("{docs_base}/{}", v.file(ext)));
        g.iri(&distribution, &format!("{DCAT}mediaType"), &format!("{IANA_MEDIA_TYPE}{media_type}"));
        g.iri(&distribution, &format!("{DCT}format"), &format!("{FILE_TYPE}{file_type}"));
      }
      previous = Some(v);
    }
  }
  g.triples
}

/// A logo literal is used as one path segment of an IRI: accept only a plain
/// file name (`[A-Za-z0-9._-]+`, not hidden), which needs no percent-encoding.
fn safe_logo(s: &str) -> Option<String> {
  let ok = !s.is_empty() && !s.starts_with('.') && s.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
  ok.then(|| s.to_string())
}

#[derive(Default)]
struct Graph {
  triples: Vec<Triple>,
}

impl Graph {
  /// Adds the triple unless one of its IRIs is invalid, which would
  /// otherwise be written raw and could inject triples.
  fn push(&mut self, s: &str, p: &str, o: Term) {
    if let (Ok(s), Ok(p)) = (NamedNode::new(s), NamedNode::new(p)) {
      self.triples.push(Triple::new(s, p, o));
    }
  }
  fn iri(&mut self, s: &str, p: &str, o: &str) {
    if let Ok(o) = NamedNode::new(o) {
      self.push(s, p, o.into());
    }
  }
  fn plain(&mut self, s: &str, p: &str, o: &str) {
    self.push(s, p, Literal::new_simple_literal(o).into());
  }
  fn lang(&mut self, s: &str, p: &str, o: &str) {
    self.push(s, p, Literal::new_language_tagged_literal_unchecked(o, "en").into());
  }
}

/// The ontologies a crosswalk's `void:Linkset` reaches (`rdfs:seeAlso`), in
/// authored order, as slugs. A target belongs to the ontology with the longest
/// matching namespace, so one namespace that prefixes another doesn't claim it.
fn sides(graph: &[Triple], namespaces: &[(&str, &str)]) -> Vec<String> {
  let linksets: HashSet<&NamedOrBlankNode> = graph
    .iter()
    .filter(|t| t.predicate.as_str() == RDF_TYPE && matches!(&t.object, Term::NamedNode(o) if o.as_str() == VOID_LINKSET))
    .map(|t| &t.subject)
    .collect();
  let mut sides: Vec<String> = Vec::new();
  for t in graph.iter().filter(|t| linksets.contains(&t.subject) && t.predicate.as_str() == RDFS_SEE_ALSO) {
    let Term::NamedNode(target) = &t.object else { continue };
    if let Some((_, slug)) = namespaces
      .iter()
      .filter(|(ns, _)| target.as_str().starts_with(ns))
      .max_by_key(|(ns, _)| ns.len())
      && !sides.iter().any(|s| s == slug)
    {
      sides.push(slug.to_string());
    }
  }
  sides
}

/// `owl:Ontology` and `skos:ConceptScheme` subjects, without those nested under another.
fn roots(graph: &[Triple]) -> Vec<String> {
  let mut iris: Vec<&str> = graph
    .iter()
    .filter_map(|t| match (&t.subject, &t.object) {
      (NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) if t.predicate.as_str() == RDF_TYPE && [OWL_ONTOLOGY, SKOS_CONCEPT_SCHEME].contains(&o.as_str()) => {
        Some(s.as_str())
      }
      _ => None,
    })
    .collect();
  iris.sort_unstable();
  iris.dedup();
  let all = iris.clone();
  iris.retain(|iri| !all.iter().any(|other| other != iri && crate::default_discovery::is_nested_under(iri, other)));
  iris.into_iter().map(String::from).collect()
}

fn objects<'a>(triples: &'a [Triple], subject: &NamedOrBlankNode, predicate: &str) -> Vec<&'a Term> {
  triples
    .iter()
    .filter(|t| &t.subject == subject && t.predicate.as_str() == predicate)
    .map(|t| &t.object)
    .collect()
}

fn literal_value(term: &Term) -> Option<String> {
  match term {
    Term::Literal(l) => Some(l.value().to_string()),
    _ => None,
  }
}

/// Expanded JSON-LD, one node object per subject in first-seen order, every
/// value in an array — the only shape the UI's parsers read.
fn expanded_jsonld(triples: &[Triple]) -> Vec<u8> {
  let mut order: Vec<String> = Vec::new();
  // Per node: its object plus, per key, the canonical text of the values already added.
  type Seen = HashMap<String, HashSet<String>>;
  let mut nodes: HashMap<String, (Map<String, Value>, Seen)> = HashMap::new();
  for t in triples {
    let id = node_id(&t.subject);
    if !nodes.contains_key(&id) {
      order.push(id.clone());
      let mut m = Map::new();
      m.insert("@id".into(), Value::String(id.clone()));
      nodes.insert(id.clone(), (m, Seen::new()));
    }
    let (node, seen) = nodes.get_mut(&id).expect("node inserted above");
    let (key, value) = match (&t.object, t.predicate.as_str()) {
      (Term::NamedNode(o), RDF_TYPE) => ("@type".to_string(), Value::String(o.as_str().into())),
      (Term::BlankNode(o), RDF_TYPE) => ("@type".to_string(), Value::String(format!("_:{}", o.as_str()))),
      (Term::NamedNode(o), p) => (p.to_string(), json!({"@id": o.as_str()})),
      (Term::BlankNode(o), p) => (p.to_string(), json!({"@id": format!("_:{}", o.as_str())})),
      (Term::Literal(l), p) => {
        let mut v = Map::new();
        v.insert("@value".into(), Value::String(l.value().into()));
        if let Some(lang) = l.language() {
          v.insert("@language".into(), Value::String(lang.into()));
        } else if ![XSD_STRING, RDF_LANG_STRING].contains(&l.datatype().as_str()) {
          v.insert("@type".into(), Value::String(l.datatype().as_str().into()));
        }
        (p.to_string(), Value::Object(v))
      }
      #[allow(unreachable_patterns)]
      _ => continue,
    };
    let canonical = value.to_string();
    if !seen.entry(key.clone()).or_default().insert(canonical) {
      continue;
    }
    if let Value::Array(a) = node.entry(key).or_insert_with(|| Value::Array(Vec::new())) {
      a.push(value);
    }
  }
  let doc: Vec<Value> = order.into_iter().map(|id| Value::Object(nodes.remove(&id).unwrap().0)).collect();
  pretty(&Value::Array(doc))
}

fn node_id(s: &NamedOrBlankNode) -> String {
  match s {
    NamedOrBlankNode::NamedNode(n) => n.as_str().into(),
    NamedOrBlankNode::BlankNode(b) => format!("_:{}", b.as_str()),
  }
}

fn rdf_xml(triples: &[Triple]) -> std::io::Result<Vec<u8>> {
  let mut s = oxrdfxml::RdfXmlSerializer::new().for_writer(Vec::new());
  for t in triples {
    s.serialize_triple(t)?;
  }
  s.finish()
}

fn n_triples(triples: &[Triple]) -> Vec<u8> {
  let mut s = oxttl::NTriplesSerializer::new().for_writer(Vec::new());
  for t in triples {
    s.serialize_triple(t).expect("writing to memory");
  }
  s.finish()
}

fn pretty(value: &Value) -> Vec<u8> {
  let mut bytes = serde_json::to_vec_pretty(value).expect("JSON values serialize");
  bytes.push(b'\n');
  bytes
}

#[cfg(test)]
mod tests {
  use super::{expanded_jsonld, safe_logo};
  use oxrdf::{Literal, NamedNode, Term, Triple};
  use serde_json::{Value, json};

  #[test]
  fn logo_must_be_a_single_plain_file_name() {
    assert_eq!(safe_logo("alpha-logo.png").as_deref(), Some("alpha-logo.png"));
    for bad in ["a/b.png", "../x.png", ".hidden", "a b.png", "a>b.svg", ""] {
      assert_eq!(safe_logo(bad), None, "{bad}");
    }
  }

  fn iri_triple(s: &str, p: &str, o: &str) -> Triple {
    Triple::new(NamedNode::new_unchecked(s), NamedNode::new_unchecked(p), NamedNode::new_unchecked(o))
  }

  fn lit_triple(s: &str, p: &str, o: Literal) -> Triple {
    Triple::new(NamedNode::new_unchecked(s), NamedNode::new_unchecked(p), Term::Literal(o))
  }

  #[test]
  fn expanded_jsonld_keeps_first_seen_order_and_drops_duplicate_values() {
    let (s, p) = ("https://example.org/s", "https://example.org/p");
    let triples = vec![
      iri_triple(s, p, "https://example.org/o1"),
      iri_triple(s, p, "https://example.org/o2"),
      iri_triple(s, p, "https://example.org/o1"),
      lit_triple(s, p, Literal::new_simple_literal("lit")),
      lit_triple(
        s,
        p,
        Literal::new_typed_literal("lit", NamedNode::new_unchecked("http://www.w3.org/2001/XMLSchema#string")),
      ),
      lit_triple(s, p, Literal::new_language_tagged_literal_unchecked("lit", "en")),
    ];
    let doc: Value = serde_json::from_slice(&expanded_jsonld(&triples)).unwrap();
    assert_eq!(
      doc,
      json!([{"@id": s, p: [
        {"@id": "https://example.org/o1"},
        {"@id": "https://example.org/o2"},
        {"@value": "lit"},
        {"@value": "lit", "@language": "en"},
      ]}])
    );
  }

  #[test]
  fn expanded_jsonld_is_linear_in_values_per_property() {
    let build = |n: usize| -> Vec<Triple> {
      (0..n)
        .map(|i| {
          iri_triple(
            "https://example.org/s",
            "https://example.org/p",
            &format!("https://example.org/a-long-shared-prefix/item/{i}"),
          )
        })
        .collect()
    };
    let time = |n: usize| {
      let triples = build(n);
      (0..3)
        .map(|_| {
          let start = std::time::Instant::now();
          let _ = expanded_jsonld(&triples);
          start.elapsed()
        })
        .min()
        .unwrap()
    };
    let (t2, t8) = (time(2_000), time(8_000));
    assert!(t8 < t2 * 8, "quadratic: 2000 values took {t2:?}, 8000 took {t8:?}");
  }
}
