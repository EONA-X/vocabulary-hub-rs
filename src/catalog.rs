//! The Vocabulary Hub catalog data: the `docs/` tree the Vocabulary Hub UI
//! (eona-vocabulary-ui) fetches at runtime, built from eona-vocabularies-reference
//! as it is. Every `<slug>/v<version>/` with a `metadata.ttl` is an asset
//! version; the newest version of each slug is the one the UI lists.
//!
//! - `ontologies.json`, `alignments.json`: the UI's manifests, latest versions only.
//! - `<slug>/v<version>/{ontology|alignment}.{ttl,jsonld,owl,nt}`: every version's
//!   graph (the file its `metadata.ttl` declares), JSON-LD expanded as the UI parses it.
//! - `catalog.jsonld`, `catalog.ttl`: a DCAT catalog of the assets, with DCAT
//!   versioning, minted under `https://vocabulary.eona-x.eu/catalog`.
//! - `assets/`: logos, copied as they are.
//!
//! Ported from eona-vocabulary-services' publish-vocabulary-catalog/generate.py.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use oxrdf::{BlankNode, Literal, NamedNode, NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;
use rdf_sitegen::{Prefixes, to_turtle, write_if_changed};
use serde_json::{Map, Value, json};

use crate::{ASSET_TYPES, SITE_BASE, discover};

pub const CATALOG_IRI: &str = "https://vocabulary.eona-x.eu/catalog";
/// Where the UI serves the `docs/` tree; the UI resolves these IRIs to its own
/// paths (`site_relative`).
pub const DOCS_BASE: &str = "https://vocabulary.eona-x.eu/docs";

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

/// The four published formats: extension, label, media type, EU file type.
const FORMATS: [(&str, &str, &str, &str); 4] = [
  ("ttl", "Turtle", "text/turtle", "RDF_TURTLE"),
  ("jsonld", "JSON-LD", "application/ld+json", "JSON_LD"),
  ("owl", "RDF/XML", "application/rdf+xml", "OWL"),
  ("nt", "N-Triples", "application/n-triples", "RDF_N_TRIPLES"),
];

/// One `<slug>/v<version>/` with its metadata and graph.
struct AssetVersion {
  slug: String,
  /// `v<version>`, the directory name.
  dir: String,
  version: String,
  title: Option<String>,
  description: Option<String>,
  /// The asset type covering its `dcat:type`, e.g. `ontology`.
  kind: Option<&'static str>,
  namespace: Option<String>,
  prefix: Option<String>,
  creator: Option<String>,
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
    format!("{CATALOG_IRI}/{}", self.slug)
  }

  fn version_iri(&self) -> String {
    format!("{CATALOG_IRI}/{}/{}", self.slug, self.dir)
  }
}

