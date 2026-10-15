//! A standalone, host-agnostic default [`Components`]: a small, local set of
//! Yew components rendering this crate's own markup — no external
//! design-system dependency. [`crate::config::HubConfig::stylesheets`] is
//! what styles them, so the CSS-class contract here (`section-head`, `badge
//! badge--<kind>`, `annotation__label`/`annotation__text`, `icons-footer`,
//! …) is load-bearing — `tests/site.rs` holds it fixed.

use yew::prelude::*;

use crate::components::{BadgeVariant, Components, LiteralValue, NavLink};

/// The simple, local-markup [`Components`] this crate ships to be useful out
/// of the box.
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultComponents;

impl Components for DefaultComponents {
  fn section_head(&self, number: AttrValue, kicker: AttrValue, title: AttrValue) -> Html {
    html! { <SectionHead {number} {kicker} {title} /> }
  }

  fn onto_badge(&self, label: AttrValue, variant: BadgeVariant) -> Html {
    html! { <OntoBadge {label} {variant} /> }
  }

  fn onto_annotation(&self, label: AttrValue, values: Vec<LiteralValue>) -> Html {
    html! { <OntoAnnotation {label} {values} /> }
  }

  fn simple_footer(&self, text: AttrValue, link: NavLink) -> Html {
    html! { <SimpleFooter {text} {link} /> }
  }
}

#[derive(Properties, PartialEq)]
struct OntoBadgeProps {
  label: AttrValue,
  variant: BadgeVariant,
}

/// A small pill labelling a term's `rdf:type`, e.g. `owl:Class`.
#[function_component]
fn OntoBadge(props: &OntoBadgeProps) -> Html {
  let class = format!("badge badge--{}", props.variant.modifier());
  html! { <span {class}>{ props.label.to_string() }</span> }
}

#[derive(Properties, PartialEq)]
struct SectionHeadProps {
  number: AttrValue,
  kicker: AttrValue,
  title: AttrValue,
}

/// A numbered section heading, e.g. `01 · Ontology · Example Ontology`.
#[function_component]
fn SectionHead(props: &SectionHeadProps) -> Html {
  html! {
    <div class="section-head">
      <span class="section-head__number">{ props.number.to_string() }</span>
      <span class="section-head__kicker">{ props.kicker.to_string() }</span>
      <h2 class="section-head__title">{ props.title.to_string() }</h2>
    </div>
  }
}

#[derive(Properties, PartialEq)]
struct OntoAnnotationProps {
  label: AttrValue,
  values: Vec<LiteralValue>,
}

/// A description-list entry: one `<dt>` label and one `<dd>` per value, each
/// value's language tag (if any) shown alongside its text.
#[function_component]
fn OntoAnnotation(props: &OntoAnnotationProps) -> Html {
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

#[derive(Properties, PartialEq)]
struct SimpleFooterProps {
  text: AttrValue,
  link: NavLink,
}

/// The page's closing footer: a line of text and one link home.
#[function_component]
fn SimpleFooter(props: &SimpleFooterProps) -> Html {
  html! {
    <footer class="icons-footer">
      <p class="icons-footer__text">{ props.text.to_string() }</p>
      <a class="icons-footer__link" href={props.link.href.clone()}>{ props.link.label.to_string() }</a>
    </footer>
  }
}
