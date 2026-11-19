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
}

pub fn default_config() -> RegionsConfig {
    serde_json::from_str::<RegionsConfig>(EMBEDDED_DEFAULT_CONFIG)
        .expect("embedded kronos-regions data/config.json must be valid")
}

/// Reads both `primary_rank` and `donate_rank` from `kronos-ranks` cross-plugin bridge.
pub fn resolve_ranks_from_bridge(player_name: &str) -> Option<(String, String)> {
    let raw = fs::read_to_string(RANKS_BRIDGE_PATH).ok()?;
    let map = serde_json::from_str::<HashMap<String, BridgeEntry>>(&raw).ok()?;
    let entry = map.get(&player_name.trim().to_lowercase())?;
    Some((entry.primary_rank.clone(), entry.donate_rank.clone()))
}

/// Batch-syncs primary and donate ranks for online players from `kronos_ranks_bridge.json`.
pub fn refresh_online_ranks_from_bridge(store: &mut RegionStore, online_ids: impl Iterator<Item = String>) {
    let Ok(raw) = fs::read_to_string(RANKS_BRIDGE_PATH) else {
        return;
    };
    let Ok(map) = serde_json::from_str::<HashMap<String, BridgeEntry>>(&raw) else {
        return;
    };
    for pid in online_ids {
        if let Some(name) = store.player_names.get(&pid).cloned() {
            if let Some(entry) = map.get(&name.trim().to_lowercase()) {
                store.player_ranks.insert(pid.clone(), entry.primary_rank.clone());
                store.player_donate_ranks.insert(pid, entry.donate_rank.clone());
            }
        }
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
