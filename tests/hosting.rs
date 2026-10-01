//! The asset types of the IRI convention are listed in three places: the build,
//! the router installed in each environment's web root, and the dev overlay's
//! canonical-host rule. The hosting files are hand-installed, so they must not
//! drift from what the build publishes.

use std::fs;

use vocabulary_hub_build::{ASSET_TYPES, asset_type_alternation};

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
  assert!(router.contains(&asset_type_alternation()), "hosting/router.htaccess must route {}", asset_type_alternation());
}

#[test]
fn the_dev_overlay_gives_asset_type_paths_the_eona_x_eu_canonical_host() {
  let dev = fs::read_to_string("hosting/dev/.htaccess").unwrap();
  assert!(dev.contains(&asset_type_alternation()), "hosting/dev/.htaccess must match {}", asset_type_alternation());
}
