use std::collections::HashMap;
use std::fmt;

use serde::{Deserialize, Serialize};

/// 1.00 монета = 100 минимальных единиц (fixed-point u64 без погрешностей f64).
pub const MINOR_SCALE: u64 = 100;
/// Стартовый баланс нового игрока: 100.00 монет.
pub const DEFAULT_STARTING_BALANCE: u64 = 100 * MINOR_SCALE;

pub fn normalize_key(name: &str) -> String {
    name.trim().to_ascii_lowercase()
}

pub fn is_valid_player_name(name: &str) -> bool {
    let trimmed = name.trim();
    !trimmed.is_empty() && trimmed.len() <= 32 && !trimmed.chars().any(char::is_whitespace)
}

/// Строгий парсер суммы из строки в минимальные единицы `u64` (до 2 знаков после запятой).
/// Поддерживает точку и запятую (`150`, `12.5`, `12,50`).
pub fn parse_amount(raw: &str, allow_zero: bool) -> Result<u64, EconomyError> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(EconomyError::InvalidAmount);
    }
    if s.starts_with('-') {
        return Err(EconomyError::NegativeAmount);
    }
    if s.starts_with('+') {
        return Err(EconomyError::InvalidAmount);
    }

    let normalized = s.replace(',', ".");
    let mut parts = normalized.split('.');
    let whole_str = parts.next().unwrap_or("");
    let frac_opt = parts.next();
    if parts.next().is_some() {
        return Err(EconomyError::InvalidAmount);
    }

    if whole_str.is_empty() || !whole_str.bytes().all(|b| b.is_ascii_digit()) {
        return Err(EconomyError::InvalidAmount);
    }

    let whole: u64 = whole_str.parse().map_err(|_| EconomyError::Overflow)?;
    let whole_minor = whole
        .checked_mul(MINOR_SCALE)
        .ok_or(EconomyError::Overflow)?;

    let frac_minor: u64 = match frac_opt {
        None => 0,
        Some(frac_str) => {
            if frac_str.is_empty()
                || frac_str.len() > 2
                || !frac_str.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(EconomyError::InvalidAmount);
            }
            let parsed: u64 = frac_str.parse().map_err(|_| EconomyError::InvalidAmount)?;
            if frac_str.len() == 1 {
                parsed * 10
            } else {
                parsed
            }
        }
    };

    let total = whole_minor
        .checked_add(frac_minor)
        .ok_or(EconomyError::Overflow)?;

    if !allow_zero && total == 0 {
        return Err(EconomyError::ZeroAmount);
    }

    Ok(total)
}

