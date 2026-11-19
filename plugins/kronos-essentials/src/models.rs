use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub const DEFAULT_COMBAT_TAG_SECONDS: u64 = 15;
pub const DEFAULT_TPA_TIMEOUT_SECONDS: u64 = 60;
pub const DEFAULT_HOME_NAME: &str = "home";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LocationPoint {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    #[serde(default)]
    pub yaw: f32,
    #[serde(default)]
    pub pitch: f32,
}

impl LocationPoint {
    pub fn rounded(world: String, x: f64, y: f64, z: f64, yaw: f32, pitch: f32) -> Self {
        Self {
            world,
            x: (x * 10.0).round() / 10.0,
            y: (y * 10.0).round() / 10.0,
            z: (z * 10.0).round() / 10.0,
            yaw,
            pitch,
        }
    }

    pub fn default_spawn() -> Self {
        Self {
            world: "minecraft:overworld".to_string(),
            x: 0.5,
            y: 65.0,
            z: 0.5,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HomePoint {
    pub name: String,
    pub location: LocationPoint,
    #[serde(default = "default_home_icon")]
    pub icon: String,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub shared_with: Vec<String>,
    #[serde(default)]
    pub uses: u64,
    #[serde(default)]
    pub created_at_ms: u64,
}

fn default_home_icon() -> String {
    "⌂".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarpPoint {
    pub name: String,
    pub location: LocationPoint,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub created_by: String,
    #[serde(default)]
    pub uses: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerData {
    #[serde(default)]
    pub homes: Vec<HomePoint>,
    #[serde(default)]
    pub last_location: Option<LocationPoint>,
    #[serde(default)]
    pub death_location: Option<LocationPoint>,
    #[serde(default)]
    pub rank: String,
    #[serde(default)]
    pub secondary_rank: Option<String>,
    #[serde(default)]
    pub visited_before: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedHomeAccess {
    pub owner: String,
    pub home: HomePoint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RtpConfig {
    #[serde(default = "default_rtp_world")]
    pub world: String,
    #[serde(default = "default_min_radius")]
    pub min_radius: i32,
    #[serde(default = "default_max_radius")]
    pub max_radius: i32,
    #[serde(default = "default_rtp_cooldown")]
    pub cooldown_seconds: u64,
    #[serde(default = "default_rtp_slow_falling")]
    pub slow_falling_ticks: u32,
    #[serde(default = "default_rtp_attempts")]
    pub max_attempts: u32,
}

fn default_rtp_world() -> String {
    "minecraft:overworld".to_string()
}
fn default_min_radius() -> i32 {
    250
}
fn default_max_radius() -> i32 {
    4500
}
fn default_rtp_cooldown() -> u64 {
    60
}
fn default_rtp_slow_falling() -> u32 {
    60
}
fn default_rtp_attempts() -> u32 {
    24
}

impl Default for RtpConfig {
    fn default() -> Self {
        Self {
            world: default_rtp_world(),
            min_radius: default_min_radius(),
            max_radius: default_max_radius(),
            cooldown_seconds: default_rtp_cooldown(),
            slow_falling_ticks: default_rtp_slow_falling(),
            max_attempts: default_rtp_attempts(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankProfile {
    pub id: String,
    #[serde(default)]
    pub tier: u8,
    #[serde(default)]
    pub display_prefix: String,
    #[serde(default = "default_max_homes")]
    pub max_homes: usize,
    #[serde(default = "default_max_shared")]
    pub max_shared_per_home: usize,
    #[serde(default)]
    pub cooldown_seconds: u64,
    #[serde(default = "default_rtp_cooldown")]
    pub rtp_cooldown_seconds: u64,
    #[serde(default)]
    pub can_set_spawn: bool,
    #[serde(default)]
    pub can_set_warp: bool,
    #[serde(default)]
    pub can_back_on_death: bool,
    #[serde(default)]
    pub unlimited_homes: bool,
}

fn default_max_homes() -> usize {
    5
}
fn default_max_shared() -> usize {
    5
}

impl Default for RankProfile {
    fn default() -> Self {
        Self {
            id: "player".to_string(),
            tier: 0,
            display_prefix: "§7[Игрок] ".to_string(),
            max_homes: 3,
            max_shared_per_home: 3,
            cooldown_seconds: 5,
            rtp_cooldown_seconds: 120,
            can_set_spawn: false,
            can_set_warp: false,
            can_back_on_death: false,
            unlimited_homes: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EssentialsConfig {
    #[serde(default = "default_plugin_name")]
    pub plugin_name: String,
    #[serde(default = "default_combat_seconds")]
    pub combat_tag_seconds: u64,
    #[serde(default = "default_true")]
    pub safe_teleport_check: bool,
    #[serde(default = "default_post_tp_ticks")]
    pub post_teleport_invulnerability_ticks: u32,
    #[serde(default = "default_true")]
    pub teleport_new_players_to_spawn: bool,
    #[serde(default = "default_tpa_timeout")]
    pub tpa_timeout_seconds: u64,
    #[serde(default = "default_blindness_ticks")]
    pub blindness_flash_ticks: u32,
    #[serde(default = "default_slow_falling_ticks")]
    pub slow_falling_ticks: u32,
    #[serde(default)]
    pub rtp: RtpConfig,
    #[serde(default)]
    pub ranks: Vec<RankProfile>,
}

fn default_plugin_name() -> String {
    "kronos-essentials".to_string()
}
fn default_combat_seconds() -> u64 {
    DEFAULT_COMBAT_TAG_SECONDS
}
fn default_true() -> bool {
    true
}
fn default_post_tp_ticks() -> u32 {
    50
}
fn default_tpa_timeout() -> u64 {
    DEFAULT_TPA_TIMEOUT_SECONDS
}
fn default_blindness_ticks() -> u32 {
    16
}
fn default_slow_falling_ticks() -> u32 {
    30
}

impl Default for EssentialsConfig {
    fn default() -> Self {
        Self {
            plugin_name: default_plugin_name(),
            combat_tag_seconds: default_combat_seconds(),
            safe_teleport_check: true,
            post_teleport_invulnerability_ticks: default_post_tp_ticks(),
            teleport_new_players_to_spawn: true,
            tpa_timeout_seconds: default_tpa_timeout(),
            blindness_flash_ticks: default_blindness_ticks(),
            slow_falling_ticks: default_slow_falling_ticks(),
            rtp: RtpConfig::default(),
            ranks: vec![RankProfile::default()],
        }
    }
}

impl EssentialsConfig {
    pub fn rank_profile(&self, rank_id: &str) -> RankProfile {
        if let Some(found) = self
            .ranks
            .iter()
            .find(|r| r.id.eq_ignore_ascii_case(rank_id))
        {
            return found.clone();
        }
        self.ranks
            .iter()
            .find(|r| r.id.eq_ignore_ascii_case("player"))
            .cloned()
            .unwrap_or_default()
    }

    /// Dual-track merge: combines primary and optional secondary rank (donate + staff)
    /// so players always keep the highest home limits and lowest teleport cooldowns.
    pub fn effective_rank_for(&self, player: &PlayerData) -> RankProfile {
        let mut best = self.rank_profile(if player.rank.is_empty() {
            "player"
        } else {
            &player.rank
        });
        if let Some(sec_id) = &player.secondary_rank {
            if !sec_id.is_empty() {
                let sec = self.rank_profile(sec_id);
                best.tier = best.tier.max(sec.tier);
                best.max_homes = best.max_homes.max(sec.max_homes);
                best.max_shared_per_home = best.max_shared_per_home.max(sec.max_shared_per_home);
                best.cooldown_seconds = best.cooldown_seconds.min(sec.cooldown_seconds);
                best.rtp_cooldown_seconds = best.rtp_cooldown_seconds.min(sec.rtp_cooldown_seconds);
                best.can_set_spawn |= sec.can_set_spawn;
                best.can_set_warp |= sec.can_set_warp;
                best.can_back_on_death |= sec.can_back_on_death;
                best.unlimited_homes |= sec.unlimited_homes;
            }
        }
        best
    }

    pub fn next_donate_after(&self, current: &RankProfile) -> Option<&RankProfile> {
        const DONATE_ORDER: [&str; 11] = [
            "player", "vip", "premium", "deluxe", "elite", "titan", "legend", "emperor",
            "overlord", "immortal", "kronos",
        ];
        let pos = DONATE_ORDER
            .iter()
            .position(|id| id.eq_ignore_ascii_case(&current.id))?;
        let next_id = DONATE_ORDER.get(pos + 1)?;
        self.ranks.iter().find(|r| r.id.eq_ignore_ascii_case(next_id))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EssentialsStore {
    #[serde(default)]
    pub spawn: Option<LocationPoint>,
    #[serde(default)]
    pub warps: HashMap<String, WarpPoint>,
    #[serde(default)]
    pub players: HashMap<String, PlayerData>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LegacyEssentialsStore {
    #[serde(default)]
    pub homes: HashMap<String, HashMap<String, LocationPoint>>,
    #[serde(default)]
    pub warps: HashMap<String, LocationPoint>,
    #[serde(default)]
    pub spawn: Option<LocationPoint>,
    #[serde(default)]
    pub last_locations: HashMap<String, LocationPoint>,
}

impl From<LegacyEssentialsStore> for EssentialsStore {
    fn from(legacy: LegacyEssentialsStore) -> Self {
        let mut store = EssentialsStore {
            spawn: legacy.spawn,
            warps: HashMap::new(),
            players: HashMap::new(),
        };

        for (warp_name, loc) in legacy.warps {
            store.warps.insert(
                warp_name.to_lowercase(),
                WarpPoint {
                    name: warp_name,
                    location: loc,
                    description: String::new(),
                    created_by: "migration".to_string(),
                    uses: 0,
                },
            );
        }

        for (player_name, player_homes) in legacy.homes {
            let entry = store.players.entry(player_name.to_lowercase()).or_default();
            for (home_name, loc) in player_homes {
                entry.homes.push(HomePoint {
                    name: home_name,
                    location: loc,
                    icon: default_home_icon(),
                    is_favorite: false,
                    shared_with: Vec::new(),
                    uses: 0,
                    created_at_ms: 0,
                });
            }
        }

        for (player_name, loc) in legacy.last_locations {
            let entry = store.players.entry(player_name.to_lowercase()).or_default();
            entry.last_location = Some(loc);
        }

        store
    }
}

#[derive(Debug, Clone)]
pub struct CombatTag {
    pub expires_at_ms: u64,
    pub last_notified_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TpaRequest {
    pub requester: String,
    pub target: String,
    pub bring_target_here: bool,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone)]
pub enum BedrockMenuAction {
    TeleportHome(String),
    TeleportShared { owner: String, home: String },
    RunSpawn,
    RunRtp,
    RunBack,
    OpenWarps,
}

#[derive(Debug, Clone)]
pub struct PendingBedrockForm {
    pub player_key: String,
    pub actions: Vec<BedrockMenuAction>,
}

pub struct EssentialsState {
    pub data_folder: String,
    pub config: EssentialsConfig,
    pub store: EssentialsStore,
    pub combat_tags: HashMap<String, CombatTag>,
    pub tpa_requests: HashMap<String, TpaRequest>,
    pub rtp_cooldowns: HashMap<String, u64>,
    pub tp_cooldowns: HashMap<String, u64>,
    pub pending_forms: HashMap<u32, PendingBedrockForm>,
    pub prng_seed: u64,
}

impl EssentialsState {
    pub fn new(data_folder: String, config: EssentialsConfig, store: EssentialsStore) -> Self {
        Self {
            data_folder,
            config,
            store,
            combat_tags: HashMap::new(),
            tpa_requests: HashMap::new(),
            rtp_cooldowns: HashMap::new(),
            tp_cooldowns: HashMap::new(),
            pending_forms: HashMap::new(),
            prng_seed: 0x9E37_79B9_7F4A_7C15,
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut s = self.prng_seed;
        if s == 0 {
            s = 0x9E37_79B9_7F4A_7C15;
        }
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        self.prng_seed = s;
        s
    }

    pub fn tag_combat(&mut self, player_name: &str, now_ms: u64) -> (u64, bool) {
        let key = normalize_key(player_name);
        let duration_secs = self.config.combat_tag_seconds.max(1);
        let expires_at_ms = now_ms.saturating_add(duration_secs * 1000);

        let should_notify = match self.combat_tags.get(&key) {
            Some(existing) if existing.expires_at_ms > now_ms => {
                now_ms.saturating_sub(existing.last_notified_ms) >= 4_000
            }
            _ => true,
        };

        let last_notified_ms = if should_notify {
            now_ms
        } else {
            self.combat_tags
                .get(&key)
                .map(|c| c.last_notified_ms)
                .unwrap_or(now_ms)
        };

        self.combat_tags.insert(
            key,
            CombatTag {
                expires_at_ms,
                last_notified_ms,
            },
        );
        (duration_secs, should_notify)
    }

    pub fn combat_remaining_secs(&mut self, player_name: &str, now_ms: u64) -> Option<u64> {
        let key = normalize_key(player_name);
        if let Some(tag) = self.combat_tags.get(&key) {
            if tag.expires_at_ms > now_ms {
                let rem_ms = tag.expires_at_ms - now_ms;
                return Some(rem_ms.div_ceil(1000));
            }
        }
        self.combat_tags.remove(&key);
        None
    }

    pub fn clear_combat(&mut self, player_name: &str) {
        self.combat_tags.remove(&normalize_key(player_name));
    }

    pub fn cooldown_remaining_secs(
        map: &mut HashMap<String, u64>,
        player_name: &str,
        now_ms: u64,
    ) -> Option<u64> {
        let key = normalize_key(player_name);
        if let Some(&until_ms) = map.get(&key) {
            if until_ms > now_ms {
                return Some((until_ms - now_ms).div_ceil(1000));
            }
        }
        map.remove(&key);
        None
    }

    pub fn set_cooldown(
        map: &mut HashMap<String, u64>,
        player_name: &str,
        seconds: u64,
        now_ms: u64,
    ) {
        if seconds == 0 {
            return;
        }
        map.insert(
            normalize_key(player_name),
            now_ms.saturating_add(seconds * 1000),
        );
    }

    pub fn cleanup_expired(&mut self, now_ms: u64) {
        self.combat_tags.retain(|_, t| t.expires_at_ms > now_ms);
        self.tpa_requests.retain(|_, r| r.expires_at_ms > now_ms);
        self.rtp_cooldowns.retain(|_, &mut until| until > now_ms);
        self.tp_cooldowns.retain(|_, &mut until| until > now_ms);
    }
}

pub fn normalize_key(input: &str) -> String {
    input.trim().to_ascii_lowercase()
}

pub fn sanitize_point_name(input: &str) -> Option<String> {
    let trimmed = input.trim();
    if trimmed.is_empty() || trimmed.len() > 24 {
        return None;
    }
    if trimmed
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
    {
        Some(trimmed.to_string())
    } else {
        None
    }
}
