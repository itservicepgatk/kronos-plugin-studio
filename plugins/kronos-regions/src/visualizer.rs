use crate::host;
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
/// - Switches to Red Alert particles if `viz.conflict` is true
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
