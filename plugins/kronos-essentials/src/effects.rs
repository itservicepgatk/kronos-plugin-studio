use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::{
        common::BlockPos,
        particles::Particle,
        sounds::SoundCategory,
        status_effect::{StatusEffectInstance, StatusEffectType},
        world::{BlockState, World},
    },
    Player, Server,
};

use crate::models::{EssentialsConfig, LocationPoint, RtpConfig};

pub const SOUND_TELEPORT: &str = "minecraft:entity.enderman.teleport";
pub const SOUND_SUCCESS: &str = "minecraft:block.note_block.pling";
pub const SOUND_CHIME: &str = "minecraft:block.note_block.chime";
pub const SOUND_CLICK: &str = "minecraft:ui.button.click";
pub const SOUND_ERROR: &str = "minecraft:entity.villager.no";
pub const SOUND_RTP_LAUNCH: &str = "minecraft:entity.firework_rocket.launch";

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

pub fn play_success_sound(player: &Player) {
    player.play_custom_sound(SOUND_SUCCESS, SoundCategory::Players, 0.9, 1.5);
}

pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(SOUND_ERROR, SoundCategory::Players, 0.9, 0.9);
}

pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(SOUND_CLICK, SoundCategory::Players, 0.7, 1.2);
}

pub fn play_chime_sound(player: &Player) {
    player.play_custom_sound(SOUND_CHIME, SoundCategory::Players, 1.0, 1.3);
}

pub fn capture_player_location(player: &Player) -> LocationPoint {
    let pos = player.get_position();
    let world_name = player.get_world().get_name();
    LocationPoint::rounded(
        world_name,
        pos.0,
        pos.1,
        pos.2,
        player.get_yaw(),
        player.get_pitch(),
    )
}

pub fn resolve_world(server: &Server, world_name: &str) -> Option<World> {
    if let Some(world) = server.get_world_by_name(world_name) {
        return Some(world);
    }
    if world_name.eq_ignore_ascii_case("world")
        || world_name.eq_ignore_ascii_case("overworld")
        || world_name.eq_ignore_ascii_case("minecraft:overworld")
    {
        if let Some(world) = server.get_world_by_name("minecraft:overworld") {
            return Some(world);
        }
        if let Some(world) = server.get_world_by_name("world") {
            return Some(world);
        }
    }
    server.get_all_worlds().into_iter().next()
}

pub fn play_teleport_effects(
    player: &Player,
    loc: &LocationPoint,
    title: &str,
    subtitle: &str,
    cfg: &EssentialsConfig,
    is_rtp: bool,
) {
    if cfg.blindness_flash_ticks > 0 {
        player.add_effect(StatusEffectInstance {
            effect_type: StatusEffectType::Blindness,
            duration: cfg.blindness_flash_ticks,
            amplifier: 0,
            ambient: false,
            show_particles: false,
            show_icon: false,
        });
    }

    if cfg.post_teleport_invulnerability_ticks > 0 {
        player.add_effect(StatusEffectInstance {
            effect_type: StatusEffectType::Resistance,
            duration: cfg.post_teleport_invulnerability_ticks,
            amplifier: 4,
            ambient: false,
            show_particles: false,
            show_icon: false,
        });
    }

    let slow_fall = if is_rtp {
        cfg.rtp.slow_falling_ticks.max(cfg.slow_falling_ticks)
    } else {
        cfg.slow_falling_ticks
    };
    if slow_fall > 0 {
        player.add_effect(StatusEffectInstance {
            effect_type: StatusEffectType::SlowFalling,
            duration: slow_fall,
            amplifier: 0,
            ambient: false,
            show_particles: false,
            show_icon: false,
        });
    }

    player.play_custom_sound(SOUND_TELEPORT, SoundCategory::Players, 1.0, 1.0);
    if is_rtp {
        player.play_custom_sound(SOUND_RTP_LAUNCH, SoundCategory::Players, 0.85, 1.15);
    }

    let particle_pos = (loc.x, loc.y + 1.0, loc.z);
    player.spawn_particles(Particle::Portal, particle_pos, 28, (0.5, 0.9, 0.5), 0.15);
    player.spawn_particles(Particle::EndRod, particle_pos, 14, (0.4, 0.7, 0.4), 0.08);

    player.send_title_animation(5, 28, 10);
    player.show_title(legacy(title));
    player.show_subtitle(legacy(subtitle));
    player.show_actionbar(legacy(&format!(
        "§6✦ §f{} §8| §7X:§a{:.0} §7Y:§a{:.0} §7Z:§a{:.0}",
        strip_legacy_codes(subtitle),
        loc.x,
        loc.y,
        loc.z
    )));
}

