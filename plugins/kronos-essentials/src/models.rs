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
    #[serde(default)]
    pub rtp_cooldown_seconds: Option<u64>,
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
    pub fn find_rank(&self, rank_id: &str) -> Option<&RankProfile> {
        self.ranks.iter().find(|r| r.id.eq_ignore_ascii_case(rank_id))
    }

    pub fn rank_for(&self, rank_id: &str) -> RankProfile {
        self.find_rank(rank_id)
            .cloned()
            .unwrap_or_else(|| self.ranks[0].clone())
    }

    /// Merges primary (staff/donate) rank and purchased donate rank so staff never lose bought homes, themes, or cooldowns.
    pub fn effective_rank_for(&self, player: &PlayerRecord) -> RankProfile {
        let primary = self.rank_for(&player.rank);
        let Some(ref donate_id) = player.donate_rank else {
            return primary;
        };
        if donate_id.eq_ignore_ascii_case(&primary.id) {
            return primary;
        }
        let donate = self.rank_for(donate_id);
        let teleport_theme = if donate.tier > primary.tier {
            donate.teleport_theme
        } else {
            primary.teleport_theme
        };
        let rtp_cd = match (primary.rtp_cooldown_seconds, donate.rtp_cooldown_seconds) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (Some(a), None) => Some(a),
            (None, Some(b)) => Some(b),
            (None, None) => None,
        };
        RankProfile {
            id: primary.id,
            tier: primary.tier.max(donate.tier),
            display_prefix: primary.display_prefix,
            max_homes: primary.max_homes.max(donate.max_homes),
            max_shared_per_home: primary.max_shared_per_home.max(donate.max_shared_per_home),
            warmup_seconds: primary.warmup_seconds.min(donate.warmup_seconds),
            cooldown_seconds: primary.cooldown_seconds.min(donate.cooldown_seconds),
            rtp_cooldown_seconds: rtp_cd,
            teleport_theme,
            can_set_spawn: primary.can_set_spawn || donate.can_set_spawn,
            can_set_warp: primary.can_set_warp || donate.can_set_warp,
            can_back_on_death: primary.can_back_on_death || donate.can_back_on_death,
            unlimited_homes: primary.unlimited_homes || donate.unlimited_homes,
        }
    }

    /// Returns the next higher donate rank strictly above `current.max_homes` (never recommends staff ranks).
    pub fn next_donate_after(&self, current: &RankProfile) -> Option<&RankProfile> {
        if current.unlimited_homes {
            return None;
        }
        const DONATE_ORDER: &[&str] = &[
            "vip", "premium", "deluxe", "elite", "titan", "legend", "emperor", "overlord", "immortal", "kronos",
        ];
        DONATE_ORDER
            .iter()
            .filter_map(|id| self.find_rank(id))
            .find(|r| r.max_homes > current.max_homes)
    }

    /// Returns the lowest donate rank that unlocks the requested home slot index (0-based).
    pub fn donate_unlock_for_slot(&self, home_index: usize) -> String {
        const DONATE_ORDER: &[&str] = &[
            "vip", "premium", "deluxe", "elite", "titan", "legend", "emperor", "overlord", "immortal", "kronos",
        ];
        DONATE_ORDER
            .iter()
            .filter_map(|id| self.find_rank(id))
            .find(|r| r.max_homes > home_index)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§6§l✦KRONOS✦".to_string())
    }

    pub fn rank_name_for_tier(&self, min_tier: u8) -> String {
        self.ranks
            .iter()
            .find(|r| r.tier >= min_tier)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§6§l✦KRONOS✦".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerRecord {
    pub player_id: String,
    pub username: String,
    pub rank: String,
    #[serde(default)]
    pub donate_rank: Option<String>,
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
            donate_rank: None,
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
