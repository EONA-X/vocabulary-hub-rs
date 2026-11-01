//! A standalone, host-agnostic default [`Discoverer`]: every `<slug>/<version>`
//! directory with an `ontology.ttl` or `alignment.ttl` graph is published
//! under the single `default_base` given — no host-splitting, no vendoring
//! rules. Its asset type comes from an optional `metadata.ttl`'s `dcat:type`
//! against [`ASSET_TYPES`]; absent that (or absent `metadata.ttl` itself) it
//! defaults to `"ontology"`. Symlinks inside the checkout (directories, graph
//! and metadata files) are ignored, never followed.

use std::fs;
use std::path::Path;

use oxrdf::{NamedOrBlankNode, Term, Triple};

use crate::discoverer::Discoverer;
use crate::model::{ASSET_TYPES, Discovery, Skipped, Vocabulary};
use crate::turtle::{is_regular_file, parse_turtle};

const DCAT_TYPE: &str = "http://www.w3.org/ns/dcat#type";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const OWL_ONTOLOGY: &str = "http://www.w3.org/2002/07/owl#Ontology";
const SKOS_CONCEPT_SCHEME: &str = "http://www.w3.org/2004/02/skos/core#ConceptScheme";

/// The graph file names a version directory may publish, in the order tried.
const GRAPH_FILES: [&str; 2] = ["ontology.ttl", "alignment.ttl"];

/// The simple, single-host [`Discoverer`] this crate ships to be useful out
/// of the box.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultDiscoverer;

impl Discoverer for DefaultDiscoverer {
  fn discover(&self, dir: &Path, default_base: &str) -> Result<Discovery, String> {
    // Bases are joined to `<kind>/...` by plain formatting: make sure exactly one `/` separates them.
    let default_base = &format!("{}/", default_base.trim_end_matches('/'));
    let mut discovery = Discovery::default();
    for dir_name in self.version_dirs(dir)? {
      let version_dir = dir.join(&dir_name);
      let graph_file = match self.graph_file(&version_dir) {
        Ok(f) => f,
        Err(reason) => {
          discovery.skipped.push(Skipped { dir_name, reason });
          continue;
        }
      };
      let Some((slug, version)) = dir_name.split_once('/') else {
        discovery.skipped.push(Skipped {
          dir_name,
          reason: "not <slug>/<version>".into(),
        });
        continue;
      };
      let kind = match kind(&version_dir, default_base) {
        Ok(kind) => kind.unwrap_or("ontology"),
        Err(reason) => {
          discovery.skipped.push(Skipped { dir_name, reason });
          continue;
        }
      };
      let parsed = parse_turtle(&version_dir.join(&graph_file), &format!("{default_base}{kind}/{dir_name}"));
      let (triples, prefixes) = match parsed {
        Ok(parsed) => parsed,
        Err(reason) => {
          discovery.skipped.push(Skipped { dir_name, reason });
          continue;
        }
      };
      // A crosswalk (`alignment.ttl`) describes links between other
      // vocabularies and usually has no root of its own, so it falls back to
      // its published document IRI as namespace.
      let namespace = match root(&triples) {
        Some(namespace) => namespace,
        None if graph_file == "alignment.ttl" => format!("{default_base}{kind}/{dir_name}#"),
        None => {
          discovery.skipped.push(Skipped {
            dir_name,
            reason: format!("no single {OWL_ONTOLOGY} or {SKOS_CONCEPT_SCHEME} root"),
          });
          continue;
        }
      };
      discovery.published.push(Vocabulary {
        base: default_base.to_string(),
        kind: kind.to_string(),
        slug: slug.to_string(),
        version: version.to_string(),
        dir_name,
        namespace,
        prefixes,
        triples,
      });
    }
    Ok(discovery)
  }

  fn version_dirs(&self, dir: &Path) -> Result<Vec<String>, String> {
    if !dir.is_dir() {
      return Err(format!("{}: not a directory", dir.display()));
    }
    let mut dirs = Vec::new();
    for slug in subdirs(dir)? {
      for version in subdirs(&dir.join(&slug))? {
        dirs.push(format!("{slug}/{version}"));
      }
    }
    dirs.sort();
    Ok(dirs)
  }

