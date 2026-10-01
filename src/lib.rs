//! Static publication of Eona-X vocabularies on `https://eona-x.eu/`.
//!
//! The hosting is a plain static file host fronted by `.htaccess` (see the ADR
//! "Sourcing content.vocabulary-hub.eona-x.eu's ontologies via a git submodule"),
//! so everything here happens at build time: [`discover`] decides which
//! vocabularies of the pinned `eona-vocabularies-reference` checkout are
//! publishable, and [`site::write_site`] emits, per vocabulary version, one
//! `.ttl`, one `.jsonld` and one `.html` file plus the `.htaccess` that picks
//! between them by `Accept` header.

pub mod discover;

pub use discover::{Discovery, Skipped, Vocabulary, discover};

/// Where the site is served.
pub const SITE_BASE: &str = "https://eona-x.eu/";

pub use discover::{ASSET_CLASS_PATTERN, ASSET_CLASSES, ASSET_TYPES, asset_type_alternation};

mod htaccess;
mod html;
pub mod site;
