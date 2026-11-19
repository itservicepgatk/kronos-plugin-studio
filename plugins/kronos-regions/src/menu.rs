use crate::host;
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
    let rank = config.effective_rank_for(store, player_id);
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
    let next_rank_hint = match config.next_donate_after(&rank) {
        Some(next) if !rank.unlimited_regions => format!(
            "§7Следующий донат {}: §e{} приватов §7(до §e{} §7бл.)",
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
            let unlock = config.donate_unlock_for_slot(idx);
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
    let rank = config.effective_rank_for(store, player_id);
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
    let rank = config.effective_rank_for(store, player_id);
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
    let rank = config.effective_rank_for(store, player_id);
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
        let rank = ctx.config.effective_rank_for(ctx.store, player_id);
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
            let rank = ctx.config.effective_rank_for(ctx.store, player_id);
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
                    let rank = ctx.config.effective_rank_for(ctx.store, player_id);
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
