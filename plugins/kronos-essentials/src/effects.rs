use crate::host;
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
