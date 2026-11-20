use pumpkin_plugin_api::{
    command::{
        ArgumentType, Command, CommandError, CommandNode, CommandSender, ConsumedArgs, StringType,
    },
    commands::CommandHandler,
    wit::pumpkin::plugin::command::Arg,
    Context, Player, Server,
};

use crate::menu::{
    clickable_suggest, legacy, open_balance_menu, open_baltop_menu, play_coin_sound,
    play_error_sound,
};
use crate::models::{
    format_amount, format_amount_u128, normalize_key, parse_amount, Account, BedrockEcoAction,
};
use crate::storage::{now_ms, save_current_store, with_state};

pub const PERM_PLAYER: &str = "kronos-economy:player";
pub const PERM_ADMIN: &str = "kronos-economy:admin";

fn extract_args(args: &ConsumedArgs) -> String {
    match args.get_value("args") {
        Arg::Simple(s) | Arg::Msg(s) => s,
        _ => String::new(),
    }
}

fn find_online_player_ci(server: &Server, target_name: &str) -> Option<Player> {
    let target_key = normalize_key(target_name);
    server
        .get_all_players()
        .into_iter()
        .find(|p| normalize_key(&p.get_name()) == target_key)
}

fn is_admin_sender(sender: &CommandSender) -> bool {
    if sender.is_console() {
        return true;
    }
    if let Some(p) = sender.as_player() {
        return p.has_permission(PERM_ADMIN);
    }
    false
}

/// Обработчик нажатий кнопок в Bedrock `SimpleForm`.
pub fn execute_bedrock_action(player: &Player, _server: &Server, action: BedrockEcoAction) {
    let viewer_name = player.get_name();
    let is_admin = player.has_permission(PERM_ADMIN);
    let now = now_ms();

    match action {
        BedrockEcoAction::OpenOwnBalance => {
            let (account, symbol, rank_pos, total_accounts) = with_state(|st| {
                let (acc, _) = st.store.ensure_account(&viewer_name, now);
                let cloned = acc.clone();
                let rank = st.store.rank_of(&viewer_name).map(|(r, _)| r);
                (
                    cloned,
                    st.store.currency_symbol.clone(),
                    rank,
                    st.store.accounts.len(),
                )
            });
            open_balance_menu(player, &account, &symbol, rank_pos, total_accounts, true, is_admin);
        }
        BedrockEcoAction::OpenBaltop => {
            let (entries, symbol, total_supply, viewer_rank) = with_state(|st| {
                (
                    st.store.top_balances(10),
                    st.store.currency_symbol.clone(),
                    st.store.total_supply(),
                    st.store.rank_of(&viewer_name),
                )
            });
            open_baltop_menu(player, &entries, &symbol, total_supply, viewer_rank);
        }
        BedrockEcoAction::OpenPlayerBalance(target_name) => {
            let is_self = normalize_key(&viewer_name) == normalize_key(&target_name);
            let (account, symbol, rank_pos, total_accounts) = with_state(|st| {
                let acc = st
                    .store
                    .get_account(&target_name)
                    .cloned()
                    .unwrap_or_else(|| Account::new(&target_name, st.store.starting_balance, now));
                let rank = st.store.rank_of(&target_name).map(|(r, _)| r);
                (
                    acc,
                    st.store.currency_symbol.clone(),
                    rank,
                    st.store.accounts.len(),
                )
            });
            open_balance_menu(
                player,
                &account,
                &symbol,
                rank_pos,
                total_accounts,
                is_self,
                is_admin,
            );
        }
        BedrockEcoAction::SuggestPay(target_name) => {
            player.send_system_message(
                legacy(&format!(
                    "§6§l✦ KRONOS ECO §8» §7Для перевода введите в чат: §e/pay {} <сумма>",
                    target_name
                )),
                false,
            );
        }
    }
}

