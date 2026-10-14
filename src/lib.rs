//! A static-site generator for a vocabulary hub.
//!
//! Given a checkout of versioned RDF vocabularies, a [`discoverer::Discoverer`]
//! decides which `<slug>/<version>` directories to publish ([`default_discovery::DefaultDiscoverer`]
//! ships a standalone, single-host default); [`site::write_site`] then
//! renders each as Turtle, JSON-LD and a styled HTML page plus the
//! `.htaccess` that content-negotiates between them, and [`catalog::write_catalog`]
//! builds a DCAT catalog of the whole collection. Branding, hosting bases
//! and the catalog's own identity are configured through [`config::HubConfig`];
//! the caller (see `main.rs`) owns both a `HubConfig` and a `Discoverer` and
//! threads them through explicitly.

pub mod catalog;
mod components;
pub mod config;
pub mod default_discovery;
pub mod discoverer;
mod htaccess;
mod html;
pub mod model;
pub mod site;
pub mod theme;
mod turtle;
