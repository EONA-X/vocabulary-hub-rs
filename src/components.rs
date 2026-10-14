//! A small, local set of Yew components used to render a vocabulary page —
//! no external design-system dependency. [`crate::html`] composes these;
//! [`crate::config::HubConfig::stylesheets`] is what styles them, so the
//! CSS-class contract here (`section-head`, `badge badge--<kind>`,
//! `annotation__label`/`annotation__text`, `icons-footer`, …) is load-bearing
//! — `tests/site.rs` holds it fixed.

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

/// The RDF/OWL/SKOS role a term's `rdf:type` plays, driving [`OntoBadge`]'s styling.
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
  fn modifier(self) -> &'static str {
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

/// What an [`OntoBadge`] is coloured by.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum BadgeVariant {
  Kind(TermKind),
}

impl BadgeVariant {
  fn modifier(self) -> &'static str {
    match self {
      BadgeVariant::Kind(kind) => kind.modifier(),
    }
  }
}

#[derive(Properties, PartialEq)]
pub struct OntoBadgeProps {
  pub label: AttrValue,
  pub variant: BadgeVariant,
}

/// A small pill labelling a term's `rdf:type`, e.g. `owl:Class`.
#[function_component]
pub fn OntoBadge(props: &OntoBadgeProps) -> Html {
  let class = format!("badge badge--{}", props.variant.modifier());
  html! { <span {class}>{ props.label.to_string() }</span> }
}

#[derive(Properties, PartialEq)]
pub struct SectionHeadProps {
  pub number: AttrValue,
  pub kicker: AttrValue,
  pub title: AttrValue,
}

/// A numbered section heading, e.g. `01 · Ontology · Example Ontology`.
#[function_component]
pub fn SectionHead(props: &SectionHeadProps) -> Html {
  html! {
    <div class="section-head">
      <span class="section-head__number">{ props.number.to_string() }</span>
      <span class="section-head__kicker">{ props.kicker.to_string() }</span>
      <h2 class="section-head__title">{ props.title.to_string() }</h2>
    </div>
  }
}

#[derive(Properties, PartialEq)]
pub struct OntoAnnotationProps {
  pub label: AttrValue,
  pub values: Vec<LiteralValue>,
}

/// A description-list entry: one `<dt>` label and one `<dd>` per value, each
/// value's language tag (if any) shown alongside its text.
#[function_component]
pub fn OntoAnnotation(props: &OntoAnnotationProps) -> Html {
  html! {
    <>
      <dt class="annotation__label">{ props.label.to_string() }</dt>
      { for props.values.iter().map(|v| html! {
          <dd class="annotation__value">
            <span class="annotation__text">{ v.value.clone() }</span>
            if let Some(lang) = &v.lang {
              <span class="annotation__lang">{ lang.clone() }</span>
            }
          </dd>
      }) }
    </>
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

#[derive(Properties, PartialEq)]
pub struct SimpleFooterProps {
  pub text: AttrValue,
  pub link: NavLink,
}

/// The page's closing footer: a line of text and one link home.
#[function_component]
pub fn SimpleFooter(props: &SimpleFooterProps) -> Html {
  html! {
    <footer class="icons-footer">
      <p class="icons-footer__text">{ props.text.to_string() }</p>
      <a class="icons-footer__link" href={props.link.href.clone()}>{ props.link.label.to_string() }</a>
    </footer>
  }
}
