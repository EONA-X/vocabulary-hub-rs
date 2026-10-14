//! The human-readable representation of one vocabulary version, rendered
//! with this crate's own small set of Yew components ([`crate::components`]).
//!
//! The page is a Yew component rendered server-side to a static file, styled
//! by the stylesheets a [`crate::config::HubConfig`] carries — [`crate::site::write_site`]
//! ships them next to it under `assets/`. One `<article id="{local name}">`
//! per term under the namespace, so the hash IRI a person follows
//! (`…/v0.0.1#GenericClaim`) scrolls to that term. No script: the host serves
//! flat files and nothing else.

use std::collections::BTreeMap;
use std::rc::Rc;

use rdf_sitegen::{Indices, Ontology, RDF_TYPE, RDFS_COMMENT, RDFS_LABEL, UNTAGGED, subjects_in_namespace};
use yew::{AttrValue, Html, Properties, function_component, html};

use crate::components::{BadgeVariant, LiteralValue, NavLink, OntoAnnotation, OntoBadge, SectionHead, SimpleFooter, TermKind};
use crate::config::HubConfig;
use crate::model::Vocabulary;

const DCT_TITLE: &str = "http://purl.org/dc/terms/title";
const DCT_DESCRIPTION: &str = "http://purl.org/dc/terms/description";
const OWL: &str = "http://www.w3.org/2002/07/owl#";
const SKOS: &str = "http://www.w3.org/2004/02/skos/core#";

/// Annotations shown under a term, in reading order.
const TERM_NOTES: [(&str, &str); 6] = [
  ("http://www.w3.org/2004/02/skos/core#definition", "Definition"),
  (RDFS_COMMENT, "Comment"),
  (DCT_DESCRIPTION, "Description"),
  ("http://www.w3.org/2004/02/skos/core#note", "Note"),
  ("http://www.w3.org/2004/02/skos/core#scopeNote", "Scope note"),
  ("http://www.w3.org/2004/02/skos/core#example", "Example"),
];

