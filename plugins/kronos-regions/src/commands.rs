use crate::host;
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
