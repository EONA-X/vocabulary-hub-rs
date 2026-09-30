use std::fs;
use std::path::Path;

use oxrdf::{NamedOrBlankNode, Term, Triple};
use oxttl::TurtleParser;

const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const OWL_VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";

pub const ASSET_CLASSES: [&str; 0] = [];
pub const ASSET_CLASS_PATTERN: &str = "";

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
/// a vocabulary is published exactly when its `owl:Ontology` IRI is
/// `{site_base}{slug}/{version}#`, i.e. when upstream has frozen its namespace
/// on the site (as eona-vocabularies-reference#4 does for the ODRL profile).
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
    let (slug, version) = split_slug_version(rest).ok_or_else(|| {
      format!(
        "{}: namespace {namespace} is under {site_base} but is not {site_base}<slug>/<version>#",
        path.display()
      )
    })?;
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
      kind: String::new(),
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
  let data = fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
  let mut parser = TurtleParser::new().for_reader(data.as_slice());
  let mut triples = Vec::new();
  for t in parser.by_ref() {
    triples.push(t.map_err(|e| format!("{} is not valid Turtle: {e}", path.display()))?);
  }
  let prefixes = parser.prefixes().map(|(p, iri)| (p.to_string(), iri.to_string())).collect();
  Ok((triples, prefixes))
}

fn ontology_iri(triples: &[Triple]) -> Result<String, String> {
  let mut iris = triples.iter().filter_map(|t| match (&t.subject, &t.object) {
    (NamedOrBlankNode::NamedNode(s), Term::NamedNode(o)) if t.predicate.as_str() == rdf_sitegen::RDF_TYPE && o.as_str() == OWL_ONTOLOGY => Some(s.as_str()),
    _ => None,
  });
  match (iris.next(), iris.next()) {
    (Some(iri), None) => Ok(iri.to_string()),
    (None, _) => Err("declares no owl:Ontology".into()),
    (Some(a), Some(b)) => Err(format!("declares more than one owl:Ontology ({a}, {b})")),
  }
}

/// `odrl-profile/v0.0.1#` -> `("odrl-profile", "v0.0.1")`.
fn split_slug_version(rest: &str) -> Option<(&str, &str)> {
  let (slug, version) = rest.strip_suffix('#')?.split_once('/')?;
  let valid = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || "-._".contains(c));
  (valid(slug) && valid(version)).then_some((slug, version))
}

fn literal<'a>(triples: &'a [Triple], subject: &str, predicate: &str) -> Option<&'a str> {
  triples.iter().find_map(|t| match (&t.subject, &t.object) {
    (NamedOrBlankNode::NamedNode(s), Term::Literal(l)) if s.as_str() == subject && t.predicate.as_str() == predicate => Some(l.value()),
    _ => None,
  })
}
