use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Location {
    pub world: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub yaw: f32,
    pub pitch: f32,
}

impl Location {
    pub fn distance_squared(&self, other: &Location) -> f64 {
        if self.world != other.world {
            return f64::MAX;
        }
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        dx * dx + dy * dy + dz * dz
    }

    pub fn coords_label(&self) -> String {
        format!(
            "{} ({:.0}, {:.0}, {:.0})",
            self.world, self.x, self.y, self.z
        )
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct HomePoint {
    pub name: String,
    pub icon: String,
    pub location: Location,
    pub created_at_ms: u64,
    pub uses: u64,
    #[serde(default)]
    pub is_favorite: bool,
    #[serde(default)]
    pub shared_with: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct WarpPoint {
    pub name: String,
    pub icon: String,
    pub description: String,
    pub location: Location,
    pub min_tier: u8,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RtpConfig {
    pub world: String,
    pub min_radius: u32,
    pub max_radius: u32,
    pub cooldown_seconds: u64,
    pub drop_height: f64,
    pub slow_falling_ticks: u32,
    pub max_attempts: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RankProfile {
    pub id: String,
    pub tier: u8,
    pub display_prefix: String,
    pub max_homes: usize,
    pub max_shared_per_home: usize,
    pub warmup_seconds: u64,
    pub cooldown_seconds: u64,
    pub teleport_theme: String,
    pub can_set_spawn: bool,
    pub can_set_warp: bool,
    pub can_back_on_death: bool,
    pub unlimited_homes: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EssentialsConfig {
    pub plugin_name: String,
    pub cancel_on_move_blocks: f64,
    pub cancel_on_damage: bool,
    pub combat_tag_seconds: u64,
    pub safe_teleport_check: bool,
    pub post_teleport_invulnerability_ticks: u32,
    pub teleport_new_players_to_spawn: bool,
    pub tpa_timeout_seconds: u64,
    pub autosave_interval_seconds: u64,
    pub blindness_flash_ticks: u32,
    pub slow_falling_ticks: u32,
    pub rtp: RtpConfig,
    pub home_icons: Vec<String>,
    pub ranks: Vec<RankProfile>,
}

impl EssentialsConfig {
    pub fn rank_for(&self, rank_id: &str) -> RankProfile {
        self.ranks
            .iter()
            .find(|r| r.id.eq_ignore_ascii_case(rank_id))
            .cloned()
            .unwrap_or_else(|| self.ranks[0].clone())
    }

    pub fn next_rank_after(&self, tier: u8) -> Option<&RankProfile> {
        self.ranks
            .iter()
            .filter(|r| r.tier > tier)
            .min_by_key(|r| r.tier)
    }

    pub fn rank_name_for_tier(&self, min_tier: u8) -> String {
        self.ranks
            .iter()
            .find(|r| r.tier >= min_tier)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§c[ADMIN]".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerRecord {
    pub player_id: String,
    pub username: String,
    pub rank: String,
    pub selected_theme: Option<String>,
    pub homes: Vec<HomePoint>,
    pub last_location: Option<Location>,
    #[serde(default)]
    pub last_death_location: Option<Location>,
    pub last_teleport_ms: u64,
    #[serde(default)]
    pub last_rtp_ms: u64,
    #[serde(default)]
    pub first_join_completed: bool,
}

impl PlayerRecord {
    pub fn new(player_id: &str, username: &str) -> Self {
        Self {
            player_id: player_id.to_string(),
            username: username.to_string(),
            rank: "player".to_string(),
            selected_theme: None,
            homes: Vec::new(),
            last_location: None,
            last_death_location: None,
            last_teleport_ms: 0,
            last_rtp_ms: 0,
            first_join_completed: false,
        }
    }

    pub fn active_theme<'a>(&'a self, rank: &'a RankProfile) -> &'a str {
        self.selected_theme
            .as_deref()
            .unwrap_or(rank.teleport_theme.as_str())
    }

    pub fn favorite_home(&self) -> Option<&HomePoint> {
        self.homes.iter().find(|h| h.is_favorite)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct EssentialsStore {
    pub spawn: Option<Location>,
    pub warps: HashMap<String, WarpPoint>,
    pub players: HashMap<String, PlayerRecord>,
}

#[derive(Clone, Debug)]
pub struct PendingTeleport {
    pub player_id: String,
    pub target_label: String,
    pub destination: Location,
    pub origin: Location,
    pub theme: String,
    pub started_at_ms: u64,
    pub duration_ms: u64,
    pub ticks_elapsed: u32,
    pub last_second_announced: u64,
    pub is_rtp_meteor: bool,
}

#[derive(Clone, Debug)]
pub struct ActiveMeteorFall {
    pub player_id: String,
    pub world: String,
    pub expires_at_ms: u64,
    pub ticks: u32,
}

#[derive(Clone, Debug)]
pub struct CombatTag {
    pub opponent: String,
    pub expires_at_ms: u64,
}

#[derive(Clone, Debug)]
pub struct TpaRequest {
    pub from_id: String,
    pub from_name: String,
    pub to_id: String,
    pub summon_here: bool,
    pub expires_at_ms: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MenuSlot {
    pub slot: u8,
    pub item: String,
    pub name: String,
    pub lore: Vec<String>,
    pub enchanted: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChestMenuDescriptor {
    pub menu_id: String,
    pub title: String,
    pub rows: u8,
    pub slots: Vec<MenuSlot>,
}
