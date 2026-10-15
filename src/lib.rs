//! A static-site generator for a vocabulary hub.
//!
//! Given a checkout of versioned RDF vocabularies, a [`discoverer::Discoverer`]
//! decides which `<slug>/<version>` directories to publish ([`default_discovery::DefaultDiscoverer`]
//! ships a standalone, single-host default); [`site::write_site`] then
//! renders each, through a [`components::Components`] ([`default_components::DefaultComponents`]
//! ships a standalone default), as Turtle, JSON-LD and a styled HTML page
//! plus the `.htaccess` that content-negotiates between them, and
//! [`catalog::write_catalog`] builds a DCAT catalog of the whole collection.
//! Branding, hosting bases and the catalog's own identity are configured
//! through [`config::HubConfig`]; the caller (see `main.rs`) owns a
//! `HubConfig`, a `Discoverer` and a `Components` and threads them through
//! explicitly.

pub mod catalog;
pub mod components;
pub mod config;
pub mod default_components;
pub mod default_discovery;
pub mod discoverer;
mod htaccess;
mod html;
pub mod model;
pub mod site;
pub mod theme;
mod turtle;
