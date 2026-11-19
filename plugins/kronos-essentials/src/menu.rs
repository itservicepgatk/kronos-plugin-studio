use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::forms::{Form, SimpleForm, SimpleFormButton},
    Player,
};

use crate::effects::{legacy, play_click_sound};
use crate::models::{
    normalize_key, BedrockMenuAction, HomePoint, PendingBedrockForm, SharedHomeAccess, TpaRequest,
    WarpPoint,
};
use crate::storage::with_state;

const LINE_SEP: &str = "§8§m────────────────────────────────────────────";

pub fn clickable_run(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_run_command(command)
        .hover_show_text(legacy(hover))
}

pub fn clickable_suggest(label: &str, command: &str, hover: &str) -> TextComponent {
    legacy(label)
        .click_suggest_command(command)
        .hover_show_text(legacy(hover))
}

pub struct HomesMenuSnapshot {
    pub player_name: String,
    pub homes: Vec<HomePoint>,
    pub shared_with_me: Vec<SharedHomeAccess>,
    pub max_homes: usize,
    pub unlimited: bool,
    pub combat_remaining: Option<u64>,
    pub incoming_tpa: Option<TpaRequest>,
    pub warps_count: usize,
    pub has_back: bool,
}

/// Opens the interactive KRONOS Essentials Hub:
/// - For Bedrock players: opens a native Bedrock `SimpleForm` (via `pumpkin_plugin_api::forms`)
///   plus sends a quick chat hint.
/// - For Java players (1.20.1–26.3): renders a rich interactive clickable chat menu using
///   `TextComponent` (`click_run_command`, `click_suggest_command`, `hover_show_text`).
pub fn open_homes_menu(player: &Player, snap: &HomesMenuSnapshot) {
    play_click_sound(player);

    if let Some(bedrock) = player.as_bedrock() {
        let (form, actions) = build_bedrock_hub_form(snap);
        let form_id = bedrock.open_form(form);
        let player_key = normalize_key(&snap.player_name);
        with_state(|st| {
            st.pending_forms.insert(
                form_id,
                PendingBedrockForm {
                    player_key,
                    actions,
                },
            );
        });
    }

    send_interactive_homes_chat_menu(player, snap);
}

fn build_bedrock_hub_form(snap: &HomesMenuSnapshot) -> (Form, Vec<BedrockMenuAction>) {
    let mut buttons = Vec::new();
    let mut actions = Vec::new();

    for home in &snap.homes {
        let star = if home.is_favorite { "★ " } else { "" };
        buttons.push(SimpleFormButton {
            text: legacy(&format!(
                "§6{}{}\n§7X:{:.0} Y:{:.0} Z:{:.0}",
                star, home.name, home.location.x, home.location.y, home.location.z
            )),
            image: None,
        });
        actions.push(BedrockMenuAction::TeleportHome(home.name.clone()));
    }

    for shared in &snap.shared_with_me {
        buttons.push(SimpleFormButton {
            text: legacy(&format!(
                "§d👥 {}:{}\n§7Общий дом друга",
                shared.owner, shared.home.name
            )),
            image: None,
        });
        actions.push(BedrockMenuAction::TeleportShared {
            owner: shared.owner.clone(),
            home: shared.home.name.clone(),
        });
    }

    buttons.push(SimpleFormButton {
        text: legacy("§a☄ Случайный телепорт (/rtp)"),
        image: None,
    });
    actions.push(BedrockMenuAction::RunRtp);

    buttons.push(SimpleFormButton {
        text: legacy("§e✦ Спавн сервера (/spawn)"),
        image: None,
    });
    actions.push(BedrockMenuAction::RunSpawn);

    if snap.has_back {
        buttons.push(SimpleFormButton {
            text: legacy("§b↩ Вернуться назад (/back)"),
            image: None,
        });
        actions.push(BedrockMenuAction::RunBack);
    }

    buttons.push(SimpleFormButton {
        text: legacy(&format!("§6⚑ Варпы сервера ({})", snap.warps_count)),
        image: None,
    });
    actions.push(BedrockMenuAction::OpenWarps);

    let limit_text = if snap.unlimited {
        "∞".to_string()
    } else {
        snap.max_homes.to_string()
    };

    let content = match snap.combat_remaining {
        Some(sec) => format!(
            "§7Дома: §e{}/{}\n§c⚔ Вы в бою! Телепортация заблокирована на {}с",
            snap.homes.len(),
            limit_text,
            sec
        ),
        None => format!(
            "§7Дома: §e{}/{} §8| §7Общие: §d{}\n§7Выберите точку для телепортации:",
            snap.homes.len(),
            limit_text,
            snap.shared_with_me.len()
        ),
    };

    let form = Form::Simple(SimpleForm {
        title: legacy("§6§l✦ KRONOS Essentials"),
        content: legacy(&content),
        buttons,
    });

    (form, actions)
}

