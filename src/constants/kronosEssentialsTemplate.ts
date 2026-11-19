import { PluginTemplate } from '../types';

export const KRONOS_ESSENTIALS_TEMPLATE: PluginTemplate = {
  id: 'kronos-essentials',
  title: 'KRONOS Essentials (Homes, Spawn, GUI & FX)',
  description:
    'Complete Rust WASM teleport suite: /spawn, /sethome, /home, /warps, /back, /tpa, interactive 54-slot GUI menu, rank-scaled home limits (3 → 5 → 7 → 10 → ∞), and spiral particle animations.',
  tags: ['Essentials', 'Homes & Spawn', 'Chest GUI', 'Particles & FX'],
  files: {
    'Cargo.toml': `[package]
name = "kronos-essentials"
version = "1.0.0"
edition = "2021"
description = "KRONOS Essentials: /spawn, /sethome, /home, /warps, /back, /tpa with Chest GUI and cinematic teleport effects for Pumpkin 26.3"

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
  "plugin_name": "kronos-essentials",
  "cancel_on_move_blocks": 0.45,
  "cancel_on_damage": true,
  "tpa_timeout_seconds": 60,
  "blindness_flash_ticks": 16,
  "slow_falling_ticks": 30,
  "home_icons": [
    "minecraft:red_bed",
    "minecraft:campfire",
    "minecraft:diamond_pickaxe",
    "minecraft:chest",
    "minecraft:lantern",
    "minecraft:enchanting_table",
    "minecraft:beacon",
    "minecraft:nether_star"
  ],
  "ranks": [
    {
      "id": "player",
      "tier": 0,
      "display_prefix": "§7[ИГРОК] ",
      "max_homes": 3,
      "warmup_seconds": 3,
      "cooldown_seconds": 15,
      "teleport_theme": "amethyst_spiral",
      "can_set_spawn": false,
      "can_set_warp": false,
      "unlimited_homes": false
    },
    {
      "id": "vip",
      "tier": 1,
      "display_prefix": "§e§l[VIP] ",
      "max_homes": 5,
      "warmup_seconds": 2,
      "cooldown_seconds": 10,
      "teleport_theme": "golden_vortex",
      "can_set_spawn": false,
      "can_set_warp": false,
      "unlimited_homes": false
    },
    {
      "id": "premium",
      "tier": 2,
      "display_prefix": "§a§l[PREMIUM] ",
      "max_homes": 7,
      "warmup_seconds": 2,
      "cooldown_seconds": 6,
      "teleport_theme": "emerald_pulse",
      "can_set_spawn": false,
      "can_set_warp": false,
      "unlimited_homes": false
    },
    {
      "id": "legend",
      "tier": 3,
      "display_prefix": "§d§l[LEGEND] ",
      "max_homes": 10,
      "warmup_seconds": 1,
      "cooldown_seconds": 3,
      "teleport_theme": "dragon_nova",
      "can_set_spawn": false,
      "can_set_warp": true,
      "unlimited_homes": false
    },
    {
      "id": "admin",
      "tier": 4,
      "display_prefix": "§c§l[ADMIN] ",
      "max_homes": 25,
      "warmup_seconds": 0,
      "cooldown_seconds": 0,
      "teleport_theme": "kronos_rift",
      "can_set_spawn": true,
      "can_set_warp": true,
      "unlimited_homes": true
    },
    {
      "id": "owner",
      "tier": 5,
      "display_prefix": "§4§l[OWNER] ",
      "max_homes": 50,
      "warmup_seconds": 0,
      "cooldown_seconds": 0,
      "teleport_theme": "kronos_rift",
      "can_set_spawn": true,
      "can_set_warp": true,
      "unlimited_homes": true
    }
  ]
}
`,

    'src/models.rs': `use serde::{Deserialize, Serialize};
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
pub struct RankProfile {
    pub id: String,
    pub tier: u8,
    pub display_prefix: String,
    pub max_homes: usize,
    pub warmup_seconds: u64,
    pub cooldown_seconds: u64,
    pub teleport_theme: String,
    pub can_set_spawn: bool,
    pub can_set_warp: bool,
    pub unlimited_homes: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct EssentialsConfig {
    pub plugin_name: String,
    pub cancel_on_move_blocks: f64,
    pub cancel_on_damage: bool,
    pub tpa_timeout_seconds: u64,
    pub blindness_flash_ticks: u32,
    pub slow_falling_ticks: u32,
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PlayerRecord {
    pub player_id: String,
    pub username: String,
    pub rank: String,
    pub selected_theme: Option<String>,
    pub homes: Vec<HomePoint>,
    pub last_location: Option<Location>,
    pub last_teleport_ms: u64,
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
            last_teleport_ms: 0,
        }
    }

    pub fn active_theme<'a>(&'a self, rank: &'a RankProfile) -> &'a str {
        self.selected_theme
            .as_deref()
            .unwrap_or(rank.teleport_theme.as_str())
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
`,

    'src/host.rs': `use crate::models::{ChestMenuDescriptor, Location};
use log::info;

/// Pumpkin 26.3 WASM Host Imports.
/// Bind these functions in the Pumpkin WASM runtime under the \`pumpkin_host\` namespace.
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
    fn host_apply_effect(
        pid_ptr: *const u8,
        pid_len: usize,
        effect_ptr: *const u8,
        effect_len: usize,
        duration_ticks: u32,
        amplifier: u8,
    );
    fn host_open_chest_menu(pid_ptr: *const u8, pid_len: usize, json_ptr: *const u8, json_len: usize);
    fn host_close_menu(pid_ptr: *const u8, pid_len: usize);
    fn host_get_player_location(
        pid_ptr: *const u8,
        pid_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn host_now_ms() -> u64;
}

pub fn send_chat(player_id: &str, message: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_chat(player_id.as_ptr(), player_id.len(), message.as_ptr(), message.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[CHAT -> {}] {}", player_id, message);
}

pub fn send_action_bar(player_id: &str, text: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_action_bar(player_id.as_ptr(), player_id.len(), text.as_ptr(), text.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[ACTIONBAR -> {}] {}", player_id, text);
}

pub fn send_title(
    player_id: &str,
    title: &str,
    subtitle: &str,
    fade_in: u32,
    stay: u32,
    fade_out: u32,
) {
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

pub fn teleport_player(player_id: &str, loc: &Location) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_teleport(
            player_id.as_ptr(),
            player_id.len(),
            loc.world.as_ptr(),
            loc.world.len(),
            loc.x,
            loc.y,
            loc.z,
            loc.yaw,
            loc.pitch,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TELEPORT -> {}] {}", player_id, loc.coords_label());
}

pub fn spawn_particle(
    world: &str,
    particle: &str,
    x: f64,
    y: f64,
    z: f64,
    count: u32,
    speed: f32,
) {
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
        host_play_sound(
            player_id.as_ptr(),
            player_id.len(),
            sound.as_ptr(),
            sound.len(),
            volume,
            pitch,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, sound, volume, pitch);
}

pub fn apply_effect(player_id: &str, effect: &str, duration_ticks: u32, amplifier: u8) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_apply_effect(
            player_id.as_ptr(),
            player_id.len(),
            effect.as_ptr(),
            effect.len(),
            duration_ticks,
            amplifier,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, effect, duration_ticks, amplifier);
}

pub fn open_chest_menu(player_id: &str, descriptor: &ChestMenuDescriptor) {
    if let Ok(json) = serde_json::to_string(descriptor) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            host_open_chest_menu(
                player_id.as_ptr(),
                player_id.len(),
                json.as_ptr(),
                json.len(),
            );
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

pub fn get_player_location(player_id: &str) -> Location {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 512];
        let written = unsafe {
            host_get_player_location(
                player_id.as_ptr(),
                player_id.len(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        if written > 0 {
            if let Ok(loc) = serde_json::from_slice::<Location>(&buf[..written as usize]) {
                return loc;
            }
        }
    }

    Location {
        world: "world".to_string(),
        x: 0.5,
        y: 65.0,
        z: 0.5,
        yaw: 0.0,
        pitch: 0.0,
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

    'src/effects.rs': `use crate::host;
use crate::models::{EssentialsConfig, Location, PendingTeleport};
use std::f64::consts::PI;

pub const AVAILABLE_THEMES: &[(&str, &str)] = &[
    ("amethyst_spiral", "§dАметистовая спираль"),
    ("golden_vortex", "§6Золотой вихрь"),
    ("emerald_pulse", "§aИзумрудный импульс"),
    ("dragon_nova", "§5Драконья сверхновая"),
    ("kronos_rift", "§bРазлом KRONOS"),
];

struct ThemePalette {
    primary: &'static str,
    secondary: &'static str,
    accent: &'static str,
    arrival_sound: &'static str,
    title_color: &'static str,
}

fn palette_for(theme: &str) -> ThemePalette {
    match theme {
        "golden_vortex" => ThemePalette {
            primary: "minecraft:flame",
            secondary: "minecraft:wax_on",
            accent: "minecraft:totem_of_undying",
            arrival_sound: "minecraft:item.totem.use",
            title_color: "§6",
        },
        "emerald_pulse" => ThemePalette {
            primary: "minecraft:happy_villager",
            secondary: "minecraft:glow",
            accent: "minecraft:end_rod",
            arrival_sound: "minecraft:entity.player.levelup",
            title_color: "§a",
        },
        "dragon_nova" => ThemePalette {
            primary: "minecraft:dragon_breath",
            secondary: "minecraft:reverse_portal",
            accent: "minecraft:end_rod",
            arrival_sound: "minecraft:entity.ender_dragon.flap",
            title_color: "§d",
        },
        "kronos_rift" => ThemePalette {
            primary: "minecraft:soul_fire_flame",
            secondary: "minecraft:end_rod",
            accent: "minecraft:enchant",
            arrival_sound: "minecraft:block.beacon.power_select",
            title_color: "§b",
        },
        _ => ThemePalette {
            primary: "minecraft:portal",
            secondary: "minecraft:enchant",
            accent: "minecraft:end_rod",
            arrival_sound: "minecraft:block.amethyst_block.chime",
            title_color: "§d",
        },
    }
}

pub fn theme_display_name(theme: &str) -> &'static str {
    AVAILABLE_THEMES
        .iter()
        .find(|(id, _)| *id == theme)
        .map(|(_, label)| *label)
        .unwrap_or("§dАметистовая спираль")
}

/// Renders one 20-TPS animation frame during teleport warmup:
/// - Ascending dual-helix particle vortex around the player
/// - Pulsing rune circle under the player's feet
/// - Animated ActionBar progress bar & rising harmonic chimes
pub fn render_warmup_frame(pending: &mut PendingTeleport, now_ms: u64) {
    let elapsed = now_ms.saturating_sub(pending.started_at_ms);
    let duration = pending.duration_ms.max(1);
    let progress = (elapsed as f64 / duration as f64).clamp(0.0, 1.0);
    let palette = palette_for(&pending.theme);

    let t = pending.ticks_elapsed as f64;
    let base_angle = t * 0.36;
    let radius = 1.2 - progress * 0.55;
    let height = progress * 2.15;

    // 1. Dual ascending helix arms
    for arm in 0..2 {
        let angle = base_angle + (arm as f64) * PI;
        let px = pending.origin.x + radius * angle.cos();
        let py = pending.origin.y + 0.15 + height;
        let pz = pending.origin.z + radius * angle.sin();
        host::spawn_particle(&pending.origin.world, palette.primary, px, py, pz, 2, 0.01);
        host::spawn_particle(&pending.origin.world, palette.secondary, px, py + 0.25, pz, 1, 0.02);
    }

    // 2. Ground rune ring every 4 ticks
    if pending.ticks_elapsed % 4 == 0 {
        let ring_points = 8;
        let ring_r = 0.95 + (t * 0.15).sin() * 0.15;
        for i in 0..ring_points {
            let a = (i as f64 / ring_points as f64) * 2.0 * PI + base_angle * 0.35;
            let rx = pending.origin.x + ring_r * a.cos();
            let rz = pending.origin.z + ring_r * a.sin();
            host::spawn_particle(
                &pending.origin.world,
                palette.accent,
                rx,
                pending.origin.y + 0.08,
                rz,
                1,
                0.0,
            );
        }
    }

    // 3. ActionBar progress meter
    let filled = ((progress * 10.0).round() as usize).min(10);
    let bar = format!(
        "§a{}§8{}",
        "■".repeat(filled),
        "□".repeat(10 - filled)
    );
    let remaining_sec = ((duration.saturating_sub(elapsed) + 999) / 1000).max(1);
    host::send_action_bar(
        &pending.player_id,
        &format!(
            "{}✦ §fТелепорт: §e{} §8[{}] §f{}с §7(не двигайтесь)",
            palette.title_color, pending.target_label, bar, remaining_sec
        ),
    );

    // 4. Harmonic tick sound once per second
    if remaining_sec != pending.last_second_announced {
        pending.last_second_announced = remaining_sec;
        let pitch = 0.85 + (progress as f32) * 0.75;
        host::play_sound(&pending.player_id, "minecraft:block.note_block.chime", 0.75, pitch);
    }

    pending.ticks_elapsed = pending.ticks_elapsed.saturating_add(1);
}

/// Executes the cinematic teleport burst at destination:
/// - Departure flash at origin
/// - Teleportation + arrival shockwave ring + vertical beacon column
/// - Layered spatial audio + brief fade/slow-falling polish + Title banner
pub fn play_arrival_burst(
    player_id: &str,
    target_label: &str,
    origin: &Location,
    dest: &Location,
    theme: &str,
    config: &EssentialsConfig,
) {
    let palette = palette_for(theme);

    // Departure burst at old location
    host::spawn_particle(
        &origin.world,
        palette.primary,
        origin.x,
        origin.y + 1.0,
        origin.z,
        24,
        0.18,
    );

    // Perform actual teleport
    host::teleport_player(player_id, dest);

    // Arrival radial shockwave ring (16 particles)
    let ring_count = 16;
    for i in 0..ring_count {
        let a = (i as f64 / ring_count as f64) * 2.0 * PI;
        let rx = dest.x + 1.15 * a.cos();
        let rz = dest.z + 1.15 * a.sin();
        host::spawn_particle(&dest.world, palette.primary, rx, dest.y + 0.35, rz, 2, 0.04);
        host::spawn_particle(&dest.world, palette.accent, rx, dest.y + 1.2, rz, 1, 0.05);
    }

    // Vertical light pillar
    for step in 0..7 {
        let py = dest.y + 0.25 + (step as f64) * 0.4;
        host::spawn_particle(&dest.world, palette.secondary, dest.x, py, dest.z, 2, 0.02);
    }

    // Layered audio
    host::play_sound(player_id, "minecraft:entity.enderman.teleport", 0.85, 1.05);
    host::play_sound(player_id, palette.arrival_sound, 0.65, 1.2);

    // Smooth visual landing polish
    if config.blindness_flash_ticks > 0 {
        host::apply_effect(player_id, "minecraft:blindness", config.blindness_flash_ticks, 0);
    }
    if config.slow_falling_ticks > 0 {
        host::apply_effect(player_id, "minecraft:slow_falling", config.slow_falling_ticks, 0);
    }

    host::send_title(
        player_id,
        &format!("{}✦ {}", palette.title_color, target_label),
        &format!("§7Вы прибыли §8• §f{}", dest.coords_label()),
        5,
        35,
        15,
    );
    host::send_action_bar(
        player_id,
        &format!("§a✔ Телепортация завершена: §f{}", target_label),
    );
}

pub fn play_cancel_effect(player_id: &str, origin: &Location, reason: &str) {
    host::spawn_particle(
        &origin.world,
        "minecraft:smoke",
        origin.x,
        origin.y + 0.9,
        origin.z,
        16,
        0.08,
    );
    host::play_sound(player_id, "minecraft:block.note_block.bass", 0.9, 0.6);
    host::send_title(
        player_id,
        "§c✖ Телепорт отменён",
        &format!("§7{}", reason),
        4,
        28,
        10,
    );
    host::send_action_bar(player_id, &format!("§c✖ {}", reason));
}
`,

    'src/menu.rs': `use crate::effects::{theme_display_name, AVAILABLE_THEMES};
use crate::host;
use crate::models::{
    ChestMenuDescriptor, EssentialsConfig, EssentialsStore, HomePoint, MenuSlot, PendingTeleport,
};
use std::collections::HashMap;

/// 14 interior slots (rows 2 & 3, cols 2..8) reserved for player homes
pub const HOME_GRID_SLOTS: [u8; 14] = [
    19, 20, 21, 22, 23, 24, 25,
    28, 29, 30, 31, 32, 33, 34,
];

const BORDER_SLOTS: [u8; 26] = [
    0, 1, 2, 3, 5, 6, 7, 8,
    9, 17, 18, 26, 27, 35, 36, 44,
    45, 46, 52, 53,
    10, 11, 12, 13, 14, 15,
];

/// Opens the 54-slot KRONOS Teleport & Homes GUI Menu
pub fn open_homes_menu(
    store: &EssentialsStore,
    config: &EssentialsConfig,
    player_id: &str,
    page: usize,
) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.rank_for(&player.rank);
    let limit_text = if rank.unlimited_homes {
        "∞".to_string()
    } else {
        rank.max_homes.to_string()
    };

    let mut slots: Vec<MenuSlot> = Vec::with_capacity(54);

    // Decorative frame
    for &slot in &BORDER_SLOTS {
        let is_corner = matches!(slot, 0 | 8 | 45 | 53);
        slots.push(MenuSlot {
            slot,
            item: if is_corner {
                "minecraft:orange_stained_glass_pane".to_string()
            } else {
                "minecraft:black_stained_glass_pane".to_string()
            },
            name: "§8".to_string(),
            lore: vec![],
            enchanted: false,
        });
    }
    slots.push(MenuSlot {
        slot: 16,
        item: "minecraft:black_stained_glass_pane".to_string(),
        name: "§8".to_string(),
        lore: vec![],
        enchanted: false,
    });

    // Top-center Player Status Card (slot 4)
    let next_hint = match config.next_rank_after(rank.tier) {
        Some(next) if !rank.unlimited_homes => {
            format!("§7Следующий ранг {}: §e{} меток", next.display_prefix.trim(), next.max_homes)
        }
        _ => "§aМаксимальный лимит меток разблокирован!".to_string(),
    };

    slots.push(MenuSlot {
        slot: 4,
        item: "minecraft:player_head".to_string(),
        name: format!("{}§f{}", rank.display_prefix, player.username),
        lore: vec![
            format!("§7Точки дома: §e{} §8/ §a{}", player.homes.len(), limit_text),
            format!("§7Задержка телепорта: §f{}с", rank.warmup_seconds),
            format!("§7Кулдаун: §f{}с", rank.cooldown_seconds),
            format!("§7Эффект: {}", theme_display_name(player.active_theme(&rank))),
            "§8────────────────────".to_string(),
            next_hint,
        ],
        enchanted: true,
    });

    // Populate the 14-slot home grid
    let per_page = HOME_GRID_SLOTS.len();
    let offset = page * per_page;

    for (idx, &slot_num) in HOME_GRID_SLOTS.iter().enumerate() {
        let home_index = offset + idx;
        if let Some(home) = player.homes.get(home_index) {
            slots.push(MenuSlot {
                slot: slot_num,
                item: home.icon.clone(),
                name: format!("§6✦ §eДом: §f{}", home.name),
                lore: vec![
                    format!("§7Координаты: §f{}", home.location.coords_label()),
                    format!("§7Использований: §b{}", home.uses),
                    "§8────────────────────".to_string(),
                    "§a▸ ЛКМ §8— §fТелепортироваться".to_string(),
                    "§e▸ Shift+ЛКМ §8— §fСменить иконку".to_string(),
                    "§c▸ ПКМ §8— §fУдалить метку".to_string(),
                ],
                enchanted: true,
            });
        } else if rank.unlimited_homes || home_index < rank.max_homes {
            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:lime_stained_glass_pane".to_string(),
                name: format!("§a+ Свободный слот #{}", home_index + 1),
                lore: vec![
                    "§7Нажмите, чтобы быстро создать".to_string(),
                    "§7точку дома на текущей позиции!".to_string(),
                    "§8Или введите: §f/sethome <название>".to_string(),
                ],
                enchanted: false,
            });
        } else {
            let unlock_rank = config
                .ranks
                .iter()
                .find(|r| r.unlimited_homes || r.max_homes > home_index)
                .map(|r| r.display_prefix.trim().to_string())
                .unwrap_or_else(|| "§d[LEGEND]".to_string());

            slots.push(MenuSlot {
                slot: slot_num,
                item: "minecraft:gray_stained_glass_pane".to_string(),
                name: "§8🔒 Заблокированный слот".to_string(),
                lore: vec![
                    format!("§7Доступно с ранга {}", unlock_rank),
                    "§7Повысьте статус для увеличения".to_string(),
                    "§7лимита точек дома (3 → 5 → 7 → 10)!".to_string(),
                ],
                enchanted: false,
            });
        }
    }

    // Bottom Navigation Bar
    if page > 0 {
        slots.push(MenuSlot {
            slot: 45,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Предыдущая страница".to_string(),
            lore: vec![format!("§7Страница {}", page)],
            enchanted: false,
        });
    }

    slots.push(MenuSlot {
        slot: 47,
        item: "minecraft:nether_star".to_string(),
        name: "§6✦ Телепорт на Спавн".to_string(),
        lore: vec![
            match &store.spawn {
                Some(loc) => format!("§7Точка: §f{}", loc.coords_label()),
                None => "§cСпавн ещё не установлен (/setspawn)".to_string(),
            },
            "§a▸ Нажмите для перемещения".to_string(),
        ],
        enchanted: store.spawn.is_some(),
    });

    slots.push(MenuSlot {
        slot: 48,
        item: "minecraft:compass".to_string(),
        name: "§b✦ Серверные Варпы".to_string(),
        lore: vec![
            format!("§7Доступно варпов: §e{}", store.warps.len()),
            "§a▸ Нажмите, чтобы открыть список".to_string(),
        ],
        enchanted: false,
    });

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:ender_pearl".to_string(),
        name: "§d✦ Вернуться назад (/back)".to_string(),
        lore: vec![match &player.last_location {
            Some(loc) => format!("§7Прошлая точка: §f{}", loc.coords_label()),
            None => "§8Нет сохранённой прошлой точки".to_string(),
        }],
        enchanted: player.last_location.is_some(),
    });

    slots.push(MenuSlot {
        slot: 50,
        item: "minecraft:amethyst_shard".to_string(),
        name: "§d✦ Стиль анимации телепорта".to_string(),
        lore: vec![
            format!("§7Текущий: {}", theme_display_name(player.active_theme(&rank))),
            "§e▸ Нажмите, чтобы переключить эффект".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 51,
        item: "minecraft:barrier".to_string(),
        name: "§cЗакрыть меню".to_string(),
        lore: vec![],
        enchanted: false,
    });

    if rank.unlimited_homes || (page + 1) * per_page < rank.max_homes {
        slots.push(MenuSlot {
            slot: 53,
            item: "minecraft:arrow".to_string(),
            name: "§eСледующая страница ▶".to_string(),
            lore: vec![format!("§7Страница {}", page + 2)],
            enchanted: false,
        });
    }

    let menu = ChestMenuDescriptor {
        menu_id: format!("kronos_homes:{}", page),
        title: format!("§8✦ §6KRONOS Телепорты §8[§e{}/{}§8]", player.homes.len(), limit_text),
        rows: 6,
        slots,
    };

    host::open_chest_menu(player_id, &menu);
    host::play_sound(player_id, "minecraft:ui.button.click", 0.6, 1.2);
}

