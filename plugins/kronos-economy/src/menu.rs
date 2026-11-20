use pumpkin_plugin_api::{
    text::TextComponent,
    wit::pumpkin::plugin::{
        forms::{Form, SimpleForm, SimpleFormButton},
        sounds::SoundCategory,
    },
    Player,
};

use crate::models::{
    format_amount, format_amount_u128, normalize_key, Account, BedrockEcoAction,
    PendingBedrockEcoForm, TopEntry,
};
use crate::storage::with_state;

const LINE_SEP: &str = "§8§m────────────────────────────────────────────";

pub fn legacy(s: &str) -> TextComponent {
    TextComponent::from_legacy_string(s)
}

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

/// Звук успешного перевода / начисления монет (стандартный ID 1.20.1).
pub fn play_coin_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:entity.experience_orb.pickup",
        SoundCategory::Players,
        1.0,
        1.0,
    );
}

/// Звук ошибки / отказа операции (стандартный ID 1.20.1).
pub fn play_error_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:entity.villager.no",
        SoundCategory::Players,
        1.0,
        1.0,
    );
}

/// Звук навигации по меню (стандартный ID 1.20.1).
pub fn play_click_sound(player: &Player) {
    player.play_custom_sound(
        "minecraft:ui.button.click",
        SoundCategory::Players,
        0.8,
        1.1,
    );
}

fn open_bedrock_form(
    player: &Player,
    player_name: &str,
    title: &str,
    content: &str,
    buttons: Vec<SimpleFormButton>,
    actions: Vec<BedrockEcoAction>,
) {
    if let Some(bedrock) = player.as_bedrock() {
        let form = Form::Simple(SimpleForm {
            title: legacy(title),
            content: legacy(content),
            buttons,
        });
        let form_id = bedrock.open_form(form);
        let player_key = normalize_key(player_name);
        with_state(|st| {
            st.pending_forms.insert(
                form_id,
                PendingBedrockEcoForm {
                    player_key,
                    actions,
                },
            );
        });
    }
}