pub fn strip_legacy_codes(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '§' {
            let _ = chars.next();
        } else {
            out.push(c);
        }
    }
    out
}

/// Teleports a player safely using real `pumpkin_plugin_api` World & Player resources.
pub fn execute_teleport(
    player: &Player,
    server: &Server,
    loc: &LocationPoint,
    title: &str,
    subtitle: &str,
    cfg: &EssentialsConfig,
    is_rtp: bool,
    verify_safe: bool,
) -> Result<(), String> {
    let Some(world) = resolve_world(server, &loc.world) else {
        return Err("Целевой мир недоступен на сервере.".to_string());
    };

    if verify_safe && cfg.safe_teleport_check && !is_home_location_safe(&world, loc) {
        return Err(
            "Целевая точка небезопасна (лава, пустота или заблокирована блоками).".to_string(),
        );
    }

    player.teleport((loc.x, loc.y, loc.z), Some(loc.yaw), Some(loc.pitch), world);
    play_teleport_effects(player, loc, title, subtitle, cfg, is_rtp);
    Ok(())
}

// =============================================================================
// Safe Surface & Block Hazard Analysis (Pure Helpers + Real World Queries)
// =============================================================================

pub fn normalize_block_name(block_name: &str) -> &str {
    let trimmed = block_name.trim();
    trimmed.strip_prefix("minecraft:").unwrap_or(trimmed)
}

pub fn is_block_hazardous(block_name: &str) -> bool {
    let id = normalize_block_name(block_name).to_ascii_lowercase();
    matches!(
        id.as_str(),
        "lava"
            | "flowing_lava"
            | "water"
            | "flowing_water"
            | "bubble_column"
            | "fire"
            | "soul_fire"
            | "magma_block"
            | "cactus"
            | "sweet_berry_bush"
            | "wither_rose"
            | "campfire"
            | "soul_campfire"
            | "powder_snow"
            | "pointed_dripstone"
            | "cobweb"
            | "bedrock"
            | "void_air"
            | "cave_air"
            | "air"
            | "kelp"
            | "kelp_plant"
            | "seagrass"
            | "tall_seagrass"
            | "lily_pad"
            | "bamboo"
            | "scaffolding"
    )
}

pub fn is_leaves_block(block_name: &str) -> bool {
    normalize_block_name(block_name)
        .to_ascii_lowercase()
        .ends_with("_leaves")
}

pub fn is_standable_ground_props(
    is_solid: bool,
    is_liquid: bool,
    is_air: bool,
    up_side_solid: bool,
    block_name: &str,
    strict_rtp: bool,
) -> bool {
    if !is_solid || is_liquid || is_air || is_block_hazardous(block_name) {
        return false;
    }
    if strict_rtp && (!up_side_solid || is_leaves_block(block_name)) {
        return false;
    }
    true
}

pub fn is_passable_space_props(
    is_air: bool,
    is_liquid: bool,
    is_solid: bool,
    replaceable: bool,
    block_name: &str,
) -> bool {
    if is_liquid {
        return false;
    }
    let id = normalize_block_name(block_name).to_ascii_lowercase();
    if matches!(
        id.as_str(),
        "lava"
            | "flowing_lava"
            | "water"
            | "flowing_water"
            | "fire"
            | "soul_fire"
            | "cactus"
            | "sweet_berry_bush"
            | "wither_rose"
            | "campfire"
            | "soul_campfire"
            | "powder_snow"
            | "cobweb"
            | "magma_block"
            | "pointed_dripstone"
    ) {
        return false;
    }
    is_air || (replaceable && !is_solid)
}

fn is_state_standable(state: &BlockState, strict_rtp: bool) -> bool {
    is_standable_ground_props(
        state.is_solid,
        state.is_liquid,
        state.is_air,
        state.up_side_solid,
        &state.block_name,
        strict_rtp,
    )
}

fn is_state_passable(state: &BlockState) -> bool {
    is_passable_space_props(
        state.is_air,
        state.is_liquid,
        state.is_solid,
        state.replaceable,
        &state.block_name,
    )
}

