//! The human-readable representation of one vocabulary version.
//!
//! One `<section id="{local name}">` per term under the namespace, so the hash
//! IRI a person follows (`…/v0.0.1#GenericClaim`) scrolls to that term. The
//! page is self-contained (inline CSS, no script) because the host serves
//! flat files and nothing else.

use std::fmt::Write;

use rdf_sitegen::{Indices, Ontology, RDF_TYPE, RDFS_COMMENT, RDFS_LABEL, UNTAGGED, subjects_in_namespace};

use crate::Vocabulary;

const DCT_TITLE: &str = "http://purl.org/dc/terms/title";
const DCT_DESCRIPTION: &str = "http://purl.org/dc/terms/description";
const OWL_VERSION_INFO: &str = "http://www.w3.org/2002/07/owl#versionInfo";
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

pub fn render(v: &Vocabulary) -> String {
  let idx = Indices::build(&Ontology { triples: v.triples.clone() });
  let ns = v.namespace.as_str();
  let title = text(&idx, ns, DCT_TITLE).or_else(|| text(&idx, ns, RDFS_LABEL)).unwrap_or(&v.slug);

  let mut h = String::new();
  let _ = write!(
    h,
    r#"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} {version}</title>
<link rel="alternate" type="text/turtle" href="{version}.ttl">
<link rel="alternate" type="application/ld+json" href="{version}.jsonld">
<style>{CSS}</style>
</head>
<body>
<main>
<header>
<h1>{title}</h1>
<p class="meta">Version <strong>{version_info}</strong> · Namespace <code>{ns_e}</code></p>
<p class="meta">Also available as <a href="{version}.ttl">Turtle</a> · <a href="{version}.jsonld">JSON-LD</a></p>
</header>
"#,
    title = esc(title),
    version = esc(&v.version),
    version_info = esc(text(&idx, ns, OWL_VERSION_INFO).unwrap_or(&v.version)),
    ns_e = esc(ns),
  );
  for pred in [DCT_DESCRIPTION, RDFS_COMMENT] {
    if let Some(t) = text(&idx, ns, pred) {
      let _ = writeln!(h, "<p>{}</p>", esc(t));
    }
  }

  let terms: Vec<String> = subjects_in_namespace(&v.triples, ns).into_iter().filter(|t| t != ns).collect();
  if !terms.is_empty() {
    h.push_str("<nav><h2>Terms</h2><ul>\n");
    for t in &terms {
      let local = &t[ns.len()..];
      let _ = writeln!(
        h,
        r##"<li><a href="#{a}">{l}</a></li>"##,
        a = esc(local),
        l = esc(label(&idx, t).unwrap_or(local))
      );
    }
    h.push_str("</ul></nav>\n");
  }

  for t in &terms {
    let local = &t[ns.len()..];
    let _ = writeln!(h, r#"<section id="{a}">"#, a = esc(local));
    let _ = writeln!(h, "<h2>{}</h2>", esc(label(&idx, t).unwrap_or(local)));
    let _ = writeln!(h, r#"<p class="meta"><code>{}</code></p>"#, esc(&curie(v, t)));
    let types = idx.links(t, RDF_TYPE);
    if !types.is_empty() {
      let _ = writeln!(
        h,
        r#"<p class="meta">Type: {}</p>"#,
        types.iter().map(|ty| link(v, ty)).collect::<Vec<_>>().join(", ")
      );
    }
    h.push_str("<dl>\n");
    for (pred, heading) in TERM_NOTES {
      if let Some(note) = text(&idx, t, pred) {
        let _ = writeln!(h, "<dt>{heading}</dt><dd>{}</dd>", esc(note));
      }
    }
    let members = idx.links(t, &format!("{SKOS}member"));
    if !members.is_empty() {
      let _ = writeln!(
        h,
        "<dt>Members</dt><dd><ul>{}</ul></dd>",
        members.iter().map(|m| format!("<li>{}</li>", link(v, m))).collect::<String>()
      );
    }
    h.push_str("</dl>\n</section>\n");
  }

  h.push_str("</main>\n</body>\n</html>\n");
  h
}

/// English, else untagged — the corpus is English-first today.
fn text<'a>(idx: &'a Indices, subject: &str, predicate: &str) -> Option<&'a str> {
  idx.literal(subject, predicate, "en").or_else(|| idx.literal(subject, predicate, UNTAGGED))
}

fn label<'a>(idx: &'a Indices, subject: &str) -> Option<&'a str> {
  text(idx, subject, RDFS_LABEL).or_else(|| text(idx, subject, &format!("{SKOS}prefLabel")))
}

/// In-page anchor for a term of this vocabulary, absolute link otherwise;
/// shown as a CURIE either way.
fn link(v: &Vocabulary, iri: &str) -> String {
  let href = match iri.strip_prefix(v.namespace.as_str()) {
    Some(local) if !local.is_empty() => format!("#{local}"),
    _ => iri.to_string(),
  };
  format!(r#"<a href="{}"><code>{}</code></a>"#, esc(&href), esc(&curie(v, iri)))
}

/// Shortest CURIE the source's own prefix table allows, else the full IRI.
fn curie(v: &Vocabulary, iri: &str) -> String {
  v.prefixes
    .iter()
    .filter_map(|(p, ns)| iri.strip_prefix(ns.as_str()).map(|local| (ns.len(), format!("{p}:{local}"))))
    .max_by_key(|(len, _)| *len)
    .map_or_else(|| iri.to_string(), |(_, c)| c)
}

fn esc(s: &str) -> String {
  s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

const CSS: &str = r#"
:root{--bg:#fff;--fg:#1b1f24;--muted:#57606a;--line:#d0d7de;--accent:#0b5cad;--code:#f3f5f7}
@media (prefers-color-scheme:dark){:root{--bg:#0f1419;--fg:#e6e8eb;--muted:#9aa4ae;--line:#2d333b;--accent:#6cb6ff;--code:#1c2128}}
*{box-sizing:border-box}
body{margin:0;background:var(--bg);color:var(--fg);font:16px/1.55 system-ui,-apple-system,"Segoe UI",sans-serif}
main{max-width:52rem;margin:0 auto;padding:2rem 16px 4rem}
h1{font-size:1.9rem;margin:0 0 .5rem}
h2{font-size:1.2rem;margin:0 0 .25rem}
a{color:var(--accent)}
code{background:var(--code);padding:.05rem .3rem;border-radius:4px;font-size:.9em;overflow-wrap:anywhere}
.meta{color:var(--muted);margin:.25rem 0}
header{border-bottom:1px solid var(--line);padding-bottom:1rem;margin-bottom:1rem}
nav ul{columns:2 14rem;padding-left:1.1rem}
section{border-top:1px solid var(--line);padding:1.25rem 0;scroll-margin-top:1rem}
section:target{background:linear-gradient(90deg,var(--code),transparent)}
dt{font-weight:600;margin-top:.6rem}
dd{margin:0 0 0 1rem;white-space:pre-line}
dd ul{margin:0;padding-left:1.1rem}
"#;
