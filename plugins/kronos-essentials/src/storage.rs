use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use crate::models::{
    normalize_key, EssentialsConfig, EssentialsState, EssentialsStore, HomePoint,
    LegacyEssentialsStore, LocationPoint, SharedHomeAccess, WarpPoint,
};

pub const STORE_FILE_NAME: &str = "essentials_store.json";
pub const CONFIG_FILE_NAME: &str = "config.json";
pub const RANKS_BRIDGE_FILE: &str = "kronos_ranks_bridge.json";

static STATE: OnceLock<Mutex<EssentialsState>> = OnceLock::new();

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn with_state<R>(f: impl FnOnce(&mut EssentialsState) -> R) -> R {
    let mutex = STATE.get_or_init(|| {
        Mutex::new(EssentialsState::new(
            ".".to_string(),
            EssentialsConfig::default(),
            EssentialsStore::default(),
        ))
    });
    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    f(&mut guard)
}

pub fn init_storage(data_folder: String) {
    let _ = fs::create_dir_all(&data_folder);
    let config = load_or_create_config(&data_folder);
    let mut store = load_store(&data_folder);
    sync_ranks_bridge(&data_folder, &mut store);

    let seed = now_ms() ^ 0x9E37_79B9_7F4A_7C15;

    let mutex = STATE.get_or_init(|| {
        let mut state = EssentialsState::new(data_folder.clone(), config.clone(), store.clone());
        state.prng_seed = seed;
        Mutex::new(state)
    });

    let mut guard = mutex.lock().unwrap_or_else(|e| e.into_inner());
    guard.data_folder = data_folder;
    guard.config = config;
    guard.store = store;
    guard.prng_seed = seed;
}

#[derive(Debug, Deserialize)]
struct RanksBridgeEntry {
    #[serde(default)]
    rank: String,
    #[serde(default)]
    secondary_rank: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RanksBridgeSnapshot {
    #[serde(default)]
    players: std::collections::HashMap<String, RanksBridgeEntry>,
}

pub fn sync_ranks_bridge(data_folder: &str, store: &mut EssentialsStore) {
    let path = PathBuf::from(data_folder).join(RANKS_BRIDGE_FILE);
    let Ok(raw) = fs::read_to_string(&path) else {
        return;
    };
    let Ok(snapshot) = serde_json::from_str::<RanksBridgeSnapshot>(&raw) else {
        return;
    };
    for (player_id, entry) in snapshot.players {
        let key = normalize_key(&player_id);
        let p = store.players.entry(key).or_default();
        if !entry.rank.is_empty() {
            p.rank = entry.rank;
        }
        if entry.secondary_rank.is_some() {
            p.secondary_rank = entry.secondary_rank;
        }
    }
}

pub fn load_or_create_config(data_folder: &str) -> EssentialsConfig {
    let path = PathBuf::from(data_folder).join(CONFIG_FILE_NAME);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<EssentialsConfig>(&raw) {
            return parsed;
        }
    }
    let cfg = EssentialsConfig::default();
    if let Ok(json) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(&path, json);
    }
    cfg
}

pub fn load_store(data_folder: &str) -> EssentialsStore {
    let path = PathBuf::from(data_folder).join(STORE_FILE_NAME);
    if let Ok(raw) = fs::read_to_string(&path) {
        if let Ok(parsed) = parse_store_json(&raw) {
            return parsed;
        }
    }
    EssentialsStore {
        spawn: Some(LocationPoint::default_spawn()),
        ..EssentialsStore::default()
    }
}

pub fn parse_store_json(raw: &str) -> Result<EssentialsStore, String> {
    if let Ok(store) = serde_json::from_str::<EssentialsStore>(raw) {
        // Distinguish v2 format from legacy format when "players" is empty but "homes" exists
        if !store.players.is_empty() || !raw.contains("\"homes\"") {
            return Ok(store);
        }
    }
    if let Ok(legacy) = serde_json::from_str::<LegacyEssentialsStore>(raw) {
        return Ok(EssentialsStore::from(legacy));
    }
    serde_json::from_str::<EssentialsStore>(raw).map_err(|e| e.to_string())
}

pub fn save_store_atomic(data_folder: &str, store: &EssentialsStore) -> Result<(), String> {
    let _ = fs::create_dir_all(data_folder);
    let target = PathBuf::from(data_folder).join(STORE_FILE_NAME);
    let tmp = PathBuf::from(data_folder).join(format!("{STORE_FILE_NAME}.tmp"));
    let json = serde_json::to_string_pretty(store).map_err(|e| e.to_string())?;
    fs::write(&tmp, json).map_err(|e| e.to_string())?;
    if fs::rename(&tmp, &target).is_err() {
        fs::copy(&tmp, &target).map_err(|e| e.to_string())?;
        let _ = fs::remove_file(&tmp);
    }
    Ok(())
}