/// Opens the Warps GUI Menu
pub fn open_warps_menu(store: &EssentialsStore, config: &EssentialsConfig, player_id: &str) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.rank_for(&player.rank);
    let mut slots: Vec<MenuSlot> = Vec::new();

    let mut sorted_warps: Vec<_> = store.warps.values().collect();
    sorted_warps.sort_by(|a, b| a.name.cmp(&b.name));

    for (idx, warp) in sorted_warps.iter().take(HOME_GRID_SLOTS.len()).enumerate() {
        let unlocked = rank.tier >= warp.min_tier;
        slots.push(MenuSlot {
            slot: HOME_GRID_SLOTS[idx],
            item: if unlocked {
                warp.icon.clone()
            } else {
                "minecraft:iron_bars".to_string()
            },
            name: format!("§b✦ Варп: §f{}", warp.name),
            lore: vec![
                format!("§7{}", warp.description),
                format!("§7Точка: §f{}", warp.location.coords_label()),
                if unlocked {
                    "§a▸ Нажмите для телепортации".to_string()
                } else {
                    "§c🔒 Требуется более высокий статус".to_string()
                },
            ],
            enchanted: unlocked,
        });
    }

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec!["§7Вернуться в главное меню телепортов".to_string()],
        enchanted: false,
    });

    let menu = ChestMenuDescriptor {
        menu_id: "kronos_warps".to_string(),
        title: "§8✦ §bKRONOS Варпы Сервера".to_string(),
        rows: 6,
        slots,
    };

    host::open_chest_menu(player_id, &menu);
}

