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
