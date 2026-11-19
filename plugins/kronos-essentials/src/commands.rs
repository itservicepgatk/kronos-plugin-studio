use crate::effects::{self, can_use_theme, find_theme};
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