/// Handles clicks inside KRONOS GUI menus.
/// click_type: "left" | "right" | "shift_left"
pub fn handle_menu_click(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    player_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    schedule_tp: impl Fn(
        &mut EssentialsStore,
        &EssentialsConfig,
        &mut HashMap<String, PendingTeleport>,
        &str,
        &str,
        crate::models::Location,
    ),
) -> bool {
    if menu_id == "kronos_warps" {
        if slot == 49 {
            open_homes_menu(store, config, player_id, 0);
            return true;
        }
        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let mut sorted_warps: Vec<_> = store.warps.values().cloned().collect();
            sorted_warps.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(warp) = sorted_warps.get(grid_idx) {
                let rank = store
                    .players
                    .get(player_id)
                    .map(|p| config.rank_for(&p.rank))
                    .unwrap_or_else(|| config.ranks[0].clone());
                if rank.tier < warp.min_tier {
                    host::send_chat(player_id, "§c✖ Этот варп доступен с более высокого статуса.");
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 1.0);
                    return true;
                }
                host::close_menu(player_id);
                schedule_tp(
                    store,
                    config,
                    pending,
                    player_id,
                    &format!("Варп {}", warp.name),
                    warp.location.clone(),
                );
            }
        }
        return true;
    }

    if let Some(page_str) = menu_id.strip_prefix("kronos_homes:") {
        let page: usize = page_str.parse().unwrap_or(0);

        match slot {
            45 if page > 0 => {
                open_homes_menu(store, config, player_id, page - 1);
                return true;
            }
            53 => {
                open_homes_menu(store, config, player_id, page + 1);
                return true;
            }
            47 => {
                host::close_menu(player_id);
                if let Some(spawn) = store.spawn.clone() {
                    schedule_tp(store, config, pending, player_id, "Спавн", spawn);
                } else {
                    host::send_chat(player_id, "§c✖ Точка спавна пока не задана (/setspawn).");
                }
                return true;
            }
            48 => {
                open_warps_menu(store, config, player_id);
                return true;
            }
            49 => {
                host::close_menu(player_id);
                let back_loc = store
                    .players
                    .get(player_id)
                    .and_then(|p| p.last_location.clone());
                if let Some(loc) = back_loc {
                    schedule_tp(store, config, pending, player_id, "Прошлая точка", loc);
                } else {
                    host::send_chat(player_id, "§c✖ У вас ещё нет сохранённой точки для /back.");
                }
                return true;
            }
            50 => {
                if let Some(player) = store.players.get_mut(player_id) {
                    let rank = config.rank_for(&player.rank);
                    let current = player.active_theme(&rank).to_string();
                    let cur_idx = AVAILABLE_THEMES
                        .iter()
                        .position(|(id, _)| *id == current)
                        .unwrap_or(0);
                    let next_idx = (cur_idx + 1) % AVAILABLE_THEMES.len();
                    let (next_id, next_label) = AVAILABLE_THEMES[next_idx];
                    player.selected_theme = Some(next_id.to_string());
                    host::send_chat(
                        player_id,
                        &format!("§6✦ §fЭффект телепортации изменён на: {}", next_label),
                    );
                }
                open_homes_menu(store, config, player_id, page);
                return true;
            }
            51 => {
                host::close_menu(player_id);
                return true;
            }
            _ => {}
        }

        // Click on one of the 14 home slots
        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let home_idx = page * HOME_GRID_SLOTS.len() + grid_idx;
            let Some(player) = store.players.get_mut(player_id) else {
                return true;
            };
            let rank = config.rank_for(&player.rank);

            if home_idx < player.homes.len() {
                match click_type {
                    "right" => {
                        let removed = player.homes.remove(home_idx);
                        host::send_chat(
                            player_id,
                            &format!("§c✖ Метка дома §e{} §cудалена.", removed.name),
                        );
                        host::play_sound(player_id, "minecraft:entity.item.break", 0.7, 1.0);
                        open_homes_menu(store, config, player_id, page);
                    }
                    "shift_left" => {
                        if !config.home_icons.is_empty() {
                            let cur_icon = &player.homes[home_idx].icon;
                            let pos = config
                                .home_icons
                                .iter()
                                .position(|i| i == cur_icon)
                                .unwrap_or(0);
                            let next_icon =
                                config.home_icons[(pos + 1) % config.home_icons.len()].clone();
                            player.homes[home_idx].icon = next_icon;
                            host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.6, 1.4);
                        }
                        open_homes_menu(store, config, player_id, page);
                    }
                    _ => {
                        player.homes[home_idx].uses += 1;
                        let home_name = player.homes[home_idx].name.clone();
                        let dest = player.homes[home_idx].location.clone();
                        host::close_menu(player_id);
                        schedule_tp(
                            store,
                            config,
                            pending,
                            player_id,
                            &format!("Дом {}", home_name),
                            dest,
                        );
                    }
                }
            } else if rank.unlimited_homes || home_idx < rank.max_homes {
                // Quick-create home in empty unlocked slot
                let auto_name = format!("home{}", player.homes.len() + 1);
                let loc = host::get_player_location(player_id);
                let icon = config
                    .home_icons
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "minecraft:red_bed".to_string());
                player.homes.push(HomePoint {
                    name: auto_name.clone(),
                    icon,
                    location: loc,
                    created_at_ms: host::now_ms(),
                    uses: 0,
                });
                host::send_chat(
                    player_id,
                    &format!("§a✔ Быстро создана метка дома §e{}§a!", auto_name),
                );
                host::play_sound(player_id, "minecraft:entity.player.levelup", 0.65, 1.4);
                open_homes_menu(store, config, player_id, page);
            } else {
                host::send_chat(
                    player_id,
                    "§c🔒 Этот слот заблокирован. Повысьте статус, чтобы увеличить лимит домов!",
                );
                host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
            }
            return true;
        }
        return true;
    }

    false
}
`,

    'src/storage.rs': `use crate::models::{EssentialsConfig, EssentialsStore, RankProfile};