struct BalanceCmd;
impl CommandHandler for BalanceCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        let now = now_ms();

        if let Some(player) = sender.as_player() {
            let viewer_name = player.get_name();
            let is_admin = player.has_permission(PERM_ADMIN);

            if let Some(target_arg) = tokens.first() {
                let is_self = normalize_key(&viewer_name) == normalize_key(target_arg);
                let (account_opt, symbol, rank_pos, total_accounts) = with_state(|st| {
                    let acc = if is_self {
                        let (a, _) = st.store.ensure_account(&viewer_name, now);
                        Some(a.clone())
                    } else {
                        st.store.get_account(target_arg).cloned()
                    };
                    let rank = st.store.rank_of(target_arg).map(|(r, _)| r);
                    (
                        acc,
                        st.store.currency_symbol.clone(),
                        rank,
                        st.store.accounts.len(),
                    )
                });

                match account_opt {
                    Some(account) => {
                        open_balance_menu(
                            &player,
                            &account,
                            &symbol,
                            rank_pos,
                            total_accounts,
                            is_self,
                            is_admin,
                        );
                    }
                    None => {
                        play_error_sound(&player);
                        player.send_system_message(
                            legacy(&format!(
                                "§6§l✦ KRONOS ECO §8» §cИгрок §f{} §cещё не имеет счёта на сервере.",
                                target_arg
                            )),
                            false,
                        );
                    }
                }
            } else {
                let (account, created, symbol, rank_pos, total_accounts) = with_state(|st| {
                    let (a, created) = st.store.ensure_account(&viewer_name, now);
                    let cloned = a.clone();
                    let rank = st.store.rank_of(&viewer_name).map(|(r, _)| r);
                    (
                        cloned,
                        created,
                        st.store.currency_symbol.clone(),
                        rank,
                        st.store.accounts.len(),
                    )
                });
                if created {
                    save_current_store();
                }
                open_balance_menu(
                    &player,
                    &account,
                    &symbol,
                    rank_pos,
                    total_accounts,
                    true,
                    is_admin,
                );
            }
            return Ok(0);
        }

        let Some(target_arg) = tokens.first() else {
            sender.send_error(legacy("§cИспользование из консоли: /bal <игрок>"));
            return Ok(0);
        };
        let (acc_opt, symbol) = with_state(|st| {
            (
                st.store.get_account(target_arg).cloned(),
                st.store.currency_symbol.clone(),
            )
        });
        match acc_opt {
            Some(acc) => {
                sender.send_message(legacy(&format!(
                    "§6[KRONOS ECO] §7Баланс §f{}§7: §e{} {}",
                    acc.player_name,
                    format_amount(acc.balance),
                    symbol
                )));
            }
            None => {
                sender.send_error(legacy(&format!("§cСчёт игрока {} не найден.", target_arg)));
            }
        }
        Ok(0)
    }
}

struct PayCmd;
impl CommandHandler for PayCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        let Some(player) = sender.as_player() else {
            sender.send_error(legacy(
                "§cКоманда /pay доступна только игрокам. Для консоли используйте /eco give <игрок> <сумма>.",
            ));
            return Ok(0);
        };

        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        if tokens.len() < 2 {
            play_error_sound(&player);
            player.send_system_message(
                legacy("§6§l✦ KRONOS ECO §8» §eИспользование: §f/pay <игрок> <сумма>"),
                false,
            );
            return Ok(0);
        }

        let target_arg = tokens[0];
        let amount_raw = tokens[1];
        let amount = match parse_amount(amount_raw, false) {
            Ok(v) => v,
            Err(err) => {
                play_error_sound(&player);
                player.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS ECO §8» §c✗ {}", err)),
                    false,
                );
                return Ok(0);
            }
        };

        let sender_name = player.get_name();
        let online_target = find_online_player_ci(&server, target_arg);
        let resolved_target_name = online_target
            .as_ref()
            .map(|p| p.get_name())
            .unwrap_or_else(|| target_arg.to_string());
        let now = now_ms();

        let (result, symbol) = with_state(|st| {
            let res = st
                .store
                .transfer(&sender_name, &resolved_target_name, amount, now);
            (res, st.store.currency_symbol.clone())
        });

        match result {
            Ok(receipt) => {
                save_current_store();

                play_coin_sound(&player);
                player.send_system_message(
                    legacy(&format!(
                        "§6§l✦ KRONOS ECO §8» §aВы перевели §e{} {} §aигроку §f{}§a. Остаток: §e{} {}",
                        format_amount(receipt.amount),
                        symbol,
                        receipt.to_name,
                        format_amount(receipt.from_new_balance),
                        symbol
                    )),
                    false,
                );

                if let Some(target_player) = online_target {
                    play_coin_sound(&target_player);
                    let notice = legacy(&format!(
                        "§6§l✦ KRONOS ECO §8» §aИгрок §f{} §aперевёл вам §e+{} {}§a! Баланс: §e{} {} ",
                        receipt.from_name,
                        format_amount(receipt.amount),
                        symbol,
                        format_amount(receipt.to_new_balance),
                        symbol
                    ))
                    .add_child(clickable_suggest(
                        "§b[Ответить]",
                        &format!("/pay {} ", receipt.from_name),
                        &format!("§bПеревести монеты игроку {}", receipt.from_name),
                    ));
                    target_player.send_system_message(notice, false);
                }
            }
            Err(err) => {
                play_error_sound(&player);
                player.send_system_message(
                    legacy(&format!("§6§l✦ KRONOS ECO §8» §c✗ {}", err)),
                    false,
                );
            }
        }

        Ok(0)
    }
}

