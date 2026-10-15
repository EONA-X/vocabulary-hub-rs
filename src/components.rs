//! What [`crate::site::write_site`] renders a vocabulary page's markup with.
//!
//! A [`crate::config::HubConfig`] stays pure data: it does not hold a
//! `Components`. The caller (`main.rs`, or any other entry point) owns a
//! concrete `Components` value and passes it in explicitly, so a deployment
//! can plug in its own markup — a different design system, a different
//! HTML structure, a different rendering framework entirely — without
//! forking this crate. [`crate::default_components::DefaultComponents`] is a
//! ready-to-use default.
//!
//! [`LiteralValue`], [`TermKind`], [`BadgeVariant`] and [`NavLink`] are the
//! plain data the trait's methods take; they carry no markup of their own.

use yew::prelude::*;

/// One literal value of an RDF annotation: its text, language tag and
/// datatype IRI (only the text is required).
#[derive(Clone, PartialEq, Debug)]
pub struct LiteralValue {
  pub value: String,
  pub lang: Option<String>,
  pub datatype: Option<String>,
}

impl LiteralValue {
  pub fn new(value: String, lang: Option<String>, datatype: Option<String>) -> Self {
    LiteralValue { value, lang, datatype }
  }
}

/// The RDF/OWL/SKOS role a term's `rdf:type` plays, driving an `onto_badge`'s styling.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TermKind {
  Ontology,
  Class,
  ObjectProperty,
  DatatypeProperty,
  AnnotationProperty,
  NamedIndividual,
  Property,
  Other,
}

impl TermKind {
  pub(crate) fn modifier(self) -> &'static str {
    match self {
      TermKind::Ontology => "ontology",
      TermKind::Class => "class",
      TermKind::ObjectProperty => "object-property",
      TermKind::DatatypeProperty => "datatype-property",
      TermKind::AnnotationProperty => "annotation-property",
      TermKind::NamedIndividual => "named-individual",
      TermKind::Property => "property",
      TermKind::Other => "default",
    }
  }
}

/// What an `onto_badge` is coloured by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BadgeVariant {
  Kind(TermKind),
}

impl BadgeVariant {
  pub(crate) fn modifier(self) -> &'static str {
    match self {
      BadgeVariant::Kind(kind) => kind.modifier(),
    }
  }
}

/// A link, e.g. the footer's link back to the home page.
#[derive(Clone, PartialEq, Debug)]
pub struct NavLink {
  pub href: AttrValue,
  pub label: AttrValue,
}

impl NavLink {
  pub fn new(href: impl Into<AttrValue>, label: impl Into<AttrValue>) -> Self {
    NavLink {
      href: href.into(),
      label: label.into(),
    }
  }
}

/// The markup units a vocabulary page is built from. [`crate::html::render`]
/// calls each method once per occurrence, up front, and embeds the
/// resulting [`Html`] in the page it renders — a deployment implementing its
/// own `Components` controls every pixel without forking this crate.
pub trait Components {
  /// A numbered section heading, e.g. `01 · Ontology · Example Ontology`.
  fn section_head(&self, number: AttrValue, kicker: AttrValue, title: AttrValue) -> Html;

  /// A small pill labelling a term's `rdf:type`, e.g. `owl:Class`.
  fn onto_badge(&self, label: AttrValue, variant: BadgeVariant) -> Html;

  /// A description-list entry: one `<dt>` label and one `<dd>` per value, each
  /// value's language tag (if any) shown alongside its text.
  fn onto_annotation(&self, label: AttrValue, values: Vec<LiteralValue>) -> Html;

  /// The page's closing footer: a line of text and one link home.
  fn simple_footer(&self, text: AttrValue, link: NavLink) -> Html;
}