use log::{info, warn};
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data/kronos-essentials";
const CONFIG_PATH: &str = "data/kronos-essentials/config.json";
const STORE_PATH: &str = "data/kronos-essentials/store.json";

pub fn default_config() -> EssentialsConfig {
    EssentialsConfig {
        plugin_name: "kronos-essentials".to_string(),
        cancel_on_move_blocks: 0.45,
        cancel_on_damage: true,
        tpa_timeout_seconds: 60,
        blindness_flash_ticks: 16,
        slow_falling_ticks: 30,
        home_icons: vec![
            "minecraft:red_bed".to_string(),
            "minecraft:campfire".to_string(),
            "minecraft:diamond_pickaxe".to_string(),
            "minecraft:chest".to_string(),
            "minecraft:lantern".to_string(),
            "minecraft:enchanting_table".to_string(),
            "minecraft:beacon".to_string(),
            "minecraft:nether_star".to_string(),
        ],
        ranks: vec![
            RankProfile {
                id: "player".to_string(),
                tier: 0,
                display_prefix: "§7[ИГРОК] ".to_string(),
                max_homes: 3,
                warmup_seconds: 3,
                cooldown_seconds: 15,
                teleport_theme: "amethyst_spiral".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                unlimited_homes: false,
            },
            RankProfile {
                id: "vip".to_string(),
                tier: 1,
                display_prefix: "§e§l[VIP] ".to_string(),
                max_homes: 5,
                warmup_seconds: 2,
                cooldown_seconds: 10,
                teleport_theme: "golden_vortex".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                unlimited_homes: false,
            },
            RankProfile {
                id: "premium".to_string(),
                tier: 2,
                display_prefix: "§a§l[PREMIUM] ".to_string(),
                max_homes: 7,
                warmup_seconds: 2,
                cooldown_seconds: 6,
                teleport_theme: "emerald_pulse".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                unlimited_homes: false,
            },
            RankProfile {
                id: "legend".to_string(),
                tier: 3,
                display_prefix: "§d§l[LEGEND] ".to_string(),
                max_homes: 10,
                warmup_seconds: 1,
                cooldown_seconds: 3,
                teleport_theme: "dragon_nova".to_string(),
                can_set_spawn: false,
                can_set_warp: true,
                unlimited_homes: false,
            },
            RankProfile {
                id: "admin".to_string(),
                tier: 4,
                display_prefix: "§c§l[ADMIN] ".to_string(),
                max_homes: 25,
                warmup_seconds: 0,
                cooldown_seconds: 0,
                teleport_theme: "kronos_rift".to_string(),
                can_set_spawn: true,
                can_set_warp: true,
                unlimited_homes: true,
            },
            RankProfile {
                id: "owner".to_string(),
                tier: 5,
                display_prefix: "§4§l[OWNER] ".to_string(),
                max_homes: 50,
                warmup_seconds: 0,
                cooldown_seconds: 0,
                teleport_theme: "kronos_rift".to_string(),
                can_set_spawn: true,
                can_set_warp: true,
                unlimited_homes: true,
            },
        ],
    }
}

