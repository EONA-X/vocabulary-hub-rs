//! What a deployment configures: branding, hosting bases, and the catalog's
//! own identity. Pure data — it does not hold a
//! [`Discoverer`](crate::discoverer::Discoverer); the caller owns one
//! alongside a `HubConfig` and threads both through [`crate::site::write_site`]
//! and [`crate::catalog::write_catalog`].

use crate::theme::default as theme;

/// Everything a Vocabulary Hub build needs that is not the vocabularies
/// themselves: where it publishes to, how its pages are branded, and the
/// identity of the DCAT catalog it mints.
#[derive(Debug, Clone, PartialEq)]
pub struct HubConfig {
  /// The default host tree vocabularies publish to, e.g.
  /// `https://vocabulary-hub.example/`, passed to the
  /// [`Discoverer`](crate::discoverer::Discoverer) as the base to publish
  /// under when nothing else overrides it.
  pub site_base: String,
  /// `(filename, CSS)` pairs linked by every vocabulary HTML page and
  /// copied under each slug's `assets/`.
  pub stylesheets: [(&'static str, &'static str); 3],
  /// Short org/site name used as the header mark's accessible label.
  pub brand_name: String,
  /// URL the header mark and footer link point to.
  pub brand_link: String,
  /// Visible text of the footer's home link.
  pub brand_link_label: String,
  /// Inline SVG markup for the header logo mark.
  pub brand_mark_svg: &'static str,
  /// Text shown in the page footer.
  pub footer_text: String,
  /// Base IRI minting the DCAT catalog and every asset/version IRI.
  pub catalog_iri: String,
  /// Base URL where the `docs/` catalog data tree is served; used in
  /// `landingPage`, `thumbnail` and `downloadURL` IRIs.
  pub docs_base: String,
  /// The DCAT catalog's `dct:title`.
  pub catalog_title: String,
  /// The DCAT catalog's `dct:description`.
  pub catalog_description: String,
}

/// A plain circular mark: just enough that the default header is not empty.
const DEFAULT_MARK_SVG: &str =
  r##"<svg viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg" aria-hidden="true"><circle cx="12" cy="12" r="10" fill="currentColor"/></svg>"##;

impl Default for HubConfig {
  /// `https://vocabulary-hub.example/` (RFC 2606 reserved) is this crate's
  /// own demo domain — safe to bake in as a generic, obviously-fake default.
  fn default() -> Self {
    let site_base = "https://vocabulary-hub.example/".to_string();
    HubConfig {
      stylesheets: [
        ("tokens.css", theme::TOKENS_CSS),
        ("components.css", theme::COMPONENTS_CSS),
        ("ontology.css", theme::ONTOLOGY_CSS),
      ],
      brand_name: "Vocabulary Hub".to_string(),
      brand_link: site_base.clone(),
      brand_link_label: "vocabulary-hub.example".to_string(),
      brand_mark_svg: DEFAULT_MARK_SVG,
      footer_text: "Vocabulary Hub".to_string(),
      catalog_iri: format!("{site_base}catalog"),
      docs_base: format!("{site_base}docs"),
      catalog_title: "Vocabulary Hub".to_string(),
      catalog_description: "A catalog of ontologies, shapes, vocabularies, code lists and crosswalks.".to_string(),
      site_base,
    }
  }
}
