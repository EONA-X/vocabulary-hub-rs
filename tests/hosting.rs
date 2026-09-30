//! The asset-class segment is an EU asset-classification concept code. The
//! build checks it against the table; the router installed in each
//! environment's web root and the dev overlay's canonical-host rule match it by
//! shape, and are hand-installed, so they must not drift from the build.

use std::fs;

use vocabulary_hub_build::{ASSET_CLASSES, ASSET_CLASS_PATTERN};

#[test]
fn every_asset_classification_code_has_the_shape_the_hosting_files_match() {
  let re = format!("^{ASSET_CLASS_PATTERN}$");
  for code in ASSET_CLASSES {
    assert!(
      code.len() == 10 && code.starts_with("c_") && code[2..].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
      "{code} does not match {re}"
    );
  }
}

#[test]
fn the_router_routes_asset_class_codes() {
  let router = fs::read_to_string("hosting/router.htaccess").unwrap();
  assert!(router.contains(ASSET_CLASS_PATTERN), "hosting/router.htaccess must route {ASSET_CLASS_PATTERN}");
}

#[test]
fn the_dev_overlay_gives_asset_class_paths_the_eona_x_eu_canonical_host() {
  let dev = fs::read_to_string("hosting/dev/.htaccess").unwrap();
  assert!(dev.contains(ASSET_CLASS_PATTERN), "hosting/dev/.htaccess must match {ASSET_CLASS_PATTERN}");
}
