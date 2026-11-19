use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct BlockPos {
    pub world: String,
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExpandDirection {
    North, // -Z
    South, // +Z
    West,  // -X
    East,  // +X
    Up,    // +Y
    Down,  // -Y
    All,   // Horizontal +X,-X,+Z,-Z
    Vert,  // Full world height (min_y..max_y)
}

impl ExpandDirection {
    pub fn parse(raw: &str) -> Option<Self> {
        match raw.to_lowercase().as_str() {
            "north" | "n" | "север" => Some(Self::North),
            "south" | "s" | "юг" => Some(Self::South),
            "west" | "w" | "запад" => Some(Self::West),
            "east" | "e" | "восток" => Some(Self::East),
            "up" | "u" | "вверх" => Some(Self::Up),
            "down" | "d" | "вниз" => Some(Self::Down),
            "all" | "around" | "все" => Some(Self::All),
            "vert" | "vertical" | "вертикаль" => Some(Self::Vert),
            _ => None,
        }
    }

    pub fn label_ru(self) -> &'static str {
        match self {
            Self::North => "Север (-Z)",
            Self::South => "Юг (+Z)",
            Self::West => "Запад (-X)",
            Self::East => "Восток (+X)",
            Self::Up => "Вверх (+Y)",
            Self::Down => "Вниз (-Y)",
            Self::All => "Во все стороны",
            Self::Vert => "От бедрока до неба",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct RegionBounds {
    pub world: String,
    pub min_x: i32,
    pub min_y: i32,
    pub min_z: i32,
    pub max_x: i32,
    pub max_y: i32,
    pub max_z: i32,
}

impl RegionBounds {
    pub fn from_points(a: &BlockPos, b: &BlockPos) -> Option<Self> {
        if a.world != b.world {
            return None;
        }
        Some(Self {
            world: a.world.clone(),
            min_x: a.x.min(b.x),
            min_y: a.y.min(b.y),
            min_z: a.z.min(b.z),
            max_x: a.x.max(b.x),
            max_y: a.y.max(b.y),
            max_z: a.z.max(b.z),
        })
    }

    pub fn contains(&self, world: &str, x: i32, y: i32, z: i32) -> bool {
        self.world == world
            && x >= self.min_x
            && x <= self.max_x
            && y >= self.min_y
            && y <= self.max_y
            && z >= self.min_z
            && z <= self.max_z
    }

    pub fn intersects(&self, other: &RegionBounds) -> bool {
        self.world == other.world
            && self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
            && self.min_z <= other.max_z
            && self.max_z >= other.min_z
    }

    pub fn dimensions(&self) -> (u64, u64, u64) {
        let dx = (self.max_x as i64 - self.min_x as i64 + 1).max(0) as u64;
        let dy = (self.max_y as i64 - self.min_y as i64 + 1).max(0) as u64;
        let dz = (self.max_z as i64 - self.min_z as i64 + 1).max(0) as u64;
        (dx, dy, dz)
    }

    pub fn volume(&self) -> u64 {
        let (dx, dy, dz) = self.dimensions();
        dx.saturating_mul(dy).saturating_mul(dz)
    }

    pub fn center(&self) -> (f64, f64, f64) {
        (
            (self.min_x as f64 + self.max_x as f64) * 0.5 + 0.5,
            (self.max_y as f64).min((self.min_y as f64 + self.max_y as f64) * 0.5 + 1.0),
            (self.min_z as f64 + self.max_z as f64) * 0.5 + 0.5,
        )
    }

    pub fn resized(
        &self,
        dir: ExpandDirection,
        delta: i32,
        world_min_y: i32,
        world_max_y: i32,
    ) -> Self {
        let mut next = self.clone();
        match dir {
            ExpandDirection::North => next.min_z = next.min_z.saturating_sub(delta),
            ExpandDirection::South => next.max_z = next.max_z.saturating_add(delta),
            ExpandDirection::West => next.min_x = next.min_x.saturating_sub(delta),
            ExpandDirection::East => next.max_x = next.max_x.saturating_add(delta),
            ExpandDirection::Up => next.max_y = next.max_y.saturating_add(delta).clamp(world_min_y, world_max_y),
            ExpandDirection::Down => next.min_y = next.min_y.saturating_sub(delta).clamp(world_min_y, world_max_y),
            ExpandDirection::All => {
                next.min_x = next.min_x.saturating_sub(delta);
                next.max_x = next.max_x.saturating_add(delta);
                next.min_z = next.min_z.saturating_sub(delta);
                next.max_z = next.max_z.saturating_add(delta);
            }
            ExpandDirection::Vert => {
                next.min_y = world_min_y;
                next.max_y = world_max_y;
            }
        }
        next
    }

    pub fn meets_min_side(&self, min_side: u64) -> bool {
        let (dx, dy, dz) = self.dimensions();
        dx >= min_side && dy >= min_side && dz >= min_side
    }
}

/// Granular per-action flags for a region.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionFlags {
    pub explosions: bool,
    pub pvp: bool,
    pub fall_damage: bool,
    pub open_chests: bool,
    pub use_doors: bool,
    pub use_buttons_levers: bool,
    pub use_workstations: bool,
    pub mob_spawning: bool,
    pub fire_spread: bool,
    pub item_pickup: bool,
    pub entry_allowed: bool,
}

impl Default for RegionFlags {
    fn default() -> Self {
        Self {
            explosions: false,         // Protected from TNT/Creepers by default
            pvp: true,                 // PvP allowed until VIP+ disables it
            fall_damage: true,         // Fall damage on until PREMIUM+ disables it
            open_chests: false,        // Strangers cannot open chests/barrels/shulkers
            use_doors: false,          // Strangers cannot open doors/trapdoors/gates
            use_buttons_levers: false, // Strangers cannot press buttons/levers/plates
            use_workstations: false,   // Strangers cannot use anvils/furnaces/crafting
            mob_spawning: true,        // Hostile mobs allowed until VIP+ disables
            fire_spread: false,        // Fire spread blocked by default
            item_pickup: true,         // Drops can be picked up
            entry_allowed: true,       // Entry open until LEGEND+ restricts
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberRole {
    Member,
    Officer,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Region {
    pub id: String,
    pub name: String,
    pub owner_id: String,
    pub owner_name: String,
    pub bounds: RegionBounds,
    pub flags: RegionFlags,
    pub members: HashMap<String, MemberRole>,
    pub border_theme: String,
    pub greeting_title: Option<String>,
    pub created_at_ms: u64,
}

impl Region {
    pub fn is_owner(&self, player_id: &str) -> bool {
        self.owner_id.eq_ignore_ascii_case(player_id)
    }

    pub fn is_officer_or_owner(&self, player_id: &str) -> bool {
        self.is_owner(player_id)
            || matches!(self.members.get(player_id), Some(MemberRole::Officer))
    }

    pub fn is_trusted(&self, player_id: &str) -> bool {
        self.is_owner(player_id) || self.members.contains_key(player_id)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RankPolicy {
    pub id: String,
    pub tier: u8,
    pub display_prefix: String,
    pub max_regions: usize,
    pub max_volume: u64,
    pub max_members: usize,
    pub default_border_theme: String,
    pub can_toggle_pvp: bool,
    pub can_toggle_fall_damage: bool,
    pub can_toggle_mob_spawning: bool,
    pub can_toggle_entry: bool,
    pub can_set_greeting: bool,
    pub unlimited_regions: bool,
    pub bypass_protection: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RegionsConfig {
    pub plugin_name: String,
    pub wand_item_id: String,
    pub wand_display_name: String,
    pub wand_tag: String,
    pub min_region_side: u64,
    pub world_min_y: i32,
    pub world_max_y: i32,
    pub visualizer_duration_seconds: u64,
    pub visualizer_step_blocks: f64,
    pub max_particles_per_tick: usize,
    pub autosave_interval_seconds: u64,
    pub ranks: Vec<RankPolicy>,
}

impl RegionsConfig {
    pub fn find_rank(&self, rank_id: &str) -> Option<&RankPolicy> {
        self.ranks.iter().find(|r| r.id.eq_ignore_ascii_case(rank_id))
    }

    pub fn rank_for(&self, rank_id: &str) -> RankPolicy {
        self.find_rank(rank_id)
            .cloned()
            .unwrap_or_else(|| self.ranks[0].clone())
    }

    /// Merges primary (staff/donate) rank and purchased donate rank so staff never lose bought region limits or themes.
    pub fn effective_rank_for(&self, store: &RegionStore, player_id: &str) -> RankPolicy {
        let primary = self.rank_for(store.rank_id_of(player_id));
        let Some(donate_id) = store.player_donate_ranks.get(player_id) else {
            return primary;
        };
        if donate_id.eq_ignore_ascii_case(&primary.id) {
            return primary;
        }
        let donate = self.rank_for(donate_id);
        let default_border_theme = if donate.tier > primary.tier {
            donate.default_border_theme
        } else {
            primary.default_border_theme
        };
        RankPolicy {
            id: primary.id,
            tier: primary.tier.max(donate.tier),
            display_prefix: primary.display_prefix,
            max_regions: primary.max_regions.max(donate.max_regions),
            max_volume: primary.max_volume.max(donate.max_volume),
            max_members: primary.max_members.max(donate.max_members),
            default_border_theme,
            can_toggle_pvp: primary.can_toggle_pvp || donate.can_toggle_pvp,
            can_toggle_fall_damage: primary.can_toggle_fall_damage || donate.can_toggle_fall_damage,
            can_toggle_mob_spawning: primary.can_toggle_mob_spawning || donate.can_toggle_mob_spawning,
            can_toggle_entry: primary.can_toggle_entry || donate.can_toggle_entry,
            can_set_greeting: primary.can_set_greeting || donate.can_set_greeting,
            unlimited_regions: primary.unlimited_regions || donate.unlimited_regions,
            bypass_protection: primary.bypass_protection || donate.bypass_protection,
        }
    }

    /// Returns the next higher donate rank strictly above `current.max_regions` (never recommends staff ranks).
    pub fn next_donate_after(&self, current: &RankPolicy) -> Option<&RankPolicy> {
        if current.unlimited_regions {
            return None;
        }
        const DONATE_ORDER: &[&str] = &[
            "vip", "premium", "deluxe", "elite", "titan", "legend", "emperor", "overlord", "immortal", "kronos",
        ];
        DONATE_ORDER
            .iter()
            .filter_map(|id| self.find_rank(id))
            .find(|r| r.max_regions > current.max_regions)
    }

    /// Returns the lowest donate rank that unlocks the requested region slot index (0-based).
    pub fn donate_unlock_for_slot(&self, slot_idx: usize) -> String {
        const DONATE_ORDER: &[&str] = &[
            "vip", "premium", "deluxe", "elite", "titan", "legend", "emperor", "overlord", "immortal", "kronos",
        ];
        DONATE_ORDER
            .iter()
            .filter_map(|id| self.find_rank(id))
            .find(|r| r.max_regions > slot_idx)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§6§l✦KRONOS✦".to_string())
    }

    pub fn rank_label_for_tier(&self, min_tier: u8) -> String {
        self.ranks
            .iter()
            .find(|r| r.tier >= min_tier)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§6§l✦KRONOS✦".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RegionStore {
    pub regions: HashMap<String, Region>,
    pub player_ranks: HashMap<String, String>,
    #[serde(default)]
    pub player_donate_ranks: HashMap<String, String>,
    pub player_names: HashMap<String, String>,
    pub next_id_counter: u64,
}

impl RegionStore {
    pub fn find_by_name_or_id(&self, query: &str) -> Option<&Region> {
        let q = query.trim().to_lowercase();
        self.regions
            .get(&q)
            .or_else(|| self.regions.values().find(|r| r.name.to_lowercase() == q))
    }

    pub fn find_id_by_name_or_id(&self, query: &str) -> Option<String> {
        self.find_by_name_or_id(query).map(|r| r.id.clone())
    }

    pub fn owned_count(&self, player_id: &str) -> usize {
        self.regions
            .values()
            .filter(|r| r.is_owner(player_id))
            .count()
    }

    pub fn rank_id_of(&self, player_id: &str) -> &str {
        self.player_ranks
            .get(player_id)
            .map(|s| s.as_str())
            .unwrap_or("player")
    }
}

#[derive(Clone, Debug, Default)]
pub struct WandSelection {
    pub pos1: Option<BlockPos>,
    pub pos2: Option<BlockPos>,
    pub updated_at_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ActiveBorderViz {
    pub player_id: String,
    pub bounds: RegionBounds,
    pub theme: String,
    pub conflict: bool,
    pub expires_at_ms: u64,
    pub ticks: u32,
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