  fn graph_file(&self, dir: &Path) -> Result<String, String> {
    GRAPH_FILES
      .into_iter()
      .find(|name| is_regular_file(&dir.join(name)))
      .map(str::to_string)
      .ok_or_else(|| format!("{}: no ontology.ttl or alignment.ttl", dir.display()))
  }
}

/// The asset type its `metadata.ttl`'s `dcat:type` covers, if it has a
/// `metadata.ttl` and that predicate resolves to one of [`ASSET_TYPES`]. A
/// `metadata.ttl` that does not parse is an error, not a silent default.
fn kind(dir: &Path, default_base: &str) -> Result<Option<&'static str>, String> {
  let metadata = dir.join("metadata.ttl");
  match fs::symlink_metadata(&metadata) {
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
    // Any other outcome (including a symlink) goes to parse_turtle, which refuses links.
    _ => {}
  }
  let (meta, _) = parse_turtle(&metadata, default_base)?;
  let codes: Vec<&str> = meta
    .iter()
    .filter(|t| t.predicate.as_str() == DCAT_TYPE)
    .filter_map(|t| match &t.object {
      Term::NamedNode(n) => n.as_str().rsplit('/').next(),
      _ => None,
    })
    .collect();
  Ok(
    ASSET_TYPES
      .iter()
      .find(|(_, covered)| codes.iter().any(|c| covered.contains(c)))
      .map(|(name, _)| *name),
  )
}

/// Every directory name directly under `dir`, or none if `dir` itself does
/// not exist (callers check the vocabularies root exists first).
fn subdirs(dir: &Path) -> Result<Vec<String>, String> {
  let entries = match fs::read_dir(dir) {
    Ok(entries) => entries,
    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
    Err(e) => return Err(format!("{}: {e}", dir.display())),
  };
  let mut names = Vec::new();
  for entry in entries {
    let entry = entry.map_err(|e| format!("{}: {e}", dir.display()))?;
    let file_type = entry.file_type().map_err(|e| format!("{}: {e}", dir.display()))?;
    if file_type.is_dir() {
      names.push(entry.file_name().to_string_lossy().into_owned());
    }
  }
  Ok(names)
}

/// Whether `iri` lies under `other`: it extends it past a `#` or `/` boundary,
/// so `http://ex.org/core-ext` is not nested under `http://ex.org/core`.
pub(crate) fn is_nested_under(iri: &str, other: &str) -> bool {
  iri
    .strip_prefix(other)
    .is_some_and(|rest| other.ends_with(['#', '/']) || rest.starts_with(['#', '/']))
}