pub fn save_current_store() {
    let snapshot = with_state(|st| (st.data_folder.clone(), st.store.clone()));
    if let Err(err) = save_store_atomic(&snapshot.0, &snapshot.1) {
        tracing::warn!("Failed to save essentials_store.json: {err}");
    }
}

// =============================================================================
// Domain Operations on EssentialsStore (pure, unit-testable, no WIT calls)
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetHomeOutcome {
    Created { name: String, total: usize },
    Updated { name: String, total: usize },
    LimitReached { max_homes: usize },
}

pub fn upsert_home(
    store: &mut EssentialsStore,
    player_name: &str,
    home_name: &str,
    location: LocationPoint,
    max_homes: usize,
    unlimited: bool,
    now_ms: u64,
) -> SetHomeOutcome {
    let p_key = normalize_key(player_name);
    let data = store.players.entry(p_key).or_default();

    if let Some(existing) = data
        .homes
        .iter_mut()
        .find(|h| h.name.eq_ignore_ascii_case(home_name))
    {
        existing.location = location;
        return SetHomeOutcome::Updated {
            name: existing.name.clone(),
            total: data.homes.len(),
        };
    }

    if !unlimited && data.homes.len() >= max_homes {
        return SetHomeOutcome::LimitReached { max_homes };
    }

    let is_first = data.homes.is_empty();
    data.homes.push(HomePoint {
        name: home_name.to_string(),
        location,
        icon: if is_first {
            "★".to_string()
        } else {
            "⌂".to_string()
        },
        is_favorite: is_first,
        shared_with: Vec::new(),
        uses: 0,
        created_at_ms: now_ms,
    });

    SetHomeOutcome::Created {
        name: home_name.to_string(),
        total: data.homes.len(),
    }
}

pub fn delete_home(store: &mut EssentialsStore, player_name: &str, home_name: &str) -> bool {
    let p_key = normalize_key(player_name);
    if let Some(data) = store.players.get_mut(&p_key) {
        let before = data.homes.len();
        data.homes
            .retain(|h| !h.name.eq_ignore_ascii_case(home_name));
        return data.homes.len() < before;
    }
    false
}

pub fn toggle_favorite_home(
    store: &mut EssentialsStore,
    player_name: &str,
    home_name: &str,
) -> Option<bool> {
    let p_key = normalize_key(player_name);
    let data = store.players.get_mut(&p_key)?;
    let target_idx = data
        .homes
        .iter()
        .position(|h| h.name.eq_ignore_ascii_case(home_name))?;

    let new_state = !data.homes[target_idx].is_favorite;
    for (idx, home) in data.homes.iter_mut().enumerate() {
        home.is_favorite = idx == target_idx && new_state;
    }
    Some(new_state)
}