/// Открывает карточку баланса (`/money`, `/bal`) в двойном UI:
/// - Bedrock: нативная форма `SimpleForm`
/// - Java: интерактивный блок с кликабельными кнопками и hover-подсказками
pub fn open_balance_menu(
    player: &Player,
    account: &Account,
    currency_symbol: &str,
    rank_pos: Option<usize>,
    total_accounts: usize,
    is_self: bool,
    is_admin: bool,
) {
    play_click_sound(player);
    let viewer_name = player.get_name();
    let bal_fmt = format_amount(account.balance);
    let rank_label = rank_pos
        .map(|r| format!("#{r} из {total_accounts}"))
        .unwrap_or_else(|| "—".to_string());

    if player.as_bedrock().is_some() {
        let mut buttons = vec![
            SimpleFormButton {
                text: legacy("§6★ Топ-10 богачей (/baltop)"),
                image: None,
            },
            SimpleFormButton {
                text: legacy("§e⟳ Обновить свой баланс"),
                image: None,
            },
        ];
        let mut actions = vec![
            BedrockEcoAction::OpenBaltop,
            BedrockEcoAction::OpenOwnBalance,
        ];

        if !is_self {
            buttons.push(SimpleFormButton {
                text: legacy(&format!("§a⇄ Перевести игроку {}", account.player_name)),
                image: None,
            });
            actions.push(BedrockEcoAction::SuggestPay(account.player_name.clone()));
        }

        let content = format!(
            "§7Игрок: §f{}\n§7Баланс: §e§l{} {}\n§7Место в топе: §b{}\n§7Получено всего: §a+{} {}\n§7Отправлено всего: §c-{} {}",
            account.player_name,
            bal_fmt,
            currency_symbol,
            rank_label,
            format_amount(account.total_received),
            currency_symbol,
            format_amount(account.total_sent),
            currency_symbol
        );
        open_bedrock_form(
            player,
            &viewer_name,
            "§6§l✦ KRONOS ECONOMY",
            &content,
            buttons,
            actions,
        );
    }

    player.send_system_message(legacy(LINE_SEP), false);
    let header = if is_self {
        "§6§l✦ ВАШ БАЛАНС §8| §eKRONOS ECONOMY".to_string()
    } else {
        format!("§6§l✦ БАЛАНС ИГРОКА §f{}", account.player_name)
    };
    player.send_system_message(legacy(&header), false);

    let balance_row = legacy(" §7Баланс: ")
        .add_child(
            legacy(&format!("§e§l{} {}", bal_fmt, currency_symbol)).hover_show_text(legacy(&format!(
                "§7Получено переводами: §a+{} {}\n§7Отправлено переводами: §c-{} {}\n§7Место в рейтинге: §b{}",
                format_amount(account.total_received),
                currency_symbol,
                format_amount(account.total_sent),
                currency_symbol,
                rank_label
            ))),
        )
        .add_child(legacy(&format!("  §8(Место: §b{}§8)", rank_label)));
    player.send_system_message(balance_row, false);

    let mut controls = legacy(" ")
        .add_child(clickable_run(
            "§6[★ Топ-10 богачей]",
            "/baltop",
            "§6Открыть таблицу самых богатых игроков сервера",
        ))
        .add_child(legacy(" "));

    if is_self {
        controls = controls.add_child(clickable_suggest(
            "§a[⇄ Перевести]",
            "/pay ",
            "§aНажмите, чтобы подставить команду /pay <игрок> <сумма>",
        ));
    } else {
        controls = controls.add_child(clickable_suggest(
            &format!("§a[⇄ Перевести {}]", account.player_name),
            &format!("/pay {} ", account.player_name),
            &format!(
                "§aБыстрый перевод игроку §f{}\n§7Введите сумму после пробела",
                account.player_name
            ),
        ));
    }

    if is_admin {
        controls = controls
            .add_child(legacy(" "))
            .add_child(clickable_suggest(
                "§c[⚙ /eco]",
                &format!("/eco give {} ", account.player_name),
                "§cАдмин-управление балансом: /eco give|take|set <игрок> <сумма>",
            ));
    }

    player.send_system_message(controls, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

/// Открывает Топ-10 богачей (`/baltop`) в двойном UI (Bedrock `SimpleForm` + Java `TextComponent`).
pub fn open_baltop_menu(
    player: &Player,
    entries: &[TopEntry],
    currency_symbol: &str,
    total_supply: u128,
    viewer_rank: Option<(usize, u64)>,
) {
    play_click_sound(player);
    let viewer_name = player.get_name();
    let viewer_key = normalize_key(&viewer_name);

    if player.as_bedrock().is_some() {
        let mut buttons = Vec::new();
        let mut actions = Vec::new();

        for entry in entries {
            let badge = medal_prefix(entry.rank);
            buttons.push(SimpleFormButton {
                text: legacy(&format!(
                    "{} §f{}\n§e{} {}",
                    badge,
                    entry.player_name,
                    format_amount(entry.balance),
                    currency_symbol
                )),
                image: None,
            });
            actions.push(BedrockEcoAction::OpenPlayerBalance(
                entry.player_name.clone(),
            ));
        }

        buttons.push(SimpleFormButton {
            text: legacy("§a★ Мой баланс (/bal)"),
            image: None,
        });
        actions.push(BedrockEcoAction::OpenOwnBalance);

        let viewer_line = match viewer_rank {
            Some((r, b)) => format!(
                "§7Ваше место: §b#{r} §8(§e{} {}§8)",
                format_amount(b),
                currency_symbol
            ),
            None => "§7Ваше место: §8нет данных".to_string(),
        };
        let content = format!(
            "§7Всего в экономике: §e{} {}\n{}\n§7Выберите игрока для просмотра профиля:",
            format_amount_u128(total_supply),
            currency_symbol,
            viewer_line
        );

        open_bedrock_form(
            player,
            &viewer_name,
            "§6§l★ ТОП-10 БОГАЧЕЙ KRONOS",
            &content,
            buttons,
            actions,
        );
    }

    player.send_system_message(legacy(LINE_SEP), false);
    player.send_system_message(
        legacy(&format!(
            "§6§l★ ТОП-10 БОГАЧЕЙ СЕРВЕРА §8| §7Оборот: §e{} {}",
            format_amount_u128(total_supply),
            currency_symbol
        )),
        false,
    );

    if entries.is_empty() {
        player.send_system_message(legacy(" §7Пока нет зарегистрированных счетов."), false);
    } else {
        for entry in entries {
            let badge = medal_prefix(entry.rank);
            let is_me = entry.player_key == viewer_key;
            let share = if total_supply > 0 {
                (entry.balance as f64 / total_supply as f64) * 100.0
            } else {
                0.0
            };

            let hover = format!(
                "§6§l#{} {}§r\n§7Баланс: §e{} {}\n§7Доля экономики: §b{:.2}%\n§7Получено: §a+{} {}\n§7Отправлено: §c-{} {}\n\n§a▸ Клик — перевести игроку (/pay)",
                entry.rank,
                entry.player_name,
                format_amount(entry.balance),
                currency_symbol,
                share,
                format_amount(entry.total_received),
                currency_symbol,
                format_amount(entry.total_sent),
                currency_symbol
            );

            let name_label = if is_me {
                format!("§a§l{}", entry.player_name)
            } else {
                format!("§f{}", entry.player_name)
            };

            let mut row = legacy(&format!("  {} ", badge))
                .add_child(clickable_suggest(
                    &name_label,
                    &format!("/pay {} ", entry.player_name),
                    &hover,
                ))
                .add_child(legacy(&format!(
                    " §8— §e§l{} {}",
                    format_amount(entry.balance),
                    currency_symbol
                )));

            if is_me {
                row = row.add_child(legacy(" §a(Вы)"));
            }

            player.send_system_message(row, false);
        }
    }

    let mut footer = legacy(" ");
    if let Some((rank, bal)) = viewer_rank {
        footer = footer.add_child(legacy(&format!(
            "§7Ваша позиция: §b#{rank} §8(§e{} {}§8)  ",
            format_amount(bal),
            currency_symbol
        )));
    }
    footer = footer
        .add_child(clickable_run(
            "§e[Мой баланс]",
            "/bal",
            "§eОткрыть карточку своего баланса",
        ))
        .add_child(legacy(" "))
        .add_child(clickable_suggest(
            "§a[Перевести]",
            "/pay ",
            "§aПодставить команду /pay <игрок> <сумма>",
        ));

    player.send_system_message(footer, false);
    player.send_system_message(legacy(LINE_SEP), false);
}

fn medal_prefix(rank: usize) -> String {
    match rank {
        1 => "§6§l#1".to_string(),
        2 => "§f§l#2".to_string(),
        3 => "§c§l#3".to_string(),
        n => format!("§7#{n}"),
    }
}
