use crate::models::{RegionStore, RegionsConfig};
use log::{info, warn};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data/kronos-regions";
const CONFIG_PATH: &str = "data/kronos-regions/config.json";
const STORE_PATH: &str = "data/kronos-regions/regions.json";
const STORE_TMP_PATH: &str = "data/kronos-regions/regions.json.tmp";
const RANKS_BRIDGE_PATH: &str = "data/kronos_ranks_bridge.json";
const EMBEDDED_DEFAULT_CONFIG: &str = include_str!("../data/config.json");

#[derive(Deserialize)]
struct BridgeEntry {
    primary_rank: String,
    donate_rank: String,
    max_regions: u32,
}

pub fn default_config() -> RegionsConfig {
    serde_json::from_str::<RegionsConfig>(EMBEDDED_DEFAULT_CONFIG)
        .expect("embedded kronos-regions data/config.json must be valid")
}

/// Resolves player's effective rank from `kronos-ranks` cross-plugin bridge (respecting dual-track donate+staff).
pub fn resolve_rank_from_bridge(config: &RegionsConfig, player_name: &str) -> Option<String> {
    let raw = fs::read_to_string(RANKS_BRIDGE_PATH).ok()?;
    let map = serde_json::from_str::<HashMap<String, BridgeEntry>>(&raw).ok()?;
    let entry = map.get(&player_name.trim().to_lowercase())?;
    let primary_pol = config.rank_for(&entry.primary_rank);
    let donate_pol = config.rank_for(&entry.donate_rank);
    if primary_pol.bypass_protection || primary_pol.max_regions >= entry.max_regions {
        Some(entry.primary_rank.clone())
    } else if donate_pol.max_regions > primary_pol.max_regions {
        Some(entry.donate_rank.clone())
    } else {
        Some(entry.primary_rank.clone())
    }
}

pub fn load_config() -> RegionsConfig {
    if Path::new(CONFIG_PATH).exists() {
        if let Some(cfg) = fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|s| serde_json::from_str::<RegionsConfig>(&s).ok())
        {
            info!("Loaded KRONOS Regions config from {}", CONFIG_PATH);
            return cfg;
        }
        warn!("Invalid regions config.json; using defaults");
    }
    let cfg = default_config();
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(CONFIG_PATH, pretty);
    }
    cfg
}

pub fn load_store() -> RegionStore {
    if Path::new(STORE_PATH).exists() {
        if let Ok(raw) = fs::read_to_string(STORE_PATH) {
            if let Ok(store) = serde_json::from_str::<RegionStore>(&raw) {
                info!("Loaded {} protected regions", store.regions.len());
                return store;
            }
        }
    }
    RegionStore::default()
}

pub fn save_store_atomic(store: &RegionStore) {
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(store) {
        if let Err(err) = fs::write(STORE_TMP_PATH, pretty) {
            warn!("Failed writing temp regions file: {}", err);
            return;
        }
        if let Err(err) = fs::rename(STORE_TMP_PATH, STORE_PATH) {
            warn!("Failed atomic rename {}: {}", STORE_PATH, err);
        }
    }
}
