import { PluginTemplate } from '../types';

export const KRONOS_REGIONS_TEMPLATE: PluginTemplate = {
  id: 'kronos-regions',
  title: 'KRONOS Regions (Land Claim, Wand, 3D Borders & Flags)',
  description:
    'Premium Rust WASM region protection: Claim Wand (Pos1/Pos2), 3D particle borders with 5 themes, directional expand/shrink GUI, rename, granular flags (explosions, PvP, fall damage, chests, doors, buttons, mobs), members/roles, confirm delete, and rank perks.',
  tags: ['Regions', 'Claim Wand', '3D Particles', 'Granular Flags'],
  files: {
    'Cargo.toml': `[package]
name = "kronos-regions"
version = "1.0.0"
edition = "2021"
description = "KRONOS Regions: Land protection with Claim Wand, 3D particle border visualization, directional expand/shrink, granular flags, Chest GUIs, and rank monetization perks for Pumpkin 26.3"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
`,

    'data/config.json': `{
  "plugin_name": "kronos-regions",
  "wand_item_id": "minecraft:golden_axe",
  "wand_display_name": "§6✦ Топор Привата KRONOS",
  "wand_tag": "kronos_region_wand",
  "min_region_side": 3,
  "world_min_y": -64,
  "world_max_y": 319,
  "visualizer_duration_seconds": 25,
  "visualizer_step_blocks": 1.5,
  "max_particles_per_tick": 140,
  "autosave_interval_seconds": 30,
  "ranks": [
    {
      "id": "player",
      "tier": 0,
      "display_prefix": "§7[ИГРОК] ",
      "max_regions": 2,
      "max_volume": 40000,
      "max_members": 3,
      "default_border_theme": "emerald_wire",
      "can_toggle_pvp": false,
      "can_toggle_fall_damage": false,
      "can_toggle_mob_spawning": false,
      "can_toggle_entry": false,
      "can_set_greeting": false,
      "unlimited_regions": false,
      "bypass_protection": false
    },
    {
      "id": "vip",
      "tier": 1,
      "display_prefix": "§e§l[VIP] ",
      "max_regions": 4,
      "max_volume": 120000,
      "max_members": 6,
      "default_border_theme": "golden_beam",
      "can_toggle_pvp": true,
      "can_toggle_fall_damage": false,
      "can_toggle_mob_spawning": true,
      "can_toggle_entry": false,
      "can_set_greeting": true,
      "unlimited_regions": false,
      "bypass_protection": false
    },
    {
      "id": "premium",
      "tier": 2,
      "display_prefix": "§a§l[PREMIUM] ",
      "max_regions": 6,
      "max_volume": 300000,
      "max_members": 10,
      "default_border_theme": "amethyst_grid",
      "can_toggle_pvp": true,
      "can_toggle_fall_damage": true,
      "can_toggle_mob_spawning": true,
      "can_toggle_entry": false,
      "can_set_greeting": true,
      "unlimited_regions": false,
      "bypass_protection": false
    },
    {
      "id": "legend",
      "tier": 3,
      "display_prefix": "§d§l[LEGEND] ",
      "max_regions": 10,
      "max_volume": 750000,
      "max_members": 20,
      "default_border_theme": "dragon_aurora",
      "can_toggle_pvp": true,
      "can_toggle_fall_damage": true,
      "can_toggle_mob_spawning": true,
      "can_toggle_entry": true,
      "can_set_greeting": true,
      "unlimited_regions": false,
      "bypass_protection": false
    },
    {
      "id": "admin",
      "tier": 4,
      "display_prefix": "§c§l[ADMIN] ",
      "max_regions": 50,
      "max_volume": 50000000,
      "max_members": 100,
      "default_border_theme": "kronos_prism",
      "can_toggle_pvp": true,
      "can_toggle_fall_damage": true,
      "can_toggle_mob_spawning": true,
      "can_toggle_entry": true,
      "can_set_greeting": true,
      "unlimited_regions": true,
      "bypass_protection": true
    },
    {
      "id": "owner",
      "tier": 5,
      "display_prefix": "§4§l[OWNER] ",
      "max_regions": 100,
      "max_volume": 50000000,
      "max_members": 100,
      "default_border_theme": "kronos_prism",
      "can_toggle_pvp": true,
      "can_toggle_fall_damage": true,
      "can_toggle_mob_spawning": true,
      "can_toggle_entry": true,
      "can_set_greeting": true,
      "unlimited_regions": true,
      "bypass_protection": true
    }
  ]
}
`,

    'src/models.rs': `use serde::{Deserialize, Serialize};
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
    pub fn rank_for(&self, rank_id: &str) -> RankPolicy {
        self.ranks
            .iter()
            .find(|r| r.id.eq_ignore_ascii_case(rank_id))
            .cloned()
            .unwrap_or_else(|| self.ranks[0].clone())
    }

    pub fn next_rank_after(&self, tier: u8) -> Option<&RankPolicy> {
        self.ranks
            .iter()
            .filter(|r| r.tier > tier)
            .min_by_key(|r| r.tier)
    }

    pub fn rank_label_for_tier(&self, min_tier: u8) -> String {
        self.ranks
            .iter()
            .find(|r| r.tier >= min_tier)
            .map(|r| r.display_prefix.trim().to_string())
            .unwrap_or_else(|| "§c[ADMIN]".to_string())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct RegionStore {
    pub regions: HashMap<String, Region>,
    pub player_ranks: HashMap<String, String>,
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
`,

    'src/host.rs': `use crate::models::{BlockPos, ChestMenuDescriptor};
use log::info;

/// Pumpkin 26.3 WASM Host Imports (\`pumpkin_host\` namespace).
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "pumpkin_host")]
extern "C" {
    fn host_send_chat(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
    fn host_send_action_bar(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
    fn host_send_title(
        pid_ptr: *const u8,
        pid_len: usize,
        title_ptr: *const u8,
        title_len: usize,
        sub_ptr: *const u8,
        sub_len: usize,
        fade_in: u32,
        stay: u32,
        fade_out: u32,
    );
    fn host_spawn_particle(
        world_ptr: *const u8,
        world_len: usize,
        particle_ptr: *const u8,
        particle_len: usize,
        x: f64,
        y: f64,
        z: f64,
        count: u32,
        speed: f32,
    );
    fn host_play_sound(
        pid_ptr: *const u8,
        pid_len: usize,
        sound_ptr: *const u8,
        sound_len: usize,
        volume: f32,
        pitch: f32,
    );
    fn host_give_custom_item(
        pid_ptr: *const u8,
        pid_len: usize,
        json_ptr: *const u8,
        json_len: usize,
    );
    fn host_teleport(
        pid_ptr: *const u8,
        pid_len: usize,
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
    );
    fn host_open_chest_menu(pid_ptr: *const u8, pid_len: usize, json_ptr: *const u8, json_len: usize);
    fn host_close_menu(pid_ptr: *const u8, pid_len: usize);
    fn host_get_player_block_pos(
        pid_ptr: *const u8,
        pid_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn host_now_ms() -> u64;
}

pub fn send_chat(player_id: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_chat(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[CHAT -> {}] {}", player_id, msg);
}

pub fn send_action_bar(player_id: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_action_bar(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, msg);
}

pub fn send_title(player_id: &str, title: &str, subtitle: &str, fade_in: u32, stay: u32, fade_out: u32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_title(
            player_id.as_ptr(),
            player_id.len(),
            title.as_ptr(),
            title.len(),
            subtitle.as_ptr(),
            subtitle.len(),
            fade_in,
            stay,
            fade_out,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TITLE -> {}] {} | {}", player_id, title, subtitle);
}

pub fn spawn_particle(world: &str, particle: &str, x: f64, y: f64, z: f64, count: u32, speed: f32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_spawn_particle(
            world.as_ptr(),
            world.len(),
            particle.as_ptr(),
            particle.len(),
            x,
            y,
            z,
            count,
            speed,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (world, particle, x, y, z, count, speed);
}

pub fn play_sound(player_id: &str, sound: &str, volume: f32, pitch: f32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_play_sound(player_id.as_ptr(), player_id.len(), sound.as_ptr(), sound.len(), volume, pitch);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, sound, volume, pitch);
}

pub fn give_claim_wand(player_id: &str, item_id: &str, display_name: &str, tag: &str) {
    let payload = serde_json::json!({
        "item": item_id,
        "name": display_name,
        "tag": tag,
        "enchanted": true,
        "lore": [
            "§7Специальный инструмент привата территории",
            "§8────────────────────────",
            "§a▸ ЛКМ по блоку §8— §fТочка #1",
            "§e▸ ПКМ по блоку §8— §fТочка #2 + 3D-превью",
            "§b▸ Shift+ПКМ §8— §fОткрыть GUI меню (/rg)",
            "§8────────────────────────",
            "§7Создать приват: §e/rg claim <название>"
        ]
    });
    let json = payload.to_string();
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_give_custom_item(player_id.as_ptr(), player_id.len(), json.as_ptr(), json.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[GIVE WAND -> {}] {}", player_id, json);
}

pub fn teleport_player(player_id: &str, world: &str, x: f64, y: f64, z: f64) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_teleport(player_id.as_ptr(), player_id.len(), world.as_ptr(), world.len(), x, y, z, 0.0, 0.0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TP -> {}] {} ({:.1}, {:.1}, {:.1})", player_id, world, x, y, z);
}

pub fn open_chest_menu(player_id: &str, descriptor: &ChestMenuDescriptor) {
    if let Ok(json) = serde_json::to_string(descriptor) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            host_open_chest_menu(player_id.as_ptr(), player_id.len(), json.as_ptr(), json.len());
        }
        #[cfg(not(target_arch = "wasm32"))]
        info!("[MENU -> {}] {} ({} slots)", player_id, descriptor.title, descriptor.slots.len());
    }
}

pub fn close_menu(player_id: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_close_menu(player_id.as_ptr(), player_id.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = player_id;
}

pub fn get_player_block_pos(player_id: &str) -> BlockPos {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 256];
        let n = unsafe { host_get_player_block_pos(player_id.as_ptr(), player_id.len(), buf.as_mut_ptr(), buf.len()) };
        if n > 0 {
            if let Ok(pos) = serde_json::from_slice::<BlockPos>(&buf[..n as usize]) {
                return pos;
            }
        }
    }
    BlockPos {
        world: "world".to_string(),
        x: 0,
        y: 65,
        z: 0,
    }
}

pub fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_now_ms()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
`,

    'src/visualizer.rs': `use crate::host;
use crate::models::{ActiveBorderViz, BlockPos, RegionBounds, RegionsConfig};

pub struct BorderThemeSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub min_tier: u8,
    pub description: &'static str,
    pub edge_particle: &'static str,
    pub corner_particle: &'static str,
}

pub const BORDER_THEMES: &[BorderThemeSpec] = &[
    BorderThemeSpec {
        id: "emerald_wire",
        label: "§aИзумрудный контур",
        icon: "minecraft:emerald",
        min_tier: 0,
        description: "Чёткий зелёный 3D-каркас границ",
        edge_particle: "minecraft:happy_villager",
        corner_particle: "minecraft:end_rod",
    },
    BorderThemeSpec {
        id: "golden_beam",
        label: "§6Золотые лучи",
        icon: "minecraft:gold_block",
        min_tier: 1,
        description: "Сияющие золотые столбы и искры",
        edge_particle: "minecraft:wax_on",
        corner_particle: "minecraft:flame",
    },
    BorderThemeSpec {
        id: "amethyst_grid",
        label: "§dАметистовая сетка",
        icon: "minecraft:amethyst_cluster",
        min_tier: 2,
        description: "Фиолетовая магическая решётка по периметру",
        edge_particle: "minecraft:portal",
        corner_particle: "minecraft:enchant",
    },
    BorderThemeSpec {
        id: "dragon_aurora",
        label: "§5Аврора Дракона",
        icon: "minecraft:dragon_breath",
        min_tier: 3,
        description: "Неоновое дыхание дракона по краям привата",
        edge_particle: "minecraft:dragon_breath",
        corner_particle: "minecraft:reverse_portal",
    },
    BorderThemeSpec {
        id: "kronos_prism",
        label: "§bПризма KRONOS",
        icon: "minecraft:beacon",
        min_tier: 4,
        description: "Синее пламя душ и световые маяки на углах",
        edge_particle: "minecraft:soul_fire_flame",
        corner_particle: "minecraft:end_rod",
    },
];

pub fn find_border_theme(id: &str) -> &'static BorderThemeSpec {
    BORDER_THEMES
        .iter()
        .find(|t| t.id.eq_ignore_ascii_case(id))
        .unwrap_or(&BORDER_THEMES[0])
}

pub fn can_use_border_theme(rank_tier: u8, theme_id: &str) -> bool {
    rank_tier >= find_border_theme(theme_id).min_tier
}

/// Renders a crisp vertical marker beam when placing Pos1 or Pos2 with the Claim Wand.
pub fn spawn_point_beacon(pos: &BlockPos, is_first: bool) {
    let particle = if is_first {
        "minecraft:happy_villager"
    } else {
        "minecraft:flame"
    };
    for dy in 0..8 {
        host::spawn_particle(
            &pos.world,
            particle,
            pos.x as f64 + 0.5,
            pos.y as f64 + 0.3 + (dy as f64) * 0.65,
            pos.z as f64 + 0.5,
            2,
            0.01,
        );
    }
}

/// Renders a 3D wireframe border around a region every 6 ticks:
/// - 4 vertical corner pillars around player eye height
/// - Horizontal perimeter ring at player eye level + base + ceiling
/// - Switches to Red Alert particles if \`viz.conflict\` is true
pub fn render_border_frame(viz: &mut ActiveBorderViz, config: &RegionsConfig) {
    viz.ticks = viz.ticks.saturating_add(1);
    if viz.ticks % 6 != 0 {
        return;
    }

    let player_pos = host::get_player_block_pos(&viz.player_id);
    if player_pos.world != viz.bounds.world {
        return;
    }

    let theme = find_border_theme(&viz.theme);
    let edge_p = if viz.conflict { "minecraft:flame" } else { theme.edge_particle };
    let corner_p = if viz.conflict { "minecraft:smoke" } else { theme.corner_particle };

    let x0 = viz.bounds.min_x as f64;
    let x1 = viz.bounds.max_x as f64 + 1.0;
    let z0 = viz.bounds.min_z as f64;
    let z1 = viz.bounds.max_z as f64 + 1.0;
    let eye_y = (player_pos.y as f64 + 1.2)
        .clamp(viz.bounds.min_y as f64, viz.bounds.max_y as f64 + 1.0);
    let top_y = (eye_y + 4.0).min(viz.bounds.max_y as f64 + 1.0);
    let bot_y = (eye_y - 2.0).max(viz.bounds.min_y as f64);

    let step = config.visualizer_step_blocks.max(1.0);
    let max_budget = config.max_particles_per_tick.max(32);
    let mut emitted = 0usize;

    // 1. 4 vertical corner pillars
    let corners = [(x0, z0), (x1, z0), (x0, z1), (x1, z1)];
    for &(cx, cz) in &corners {
        let mut y = bot_y;
        while y <= top_y && emitted < max_budget {
            host::spawn_particle(&viz.bounds.world, corner_p, cx, y, cz, 1, 0.0);
            emitted += 1;
            y += step;
        }
    }

    // 2. Horizontal perimeter edges at eye_y and top_y
    for &ring_y in &[eye_y, top_y] {
        // Along X edges (North & South)
        let mut x = x0;
        while x <= x1 && emitted < max_budget {
            host::spawn_particle(&viz.bounds.world, edge_p, x, ring_y, z0, 1, 0.0);
            emitted += 1;
            if emitted < max_budget {
                host::spawn_particle(&viz.bounds.world, edge_p, x, ring_y, z1, 1, 0.0);
                emitted += 1;
            }
            x += step;
        }

        // Along Z edges (West & East)
        let mut z = z0 + step;
        while z < z1 && emitted < max_budget {
            host::spawn_particle(&viz.bounds.world, edge_p, x0, ring_y, z, 1, 0.0);
            emitted += 1;
            if emitted < max_budget {
                host::spawn_particle(&viz.bounds.world, edge_p, x1, ring_y, z, 1, 0.0);
                emitted += 1;
            }
            z += step;
        }
    }
}

pub fn build_viz(
    player_id: &str,
    bounds: RegionBounds,
    theme: &str,
    conflict: bool,
    config: &RegionsConfig,
    now_ms: u64,
) -> ActiveBorderViz {
    ActiveBorderViz {
        player_id: player_id.to_string(),
        bounds,
        theme: theme.to_string(),
        conflict,
        expires_at_ms: now_ms + config.visualizer_duration_seconds * 1000,
        ticks: 0,
    }
}
`,

    'src/protection.rs': `use crate::host;
use crate::models::{Region, RegionBounds, RegionStore, RegionsConfig};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InteractCategory {
    Container,   // Chests, barrels, shulkers, hoppers, furnaces, dispensers
    Door,        // Doors, trapdoors, fence gates
    Redstone,    // Buttons, levers, pressure plates, repeaters
    Workstation, // Crafting table, anvil, enchanting table, grindstone
    Other,
}

pub fn classify_block(block_id: &str) -> InteractCategory {
    let b = block_id.to_lowercase();
    if b.contains("chest")
        || b.contains("barrel")
        || b.contains("shulker")
        || b.contains("hopper")
        || b.contains("furnace")
        || b.contains("smoker")
        || b.contains("dispenser")
        || b.contains("dropper")
        || b.contains("brewing_stand")
    {
        return InteractCategory::Container;
    }
    if b.contains("door") || b.contains("gate") {
        return InteractCategory::Door;
    }
    if b.contains("button")
        || b.contains("lever")
        || b.contains("pressure_plate")
        || b.contains("repeater")
        || b.contains("comparator")
        || b.contains("note_block")
    {
        return InteractCategory::Redstone;
    }
    if b.contains("crafting_table")
        || b.contains("anvil")
        || b.contains("enchanting_table")
        || b.contains("grindstone")
        || b.contains("smithing_table")
        || b.contains("stonecutter")
        || b.contains("loom")
        || b.contains("cartography_table")
    {
        return InteractCategory::Workstation;
    }
    InteractCategory::Other
}

pub fn region_at<'a>(store: &'a RegionStore, world: &str, x: i32, y: i32, z: i32) -> Option<&'a Region> {
    store
        .regions
        .values()
        .find(|r| r.bounds.contains(world, x, y, z))
}

pub fn find_overlap<'a>(
    store: &'a RegionStore,
    candidate: &RegionBounds,
    ignore_id: Option<&str>,
) -> Option<&'a Region> {
    store.regions.values().find(|r| {
        if let Some(skip) = ignore_id {
            if r.id == skip {
                return false;
            }
        }
        r.bounds.intersects(candidate)
    })
}

pub fn has_bypass(store: &RegionStore, config: &RegionsConfig, player_id: &str) -> bool {
    config.rank_for(store.rank_id_of(player_id)).bypass_protection
}

/// Checks if a player may break or place a block at (world, x, y, z).
pub fn can_modify_block(
    store: &RegionStore,
    config: &RegionsConfig,
    player_id: &str,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let Some(region) = region_at(store, world, x, y, z) else {
        return true;
    };
    if region.is_trusted(player_id) || has_bypass(store, config, player_id) {
        return true;
    }
    host::send_action_bar(
        player_id,
        &format!("§c🔒 Территория §e{} §cзащищена (владелец: §f{}§c)", region.name, region.owner_name),
    );
    host::play_sound(player_id, "minecraft:entity.villager.no", 0.55, 1.1);
    false
}

/// Granular per-category interaction check (chests, doors, buttons/levers, workstations).
pub fn can_interact_block(
    store: &RegionStore,
    config: &RegionsConfig,
    player_id: &str,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
    block_id: &str,
) -> bool {
    let Some(region) = region_at(store, world, x, y, z) else {
        return true;
    };
    if region.is_trusted(player_id) || has_bypass(store, config, player_id) {
        return true;
    }

    let (allowed, label) = match classify_block(block_id) {
        InteractCategory::Container => (region.flags.open_chests, "сундуки и хранилища"),
        InteractCategory::Door => (region.flags.use_doors, "двери и люки"),
        InteractCategory::Redstone => (region.flags.use_buttons_levers, "кнопки и рычаги"),
        InteractCategory::Workstation => (region.flags.use_workstations, "верстаки и печи"),
        InteractCategory::Other => (true, ""),
    };

    if !allowed {
        host::send_action_bar(
            player_id,
            &format!("§c🔒 В привате §e{} §cзапрещено использовать {}", region.name, label),
        );
        host::play_sound(player_id, "minecraft:block.chest.locked", 0.6, 1.0);
    }
    allowed
}

pub fn can_pvp_damage(
    store: &RegionStore,
    attacker_id: &str,
    world: &str,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    if let Some(region) = region_at(store, world, x, y, z) {
        if !region.flags.pvp {
            host::send_action_bar(
                attacker_id,
                &format!("§b🛡 В регионе §e{} §bотключён PvP!", region.name),
            );
            return false;
        }
    }
    true
}

pub fn can_fall_damage(store: &RegionStore, world: &str, x: i32, y: i32, z: i32) -> bool {
    region_at(store, world, x, y, z)
        .map(|r| r.flags.fall_damage)
        .unwrap_or(true)
}

pub fn can_explode(store: &RegionStore, world: &str, x: i32, y: i32, z: i32) -> bool {
    region_at(store, world, x, y, z)
        .map(|r| r.flags.explosions)
        .unwrap_or(true)
}

pub fn can_mob_spawn(store: &RegionStore, world: &str, x: i32, y: i32, z: i32, hostile: bool) -> bool {
    if !hostile {
        return true;
    }
    region_at(store, world, x, y, z)
        .map(|r| r.flags.mob_spawning)
        .unwrap_or(true)
}

pub fn can_fire_spread(store: &RegionStore, world: &str, x: i32, y: i32, z: i32) -> bool {
    region_at(store, world, x, y, z)
        .map(|r| r.flags.fire_spread)
        .unwrap_or(true)
}
`,

    'src/storage.rs': `use crate::models::{RankPolicy, RegionStore, RegionsConfig};
use log::{info, warn};
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data/kronos-regions";
const CONFIG_PATH: &str = "data/kronos-regions/config.json";
const STORE_PATH: &str = "data/kronos-regions/regions.json";
const STORE_TMP_PATH: &str = "data/kronos-regions/regions.json.tmp";

pub fn default_config() -> RegionsConfig {
    RegionsConfig {
        plugin_name: "kronos-regions".to_string(),
        wand_item_id: "minecraft:golden_axe".to_string(),
        wand_display_name: "§6✦ Топор Привата KRONOS".to_string(),
        wand_tag: "kronos_region_wand".to_string(),
        min_region_side: 3,
        world_min_y: -64,
        world_max_y: 319,
        visualizer_duration_seconds: 25,
        visualizer_step_blocks: 1.5,
        max_particles_per_tick: 140,
        autosave_interval_seconds: 30,
        ranks: vec![
            RankPolicy {
                id: "player".to_string(),
                tier: 0,
                display_prefix: "§7[ИГРОК] ".to_string(),
                max_regions: 2,
                max_volume: 40_000,
                max_members: 3,
                default_border_theme: "emerald_wire".to_string(),
                can_toggle_pvp: false,
                can_toggle_fall_damage: false,
                can_toggle_mob_spawning: false,
                can_toggle_entry: false,
                can_set_greeting: false,
                unlimited_regions: false,
                bypass_protection: false,
            },
            RankPolicy {
                id: "vip".to_string(),
                tier: 1,
                display_prefix: "§e§l[VIP] ".to_string(),
                max_regions: 4,
                max_volume: 120_000,
                max_members: 6,
                default_border_theme: "golden_beam".to_string(),
                can_toggle_pvp: true,
                can_toggle_fall_damage: false,
                can_toggle_mob_spawning: true,
                can_toggle_entry: false,
                can_set_greeting: true,
                unlimited_regions: false,
                bypass_protection: false,
            },
            RankPolicy {
                id: "premium".to_string(),
                tier: 2,
                display_prefix: "§a§l[PREMIUM] ".to_string(),
                max_regions: 6,
                max_volume: 300_000,
                max_members: 10,
                default_border_theme: "amethyst_grid".to_string(),
                can_toggle_pvp: true,
                can_toggle_fall_damage: true,
                can_toggle_mob_spawning: true,
                can_toggle_entry: false,
                can_set_greeting: true,
                unlimited_regions: false,
                bypass_protection: false,
            },
            RankPolicy {
                id: "legend".to_string(),
                tier: 3,
                display_prefix: "§d§l[LEGEND] ".to_string(),
                max_regions: 10,
                max_volume: 750_000,
                max_members: 20,
                default_border_theme: "dragon_aurora".to_string(),
                can_toggle_pvp: true,
                can_toggle_fall_damage: true,
                can_toggle_mob_spawning: true,
                can_toggle_entry: true,
                can_set_greeting: true,
                unlimited_regions: false,
                bypass_protection: false,
            },
            RankPolicy {
                id: "admin".to_string(),
                tier: 4,
                display_prefix: "§c§l[ADMIN] ".to_string(),
                max_regions: 50,
                max_volume: 50_000_000,
                max_members: 100,
                default_border_theme: "kronos_prism".to_string(),
                can_toggle_pvp: true,
                can_toggle_fall_damage: true,
                can_toggle_mob_spawning: true,
                can_toggle_entry: true,
                can_set_greeting: true,
                unlimited_regions: true,
                bypass_protection: true,
            },
            RankPolicy {
                id: "owner".to_string(),
                tier: 5,
                display_prefix: "§4§l[OWNER] ".to_string(),
                max_regions: 100,
                max_volume: 50_000_000,
                max_members: 100,
                default_border_theme: "kronos_prism".to_string(),
                can_toggle_pvp: true,
                can_toggle_fall_damage: true,
                can_toggle_mob_spawning: true,
                can_toggle_entry: true,
                can_set_greeting: true,
                unlimited_regions: true,
                bypass_protection: true,
            },
        ],
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
`,

    'src/menu.rs': `use crate::host;
use crate::models::{
    ActiveBorderViz, ChestMenuDescriptor, ExpandDirection, MemberRole, MenuSlot, RegionBounds,
    RegionStore, RegionsConfig, WandSelection,
};
use crate::protection;
use crate::visualizer::{self, BORDER_THEMES};
use std::collections::HashMap;

const LIST_SLOTS: [u8; 14] = [
    19, 20, 21, 22, 23, 24, 25,
    28, 29, 30, 31, 32, 33, 34,
];

fn flag_badge(enabled: bool) -> &'static str {
    if enabled { "§a✔ РАЗРЕШЕНО" } else { "§c✖ ЗАПРЕЩЕНО" }
}

/// 1. Main 54-slot My Regions Hub (/rg)
pub fn open_regions_hub(
    store: &RegionStore,
    config: &RegionsConfig,
    selections: &HashMap<String, WandSelection>,
    player_id: &str,
) {
    let rank = config.rank_for(store.rank_id_of(player_id));
    let owned_count = store.owned_count(player_id);
    let limit_txt = if rank.unlimited_regions { "∞".to_string() } else { rank.max_regions.to_string() };

    let mut slots: Vec<MenuSlot> = Vec::with_capacity(54);
    for &b in &[0u8, 1, 2, 3, 5, 6, 7, 8, 9, 17, 18, 26, 27, 35, 36, 44, 45, 53] {
        let corner = matches!(b, 0 | 8 | 45 | 53);
        slots.push(MenuSlot {
            slot: b,
            item: if corner {
                "minecraft:orange_stained_glass_pane".to_string()
            } else {
                "minecraft:black_stained_glass_pane".to_string()
            },
            name: "§8".to_string(),
            lore: vec![],
            enchanted: false,
        });
    }

    // Status Card (slot 4)
    let next_rank_hint = match config.next_rank_after(rank.tier) {
        Some(next) if !rank.unlimited_regions => format!(
            "§7Следующий ранг {}: §e{} приватов §7(до §e{} §7бл.)",
            next.display_prefix.trim(),
            next.max_regions,
            next.max_volume
        ),
        _ => "§aМаксимальные привилегии привата активны!".to_string(),
    };

    slots.push(MenuSlot {
        slot: 4,
        item: "minecraft:shield".to_string(),
        name: format!("{}§fВаши приваты KRONOS", rank.display_prefix),
        lore: vec![
            format!("§7Приваты: §e{} §8/ §a{}", owned_count, limit_txt),
            format!("§7Макс. объём: §b{} блоков", rank.max_volume),
            format!("§7Макс. друзей в привате: §e{}", rank.max_members),
            format!("§7Отключение PvP: {}", if rank.can_toggle_pvp { "§a✔ Доступно" } else { "§e🔒 С VIP" }),
            format!("§7Отключение урона от падения: {}", if rank.can_toggle_fall_damage { "§a✔ Доступно" } else { "§a🔒 С PREMIUM" }),
            "§8────────────────────────".to_string(),
            next_rank_hint,
        ],
        enchanted: true,
    });

    // Player's owned & member regions
    let mut player_regions: Vec<_> = store
        .regions
        .values()
        .filter(|r| r.is_trusted(player_id))
        .collect();
    player_regions.sort_by(|a, b| a.name.cmp(&b.name));

    for (idx, &slot_num) in LIST_SLOTS.iter().enumerate() {
        if let Some(rg) = player_regions.get(idx) {
            let (dx, dy, dz) = rg.bounds.dimensions();
            let role_txt = if rg.is_owner(player_id) {
                "§6Владелец"
            } else if rg.is_officer_or_owner(player_id) {
                "§bОфицер"
            } else {
                "§7Участник"
            };
            slots.push(MenuSlot {
                slot: slot_num,
                item: if rg.is_owner(player_id) {
                    "minecraft:grass_block".to_string()
                } else {
                    "minecraft:oak_planks".to_string()
                },
                name: format!("§6✦ Приват: §f{}", rg.name),
                lore: vec![
                    format!("§7Роль: {}", role_txt),
                    format!("§7Размер: §e{}×{}×{} §8({} бл.)", dx, dy, dz, rg.bounds.volume()),
                    format!("§7Участников: §b{}", rg.members.len()),
                    format!("§7Взрывы: {} §8| §7PvP: {}", flag_badge(rg.flags.explosions), flag_badge(rg.flags.pvp)),
                    "§8────────────────────────".to_string(),
                    "§a▸ ЛКМ §8— §fПанель управления и флаги".to_string(),
                    "§b▸ ПКМ §8— §fПодсветить 3D-границы".to_string(),
                ],
                enchanted: rg.is_owner(player_id),
            });
        } else if rank.unlimited_regions || idx < rank.max_regions {
            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:lime_stained_glass_pane".to_string(),
                name: format!("§a+ Свободный слот привата #{}", idx + 1),
                lore: vec![
                    "§7Возьмите §6Топор Привата§7 (кнопка снизу),".to_string(),
                    "§7выделите §aЛКМ§7 и §eПКМ§7 два угла".to_string(),
                    "§7и введите §f/rg claim <название>".to_string(),
                ],
                enchanted: false,
            });
        } else {
            let unlock = config
                .ranks
                .iter()
                .find(|r| r.unlimited_regions || r.max_regions > idx)
                .map(|r| r.display_prefix.trim().to_string())
                .unwrap_or_else(|| "§d[LEGEND]".to_string());
            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:gray_stained_glass_pane".to_string(),
                name: "§8🔒 Заблокированный приват".to_string(),
                lore: vec![
                    format!("§7Открывается со статуса {}", unlock),
                    "§7Больше территорий + PvP/Падение/Аврора!".to_string(),
                ],
                enchanted: false,
            });
        }
    }

    // Check if player has a ready wand selection
    let sel_ready = selections
        .get(player_id)
        .and_then(|s| match (&s.pos1, &s.pos2) {
            (Some(a), Some(b)) => RegionBounds::from_points(a, b),
            _ => None,
        });

    slots.push(MenuSlot {
        slot: 47,
        item: config.wand_item_id.clone(),
        name: "§6✦ Получить Топор Привата".to_string(),
        lore: vec![
            "§aЛКМ §7— точка #1 • §eПКМ §7— точка #2".to_string(),
            "§7Авто-подсветка 3D-границ при выделении!".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 48,
        item: if sel_ready.is_some() {
            "minecraft:emerald_block".to_string()
        } else {
            "minecraft:coal_block".to_string()
        },
        name: if sel_ready.is_some() {
            "§a✔ Быстро создать приват из выделения".to_string()
        } else {
            "§7Выделите 2 точки Топором Привата".to_string()
        },
        lore: match sel_ready {
            Some(ref b) => vec![
                format!("§7Выделено: §e{} блоков", b.volume()),
                "§a▸ Нажмите для мгновенного создания!".to_string(),
            ],
            None => vec!["§8Точки #1 и #2 ещё не заданы".to_string()],
        },
        enchanted: sel_ready.is_some(),
    });

    slots.push(MenuSlot {
        slot: 50,
        item: "minecraft:spyglass".to_string(),
        name: "§b✦ Показать границы рядом (/rg show)".to_string(),
        lore: vec!["§7Включает 3D-подсветку текущего привата на 25с".to_string()],
        enchanted: false,
    });

    slots.push(MenuSlot {
        slot: 51,
        item: "minecraft:barrier".to_string(),
        name: "§cЗакрыть меню".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "rg_list".to_string(),
            title: format!("§8✦ §6KRONOS Приваты §8[§e{}/{}§8]", owned_count, limit_txt),
            rows: 6,
            slots,
        },
    );
}

/// 2. Region Control Panel (54 slots)
pub fn open_region_manage(store: &RegionStore, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let (dx, dy, dz) = rg.bounds.dimensions();
    let theme = visualizer::find_border_theme(&rg.border_theme);

    let slots = vec![
        MenuSlot {
            slot: 4,
            item: "minecraft:beacon".to_string(),
            name: format!("§6✦ Приват: §f{}", rg.name),
            lore: vec![
                format!("§7Владелец: §e{}", rg.owner_name),
                format!("§7Размер: §b{}×{}×{} §8({} блоков)", dx, dy, dz, rg.bounds.volume()),
                format!("§7Стиль границ: {}", theme.label),
            ],
            enchanted: true,
        },
        MenuSlot {
            slot: 20,
            item: "minecraft:comparator".to_string(),
            name: "§e✦ Флаги и защита (Конфиг)".to_string(),
            lore: vec![
                "§7Отдельная настройка каждого флага:".to_string(),
                "§7• Взрывы, PvP, Урон от падения".to_string(),
                "§7• Сундуки, Двери, Кнопки, Верстаки, Мобы".to_string(),
            ],
            enchanted: true,
        },
        MenuSlot {
            slot: 21,
            item: "minecraft:piston".to_string(),
            name: "§a✦ Расширить / Сжать границы".to_string(),
            lore: vec![
                "§7Сдвинуть стену на Север, Юг, Запад, Восток,".to_string(),
                "§7Вверх, Вниз или от Бедрока до Неба!".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 22,
            item: "minecraft:player_head".to_string(),
            name: format!("§b✦ Участники ({})", rg.members.len()),
            lore: vec![
                "§7Управление друзьями и ролями Офицеров".to_string(),
                "§8Добавить: §f/rg add <приват> <ник>".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 23,
            item: "minecraft:ender_eye".to_string(),
            name: "§d✦ Подсветить 3D-границы (25с)".to_string(),
            lore: vec![format!("§7Эффект: {}", theme.label)],
            enchanted: true,
        },
        MenuSlot {
            slot: 24,
            item: theme.icon.to_string(),
            name: "§d✦ Стиль частиц границ".to_string(),
            lore: vec![
                format!("§7Текущий: {}", theme.label),
                "§e▸ Выбрать свечение по статусу".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 30,
            item: "minecraft:name_tag".to_string(),
            name: "§f✦ Переименовать приват".to_string(),
            lore: vec![format!("§7Команда: §e/rg rename {} <новое_имя>", rg.name)],
            enchanted: false,
        },
        MenuSlot {
            slot: 31,
            item: "minecraft:ender_pearl".to_string(),
            name: "§b✦ Телепорт в центр привата".to_string(),
            lore: vec!["§7Быстрый переход на вашу территорию".to_string()],
            enchanted: false,
        },
        MenuSlot {
            slot: 32,
            item: "minecraft:tnt".to_string(),
            name: "§c✖ Удалить приват".to_string(),
            lore: vec!["§7Откроет окно подтверждения удаления".to_string()],
            enchanted: false,
        },
        MenuSlot {
            slot: 49,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ К списку приватов".to_string(),
            lore: vec![],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_manage:{}", rg.id),
            title: format!("§8✦ Управление: §6{}", rg.name),
            rows: 6,
            slots,
        },
    );
}

/// 3. Granular Flags Toggle Board (54 slots)
pub fn open_flags_menu(store: &RegionStore, config: &RegionsConfig, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let rank = config.rank_for(store.rank_id_of(player_id));
    let f = &rg.flags;

    let make_flag = |slot: u8, item: &str, title: &str, enabled: bool, unlocked: bool, lock_hint: &str, desc: &str| {
        MenuSlot {
            slot,
            item: if unlocked { item.to_string() } else { "minecraft:iron_bars".to_string() },
            name: format!("§f{} §8— {}", title, flag_badge(enabled)),
            lore: vec![
                format!("§7{}", desc),
                "§8────────────────────────".to_string(),
                if unlocked {
                    "§e▸ Нажмите, чтобы переключить".to_string()
                } else {
                    format!("§c🔒 Настройка доступна со статуса {}", lock_hint)
                },
            ],
            enchanted: enabled && unlocked,
        }
    };

    let slots = vec![
        make_flag(10, "minecraft:tnt", "Взрывы (TNT / Криперы)", f.explosions, true, "", "Разрушение территории взрывами"),
        make_flag(11, "minecraft:diamond_sword", "PvP / Убийство игроков", f.pvp, rank.can_toggle_pvp, "§e[VIP]", "Урон от игроков внутри привата"),
        make_flag(12, "minecraft:feather", "Урон от падения", f.fall_damage, rank.can_toggle_fall_damage, "§a[PREMIUM]", "Получение урона при падении с высоты"),
        make_flag(13, "minecraft:zombie_head", "Спавн враждебных мобов", f.mob_spawning, rank.can_toggle_mob_spawning, "§e[VIP]", "Появление криперов, зомби и скелетов"),
        make_flag(14, "minecraft:flint_and_steel", "Распространение огня", f.fire_spread, true, "", "Горение построек и растекание огня"),
        make_flag(15, "minecraft:oak_fence_gate", "Вход чужаков", f.entry_allowed, rank.can_toggle_entry, "§d[LEGEND]", "Доступ посторонних на территорию"),
        make_flag(16, "minecraft:hopper", "Подбор дропа чужаками", f.item_pickup, true, "", "Могут ли чужаки подбирать выпавшие вещи"),

        // Individual interaction toggles
        make_flag(29, "minecraft:chest", "Открытие сундуков / бочек", f.open_chests, true, "", "Сундуки, бочки, шалкеры, воронки, печи"),
        make_flag(30, "minecraft:oak_door", "Двери, люки и калитки", f.use_doors, true, "", "Открывание дверей и люков чужаками"),
        make_flag(31, "minecraft:stone_button", "Кнопки, рычаги и плиты", f.use_buttons_levers, true, "", "Использование редстоун-переключателей"),
        make_flag(32, "minecraft:crafting_table", "Верстаки и наковальни", f.use_workstations, true, "", "Доступ к верстакам, столам зачарования"),

        MenuSlot {
            slot: 49,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Назад к управлению".to_string(),
            lore: vec![],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_flags:{}", rg.id),
            title: format!("§8✦ Флаги привата: §e{}", rg.name),
            rows: 6,
            slots,
        },
    );
}

/// 4. Interactive Compass Boundary Expander GUI (54 slots)
pub fn open_expand_menu(store: &RegionStore, config: &RegionsConfig, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let rank = config.rank_for(store.rank_id_of(player_id));
    let (dx, dy, dz) = rg.bounds.dimensions();

    let dir_btn = |slot: u8, item: &str, title: &str, extra: &str| MenuSlot {
        slot,
        item: item.to_string(),
        name: title.to_string(),
        lore: vec![
            format!("§7{}", extra),
            "§a▸ ЛКМ §8— §fРасширить на +5 блоков".to_string(),
            "§e▸ Shift+ЛКМ §8— §fРасширить на +10 блоков".to_string(),
            "§c▸ ПКМ §8— §fСжать на -5 блоков".to_string(),
        ],
        enchanted: false,
    };

    let slots = vec![
        MenuSlot {
            slot: 4,
            item: "minecraft:compass".to_string(),
            name: format!("§6✦ Границы: §f{}×{}×{}", dx, dy, dz),
            lore: vec![
                format!("§7Текущий объём: §e{} §8/ §a{}", rg.bounds.volume(), rank.max_volume),
                "§7После сдвига сразу включится 3D-подсветка!".to_string(),
            ],
            enchanted: true,
        },
        dir_btn(13, "minecraft:lime_banner", "§a▲ Север (-Z)", "Расширить в северную сторону"),
        dir_btn(21, "minecraft:lime_banner", "§a◀ Запад (-X)", "Расширить в западную сторону"),
        dir_btn(22, "minecraft:emerald_block", "§b✦ Во все стороны (Горизонталь)", "Сразу по X и Z одновременно"),
        dir_btn(23, "minecraft:lime_banner", "§a▶ Восток (+X)", "Расширить в восточную сторону"),
        dir_btn(31, "minecraft:lime_banner", "§a▼ Юг (+Z)", "Расширить в южную сторону"),

        dir_btn(15, "minecraft:feather", "§e⬆ Вверх (+Y)", "Поднять потолок привата"),
        dir_btn(33, "minecraft:anvil", "§e⬇ Вниз (-Y)", "Опустить пол привата"),
        MenuSlot {
            slot: 25,
            item: "minecraft:bedrock".to_string(),
            name: "§6✦ От Бедрока до Неба (Vert)".to_string(),
            lore: vec![
                format!("§7Растянет Y от {} до {}", config.world_min_y, config.world_max_y),
                "§e▸ Нажмите для расширения по высоте".to_string(),
            ],
            enchanted: true,
        },
        MenuSlot {
            slot: 49,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Назад к управлению".to_string(),
            lore: vec![],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_expand:{}", rg.id),
            title: format!("§8✦ Границы привата: §a{}", rg.name),
            rows: 6,
            slots,
        },
    );
}

/// 5. Members & Roles GUI (54 slots)
pub fn open_members_menu(store: &RegionStore, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let mut members: Vec<(&String, &MemberRole)> = rg.members.iter().collect();
    members.sort_by(|a, b| a.0.cmp(b.0));

    let mut slots: Vec<MenuSlot> = Vec::new();
    for (idx, (member_id, role)) in members.iter().take(LIST_SLOTS.len()).enumerate() {
        let is_officer = matches!(role, MemberRole::Officer);
        slots.push(MenuSlot {
            slot: LIST_SLOTS[idx],
            item: if is_officer {
                "minecraft:golden_helmet".to_string()
            } else {
                "minecraft:player_head".to_string()
            },
            name: format!("§f{} §8[{}§8]", member_id, if is_officer { "§bОфицер" } else { "§7Участник" }),
            lore: vec![
                "§a▸ ЛКМ §8— §fПереключить роль (Участник ↔ Офицер)".to_string(),
                "§c▸ ПКМ §8— §fИсключить из привата".to_string(),
            ],
            enchanted: is_officer,
        });
    }

    slots.push(MenuSlot {
        slot: 4,
        item: "minecraft:writable_book".to_string(),
        name: "§eДобавить друга в приват".to_string(),
        lore: vec![format!("§7Введите в чат: §f/rg add {} <ник>", rg.name)],
        enchanted: false,
    });
    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к управлению".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_members:{}", rg.id),
            title: format!("§8✦ Участники: §b{}", rg.name),
            rows: 6,
            slots,
        },
    );
}

/// 6. Border Themes Catalog (27 slots) & Confirm Delete Modal (27 slots)
pub fn open_themes_menu(store: &RegionStore, config: &RegionsConfig, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let rank = config.rank_for(store.rank_id_of(player_id));
    let theme_slots: [u8; 5] = [11, 12, 13, 14, 15];

    let mut slots: Vec<MenuSlot> = Vec::new();
    for (idx, theme) in BORDER_THEMES.iter().enumerate() {
        let unlocked = visualizer::can_use_border_theme(rank.tier, theme.id);
        let selected = rg.border_theme == theme.id;
        slots.push(MenuSlot {
            slot: theme_slots[idx],
            item: if unlocked { theme.icon.to_string() } else { "minecraft:gray_dye".to_string() },
            name: if selected { format!("{} §a✔", theme.label) } else { theme.label.to_string() },
            lore: vec![
                format!("§7{}", theme.description),
                if selected {
                    "§aАктивно сейчас".to_string()
                } else if unlocked {
                    "§e▸ Нажмите, чтобы применить".to_string()
                } else {
                    format!("§c🔒 Доступно со статуса {}", config.rank_label_for_tier(theme.min_tier))
                },
            ],
            enchanted: selected,
        });
    }
    slots.push(MenuSlot {
        slot: 22,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_themes:{}", rg.id),
            title: "§8✦ §dСтиль границ привата".to_string(),
            rows: 3,
            slots,
        },
    );
}

pub fn open_confirm_delete(store: &RegionStore, player_id: &str, rg_id: &str) {
    let Some(rg) = store.regions.get(rg_id) else {
        return;
    };
    let slots = vec![
        MenuSlot {
            slot: 11,
            item: "minecraft:lime_concrete".to_string(),
            name: "§a✔ Подтвердить удаление".to_string(),
            lore: vec![format!("§7Снять защиту с §e{} §7навсегда", rg.name)],
            enchanted: false,
        },
        MenuSlot {
            slot: 13,
            item: "minecraft:tnt".to_string(),
            name: format!("§cУдалить приват {}?", rg.name),
            lore: vec![format!("§7Объём: §e{} блоков", rg.bounds.volume())],
            enchanted: true,
        },
        MenuSlot {
            slot: 15,
            item: "minecraft:red_concrete".to_string(),
            name: "§c✖ Отмена".to_string(),
            lore: vec!["§7Оставить территорию под защитой".to_string()],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("rg_confirm_delete:{}", rg.id),
            title: format!("§cУдаление: {}", rg.name),
            rows: 3,
            slots,
        },
    );
}

pub struct RegionMenuCtx<'a> {
    pub store: &'a mut RegionStore,
    pub config: &'a RegionsConfig,
    pub selections: &'a mut HashMap<String, WandSelection>,
    pub active_viz: &'a mut HashMap<String, ActiveBorderViz>,
    pub dirty: &'a mut bool,
}

pub fn handle_menu_click(
    ctx: &mut RegionMenuCtx<'_>,
    player_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    try_claim: impl Fn(&mut RegionStore, &RegionsConfig, &mut HashMap<String, WandSelection>, &mut HashMap<String, ActiveBorderViz>, &mut bool, &str, &str),
    try_resize: impl Fn(&mut RegionStore, &RegionsConfig, &mut HashMap<String, ActiveBorderViz>, &mut bool, &str, &str, ExpandDirection, i32),
) -> bool {
    let now = host::now_ms();

    if menu_id == "rg_list" {
        match slot {
            47 => {
                host::close_menu(player_id);
                host::give_claim_wand(
                    player_id,
                    &ctx.config.wand_item_id,
                    &ctx.config.wand_display_name,
                    &ctx.config.wand_tag,
                );
                host::send_chat(player_id, "§6✦ §fВыдан §6Топор Привата§f! ЛКМ — точка #1, ПКМ — точка #2.");
                host::play_sound(player_id, "minecraft:entity.item.pickup", 0.8, 1.2);
                return true;
            }
            48 => {
                let auto_name = format!("base{}", ctx.store.owned_count(player_id) + 1);
                host::close_menu(player_id);
                try_claim(ctx.store, ctx.config, ctx.selections, ctx.active_viz, ctx.dirty, player_id, &auto_name);
                return true;
            }
            50 => {
                host::close_menu(player_id);
                let pos = host::get_player_block_pos(player_id);
                if let Some(rg) = protection::region_at(ctx.store, &pos.world, pos.x, pos.y, pos.z) {
                    let viz = visualizer::build_viz(player_id, rg.bounds.clone(), &rg.border_theme, false, ctx.config, now);
                    ctx.active_viz.insert(player_id.to_string(), viz);
                    host::send_chat(player_id, &format!("§a✦ 3D-границы привата §e{} §aвключены на 25с!", rg.name));
                } else {
                    host::send_chat(player_id, "§7Вы не находитесь внутри привата.");
                }
                return true;
            }
            51 => {
                host::close_menu(player_id);
                return true;
            }
            _ => {}
        }

        if let Some(idx) = LIST_SLOTS.iter().position(|&s| s == slot) {
            let mut rgs: Vec<_> = ctx
                .store
                .regions
                .values()
                .filter(|r| r.is_trusted(player_id))
                .cloned()
                .collect();
            rgs.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(rg) = rgs.get(idx) {
                if click_type == "right" {
                    host::close_menu(player_id);
                    let viz = visualizer::build_viz(player_id, rg.bounds.clone(), &rg.border_theme, false, ctx.config, now);
                    ctx.active_viz.insert(player_id.to_string(), viz);
                    host::send_chat(player_id, &format!("§b✦ Подсвечены границы §e{}§b!", rg.name));
                } else {
                    open_region_manage(ctx.store, player_id, &rg.id);
                }
            }
        }
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_manage:") {
        match slot {
            20 => open_flags_menu(ctx.store, ctx.config, player_id, rg_id),
            21 => open_expand_menu(ctx.store, ctx.config, player_id, rg_id),
            22 => open_members_menu(ctx.store, player_id, rg_id),
            23 => {
                if let Some(rg) = ctx.store.regions.get(rg_id) {
                    let viz = visualizer::build_viz(player_id, rg.bounds.clone(), &rg.border_theme, false, ctx.config, now);
                    ctx.active_viz.insert(player_id.to_string(), viz);
                    host::close_menu(player_id);
                    host::send_chat(player_id, &format!("§b✦ 3D-границы §e{} §bподсвечены!", rg.name));
                }
            }
            24 => open_themes_menu(ctx.store, ctx.config, player_id, rg_id),
            30 => {
                if let Some(rg) = ctx.store.regions.get(rg_id) {
                    host::close_menu(player_id);
                    host::send_chat(player_id, &format!("§eДля переименования введите: §f/rg rename {} <новое_имя>", rg.name));
                }
            }
            31 => {
                if let Some(rg) = ctx.store.regions.get(rg_id) {
                    let (cx, cy, cz) = rg.bounds.center();
                    host::close_menu(player_id);
                    host::teleport_player(player_id, &rg.bounds.world, cx, cy, cz);
                    host::play_sound(player_id, "minecraft:entity.enderman.teleport", 0.8, 1.1);
                }
            }
            32 => open_confirm_delete(ctx.store, player_id, rg_id),
            49 => open_regions_hub(ctx.store, ctx.config, ctx.selections, player_id),
            _ => {}
        }
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_flags:") {
        if slot == 49 {
            open_region_manage(ctx.store, player_id, rg_id);
            return true;
        }
        let rank = ctx.config.rank_for(ctx.store.rank_id_of(player_id));
        if let Some(rg) = ctx.store.regions.get_mut(rg_id) {
            if !rg.is_officer_or_owner(player_id) && !rank.bypass_protection {
                host::send_chat(player_id, "§c✖ Только владелец или офицер может менять флаги.");
                return true;
            }
            let mut changed = true;
            match slot {
                10 => rg.flags.explosions = !rg.flags.explosions,
                11 if rank.can_toggle_pvp => rg.flags.pvp = !rg.flags.pvp,
                12 if rank.can_toggle_fall_damage => rg.flags.fall_damage = !rg.flags.fall_damage,
                13 if rank.can_toggle_mob_spawning => rg.flags.mob_spawning = !rg.flags.mob_spawning,
                14 => rg.flags.fire_spread = !rg.flags.fire_spread,
                15 if rank.can_toggle_entry => rg.flags.entry_allowed = !rg.flags.entry_allowed,
                16 => rg.flags.item_pickup = !rg.flags.item_pickup,
                29 => rg.flags.open_chests = !rg.flags.open_chests,
                30 => rg.flags.use_doors = !rg.flags.use_doors,
                31 => rg.flags.use_buttons_levers = !rg.flags.use_buttons_levers,
                32 => rg.flags.use_workstations = !rg.flags.use_workstations,
                11 | 12 | 13 | 15 => {
                    changed = false;
                    host::send_chat(player_id, "§c🔒 Этот флаг доступен на более высоком статусе!");
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
                }
                _ => changed = false,
            }
            if changed {
                *ctx.dirty = true;
                host::play_sound(player_id, "minecraft:ui.button.click", 0.7, 1.3);
            }
        }
        open_flags_menu(ctx.store, ctx.config, player_id, rg_id);
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_expand:") {
        if slot == 49 {
            open_region_manage(ctx.store, player_id, rg_id);
            return true;
        }
        let delta = match click_type {
            "right" => -5,
            "shift_left" => 10,
            _ => 5,
        };
        let dir = match slot {
            13 => Some(ExpandDirection::North),
            31 => Some(ExpandDirection::South),
            21 => Some(ExpandDirection::West),
            23 => Some(ExpandDirection::East),
            15 => Some(ExpandDirection::Up),
            33 => Some(ExpandDirection::Down),
            22 => Some(ExpandDirection::All),
            25 => Some(ExpandDirection::Vert),
            _ => None,
        };
        if let Some(d) = dir {
            try_resize(ctx.store, ctx.config, ctx.active_viz, ctx.dirty, player_id, rg_id, d, delta);
            open_expand_menu(ctx.store, ctx.config, player_id, rg_id);
        }
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_members:") {
        if slot == 49 {
            open_region_manage(ctx.store, player_id, rg_id);
            return true;
        }
        if let Some(idx) = LIST_SLOTS.iter().position(|&s| s == slot) {
            if let Some(rg) = ctx.store.regions.get_mut(rg_id) {
                if !rg.is_owner(player_id) {
                    host::send_chat(player_id, "§c✖ Только владелец может менять состав участников.");
                    return true;
                }
                let mut names: Vec<String> = rg.members.keys().cloned().collect();
                names.sort();
                if let Some(target) = names.get(idx).cloned() {
                    if click_type == "right" {
                        rg.members.remove(&target);
                        *ctx.dirty = true;
                        host::send_chat(player_id, &format!("§c✖ Игрок §e{} §cудалён из привата.", target));
                    } else if let Some(role) = rg.members.get_mut(&target) {
                        *role = match *role {
                            MemberRole::Member => MemberRole::Officer,
                            MemberRole::Officer => MemberRole::Member,
                        };
                        *ctx.dirty = true;
                    }
                    open_members_menu(ctx.store, player_id, rg_id);
                }
            }
        }
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_themes:") {
        if slot == 22 {
            open_region_manage(ctx.store, player_id, rg_id);
            return true;
        }
        let theme_slots: [u8; 5] = [11, 12, 13, 14, 15];
        if let Some(idx) = theme_slots.iter().position(|&s| s == slot) {
            let theme = &BORDER_THEMES[idx];
            let rank = ctx.config.rank_for(ctx.store.rank_id_of(player_id));
            if !visualizer::can_use_border_theme(rank.tier, theme.id) {
                host::send_chat(
                    player_id,
                    &format!("§c🔒 Стиль {} §cдоступен со статуса {}", theme.label, ctx.config.rank_label_for_tier(theme.min_tier)),
                );
                host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
            } else if let Some(rg) = ctx.store.regions.get_mut(rg_id) {
                rg.border_theme = theme.id.to_string();
                *ctx.dirty = true;
                let viz = visualizer::build_viz(player_id, rg.bounds.clone(), &rg.border_theme, false, ctx.config, now);
                ctx.active_viz.insert(player_id.to_string(), viz);
                host::send_chat(player_id, &format!("§6✦ Стиль границ обновлён: {}", theme.label));
            }
            open_themes_menu(ctx.store, ctx.config, player_id, rg_id);
        }
        return true;
    }

    if let Some(rg_id) = menu_id.strip_prefix("rg_confirm_delete:") {
        match slot {
            11 => {
                if let Some(rg) = ctx.store.regions.get(rg_id) {
                    let rank = ctx.config.rank_for(ctx.store.rank_id_of(player_id));
                    if rg.is_owner(player_id) || rank.bypass_protection {
                        let name = rg.name.clone();
                        ctx.store.regions.remove(rg_id);
                        ctx.active_viz.remove(player_id);
                        *ctx.dirty = true;
                        host::send_chat(player_id, &format!("§c✖ Приват §e{} §cудалён.", name));
                        host::play_sound(player_id, "minecraft:entity.item.break", 0.8, 0.9);
                    }
                }
                open_regions_hub(ctx.store, ctx.config, ctx.selections, player_id);
            }
            15 => open_region_manage(ctx.store, player_id, rg_id),
            _ => {}
        }
        return true;
    }

    false
}
`,

    'src/commands.rs': `use crate::host;
use crate::menu;
use crate::models::{
    ActiveBorderViz, BlockPos, ExpandDirection, MemberRole, Region, RegionBounds, RegionFlags,
    RegionStore, RegionsConfig, WandSelection,
};
use crate::protection;
use crate::visualizer;
use std::collections::HashMap;

pub fn handle_wand_click(
    store: &RegionStore,
    config: &RegionsConfig,
    selections: &mut HashMap<String, WandSelection>,
    active_viz: &mut HashMap<String, ActiveBorderViz>,
    player_id: &str,
    click_type: &str,
    pos: BlockPos,
) -> bool {
    if click_type == "shift_right" {
        menu::open_regions_hub(store, config, selections, player_id);
        return true;
    }

    let now = host::now_ms();
    let sel = selections.entry(player_id.to_string()).or_default();
    sel.updated_at_ms = now;

    if click_type == "left" {
        sel.pos1 = Some(pos.clone());
        visualizer::spawn_point_beacon(&pos, true);
        host::play_sound(player_id, "minecraft:block.note_block.pling", 0.8, 1.2);
        host::send_chat(
            player_id,
            &format!("§a✦ Точка #1 установлена: §f({}, {}, {})", pos.x, pos.y, pos.z),
        );
    } else {
        sel.pos2 = Some(pos.clone());
        visualizer::spawn_point_beacon(&pos, false);
        host::play_sound(player_id, "minecraft:block.note_block.pling", 0.8, 1.5);
        host::send_chat(
            player_id,
            &format!("§e✦ Точка #2 установлена: §f({}, {}, {})", pos.x, pos.y, pos.z),
        );
    }

    // Auto 3D preview when both points are set
    if let (Some(p1), Some(p2)) = (&sel.pos1, &sel.pos2) {
        if let Some(bounds) = RegionBounds::from_points(p1, p2) {
            let rank = config.rank_for(store.rank_id_of(player_id));
            let overlap = protection::find_overlap(store, &bounds, None);
            let vol = bounds.volume();
            let (dx, dy, dz) = bounds.dimensions();
            let has_conflict = overlap.is_some() || vol > rank.max_volume;

            let viz = visualizer::build_viz(
                player_id,
                bounds,
                &rank.default_border_theme,
                has_conflict,
                config,
                now,
            );
            active_viz.insert(player_id.to_string(), viz);

            if let Some(other) = overlap {
                host::send_action_bar(
                    player_id,
                    &format!("§c✖ Пересечение с чужим приватом §e{}§c!", other.name),
                );
            } else if vol > rank.max_volume {
                host::send_action_bar(
                    player_id,
                    &format!("§c✖ Превышен объём: §e{} §c/ {} бл. (повысьте ранг!)", vol, rank.max_volume),
                );
            } else {
                host::send_action_bar(
                    player_id,
                    &format!("§a✔ Выделено {}×{}×{} ({} бл.) — введите §e/rg claim <имя>", dx, dy, dz, vol),
                );
            }
        } else {
            host::send_chat(player_id, "§c✖ Точки #1 и #2 находятся в разных мирах!");
        }
    }

    true
}

pub fn try_claim_selection(
    store: &mut RegionStore,
    config: &RegionsConfig,
    selections: &mut HashMap<String, WandSelection>,
    active_viz: &mut HashMap<String, ActiveBorderViz>,
    dirty: &mut bool,
    player_id: &str,
    raw_name: &str,
) {
    let name = raw_name.trim().to_string();
    if name.is_empty() || name.len() > 24 || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
        host::send_chat(player_id, "§c✖ Имя привата: 1–24 символа (буквы, цифры, _ и -).");
        return;
    }

    if store.find_by_name_or_id(&name).is_some() {
        host::send_chat(player_id, &format!("§c✖ Приват с именем §e{} §cуже существует.", name));
        return;
    }

    let rank = config.rank_for(store.rank_id_of(player_id));
    let owned = store.owned_count(player_id);
    if !rank.unlimited_regions && owned >= rank.max_regions {
        let next_hint = config
            .next_rank_after(rank.tier)
            .map(|n| format!(" Статус {} открывает до {} приватов!", n.display_prefix.trim(), n.max_regions))
            .unwrap_or_default();
        host::send_chat(
            player_id,
            &format!("§c✖ Лимит приватов исчерпан ({}/{}).{}", owned, rank.max_regions, next_hint),
        );
        return;
    }

    let bounds = selections
        .get(player_id)
        .and_then(|s| match (&s.pos1, &s.pos2) {
            (Some(a), Some(b)) => RegionBounds::from_points(a, b),
            _ => None,
        });
    let Some(bounds) = bounds else {
        host::send_chat(player_id, "§c✖ Сначала выделите 2 угла Топором Привата (§e/rg wand§c)!");
        return;
    };

    if !bounds.meets_min_side(config.min_region_side) {
        host::send_chat(
            player_id,
            &format!("§c✖ Минимальный размер каждой стороны — {} блока.", config.min_region_side),
        );
        return;
    }

    if bounds.volume() > rank.max_volume {
        host::send_chat(
            player_id,
            &format!("§c✖ Объём {} блоков превышает ваш лимит ({}).", bounds.volume(), rank.max_volume),
        );
        return;
    }

    if let Some(other) = protection::find_overlap(store, &bounds, None) {
        host::send_chat(
            player_id,
            &format!("§c✖ Область пересекается с приватом §e{} §c(владелец: {}).", other.name, other.owner_name),
        );
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.85, 0.9);
        return;
    }

    let now = host::now_ms();
    store.next_id_counter += 1;
    let id = format!("rg_{}", store.next_id_counter);
    let owner_name = store
        .player_names
        .get(player_id)
        .cloned()
        .unwrap_or_else(|| player_id.to_string());

    let region = Region {
        id: id.clone(),
        name: name.clone(),
        owner_id: player_id.to_string(),
        owner_name,
        bounds: bounds.clone(),
        flags: RegionFlags::default(),
        members: HashMap::new(),
        border_theme: rank.default_border_theme.clone(),
        greeting_title: None,
        created_at_ms: now,
    };

    store.regions.insert(id, region);
    selections.remove(player_id);
    *dirty = true;

    let viz = visualizer::build_viz(player_id, bounds.clone(), &rank.default_border_theme, false, config, now);
    active_viz.insert(player_id.to_string(), viz);

    host::send_title(
        player_id,
        &format!("§a✦ Приват {} создан!", name),
        &format!("§7Защищено §e{} §7блоков • §f/rg", bounds.volume()),
        5,
        40,
        15,
    );
    host::play_sound(player_id, "minecraft:ui.toast.challenge_complete", 0.85, 1.15);
}

pub fn try_resize_region(
    store: &mut RegionStore,
    config: &RegionsConfig,
    active_viz: &mut HashMap<String, ActiveBorderViz>,
    dirty: &mut bool,
    player_id: &str,
    rg_query: &str,
    dir: ExpandDirection,
    delta: i32,
) {
    let Some(rg_id) = store.find_id_by_name_or_id(rg_query) else {
        host::send_chat(player_id, "§c✖ Приват не найден.");
        return;
    };
    let rank = config.rank_for(store.rank_id_of(player_id));
    let Some(current) = store.regions.get(&rg_id).cloned() else {
        return;
    };

    if !current.is_owner(player_id) && !rank.bypass_protection {
        host::send_chat(player_id, "§c✖ Только владелец может изменять границы привата.");
        return;
    }

    let candidate = current.bounds.resized(dir, delta, config.world_min_y, config.world_max_y);
    if !candidate.meets_min_side(config.min_region_side) {
        host::send_chat(player_id, "§c✖ Нельзя сжать приват меньше минимального размера (3×3×3).");
        return;
    }
    if candidate.volume() > rank.max_volume {
        host::send_chat(
            player_id,
            &format!("§c✖ Новый объём ({} бл.) превышает лимит вашего статуса ({} бл.)!", candidate.volume(), rank.max_volume),
        );
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
        return;
    }

    let now = host::now_ms();
    if let Some(other) = protection::find_overlap(store, &candidate, Some(&rg_id)) {
        let viz = visualizer::build_viz(player_id, candidate, &current.border_theme, true, config, now);
        active_viz.insert(player_id.to_string(), viz);
        host::send_chat(player_id, &format!("§c✖ Расширение упирается в чужой приват §e{}§c!", other.name));
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.85, 0.85);
        return;
    }

    if let Some(rg) = store.regions.get_mut(&rg_id) {
        rg.bounds = candidate.clone();
        *dirty = true;
        let viz = visualizer::build_viz(player_id, candidate.clone(), &rg.border_theme, false, config, now);
        active_viz.insert(player_id.to_string(), viz);
        let (dx, dy, dz) = candidate.dimensions();
        host::send_chat(
            player_id,
            &format!("§a✔ Границы §e{} §aизменены ({})! Новый размер: §b{}×{}×{} §8({} бл.)", rg.name, dir.label_ru(), dx, dy, dz, candidate.volume()),
        );
        host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.75, 1.35);
    }
}

pub fn handle_command(
    store: &mut RegionStore,
    config: &RegionsConfig,
    selections: &mut HashMap<String, WandSelection>,
    active_viz: &mut HashMap<String, ActiveBorderViz>,
    dirty: &mut bool,
    player_id: &str,
    command: &str,
) -> bool {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }
    let root = parts[0].to_lowercase();
    if root != "/rg" && root != "/region" && root != "/regions" {
        return false;
    }

    let sub = parts.get(1).map(|s| s.to_lowercase()).unwrap_or_else(|| "menu".to_string());
    match sub.as_str() {
        "menu" | "list" | "gui" => {
            menu::open_regions_hub(store, config, selections, player_id);
        }
        "wand" => {
            host::give_claim_wand(player_id, &config.wand_item_id, &config.wand_display_name, &config.wand_tag);
            host::send_chat(player_id, "§6✦ §fВы получили §6Топор Привата§f! ЛКМ — точка #1, ПКМ — точка #2, Shift+ПКМ — GUI.");
            host::play_sound(player_id, "minecraft:entity.item.pickup", 0.8, 1.2);
        }
        "pos1" => {
            let pos = host::get_player_block_pos(player_id);
            handle_wand_click(store, config, selections, active_viz, player_id, "left", pos);
        }
        "pos2" => {
            let pos = host::get_player_block_pos(player_id);
            handle_wand_click(store, config, selections, active_viz, player_id, "right", pos);
        }
        "claim" | "create" => {
            let Some(name) = parts.get(2) else {
                host::send_chat(player_id, "§eИспользование: §f/rg claim <название>");
                return true;
            };
            try_claim_selection(store, config, selections, active_viz, dirty, player_id, name);
        }
        "rename" => {
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg rename <старое_имя> <новое_имя>");
                return true;
            }
            let new_name = parts[3].trim().to_string();
            if new_name.len() > 24 || !new_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                host::send_chat(player_id, "§c✖ Некорректное имя (1–24 символа: буквы, цифры, _ и -).");
                return true;
            }
            if store.find_by_name_or_id(&new_name).is_some() {
                host::send_chat(player_id, "§c✖ Приват с таким именем уже существует.");
                return true;
            }
            let Some(rg_id) = store.find_id_by_name_or_id(parts[2]) else {
                host::send_chat(player_id, "§c✖ Приват не найден.");
                return true;
            };
            let rank = config.rank_for(store.rank_id_of(player_id));
            if let Some(rg) = store.regions.get_mut(&rg_id) {
                if !rg.is_owner(player_id) && !rank.bypass_protection {
                    host::send_chat(player_id, "§c✖ Только владелец может переименовать приват.");
                    return true;
                }
                let old = rg.name.clone();
                rg.name = new_name.clone();
                *dirty = true;
                host::send_chat(player_id, &format!("§a✔ Приват §e{} §aпереименован в §e{}§a!", old, new_name));
            }
        }
        "delete" | "remove" => {
            let Some(name) = parts.get(2) else {
                host::send_chat(player_id, "§eИспользование: §f/rg delete <название>");
                return true;
            };
            let Some(rg_id) = store.find_id_by_name_or_id(name) else {
                host::send_chat(player_id, "§c✖ Приват не найден.");
                return true;
            };
            menu::open_confirm_delete(store, player_id, &rg_id);
        }
        "expand" | "shrink" => {
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg expand <приват> <north|south|east|west|up|down|all|vert> [блоки]");
                return true;
            }
            let Some(dir) = ExpandDirection::parse(parts[3]) else {
                host::send_chat(player_id, "§c✖ Направление: north, south, east, west, up, down, all, vert");
                return true;
            };
            let raw_amount: i32 = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(5).clamp(1, 500);
            let delta = if sub == "shrink" { -raw_amount } else { raw_amount };
            try_resize_region(store, config, active_viz, dirty, player_id, parts[2], dir, delta);
        }
        "flag" => {
            if parts.len() < 5 {
                host::send_chat(player_id, "§eИспользование: §f/rg flag <приват> <explosions|pvp|fall|chests|doors|buttons|workstations|mobs|fire|entry|pickup> <on|off>");
                return true;
            }
            let Some(rg_id) = store.find_id_by_name_or_id(parts[2]) else {
                host::send_chat(player_id, "§c✖ Приват не найден.");
                return true;
            };
            let val = matches!(parts[4].to_lowercase().as_str(), "on" | "true" | "allow" | "1");
            let rank = config.rank_for(store.rank_id_of(player_id));
            if let Some(rg) = store.regions.get_mut(&rg_id) {
                if !rg.is_officer_or_owner(player_id) && !rank.bypass_protection {
                    host::send_chat(player_id, "§c✖ Нет прав для изменения флагов.");
                    return true;
                }
                match parts[3].to_lowercase().as_str() {
                    "explosions" | "tnt" => rg.flags.explosions = val,
                    "pvp" => {
                        if !rank.can_toggle_pvp {
                            host::send_chat(player_id, "§c🔒 Настройка PvP доступна со статуса §e[VIP]§c!");
                            return true;
                        }
                        rg.flags.pvp = val;
                    }
                    "fall" | "fall_damage" => {
                        if !rank.can_toggle_fall_damage {
                            host::send_chat(player_id, "§c🔒 Отключение урона от падения доступно с §a[PREMIUM]§c!");
                            return true;
                        }
                        rg.flags.fall_damage = val;
                    }
                    "chests" => rg.flags.open_chests = val,
                    "doors" => rg.flags.use_doors = val,
                    "buttons" | "levers" => rg.flags.use_buttons_levers = val,
                    "workstations" => rg.flags.use_workstations = val,
                    "mobs" => {
                        if !rank.can_toggle_mob_spawning {
                            host::send_chat(player_id, "§c🔒 Управление спавном мобов доступно с §e[VIP]§c!");
                            return true;
                        }
                        rg.flags.mob_spawning = val;
                    }
                    "fire" => rg.flags.fire_spread = val,
                    "entry" => {
                        if !rank.can_toggle_entry {
                            host::send_chat(player_id, "§c🔒 Закрытие входа доступно со статуса §d[LEGEND]§c!");
                            return true;
                        }
                        rg.flags.entry_allowed = val;
                    }
                    "pickup" => rg.flags.item_pickup = val,
                    _ => {
                        host::send_chat(player_id, "§c✖ Неизвестный флаг. Откройте GUI: /rg");
                        return true;
                    }
                }
                *dirty = true;
                host::send_chat(player_id, &format!("§a✔ Флаг §e{} §aв привате §f{} §aустановлен: {}", parts[3], rg.name, if val { "§aВКЛ" } else { "§cВЫКЛ" }));
            }
        }
        "add" | "trust" => {
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg add <приват> <игрок> [officer]");
                return true;
            }
            let Some(rg_id) = store.find_id_by_name_or_id(parts[2]) else {
                host::send_chat(player_id, "§c✖ Приват не найден.");
                return true;
            };
            let rank = config.rank_for(store.rank_id_of(player_id));
            if let Some(rg) = store.regions.get_mut(&rg_id) {
                if !rg.is_officer_or_owner(player_id) && !rank.bypass_protection {
                    host::send_chat(player_id, "§c✖ Нет прав для добавления участников.");
                    return true;
                }
                if rg.members.len() >= rank.max_members && !rg.members.contains_key(parts[3]) {
                    host::send_chat(player_id, &format!("§c✖ Лимит участников привата: §e{}", rank.max_members));
                    return true;
                }
                let role = if parts.get(4).map(|r| r.eq_ignore_ascii_case("officer")).unwrap_or(false) && rg.is_owner(player_id) {
                    MemberRole::Officer
                } else {
                    MemberRole::Member
                };
                rg.members.insert(parts[3].to_string(), role);
                *dirty = true;
                host::send_chat(player_id, &format!("§a✔ Игрок §e{} §aдобавлен в приват §f{}§a!", parts[3], rg.name));
            }
        }
        "kick" | "untrust" => {
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg kick <приват> <игрок>");
                return true;
            }
            let Some(rg_id) = store.find_id_by_name_or_id(parts[2]) else {
                host::send_chat(player_id, "§c✖ Приват не найден.");
                return true;
            };
            if let Some(rg) = store.regions.get_mut(&rg_id) {
                if !rg.is_owner(player_id) {
                    host::send_chat(player_id, "§c✖ Только владелец может исключать участников.");
                    return true;
                }
                if rg.members.remove(parts[3]).is_some() {
                    *dirty = true;
                    host::send_chat(player_id, &format!("§a✔ Игрок §e{} §aудалён из привата.", parts[3]));
                } else {
                    host::send_chat(player_id, "§c✖ Игрок не состоит в этом привате.");
                }
            }
        }
        "greeting" => {
            let rank = config.rank_for(store.rank_id_of(player_id));
            if !rank.can_set_greeting {
                host::send_chat(player_id, "§c🔒 Приветствие при входе доступно со статуса §e[VIP]§c!");
                return true;
            }
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg greeting <приват> <текст>");
                return true;
            }
            if let Some(rg_id) = store.find_id_by_name_or_id(parts[2]) {
                if let Some(rg) = store.regions.get_mut(&rg_id) {
                    if rg.is_owner(player_id) {
                        rg.greeting_title = Some(parts[3..].join(" "));
                        *dirty = true;
                        host::send_chat(player_id, "§a✔ Приветствие территории обновлено!");
                    }
                }
            }
        }
        "show" | "border" => {
            let target_rg = if let Some(q) = parts.get(2) {
                store.find_by_name_or_id(q).cloned()
            } else {
                let pos = host::get_player_block_pos(player_id);
                protection::region_at(store, &pos.world, pos.x, pos.y, pos.z).cloned()
            };
            if let Some(rg) = target_rg {
                let viz = visualizer::build_viz(player_id, rg.bounds, &rg.border_theme, false, config, host::now_ms());
                active_viz.insert(player_id.to_string(), viz);
                host::send_chat(player_id, &format!("§b✦ 3D-границы привата §e{} §bвключены на 25с!", rg.name));
            } else {
                host::send_chat(player_id, "§7В этой точке нет привата.");
            }
        }
        "info" => {
            let pos = host::get_player_block_pos(player_id);
            if let Some(rg) = protection::region_at(store, &pos.world, pos.x, pos.y, pos.z) {
                let (dx, dy, dz) = rg.bounds.dimensions();
                host::send_chat(
                    player_id,
                    &format!("§6✦ Приват: §f{} §8| §7Владелец: §e{} §8| §7Размер: §b{}×{}×{} §8({} бл.)", rg.name, rg.owner_name, dx, dy, dz, rg.bounds.volume()),
                );
            } else {
                host::send_chat(player_id, "§aТерритория свободна — можно заприватить через §e/rg wand§a!");
            }
        }
        "setrank" => {
            let caller = config.rank_for(store.rank_id_of(player_id));
            if !caller.bypass_protection {
                host::send_chat(player_id, "§c✖ Недостаточно прав.");
                return true;
            }
            if parts.len() < 4 {
                host::send_chat(player_id, "§eИспользование: §f/rg setrank <игрок> <player|vip|premium|legend|admin|owner>");
                return true;
            }
            let profile = config.rank_for(parts[3]);
            store.player_ranks.insert(parts[2].to_string(), profile.id.clone());
            *dirty = true;
            host::send_chat(
                player_id,
                &format!("§a✔ Игроку §e{} §aвыдан статус {}§a(приватов: §e{}§a, объём: §b{}§a)", parts[2], profile.display_prefix, profile.max_regions, profile.max_volume),
            );
        }
        _ => {
            host::send_chat(player_id, "§6✦ KRONOS Regions: §f/rg • /rg wand • /rg claim <имя> • /rg expand • /rg rename • /rg flag • /rg show");
        }
    }
    true
}
`,

    'src/lib.rs': `mod commands;
mod host;
mod menu;
mod models;
mod protection;
mod storage;
mod visualizer;

use log::info;
use models::{ActiveBorderViz, BlockPos, RegionStore, RegionsConfig, WandSelection};
use std::collections::HashMap;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: RegionsConfig,
    store: RegionStore,
    selections: HashMap<String, WandSelection>,
    active_viz: HashMap<String, ActiveBorderViz>,
    last_player_region: HashMap<String, Option<String>>,
    dirty: bool,
    last_autosave_ms: u64,
    tick_counter: u64,
}

static STATE: Mutex<Option<PluginState>> = Mutex::new(None);

fn read_str<'a>(ptr: *const u8, len: usize) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    unsafe { str::from_utf8(slice::from_raw_parts(ptr, len)).unwrap_or("") }
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let config = storage::load_config();
    let store = storage::load_store();
    let now = host::now_ms();

    let mut guard = STATE.lock().unwrap();
    *guard = Some(PluginState {
        config,
        store,
        selections: HashMap::new(),
        active_viz: HashMap::new(),
        last_player_region: HashMap::new(),
        dirty: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });

    info!("KRONOS Regions v1.0.0 initialized (Claim Wand, 3D Borders, Granular Flags & GUIs)");
    0
}

#[no_mangle]
pub extern "C" fn cleanup() {
    let mut guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        if state.dirty {
            storage::save_store_atomic(&state.store);
        }
    }
    *guard = None;
    info!("KRONOS Regions saved state and unloaded.");
}

#[no_mangle]
pub extern "C" fn on_player_join(
    player_id_ptr: *const u8,
    player_id_len: usize,
    name_ptr: *const u8,
    name_len: usize,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let name = read_str(name_ptr, name_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if !name.is_empty() {
            state.store.player_names.insert(player_id.to_string(), name.to_string());
        }
        state.last_player_region.insert(player_id.to_string(), None);
    }
}

#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.active_viz.remove(player_id);
        state.selections.remove(player_id);
        state.last_player_region.remove(player_id);
    }
}

/// 20-TPS tick: renders 3D particle borders, tracks region entry/exit banners, and autosaves every 30s.
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };
    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // 1. Render active 3D borders
    if !state.active_viz.is_empty() {
        let cfg = &state.config;
        state.active_viz.retain(|_, viz| {
            if now >= viz.expires_at_ms {
                return false;
            }
            visualizer::render_border_frame(viz, cfg);
            true
        });
    }

    // 2. Every 10 ticks (0.5s): detect region entry / farewell titles
    if state.tick_counter % 10 == 0 {
        let tracked_players: Vec<String> = state.last_player_region.keys().cloned().collect();
        for pid in tracked_players {
            let pos = host::get_player_block_pos(&pid);
            let current = protection::region_at(&state.store, &pos.world, pos.x, pos.y, pos.z)
                .map(|r| (r.id.clone(), r.name.clone(), r.greeting_title.clone(), r.owner_name.clone()));
            let current_id = current.as_ref().map(|(id, _, _, _)| id.clone());
            let prev_id = state.last_player_region.get(&pid).cloned().flatten();

            if current_id != prev_id {
                state.last_player_region.insert(pid.clone(), current_id);
                if let Some((_, name, greeting, owner)) = current {
                    if let Some(custom) = greeting {
                        host::send_title(&pid, &format!("§6✦ {}", name), &format!("§f{}", custom), 5, 35, 10);
                    } else {
                        host::send_action_bar(&pid, &format!("§6✦ §fВы вошли в приват §e{} §8(§7{}§8)", name, owner));
                    }
                } else if prev_id.is_some() {
                    host::send_action_bar(&pid, "§7Вы вышли на свободную территорию");
                }
            }
        }
    }

    // 3. Debounced atomic autosave every 30s
    if state.tick_counter % 20 == 0 && state.dirty {
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = now;
        }
    }
}

#[no_mangle]
pub extern "C" fn on_player_command(
    player_id_ptr: *const u8,
    player_id_len: usize,
    cmd_ptr: *const u8,
    cmd_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let command = read_str(cmd_ptr, cmd_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        return commands::handle_command(
            &mut state.store,
            &state.config,
            &mut state.selections,
            &mut state.active_viz,
            &mut state.dirty,
            player_id,
            command,
        );
    }
    false
}

#[no_mangle]
pub extern "C" fn on_wand_click(
    player_id_ptr: *const u8,
    player_id_len: usize,
    click_type_ptr: *const u8,
    click_type_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let click_type = read_str(click_type_ptr, click_type_len);
    let world = read_str(world_ptr, world_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        return commands::handle_wand_click(
            &state.store,
            &state.config,
            &mut state.selections,
            &mut state.active_viz,
            player_id,
            click_type,
            BlockPos { world: world.to_string(), x, y, z },
        );
    }
    false
}

#[no_mangle]
pub extern "C" fn on_menu_click(
    player_id_ptr: *const u8,
    player_id_len: usize,
    menu_id_ptr: *const u8,
    menu_id_len: usize,
    slot: u8,
    click_type_ptr: *const u8,
    click_type_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let menu_id = read_str(menu_id_ptr, menu_id_len);
    let click_type = read_str(click_type_ptr, click_type_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let mut ctx = menu::RegionMenuCtx {
            store: &mut state.store,
            config: &state.config,
            selections: &mut state.selections,
            active_viz: &mut state.active_viz,
            dirty: &mut state.dirty,
        };
        return menu::handle_menu_click(
            &mut ctx,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::try_claim_selection,
            commands::try_resize_region,
        );
    }
    false
}

#[no_mangle]
pub extern "C" fn on_block_break(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_modify_block(&s.store, &s.config, player_id, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_block_place(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    on_block_break(player_id_ptr, player_id_len, world_ptr, world_len, x, y, z)
}

#[no_mangle]
pub extern "C" fn on_block_interact(
    player_id_ptr: *const u8,
    player_id_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    block_id_ptr: *const u8,
    block_id_len: usize,
) -> bool {
    let player_id = read_str(player_id_ptr, player_id_len);
    let world = read_str(world_ptr, world_len);
    let block_id = read_str(block_id_ptr, block_id_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_interact_block(&s.store, &s.config, player_id, world, x, y, z, block_id))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_explosion(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_explode(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_pvp_damage(
    attacker_ptr: *const u8,
    attacker_len: usize,
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
) -> bool {
    let attacker = read_str(attacker_ptr, attacker_len);
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_pvp_damage(&s.store, attacker, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_fall_damage(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_fall_damage(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_mob_spawn(
    world_ptr: *const u8,
    world_len: usize,
    x: i32,
    y: i32,
    z: i32,
    hostile: bool,
) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_mob_spawn(&s.store, world, x, y, z, hostile))
        .unwrap_or(true)
}

#[no_mangle]
pub extern "C" fn on_fire_spread(world_ptr: *const u8, world_len: usize, x: i32, y: i32, z: i32) -> bool {
    let world = read_str(world_ptr, world_len);
    let guard = STATE.lock().unwrap();
    guard
        .as_ref()
        .map(|s| protection::can_fire_spread(&s.store, world, x, y, z))
        .unwrap_or(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{ExpandDirection, Region, RegionBounds, RegionFlags};

    #[test]
    fn aabb_overlap_and_directional_expansion_work() {
        let a = RegionBounds {
            world: "world".into(),
            min_x: 0, min_y: 60, min_z: 0,
            max_x: 9, max_y: 69, max_z: 9,
        };
        assert_eq!(a.volume(), 1000);

        let expanded = a.resized(ExpandDirection::East, 5, -64, 319);
        assert_eq!(expanded.max_x, 14);
        assert_eq!(expanded.volume(), 1500);

        let vert = a.resized(ExpandDirection::Vert, 0, -64, 319);
        assert_eq!(vert.min_y, -64);
        assert_eq!(vert.max_y, 319);

        let far = RegionBounds {
            world: "world".into(),
            min_x: 20, min_y: 60, min_z: 20,
            max_x: 30, max_y: 70, max_z: 30,
        };
        assert!(!a.intersects(&far));
        assert!(a.intersects(&expanded));
    }

    #[test]
    fn granular_flags_control_chests_doors_buttons_and_damage_independently() {
        let cfg = storage::default_config();
        let mut store = RegionStore::default();
        let mut flags = RegionFlags::default();
        flags.use_doors = true;
        flags.open_chests = false;
        flags.use_buttons_levers = false;
        flags.pvp = false;
        flags.fall_damage = false;
        flags.explosions = false;

        store.regions.insert(
            "rg_1".into(),
            Region {
                id: "rg_1".into(),
                name: "castle".into(),
                owner_id: "steve".into(),
                owner_name: "Steve".into(),
                bounds: RegionBounds {
                    world: "world".into(),
                    min_x: 0, min_y: 0, min_z: 0,
                    max_x: 20, max_y: 100, max_z: 20,
                },
                flags,
                members: HashMap::new(),
                border_theme: "emerald_wire".into(),
                greeting_title: None,
                created_at_ms: 0,
            },
        );

        assert!(protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:oak_door"));
        assert!(!protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:chest"));
        assert!(!protection::can_interact_block(&store, &cfg, "stranger", "world", 5, 65, 5, "minecraft:stone_button"));
        assert!(protection::can_interact_block(&store, &cfg, "steve", "world", 5, 65, 5, "minecraft:chest"));
        assert!(!protection::can_pvp_damage(&store, "stranger", "world", 5, 65, 5));
        assert!(!protection::can_fall_damage(&store, "world", 5, 65, 5));
        assert!(!protection::can_explode(&store, "world", 5, 65, 5));
    }

    #[test]
    fn rank_monetization_tiers_gate_premium_flags_and_themes() {
        let cfg = storage::default_config();
        assert!(!cfg.rank_for("player").can_toggle_pvp);
        assert!(cfg.rank_for("vip").can_toggle_pvp);
        assert!(!cfg.rank_for("vip").can_toggle_fall_damage);
        assert!(cfg.rank_for("premium").can_toggle_fall_damage);
        assert!(cfg.rank_for("legend").can_toggle_entry);
        assert!(!visualizer::can_use_border_theme(0, "golden_beam"));
        assert!(visualizer::can_use_border_theme(1, "golden_beam"));
        assert!(visualizer::can_use_border_theme(3, "dragon_aurora"));
    }
}
`,

    'README.md': `# KRONOS Regions v1.0.0 — Rust WASM Land Protection (Pumpkin 26.3)

Полноценный плагин привата территорий с **Топором Привата**, живой **3D-подсветкой границ частицами**, **6 интерактивными Chest GUI-меню**, раздельными флагами безопасности и продуманной прогрессией донат-статусов.

## Ключевые возможности
- **Специальный предмет — Топор Привата (\`/rg wand\`):**
  - \`ЛКМ\` по блоку — ставит **Точку #1** (зелёный луч частиц).
  - \`ПКМ\` по блоку — ставит **Точку #2** и сразу подсвечивает весь 3D-каркас будущего привата (зелёным, если свободно, или красным при пересечении с чужим регионом / превышении объёма).
  - \`Shift+ПКМ\` — открывает главное GUI-меню \`/rg\`.
- **Раздельная настройка каждого флага (в GUI и через \`/rg flag\`):**
  - Взрывы (TNT / криперы), PvP / убийство игроков, Урон от падения, Спавн мобов, Огонь, Вход чужаков, Подбор дропа.
  - Отдельные переключатели взаимодействий: **Сундуки/бочки**, **Двери/люки**, **Кнопки/рычаги/плиты**, **Верстаки/наковальни**.
- **Расширение и сжатие границ в любую сторону (GUI-компас + команды):**
  - \`/rg expand <приват> <north|south|east|west|up|down|all|vert> [блоки]\` и \`/rg shrink ...\`
  - В GUI: ЛКМ \`+5\`, Shift+ЛКМ \`+10\`, ПКМ \`-5\`, плюс кнопка **«От бедрока до неба»** с проверкой коллизий и мгновенной 3D-подсветкой.
- **Переименование, удаление с подтверждением, участники и роли (\`Участник ↔ Офицер\`), приветствия при входе (\`/rg greeting\`).**

## Прогрессия статусов (мотивация к апгрейду в \`data/config.json\`)
| Статус | Приваты | Макс. объём | Друзей | Откл. PvP | Откл. падения | Стиль границ |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| \`player\` | **2** | 40 000 | 3 | ✖ | ✖ | Изумрудный контур |
| \`vip\` | **4** | 120 000 | 6 | ✔ | ✖ | Золотые лучи |
| \`premium\` | **6** | 300 000 | 10 | ✔ | ✔ | Аметистовая сетка |
| \`legend\` | **10** | 750 000 | 20 | ✔ | ✔ | Аврора Дракона (+ закрытый вход) |
| \`admin\` / \`owner\` | **∞** | 50 000 000 | 100 | ✔ | ✔ | Призма KRONOS |

## Тесты и сборка
\`\`\`bash
cargo test
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
\`\`\`
`,
  },
};