pub fn load_config() -> EssentialsConfig {
    if Path::new(CONFIG_PATH).exists() {
        match fs::read_to_string(CONFIG_PATH)
            .ok()
            .and_then(|raw| serde_json::from_str::<EssentialsConfig>(&raw).ok())
        {
            Some(cfg) => {
                info!("Loaded KRONOS Essentials config from {}", CONFIG_PATH);
                return cfg;
            }
            None => warn!("Invalid config.json, falling back to built-in defaults"),
        }
    }

    let cfg = default_config();
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(&cfg) {
        let _ = fs::write(CONFIG_PATH, pretty);
    }
    cfg
}

pub fn load_store() -> EssentialsStore {
    if Path::new(STORE_PATH).exists() {
        if let Ok(raw) = fs::read_to_string(STORE_PATH) {
            if let Ok(store) = serde_json::from_str::<EssentialsStore>(&raw) {
                info!("Loaded KRONOS Essentials store ({} players)", store.players.len());
                return store;
            }
        }
    }
    EssentialsStore::default()
}

pub fn save_store(store: &EssentialsStore) {
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(store) {
        if let Err(err) = fs::write(STORE_PATH, pretty) {
            warn!("Failed to persist {}: {}", STORE_PATH, err);
        }
    }
}
`,

    'src/commands.rs': `use crate::effects::{self, AVAILABLE_THEMES};
use crate::host;
use crate::menu;
use crate::models::{
    EssentialsConfig, EssentialsStore, HomePoint, Location, PendingTeleport, PlayerRecord,
    TpaRequest, WarpPoint,
};
use crate::storage;
use std::collections::HashMap;

pub fn schedule_teleport(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    player_id: &str,
    target_label: &str,
    destination: Location,
) {
    let now = host::now_ms();
    let player = store
        .players
        .entry(player_id.to_string())
        .or_insert_with(|| PlayerRecord::new(player_id, player_id));
    let rank = config.rank_for(&player.rank);

    // Cooldown check
    let cooldown_ms = rank.cooldown_seconds * 1000;
    let since_last = now.saturating_sub(player.last_teleport_ms);
    if cooldown_ms > 0 && player.last_teleport_ms > 0 && since_last < cooldown_ms {
        let wait_sec = ((cooldown_ms - since_last) + 999) / 1000;
        host::send_chat(
            player_id,
            &format!("§c⏳ Подождите ещё §e{}с §cперед следующим телепортом.", wait_sec),
        );
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.7, 1.0);
        return;
    }

    let origin = host::get_player_location(player_id);
    let theme = player.active_theme(&rank).to_string();

    // Instant teleport for high ranks with warmup_seconds == 0
    if rank.warmup_seconds == 0 {
        player.last_location = Some(origin.clone());
        player.last_teleport_ms = now;
        effects::play_arrival_burst(player_id, target_label, &origin, &destination, &theme, config);
        storage::save_store(store);
        return;
    }

    host::send_chat(
        player_id,
        &format!(
            "§6✦ §fЗапуск телепортации к §e{}§f ({}с). Не двигайтесь!",
            target_label, rank.warmup_seconds
        ),
    );
    host::play_sound(player_id, "minecraft:block.portal.trigger", 0.55, 1.25);

    pending.insert(
        player_id.to_string(),
        PendingTeleport {
            player_id: player_id.to_string(),
            target_label: target_label.to_string(),
            destination,
            origin,
            theme,
            started_at_ms: now,
            duration_ms: rank.warmup_seconds * 1000,
            ticks_elapsed: 0,
            last_second_announced: u64::MAX,
        },
    );
}

