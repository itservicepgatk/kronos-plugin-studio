pub mod commands;
pub mod menu;
pub mod models;
pub mod storage;

use pumpkin_plugin_api::{
    events::{BedrockFormResponseEvent, EventHandler, EventPriority, PlayerJoinEvent, PlayerLeaveEvent},
    permission::{Permission, PermissionDefault, PermissionLevel},
    permissions::{FS_READ_DATA, FS_WRITE_DATA},
    register_plugin,
    wit::pumpkin::plugin::event::{
        BedrockFormResponseEventData, PlayerJoinEventData, PlayerLeaveEventData,
    },
    Context, Plugin, PluginMetadata, Server,
};

use crate::commands::{execute_bedrock_action, PERM_ADMIN, PERM_PLAYER};
use crate::menu::play_error_sound;
use crate::models::normalize_key;
use crate::storage::{init_storage, now_ms, save_current_store, with_state};

struct JoinHandler;
impl EventHandler<PlayerJoinEvent> for JoinHandler {
    fn handle(&self, _server: Server, event: PlayerJoinEventData) -> PlayerJoinEventData {
        let player_name = event.player.get_name();
        let now = now_ms();

        let created = with_state(|st| {
            let (_, created) = st.store.ensure_account(&player_name, now);
            created
        });

        if created {
            save_current_store();
        }

        event
    }
}

struct LeaveHandler;
impl EventHandler<PlayerLeaveEvent> for LeaveHandler {
    fn handle(&self, _server: Server, event: PlayerLeaveEventData) -> PlayerLeaveEventData {
        let key = normalize_key(&event.player.get_name());
        with_state(|st| {
            st.pending_forms.retain(|_, f| f.player_key != key);
        });
        event
    }
}

struct BedrockFormHandler;
impl EventHandler<BedrockFormResponseEvent> for BedrockFormHandler {
    fn handle(
        &self,
        server: Server,
        event: BedrockFormResponseEventData,
    ) -> BedrockFormResponseEventData {
        let pending_opt = with_state(|st| st.pending_forms.remove(&event.form_id));
        if let (Some(pending), Some(raw_response)) = (pending_opt, event.response_data.as_deref()) {
            let p_key = normalize_key(&event.player.get_name());
            if pending.player_key == p_key {
                if let Ok(button_idx) = raw_response.trim().parse::<usize>() {
                    if let Some(action) = pending.actions.into_iter().nth(button_idx) {
                        execute_bedrock_action(&event.player, &server, action);
                    } else {
                        play_error_sound(&event.player);
                    }
                }
            }
        }
        event
    }
}

pub struct KronosEconomyPlugin;

impl Plugin for KronosEconomyPlugin {
    fn new() -> Self {
        Self
    }

    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "kronos-economy".into(),
            version: "1.0.0".into(),
            authors: vec!["KRONOS Team".into()],
            description: "Server economy system with currency, safe transfers, /baltop, /eco admin tools, and dual Java/Bedrock UI for Pumpkin 26.3".into(),
            dependencies: vec![],
            permissions: vec![FS_READ_DATA.into(), FS_WRITE_DATA.into()],
        }
    }

    fn on_load(&self, context: Context) -> Result<(), String> {
        init_storage(context.get_data_folder());

        context.register_permission(&Permission {
            node: PERM_PLAYER.to_string(),
            description: "Access to /money, /bal, /pay and /baltop commands".to_string(),
            default: PermissionDefault::Allow,
            children: vec![],
        })?;

        context.register_permission(&Permission {
            node: PERM_ADMIN.to_string(),
            description: "Administrative access to /eco give|take|set commands".to_string(),
            default: PermissionDefault::Op(PermissionLevel::Three),
            children: vec![],
        })?;

        commands::register_all(&context);

        context.register_event_handler::<PlayerJoinEvent, _>(
            JoinHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<PlayerLeaveEvent, _>(
            LeaveHandler,
            EventPriority::Normal,
            false,
        )?;
        context.register_event_handler::<BedrockFormResponseEvent, _>(
            BedrockFormHandler,
            EventPriority::Normal,
            false,
        )?;

        tracing::info!("KRONOS Economy v1.0.0 loaded (wasm32-wasip2, fixed-point u64).");
        Ok(())
    }

    fn on_unload(&self, _context: Context) -> Result<(), String> {
        save_current_store();
        tracing::info!("KRONOS Economy unloaded: economy.json saved.");
        Ok(())
    }
}

