import { PluginTemplate } from '../types';

export const KRONOS_ESSENTIALS_TEMPLATE: PluginTemplate = {
  id: 'kronos-essentials',
  title: 'KRONOS Essentials v2 (Homes, RTP, Anti-Combat, GUI & FX)',
  description:
    'Full Rust WASM suite: /spawn, /sethome, /home (with sharing & favorites), /rtp meteor drop, /warps, /back (death back for VIP+), /tpa, Anti-Combat PvP lock, Safe Teleport, 5 Chest GUI menus, rank-scaled limits (3 → 5 → 7 → 10 → ∞), rank-locked FX, and atomic autosave.',
  tags: ['Essentials v2', 'RTP & Homes', 'Anti-Combat', '5 GUI Menus'],
  files: {
    'Cargo.toml': `[package]
name = "kronos-essentials"
version = "2.0.0"
edition = "2021"
description = "KRONOS Essentials v2: Homes, Shared Homes, Spawn, Warps, RTP Meteor Drop, TPA, Anti-Combat, Safe Teleport, 5 Chest GUIs, and Cinematic FX for Pumpkin 26.3"

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
  "combat_tag_seconds": 15,
  "safe_teleport_check": true,
  "post_teleport_invulnerability_ticks": 50,
  "teleport_new_players_to_spawn": true,
  "tpa_timeout_seconds": 60,
  "autosave_interval_seconds": 30,
  "blindness_flash_ticks": 16,
  "slow_falling_ticks": 30,
  "rtp": {
    "world": "world",
    "min_radius": 250,
    "max_radius": 4500,
    "cooldown_seconds": 120,
    "drop_height": 22.0,
    "slow_falling_ticks": 140,
    "max_attempts": 12
  },
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
      "max_shared_per_home": 2,
      "warmup_seconds": 3,
      "cooldown_seconds": 15,
      "teleport_theme": "amethyst_spiral",
      "can_set_spawn": false,
      "can_set_warp": false,
      "can_back_on_death": false,
      "unlimited_homes": false
    },
    {
      "id": "vip",
      "tier": 1,
      "display_prefix": "§e§l[VIP] ",
      "max_homes": 5,
      "max_shared_per_home": 4,
      "warmup_seconds": 2,
      "cooldown_seconds": 10,
      "teleport_theme": "golden_vortex",
      "can_set_spawn": false,
      "can_set_warp": false,
      "can_back_on_death": true,
      "unlimited_homes": false
    },
    {
      "id": "premium",
      "tier": 2,
      "display_prefix": "§a§l[PREMIUM] ",
      "max_homes": 7,
      "max_shared_per_home": 6,
      "warmup_seconds": 2,
      "cooldown_seconds": 6,
      "teleport_theme": "emerald_pulse",
      "can_set_spawn": false,
      "can_set_warp": false,
      "can_back_on_death": true,
      "unlimited_homes": false
    },
    {
      "id": "legend",
      "tier": 3,
      "display_prefix": "§d§l[LEGEND] ",
      "max_homes": 10,
      "max_shared_per_home": 10,
      "warmup_seconds": 1,
      "cooldown_seconds": 3,
      "teleport_theme": "dragon_nova",
      "can_set_spawn": false,
      "can_set_warp": true,
      "can_back_on_death": true,
      "unlimited_homes": false
    },
    {
      "id": "admin",
      "tier": 4,
      "display_prefix": "§c§l[ADMIN] ",
      "max_homes": 25,
      "max_shared_per_home": 50,
      "warmup_seconds": 0,
      "cooldown_seconds": 0,
      "teleport_theme": "kronos_rift",
      "can_set_spawn": true,
      "can_set_warp": true,
      "can_back_on_death": true,
      "unlimited_homes": true
    },
    {
      "id": "owner",
      "tier": 5,
      "display_prefix": "§4§l[OWNER] ",
      "max_homes": 50,
      "max_shared_per_home": 50,
      "warmup_seconds": 0,
      "cooldown_seconds": 0,
      "teleport_theme": "kronos_rift",
      "can_set_spawn": true,
      "can_set_warp": true,
      "can_back_on_death": true,
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
`,

    'src/host.rs': `use crate::models::{ChestMenuDescriptor, Location};
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
    fn host_is_location_safe(
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        y: f64,
        z: f64,
    ) -> i32;
    fn host_find_safe_surface_y(
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        z: f64,
    ) -> f64;
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

/// Checks if a destination block has solid ground, 2 blocks of breathing room, and no lava/void.
pub fn is_location_safe(loc: &Location) -> bool {
    if loc.y < -60.0 || loc.y > 320.0 {
        return false;
    }
    #[cfg(target_arch = "wasm32")]
    unsafe {
        return host_is_location_safe(loc.world.as_ptr(), loc.world.len(), loc.x, loc.y, loc.z) != 0;
    }
    #[cfg(not(target_arch = "wasm32"))]
    true
}

/// Finds a safe solid surface Y coordinate for /rtp (returns None for ocean/lava/void).
pub fn find_safe_surface_y(world: &str, x: f64, z: f64) -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let y = host_find_safe_surface_y(world.as_ptr(), world.len(), x, z);
        if y >= -60.0 {
            return Some(y);
        }
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (world, x, z);
        Some(72.0)
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
use crate::models::{ActiveMeteorFall, EssentialsConfig, Location, PendingTeleport};
use std::f64::consts::PI;

pub struct ThemeSpec {
    pub id: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub min_tier: u8,
    pub description: &'static str,
    pub primary: &'static str,
    pub secondary: &'static str,
    pub accent: &'static str,
    pub arrival_sound: &'static str,
    pub title_color: &'static str,
}

pub const THEME_SPECS: &[ThemeSpec] = &[
    ThemeSpec {
        id: "amethyst_spiral",
        label: "§dАметистовая спираль",
        icon: "minecraft:amethyst_shard",
        min_tier: 0,
        description: "Фиолетовая спираль портала и искры аметиста",
        primary: "minecraft:portal",
        secondary: "minecraft:enchant",
        accent: "minecraft:end_rod",
        arrival_sound: "minecraft:block.amethyst_block.chime",
        title_color: "§d",
    },
    ThemeSpec {
        id: "golden_vortex",
        label: "§6Золотой вихрь",
        icon: "minecraft:blaze_powder",
        min_tier: 1,
        description: "Огненно-золотой вихрь с частицами тотема",
        primary: "minecraft:flame",
        secondary: "minecraft:wax_on",
        accent: "minecraft:totem_of_undying",
        arrival_sound: "minecraft:item.totem.use",
        title_color: "§6",
    },
    ThemeSpec {
        id: "emerald_pulse",
        label: "§aИзумрудный импульс",
        icon: "minecraft:emerald",
        min_tier: 2,
        description: "Яркий изумрудный импульс и световые руны",
        primary: "minecraft:happy_villager",
        secondary: "minecraft:glow",
        accent: "minecraft:end_rod",
        arrival_sound: "minecraft:entity.player.levelup",
        title_color: "§a",
    },
    ThemeSpec {
        id: "dragon_nova",
        label: "§5Драконья сверхновая",
        icon: "minecraft:dragon_breath",
        min_tier: 3,
        description: "Дыхание дракона и обратный портальный шторм",
        primary: "minecraft:dragon_breath",
        secondary: "minecraft:reverse_portal",
        accent: "minecraft:end_rod",
        arrival_sound: "minecraft:entity.ender_dragon.flap",
        title_color: "§d",
    },
    ThemeSpec {
        id: "kronos_rift",
        label: "§bРазлом KRONOS",
        icon: "minecraft:nether_star",
        min_tier: 4,
        description: "Пространственный разлом синего пламени душ",
        primary: "minecraft:soul_fire_flame",
        secondary: "minecraft:end_rod",
        accent: "minecraft:enchant",
        arrival_sound: "minecraft:block.beacon.power_select",
        title_color: "§b",
    },
];

pub fn find_theme(theme_id: &str) -> &'static ThemeSpec {
    THEME_SPECS
        .iter()
        .find(|t| t.id.eq_ignore_ascii_case(theme_id))
        .unwrap_or(&THEME_SPECS[0])
}

pub fn can_use_theme(rank_tier: u8, theme_id: &str) -> bool {
    rank_tier >= find_theme(theme_id).min_tier
}

pub fn compute_helix_point(origin: &Location, angle: f64, radius: f64, height: f64) -> (f64, f64, f64) {
    (
        origin.x + radius * angle.cos(),
        origin.y + 0.15 + height,
        origin.z + radius * angle.sin(),
    )
}

/// Renders one 20-TPS animation frame during teleport warmup.
pub fn render_warmup_frame(pending: &mut PendingTeleport, now_ms: u64) {
    let elapsed = now_ms.saturating_sub(pending.started_at_ms);
    let duration = pending.duration_ms.max(1);
    let progress = (elapsed as f64 / duration as f64).clamp(0.0, 1.0);
    let spec = find_theme(&pending.theme);

    let t = pending.ticks_elapsed as f64;
    let base_angle = t * 0.36;
    let radius = 1.2 - progress * 0.55;
    let height = progress * 2.15;

    // 1. Dual ascending helix arms
    for arm in 0..2 {
        let angle = base_angle + (arm as f64) * PI;
        let (px, py, pz) = compute_helix_point(&pending.origin, angle, radius, height);
        host::spawn_particle(&pending.origin.world, spec.primary, px, py, pz, 2, 0.01);
        host::spawn_particle(&pending.origin.world, spec.secondary, px, py + 0.25, pz, 1, 0.02);
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
                spec.accent,
                rx,
                pending.origin.y + 0.08,
                rz,
                1,
                0.0,
            );
        }
    }

    // 3. ActionBar progress bar
    let filled = ((progress * 10.0).round() as usize).min(10);
    let bar = format!("§a{}§8{}", "■".repeat(filled), "□".repeat(10 - filled));
    let remaining_sec = ((duration.saturating_sub(elapsed) + 999) / 1000).max(1);
    host::send_action_bar(
        &pending.player_id,
        &format!(
            "{}✦ §fТелепорт: §e{} §8[{}] §f{}с §7(не двигайтесь)",
            spec.title_color, pending.target_label, bar, remaining_sec
        ),
    );

    // 4. Harmonic chime once per second
    if remaining_sec != pending.last_second_announced {
        pending.last_second_announced = remaining_sec;
        let pitch = 0.85 + (progress as f32) * 0.75;
        host::play_sound(&pending.player_id, "minecraft:block.note_block.chime", 0.75, pitch);
    }

    pending.ticks_elapsed = pending.ticks_elapsed.saturating_add(1);
}

/// Executes standard arrival burst + post-teleport invulnerability shield.
pub fn play_arrival_burst(
    player_id: &str,
    target_label: &str,
    origin: &Location,
    dest: &Location,
    theme: &str,
    config: &EssentialsConfig,
) {
    let spec = find_theme(theme);

    host::spawn_particle(&origin.world, spec.primary, origin.x, origin.y + 1.0, origin.z, 24, 0.18);
    host::teleport_player(player_id, dest);

    let ring_count = 16;
    for i in 0..ring_count {
        let a = (i as f64 / ring_count as f64) * 2.0 * PI;
        let rx = dest.x + 1.15 * a.cos();
        let rz = dest.z + 1.15 * a.sin();
        host::spawn_particle(&dest.world, spec.primary, rx, dest.y + 0.35, rz, 2, 0.04);
        host::spawn_particle(&dest.world, spec.accent, rx, dest.y + 1.2, rz, 1, 0.05);
    }

    for step in 0..7 {
        let py = dest.y + 0.25 + (step as f64) * 0.4;
        host::spawn_particle(&dest.world, spec.secondary, dest.x, py, dest.z, 2, 0.02);
    }

    host::play_sound(player_id, "minecraft:entity.enderman.teleport", 0.85, 1.05);
    host::play_sound(player_id, spec.arrival_sound, 0.65, 1.2);

    if config.blindness_flash_ticks > 0 {
        host::apply_effect(player_id, "minecraft:blindness", config.blindness_flash_ticks, 0);
    }
    if config.slow_falling_ticks > 0 {
        host::apply_effect(player_id, "minecraft:slow_falling", config.slow_falling_ticks, 0);
    }
    if config.post_teleport_invulnerability_ticks > 0 {
        host::apply_effect(
            player_id,
            "minecraft:resistance",
            config.post_teleport_invulnerability_ticks,
            4,
        );
    }

    host::send_title(
        player_id,
        &format!("{}✦ {}", spec.title_color, target_label),
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

/// Launches the cinematic /rtp meteor drop from the sky with slow falling + fiery trail.
pub fn play_rtp_meteor_launch(
    player_id: &str,
    origin: &Location,
    surface_dest: &Location,
    config: &EssentialsConfig,
    now_ms: u64,
) -> ActiveMeteorFall {
    let mut sky_dest = surface_dest.clone();
    sky_dest.y += config.rtp.drop_height;

    host::spawn_particle(&origin.world, "minecraft:flame", origin.x, origin.y + 1.0, origin.z, 28, 0.2);
    host::teleport_player(player_id, &sky_dest);

    host::apply_effect(
        player_id,
        "minecraft:slow_falling",
        config.rtp.slow_falling_ticks,
        0,
    );
    host::apply_effect(
        player_id,
        "minecraft:resistance",
        config.rtp.slow_falling_ticks,
        4,
    );
    host::play_sound(player_id, "minecraft:entity.firework_rocket.launch", 0.9, 0.9);
    host::play_sound(player_id, "minecraft:entity.ender_dragon.flap", 0.7, 1.15);

    host::send_title(
        player_id,
        "§6☄ Метеоритный десант",
        &format!("§7Сектор: §f{}", surface_dest.coords_label()),
        5,
        45,
        15,
    );

    ActiveMeteorFall {
        player_id: player_id.to_string(),
        world: sky_dest.world,
        expires_at_ms: now_ms + (config.rtp.slow_falling_ticks as u64) * 50,
        ticks: 0,
    }
}

pub fn render_meteor_trail_frame(fall: &mut ActiveMeteorFall) {
    if fall.ticks % 2 == 0 {
        let loc = host::get_player_location(&fall.player_id);
        host::spawn_particle(&fall.world, "minecraft:flame", loc.x, loc.y + 0.4, loc.z, 4, 0.06);
        host::spawn_particle(&fall.world, "minecraft:end_rod", loc.x, loc.y + 0.9, loc.z, 2, 0.04);
        host::spawn_particle(&fall.world, "minecraft:cloud", loc.x, loc.y + 0.1, loc.z, 2, 0.03);
    }
    fall.ticks = fall.ticks.saturating_add(1);
}

pub fn play_cancel_effect(player_id: &str, origin: &Location, reason: &str) {
    host::spawn_particle(&origin.world, "minecraft:smoke", origin.x, origin.y + 0.9, origin.z, 16, 0.08);
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

    'src/menu.rs': `use crate::effects::{can_use_theme, find_theme, THEME_SPECS};
use crate::host;
use crate::models::{
    ChestMenuDescriptor, CombatTag, EssentialsConfig, EssentialsStore, HomePoint, Location,
    MenuSlot, PendingTeleport, TpaRequest,
};
use std::collections::HashMap;

pub const HOME_GRID_SLOTS: [u8; 14] = [
    19, 20, 21, 22, 23, 24, 25,
    28, 29, 30, 31, 32, 33, 34,
];

const BORDER_SLOTS: [u8; 25] = [
    0, 1, 2, 3, 5, 6, 7, 8,
    9, 10, 11, 12, 13, 14, 15, 16, 17,
    18, 26, 27, 35, 36, 44,
    45, 53,
];

/// 1. Main 54-slot KRONOS Homes & Teleports Hub
pub fn open_homes_menu(
    store: &EssentialsStore,
    config: &EssentialsConfig,
    tpa_requests: &HashMap<String, TpaRequest>,
    combat_tags: &HashMap<String, CombatTag>,
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
    let now = host::now_ms();
    let in_combat = combat_tags
        .get(player_id)
        .map(|c| c.expires_at_ms > now)
        .unwrap_or(false);
    let has_tpa = tpa_requests
        .get(player_id)
        .map(|r| r.expires_at_ms > now)
        .unwrap_or(false);

    let mut slots: Vec<MenuSlot> = Vec::with_capacity(54);

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

    // Top-center Player Status Card (slot 4)
    let active_theme = find_theme(player.active_theme(&rank));
    let fav_label = player
        .favorite_home()
        .map(|h| format!("§6★ {}", h.name))
        .unwrap_or_else(|| "§8не выбран".to_string());
    let next_hint = match config.next_rank_after(rank.tier) {
        Some(next) if !rank.unlimited_homes => format!(
            "§7Следующий ранг {}: §e{} меток",
            next.display_prefix.trim(),
            next.max_homes
        ),
        _ => "§aМаксимальный лимит меток открыт!".to_string(),
    };

    slots.push(MenuSlot {
        slot: 4,
        item: "minecraft:player_head".to_string(),
        name: format!("{}§f{}", rank.display_prefix, player.username),
        lore: vec![
            format!("§7Точки дома: §e{} §8/ §a{}", player.homes.len(), limit_text),
            format!("§7Главный дом: {}", fav_label),
            format!("§7Задержка / Кулдаун: §f{}с §8/ §f{}с", rank.warmup_seconds, rank.cooldown_seconds),
            format!("§7Эффект: {}", active_theme.label),
            if in_combat {
                "§c⚔ Вы в бою! Телепорты временно заблокированы".to_string()
            } else {
                "§a✔ Статус: Безопасно".to_string()
            },
            "§8────────────────────".to_string(),
            next_hint,
        ],
        enchanted: true,
    });

    // 14 Home Slots
    let per_page = HOME_GRID_SLOTS.len();
    let offset = page * per_page;

    for (idx, &slot_num) in HOME_GRID_SLOTS.iter().enumerate() {
        let home_index = offset + idx;
        if let Some(home) = player.homes.get(home_index) {
            let star = if home.is_favorite { "§6★ " } else { "" };
            let shared_line = if home.shared_with.is_empty() {
                "§7Доступ: §8только вы".to_string()
            } else {
                format!("§7Друзья ({}): §b{}", home.shared_with.len(), home.shared_with.join(", "))
            };
            slots.push(MenuSlot {
                slot: slot_num,
                item: home.icon.clone(),
                name: format!("{}§6✦ §eДом: §f{}", star, home.name),
                lore: vec![
                    format!("§7Координаты: §f{}", home.location.coords_label()),
                    format!("§7Использований: §b{}", home.uses),
                    shared_line,
                    "§8────────────────────".to_string(),
                    "§a▸ ЛКМ §8— §fТелепортироваться".to_string(),
                    "§e▸ Shift+ЛКМ §8— §fНастройки дома (★, иконка, перенос)".to_string(),
                    "§c▸ ПКМ §8— §fУдалить (с подтверждением)".to_string(),
                ],
                enchanted: home.is_favorite,
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
                    "§7Лимиты статусов: §e3 → 5 → 7 → 10 → ∞".to_string(),
                ],
                enchanted: false,
            });
        }
    }

    // Bottom Navigation Bar (Row 6)
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
        slot: 46,
        item: "minecraft:firework_rocket".to_string(),
        name: "§6☄ Случайный телепорт (/rtp)".to_string(),
        lore: vec![
            "§7Безопасный поиск суши".to_string(),
            "§7+ метеоритный спуск с неба!".to_string(),
            "§a▸ Нажмите для запуска".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 47,
        item: "minecraft:nether_star".to_string(),
        name: "§6✦ Телепорт на Спавн".to_string(),
        lore: vec![match &store.spawn {
            Some(loc) => format!("§7Точка: §f{}", loc.coords_label()),
            None => "§cСпавн не установлен (/setspawn)".to_string(),
        }],
        enchanted: store.spawn.is_some(),
    });

    slots.push(MenuSlot {
        slot: 48,
        item: "minecraft:compass".to_string(),
        name: "§b✦ Варпы Сервера".to_string(),
        lore: vec![format!("§7Доступно варпов: §e{}", store.warps.len())],
        enchanted: false,
    });

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:ender_pearl".to_string(),
        name: "§d✦ Вернуться назад (/back)".to_string(),
        lore: vec![
            match &player.last_location {
                Some(loc) => format!("§7Прошлая точка: §f{}", loc.coords_label()),
                None => "§8Нет прошлой точки".to_string(),
            },
            if rank.can_back_on_death {
                match &player.last_death_location {
                    Some(d) => format!("§c☠ Точка смерти: §f{}", d.coords_label()),
                    None => "§7Возврат на точку смерти: §aдоступен".to_string(),
                }
            } else {
                "§8☠ Возврат на точку смерти: с VIP+".to_string()
            },
        ],
        enchanted: player.last_location.is_some() || (rank.can_back_on_death && player.last_death_location.is_some()),
    });

    slots.push(MenuSlot {
        slot: 50,
        item: "minecraft:amethyst_shard".to_string(),
        name: "§d✦ Меню эффектов телепорта".to_string(),
        lore: vec![
            format!("§7Активен: {}", active_theme.label),
            "§e▸ Открыть каталог анимаций".to_string(),
        ],
        enchanted: true,
    });

    slots.push(MenuSlot {
        slot: 51,
        item: "minecraft:bell".to_string(),
        name: if has_tpa {
            "§a✦ Входящий TPA-запрос! (1)".to_string()
        } else {
            "§7✦ TPA-запросы (0)".to_string()
        },
        lore: vec![if has_tpa {
            "§a▸ Нажмите, чтобы принять или отклонить".to_string()
        } else {
            "§8Нет активных входящих запросов".to_string()
        }],
        enchanted: has_tpa,
    });

    slots.push(MenuSlot {
        slot: 52,
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

/// 2. Confirm Delete Modal (27 slots) — prevents accidental right-click deletion
pub fn open_confirm_delete_menu(store: &EssentialsStore, player_id: &str, home_name: &str) {
    let Some(home) = store
        .players
        .get(player_id)
        .and_then(|p| p.homes.iter().find(|h| h.name == home_name))
    else {
        return;
    };

    let slots = vec![
        MenuSlot {
            slot: 11,
            item: "minecraft:lime_concrete".to_string(),
            name: "§a✔ Подтвердить удаление".to_string(),
            lore: vec![format!("§7Удалить точку дома §e{} §7навсегда", home.name)],
            enchanted: false,
        },
        MenuSlot {
            slot: 13,
            item: home.icon.clone(),
            name: format!("§6✦ Дом: §f{}", home.name),
            lore: vec![
                format!("§7Координаты: §f{}", home.location.coords_label()),
                format!("§7Использований: §b{}", home.uses),
            ],
            enchanted: true,
        },
        MenuSlot {
            slot: 15,
            item: "minecraft:red_concrete".to_string(),
            name: "§c✖ Отмена".to_string(),
            lore: vec!["§7Вернуться обратно к списку домов".to_string()],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("kronos_confirm_delete:{}", home.name),
            title: format!("§cУдалить дом {}?", home.name),
            rows: 3,
            slots,
        },
    );
}

/// 3. Home Settings Submenu (27 slots): Favorite ★, Move Here, Cycle Icon, Delete
pub fn open_home_edit_menu(store: &EssentialsStore, player_id: &str, home_name: &str) {
    let Some(home) = store
        .players
        .get(player_id)
        .and_then(|p| p.homes.iter().find(|h| h.name == home_name))
    else {
        return;
    };

    let slots = vec![
        MenuSlot {
            slot: 10,
            item: "minecraft:gold_ingot".to_string(),
            name: if home.is_favorite {
                "§6★ Главный дом (активен)".to_string()
            } else {
                "§e☆ Сделать главным домом".to_string()
            },
            lore: vec!["§7Отмечает дом звёздочкой и поднимает статус".to_string()],
            enchanted: home.is_favorite,
        },
        MenuSlot {
            slot: 12,
            item: "minecraft:ender_eye".to_string(),
            name: "§b✦ Переместить точку сюда".to_string(),
            lore: vec![
                format!("§7Сейчас: §f{}", home.location.coords_label()),
                "§a▸ Обновит координаты на вашу текущую позицию".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 14,
            item: home.icon.clone(),
            name: "§e✦ Сменить иконку".to_string(),
            lore: vec![
                format!("§7Текущая: §f{}", home.icon),
                "§a▸ Нажмите для переключения".to_string(),
            ],
            enchanted: false,
        },
        MenuSlot {
            slot: 16,
            item: "minecraft:tnt".to_string(),
            name: "§c✖ Удалить метку".to_string(),
            lore: vec!["§7Откроет окно подтверждения".to_string()],
            enchanted: false,
        },
        MenuSlot {
            slot: 22,
            item: "minecraft:arrow".to_string(),
            name: "§e◀ Назад к Домам".to_string(),
            lore: vec![],
            enchanted: false,
        },
    ];

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: format!("kronos_home_edit:{}", home.name),
            title: format!("§8Настройки дома: §6{}", home.name),
            rows: 3,
            slots,
        },
    );
}

/// 4. Rank-Locked Teleport Effects Catalog (27 slots)
pub fn open_effects_menu(store: &EssentialsStore, config: &EssentialsConfig, player_id: &str) {
    let Some(player) = store.players.get(player_id) else {
        return;
    };
    let rank = config.rank_for(&player.rank);
    let active_id = player.active_theme(&rank);
    let effect_slots: [u8; 5] = [11, 12, 13, 14, 15];

    let mut slots: Vec<MenuSlot> = Vec::new();
    for (idx, theme) in THEME_SPECS.iter().enumerate() {
        let unlocked = rank.tier >= theme.min_tier;
        let is_selected = theme.id == active_id;
        slots.push(MenuSlot {
            slot: effect_slots[idx],
            item: if unlocked {
                theme.icon.to_string()
            } else {
                "minecraft:gray_dye".to_string()
            },
            name: if is_selected {
                format!("{} §a✔", theme.label)
            } else {
                theme.label.to_string()
            },
            lore: vec![
                format!("§7{}", theme.description),
                "§8────────────────────".to_string(),
                if is_selected {
                    "§aВыбрано сейчас".to_string()
                } else if unlocked {
                    "§e▸ Нажмите, чтобы активировать".to_string()
                } else {
                    format!(
                        "§c🔒 Требуется статус {}",
                        config.rank_name_for_tier(theme.min_tier)
                    )
                },
            ],
            enchanted: is_selected,
        });
    }

    slots.push(MenuSlot {
        slot: 22,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_effects".to_string(),
            title: "§8✦ §dЭффекты телепортации".to_string(),
            rows: 3,
            slots,
        },
    );
}

/// 5. Incoming TPA Manager Menu (27 slots)
pub fn open_tpa_menu(tpa_requests: &HashMap<String, TpaRequest>, player_id: &str) {
    let now = host::now_ms();
    let mut slots: Vec<MenuSlot> = Vec::new();

    if let Some(req) = tpa_requests.get(player_id).filter(|r| r.expires_at_ms > now) {
        let rem = (req.expires_at_ms - now + 999) / 1000;
        let kind = if req.summon_here {
            "приглашает вас к себе"
        } else {
            "хочет телепортироваться к вам"
        };
        slots.push(MenuSlot {
            slot: 11,
            item: "minecraft:lime_concrete".to_string(),
            name: "§a✔ Принять запрос (/tpaccept)".to_string(),
            lore: vec![format!("§7Игрок §e{} §7{}", req.from_name, kind)],
            enchanted: true,
        });
        slots.push(MenuSlot {
            slot: 13,
            item: "minecraft:player_head".to_string(),
            name: format!("§6✦ Запрос от §e{}", req.from_name),
            lore: vec![
                format!("§7{}", kind),
                format!("§7Истекает через: §e{}с", rem),
            ],
            enchanted: false,
        });
        slots.push(MenuSlot {
            slot: 15,
            item: "minecraft:red_concrete".to_string(),
            name: "§c✖ Отклонить (/tpdeny)".to_string(),
            lore: vec![],
            enchanted: false,
        });
    } else {
        slots.push(MenuSlot {
            slot: 13,
            item: "minecraft:paper".to_string(),
            name: "§7Нет входящих TPA-запросов".to_string(),
            lore: vec!["§8Отправить запрос: §f/tpa <игрок>".to_string()],
            enchanted: false,
        });
    }

    slots.push(MenuSlot {
        slot: 22,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_tpa".to_string(),
            title: "§8✦ §eВходящие TPA-запросы".to_string(),
            rows: 3,
            slots,
        },
    );
}

/// Server Warps Menu (54 slots)
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
                    format!("§c🔒 Требуется {}", config.rank_name_for_tier(warp.min_tier))
                },
            ],
            enchanted: unlocked,
        });
    }

    slots.push(MenuSlot {
        slot: 49,
        item: "minecraft:arrow".to_string(),
        name: "§e◀ Назад к Домам".to_string(),
        lore: vec![],
        enchanted: false,
    });

    host::open_chest_menu(
        player_id,
        &ChestMenuDescriptor {
            menu_id: "kronos_warps".to_string(),
            title: "§8✦ §bKRONOS Варпы Сервера".to_string(),
            rows: 6,
            slots,
        },
    );
}