pub fn handle_command(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    tpa_requests: &mut HashMap<String, TpaRequest>,
    player_id: &str,
    command: &str,
) -> bool {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    // Ensure player record exists
    store
        .players
        .entry(player_id.to_string())
        .or_insert_with(|| PlayerRecord::new(player_id, player_id));

    let cmd = parts[0].to_lowercase();
    match cmd.as_str() {
        "/spawn" => {
            if let Some(spawn) = store.spawn.clone() {
                schedule_teleport(store, config, pending, player_id, "Спавн", spawn);
            } else {
                host::send_chat(player_id, "§c✖ Точка спавна не установлена. Админ: /setspawn");
            }
            true
        }

        "/setspawn" => {
            let rank = config.rank_for(&store.players[player_id].rank);
            if !rank.can_set_spawn {
                host::send_chat(player_id, "§c✖ Недостаточно прав для установки спавна.");
                return true;
            }
            let loc = host::get_player_location(player_id);
            store.spawn = Some(loc.clone());
            storage::save_store(store);
            host::send_chat(
                player_id,
                &format!("§a✔ Точка спавна установлена: §f{}", loc.coords_label()),
            );
            host::play_sound(player_id, "minecraft:ui.toast.challenge_complete", 0.8, 1.1);
            true
        }

        "/sethome" => {
            let name = parts.get(1).copied().unwrap_or("home").to_lowercase();
            if name.len() > 20 || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                host::send_chat(player_id, "§c✖ Название дома: до 20 символов (буквы, цифры, _ и -).");
                return true;
            }

            let rank = config.rank_for(&store.players[player_id].rank);
            let loc = host::get_player_location(player_id);
            let requested_icon = parts.get(2).copied().map(|s| {
                if s.contains(':') {
                    s.to_string()
                } else {
                    format!("minecraft:{}", s)
                }
            });

            let player = store.players.get_mut(player_id).unwrap();
            if let Some(existing) = player.homes.iter_mut().find(|h| h.name == name) {
                existing.location = loc.clone();
                if let Some(icon) = requested_icon {
                    existing.icon = icon;
                }
                host::send_chat(
                    player_id,
                    &format!("§a✔ Точка дома §e{} §aобновлена: §f{}", name, loc.coords_label()),
                );
            } else {
                if !rank.unlimited_homes && player.homes.len() >= rank.max_homes {
                    let next_info = config
                        .next_rank_after(rank.tier)
                        .map(|n| format!(" (следующий ранг {} даёт {} меток!)", n.display_prefix.trim(), n.max_homes))
                        .unwrap_or_default();
                    host::send_chat(
                        player_id,
                        &format!(
                            "§c✖ Лимит домов исчерпан: §e{}/{}§c{}",
                            player.homes.len(),
                            rank.max_homes,
                            next_info
                        ),
                    );
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 1.0);
                    return true;
                }

                let icon = requested_icon.unwrap_or_else(|| {
                    config
                        .home_icons
                        .get(player.homes.len() % config.home_icons.len().max(1))
                        .cloned()
                        .unwrap_or_else(|| "minecraft:red_bed".to_string())
                });

                player.homes.push(HomePoint {
                    name: name.clone(),
                    icon,
                    location: loc.clone(),
                    created_at_ms: host::now_ms(),
                    uses: 0,
                });

                let limit_str = if rank.unlimited_homes {
                    "∞".to_string()
                } else {
                    rank.max_homes.to_string()
                };
                host::send_chat(
                    player_id,
                    &format!(
                        "§a✔ Дом §e{} §aсохранён! §8[§e{}/{}§8] §7• §f{}",
                        name,
                        player.homes.len(),
                        limit_str,
                        loc.coords_label()
                    ),
                );
            }

            host::spawn_particle(&loc.world, "minecraft:happy_villager", loc.x, loc.y + 1.0, loc.z, 14, 0.25);
            host::play_sound(player_id, "minecraft:entity.player.levelup", 0.7, 1.35);
            storage::save_store(store);
            true
        }

        "/home" => {
            if parts.len() == 1 {
                menu::open_homes_menu(store, config, player_id, 0);
                return true;
            }
            let target = parts[1].to_lowercase();
            let found = store.players.get_mut(player_id).and_then(|p| {
                p.homes.iter_mut().find(|h| h.name == target).map(|h| {
                    h.uses += 1;
                    (h.name.clone(), h.location.clone())
                })
            });

            match found {
                Some((name, loc)) => {
                    schedule_teleport(store, config, pending, player_id, &format!("Дом {}", name), loc);
                }
                None => {
                    host::send_chat(
                        player_id,
                        &format!("§c✖ Дом §e{} §cне найден. Откройте меню: §f/homes", target),
                    );
                }
            }
            true
        }

        "/homes" | "/menu" | "/tp" => {
            menu::open_homes_menu(store, config, player_id, 0);
            true
        }

        "/delhome" => {
            let Some(target) = parts.get(1) else {
                host::send_chat(player_id, "§eИспользование: §f/delhome <название>");
                return true;
            };
            let target = target.to_lowercase();
            let player = store.players.get_mut(player_id).unwrap();
            let before = player.homes.len();
            player.homes.retain(|h| h.name != target);
            if player.homes.len() < before {
                storage::save_store(store);
                host::send_chat(player_id, &format!("§a✔ Дом §e{} §aудалён.", target));
                host::play_sound(player_id, "minecraft:entity.item.break", 0.7, 1.0);
            } else {
                host::send_chat(player_id, &format!("§c✖ Дом §e{} §cне найден.", target));
            }
            true
        }

        "/warps" => {
            menu::open_warps_menu(store, config, player_id);
            true
        }

        "/warp" => {
            let Some(name) = parts.get(1) else {
                menu::open_warps_menu(store, config, player_id);
                return true;
            };
            let key = name.to_lowercase();
            if let Some(warp) = store.warps.get(&key).cloned() {
                let rank = config.rank_for(&store.players[player_id].rank);
                if rank.tier < warp.min_tier {
                    host::send_chat(player_id, "§c🔒 Этот варп доступен с более высокого ранга.");
                    return true;
                }
                schedule_teleport(
                    store,
                    config,
                    pending,
                    player_id,
                    &format!("Варп {}", warp.name),
                    warp.location,
                );
            } else {
                host::send_chat(player_id, &format!("§c✖ Варп §e{} §cне найден. Список: /warps", key));
            }
            true
        }

        "/setwarp" => {
            let rank = config.rank_for(&store.players[player_id].rank);
            if !rank.can_set_warp {
                host::send_chat(player_id, "§c✖ Недостаточно прав для создания варпа.");
                return true;
            }
            let Some(name) = parts.get(1) else {
                host::send_chat(player_id, "§eИспользование: §f/setwarp <название> [описание]");
                return true;
            };
            let key = name.to_lowercase();
            let desc = if parts.len() > 2 {
                parts[2..].join(" ")
            } else {
                "Общая точка телепортации".to_string()
            };
            let loc = host::get_player_location(player_id);
            store.warps.insert(
                key.clone(),
                WarpPoint {
                    name: key.clone(),
                    icon: "minecraft:compass".to_string(),
                    description: desc,
                    location: loc.clone(),
                    min_tier: 0,
                },
            );
            storage::save_store(store);
            host::send_chat(
                player_id,
                &format!("§a✔ Варп §b{} §aсоздан: §f{}", key, loc.coords_label()),
            );
            true
        }

        "/delwarp" => {
            let rank = config.rank_for(&store.players[player_id].rank);
            if !rank.can_set_warp {
                host::send_chat(player_id, "§c✖ Недостаточно прав.");
                return true;
            }
            let Some(name) = parts.get(1) else {
                host::send_chat(player_id, "§eИспользование: §f/delwarp <название>");
                return true;
            };
            if store.warps.remove(&name.to_lowercase()).is_some() {
                storage::save_store(store);
                host::send_chat(player_id, &format!("§a✔ Варп §b{} §aудалён.", name));
            } else {
                host::send_chat(player_id, "§c✖ Варп не найден.");
            }
            true
        }

        "/back" => {
            if let Some(loc) = store.players[player_id].last_location.clone() {
                schedule_teleport(store, config, pending, player_id, "Прошлая точка", loc);
            } else {
                host::send_chat(player_id, "§c✖ Нет сохранённой предыдущей локации.");
            }
            true
        }

        "/tpa" | "/tpahere" => {
            let Some(target_id) = parts.get(1) else {
                host::send_chat(player_id, &format!("§eИспользование: §f{} <игрок>", cmd));
                return true;
            };
            if *target_id == player_id {
                host::send_chat(player_id, "§c✖ Нельзя отправить запрос самому себе.");
                return true;
            }
            let summon_here = cmd == "/tpahere";
            let from_name = store.players[player_id].username.clone();
            let expires = host::now_ms() + config.tpa_timeout_seconds * 1000;

            tpa_requests.insert(
                target_id.to_string(),
                TpaRequest {
                    from_id: player_id.to_string(),
                    from_name: from_name.clone(),
                    to_id: target_id.to_string(),
                    summon_here,
                    expires_at_ms: expires,
                },
            );

            host::send_chat(
                player_id,
                &format!("§6✦ §fЗапрос на телепортацию отправлен игроку §e{}§f.", target_id),
            );
            let action_txt = if summon_here {
                "приглашает вас телепортироваться к себе"
            } else {
                "хочет телепортироваться к вам"
            };
            host::send_chat(
                target_id,
                &format!(
                    "§6✦ §e{} §f{}. Введите §a/tpaccept §fили §c/tpdeny §7({}с)",
                    from_name, action_txt, config.tpa_timeout_seconds
                ),
            );
            host::play_sound(target_id, "minecraft:entity.experience_orb.pickup", 0.8, 1.2);
            true
        }

        "/tpaccept" => {
            let now = host::now_ms();
            match tpa_requests.remove(player_id) {
                Some(req) if now <= req.expires_at_ms => {
                    if req.summon_here {
                        let dest = host::get_player_location(&req.from_id);
                        schedule_teleport(
                            store,
                            config,
                            pending,
                            &req.to_id,
                            &format!("Игрок {}", req.from_name),
                            dest,
                        );
                    } else {
                        let dest = host::get_player_location(&req.to_id);
                        schedule_teleport(
                            store,
                            config,
                            pending,
                            &req.from_id,
                            &format!("Игрок {}", req.to_id),
                            dest,
                        );
                    }
                    host::send_chat(player_id, "§a✔ Запрос на телепортацию принят!");
                }
                _ => {
                    host::send_chat(player_id, "§c✖ У вас нет активных запросов на телепортацию.");
                }
            }
            true
        }

        "/tpdeny" => {
            if let Some(req) = tpa_requests.remove(player_id) {
                host::send_chat(player_id, "§c✖ Вы отклонили запрос на телепортацию.");
                host::send_chat(
                    &req.from_id,
                    &format!("§c✖ Игрок §e{} §cотклонил ваш запрос.", player_id),
                );
            } else {
                host::send_chat(player_id, "§c✖ Нет активных запросов.");
            }
            true
        }

        "/tpeffect" => {
            let Some(theme_arg) = parts.get(1) else {
                let list: Vec<String> = AVAILABLE_THEMES
                    .iter()
                    .map(|(id, label)| format!("§f{} §8({})", id, label))
                    .collect();
                host::send_chat(
                    player_id,
                    &format!("§6✦ Доступные эффекты: {}", list.join("§7, ")),
                );
                return true;
            };
            if let Some((id, label)) = AVAILABLE_THEMES
                .iter()
                .find(|(id, _)| id.eq_ignore_ascii_case(theme_arg))
            {
                if let Some(p) = store.players.get_mut(player_id) {
                    p.selected_theme = Some((*id).to_string());
                    storage::save_store(store);
                }
                host::send_chat(player_id, &format!("§a✔ Выбран эффект телепорта: {}", label));
            } else {
                host::send_chat(player_id, "§c✖ Неизвестный эффект. Введите /tpeffect для списка.");
            }
            true
        }

        "/setrank" => {
            let caller_rank = config.rank_for(&store.players[player_id].rank);
            if !caller_rank.can_set_spawn {
                host::send_chat(player_id, "§c✖ Только администратор может менять статусы.");
                return true;
            }
            if parts.len() < 3 {
                host::send_chat(player_id, "§eИспользование: §f/setrank <игрок> <player|vip|premium|legend|admin|owner>");
                return true;
            }
            let target_id = parts[1];
            let new_rank_id = parts[2].to_lowercase();
            let new_profile = config.rank_for(&new_rank_id);
            let target_player = store
                .players
                .entry(target_id.to_string())
                .or_insert_with(|| PlayerRecord::new(target_id, target_id));
            target_player.rank = new_profile.id.clone();
            storage::save_store(store);

            let limit_txt = if new_profile.unlimited_homes {
                "∞".to_string()
            } else {
                new_profile.max_homes.to_string()
            };
            host::send_chat(
                player_id,
                &format!(
                    "§a✔ Игроку §e{} §aвыдан статус {}§a(лимит домов: §e{}§a)",
                    target_id, new_profile.display_prefix, limit_txt
                ),
            );
            host::send_chat(
                target_id,
                &format!(
                    "§6✦ §fВаш статус обновлён до {}§f! Новый лимит домов: §e{}",
                    new_profile.display_prefix, limit_txt
                ),
            );
            true
        }

        _ => false,
    }
}
`,

    'src/lib.rs': `mod commands;
