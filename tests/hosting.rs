//! The asset types of the IRI convention are listed in two places: the build
//! and the router installed in each environment's web root. The hosting files
//! are hand-installed, so they must not drift from what the build publishes.

use std::fs;

use vocabulary_hub_rs::model::{ASSET_TYPES, asset_type_alternation};

#[test]
fn the_alternation_lists_every_asset_type_in_order() {
  let names: Vec<&str> = ASSET_TYPES.iter().map(|(name, _)| *name).collect();
  assert_eq!(names, ["ontology", "shape", "crosswalk", "vocabulary", "codelist"]);
  assert_eq!(asset_type_alternation(), "(ontology|shape|crosswalk|vocabulary|codelist)");
}

#[test]
fn no_eu_concept_is_covered_by_two_asset_types() {
  let mut codes: Vec<&str> = ASSET_TYPES.iter().flat_map(|(_, codes)| codes.iter().copied()).collect();
  let all = codes.len();
  codes.sort_unstable();
  codes.dedup();
  assert_eq!(codes.len(), all);
}

#[test]
fn the_router_routes_exactly_the_asset_types() {
  let router = fs::read_to_string("hosting/router.htaccess").unwrap();
  assert!(
    router.contains(&asset_type_alternation()),
    "hosting/router.htaccess must route {}",
    asset_type_alternation()
  );
}

#[test]
fn the_dev_overlay_leaves_the_canonical_host_of_a_vocabulary_to_its_slug() {
  // A path no longer tells the host: vendored standards are on a distinct
  // host under the same asset types. The overlay only gives the default
  // (the Hub UI's host); each slug's .htaccess names its own. This file is a
  // worked example (hosting/examples/): {{VENDORED_HOST}} is the
  // instantiation's own substitution point, not a literal host.
  let dev = fs::read_to_string("hosting/examples/dev/.htaccess").unwrap();
  assert!(!dev.contains(&asset_type_alternation()), "{dev}");
  assert!(dev.contains("E=CANONICAL_HOST:{{VENDORED_HOST}}"), "{dev}");
  assert!(!dev.contains("E=CANONICAL_HOST:{{SITE_BASE_HOST}}"), "{dev}");
}

#[test]
fn the_router_adds_the_trailing_slash_with_an_explicit_https_url() {
  // A relative target lets Apache build the URL, and behind OVH's TLS front it
  // adds ":443" (observed on dev-voc.eona-x.eu).
  let router = fs::read_to_string("hosting/router.htaccess").unwrap();
  assert!(
    router.contains(&format!(
      "RewriteRule ^({}/[^/]+)$ https://%{{HTTP_HOST}}/$1/ [R=301,L]",
      asset_type_alternation()
    )),
    "{router}"
  );
}

#[test]
fn every_hosting_path_the_scripts_reference_exists() {
  let mut missing = Vec::new();
  for dir in ["scripts", "scripts/examples"] {
    for entry in fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      if path.extension().is_none_or(|ext| ext != "sh") {
        continue;
      }
      for line in fs::read_to_string(&path).unwrap().lines() {
        let mut rest = line;
        while let Some(at) = rest.find("hosting/") {
          let tail = &rest[at..];
          let end = tail.find(|c: char| !(c.is_ascii_alphanumeric() || "_./-".contains(c))).unwrap_or(tail.len());
          let referenced = tail[..end].trim_end_matches(['.', ',']);
          if !std::path::Path::new(referenced).exists() {
            missing.push(format!("{}: {referenced}", path.display()));
          }
          rest = &tail[end..];
        }
      }
    }
  }
  assert!(missing.is_empty(), "scripts reference hosting files that do not exist: {missing:?}");
}

#[test]
fn the_deploy_script_refuses_a_tilde_target_that_lftp_expands_to_the_account_root() {
  use std::os::unix::fs::PermissionsExt;
  use std::process::Command;

  // A stub lftp on PATH: if the guard lets the value through, it runs.
  let bin = std::env::temp_dir().join(format!("vocab-hub-stub-lftp-{}", std::process::id()));
  fs::create_dir_all(&bin).unwrap();
  let stub = bin.join("lftp");
  fs::write(&stub, "#!/bin/sh\ncat >/dev/null\nexit 0\n").unwrap();
  fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
  let path = format!("{}:{}", bin.display(), std::env::var("PATH").unwrap());

  for dir in ["~", "~/", "~/voc", "~user"] {
    let status = Command::new("sh")
      .args(["scripts/deploy-sftp.sh", "public", "mirror"])
      .env("PATH", &path)
      .env("OVH_SFTP_HOST", "h")
      .env("OVH_SFTP_USER", "u")
      .env("OVH_SFTP_PASSWORD", "p")
      .env("OVH_SSH_KNOWN_HOSTS", "k")
      .env("OVH_SFTP_DIR", dir)
      .status()
      .unwrap();
    assert_eq!(status.code(), Some(2), "OVH_SFTP_DIR='{dir}' must be refused");
  }
  fs::remove_dir_all(&bin).ok();
}

#[test]
fn check_conneg_reports_a_fail_line_for_every_check_when_the_host_is_unreachable() {
  use std::process::Command;

  // Port 1 refuses the connection: curl fails at the transport level.
  let output = Command::new("sh")
    .args(["scripts/check-conneg.sh", "http://127.0.0.1:1/", "ontology/x", "v1", "https://x/"])
    .output()
    .unwrap();
  let stdout = String::from_utf8_lossy(&output.stdout);
  assert_eq!(output.status.code(), Some(1), "stdout: {stdout}");
  assert!(
    stdout.lines().filter(|l| l.starts_with("FAIL ")).count() >= 5,
    "every check must report: {stdout}"
  );
}