struct BaltopCmd;
impl CommandHandler for BaltopCmd {
    fn handle(
        &self,
        sender: CommandSender,
        _server: Server,
        _args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if let Some(player) = sender.as_player() {
            let viewer_name = player.get_name();
            let (entries, symbol, total_supply, viewer_rank) = with_state(|st| {
                (
                    st.store.top_balances(10),
                    st.store.currency_symbol.clone(),
                    st.store.total_supply(),
                    st.store.rank_of(&viewer_name),
                )
            });
            open_baltop_menu(&player, &entries, &symbol, total_supply, viewer_rank);
            return Ok(0);
        }

        let (entries, symbol, total_supply) = with_state(|st| {
            (
                st.store.top_balances(10),
                st.store.currency_symbol.clone(),
                st.store.total_supply(),
            )
        });
        sender.send_message(legacy(&format!(
            "§6=== ТОП-10 БОГАЧЕЙ (Всего: {} {}) ===",
            format_amount_u128(total_supply),
            symbol
        )));
        for entry in entries {
            sender.send_message(legacy(&format!(
                " #{:<2} {} — {} {}",
                entry.rank,
                entry.player_name,
                format_amount(entry.balance),
                symbol
            )));
        }
        Ok(0)
    }
}

struct EcoAdminCmd;
impl CommandHandler for EcoAdminCmd {
    fn handle(
        &self,
        sender: CommandSender,
        server: Server,
        args: ConsumedArgs,
    ) -> Result<i32, CommandError> {
        if !is_admin_sender(&sender) {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(
                "§6§l✦ KRONOS ECO §8» §c✗ Недостаточно прав (требуется kronos-economy:admin).",
            ));
            return Ok(0);
        }