/// Форматирование целого числа с пробелами-разделителями тысяч (`12500` -> `"12 500"`).
pub fn format_thousands_u128(value: u128) -> String {
    let digits = value.to_string();
    let len = digits.len();
    let mut out = String::with_capacity(len + len / 3);
    for (idx, ch) in digits.chars().enumerate() {
        if idx > 0 && (len - idx) % 3 == 0 {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

/// Форматирование баланса из минимальных единиц `u64` (`12500_00` -> `"12 500"`, `12500_50` -> `"12 500.50"`).
pub fn format_amount(minor_units: u64) -> String {
    let whole = (minor_units / MINOR_SCALE) as u128;
    let frac = minor_units % MINOR_SCALE;
    let whole_fmt = format_thousands_u128(whole);
    if frac == 0 {
        whole_fmt
    } else {
        format!("{whole_fmt}.{frac:02}")
    }
}

/// Форматирование общей денежной массы (`u128`) в минимальных единицах.
pub fn format_amount_u128(minor_units: u128) -> String {
    let scale = MINOR_SCALE as u128;
    let whole = minor_units / scale;
    let frac = minor_units % scale;
    let whole_fmt = format_thousands_u128(whole);
    if frac == 0 {
        whole_fmt
    } else {
        format!("{whole_fmt}.{frac:02}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyError {
    NegativeAmount,
    ZeroAmount,
    InvalidAmount,
    SelfTransfer,
    InvalidPlayerName,
    InsufficientFunds { available: u64, required: u64 },
    Overflow,
}

impl fmt::Display for EconomyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EconomyError::NegativeAmount => {
                write!(f, "Сумма не может быть отрицательной.")
            }
            EconomyError::ZeroAmount => {
                write!(f, "Сумма должна быть больше 0.")
            }
            EconomyError::InvalidAmount => {
                write!(
                    f,
                    "Некорректный формат суммы. Пример: 100 или 25.50 (до 2 знаков после точки)."
                )
            }
            EconomyError::SelfTransfer => {
                write!(f, "Нельзя переводить монеты самому себе.")
            }
            EconomyError::InvalidPlayerName => {
                write!(f, "Указано некорректное имя игрока.")
            }
            EconomyError::InsufficientFunds {
                available,
                required,
            } => {
                write!(
                    f,
                    "Недостаточно средств! Требуется: {}, доступно: {}.",
                    format_amount(*required),
                    format_amount(*available)
                )
            }
            EconomyError::Overflow => {
                write!(f, "Операция отклонена: превышен максимальный лимит баланса.")
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Account {
    pub player_key: String,
    pub player_name: String,
    pub balance: u64,
    #[serde(default)]
    pub total_received: u64,
    #[serde(default)]
    pub total_sent: u64,
    #[serde(default)]
    pub updated_at_ms: u64,
}

impl Account {
    pub fn new(player_name: &str, starting_balance: u64, now_ms: u64) -> Self {
        Self {
            player_key: normalize_key(player_name),
            player_name: player_name.trim().to_string(),
            balance: starting_balance,
            total_received: 0,
            total_sent: 0,
            updated_at_ms: now_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyStore {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default = "default_currency_symbol")]
    pub currency_symbol: String,
    #[serde(default = "default_currency_name")]
    pub currency_name: String,
    #[serde(default = "default_starting_balance")]
    pub starting_balance: u64,
    #[serde(default)]
    pub accounts: HashMap<String, Account>,
}

fn default_version() -> u32 {
    1
}

fn default_currency_symbol() -> String {
    "⛃".to_string()
}

fn default_currency_name() -> String {
    "Кронов".to_string()
}

fn default_starting_balance() -> u64 {
    DEFAULT_STARTING_BALANCE
}

impl Default for EconomyStore {
    fn default() -> Self {
        Self {
            version: default_version(),
            currency_symbol: default_currency_symbol(),
            currency_name: default_currency_name(),
            starting_balance: default_starting_balance(),
            accounts: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferReceipt {
    pub from_name: String,
    pub to_name: String,
    pub amount: u64,
    pub from_new_balance: u64,
    pub to_new_balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminReceipt {
    pub target_name: String,
    pub amount: u64,
    pub old_balance: u64,
    pub new_balance: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopEntry {
    pub rank: usize,
    pub player_key: String,
    pub player_name: String,
    pub balance: u64,
    pub total_received: u64,
    pub total_sent: u64,
}

impl EconomyStore {
    pub fn with_starting_balance(starting_balance: u64) -> Self {
        Self {
            starting_balance,
            ..Default::default()
        }
    }

    /// Гарантирует существование аккаунта игрока. Возвращает `(аккаунт, создан_ли_новый)`.
    pub fn ensure_account(&mut self, player_name: &str, now_ms: u64) -> (&mut Account, bool) {
        let key = normalize_key(player_name);
        let existed = self.accounts.contains_key(&key);
        let starting = self.starting_balance;
        let account = self
            .accounts
            .entry(key)
            .or_insert_with(|| Account::new(player_name, starting, now_ms));
        if !player_name.trim().is_empty() {
            account.player_name = player_name.trim().to_string();
        }
        (account, !existed)
    }

    pub fn get_account(&self, player_name: &str) -> Option<&Account> {
        let key = normalize_key(player_name);
        self.accounts.get(&key)
    }

    pub fn balance_of(&self, player_name: &str) -> u64 {
        self.get_account(player_name)
            .map(|a| a.balance)
            .unwrap_or(self.starting_balance)
    }

    /// Атомарный перевод между игроками.
    /// Сначала проверяет все инварианты (без мутации), и только затем обновляет оба счёта.
    pub fn transfer(
        &mut self,
        from_name: &str,
        to_name: &str,
        amount: u64,
        now_ms: u64,
    ) -> Result<TransferReceipt, EconomyError> {
        if !is_valid_player_name(from_name) || !is_valid_player_name(to_name) {
            return Err(EconomyError::InvalidPlayerName);
        }
        if amount == 0 {
            return Err(EconomyError::ZeroAmount);
        }

        let from_key = normalize_key(from_name);
        let to_key = normalize_key(to_name);
        if from_key == to_key {
            return Err(EconomyError::SelfTransfer);
        }

        let from_current = self
            .accounts
            .get(&from_key)
            .map(|a| a.balance)
            .unwrap_or(self.starting_balance);
        let to_current = self
            .accounts
            .get(&to_key)
            .map(|a| a.balance)
            .unwrap_or(self.starting_balance);

        let from_new = from_current.checked_sub(amount).ok_or(
            EconomyError::InsufficientFunds {
                available: from_current,
                required: amount,
            },
        )?;
        let to_new = to_current
            .checked_add(amount)
            .ok_or(EconomyError::Overflow)?;

        let starting = self.starting_balance;
        let from_acc = self
            .accounts
            .entry(from_key)
            .or_insert_with(|| Account::new(from_name, starting, now_ms));
        from_acc.player_name = from_name.trim().to_string();
        from_acc.balance = from_new;
        from_acc.total_sent = from_acc.total_sent.saturating_add(amount);
        from_acc.updated_at_ms = now_ms;
        let resolved_from_name = from_acc.player_name.clone();

        let to_acc = self
            .accounts
            .entry(to_key)
            .or_insert_with(|| Account::new(to_name, starting, now_ms));
        if to_acc.player_name.is_empty() {
            to_acc.player_name = to_name.trim().to_string();
        }
        to_acc.balance = to_new;
        to_acc.total_received = to_acc.total_received.saturating_add(amount);
        to_acc.updated_at_ms = now_ms;
        let resolved_to_name = to_acc.player_name.clone();

        Ok(TransferReceipt {
            from_name: resolved_from_name,
            to_name: resolved_to_name,
            amount,
            from_new_balance: from_new,
            to_new_balance: to_new,
        })
    }

    pub fn admin_give(
        &mut self,
        target_name: &str,
        amount: u64,
        now_ms: u64,
    ) -> Result<AdminReceipt, EconomyError> {
        if !is_valid_player_name(target_name) {
            return Err(EconomyError::InvalidPlayerName);
        }
        if amount == 0 {
            return Err(EconomyError::ZeroAmount);
        }

        let key = normalize_key(target_name);
        let old_balance = self
            .accounts
            .get(&key)
            .map(|a| a.balance)
            .unwrap_or(self.starting_balance);
        let new_balance = old_balance
            .checked_add(amount)
            .ok_or(EconomyError::Overflow)?;

        let starting = self.starting_balance;
        let acc = self
            .accounts
            .entry(key)
            .or_insert_with(|| Account::new(target_name, starting, now_ms));
        acc.balance = new_balance;
        acc.updated_at_ms = now_ms;

        Ok(AdminReceipt {
            target_name: acc.player_name.clone(),
            amount,
            old_balance,
            new_balance,
        })
    }

    pub fn admin_take(
        &mut self,
        target_name: &str,
        amount: u64,
        now_ms: u64,
    ) -> Result<AdminReceipt, EconomyError> {
        if !is_valid_player_name(target_name) {
            return Err(EconomyError::InvalidPlayerName);
        }
        if amount == 0 {
            return Err(EconomyError::ZeroAmount);
        }

        let key = normalize_key(target_name);
        let old_balance = self
            .accounts
            .get(&key)
            .map(|a| a.balance)
            .unwrap_or(self.starting_balance);
        let new_balance = old_balance.checked_sub(amount).ok_or(
            EconomyError::InsufficientFunds {
                available: old_balance,
                required: amount,
            },
        )?;

        let starting = self.starting_balance;
        let acc = self
            .accounts
            .entry(key)
            .or_insert_with(|| Account::new(target_name, starting, now_ms));
        acc.balance = new_balance;
        acc.updated_at_ms = now_ms;

        Ok(AdminReceipt {
            target_name: acc.player_name.clone(),
            amount,
            old_balance,
            new_balance,
        })
    }

    pub fn admin_set(
        &mut self,
        target_name: &str,
        amount: u64,
        now_ms: u64,
    ) -> Result<AdminReceipt, EconomyError> {
        if !is_valid_player_name(target_name) {
            return Err(EconomyError::InvalidPlayerName);
        }

        let key = normalize_key(target_name);
        let starting = self.starting_balance;
        let acc = self
            .accounts
            .entry(key)
            .or_insert_with(|| Account::new(target_name, starting, now_ms));
        let old_balance = acc.balance;
        acc.balance = amount;
        acc.updated_at_ms = now_ms;

        Ok(AdminReceipt {
            target_name: acc.player_name.clone(),
            amount,
            old_balance,
            new_balance: amount,
        })
    }

    pub fn total_supply(&self) -> u128 {
        self.accounts.values().map(|a| a.balance as u128).sum()
    }

    /// Возвращает отсортированный Топ-N игроков по балансу (при равенстве — по алфавиту).
    pub fn top_balances(&self, limit: usize) -> Vec<TopEntry> {
        let mut sorted: Vec<&Account> = self.accounts.values().collect();
        sorted.sort_by(|a, b| {
            b.balance
                .cmp(&a.balance)
                .then_with(|| a.player_key.cmp(&b.player_key))
        });

        sorted
            .into_iter()
            .take(limit)
            .enumerate()
            .map(|(idx, acc)| TopEntry {
                rank: idx + 1,
                player_key: acc.player_key.clone(),
                player_name: acc.player_name.clone(),
                balance: acc.balance,
                total_received: acc.total_received,
                total_sent: acc.total_sent,
            })
            .collect()
    }

    /// Позиция игрока в глобальном рейтинге (1-based).
    pub fn rank_of(&self, player_name: &str) -> Option<(usize, u64)> {
        let target_key = normalize_key(player_name);
        let mut sorted: Vec<&Account> = self.accounts.values().collect();
        sorted.sort_by(|a, b| {
            b.balance
                .cmp(&a.balance)
                .then_with(|| a.player_key.cmp(&b.player_key))
        });
        sorted
            .into_iter()
            .position(|a| a.player_key == target_key)
            .map(|idx| (idx + 1, self.balance_of(player_name)))
    }
}

#[derive(Debug, Clone)]
pub enum BedrockEcoAction {
    OpenOwnBalance,
    OpenBaltop,
    OpenPlayerBalance(String),
    SuggestPay(String),
}

#[derive(Debug, Clone)]
pub struct PendingBedrockEcoForm {
    pub player_key: String,
    pub actions: Vec<BedrockEcoAction>,
}

pub struct EconomyState {
    pub data_folder: String,
    pub store: EconomyStore,
    pub pending_forms: HashMap<u32, PendingBedrockEcoForm>,
}

impl EconomyState {
    pub fn new(data_folder: String, store: EconomyStore) -> Self {
        Self {
            data_folder,
            store,
            pending_forms: HashMap::new(),
        }
    }
}