/// Layout only — colours, type and components come from the configured
/// stylesheets.
const PAGE_CSS: &str = "
body{margin:0}
.voc-header{background:var(--vh-navy, #0b1f3a);color:#fff}
.voc-header__inner{display:flex;align-items:center;gap:.75rem;max-width:56rem;margin:0 auto;padding:.9rem 16px}
.voc-header a{display:inline-flex;color:inherit}
.voc-header svg{width:38px;height:36px}
.voc-header__title{font-weight:600}
.voc-main{max-width:56rem;margin:0 auto;padding:2rem 16px 3rem}
.voc-main code{overflow-wrap:anywhere}
.voc-formats{margin:1rem 0 2rem}
.voc-toc{columns:2 14rem;padding-left:1.1rem;margin-bottom:2rem}
.voc-term{border-top:1px solid var(--vh-border-subtle, #d8dde3);padding:1.25rem 0;scroll-margin-top:1rem}
.voc-term h3{margin:0 0 .35rem}
.voc-term__id{display:flex;flex-wrap:wrap;align-items:center;gap:.4rem;margin:0 0 .75rem}
.voc-term dl{margin:0}
.voc-members{margin:.25rem 0 0;padding-left:1.1rem}
";

pub fn render(v: &Vocabulary, config: &HubConfig) -> String {
  let page = Rc::new(PageData::build(v, config));
  let runtime = tokio::runtime::Builder::new_current_thread()
    .build()
    .expect("a single-thread runtime for Yew's server renderer");
  let body = runtime.block_on(
    yew::LocalServerRenderer::<VocabularyPage>::with_props(PageProps { page: page.clone() })
      .hydratable(false)
      .render(),
  );
  let stylesheets: String = config
    .stylesheets
    .iter()
    .map(|(file, _)| format!("<link rel=\"stylesheet\" href=\"assets/{file}\">\n"))
    .collect();
  format!(
    r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} {version}</title>
<link rel="canonical" href="{canonical}">
<link rel="alternate" type="text/turtle" href="{version}.ttl">
<link rel="alternate" type="application/ld+json" href="{version}.jsonld">
{stylesheets}<style>{PAGE_CSS}</style>
</head>
<body>
{body}
</body>
</html>
"#,
    title = esc(&page.title),
    version = esc(&page.version),
    canonical = esc(page.namespace.trim_end_matches('#')),
  )
}

/// Everything the page shows, extracted from the graph and the config up
/// front so the component is a pure function of its props.
#[derive(PartialEq)]
struct PageData {
  title: String,
  asset_type: String,
  version: String,
  namespace: String,
  overview: Vec<(String, Vec<LiteralValue>)>,
  terms: Vec<TermData>,
  brand_name: String,
  brand_link: String,
  brand_link_label: String,
  brand_mark_svg: &'static str,
  footer_text: String,
}

#[derive(PartialEq)]
struct TermData {
  anchor: String,
  label: String,
  curie: String,
  types: Vec<(String, TermKind)>,
  notes: Vec<(&'static str, Vec<LiteralValue>)>,
  members: Vec<(String, String)>,
}

impl PageData {
  fn build(v: &Vocabulary, config: &HubConfig) -> PageData {
    let idx = Indices::build(&Ontology { triples: v.triples.clone() });
    let ns = v.namespace.as_str();
    let plain = |s: &str| vec![LiteralValue::new(s.to_string(), None, None)];

    let mut overview = vec![
      ("Version", plain(text(&idx, ns, &format!("{OWL}versionInfo")).unwrap_or(&v.version))),
      ("Namespace", plain(ns)),
    ];
    for (pred, heading) in [(DCT_DESCRIPTION, "Description"), (RDFS_COMMENT, "Comment")] {
      let values = literals(&idx, ns, pred);
      if !values.is_empty() {
        overview.push((heading, values));
      }
    }

    let terms = subjects_in_namespace(&v.triples, ns)
      .into_iter()
      .filter(|t| t != ns)
      .map(|t| {
        let local = t[ns.len()..].to_string();
        TermData {
          label: label(&idx, &t).unwrap_or(&local).to_string(),
          curie: curie(v, &t),
          types: idx.links(&t, RDF_TYPE).iter().map(|ty| (curie(v, ty), term_kind(ty))).collect(),
          notes: TERM_NOTES
            .iter()
            .filter_map(|(pred, heading)| {
              let values = literals(&idx, &t, pred);
              (!values.is_empty()).then_some((*heading, values))
            })
            .collect(),
          members: idx.links(&t, &format!("{SKOS}member")).iter().map(|m| (href(v, m), curie(v, m))).collect(),
          anchor: local,
        }
      })
      .collect();

    PageData {
      title: text(&idx, ns, DCT_TITLE).or_else(|| label(&idx, ns)).unwrap_or(&v.slug).to_string(),
      asset_type: v.kind.clone(),
      version: v.version.clone(),
      namespace: v.namespace.clone(),
      overview: overview.into_iter().map(|(h, vals)| (h.to_string(), vals)).collect(),
      terms,
      brand_name: config.brand_name.clone(),
      brand_link: config.brand_link.clone(),
      brand_link_label: config.brand_link_label.clone(),
      brand_mark_svg: config.brand_mark_svg,
      footer_text: config.footer_text.clone(),
    }
  }
}

#[derive(Properties, PartialEq)]
struct PageProps {
  page: Rc<PageData>,
}

#[function_component]
fn VocabularyPage(props: &PageProps) -> Html {
  let p = &props.page;
  let logo = Html::from_html_unchecked(AttrValue::from(p.brand_mark_svg));
  let term_count = format!("{} terms", p.terms.len());
  html! {
    <>
      <header class="voc-header">
        <div class="voc-header__inner">
          <a href={p.brand_link.clone()} aria-label={p.brand_name.clone()}>{ logo }</a>
          <span class="voc-header__title">{ "Vocabulary Hub" }</span>
        </div>
      </header>
      <main class="voc-main">
        <SectionHead number="01" kicker={AttrValue::from(p.asset_type.clone())} title={AttrValue::from(p.title.clone())} />
        <dl>
          { for p.overview.iter().map(|(heading, values)| html! {
              <OntoAnnotation label={AttrValue::from(heading.clone())} values={values.clone()} />
          }) }
        </dl>
        <p class="voc-formats">
          { "Also available as " }
          <a href={format!("{}.ttl", p.version)}>{ "Turtle" }</a>
          { " · " }
          <a href={format!("{}.jsonld", p.version)}>{ "JSON-LD" }</a>
        </p>
        if !p.terms.is_empty() {
          <SectionHead number="02" kicker="Terms" title={AttrValue::from(term_count)} />
          <ul class="voc-toc">
            { for p.terms.iter().map(|t| html! { <li><a href={format!("#{}", t.anchor)}>{ &t.label }</a></li> }) }
          </ul>
          { for p.terms.iter().map(term) }
        }
      </main>
      <SimpleFooter text={AttrValue::from(p.footer_text.clone())} link={NavLink::new(p.brand_link.clone(), p.brand_link_label.clone())} />
    </>
  }
}

fn term(t: &TermData) -> Html {
  html! {
    <article id={t.anchor.clone()} class="voc-term">
      <h3>{ &t.label }</h3>
      <p class="voc-term__id">
        <code>{ &t.curie }</code>
        { for t.types.iter().map(|(label, kind)| html! {
            <OntoBadge label={AttrValue::from(label.clone())} variant={BadgeVariant::Kind(*kind)} />
        }) }
      </p>
      <dl>
        { for t.notes.iter().map(|(heading, values)| html! {
            <OntoAnnotation label={*heading} values={values.clone()} />
        }) }
      </dl>
      if !t.members.is_empty() {
        <ul class="voc-members">
          { for t.members.iter().map(|(href, curie)| html! { <li><a href={href.clone()}><code>{ curie }</code></a></li> }) }
        </ul>
      }
    </article>
  }
}

/// The badge hue for an `rdf:type`.
fn term_kind(iri: &str) -> TermKind {
  match iri.strip_prefix(OWL) {
    Some("Ontology") => TermKind::Ontology,
    Some("Class") => TermKind::Class,
    Some("ObjectProperty") => TermKind::ObjectProperty,
    Some("DatatypeProperty") => TermKind::DatatypeProperty,
    Some("AnnotationProperty") => TermKind::AnnotationProperty,
    Some("NamedIndividual") => TermKind::NamedIndividual,
    _ if iri == "http://www.w3.org/2000/01/rdf-schema#Class" => TermKind::Class,
    _ if iri == "http://www.w3.org/1999/02/22-rdf-syntax-ns#Property" => TermKind::Property,
    _ => TermKind::Other,
  }
}

/// Every language's value of `predicate` on `subject`, English first, then the
/// untagged one, then the others alphabetically.
fn literals(idx: &Indices, subject: &str, predicate: &str) -> Vec<LiteralValue> {
  let Some(by_lang) = idx.literal_all_langs(subject, predicate) else {
    return Vec::new();
  };
  let sorted: BTreeMap<(u8, &str), &str> = by_lang
    .iter()
    .map(|(lang, value)| {
      let rank = match lang.as_str() {
        "en" => 0,
        UNTAGGED => 1,
        _ => 2,
      };
      ((rank, lang.as_str()), value.as_str())
    })
    .collect();
  sorted
    .into_iter()
    .map(|((_, lang), value)| LiteralValue::new(value.to_string(), (!lang.is_empty()).then(|| lang.to_string()), None))
    .collect()
}

/// English, else untagged — the corpus is English-first today.
fn text<'a>(idx: &'a Indices, subject: &str, predicate: &str) -> Option<&'a str> {
  idx.literal(subject, predicate, "en").or_else(|| idx.literal(subject, predicate, UNTAGGED))
}

fn label<'a>(idx: &'a Indices, subject: &str) -> Option<&'a str> {
  text(idx, subject, RDFS_LABEL).or_else(|| text(idx, subject, &format!("{SKOS}prefLabel")))
}

/// In-page anchor for a term of this vocabulary, the IRI itself otherwise.
fn href(v: &Vocabulary, iri: &str) -> String {
  match iri.strip_prefix(v.namespace.as_str()) {
    Some(local) if !local.is_empty() => format!("#{local}"),
    _ => iri.to_string(),
  }
}

/// Shortest CURIE the source's own prefix table allows, else the full IRI.
fn curie(v: &Vocabulary, iri: &str) -> String {
  v.prefixes
    .iter()
    .filter_map(|(p, ns)| iri.strip_prefix(ns.as_str()).map(|local| (ns.len(), format!("{p}:{local}"))))
    .max_by_key(|(len, _)| *len)
    .map_or_else(|| iri.to_string(), |(_, c)| c)
}

/// For the hand-written `<head>` only; the body is escaped by Yew.
fn esc(s: &str) -> String {
  s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}
