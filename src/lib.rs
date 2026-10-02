//! Static publication of Eona-X vocabularies on `https://eona-x.eu/`.
//!
//! The hosting is a plain static file host fronted by `.htaccess` (see the ADR
//! "Sourcing content.vocabulary-hub.eona-x.eu's ontologies via a git submodule"),
//! so everything here happens at build time: [`discover`] — the source
//! repository's own rules, `eona-vocabularies-reference/tools/vocabulary-check`
//! — decides which vocabularies of the pinned checkout are publishable, and [`site::write_site`] emits, per vocabulary version, one
//! `.ttl`, one `.jsonld` and one `.html` file plus the `.htaccess` that picks
//! between them by `Accept` header.

pub use eona_vocabulary_check::{ASSET_TYPES, Discovery, SITE_BASE, Skipped, Vocabulary, asset_type_alternation, discover};

pub mod catalog;
mod htaccess;
mod html;
pub mod site;
