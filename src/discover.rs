use std::fs;
use std::path::Path;

use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;

const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const SKOS_CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";
const DCAT_TYPE: &str = "http://www.w3.org/ns/dcat#type";
const OWL_VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";

/// The EU asset-classification authority table, whose concept codes are the
/// first path segment of every published IRI.
pub const ASSET_CLASSIFICATION: &str = "http://publications.europa.eu/resource/authority/asset-classification/";

/// Every concept of [`ASSET_CLASSIFICATION`] an Eona-X asset can be classified
/// under (the table's `c_*` concepts, as retrieved on 2026-09-30).
pub const ASSET_CLASSES: [&str; 34] = [
  "c_1fb7be39", // Relational model
  "c_230aaab7", // Presentation style sheet
  "c_25e514f4", // Lexicon
  "c_2db4690d", // Mind map
  "c_32ffe82d", // Business Process Model and Notation
  "c_35dcf03c", // Argument map
  "c_3626c85a", // Subject headings
  "c_366e3b24", // Base registry
  "c_3948c2ed", // Markup schema
  "c_5796a20b", // Dictionary
  "c_5b130cc6", // Authority file
  "c_638e1719", // Authoritative source
  "c_64714767", // Terminology
  "c_722cef3f", // Data model
  "c_89b4bdb7", // Formal ontology
  "c_8da81c02", // Classification
  "c_a2df28a1", // Taxonomy
  "c_a6822084", // System network
  "c_a7773248", // Thesaurus
  "c_b0bfba6e", // Categorisation
  "c_b37963b3", // Application profile
  "c_b81ceeea", // Entity-relationship model
  "c_bba2bb35", // Alignment
  "c_c0b5956c", // Frame-based model
  "c_c2f98e5f", // Conceptual model
  "c_c2ffe197", // Semantic network
  "c_cdd11291", // Code list
  "c_d3bf7907", // Synonym ring
  "c_d5c26195", // Topic map
  "c_d9dd5715", // Syntax encoding scheme
  "c_e4cf092c", // Semantic model
  "c_ebfb658e", // Glossary
  "c_ecacbeba", // Folksonomy
  "c_eed252ff", // Object-oriented model
];

/// The shape shared by every code in [`ASSET_CLASSES`]: what the hand-installed
/// router and dev overlay match, as an Apache regex.
pub const ASSET_CLASS_PATTERN: &str = "c_[0-9a-f]{8}";

/// One publishable vocabulary version.
pub struct Vocabulary {
  /// Source directory name in eona-vocabularies-reference, e.g. `eonax-odrl-profile`.
  pub dir_name: String,
  /// The ontology IRI, which is also the term namespace, e.g. `https://eona-x.eu/odrl-profile/v0.0.1#`.
  pub namespace: String,
  /// First path segment under the site base: an EU asset-classification
  /// concept code, e.g. `c_89b4bdb7` (Formal ontology).
  pub kind: String,
  /// Second path segment, e.g. `odrl-profile`.
  pub slug: String,
  /// Second path segment, e.g. `v0.0.1`.
  pub version: String,
  pub triples: Vec<Triple>,
  /// The source file's own `@prefix` table, reused when re-serializing.
  pub prefixes: PrefixTable,
}

/// A candidate directory that is deliberately not published, and why.
pub struct Skipped {
  pub dir_name: String,
  pub reason: String,
}

pub struct Discovery {
  pub published: Vec<Vocabulary>,
  pub skipped: Vec<Skipped>,
}

/// Scans `vocabularies_root` (a checkout of eona-vocabularies-reference) for
/// `eonax-*` directories and splits them into publishable and skipped.
///
/// Publishability is read off the source itself rather than configured here:
/// a vocabulary is published exactly when its root resource — its
/// `owl:Ontology`, or `skos:ConceptScheme` for a code list or thesaurus — has
/// the IRI `{site_base}{asset-class}/{slug}/{version}#`, i.e. when upstream has
/// frozen its namespace on the site. `{asset-class}` is a concept code of the
/// EU asset-classification table, and, when the directory's `metadata.ttl`
/// declares a `dcat:type` from that table, exactly that one.
/// Anything still minted elsewhere (`w3id.org/eonax/...`) is skipped, because
/// serving it here would publish documents whose terms don't dereference here.
///
/// Errors are reserved for sources that claim the site base but break its
/// convention — publishing those would put wrong IRIs on eona-x.eu.
pub fn discover(vocabularies_root: &Path, site_base: &str) -> Result<Discovery, String> {
  let mut dirs: Vec<String> = fs::read_dir(vocabularies_root)
    .map_err(|e| format!("cannot read {}: {e}", vocabularies_root.display()))?
    .filter_map(Result::ok)
    .filter(|e| e.path().is_dir())
    .filter_map(|e| e.file_name().into_string().ok())
    .filter(|name| name.starts_with("eonax-"))
    .collect();
  dirs.sort();

  let mut found = Discovery {
    published: Vec::new(),
    skipped: Vec::new(),
  };
  for dir_name in dirs {
    let path = vocabularies_root.join(&dir_name).join("ontology.ttl");
    if !path.is_file() {
      found.skipped.push(Skipped {
        dir_name,
        reason: "no ontology.ttl".into(),
      });
      continue;
    }
    let (triples, prefixes) = parse_turtle(&path)?;
    let namespace = ontology_iri(&triples).map_err(|e| format!("{}: {e}", path.display()))?;
    let Some(rest) = namespace.strip_prefix(site_base) else {
      found.skipped.push(Skipped {
        dir_name,
        reason: format!("namespace {namespace} is not under {site_base}"),
      });
      continue;
    };
    let (asset_class, slug, version) = split_iri_path(rest).ok_or_else(|| {
      format!(
        "{}: namespace {namespace} is under {site_base} but is not {site_base}<asset-class>/<slug>/<version>#",
        path.display()
      )
    })?;
    if !ASSET_CLASSES.contains(&asset_class) {
      return Err(format!(
        "{}: namespace {namespace}: '{asset_class}' is not a concept of the EU asset-classification table <{ASSET_CLASSIFICATION}>",
        path.display()
      ));
    }
    let declared = declared_asset_classes(&vocabularies_root.join(&dir_name).join("metadata.ttl"))?;
    if !declared.is_empty() && !declared.iter().any(|c| c == asset_class) {
      return Err(format!(
        "{}: namespace {namespace} is classified {asset_class} but metadata.ttl declares dcat:type {}",
        path.display(),
        declared.join(", ")
      ));
    }
    let version_info = literal(&triples, &namespace, OWL_VERSION_INFO);
    if version_info.map(|v| format!("v{v}")).as_deref() != Some(version) {
      return Err(format!(
        "{}: namespace {namespace} says version {version} but owl:versionInfo is {}",
        path.display(),
        version_info.unwrap_or("missing")
      ));
    }
    found.published.push(Vocabulary {
      dir_name,
      kind: asset_class.into(),
      slug: slug.into(),
      version: version.into(),
      namespace,
      triples,
      prefixes,
    });
  }
  Ok(found)
}