/// The `owl:Ontology` or `skos:ConceptScheme` subject not nested under
/// another, if there is exactly one.
fn root(triples: &[Triple]) -> Option<String> {
  let mut iris: Vec<&str> = triples
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
  iris.retain(|iri| !all.iter().any(|other| other != iri && is_nested_under(iri, other)));
  (iris.len() == 1).then(|| iris[0].to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn relative_iris_resolve_against_the_published_document_iri() {
    let src = tempfile::tempdir().unwrap();
    let version_dir = src.path().join("rel/v1");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
      version_dir.join("ontology.ttl"),
      "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#> a owl:Ontology .\n<#Thing> a owl:Class .\n",
    )
    .unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    let v = &discovery.published[0];
    assert_eq!(v.namespace, "https://hub.test/ontology/rel/v1#");
    assert!(v.triples.iter().any(|t| t.subject.to_string() == "<https://hub.test/ontology/rel/v1#Thing>"));
  }

  #[test]
  fn a_malformed_metadata_file_skips_the_version_instead_of_defaulting_to_ontology() {
    let src = tempfile::tempdir().unwrap();
    let version_dir = src.path().join("bad/v1");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
      version_dir.join("ontology.ttl"),
      "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<#> a owl:Ontology .\n",
    )
    .unwrap();
    fs::write(version_dir.join("metadata.ttl"), "<> a <").unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    assert!(discovery.published.is_empty());
    assert_eq!(discovery.skipped.len(), 1);
    assert_eq!(discovery.skipped[0].dir_name, "bad/v1");
    assert!(discovery.skipped[0].reason.contains("metadata.ttl"));
  }

  #[test]
  fn an_alignment_without_an_ontology_root_is_still_published() {
    let src = tempfile::tempdir().unwrap();
    let version_dir = src.path().join("xw/v0.1.0");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
      version_dir.join("alignment.ttl"),
      "@prefix void: <http://rdfs.org/ns/void#> .\n<https://example.org/x/a-b> a void:Linkset .\n",
    )
    .unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    assert!(discovery.skipped.is_empty(), "{:?}", discovery.skipped);
    assert_eq!(discovery.published.len(), 1);
    assert_eq!(discovery.published[0].namespace, "https://hub.test/ontology/xw/v0.1.0#");
  }

  #[test]
  fn two_roots_sharing_only_a_string_prefix_are_skipped_as_having_no_single_root() {
    let src = tempfile::tempdir().unwrap();
    let version_dir = src.path().join("core/v1");
    fs::create_dir_all(&version_dir).unwrap();
    fs::write(
      version_dir.join("ontology.ttl"),
      "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<http://ex.org/core> a owl:Ontology .\n<http://ex.org/core-ext> a owl:Ontology .\n",
    )
    .unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    assert!(discovery.published.is_empty(), "{:?}", discovery.published.len());
    assert_eq!(discovery.skipped.len(), 1);
    assert!(discovery.skipped[0].reason.contains("no single"));
  }

  #[cfg(unix)]
  const ROOT_TTL: &str = "@prefix owl: <http://www.w3.org/2002/07/owl#> .\n<https://ex.org/o#> a owl:Ontology .\n";

  #[cfg(unix)]
  #[test]
  fn a_symlinked_version_dir_is_not_discovered() {
    use std::os::unix::fs::symlink;
    let src = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::create_dir_all(outside.path().join("v1")).unwrap();
    fs::write(outside.path().join("v1/ontology.ttl"), ROOT_TTL).unwrap();
    fs::create_dir_all(src.path().join("evil")).unwrap();
    symlink(outside.path().join("v1"), src.path().join("evil/v1")).unwrap();
    symlink(outside.path(), src.path().join("evil2")).unwrap();

    let dirs = DefaultDiscoverer.version_dirs(src.path()).unwrap();

    assert!(!dirs.contains(&"evil/v1".to_string()), "{dirs:?}");
    assert!(!dirs.contains(&"evil2/v1".to_string()), "{dirs:?}");
    assert!(DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap().published.is_empty());
  }

  #[cfg(unix)]
  #[test]
  fn a_symlinked_graph_file_is_skipped() {
    let src = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("ontology.ttl"), ROOT_TTL).unwrap();
    fs::create_dir_all(src.path().join("onto/v1")).unwrap();
    std::os::unix::fs::symlink(outside.path().join("ontology.ttl"), src.path().join("onto/v1/ontology.ttl")).unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    assert!(discovery.published.is_empty());
    assert!(discovery.skipped.iter().any(|s| s.dir_name == "onto/v1"));
  }

  #[cfg(unix)]
  #[test]
  fn a_symlinked_metadata_file_skips_the_version() {
    let src = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("metadata.ttl"), "<> a <http://ex.org/X> .\n").unwrap();
    fs::create_dir_all(src.path().join("onto/v1")).unwrap();
    fs::write(src.path().join("onto/v1/ontology.ttl"), ROOT_TTL).unwrap();
    std::os::unix::fs::symlink(outside.path().join("metadata.ttl"), src.path().join("onto/v1/metadata.ttl")).unwrap();

    let discovery = DefaultDiscoverer.discover(src.path(), "https://hub.test/").unwrap();

    assert!(discovery.published.is_empty());
    assert!(discovery.skipped.iter().any(|s| s.dir_name == "onto/v1"));
  }
}