        let raw = extract_args(&args);
        let tokens: Vec<&str> = raw.split_whitespace().collect();
        if tokens.len() < 3 {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(
                "§6§l✦ KRONOS ECO §8» §eИспользование: §f/eco <give|take|set> <игрок> <сумма>",
            ));
            return Ok(0);
        }

        let action = tokens[0].to_ascii_lowercase();
        let target_arg = tokens[1];
        let amount_raw = tokens[2];

        let allow_zero = action == "set";
        let amount = match parse_amount(amount_raw, allow_zero) {
            Ok(v) => v,
            Err(err) => {
                if let Some(p) = sender.as_player() {
                    play_error_sound(&p);
                }
                sender.send_error(legacy(&format!("§6§l✦ KRONOS ECO §8» §c✗ {}", err)));
                return Ok(0);
            }
        };

        let online_target = find_online_player_ci(&server, target_arg);
        let resolved_target = online_target
            .as_ref()
            .map(|p| p.get_name())
            .unwrap_or_else(|| target_arg.to_string());
        let now = now_ms();

        let (op_result, symbol) = with_state(|st| {
            let res = match action.as_str() {
                "give" => st.store.admin_give(&resolved_target, amount, now),
                "take" => st.store.admin_take(&resolved_target, amount, now),
                "set" => st.store.admin_set(&resolved_target, amount, now),
                _ => return (None, st.store.currency_symbol.clone()),
            };
            (Some(res), st.store.currency_symbol.clone())
        });

        let Some(res) = op_result else {
            if let Some(p) = sender.as_player() {
                play_error_sound(&p);
            }
            sender.send_error(legacy(
                "§6§l✦ KRONOS ECO §8» §cНеизвестное действие. Доступно: §egive§c, §etake§c, §eset§c.",
            ));
            return Ok(0);
        };

        match res {
            Ok(receipt) => {
                save_current_store();
                if let Some(p) = sender.as_player() {
                    play_coin_sound(&p);
                }

                let (verb_admin, verb_player) = match action.as_str() {
                    "give" => (
                        format!(
                            "§aВыдано §e+{} {} §aигроку §f{}",
                            format_amount(receipt.amount),
                            symbol,
                            receipt.target_name
                        ),
                        format!(
                            "§aАдминистратор выдал вам §e+{} {}",
                            format_amount(receipt.amount),
                            symbol
                        ),
                    ),
                    "take" => (
                        format!(
                            "§eСписано §c-{} {} §eу игрока §f{}",
                            format_amount(receipt.amount),
                            symbol,
                            receipt.target_name
                        ),
                        format!(
                            "§eАдминистратор списал с вашего счёта §c-{} {}",
                            format_amount(receipt.amount),
                            symbol
                        ),
                    ),
                    _ => (
                        format!(
                            "§bБаланс игрока §f{} §bустановлен в §e{} {}",
                            receipt.target_name,
                            format_amount(receipt.new_balance),
                            symbol
                        ),
                        format!(
                            "§bАдминистратор установил ваш баланс: §e{} {}",
                            format_amount(receipt.new_balance),
                            symbol
                        ),
                    ),
                };

                sender.send_message(legacy(&format!(
                    "§6§l✦ KRONOS ECO §8» {}§7. Новый баланс: §e{} {}",
                    verb_admin,
                    format_amount(receipt.new_balance),
                    symbol
                )));

                if let Some(target_player) = online_target {
                    play_coin_sound(&target_player);
                    target_player.send_system_message(
                        legacy(&format!(
                            "§6§l✦ KRONOS ECO §8» {}§7. Текущий баланс: §e{} {}",
                            verb_player,
                            format_amount(receipt.new_balance),
                            symbol
                        )),
                        false,
                    );
                }
            }
            Err(err) => {
                if let Some(p) = sender.as_player() {
                    play_error_sound(&p);
                }
                sender.send_error(legacy(&format!("§6§l✦ KRONOS ECO §8» §c✗ {}", err)));
            }
        }

        Ok(0)
    }
}

pub fn register_all(context: &Context) {
    let money = Command::new(
        &["money".into(), "bal".into(), "balance".into()],
        "Показать баланс игрока (/money [игрок])",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(BalanceCmd),
    )
    .execute(BalanceCmd);
    context.register_command(money, PERM_PLAYER);

    let pay = Command::new(
        &["pay".into()],
        "Перевести монеты другому игроку (/pay <игрок> <сумма>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy)).execute(PayCmd),
    )
    .execute(PayCmd);
    context.register_command(pay, PERM_PLAYER);

    let baltop = Command::new(
        &["baltop".into(), "moneytop".into()],
        "Топ-10 самых богатых игроков сервера (/baltop)",
    )
    .execute(BaltopCmd);
    context.register_command(baltop, PERM_PLAYER);

    let eco = Command::new(
        &["eco".into()],
        "Администрирование экономики (/eco give|take|set <игрок> <сумма>)",
    )
    .then(
        CommandNode::argument("args", &ArgumentType::String(StringType::Greedy))
            .execute(EcoAdminCmd),
    )
    .execute(EcoAdminCmd);
    context.register_command(eco, PERM_ADMIN);
}
