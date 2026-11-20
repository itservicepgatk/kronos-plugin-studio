use crate::hierarchy::{effective_perks, normalize_key, primary_rank};
use crate::models::{BridgeRankSnapshot, PlayerRankProfile, RanksConfig, RanksState, RanksStore};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CONFIG_JSON: &str = include_str!("../data/config.json");
const STORE_FILENAME: &str = "kronos_ranks.json";
const LEGACY_STORE_FILENAME: &str = "ranks_store.json";
const BRIDGE_FILENAME: &str = "kronos_ranks_bridge.json";

static STATE: OnceLock<Mutex<RanksState>> = OnceLock::new();

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn default_config() -> RanksConfig {
    serde_json::from_str(DEFAULT_CONFIG_JSON).expect("embedded kronos-ranks config.json must be valid")
}

pub fn with_state<R>(f: impl FnOnce(&mut RanksState) -> R) -> R {
    let lock = STATE.get_or_init(|| {
        Mutex::new(RanksState::new(
            ".".to_string(),
            default_config(),
            RanksStore::default(),
            now_ms(),
        ))
    });
    let mut guard = match lock.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };
    f(&mut guard)
}

pub fn init_storage(data_folder: String) {
    let folder = PathBuf::from(&data_folder);
    let _ = fs::create_dir_all(&folder);

    let config_path = folder.join("config.json");
    let config = load_or_create_config(&config_path);

    let store_path = folder.join(STORE_FILENAME);
    let legacy_path = folder.join(LEGACY_STORE_FILENAME);
    let store = load_store(&store_path, &legacy_path);

    let _ = export_bridge_snapshot(&data_folder, &config, &store);

    let now = now_ms();
    with_state(|st| {
        st.data_folder = data_folder;
        st.config = config;
        st.store = store;
        st.dirty = false;
        st.last_autosave_ms = now;
    });
}

fn load_or_create_config(path: &Path) -> RanksConfig {
    if let Ok(raw) = fs::read_to_string(path) {
        if let Ok(parsed) = serde_json::from_str::<RanksConfig>(&raw) {
            if !parsed.ranks.is_empty() {
                return parsed;
            }
        }
    }
    let _ = atomic_write(path, DEFAULT_CONFIG_JSON);
    default_config()
}

fn load_store(primary_path: &Path, legacy_path: &Path) -> RanksStore {
    for candidate in [primary_path, legacy_path] {
        if let Ok(raw) = fs::read_to_string(candidate) {
            if let Ok(parsed) = serde_json::from_str::<RanksStore>(&raw) {
                return parsed;
            }
        }
    }
    RanksStore {
        next_app_id: 1,
        next_audit_id: 1,
        ..Default::default()
    }
}

pub fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
    }
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, content).map_err(|e| e.to_string())?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::write(path, content);
        let _ = fs::remove_file(&tmp_path);
        e.to_string()
    })
}

/// Persists `kronos_ranks.json` and refreshes `kronos_ranks_bridge.json` without holding the lock during I/O.
pub fn save_current_store() {
    let (data_folder, config, store) = with_state(|st| {
        st.dirty = false;
        st.last_autosave_ms = now_ms();
        (st.data_folder.clone(), st.config.clone(), st.store.clone())
    });

    if let Err(err) = persist_store_and_bridge(&data_folder, &config, &store) {
        tracing::warn!("KRONOS Ranks: не удалось сохранить хранилище: {}", err);
    }
}

pub fn persist_store_and_bridge(
    data_folder: &str,
    config: &RanksConfig,
    store: &RanksStore,
) -> Result<(), String> {
    let store_path = PathBuf::from(data_folder).join(STORE_FILENAME);
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    atomic_write(&store_path, &json)?;
    export_bridge_snapshot(data_folder, config, store)?;
    Ok(())
}

/// Builds the unified cross-plugin bridge map for `kronos-essentials`, `kronos-regions`, and `kronos-guard`.
pub fn build_bridge_map(
    config: &RanksConfig,
    store: &RanksStore,
) -> HashMap<String, BridgeRankSnapshot> {
    let mut bridge: HashMap<String, BridgeRankSnapshot> = HashMap::new();
    for (key, profile) in &store.profiles {
        let primary = primary_rank(config, profile);
        let perks = effective_perks(config, profile);
        bridge.insert(
            key.clone(),
            BridgeRankSnapshot {
                primary_rank: primary.id.clone(),
                donate_rank: profile.donate_rank.clone(),
                staff_rank: profile.staff_rank.clone(),
                department: primary.department.code().into(),
                weight: primary.weight,
                prefix: primary.prefix.clone(),
                max_homes: perks.max_homes,
                max_regions: perks.max_regions,
                max_region_volume: perks.max_region_volume,
            },
        );
    }
    bridge
}

/// Exports `kronos_ranks_bridge.json` inside `data_folder` (and mirrors to `config.bridge_export_path`
/// when accessible inside the WASI filesystem) so sibling KRONOS plugins always see up-to-date ranks.
pub fn export_bridge_snapshot(
    data_folder: &str,
    config: &RanksConfig,
    store: &RanksStore,
) -> Result<(), String> {
    let bridge = build_bridge_map(config, store);
    let json = serde_json::to_string_pretty(&bridge).map_err(|e| e.to_string())?;

    let primary_bridge_path = PathBuf::from(data_folder).join(BRIDGE_FILENAME);
    atomic_write(&primary_bridge_path, &json)?;

    let configured_path = PathBuf::from(&config.bridge_export_path);
    if configured_path != primary_bridge_path {
        let _ = atomic_write(&configured_path, &json);
    }

    // Also mirror into sibling `../kronos_ranks_bridge.json` when plugins share a `plugins/` root
    if let Some(parent) = Path::new(data_folder).parent() {
        let shared_path = parent.join(BRIDGE_FILENAME);
        if shared_path != primary_bridge_path && shared_path != configured_path {
            let _ = atomic_write(&shared_path, &json);
        }
    }

    Ok(())
}

pub fn ensure_profile<'a>(
    store: &'a mut RanksStore,
    player_name: &str,
    now: u64,
) -> &'a mut PlayerRankProfile {
    let key = normalize_key(player_name);
    let profile = store
        .profiles
        .entry(key)
        .or_insert_with(|| PlayerRankProfile::new(player_name, now));
    profile.player_name = player_name.to_string();
    profile
}

pub fn get_or_default_profile(store: &RanksStore, player_name: &str, now: u64) -> PlayerRankProfile {
    let key = normalize_key(player_name);
    store
        .profiles
        .get(&key)
        .cloned()
        .unwrap_or_else(|| PlayerRankProfile::new(player_name, now))
}
