//! What tells [`crate::site::write_site`] and [`crate::catalog::write_catalog`]
//! which `<slug>/<version>` directories are published, and from which files.
//!
//! A [`crate::config::HubConfig`] stays pure data: it does not hold a
//! `Discoverer`. The caller (`main.rs`, or any other entry point) owns a
//! concrete `Discoverer` value and passes it in explicitly, so a deployment
//! can plug in its own publication rules — a different directory layout, a
//! multi-host split, a gate on some upstream CI check — without forking this
//! crate. [`crate::default_discovery::DefaultDiscoverer`] is a ready-to-use
//! default.

use std::path::Path;

use crate::model::Discovery;

pub trait Discoverer {
  /// Walks `dir` and classifies every vocabulary version it finds, publishing
  /// those it accepts under `default_base` (or another base of its own
  /// choosing).
  fn discover(&self, dir: &Path, default_base: &str) -> Result<Discovery, String>;

  /// Every `<slug>/<version>` directory under `dir`, as `"<slug>/<version>"`.
  fn version_dirs(&self, dir: &Path) -> Result<Vec<String>, String>;

  /// The graph file name a version directory publishes, e.g. `"ontology.ttl"`.
  fn graph_file(&self, dir: &Path) -> Result<String, String>;
}
