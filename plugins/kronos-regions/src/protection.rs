use crate::host;
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
