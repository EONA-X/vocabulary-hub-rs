//! This crate's own default theme: three plain stylesheets a
//! [`crate::config::HubConfig`] can ship as-is, or a deployment can swap for
//! its own by building a `HubConfig` with different `stylesheets`.
//!
//! The content here is a placeholder — just enough valid CSS to render a
//! readable page — for a later theming pass to replace.

/// Design tokens (colours, type, spacing) the other two stylesheets build on.
pub const TOKENS_CSS: &str = include_str!("tokens.css");
/// Generic page and component styles (header, footer, badges).
pub const COMPONENTS_CSS: &str = include_str!("components.css");
/// Styles specific to a rendered vocabulary page (terms, annotations).
pub const ONTOLOGY_CSS: &str = include_str!("ontology.css");
