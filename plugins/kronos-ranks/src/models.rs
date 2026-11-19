use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RankCategory {
    Player,
    Donate,
    Staff,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum StaffDepartment {
    None,
    Moderation,
    Recruitment,
    Survival,
    Technical,
    Management,
}

impl StaffDepartment {
    pub fn label_ru(&self) -> &'static str {
        match self {
            StaffDepartment::None => "§7Игроки",
            StaffDepartment::Moderation => "§9Отдел Модерации",
            StaffDepartment::Recruitment => "§dОтдел Набора (HR)",
            StaffDepartment::Survival => "§2Отдел Выживания",
            StaffDepartment::Technical => "§bТехнический Отдел",
            StaffDepartment::Management => "§4Руководство",
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            StaffDepartment::None => "NONE",
            StaffDepartment::Moderation => "MODERATION",
            StaffDepartment::Recruitment => "RECRUITMENT",
            StaffDepartment::Survival => "SURVIVAL",
            StaffDepartment::Technical => "TECHNICAL",
            StaffDepartment::Management => "MANAGEMENT",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankPerks {
    pub max_homes: u32,
    pub max_regions: u32,
    pub max_region_volume: u64,
    pub tp_warmup_seconds: u32,
    pub rtp_cooldown_seconds: u32,
    pub xp_multiplier: f32,
    pub keep_xp_on_death: bool,
    pub colored_chat: bool,
    pub rgb_chat: bool,
    pub custom_suffix: bool,
    pub fly_in_own_region: bool,
    pub border_tier: u8,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankDefinition {
    pub id: String,
    pub display_name: String,
    pub category: RankCategory,
    pub department: StaffDepartment,
    pub weight: u32,
    pub prefix: String,
    pub suffix: String,
    pub name_color: String,
    pub chat_color: String,
    pub price_rub: u32,
    pub inherit_from: Option<String>,
    pub max_assignable_weight: u32,
    pub assignable_departments: Vec<String>,
    pub permissions: Vec<String>,
    pub perks: RankPerks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RanksConfig {
    pub server_name: String,
    pub store_url: String,
    pub autosave_interval_seconds: u64,
    pub bridge_export_path: String,
    pub tab_header: String,
    pub tab_footer: String,
    pub ranks: Vec<RankDefinition>,
}

impl RanksConfig {
    pub fn find(&self, rank_id: &str) -> Option<&RankDefinition> {
        let lower = rank_id.trim().to_lowercase();
        self.ranks.iter().find(|r| r.id == lower)
    }

    pub fn default_rank(&self) -> &RankDefinition {
        self.find("player").unwrap_or(&self.ranks[0])
    }

    pub fn donate_ranks(&self) -> Vec<&RankDefinition> {
        let mut list: Vec<&RankDefinition> = self
            .ranks
            .iter()
            .filter(|r| r.category == RankCategory::Donate)
            .collect();
        list.sort_by_key(|r| r.weight);
        list
    }

    pub fn staff_ranks(&self) -> Vec<&RankDefinition> {
        let mut list: Vec<&RankDefinition> = self
            .ranks
            .iter()
            .filter(|r| r.category == RankCategory::Staff)
            .collect();
        list.sort_by_key(|r| r.weight);
        list
    }
}

/// Dual-track player profile: keeps purchased donate rank AND staff role simultaneously.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerRankProfile {
    pub player_key: String,
    pub player_name: String,
    /// Purchased player/donate track rank (e.g., "player", "vip" .. "kronos")
    pub donate_rank: String,
    pub donate_expires_at_ms: Option<u64>,
    /// Optional staff department rank (e.g., "trainee" .. "owner")
    pub staff_rank: Option<String>,
    pub staff_expires_at_ms: Option<u64>,
    pub custom_suffix: Option<String>,
    pub extra_permissions: Vec<String>,
    pub updated_at_ms: u64,
}

impl PlayerRankProfile {
    pub fn new(player_name: &str, now_ms: u64) -> Self {
        Self {
            player_key: player_name.trim().to_lowercase(),
            player_name: player_name.to_string(),
            donate_rank: "player".into(),
            donate_expires_at_ms: None,
            staff_rank: None,
            staff_expires_at_ms: None,
            custom_suffix: None,
            extra_permissions: Vec::new(),
            updated_at_ms: now_ms,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaffApplication {
    pub id: u64,
    pub applicant_key: String,
    pub applicant_name: String,
    pub note: String,
    pub created_at_ms: u64,
    /// "pending" | "accepted" | "rejected"
    pub status: String,
    pub reviewed_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RankAuditEntry {
    pub id: u64,
    pub actor_name: String,
    pub target_name: String,
    pub action: String,
    pub rank_id: String,
    pub duration_label: String,
    pub ts_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RanksStore {
    pub next_app_id: u64,
    pub next_audit_id: u64,
    pub profiles: HashMap<String, PlayerRankProfile>,
    pub applications: Vec<StaffApplication>,
    pub audit_log: Vec<RankAuditEntry>,
    pub id_to_name: HashMap<String, String>,
}

/// Cross-plugin bridge entry written to `data/kronos_ranks_bridge.json`
/// so kronos-essentials, kronos-regions, and kronos-guard can read unified player ranks directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeRankSnapshot {
    pub primary_rank: String,
    pub donate_rank: String,
    pub staff_rank: Option<String>,
    pub department: String,
    pub weight: u32,
    pub prefix: String,
    pub max_homes: u32,
    pub max_regions: u32,
    pub max_region_volume: u64,
}

pub fn parse_duration_seconds(input: &str) -> Result<Option<u64>, ()> {
    let raw = input.trim().to_lowercase();
    if raw.is_empty() {
        return Err(());
    }
    if matches!(raw.as_str(), "perm" | "permanent" | "forever" | "навсегда") {
        return Ok(None);
    }
    if let Some(num) = raw.strip_suffix("mo") {
        let v: u64 = num.parse().map_err(|_| ())?;
        return if v > 0 { Ok(Some(v.saturating_mul(2_592_000))) } else { Err(()) };
    }
    let (num_part, unit) = raw.split_at(raw.len() - 1);
    let value: u64 = num_part.parse().map_err(|_| ())?;
    if value == 0 {
        return Err(());
    }
    let mult: u64 = match unit {
        "s" | "с" => 1,
        "m" | "м" => 60,
        "h" | "ч" => 3600,
        "d" | "д" => 86_400,
        "w" | "н" => 604_800,
        _ => return Err(()),
    };
    Ok(Some(value.saturating_mul(mult)))
}

pub fn format_duration_ru(total_seconds: u64) -> String {
    if total_seconds == 0 {
        return "0с".into();
    }
    let days = total_seconds / 86_400;
    let hours = (total_seconds % 86_400) / 3600;
    let mins = (total_seconds % 3600) / 60;
    let mut parts = Vec::new();
    if days > 0 {
        parts.push(format!("{}д", days));
    }
    if hours > 0 {
        parts.push(format!("{}ч", hours));
    }
    if mins > 0 {
        parts.push(format!("{}м", mins));
    }
    if parts.is_empty() {
        format!("{}с", total_seconds % 60)
    } else {
        parts.join(" ")
    }
}