type PrefixTable = Vec<(String, String)>;

fn parse_turtle(path: &Path) -> Result<(Vec<Triple>, PrefixTable), String> {
  parse_turtle_with(path, TurtleParser::new())
}

fn parse_turtle_with(path: &Path, parser: TurtleParser) -> Result<(Vec<Triple>, PrefixTable), String> {
  let data = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
  let mut parser = parser.for_reader(data.as_slice());
  let mut triples = Vec::new();
  for t in parser.by_ref() {
    triples.push(t.map_err(|e| format!("{} is not valid Turtle: {e}", path.display()))?);
  }
  let prefixes = parser.prefixes().map(|(p, iri)| (p.to_string(), iri.to_string())).collect();
  Ok((triples, prefixes))
}

/// The one root resource: an `owl:Ontology`, or a `skos:ConceptScheme` (code
/// lists, thesauri).
fn ontology_iri(triples: &[Triple]) -> Result<String, String> {
  let mut iris: Vec<&str> = triples
    .iter()
    .filter_map(|t| match (&t.subject, &t.object) {
      (NamedOrBlankNode::NamedNode(s), Term::NamedNode(o))
        if t.predicate.as_str() == rdf_sitegen::RDF_TYPE && [OWL_ONTOLOGY, SKOS_CONCEPT_SCHEME].contains(&o.as_str()) =>
      {
        Some(s.as_str())
      }
      _ => None,
    })
    .collect();
  iris.sort_unstable();
  iris.dedup();
  match iris.as_slice() {
    [iri] => Ok(iri.to_string()),
    [] => Err("declares no owl:Ontology or skos:ConceptScheme".into()),
    [a, b, ..] => Err(format!("declares more than one owl:Ontology or skos:ConceptScheme ({a}, {b})")),
  }
}

/// `c_89b4bdb7/odrl-profile/v0.0.1#` -> `("c_89b4bdb7", "odrl-profile", "v0.0.1")`.
fn split_iri_path(rest: &str) -> Option<(&str, &str, &str)> {
  let mut segments = rest.strip_suffix('#')?.split('/');
  let (asset_class, slug, version) = (segments.next()?, segments.next()?, segments.next()?);
  let valid = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-._".contains(c));
  (segments.next().is_none() && [asset_class, slug, version].into_iter().all(valid)).then_some((asset_class, slug, version))
}

/// The asset-classification codes `metadata.ttl` gives as `dcat:type`, if the
/// file exists (upstream ships one per vocabulary).
fn declared_asset_classes(metadata: &Path) -> Result<Vec<String>, String> {
  if !metadata.is_file() {
    return Ok(Vec::new());
  }
  // metadata.ttl describes itself as `<>`, a relative IRI; only its dcat:type
  // objects are read, so the base is just something to resolve against.
  let base = format!("file://{}", metadata.canonicalize().unwrap_or_else(|_| metadata.to_path_buf()).display());
  let parser = TurtleParser::new().with_base_iri(base).map_err(|e| format!("{}: {e}", metadata.display()))?;
  let (triples, _) = parse_turtle_with(metadata, parser)?;
  Ok(
    triples
      .iter()
      .filter(|t| t.predicate.as_str() == DCAT_TYPE)
      .filter_map(|t| match &t.object {
        Term::NamedNode(o) => o.as_str().strip_prefix(ASSET_CLASSIFICATION).map(str::to_string),
        _ => None,
      })
      .collect(),
  )
}

fn literal<'a>(triples: &'a [Triple], subject: &str, predicate: &str) -> Option<&'a str> {
  triples.iter().find_map(|t| match (&t.subject, &t.object) {
    (NamedOrBlankNode::NamedNode(s), Term::Literal(l)) if s.as_str() == subject && t.predicate.as_str() == predicate => Some(l.value()),
    _ => None,
  })
}
