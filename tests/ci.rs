//! Repo-config guards for the build profiles in Cargo.toml.

use std::fs;

fn section(name: &str) -> String {
  let cargo = fs::read_to_string("Cargo.toml").unwrap();
  let header = format!("[profile.{name}]");
  let start = cargo.find(&header).unwrap_or_else(|| panic!("missing {header}"));
  let rest = &cargo[start + header.len()..];
  let end = rest.find("\n[").unwrap_or(rest.len());
  rest[..end].to_string()
}

#[test]
fn the_release_profile_strips_symbols() {
  let release = section("release");
  assert!(release.contains("strip = \"symbols\""), "{release}");
}

#[test]
fn the_dist_profile_inherits_release_with_lto_and_abort() {
  let dist = section("dist");
  for needle in ["inherits = \"release\"", "lto = \"fat\"", "codegen-units = 1", "panic = \"abort\""] {
    assert!(dist.contains(needle), "dist lacks {needle}");
  }
  assert!(!section("release").contains("lto"), "release must not pay LTO link time");
}

fn gitlab_ci() -> String {
  fs::read_to_string(".gitlab-ci.yml").unwrap()
}

#[test]
fn gitlab_ci_disables_incremental_and_full_debuginfo_for_the_cached_target() {
  let ci = gitlab_ci();
  let vars: String = ci
    .lines()
    .skip_while(|l| *l != "variables:")
    .skip(1)
    .take_while(|l| l.starts_with(' '))
    .collect::<Vec<_>>()
    .join("\n");
  for needle in ["CARGO_INCREMENTAL: \"0\"", "CARGO_PROFILE_DEV_DEBUG: \"line-tables-only\""] {
    assert!(vars.contains(needle), "variables lack {needle}:\n{vars}");
  }
}

#[test]
fn gitlab_ci_jobs_do_not_share_one_target_cache_key() {
  assert!(gitlab_ci().contains("prefix: $CI_JOB_NAME"));
}

fn cargo_git_url() -> String {
  let cargo = fs::read_to_string("Cargo.toml").unwrap();
  let rest = &cargo[cargo.find("git = \"").expect("git dependency") + 7..];
  rest[..rest.find('"').unwrap()].to_string()
}

#[test]
fn deny_toml_restricts_sources_to_crates_io_and_the_pinned_rdf_sitegen_repo() {
  let deny = fs::read_to_string("deny.toml").unwrap();
  for needle in [
    "[advisories]",
    "[licenses]",
    "[bans]",
    "[sources]",
    "unknown-registry = \"deny\"",
    "unknown-git = \"deny\"",
  ] {
    assert!(deny.contains(needle), "deny.toml lacks {needle}");
  }
  let allow_git = format!("allow-git = [\"{}\"]", cargo_git_url());
  assert!(deny.contains(&allow_git), "deny.toml lacks {allow_git}");
}

#[test]
fn gitlab_ci_runs_cargo_deny() {
  let ci = gitlab_ci();
  assert!(ci.contains("cargo deny --locked check") || ci.contains("cargo deny check"), "no cargo deny job");
  let rules: String = ci
    .lines()
    .skip_while(|l| *l != "workflow:")
    .take_while(|l| *l != "stages:")
    .collect::<Vec<_>>()
    .join("\n");
  assert!(rules.contains("$CI_PIPELINE_SOURCE == \"schedule\""), "workflow rules lack schedule:\n{rules}");
}

fn workflows() -> Vec<(String, String)> {
  let mut out = Vec::new();
  for entry in fs::read_dir(".github/workflows").unwrap() {
    let path = entry.unwrap().path();
    if path.extension().is_some_and(|e| e == "yml" || e == "yaml") {
      out.push((path.display().to_string(), fs::read_to_string(&path).unwrap()));
    }
  }
  assert!(!out.is_empty(), "no workflows found");
  out
}

fn is_full_sha_pin(uses: &str) -> bool {
  let Some((_, after)) = uses.rsplit_once('@') else { return false };
  let sha: String = after.chars().take(40).collect();
  let tail = &after[sha.len()..];
  sha.len() == 40
    && sha.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f'))
    && (tail.is_empty() || tail.starts_with(char::is_whitespace) || tail.starts_with('#'))
}

#[test]
fn every_github_action_is_pinned_to_a_full_commit_sha() {
  for (name, text) in workflows() {
    for line in text.lines() {
      let t = line.trim();
      let t = t.strip_prefix("- ").unwrap_or(t);
      if t.starts_with("uses:") {
        assert!(is_full_sha_pin(t), "{name}: not pinned to a commit SHA: {t}");
      }
    }
  }
}

#[test]
fn the_pages_build_uses_the_lockfile() {
  let pages = fs::read_to_string(".github/workflows/pages.yml").unwrap();
  let line = pages.lines().find(|l| l.contains("cargo run")).expect("cargo run line");
  assert!(line.contains("--locked"), "{line}");
}

#[test]
fn the_toolchain_is_pinned() {
  let tc = fs::read_to_string("rust-toolchain.toml").unwrap();
  let channel = tc
    .lines()
    .find_map(|l| l.trim().strip_prefix("channel = \""))
    .and_then(|r| r.strip_suffix('"'))
    .expect("channel");
  let parts: Vec<&str> = channel.split('.').collect();
  assert!(
    parts.len() == 3 && parts[0] == "1" && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit())),
    "channel must be an exact 1.x.y version: {channel}"
  );
  assert!(tc.contains("components = [\"rustfmt\", \"clippy\"]"), "{tc}");
}

#[test]
fn dependabot_updates_the_pinned_actions() {
  let d = fs::read_to_string(".github/dependabot.yml").unwrap();
  assert!(d.contains("package-ecosystem: \"github-actions\""), "{d}");
}