mod effects;
mod host;
mod menu;
mod models;
mod storage;

use log::info;
use models::{EssentialsConfig, EssentialsStore, PendingTeleport, PlayerRecord, TpaRequest};
use std::collections::HashMap;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: EssentialsConfig,
    store: EssentialsStore,
    pending_teleports: HashMap<String, PendingTeleport>,
    tpa_requests: HashMap<String, TpaRequest>,
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

    let mut guard = STATE.lock().unwrap();
    *guard = Some(PluginState {
        config,
        store,
        pending_teleports: HashMap::new(),
        tpa_requests: HashMap::new(),
    });

    info!("KRONOS Essentials v1.0.0 initialized (Homes, Spawn, Warps, GUI & Teleport FX)");
    0
}

#[no_mangle]
pub extern "C" fn cleanup() {
    let mut guard = STATE.lock().unwrap();
    if let Some(ref state) = *guard {
        storage::save_store(&state.store);
    }
    *guard = None;
    info!("KRONOS Essentials saved state and unloaded cleanly.");
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
        let entry = state
            .store
            .players
            .entry(player_id.to_string())
            .or_insert_with(|| PlayerRecord::new(player_id, name));
        if !name.is_empty() {
            entry.username = name.to_string();
        }
        let rank = state.config.rank_for(&entry.rank);
        let limit = if rank.unlimited_homes {
            "∞".to_string()
        } else {
            rank.max_homes.to_string()
        };
        host::send_chat(
            player_id,
            &format!(
                "§6✦ §fДобро пожаловать, {}§f{}! Ваши метки дома: §e{}/{} §7(/homes)",
                rank.display_prefix,
                entry.username,
                entry.homes.len(),
                limit
            ),
        );
    }
}