pub struct MenuDispatchContext<'a> {
    pub store: &'a mut EssentialsStore,
    pub config: &'a EssentialsConfig,
    pub pending: &'a mut HashMap<String, PendingTeleport>,
    pub tpa_requests: &'a mut HashMap<String, TpaRequest>,
    pub combat_tags: &'a HashMap<String, CombatTag>,
    pub dirty: &'a mut bool,
}

pub fn handle_menu_click(
    ctx: &mut MenuDispatchContext<'_>,
    player_id: &str,
    menu_id: &str,
    slot: u8,
    click_type: &str,
    schedule_tp: impl Fn(
        &mut EssentialsStore,
        &EssentialsConfig,
        &mut HashMap<String, PendingTeleport>,
        &HashMap<String, CombatTag>,
        &str,
        &str,
        Location,
        bool,
        &mut bool,
    ),
    trigger_rtp: impl Fn(
        &mut EssentialsStore,
        &EssentialsConfig,
        &mut HashMap<String, PendingTeleport>,
        &HashMap<String, CombatTag>,
        &str,
        &mut bool,
    ),
) -> bool {
    // Confirm Delete Modal
    if let Some(home_name) = menu_id.strip_prefix("kronos_confirm_delete:") {
        match slot {
            11 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    let before = player.homes.len();
                    player.homes.retain(|h| h.name != home_name);
                    if player.homes.len() < before {
                        *ctx.dirty = true;
                        host::send_chat(player_id, &format!("§c✖ Дом §e{} §cудалён.", home_name));
                        host::play_sound(player_id, "minecraft:entity.item.break", 0.7, 1.0);
                    }
                }
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            15 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Home Settings Submenu
    if let Some(home_name) = menu_id.strip_prefix("kronos_home_edit:") {
        match slot {
            10 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    for h in &mut player.homes {
                        h.is_favorite = h.name == home_name;
                    }
                    *ctx.dirty = true;
                    host::send_chat(player_id, &format!("§6★ Дом §e{} §6назначен главным!", home_name));
                    host::play_sound(player_id, "minecraft:entity.player.levelup", 0.65, 1.4);
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            12 => {
                let new_loc = host::get_player_location(player_id);
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    if let Some(h) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        h.location = new_loc.clone();
                        *ctx.dirty = true;
                        host::send_chat(
                            player_id,
                            &format!("§a✔ Координаты дома §e{} §aобновлены: §f{}", home_name, new_loc.coords_label()),
                        );
                        host::play_sound(player_id, "minecraft:entity.experience_orb.pickup", 0.7, 1.3);
                    }
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            14 => {
                if let Some(player) = ctx.store.players.get_mut(player_id) {
                    if let Some(h) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        if !ctx.config.home_icons.is_empty() {
                            let pos = ctx
                                .config
                                .home_icons
                                .iter()
                                .position(|i| i == &h.icon)
                                .unwrap_or(0);
                            h.icon = ctx.config.home_icons[(pos + 1) % ctx.config.home_icons.len()].clone();
                            *ctx.dirty = true;
                            host::play_sound(player_id, "minecraft:ui.button.click", 0.65, 1.3);
                        }
                    }
                }
                open_home_edit_menu(ctx.store, player_id, home_name);
            }
            16 => {
                open_confirm_delete_menu(ctx.store, player_id, home_name);
            }
            22 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Effects Catalog Menu
    if menu_id == "kronos_effects" {
        if slot == 22 {
            open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            return true;
        }
        let effect_slots: [u8; 5] = [11, 12, 13, 14, 15];
        if let Some(idx) = effect_slots.iter().position(|&s| s == slot) {
            let theme = &THEME_SPECS[idx];
            if let Some(player) = ctx.store.players.get_mut(player_id) {
                let rank = ctx.config.rank_for(&player.rank);
                if !can_use_theme(rank.tier, theme.id) {
                    host::send_chat(
                        player_id,
                        &format!(
                            "§c🔒 Эффект {} §cдоступен с ранга {}",
                            theme.label,
                            ctx.config.rank_name_for_tier(theme.min_tier)
                        ),
                    );
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 0.9);
                } else {
                    player.selected_theme = Some(theme.id.to_string());
                    *ctx.dirty = true;
                    host::send_chat(player_id, &format!("§6✦ §fЭффект телепортации: {}", theme.label));
                    host::play_sound(player_id, theme.arrival_sound, 0.7, 1.2);
                }
            }
            open_effects_menu(ctx.store, ctx.config, player_id);
        }
        return true;
    }

    // Incoming TPA Menu
    if menu_id == "kronos_tpa" {
        match slot {
            11 => {
                let now = host::now_ms();
                if let Some(req) = ctx.tpa_requests.remove(player_id) {
                    if now <= req.expires_at_ms {
                        host::close_menu(player_id);
                        if req.summon_here {
                            let dest = host::get_player_location(&req.from_id);
                            schedule_tp(
                                ctx.store,
                                ctx.config,
                                ctx.pending,
                                ctx.combat_tags,
                                &req.to_id,
                                &format!("Игрок {}", req.from_name),
                                dest,
                                false,
                                ctx.dirty,
                            );
                        } else {
                            let dest = host::get_player_location(&req.to_id);
                            schedule_tp(
                                ctx.store,
                                ctx.config,
                                ctx.pending,
                                ctx.combat_tags,
                                &req.from_id,
                                &format!("Игрок {}", req.to_id),
                                dest,
                                false,
                                ctx.dirty,
                            );
                        }
                        host::send_chat(player_id, "§a✔ TPA-запрос принят!");
                    }
                }
            }
            15 => {
                if let Some(req) = ctx.tpa_requests.remove(player_id) {
                    host::send_chat(player_id, "§c✖ Запрос отклонён.");
                    host::send_chat(&req.from_id, &format!("§c✖ {} отклонил ваш TPA-запрос.", player_id));
                }
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            22 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            }
            _ => {}
        }
        return true;
    }

    // Warps Menu
    if menu_id == "kronos_warps" {
        if slot == 49 {
            open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, 0);
            return true;
        }
        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let mut sorted_warps: Vec<_> = ctx.store.warps.values().cloned().collect();
            sorted_warps.sort_by(|a, b| a.name.cmp(&b.name));
            if let Some(warp) = sorted_warps.get(grid_idx) {
                let rank = ctx
                    .store
                    .players
                    .get(player_id)
                    .map(|p| ctx.config.rank_for(&p.rank))
                    .unwrap_or_else(|| ctx.config.ranks[0].clone());
                if rank.tier < warp.min_tier {
                    host::send_chat(player_id, "§c🔒 Недостаточный статус для этого варпа.");
                    host::play_sound(player_id, "minecraft:entity.villager.no", 0.8, 1.0);
                    return true;
                }
                host::close_menu(player_id);
                schedule_tp(
                    ctx.store,
                    ctx.config,
                    ctx.pending,
                    ctx.combat_tags,
                    player_id,
                    &format!("Варп {}", warp.name),
                    warp.location.clone(),
                    false,
                    ctx.dirty,
                );
            }
        }
        return true;
    }

    // Main Homes Hub
    if let Some(page_str) = menu_id.strip_prefix("kronos_homes:") {
        let page: usize = page_str.parse().unwrap_or(0);
        match slot {
            45 if page > 0 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page - 1);
                return true;
            }
            53 => {
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page + 1);
                return true;
            }
            46 => {
                host::close_menu(player_id);
                trigger_rtp(ctx.store, ctx.config, ctx.pending, ctx.combat_tags, player_id, ctx.dirty);
                return true;
            }
            47 => {
                host::close_menu(player_id);
                if let Some(spawn) = ctx.store.spawn.clone() {
                    schedule_tp(
                        ctx.store,
                        ctx.config,
                        ctx.pending,
                        ctx.combat_tags,
                        player_id,
                        "Спавн",
                        spawn,
                        false,
                        ctx.dirty,
                    );
                } else {
                    host::send_chat(player_id, "§c✖ Спавн ещё не задан (/setspawn).");
                }
                return true;
            }
            48 => {
                open_warps_menu(ctx.store, ctx.config, player_id);
                return true;
            }
            49 => {
                host::close_menu(player_id);
                let target = ctx.store.players.get(player_id).and_then(|p| {
                    let rank = ctx.config.rank_for(&p.rank);
                    if rank.can_back_on_death {
                        p.last_death_location.clone().or_else(|| p.last_location.clone())
                    } else {
                        p.last_location.clone()
                    }
                });
                if let Some(loc) = target {
                    schedule_tp(
                        ctx.store,
                        ctx.config,
                        ctx.pending,
                        ctx.combat_tags,
                        player_id,
                        "Прошлая точка",
                        loc,
                        false,
                        ctx.dirty,
                    );
                } else {
                    host::send_chat(player_id, "§c✖ Нет сохранённой точки для /back.");
                }
                return true;
            }
            50 => {
                open_effects_menu(ctx.store, ctx.config, player_id);
                return true;
            }
            51 => {
                open_tpa_menu(ctx.tpa_requests, player_id);
                return true;
            }
            52 => {
                host::close_menu(player_id);
                return true;
            }
            _ => {}
        }

        if let Some(grid_idx) = HOME_GRID_SLOTS.iter().position(|&s| s == slot) {
            let home_idx = page * HOME_GRID_SLOTS.len() + grid_idx;
            let Some(player) = ctx.store.players.get_mut(player_id) else {
                return true;
            };
            let rank = ctx.config.rank_for(&player.rank);

            if home_idx < player.homes.len() {
                let home_name = player.homes[home_idx].name.clone();
                match click_type {
                    "right" => {
                        open_confirm_delete_menu(ctx.store, player_id, &home_name);
                    }
                    "shift_left" => {
                        open_home_edit_menu(ctx.store, player_id, &home_name);
                    }
                    _ => {
                        player.homes[home_idx].uses += 1;
                        let dest = player.homes[home_idx].location.clone();
                        *ctx.dirty = true;
                        host::close_menu(player_id);
                        schedule_tp(
                            ctx.store,
                            ctx.config,
                            ctx.pending,
                            ctx.combat_tags,
                            player_id,
                            &format!("Дом {}", home_name),
                            dest,
                            false,
                            ctx.dirty,
                        );
                    }
                }
            } else if rank.unlimited_homes || home_idx < rank.max_homes {
                let auto_name = format!("home{}", player.homes.len() + 1);
                let loc = host::get_player_location(player_id);
                let icon = ctx
                    .config
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
                    is_favorite: player.homes.is_empty(),
                    shared_with: Vec::new(),
                });
                *ctx.dirty = true;
                host::send_chat(player_id, &format!("§a✔ Быстро создана метка §e{}§a!", auto_name));
                host::play_sound(player_id, "minecraft:entity.player.levelup", 0.65, 1.4);
                open_homes_menu(ctx.store, ctx.config, ctx.tpa_requests, ctx.combat_tags, player_id, page);
            } else {
                host::send_chat(
                    player_id,
                    "§c🔒 Слот заблокирован. Повысьте статус для расширения лимита домов!",
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

    'src/storage.rs': `use crate::models::{EssentialsConfig, EssentialsStore, RankProfile, RtpConfig};
use log::{info, warn};
use std::fs;
use std::path::Path;

const DATA_DIR: &str = "data/kronos-essentials";
const CONFIG_PATH: &str = "data/kronos-essentials/config.json";
const STORE_PATH: &str = "data/kronos-essentials/store.json";
const STORE_TMP_PATH: &str = "data/kronos-essentials/store.json.tmp";

pub fn default_config() -> EssentialsConfig {
    EssentialsConfig {
        plugin_name: "kronos-essentials".to_string(),
        cancel_on_move_blocks: 0.45,
        cancel_on_damage: true,
        combat_tag_seconds: 15,
        safe_teleport_check: true,
        post_teleport_invulnerability_ticks: 50,
        teleport_new_players_to_spawn: true,
        tpa_timeout_seconds: 60,
        autosave_interval_seconds: 30,
        blindness_flash_ticks: 16,
        slow_falling_ticks: 30,
        rtp: RtpConfig {
            world: "world".to_string(),
            min_radius: 250,
            max_radius: 4500,
            cooldown_seconds: 120,
            drop_height: 22.0,
            slow_falling_ticks: 140,
            max_attempts: 12,
        },
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
                max_shared_per_home: 2,
                warmup_seconds: 3,
                cooldown_seconds: 15,
                teleport_theme: "amethyst_spiral".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                can_back_on_death: false,
                unlimited_homes: false,
            },
            RankProfile {
                id: "vip".to_string(),
                tier: 1,
                display_prefix: "§e§l[VIP] ".to_string(),
                max_homes: 5,
                max_shared_per_home: 4,
                warmup_seconds: 2,
                cooldown_seconds: 10,
                teleport_theme: "golden_vortex".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                can_back_on_death: true,
                unlimited_homes: false,
            },
            RankProfile {
                id: "premium".to_string(),
                tier: 2,
                display_prefix: "§a§l[PREMIUM] ".to_string(),
                max_homes: 7,
                max_shared_per_home: 6,
                warmup_seconds: 2,
                cooldown_seconds: 6,
                teleport_theme: "emerald_pulse".to_string(),
                can_set_spawn: false,
                can_set_warp: false,
                can_back_on_death: true,
                unlimited_homes: false,
            },
            RankProfile {
                id: "legend".to_string(),
                tier: 3,
                display_prefix: "§d§l[LEGEND] ".to_string(),
                max_homes: 10,
                max_shared_per_home: 10,
                warmup_seconds: 1,
                cooldown_seconds: 3,
                teleport_theme: "dragon_nova".to_string(),
                can_set_spawn: false,
                can_set_warp: true,
                can_back_on_death: true,
                unlimited_homes: false,
            },
            RankProfile {
                id: "admin".to_string(),
                tier: 4,
                display_prefix: "§c§l[ADMIN] ".to_string(),
                max_homes: 25,
                max_shared_per_home: 50,
                warmup_seconds: 0,
                cooldown_seconds: 0,
                teleport_theme: "kronos_rift".to_string(),
                can_set_spawn: true,
                can_set_warp: true,
                can_back_on_death: true,
                unlimited_homes: true,
            },
            RankProfile {
                id: "owner".to_string(),
                tier: 5,
                display_prefix: "§4§l[OWNER] ".to_string(),
                max_homes: 50,
                max_shared_per_home: 50,
                warmup_seconds: 0,
                cooldown_seconds: 0,
                teleport_theme: "kronos_rift".to_string(),
                can_set_spawn: true,
                can_set_warp: true,
                can_back_on_death: true,
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
                info!("Loaded KRONOS Essentials v2 config from {}", CONFIG_PATH);
                return cfg;
            }
            None => warn!("Invalid config.json, using built-in defaults"),
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

/// Atomic crash-safe write: writes to store.json.tmp first, then atomically renames to store.json.
pub fn save_store_atomic(store: &EssentialsStore) {
    let _ = fs::create_dir_all(DATA_DIR);
    if let Ok(pretty) = serde_json::to_string_pretty(store) {
        if let Err(err) = fs::write(STORE_TMP_PATH, pretty) {
            warn!("Failed writing temporary store file: {}", err);
            return;
        }
        if let Err(err) = fs::rename(STORE_TMP_PATH, STORE_PATH) {
            warn!("Failed atomic rename {} -> {}: {}", STORE_TMP_PATH, STORE_PATH, err);
        }
    }
}
`,

    'src/commands.rs': `use crate::effects::{self, can_use_theme, find_theme};
use crate::host;
use crate::menu;
use crate::models::{
    CombatTag, EssentialsConfig, EssentialsStore, HomePoint, Location, PendingTeleport,
    PlayerRecord, TpaRequest, WarpPoint,
};
use std::collections::HashMap;

/// Lightweight deterministic xorshift64 PRNG for /rtp coordinates without extra crates.
pub fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    if x == 0 {
        x = 0x9e37_79b9_7f4a_7c15;
    }
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

pub fn sample_rtp_xz(seed: &mut u64, min_r: u32, max_r: u32) -> (f64, f64) {
    let span = (max_r.saturating_sub(min_r)).max(1) as u64;
    let r = (min_r as u64 + (next_u64(seed) % span)) as f64;
    let angle = ((next_u64(seed) % 36_000) as f64 / 36_000.0) * std::f64::consts::TAU;
    ((r * angle.cos()).floor() + 0.5, (r * angle.sin()).floor() + 0.5)
}

pub fn remaining_combat_seconds(combat_tags: &HashMap<String, CombatTag>, player_id: &str, now: u64) -> u64 {
    combat_tags
        .get(player_id)
        .filter(|tag| tag.expires_at_ms > now)
        .map(|tag| (tag.expires_at_ms - now + 999) / 1000)
        .unwrap_or(0)
}

pub fn schedule_teleport(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    combat_tags: &HashMap<String, CombatTag>,
    player_id: &str,
    target_label: &str,
    destination: Location,
    is_rtp_meteor: bool,
    dirty: &mut bool,
) {
    let now = host::now_ms();

    // 1. Anti-Combat check
    let combat_left = remaining_combat_seconds(combat_tags, player_id, now);
    if combat_left > 0 {
        host::send_chat(
            player_id,
            &format!("§c⚔ Вы в бою! Телепортация недоступна ещё §e{}с§c.", combat_left),
        );
        host::play_sound(player_id, "minecraft:entity.villager.no", 0.85, 0.8);
        return;
    }

    // 2. Safe Teleport check (skip for RTP which already validated surface_y)
    if !is_rtp_meteor && config.safe_teleport_check && !host::is_location_safe(&destination) {
        host::send_chat(
            player_id,
            &format!(
                "§c⚠ Точка §e{} §cнебезопасна (лава, блоки или пустота). Телепортация отменена!",
                target_label
            ),
        );
        host::play_sound(player_id, "minecraft:block.note_block.bass", 0.9, 0.6);
        return;
    }

    let player = store
        .players
        .entry(player_id.to_string())
        .or_insert_with(|| PlayerRecord::new(player_id, player_id));
    let rank = config.rank_for(&player.rank);

    // 3. Cooldown check
    let cooldown_ms = rank.cooldown_seconds * 1000;
    let since_last = now.saturating_sub(player.last_teleport_ms);
    if !is_rtp_meteor && cooldown_ms > 0 && player.last_teleport_ms > 0 && since_last < cooldown_ms {
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

    // Instant teleport for high ranks (non-RTP)
    if rank.warmup_seconds == 0 && !is_rtp_meteor {
        player.last_location = Some(origin.clone());
        player.last_teleport_ms = now;
        *dirty = true;
        effects::play_arrival_burst(player_id, target_label, &origin, &destination, &theme, config);
        return;
    }

    let warmup_secs = if is_rtp_meteor {
        rank.warmup_seconds.max(1)
    } else {
        rank.warmup_seconds
    };

    host::send_chat(
        player_id,
        &format!(
            "§6✦ §fЗапуск телепортации: §e{}§f ({}с). Не двигайтесь!",
            target_label, warmup_secs
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
            duration_ms: warmup_secs * 1000,
            ticks_elapsed: 0,
            last_second_announced: u64::MAX,
            is_rtp_meteor,
        },
    );
}

pub fn trigger_rtp(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    combat_tags: &HashMap<String, CombatTag>,
    player_id: &str,
    dirty: &mut bool,
) {
    let now = host::now_ms();
    let combat_left = remaining_combat_seconds(combat_tags, player_id, now);
    if combat_left > 0 {
        host::send_chat(player_id, &format!("§c⚔ Нельзя использовать /rtp в бою ({}с).", combat_left));
        return;
    }

    let player = store
        .players
        .entry(player_id.to_string())
        .or_insert_with(|| PlayerRecord::new(player_id, player_id));
    let rank = config.rank_for(&player.rank);
    let rtp_cd_ms = if rank.warmup_seconds == 0 {
        0
    } else {
        config.rtp.cooldown_seconds * 1000
    };
    let since_rtp = now.saturating_sub(player.last_rtp_ms);
    if rtp_cd_ms > 0 && player.last_rtp_ms > 0 && since_rtp < rtp_cd_ms {
        let wait = ((rtp_cd_ms - since_rtp) + 999) / 1000;
        host::send_chat(player_id, &format!("§c⏳ Кулдаун /rtp: подождите §e{}с§c.", wait));
        return;
    }

    let mut seed = now ^ player_id.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
    let mut found: Option<Location> = None;

    for _ in 0..config.rtp.max_attempts {
        let (x, z) = sample_rtp_xz(&mut seed, config.rtp.min_radius, config.rtp.max_radius);
        if let Some(surface_y) = host::find_safe_surface_y(&config.rtp.world, x, z) {
            found = Some(Location {
                world: config.rtp.world.clone(),
                x,
                y: surface_y,
                z,
                yaw: 0.0,
                pitch: 0.0,
            });
            break;
        }
    }

    let Some(dest) = found else {
        host::send_chat(player_id, "§c✖ Не удалось найти безопасную сушу. Попробуйте ещё раз!");
        return;
    };

    player.last_rtp_ms = now;
    *dirty = true;
    schedule_teleport(
        store,
        config,
        pending,
        combat_tags,
        player_id,
        "Случайная точка (RTP)",
        dest,
        true,
        dirty,
    );
}

pub fn handle_command(
    store: &mut EssentialsStore,
    config: &EssentialsConfig,
    pending: &mut HashMap<String, PendingTeleport>,
    tpa_requests: &mut HashMap<String, TpaRequest>,
    combat_tags: &HashMap<String, CombatTag>,
    dirty: &mut bool,
    player_id: &str,
    command: &str,
) -> bool {
    let parts: Vec<&str> = command.trim().split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    store
        .players
        .entry(player_id.to_string())
        .or_insert_with(|| PlayerRecord::new(player_id, player_id));

    let cmd = parts[0].to_lowercase();
    match cmd.as_str() {
        "/spawn" => {
            if let Some(spawn) = store.spawn.clone() {
                schedule_teleport(store, config, pending, combat_tags, player_id, "Спавн", spawn, false, dirty);
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
            *dirty = true;
            host::send_chat(player_id, &format!("§a✔ Спавн установлен: §f{}", loc.coords_label()));
            host::play_sound(player_id, "minecraft:ui.toast.challenge_complete", 0.8, 1.1);
            true
        }

        "/rtp" => {
            trigger_rtp(store, config, pending, combat_tags, player_id, dirty);
            true
        }

        "/sethome" => {
            let name = parts.get(1).copied().unwrap_or("home").to_lowercase();
            if name.len() > 20 || !name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                host::send_chat(player_id, "§c✖ Название: до 20 символов (буквы, цифры, _ и -).");
                return true;
            }

            let rank = config.rank_for(&store.players[player_id].rank);
            let loc = host::get_player_location(player_id);
            let requested_icon = parts.get(2).copied().map(|s| {
                if s.contains(':') { s.to_string() } else { format!("minecraft:{}", s) }
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
                        .map(|n| format!(" (ранг {} даёт {} меток!)", n.display_prefix.trim(), n.max_homes))
                        .unwrap_or_default();
                    host::send_chat(
                        player_id,
                        &format!("§c✖ Лимит домов исчерпан: §e{}/{}§c{}", player.homes.len(), rank.max_homes, next_info),
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

                let is_first = player.homes.is_empty();
                player.homes.push(HomePoint {
                    name: name.clone(),
                    icon,
                    location: loc.clone(),
                    created_at_ms: host::now_ms(),
                    uses: 0,
                    is_favorite: is_first,
                    shared_with: Vec::new(),
                });

                let limit_str = if rank.unlimited_homes { "∞".to_string() } else { rank.max_homes.to_string() };
                host::send_chat(
                    player_id,
                    &format!("§a✔ Дом §e{} §aсохранён! §8[§e{}/{}§8] §7• §f{}", name, player.homes.len(), limit_str, loc.coords_label()),
                );
            }

            host::spawn_particle(&loc.world, "minecraft:happy_villager", loc.x, loc.y + 1.0, loc.z, 14, 0.25);
            host::play_sound(player_id, "minecraft:entity.player.levelup", 0.7, 1.35);
            *dirty = true;
            true
        }

        "/home" => {
            if parts.len() == 1 {
                menu::open_homes_menu(store, config, tpa_requests, combat_tags, player_id, 0);
                return true;
            }

            // Subcommands: /home share <дом> <игрок>, /home unshare <дом> <игрок>, /home fav <дом>
            match parts[1].to_lowercase().as_str() {
                "share" => {
                    if parts.len() < 4 {
                        host::send_chat(player_id, "§eИспользование: §f/home share <дом> <игрок>");
                        return true;
                    }
                    let home_name = parts[2].to_lowercase();
                    let friend = parts[3].to_string();
                    let rank = config.rank_for(&store.players[player_id].rank);
                    let player = store.players.get_mut(player_id).unwrap();
                    if let Some(home) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        if home.shared_with.iter().any(|u| u.eq_ignore_ascii_case(&friend)) {
                            host::send_chat(player_id, &format!("§eИгрок {} уже имеет доступ к дому {}.", friend, home_name));
                            return true;
                        }
                        if home.shared_with.len() >= rank.max_shared_per_home {
                            host::send_chat(
                                player_id,
                                &format!("§c✖ Лимит друзей для этого дома: §e{}", rank.max_shared_per_home),
                            );
                            return true;
                        }
                        home.shared_with.push(friend.clone());
                        *dirty = true;
                        host::send_chat(
                            player_id,
                            &format!("§a✔ Игроку §e{} §aвыдан доступ к дому §e{}§a! Телепорт: §f/home {}:{}", friend, home_name, player_id, home_name),
                        );
                        host::send_chat(
                            &friend,
                            &format!("§6✦ §e{} §fподелился с вами домом §e{}§f! Введите: §a/home {}:{}", player_id, home_name, player_id, home_name),
                        );
                    } else {
                        host::send_chat(player_id, &format!("§c✖ Дом §e{} §cне найден.", home_name));
                    }
                    return true;
                }
                "unshare" => {
                    if parts.len() < 4 {
                        host::send_chat(player_id, "§eИспользование: §f/home unshare <дом> <игрок>");
                        return true;
                    }
                    let home_name = parts[2].to_lowercase();
                    let friend = parts[3];
                    let player = store.players.get_mut(player_id).unwrap();
                    if let Some(home) = player.homes.iter_mut().find(|h| h.name == home_name) {
                        let before = home.shared_with.len();
                        home.shared_with.retain(|u| !u.eq_ignore_ascii_case(friend));
                        if home.shared_with.len() < before {
                            *dirty = true;
                            host::send_chat(player_id, &format!("§a✔ Доступ §e{} §aк дому §e{} §aотозван.", friend, home_name));
                        } else {
                            host::send_chat(player_id, "§c✖ Этот игрок не был в списке доступа.");
                        }
                    } else {
                        host::send_chat(player_id, &format!("§c✖ Дом §e{} §cне найден.", home_name));
                    }
                    return true;
                }
                "fav" => {
                    let Some(home_name) = parts.get(2) else {
                        host::send_chat(player_id, "§eИспользование: §f/home fav <название>");
                        return true;
                    };
                    let key = home_name.to_lowercase();
                    let player = store.players.get_mut(player_id).unwrap();
                    if player.homes.iter().any(|h| h.name == key) {
                        for h in &mut player.homes {
                            h.is_favorite = h.name == key;
                        }
                        *dirty = true;
                        host::send_chat(player_id, &format!("§6★ Главный дом установлен: §e{}", key));
                    } else {
                        host::send_chat(player_id, &format!("§c✖ Дом §e{} §cне найден.", key));
                    }
                    return true;
                }
                _ => {}
            }

            // Shared home teleport: /home owner:home_name
            if let Some((owner_id, shared_home_name)) = parts[1].split_once(':') {
                let target_name = shared_home_name.to_lowercase();
                let found = store.players.get(owner_id).and_then(|owner| {
                    owner.homes.iter().find(|h| {
                        h.name == target_name
                            && (owner_id.eq_ignore_ascii_case(player_id)
                                || h.shared_with.iter().any(|u| u.eq_ignore_ascii_case(player_id)))
                    }).map(|h| h.location.clone())
                });
                match found {
                    Some(loc) => {
                        schedule_teleport(
                            store,
                            config,
                            pending,
                            combat_tags,
                            player_id,
                            &format!("Дом {}:{}", owner_id, target_name),
                            loc,
                            false,
                            dirty,
                        );
                    }
                    None => {
                        host::send_chat(player_id, "§c✖ Общий дом не найден или у вас нет доступа.");
                    }
                }
                return true;
            }

            // Own home teleport
            let target = parts[1].to_lowercase();
            let found = store.players.get_mut(player_id).and_then(|p| {
                p.homes.iter_mut().find(|h| h.name == target).map(|h| {
                    h.uses += 1;
                    (h.name.clone(), h.location.clone())
                })
            });

            match found {
                Some((name, loc)) => {
                    *dirty = true;
                    schedule_teleport(store, config, pending, combat_tags, player_id, &format!("Дом {}", name), loc, false, dirty);
                }
                None => {
                    host::send_chat(player_id, &format!("§c✖ Дом §e{} §cне найден. Откройте меню: §f/homes", target));
                }
            }
            true
        }

        "/homes" | "/menu" | "/tp" => {
            menu::open_homes_menu(store, config, tpa_requests, combat_tags, player_id, 0);
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
                *dirty = true;
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
                    host::send_chat(player_id, &format!("§c🔒 Варп доступен с ранга {}", config.rank_name_for_tier(warp.min_tier)));
                    return true;
                }
                schedule_teleport(store, config, pending, combat_tags, player_id, &format!("Варп {}", warp.name), warp.location, false, dirty);
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
            let desc = if parts.len() > 2 { parts[2..].join(" ") } else { "Общая точка телепортации".to_string() };
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
            *dirty = true;
            host::send_chat(player_id, &format!("§a✔ Варп §b{} §aсоздан: §f{}", key, loc.coords_label()));
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
                *dirty = true;
                host::send_chat(player_id, &format!("§a✔ Варп §b{} §aудалён.", name));
            } else {
                host::send_chat(player_id, "§c✖ Варп не найден.");
            }
            true
        }

        "/back" => {
            let rank = config.rank_for(&store.players[player_id].rank);
            let player = &store.players[player_id];
            if rank.can_back_on_death {
                if let Some(death_loc) = player.last_death_location.clone() {
                    schedule_teleport(store, config, pending, combat_tags, player_id, "Точка смерти", death_loc, false, dirty);
                    return true;
                }
            } else if player.last_death_location.is_some() && player.last_location.is_none() {
                host::send_chat(player_id, "§e✦ Возврат на точку смерти доступен со статуса §e[VIP]§e!");
                return true;
            }

            if let Some(loc) = player.last_location.clone() {
                schedule_teleport(store, config, pending, combat_tags, player_id, "Прошлая точка", loc, false, dirty);
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

            host::send_chat(player_id, &format!("§6✦ §fTPA-запрос отправлен игроку §e{}§f.", target_id));
            let action_txt = if summon_here {
                "приглашает вас к себе"
            } else {
                "хочет телепортироваться к вам"
            };
            host::send_chat(
                target_id,
                &format!(
                    "§6✦ §e{} §f{}. Введите §a/tpaccept§f, §c/tpdeny §fили откройте §e/homes §7({}с)",
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
                        schedule_teleport(store, config, pending, combat_tags, &req.to_id, &format!("Игрок {}", req.from_name), dest, false, dirty);
                    } else {
                        let dest = host::get_player_location(&req.to_id);
                        schedule_teleport(store, config, pending, combat_tags, &req.from_id, &format!("Игрок {}", req.to_id), dest, false, dirty);
                    }
                    host::send_chat(player_id, "§a✔ Запрос принят!");
                }
                _ => host::send_chat(player_id, "§c✖ У вас нет активных TPA-запросов."),
            }
            true
        }

        "/tpdeny" => {
            if let Some(req) = tpa_requests.remove(player_id) {
                host::send_chat(player_id, "§c✖ Запрос отклонён.");
                host::send_chat(&req.from_id, &format!("§c✖ Игрок §e{} §cотклонил ваш запрос.", player_id));
            } else {
                host::send_chat(player_id, "§c✖ Нет активных запросов.");
            }
            true
        }

        "/tpeffect" => {
            let Some(theme_arg) = parts.get(1) else {
                menu::open_effects_menu(store, config, player_id);
                return true;
            };
            let rank = config.rank_for(&store.players[player_id].rank);
            let spec = find_theme(theme_arg);
            if !spec.id.eq_ignore_ascii_case(theme_arg) {
                host::send_chat(player_id, "§c✖ Неизвестная тема. Откройте каталог: §f/tpeffect");
                return true;
            }
            if !can_use_theme(rank.tier, spec.id) {
                host::send_chat(
                    player_id,
                    &format!("§c🔒 Эффект {} §cдоступен со статуса {}", spec.label, config.rank_name_for_tier(spec.min_tier)),
                );
                return true;
            }
            if let Some(p) = store.players.get_mut(player_id) {
                p.selected_theme = Some(spec.id.to_string());
                *dirty = true;
            }
            host::send_chat(player_id, &format!("§a✔ Выбран эффект телепортации: {}", spec.label));
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
            let new_profile = config.rank_for(parts[2]);
            let target_player = store
                .players
                .entry(target_id.to_string())
                .or_insert_with(|| PlayerRecord::new(target_id, target_id));
            target_player.rank = new_profile.id.clone();
            *dirty = true;

            let limit_txt = if new_profile.unlimited_homes { "∞".to_string() } else { new_profile.max_homes.to_string() };
            host::send_chat(
                player_id,
                &format!("§a✔ Игроку §e{} §aвыдан статус {}§a(лимит домов: §e{}§a)", target_id, new_profile.display_prefix, limit_txt),
            );
            host::send_chat(
                target_id,
                &format!("§6✦ §fВаш статус обновлён до {}§f! Новый лимит домов: §e{}", new_profile.display_prefix, limit_txt),
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
use models::{
    ActiveMeteorFall, CombatTag, EssentialsConfig, EssentialsStore, PendingTeleport, PlayerRecord,
    TpaRequest,
};
use std::collections::HashMap;
use std::slice;
use std::str;
use std::sync::Mutex;

struct PluginState {
    config: EssentialsConfig,
    store: EssentialsStore,
    pending_teleports: HashMap<String, PendingTeleport>,
    active_meteors: HashMap<String, ActiveMeteorFall>,
    tpa_requests: HashMap<String, TpaRequest>,
    combat_tags: HashMap<String, CombatTag>,
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
        pending_teleports: HashMap::new(),
        active_meteors: HashMap::new(),
        tpa_requests: HashMap::new(),
        combat_tags: HashMap::new(),
        dirty: false,
        last_autosave_ms: now,
        tick_counter: 0,
    });

    info!("KRONOS Essentials v2.0.0 initialized (Homes, RTP, Anti-Combat, 5 GUIs, Atomic Save)");
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
    info!("KRONOS Essentials v2.0.0 flushed state and unloaded.");
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

        // Teleport first-time players to spawn if configured
        let is_first_join = !entry.first_join_completed;
        if is_first_join {
            entry.first_join_completed = true;
            state.dirty = true;
        }

        let rank = state.config.rank_for(&entry.rank);
        let limit = if rank.unlimited_homes {
            "∞".to_string()
        } else {
            rank.max_homes.to_string()
        };
        let home_count = entry.homes.len();
        let display_name = entry.username.clone();

        if is_first_join && state.config.teleport_new_players_to_spawn {
            if let Some(spawn) = state.store.spawn.clone() {
                host::teleport_player(player_id, &spawn);
                host::send_title(
                    player_id,
                    "§6✦ Добро пожаловать на KRONOS",
                    "§7Откройте меню телепортов: §e/homes",
                    10,
                    60,
                    20,
                );
            }
        }

        host::send_chat(
            player_id,
            &format!(
                "§6✦ §fПривет, {}§f{}! Метки дома: §e{}/{} §7(/homes • /rtp)",
                rank.display_prefix, display_name, home_count, limit
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
        state.active_meteors.remove(player_id);
        state.tpa_requests.remove(player_id);
        if state.dirty {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = host::now_ms();
        }
    }
}

/// Called every server tick (20 TPS):
/// - Animates teleport spirals & meteor trails
/// - Checks movement cancellation & completes teleports
/// - Displays PvP Combat ActionBar timers
/// - Cleans expired TPA/combat entries & runs debounced atomic autosave every 30s
#[no_mangle]
pub extern "C" fn on_tick() {
    let mut guard = STATE.lock().unwrap();
    let Some(ref mut state) = *guard else {
        return;
    };

    state.tick_counter = state.tick_counter.wrapping_add(1);
    let now = host::now_ms();

    // 1. Warmup teleports
    if !state.pending_teleports.is_empty() {
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

        for pid in completed {
            if let Some(pending) = state.pending_teleports.remove(&pid) {
                if let Some(player) = state.store.players.get_mut(&pid) {
                    player.last_location = Some(pending.origin.clone());
                    player.last_teleport_ms = now;
                    state.dirty = true;
                }
                if pending.is_rtp_meteor {
                    let meteor = effects::play_rtp_meteor_launch(
                        &pid,
                        &pending.origin,
                        &pending.destination,
                        &state.config,
                        now,
                    );
                    state.active_meteors.insert(pid, meteor);
                } else {
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
        }
    }

    // 2. Active RTP meteor drop particle trails
    if !state.active_meteors.is_empty() {
        state.active_meteors.retain(|_, fall| {
            if now >= fall.expires_at_ms {
                return false;
            }
            effects::render_meteor_trail_frame(fall);
            true
        });
    }

    // 3. Every 1 second (20 ticks): combat ActionBar countdown + expired TPA/combat cleanup
    if state.tick_counter % 20 == 0 {
        state.tpa_requests.retain(|_, req| req.expires_at_ms > now);
        state.combat_tags.retain(|pid, tag| {
            if tag.expires_at_ms <= now {
                host::send_action_bar(pid, "§a✔ Вы вышли из боя — телепорты снова доступны");
                return false;
            }
            let left = (tag.expires_at_ms - now + 999) / 1000;
            if !state.pending_teleports.contains_key(pid) {
                host::send_action_bar(
                    pid,
                    &format!("§c⚔ Режим боя ({}) §8• §e{}с §7(телепорт заблокирован)", tag.opponent, left),
                );
            }
            true
        });

        // Debounced atomic autosave
        let interval_ms = state.config.autosave_interval_seconds.max(5) * 1000;
        if state.dirty && now.saturating_sub(state.last_autosave_ms) >= interval_ms {
            storage::save_store_atomic(&state.store);
            state.dirty = false;
            state.last_autosave_ms = now;
        }
    }
}

/// Cancels warmup teleports on damage and tags both players in PvP combat if attacker_id is provided.
#[no_mangle]
pub extern "C" fn on_player_damage(
    player_id_ptr: *const u8,
    player_id_len: usize,
    attacker_id_ptr: *const u8,
    attacker_id_len: usize,
) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let attacker_id = read_str(attacker_id_ptr, attacker_id_len);

    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        if state.config.cancel_on_damage {
            if let Some(pending) = state.pending_teleports.remove(player_id) {
                effects::play_cancel_effect(player_id, &pending.origin, "Получен урон");
            }
        }

        if !attacker_id.is_empty() && attacker_id != player_id && state.config.combat_tag_seconds > 0 {
            let expires = host::now_ms() + state.config.combat_tag_seconds * 1000;
            state.combat_tags.insert(
                player_id.to_string(),
                CombatTag {
                    opponent: attacker_id.to_string(),
                    expires_at_ms: expires,
                },
            );
            state.combat_tags.insert(
                attacker_id.to_string(),
                CombatTag {
                    opponent: player_id.to_string(),
                    expires_at_ms: expires,
                },
            );
        }
    }
}

/// Saves death location for VIP+ /back and clears combat tag.
#[no_mangle]
pub extern "C" fn on_player_death(player_id_ptr: *const u8, player_id_len: usize) {
    let player_id = read_str(player_id_ptr, player_id_len);
    let mut guard = STATE.lock().unwrap();
    if let Some(ref mut state) = *guard {
        let death_loc = host::get_player_location(player_id);
        state.combat_tags.remove(player_id);
        state.pending_teleports.remove(player_id);

        let entry = state
            .store
            .players
            .entry(player_id.to_string())
            .or_insert_with(|| PlayerRecord::new(player_id, player_id));
        entry.last_death_location = Some(death_loc);
        state.dirty = true;

        let rank = state.config.rank_for(&entry.rank);
        if rank.can_back_on_death {
            host::send_chat(player_id, "§e☠ Точка смерти сохранена! Вернитесь командой §a/back");
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
            &state.combat_tags,
            &mut state.dirty,
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
        let mut ctx = menu::MenuDispatchContext {
            store: &mut state.store,
            config: &state.config,
            pending: &mut state.pending_teleports,
            tpa_requests: &mut state.tpa_requests,
            combat_tags: &state.combat_tags,
            dirty: &mut state.dirty,
        };
        return menu::handle_menu_click(
            &mut ctx,
            player_id,
            menu_id,
            slot,
            click_type,
            commands::schedule_teleport,
            commands::trigger_rtp,
        );
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank_home_limits_scale_properly() {
        let cfg = storage::default_config();
        assert_eq!(cfg.rank_for("player").max_homes, 3);
        assert_eq!(cfg.rank_for("vip").max_homes, 5);
        assert_eq!(cfg.rank_for("premium").max_homes, 7);
        assert_eq!(cfg.rank_for("legend").max_homes, 10);
        assert!(cfg.rank_for("owner").unlimited_homes);
    }

    #[test]
    fn effects_are_gated_by_rank_tier() {
        assert!(effects::can_use_theme(0, "amethyst_spiral"));
        assert!(!effects::can_use_theme(0, "golden_vortex"));
        assert!(effects::can_use_theme(1, "golden_vortex"));
        assert!(!effects::can_use_theme(2, "dragon_nova"));
        assert!(effects::can_use_theme(3, "dragon_nova"));
        assert!(effects::can_use_theme(4, "kronos_rift"));
    }

    #[test]
    fn rtp_coordinates_stay_within_configured_radius() {
        let mut seed = 123456789u64;
        for _ in 0..50 {
            let (x, z) = commands::sample_rtp_xz(&mut seed, 250, 4500);
            let dist = (x * x + z * z).sqrt();
            assert!(dist >= 248.0 && dist <= 4502.0);
        }
    }

    #[test]
    fn combat_tag_blocks_until_expiry() {
        let mut tags = HashMap::new();
        tags.insert(
            "steve".to_string(),
            CombatTag {
                opponent: "alex".to_string(),
                expires_at_ms: 10_000,
            },
        );
        assert_eq!(commands::remaining_combat_seconds(&tags, "steve", 7_500), 3);
        assert_eq!(commands::remaining_combat_seconds(&tags, "steve", 10_500), 0);
    }
}
`,

    'README.md': `# KRONOS Essentials v2.0.0 — Rust WASM Plugin (Pumpkin 26.3)

Полный комплект телепортации, безопасности, 5 интерактивных Chest GUI-меню, системы статусов с масштабируемыми лимитами домов и кинематографичных эффектов на Rust WASM (\`wasm32-wasip1\`).

## Что нового в v2.0.0
- **Anti-Combat (PvP-блокировка):** 15-секундный таймер боя с обратным отсчётом в ActionBar, запрещающий сбегать через \`/home\`, \`/spawn\`, \`/warp\`, \`/rtp\`, \`/tpa\`.
- **Safe Teleport + защита после переноса:** проверка безопасности точки (лава / блоки / пустота) + 2.5с \`resistance\` на прогрузку чанков.
- **\`/rtp\` с метеоритным десантом:** поиск безопасной суши + падение с неба с \`slow_falling\` и огненным шлейфом частиц.
- **Общие дома с друзьями:** \`/home share <дом> <игрок>\`, \`/home unshare <дом> <игрок>\` и телепорт \`/home <владелец>:<дом>\`.
- **Главный дом ★ и точка смерти \`/back\`:** авто-телепорт новичков на спавн и возврат на точку смерти для \`VIP+\`.
- **5 интерактивных GUI-меню:**
  1. Главное меню домов и быстрых телепортов (54 слота, пагинация, иконки, RTP, Spawn, Warps, Back, FX, TPA).
  2. Подменю настроек дома (\`Shift+ЛКМ\`): Главный дом ★, Переместить точку сюда, Сменить иконку, Удалить.
  3. Модальное окно подтверждения удаления (\`ПКМ\`): защита от случайного удаления базы.
  4. Каталог эффектов телепортации с привязкой к статусам и замками.
  5. Менеджер входящих \`/tpa\`-запросов (принять/отклонить в один клик).
- **Атомарное отложенное сохранение:** запись через \`store.json.tmp → rename\` раз в 30 секунд + при выгрузке, авто-очистка просроченных запросов и встроенные \`cargo test\` юнит-тесты.

## Статусы и лимиты (\`data/config.json\`)
| Статус | Дома | Друзей на дом | Отсчёт | Кулдаун | \`/back\` смерти | Эффект по умолчанию |
| :--- | :---: | :---: | :---: | :---: | :---: | :--- |
| \`player\` | **3** | 2 | 3с | 15с | ✖ | Аметистовая спираль |
| \`vip\` | **5** | 4 | 2с | 10с | ✔ | Золотой вихрь |
| \`premium\` | **7** | 6 | 2с | 6с | ✔ | Изумрудный импульс |
| \`legend\` | **10** | 10 | 1с | 3с | ✔ | Драконья сверхновая |
| \`admin\` / \`owner\` | **∞** | 50 | 0с | 0с | ✔ | Разлом KRONOS |

## Проверка тестами и сборка
\`\`\`bash
cargo test
rustup target add wasm32-wasip1
cargo build --target wasm32-wasip1 --release
\`\`\`
`,
  },
};