pub fn send_interactive_homes_chat_menu(player: &Player, snap: &HomesMenuSnapshot) {
    player.send_system_message(legacy(LINE_SEP), false);

    let limit_text = if snap.unlimited {
        "∞".to_string()
    } else {
        snap.max_homes.to_string()
    };

    // Header line
    player.send_system_message(
        legacy(&format!(
            "§6§l✦ KRONOS HUB §8— §e{} §7(Точек дома: §e{}/{}§7, Общих: §d{}§7)",
            snap.player_name,
            snap.homes.len(),
            limit_text,
            snap.shared_with_me.len()
        )),
        false,
    );

    // Combat warning if tagged
    if let Some(sec) = snap.combat_remaining {
        player.send_system_message(
            legacy(&format!(
                " §c⚔ §lВНИМАНИЕ: §cВы находитесь в бою! Телепортация заблокирована на §e{}§cс.",
                sec
            )),
            false,
        );
    }

    // Quick Action Bar
    let mut bar = legacy(" ")
        .add_child(clickable_run(
            "§a§l[☄ RTP]",
            "/rtp",
            "§aСлучайная телепортация\n§7Автопоиск безопасной поверхности без лавы и удушья",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§e§l[✦ Спавн]",
            "/spawn",
            "§eТелепортация на спавн сервера",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            &format!("§b§l[⚑ Варпы ({})]", snap.warps_count),
            "/warps",
            "§bОткрыть список серверных варпов",
        ))
        .add_child(legacy(" "));
    if snap.has_back {
        bar = bar
            .add_child(clickable_run(
                "§d§l[↩ Назад]",
                "/back",
                "§dВернуться на предыдущую позицию",
            ))
            .add_child(legacy(" "));
    }
    bar = bar.add_child(clickable_suggest(
        "§6§l[+ Дом]",
        "/sethome ",
        "§6Создать новую точку дома\n§7Нажмите, чтобы ввести имя дома",
    ));
    player.send_system_message(bar, false);

    // Owned Homes
    if snap.homes.is_empty() {
        let empty = legacy(" §7У вас пока нет домов. ").add_child(clickable_run(
            "§a[Создать дом 'home' здесь]",
            "/sethome home",
            "§aСоздать точку дома 'home' на текущих координатах",
        ));
        player.send_system_message(empty, false);
    } else {
        player.send_system_message(legacy(" §eВаши точки дома:"), false);
        for home in &snap.homes {
            let star_label = if home.is_favorite { "§6★" } else { "§8☆" };
            let star_hover = if home.is_favorite {
                "§6Главный дом (по умолчанию для /home)\n§7Нажмите, чтобы снять отметку"
            } else {
                "§7Сделать главным домом ★"
            };

            let shared_info = if home.shared_with.is_empty() {
                "§7Доступ: §8только вы".to_string()
            } else {
                format!("§7Открыт друзьям: §d{}", home.shared_with.join(", "))
            };
            let hover_details = format!(
                "§6§l{} {}\n§7Мир: §f{}\n§7Координаты: §a{:.1}, {:.1}, {:.1}\n§7Телепортаций: §e{}\n{}\n\n§a▶ Нажмите для телепортации",
                home.icon,
                home.name,
                home.location.world,
                home.location.x,
                home.location.y,
                home.location.z,
                home.uses,
                shared_info
            );

            let row = legacy("  ")
                .add_child(clickable_run(
                    star_label,
                    &format!("/home fav {}", home.name),
                    star_hover,
                ))
                .add_child(legacy(" "))
                .add_child(clickable_run(
                    &format!("§f{} §e{}", home.icon, home.name),
                    &format!("/home {}", home.name),
                    &hover_details,
                ))
                .add_child(legacy(&format!(
                    " §8({:.0}, {:.0}, {:.0}) ",
                    home.location.x, home.location.y, home.location.z
                )))
                .add_child(clickable_run(
                    "§a[ТП]",
                    &format!("/home {}", home.name),
                    &format!("§aТелепортироваться в дом §e{}", home.name),
                ))
                .add_child(legacy(" "))
                .add_child(clickable_suggest(
                    "§b[Друзья]",
                    &format!("/home share {} ", home.name),
                    &format!(
                        "§bПоделиться домом §e{} §bс другом\n§7Команда: §f/home share {} <ник>",
                        home.name, home.name
                    ),
                ))
                .add_child(legacy(" "))
                .add_child(clickable_run(
                    "§c[✕]",
                    &format!("/delhome {}", home.name),
                    &format!("§cУдалить дом §e{} §cв один клик", home.name),
                ));

            player.send_system_message(row, false);
        }
    }

    // Shared Homes Available To This Player
    if !snap.shared_with_me.is_empty() {
        let header = legacy(" §dОбщие дома друзей: ").add_child(clickable_run(
            "§7[Управление общими]",
            "/sharehome",
            "§dОткрыть подробный список всех общих домов",
        ));
        player.send_system_message(header, false);

        for entry in snap.shared_with_me.iter().take(6) {
            let cmd = format!("/home {}:{}", entry.owner, entry.home.name);
            let hover = format!(
                "§dОбщий дом §f{}:{}\n§7Мир: §f{}\n§7Координаты: §a{:.1}, {:.1}, {:.1}\n\n§a▶ Нажмите для телепортации",
                entry.owner,
                entry.home.name,
                entry.home.location.world,
                entry.home.location.x,
                entry.home.location.y,
                entry.home.location.z
            );
            let row = legacy("  §d👥 ")
                .add_child(clickable_run(
                    &format!("§f{}§7:§d{}", entry.owner, entry.home.name),
                    &cmd,
                    &hover,
                ))
                .add_child(legacy(" "))
                .add_child(clickable_run("§a[ТП]", &cmd, &hover));
            player.send_system_message(row, false);
        }
    }

    // Incoming TPA Banner
    if let Some(tpa) = &snap.incoming_tpa {
        send_tpa_inline_row(player, &tpa.requester, tpa.bring_target_here);
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn send_warps_menu(player: &Player, warps: &[WarpPoint]) {
    play_click_sound(player);
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l⚑ ВАРПЫ СЕРВЕРА §7(Всего: §e{}§7)",
            warps.len()
        )),
        false,
    );

    if warps.is_empty() {
        player.send_system_message(
            legacy(" §7На сервере пока нет созданных варпов."),
            false,
        );
    } else {
        for warp in warps {
            let desc_line = if warp.description.is_empty() {
                String::new()
            } else {
                format!("\n§7Описание: §f{}", warp.description)
            };
            let hover = format!(
                "§b§lВарп: {}§r{}\n§7Мир: §f{}\n§7Координаты: §a{:.1}, {:.1}, {:.1}\n§7Посещений: §e{}\n\n§a▶ Нажмите для телепортации",
                warp.name,
                desc_line,
                warp.location.world,
                warp.location.x,
                warp.location.y,
                warp.location.z,
                warp.uses
            );
            let cmd = format!("/warp {}", warp.name);
            let mut row = legacy("  §6⚑ ")
                .add_child(clickable_run(&format!("§b{}", warp.name), &cmd, &hover));
            if !warp.description.is_empty() {
                row = row.add_child(legacy(&format!(" §8— §7{}", warp.description)));
            }
            row = row
                .add_child(legacy(" "))
                .add_child(clickable_run("§a[ТП]", &cmd, &hover));
            player.send_system_message(row, false);
        }
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn send_shared_homes_menu(
    player: &Player,
    own_homes: &[HomePoint],
    incoming_shared: &[SharedHomeAccess],
) {
    play_click_sound(player);
    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy("§d§l👥 ОБЩИЕ ДОМА §8— §7Доступ друзей"),
        false,
    );

    // 1) Homes friends shared WITH this player
    player.send_system_message(legacy(" §bДоступные вам дома друзей:"), false);
    if incoming_shared.is_empty() {
        player.send_system_message(
            legacy("  §7Никто ещё не поделился с вами точкой дома."),
            false,
        );
    } else {
        for item in incoming_shared {
            let cmd = format!("/home {}:{}", item.owner, item.home.name);
            let hover = format!(
                "§dВладелец: §f{}\n§7Дом: §e{}\n§7Координаты: §a{:.1}, {:.1}, {:.1}\n\n§a▶ Нажмите для телепортации",
                item.owner,
                item.home.name,
                item.home.location.x,
                item.home.location.y,
                item.home.location.z
            );
            let row = legacy("  §d• ")
                .add_child(clickable_run(
                    &format!("§f{}§7:§e{}", item.owner, item.home.name),
                    &cmd,
                    &hover,
                ))
                .add_child(legacy(&format!(
                    " §8({:.0}, {:.0}, {:.0}) ",
                    item.home.location.x, item.home.location.y, item.home.location.z
                )))
                .add_child(clickable_run("§a[ТП]", &cmd, &hover));
            player.send_system_message(row, false);
        }
    }

    // 2) Homes THIS player shared with friends
    player.send_system_message(legacy(" §eВаши открытые друзьям дома:"), false);
    let mut any_outbound = false;
    for home in own_homes {
        if home.shared_with.is_empty() {
            continue;
        }
        any_outbound = true;
        let mut row = legacy(&format!("  §6⌂ §e{} §8→ ", home.name));
        for (idx, friend) in home.shared_with.iter().enumerate() {
            if idx > 0 {
                row = row.add_child(legacy("§7, "));
            }
            row = row
                .add_child(legacy(&format!("§f{}", friend)))
                .add_child(clickable_run(
                    "§c[✕]",
                    &format!("/home unshare {} {}", home.name, friend),
                    &format!(
                        "§cЗабрать доступ к дому §e{} §cу игрока §f{}",
                        home.name, friend
                    ),
                ));
        }
        player.send_system_message(row, false);
    }

    if !any_outbound {
        let hint = legacy("  §7Вы пока не делились домами. ").add_child(clickable_suggest(
            "§b[Поделиться домом]",
            "/sharehome ",
            "§bИспользование: §f/sharehome <дом> <ник_друга>",
        ));
        player.send_system_message(hint, false);
    }

    player.send_system_message(legacy(LINE_SEP), false);
}

pub fn send_tpa_prompt(target: &Player, requester: &str, bring_here: bool) {
    target.send_system_message(legacy(LINE_SEP), false);
    send_tpa_inline_row(target, requester, bring_here);
    target.send_system_message(legacy(LINE_SEP), false);
}

fn send_tpa_inline_row(target: &Player, requester: &str, bring_here: bool) {
    let text = if bring_here {
        format!(
            " §6✦ §e{} §fприглашает вас телепортироваться к нему! ",
            requester
        )
    } else {
        format!(
            " §6✦ §e{} §fхочет телепортироваться к вам! ",
            requester
        )
    };
    let row = legacy(&text)
        .add_child(clickable_run(
            "§a§l[✔ Принять]",
            "/tpaccept",
            "§aПринять запрос телепортации",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_run(
            "§c§l[✖ Отклонить]",
            "/tpdeny",
            "§cОтклонить запрос телепортации",
        ));
    target.send_system_message(row, false);
}