register_plugin!(KronosEconomyPlugin);

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::{Arc, Barrier, Mutex};
    use std::thread;

    use crate::models::{
        format_amount, parse_amount, EconomyError, EconomyStore, MINOR_SCALE,
    };
    use crate::storage::{load_store_from_path, save_store_to_path, STORE_FILENAME};

    #[test]
    fn parse_and_format_amounts_correctly() {
        assert_eq!(parse_amount("100", false), Ok(10_000));
        assert_eq!(parse_amount("12.5", false), Ok(1_250));
        assert_eq!(parse_amount("12,05", false), Ok(1_205));
        assert_eq!(parse_amount("0", true), Ok(0));

        assert_eq!(parse_amount("0", false), Err(EconomyError::ZeroAmount));
        assert_eq!(parse_amount("-50", false), Err(EconomyError::NegativeAmount));
        assert_eq!(parse_amount("-0.01", true), Err(EconomyError::NegativeAmount));
        assert_eq!(parse_amount("10.123", false), Err(EconomyError::InvalidAmount));
        assert_eq!(parse_amount("10.", false), Err(EconomyError::InvalidAmount));
        assert_eq!(parse_amount("abc", false), Err(EconomyError::InvalidAmount));
        assert_eq!(
            parse_amount("999999999999999999999999999", false),
            Err(EconomyError::Overflow)
        );

        assert_eq!(format_amount(0), "0");
        assert_eq!(format_amount(100 * MINOR_SCALE), "100");
        assert_eq!(format_amount(12_500 * MINOR_SCALE), "12 500");
        assert_eq!(format_amount(12_500 * MINOR_SCALE + 5), "12 500.05");
        assert_eq!(format_amount(1_000_000 * MINOR_SCALE + 50), "1 000 000.50");
    }

    #[test]
    fn transfer_validates_self_negative_zero_overdraft_and_overflow() {
        let mut store = EconomyStore::with_starting_balance(500 * MINOR_SCALE);
        store.ensure_account("Steve", 1);
        store.ensure_account("Alex", 1);

        let receipt = store
            .transfer("Steve", "Alex", 125 * MINOR_SCALE + 50, 2)
            .expect("valid transfer must succeed");
        assert_eq!(receipt.from_new_balance, 374 * MINOR_SCALE + 50);
        assert_eq!(receipt.to_new_balance, 625 * MINOR_SCALE + 50);
        assert_eq!(store.total_supply(), 1_000 * MINOR_SCALE as u128);

        assert_eq!(
            store.transfer("Steve", "sTeVe", 10 * MINOR_SCALE, 3),
            Err(EconomyError::SelfTransfer)
        );
        assert_eq!(
            store.transfer("Steve", "Alex", 0, 3),
            Err(EconomyError::ZeroAmount)
        );

        let err = store.transfer("Steve", "Alex", 999 * MINOR_SCALE, 4);
        assert!(matches!(err, Err(EconomyError::InsufficientFunds { .. })));
        assert_eq!(store.balance_of("Steve"), 374 * MINOR_SCALE + 50);
        assert_eq!(store.balance_of("Alex"), 625 * MINOR_SCALE + 50);

        store
            .admin_set("Rich", u64::MAX - 50, 5)
            .expect("set near u64::MAX");
        assert_eq!(
            store.transfer("Steve", "Rich", 100, 6),
            Err(EconomyError::Overflow)
        );
        assert_eq!(store.balance_of("Steve"), 374 * MINOR_SCALE + 50);
    }

    #[test]
    fn admin_give_take_set_operations() {
        let mut store = EconomyStore::with_starting_balance(0);

        let r1 = store
            .admin_give("Notch", 1_000 * MINOR_SCALE, 10)
            .expect("give");
        assert_eq!(r1.old_balance, 0);
        assert_eq!(r1.new_balance, 1_000 * MINOR_SCALE);

        let r2 = store
            .admin_take("Notch", 250 * MINOR_SCALE, 11)
            .expect("take");
        assert_eq!(r2.new_balance, 750 * MINOR_SCALE);

        assert!(matches!(
            store.admin_take("Notch", 800 * MINOR_SCALE, 12),
            Err(EconomyError::InsufficientFunds { .. })
        ));

        let r3 = store.admin_set("Notch", 0, 13).expect("set to zero");
        assert_eq!(r3.new_balance, 0);
    }

    #[test]
    fn baltop_returns_top_10_sorted_deterministically() {
        let mut store = EconomyStore::with_starting_balance(0);
        for i in 1..=15u64 {
            let name = format!("Player{:02}", i);
            store
                .admin_set(&name, i * 100 * MINOR_SCALE, 1)
                .expect("set balance");
        }
        store
            .admin_set("AlphaTie", 1_500 * MINOR_SCALE, 1)
            .expect("tie player");

        let top = store.top_balances(10);
        assert_eq!(top.len(), 10);
        assert_eq!(top[0].rank, 1);
        assert_eq!(top[0].player_name, "AlphaTie");
        assert_eq!(top[1].rank, 2);
        assert_eq!(top[1].player_name, "Player15");
        assert_eq!(top[9].rank, 10);
        assert_eq!(top[9].player_name, "Player07");
    }

    #[test]
    fn atomic_storage_roundtrip_leaves_no_tmp_file() {
        let temp_dir = std::env::temp_dir().join(format!(
            "kronos_economy_test_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).expect("create temp dir");

        let file_path = temp_dir.join(STORE_FILENAME);
        let tmp_path = file_path.with_extension("json.tmp");

        let mut store = EconomyStore::with_starting_balance(100 * MINOR_SCALE);
        store.admin_set("Alex", 777 * MINOR_SCALE + 25, 100).unwrap();
        store.admin_set("Steve", 222 * MINOR_SCALE + 75, 100).unwrap();

        save_store_to_path(&file_path, &store).expect("atomic save");
        assert!(file_path.exists());
        assert!(!tmp_path.exists(), "temporary .tmp file must be renamed away");

        let loaded = load_store_from_path(&file_path).expect("load saved economy.json");
        assert_eq!(loaded.balance_of("Alex"), 777 * MINOR_SCALE + 25);
        assert_eq!(loaded.balance_of("Steve"), 222 * MINOR_SCALE + 75);

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn concurrent_transfers_prevent_money_duplication_and_double_spend() {
        let mut initial = EconomyStore::with_starting_balance(0);
        initial.admin_set("Alice", 1_000 * MINOR_SCALE, 1).unwrap();
        initial.admin_set("Bob", 0, 1).unwrap();
        initial.admin_set("Charlie", 0, 1).unwrap();

        let expected_supply = initial.total_supply();
        let shared = Arc::new(Mutex::new(initial));

        let threads_count = 8;
        let barrier = Arc::new(Barrier::new(threads_count));
        let mut handles = Vec::with_capacity(threads_count);

        for i in 0..threads_count {
            let store_arc = Arc::clone(&shared);
            let bar = Arc::clone(&barrier);
            handles.push(thread::spawn(move || {
                bar.wait();
                let target = if i % 2 == 0 { "Bob" } else { "Charlie" };
                let mut guard = store_arc.lock().unwrap();
                guard
                    .transfer("Alice", target, 200 * MINOR_SCALE, 10 + i as u64)
                    .is_ok()
            }));
        }

        let succeeded: usize = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|&ok| ok)
            .count();

        let final_store = shared.lock().unwrap();
        assert_eq!(succeeded, 5, "exactly 5 of 8 concurrent 200-coin transfers must succeed");
        assert_eq!(final_store.balance_of("Alice"), 0);
        assert_eq!(
            final_store.total_supply(),
            expected_supply,
            "total server money supply must remain strictly conserved"
        );
    }
}