#[no_mangle]
pub extern "C" fn on_player_leave(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        state.pending_teleports.remove(player_id);
        state.tpa_requests.remove(player_id);
        storage::save_store(&state.store);
    }
}

/// Called every server tick (20 TPS) to animate spirals, check movement cancellation, and complete teleports.
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };

    if state.pending_teleports.is_empty() {
        return;
    }

    let now = host::now_ms();
    let max_move_sq = state.config.cancel_on_move_blocks * state.config.cancel_on_move_blocks;
    let mut completed: Vec<String> = Vec::new();
    let mut cancelled: Vec<(String, String)> = Vec::new();

    for (pid, pending) in state.pending_teleports.iter_mut() {
        let current_loc = host::get_player_location(pid);
        if current_loc.distance_squared(&pending.origin) > max_move_sq {
            cancelled.push((pid.clone(), "Вы сдвинулись с места".to_string()));
            continue;
        }

        if now.saturating_sub(pending.started_at_ms) >= pending.duration_ms {
            completed.push(pid.clone());
        } else {
            effects::render_warmup_frame(pending, now);
        }
    }

    for (pid, reason) in cancelled {
        if let Some(pending) = state.pending_teleports.remove(&pid) {
            effects::play_cancel_effect(&pid, &pending.origin, &reason);
        }
    }

    let mut store_dirty = false;
    for pid in completed {
        if let Some(pending) = state.pending_teleports.remove(&pid) {
            if let Some(player) = state.store.players.get_mut(&pid) {
                player.last_location = Some(pending.origin.clone());
                player.last_teleport_ms = now;
                store_dirty = true;
            }
            effects::play_arrival_burst(
                &pid,
                &pending.target_label,
                &pending.origin,
                &pending.destination,
                &pending.theme,
                &state.config,
            );
        }
    }

    if store_dirty {
        storage::save_store(&state.store);
    }
}

#[no_mangle]
pub extern "C" fn on_player_damage(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.config.cancel_on_damage {
            if let Some(pending) = state.pending_teleports.remove(player_id) {
                effects::play_cancel_effect(player_id, &pending.origin, "Получен урон");
            }
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
            &mut state.pending_teleports,
            &mut state.tpa_requests,
            player_id,
            command,
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
        let handled = menu::handle_menu_click(
            &mut state.store,
            &state.config,
            &mut state.pending_teleports,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::schedule_teleport,
        );
        if handled {
            storage::save_store(&state.store);
        }
        return handled;
    }
    false
}
`,

    'README.md': `# KRONOS Essentials — Rust WASM Plugin (Pumpkin 26.3)

Полноценный плагин базовых команд, интерактивного GUI-меню телепортов, системы статусов с масштабируемыми лимитами домов и кинематографичных анимаций телепортации.

## Возможности

### 1. Команды
- \`/spawn\` — телепорт на спавн (с отсчётом и анимацией)
- \`/setspawn\` — установить точку спавна (Admin / Owner)
- \`/sethome [название] [иконка]\` — сохранить точку дома с проверкой лимита ранга
- \`/home [название]\` — телепорт в дом (без аргументов открывает GUI-меню)
- \`/homes\` / \`/menu\` — интерактивное 54-слотовое меню домов, спавна, варпов и эффектов
- \`/delhome <название>\` — удалить метку дома
- \`/warp <название>\` / \`/warps\` — телепорт на варп и GUI-список варпов
- \`/setwarp <название> [описание]\` / \`/delwarp <название>\` — управление варпами
- \`/back\` — вернуться в точку до последнего телепорта
- \`/tpa <игрок>\`, \`/tpahere <игрок>\`, \`/tpaccept\`, \`/tpdeny\` — телепортация к игрокам
- \`/tpeffect [тема]\` — смена визуальной темы анимации телепорта
- \`/setrank <игрок> <ранг>\` — выдача статуса (мгновенно обновляет лимит домов)

### 2. Статусы и лимиты меток (\`data/config.json\`)
| Статус | Префикс | Лимит домов | Отсчёт | Кулдаун | Тема анимации |
| :--- | :--- | :---: | :---: | :---: | :--- |
| \`player\` | \`[ИГРОК]\` | **3** | 3с | 15с | Аметистовая спираль |
| \`vip\` | \`[VIP]\` | **5** | 2с | 10с | Золотой вихрь |
| \`premium\` | \`[PREMIUM]\` | **7** | 2с | 6с | Изумрудный импульс |
| \`legend\` | \`[LEGEND]\` | **10** | 1с | 3с | Драконья сверхновая |
| \`admin\` | \`[ADMIN]\` | **∞** | 0с | 0с | Разлом KRONOS |
| \`owner\` | \`[OWNER]\` | **∞** | 0с | 0с | Разлом KRONOS |

> Все лимиты, таймеры и темы редактируются прямо в \`data/config.json\` без пересборки (можно выставить хоть \`3 → 5 → 7\`, хоть \`3 → 6 → 12 → 24\`).

### 3. Эффекты и анимации телепортации (\`src/effects.rs\`)
- **Двойная восходящая спираль частиц** вокруг игрока (20 FPS через \`on_tick\`) + рунический круг под ногами.
- **Прогресс-бар в ActionBar** (\`[■■■■■■□□□□]\`) с обратным отсчётом и нарастающими нотами (\`block.note_block.chime\`).
- **Вспышка прибытия**: 3D-ударная волна из 16 частиц, световой столб, короткий эффект плавного перехода (\`blindness\` 16 тиков + \`slow_falling\` 30 тиков), многослойный звук и Title-баннер с координатами.
- **Защита от движения и урона**: автоматическая отмена с дымовым кольцом и звуковым сигналом при сдвиге или получении урона во время отсчёта.

### 4. Интерактивное Chest GUI-меню (\`src/menu.rs\`)
- **ЛКМ по дому** — запуск телепортации.
- **Shift+ЛКМ по дому** — смена иконки дома (кровать, костёр, кирка, сундук, маяк, звезда Незера и др.).
- **ПКМ по дому** — удаление метки.
- **Клик по зелёному стеклу** — быстрое создание новой метки дома в один клик.
- **Серые замки** — показывают, с какого ранга открываются следующие слоты.
- **Нижняя панель**: Спавн, Варпы, \`/back\`, переключатель стиля анимации и пагинация страниц.

## Сборка под Pumpkin 26.3
\`\`\`bash
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
\`\`\`
Готовый \`.wasm\` файл появится в \`target/wasm32-wasip1/release/kronos_essentials.wasm\` — перенесите его в папку \`plugins/\` вашего сервера.
`,
  },
};