fn is_adjacent_danger_free(world: &World, x: i32, y: i32, z: i32) -> bool {
    for (dx, dz) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
        let state = world.get_block_state(BlockPos {
            x: x + dx,
            y,
            z: z + dz,
        });
        let name = normalize_block_name(&state.block_name).to_ascii_lowercase();
        if matches!(
            name.as_str(),
            "lava" | "flowing_lava" | "fire" | "soul_fire" | "cactus"
        ) {
            return false;
        }
    }
    true
}

pub fn is_home_location_safe(world: &World, loc: &LocationPoint) -> bool {
    if !world.get_border().contains(loc.x, loc.z) {
        return false;
    }
    let min_y = world.get_min_y();
    let bx = loc.x.floor() as i32;
    let by = loc.y.floor() as i32;
    let bz = loc.z.floor() as i32;
    if by <= min_y + 1 || by >= 319 {
        return false;
    }

    let ground = world.get_block_state(BlockPos {
        x: bx,
        y: by - 1,
        z: bz,
    });
    let feet = world.get_block_state(BlockPos { x: bx, y: by, z: bz });
    let head = world.get_block_state(BlockPos {
        x: bx,
        y: by + 1,
        z: bz,
    });

    is_state_standable(&ground, false) && is_state_passable(&feet) && is_state_passable(&head)
}

/// Scans downward from the real world surface height to find a solid, non-liquid,
/// non-hazardous ground block with 2 passable headroom blocks above it.
pub fn find_safe_surface_y(world: &World, x: i32, z: i32) -> Option<i32> {
    if !world.get_border().contains(x as f64 + 0.5, z as f64 + 0.5) {
        return None;
    }

    let min_y = world.get_min_y();
    let motion_y = world.get_motion_blocking_height(x, z);
    let top_y = world.get_top_block_y(x, z);
    let highest = motion_y.max(top_y).min(316);

    // Unloaded chunk or pure void column
    if highest <= min_y + 2 {
        return None;
    }

    let lowest_scan = (highest - 48).max(min_y + 2);
    for ground_y in (lowest_scan..=highest).rev() {
        let ground = world.get_block_state(BlockPos {
            x,
            y: ground_y,
            z,
        });
        if ground.is_liquid {
            // Do not sink into an ocean or lava lake looking for the ocean floor
            return None;
        }
        if !is_state_standable(&ground, true) {
            continue;
        }
        let feet = world.get_block_state(BlockPos {
            x,
            y: ground_y + 1,
            z,
        });
        let head = world.get_block_state(BlockPos {
            x,
            y: ground_y + 2,
            z,
        });

        if is_state_passable(&feet)
            && is_state_passable(&head)
            && is_adjacent_danger_free(world, x, ground_y + 1, z)
        {
            return Some(ground_y + 1);
        }
    }

    None
}

pub fn sample_ring_coords(
    center_x: f64,
    center_z: f64,
    min_radius: i32,
    max_radius: i32,
    rand_a: u64,
    rand_b: u64,
) -> (i32, i32) {
    let min_r = min_radius.max(32) as f64;
    let max_r = max_radius.max(min_radius + 64) as f64;
    let span = (max_r - min_r).max(1.0);

    let frac_r = ((rand_a % 10_000) as f64) / 10_000.0;
    let radius = min_r + frac_r * span;
    let angle = ((rand_b % 36_000) as f64 / 100.0).to_radians();

    let bx = (center_x + radius * angle.cos()).round() as i32;
    let bz = (center_z + radius * angle.sin()).round() as i32;
    (bx, bz)
}

pub fn find_safe_rtp_location(
    world: &World,
    center_x: f64,
    center_z: f64,
    rtp: &RtpConfig,
    seed: &mut u64,
) -> Option<LocationPoint> {
    let attempts = rtp.max_attempts.clamp(8, 40);
    let world_name = world.get_name();

    for _ in 0..attempts {
        let a = advance_seed(seed);
        let b = advance_seed(seed);
        let (bx, bz) = sample_ring_coords(center_x, center_z, rtp.min_radius, rtp.max_radius, a, b);

        if let Some(safe_y) = find_safe_surface_y(world, bx, bz) {
            return Some(LocationPoint {
                world: world_name,
                x: bx as f64 + 0.5,
                y: safe_y as f64,
                z: bz as f64 + 0.5,
                yaw: 0.0,
                pitch: 0.0,
            });
        }
    }

    None
}

fn advance_seed(seed: &mut u64) -> u64 {
    let mut s = *seed;
    if s == 0 {
        s = 0x9E37_79B9_7F4A_7C15;
    }
    s ^= s << 13;
    s ^= s >> 7;
    s ^= s << 17;
    *seed = s;
    s
}