/// Writes the catalog data for the vocabularies at `vocabularies` into `out`,
/// returning the written paths relative to `out`.
pub fn write_catalog(vocabularies: &Path, out: &Path, built_at: &str) -> Result<Vec<PathBuf>, String> {
  let published: HashMap<String, String> = discover(vocabularies, SITE_BASE)?
    .published
    .into_iter()
    .map(|v| (v.dir_name, format!("{}{}/{}/{}", v.base, v.kind, v.slug, v.version)))
    .collect();

  // slug -> its versions, oldest first.
  let mut assets: BTreeMap<String, Vec<AssetVersion>> = BTreeMap::new();
  for dir_name in eona_vocabulary_check::convention::version_dirs(vocabularies)? {
    let dir = vocabularies.join(&dir_name);
    if !dir.join("metadata.ttl").is_file() {
      continue;
    }
    let v = read_version(&dir, &dir_name, published.get(&dir_name).cloned())?;
    assets.entry(v.slug.clone()).or_default().push(v);
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

  let catalog = catalog_graph(&assets, built_at);
  write("catalog.jsonld".into(), &expanded_jsonld(&catalog))?;
  let prefixes = [("dcat", DCAT), ("dcterms", DCT), ("foaf", FOAF), ("adms", "http://www.w3.org/ns/adms#")];
  write("catalog.ttl".into(), &to_turtle(&catalog, &Prefixes(&prefixes)))?;

  let assets_dir = vocabularies.join("assets");
  if assets_dir.is_dir() {
    for entry in fs::read_dir(&assets_dir).map_err(|e| format!("{}: {e}", assets_dir.display()))? {
      let path = entry.map_err(|e| e.to_string())?.path();
      if path.is_file() {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        write(format!("assets/{name}"), &fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?)?;
      }
    }
  }
  Ok(written)
}

fn read_version(dir: &Path, dir_name: &str, published: Option<String>) -> Result<AssetVersion, String> {
  let (slug, version_dir) = dir_name.split_once('/').ok_or_else(|| format!("{dir_name}: not <slug>/<version>"))?;
  let asset_iri = format!("{CATALOG_IRI}/{slug}");
  let (meta, _) = parse_turtle(&dir.join("metadata.ttl"), &asset_iri)?;
  let graph_file = eona_vocabulary_check::graph_file(dir)?;
  let (graph, prefixes) = parse_turtle(&dir.join(&graph_file), &format!("{asset_iri}/{version_dir}/"))?;

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
  let namespace = vann(VANN_URI).or_else(|| (roots.len() == 1).then(|| roots[0].clone()));
  let version = literal(&format!("{DCAT}version"))
    .or_else(|| {
      roots.first().and_then(|r| {
        graph
          .iter()
          .find(|t| t.subject.to_string() == format!("<{r}>") && t.predicate.as_str() == OWL_VERSION_INFO)
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
  let mut reached: HashSet<BlankNode> = HashSet::new();
  let mut frontier: Vec<BlankNode> = metadata
    .iter()
    .filter_map(|t| if let Term::BlankNode(b) = &t.object { Some(b.clone()) } else { None })
    .collect();
  while let Some(b) = frontier.pop() {
    if !reached.insert(b.clone()) {
      continue;
    }
    for t in meta.iter().filter(|t| t.subject == NamedOrBlankNode::BlankNode(b.clone())) {
      metadata.push(t.clone());
      if let Term::BlankNode(o) = &t.object {
        frontier.push(o.clone());
      }
    }
  }

  Ok(AssetVersion {
    slug: slug.to_string(),
    dir: version_dir.to_string(),
    version,
    title: literal(&format!("{DCT}title")),
    description: literal(&format!("{DCT}description")),
    kind,
    namespace,
    prefix: vann(VANN_PREFIX),
    creator,
    logo: SCHEMA_LOGO.iter().find_map(|p| literal(p)),
    metadata,
    graph,
    prefixes,
    published,
  })
}

/// The DCAT catalog: the catalog, one dataset per asset (its latest metadata),
/// one dataset per version, and each version's four distributions.
fn catalog_graph(assets: &BTreeMap<String, Vec<AssetVersion>>, built_at: &str) -> Vec<Triple> {
  let mut g = Graph::default();
  let catalog = CATALOG_IRI;
  g.iri(catalog, RDF_TYPE, &format!("{DCAT}Catalog"));
  g.lang(catalog, &format!("{DCT}title"), "Eona-X Vocabulary Hub");
  g.lang(
    catalog,
    &format!("{DCT}description"),
    "The ontologies, shapes, vocabularies, code lists and crosswalks of the Eona-X data space.",
  );
  g.plain(catalog, &format!("{DCT}modified"), built_at);
  // Vocabularies in slug order, then crosswalks.
  let (crosswalks, others): (Vec<_>, Vec<_>) = assets.values().filter_map(|versions| versions.last()).partition(|v| v.is_crosswalk());
  for v in others.iter().chain(&crosswalks) {
    g.iri(catalog, &format!("{DCAT}dataset"), &v.asset_iri());
  }

  for versions in assets.values() {
    let Some(latest) = versions.last() else { continue };
    let asset = latest.asset_iri();
    g.triples.extend(latest.metadata.iter().cloned());
    g.plain(&asset, &format!("{DCT}identifier"), &latest.slug);
    g.plain(&asset, &format!("{DCAT}version"), &latest.version);
    if let Some(ns) = &latest.namespace {
      g.iri(&asset, &format!("{DCT}conformsTo"), ns);
    }
    let landing = if latest.is_crosswalk() {
      format!("{DOCS_BASE}/crosswalk?crosswalk={}", latest.slug)
    } else {
      format!("{DOCS_BASE}/ontologies?ontology={}", latest.slug)
    };
    g.iri(&asset, &format!("{DCAT}landingPage"), &landing);
    let thumbnail = latest.logo.as_deref().unwrap_or(DEFAULT_THUMBNAIL);
    g.iri(&asset, &format!("{FOAF}thumbnail"), &format!("{DOCS_BASE}/assets/{thumbnail}"));
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
        g.iri(&distribution, &format!("{DCAT}downloadURL"), &format!("{DOCS_BASE}/{}", v.file(ext)));
        g.plain(&distribution, &format!("{DCAT}mediaType"), media_type);
        g.iri(&distribution, &format!("{DCT}format"), &format!("{FILE_TYPE}{file_type}"));
      }
      previous = Some(v);
    }
  }
  g.triples
}

#[derive(Default)]
struct Graph {
  triples: Vec<Triple>,
}

impl Graph {
  fn push(&mut self, s: &str, p: &str, o: Term) {
    self.triples.push(Triple::new(NamedNode::new_unchecked(s), NamedNode::new_unchecked(p), o));
  }
  fn iri(&mut self, s: &str, p: &str, o: &str) {
    self.push(s, p, NamedNode::new_unchecked(o).into());
  }
  fn plain(&mut self, s: &str, p: &str, o: &str) {
    self.push(s, p, Literal::new_simple_literal(o).into());
  }
  fn lang(&mut self, s: &str, p: &str, o: &str) {
    self.push(s, p, Literal::new_language_tagged_literal_unchecked(o, "en").into());
  }
}

/// The ontologies a crosswalk's `void:Linkset` reaches (`rdfs:seeAlso`), in
/// authored order, as slugs.
fn sides(graph: &[Triple], namespaces: &[(&str, &str)]) -> Vec<String> {
  let linksets: Vec<&NamedOrBlankNode> = graph
    .iter()
    .filter(|t| t.predicate.as_str() == RDF_TYPE && matches!(&t.object, Term::NamedNode(o) if o.as_str() == VOID_LINKSET))
    .map(|t| &t.subject)
    .collect();
  let mut sides: Vec<String> = Vec::new();
  for t in graph.iter().filter(|t| linksets.contains(&&t.subject) && t.predicate.as_str() == RDFS_SEE_ALSO) {
    let Term::NamedNode(target) = &t.object else { continue };
    if let Some((_, slug)) = namespaces.iter().find(|(ns, _)| target.as_str().starts_with(ns))
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
  iris.retain(|iri| !all.iter().any(|other| other != iri && iri.starts_with(other)));
  iris.into_iter().map(String::from).collect()
}

/// A graph and its `@prefix` table.
type Parsed = (Vec<Triple>, Vec<(String, String)>);

fn parse_turtle(path: &Path, base: &str) -> Result<Parsed, String> {
  let data = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
  let mut parser = TurtleParser::new().with_base_iri(base).map_err(|e| format!("{base}: {e}"))?.for_slice(&data);
  let mut triples = Vec::new();
  for t in parser.by_ref() {
    triples.push(t.map_err(|e| format!("{}: {e}", path.display()))?);
  }
  let prefixes = parser.prefixes().map(|(p, iri)| (p.to_string(), iri.to_string())).collect();
  Ok((triples, prefixes))
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
  let mut nodes: HashMap<String, Map<String, Value>> = HashMap::new();
  for t in triples {
    let id = node_id(&t.subject);
    let node = nodes.entry(id.clone()).or_insert_with(|| {
      order.push(id.clone());
      let mut m = Map::new();
      m.insert("@id".into(), Value::String(id.clone()));
      m
    });
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
    let values = node.entry(key).or_insert_with(|| Value::Array(Vec::new()));
    if let Value::Array(a) = values
      && !a.contains(&value)
    {
      a.push(value);
    }
  }
  let doc: Vec<Value> = order.into_iter().map(|id| Value::Object(nodes.remove(&id).unwrap())).collect();
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