pub fn resolve_own_home(
    store: &mut EssentialsStore,
    player_name: &str,
    home_name: Option<&str>,
) -> Option<HomePoint> {
    let p_key = normalize_key(player_name);
    let data = store.players.get_mut(&p_key)?;

    let found = if let Some(requested) = home_name {
        data.homes
            .iter_mut()
            .find(|h| h.name.eq_ignore_ascii_case(requested))
    } else if let Some(pos) = data.homes.iter().position(|h| h.is_favorite) {
        data.homes.get_mut(pos)
    } else {
        data.homes.first_mut()
    }?;

    found.uses = found.uses.saturating_add(1);
    Some(found.clone())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShareOutcome {
    Shared { home: String, friend: String },
    AlreadyShared { home: String, friend: String },
    HomeNotFound,
    CannotShareWithSelf,
    LimitReached { max_shared: usize },
}

pub fn share_home(
    store: &mut EssentialsStore,
    owner_name: &str,
    home_name: &str,
    friend_name: &str,
    max_shared: usize,
) -> ShareOutcome {
    let owner_key = normalize_key(owner_name);
    let friend_key = normalize_key(friend_name);

    if owner_key == friend_key {
        return ShareOutcome::CannotShareWithSelf;
    }

    let Some(data) = store.players.get_mut(&owner_key) else {
        return ShareOutcome::HomeNotFound;
    };

    let Some(home) = data
        .homes
        .iter_mut()
        .find(|h| h.name.eq_ignore_ascii_case(home_name))
    else {
        return ShareOutcome::HomeNotFound;
    };

    if home
        .shared_with
        .iter()
        .any(|f| f.eq_ignore_ascii_case(friend_name))
    {
        return ShareOutcome::AlreadyShared {
            home: home.name.clone(),
            friend: friend_name.to_string(),
        };
    }

    if home.shared_with.len() >= max_shared {
        return ShareOutcome::LimitReached { max_shared };
    }

    home.shared_with.push(friend_name.trim().to_string());
    ShareOutcome::Shared {
        home: home.name.clone(),
        friend: friend_name.trim().to_string(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnshareOutcome {
    Unshared { home: String, friend: String },
    HomeNotFound,
    FriendNotShared,
}

pub fn unshare_home(
    store: &mut EssentialsStore,
    owner_name: &str,
    home_name: &str,
    friend_name: &str,
) -> UnshareOutcome {
    let owner_key = normalize_key(owner_name);
    let Some(data) = store.players.get_mut(&owner_key) else {
        return UnshareOutcome::HomeNotFound;
    };

    let Some(home) = data
        .homes
        .iter_mut()
        .find(|h| h.name.eq_ignore_ascii_case(home_name))
    else {
        return UnshareOutcome::HomeNotFound;
    };

    let before = home.shared_with.len();
    home.shared_with
        .retain(|f| !f.eq_ignore_ascii_case(friend_name));

    if home.shared_with.len() < before {
        UnshareOutcome::Unshared {
            home: home.name.clone(),
            friend: friend_name.trim().to_string(),
        }
    } else {
        UnshareOutcome::FriendNotShared
    }
}

pub fn resolve_shared_home(
    store: &mut EssentialsStore,
    requester_name: &str,
    owner_name: &str,
    home_name: &str,
) -> Option<HomePoint> {
    let owner_key = normalize_key(owner_name);
    let data = store.players.get_mut(&owner_key)?;
    let home = data
        .homes
        .iter_mut()
        .find(|h| h.name.eq_ignore_ascii_case(home_name))?;

    let is_owner = owner_key == normalize_key(requester_name);
    let is_shared = home
        .shared_with
        .iter()
        .any(|f| f.eq_ignore_ascii_case(requester_name));

    if !is_owner && !is_shared {
        return None;
    }

    home.uses = home.uses.saturating_add(1);
    Some(home.clone())
}

pub fn list_shared_homes_for(store: &EssentialsStore, friend_name: &str) -> Vec<SharedHomeAccess> {
    let mut results = Vec::new();
    for (owner, data) in &store.players {
        if owner.eq_ignore_ascii_case(friend_name) {
            continue;
        }
        for home in &data.homes {
            if home
                .shared_with
                .iter()
                .any(|f| f.eq_ignore_ascii_case(friend_name))
            {
                results.push(SharedHomeAccess {
                    owner: owner.clone(),
                    home: home.clone(),
                });
            }
        }
    }
    results.sort_by(|a, b| {
        a.owner
            .cmp(&b.owner)
            .then_with(|| a.home.name.cmp(&b.home.name))
    });
    results
}

pub fn list_player_homes(store: &EssentialsStore, player_name: &str) -> Vec<HomePoint> {
    let mut homes = store
        .players
        .get(&normalize_key(player_name))
        .map(|p| p.homes.clone())
        .unwrap_or_default();
    homes.sort_by(|a, b| {
        b.is_favorite
            .cmp(&a.is_favorite)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    homes
}

pub fn set_last_location(store: &mut EssentialsStore, player_name: &str, loc: LocationPoint) {
    let entry = store.players.entry(normalize_key(player_name)).or_default();
    entry.last_location = Some(loc);
}

pub fn set_death_location(store: &mut EssentialsStore, player_name: &str, loc: LocationPoint) {
    let entry = store.players.entry(normalize_key(player_name)).or_default();
    entry.death_location = Some(loc.clone());
    entry.last_location = Some(loc);
}

pub fn upsert_warp(
    store: &mut EssentialsStore,
    name: &str,
    location: LocationPoint,
    description: String,
    created_by: String,
) -> WarpPoint {
    let key = normalize_key(name);
    let warp = WarpPoint {
        name: name.to_string(),
        location,
        description,
        created_by,
        uses: 0,
    };
    store.warps.insert(key, warp.clone());
    warp
}

pub fn resolve_warp(store: &mut EssentialsStore, name: &str) -> Option<WarpPoint> {
    let warp = store.warps.get_mut(&normalize_key(name))?;
    warp.uses = warp.uses.saturating_add(1);
    Some(warp.clone())
}

pub fn delete_warp(store: &mut EssentialsStore, name: &str) -> bool {
    store.warps.remove(&normalize_key(name)).is_some()
}

pub fn list_warps(store: &EssentialsStore) -> Vec<WarpPoint> {
    let mut warps: Vec<WarpPoint> = store.warps.values().cloned().collect();
    warps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    warps
}
