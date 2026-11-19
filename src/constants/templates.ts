import { PluginTemplate } from '../types';
import { KRONOS_ESSENTIALS_TEMPLATE } from './kronosEssentialsTemplate';

export const STARTER_TEMPLATES: PluginTemplate[] = [
  KRONOS_ESSENTIALS_TEMPLATE,
  {
    id: 'blank-plugin',
    title: 'Minimal Starter Plugin',
    description: 'Clean boilerplate with lifecycle hooks, logging, and command handler ready for custom logic.',
    tags: ['Basic', 'Boilerplate', 'Rust WASM'],
    files: {
      'Cargo.toml': `[package]
name = "custom-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
`,
      'src/lib.rs': `use log::{info, warn};

#[no_mangle]
pub extern "C" fn init() -> i32 {
    info!("Custom Plugin v0.1.0 initialized successfully!");
    0
}

#[no_mangle]
pub extern "C" fn on_player_join(player_id_ptr: *const u8, player_id_len: usize) {
    // Handling player join
    info!("Player joined event received");
}

#[no_mangle]
pub extern "C" fn cleanup() {
    info!("Custom Plugin unloaded");
}
`,
      'README.md': `# Custom Plugin for KRONOS / Pumpkin

A Rust WASM plugin compiled for target \`wasm32-wasip1\`.

### Build instructions:
\`\`\`bash
cargo build --target wasm32-wasip1 --release
\`\`\`
Drop target \`.wasm\` file into \`/home/unclebug/pumpkin-server/plugins/\`.
`
    }
  },
  {
    id: 'auth-plugin',
    title: 'Auth & Security Plugin',
    description: 'Secure authentication system with /register, /login, session timeouts, and movement lockdown before login.',
    tags: ['Security', 'Auth', 'Commands'],
    files: {
      'Cargo.toml': `[package]
name = "auth-plugin"
version = "0.2.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
strip = true
`,
      'src/lib.rs': `use log::{info, warn};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct UserAccount {
    pub username: String,
    pub password_hash: String,
    pub registered_at: u64,
}

static AUTH_STATE: Mutex<Option<AuthState>> = Mutex::new(None);

pub struct AuthState {
    pub users: HashMap<String, UserAccount>,
    pub logged_in: HashSet<String>,
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    let mut state = AUTH_STATE.lock().unwrap();
    *state = Some(AuthState {
        users: HashMap::new(),
        logged_in: HashSet::new(),
    });
    info!("Auth & Security Plugin v0.2.0 loaded!");
    0
}

#[no_mangle]
pub extern "C" fn on_player_join(player_name: &str) {
    let mut state = AUTH_STATE.lock().unwrap();
    if let Some(ref mut s) = *state {
        s.logged_in.remove(player_name);
        info!("Player {} needs to authenticate", player_name);
    }
}

#[no_mangle]
pub extern "C" fn on_player_command(player_name: &str, command: &str) -> bool {
    let parts: Vec<&str> = command.split_whitespace().collect();
    if parts.is_empty() {
        return false;
    }

    match parts[0] {
        "/register" => {
            if parts.len() < 2 {
                info!("Usage: /register <password>");
                return true;
            }
            let mut state = AUTH_STATE.lock().unwrap();
            if let Some(ref mut s) = *state {
                s.users.insert(player_name.to_string(), UserAccount {
                    username: player_name.to_string(),
                    password_hash: parts[1].to_string(),
                    registered_at: 0,
                });
                s.logged_in.insert(player_name.to_string());
                info!("Player {} successfully registered and logged in", player_name);
            }
            true
        }
        "/login" => {
            if parts.len() < 2 {
                info!("Usage: /login <password>");
                return true;
            }
            let mut state = AUTH_STATE.lock().unwrap();
            if let Some(ref mut s) = *state {
                if let Some(user) = s.users.get(player_name) {
                    if user.password_hash == parts[1] {
                        s.logged_in.insert(player_name.to_string());
                        info!("Player {} logged in successfully", player_name);
                    } else {
                        warn!("Player {} entered wrong password", player_name);
                    }
                }
            }
            true
        }
        _ => false
    }
}
`,
      'README.md': `# Auth & Security Plugin

Provides player authentication with:
- \`/register <password>\`
- \`/login <password>\`
- Movement prevention before login
`
    }
  },
  {
    id: 'tab-plugin',
    title: 'Tab & Custom Ranks Plugin',
    description: 'Header, footer, prefix, suffix, and permissions-based colored ranks in player tablist.',
    tags: ['Tablist', 'Ranks', 'UI'],
    files: {
      'Cargo.toml': `[package]
name = "tab-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
log = "0.4"
`,
      'src/lib.rs': `use log::info;
use std::collections::HashMap;

pub struct RankInfo {
    pub prefix: &'static str,
    pub priority: i32,
}

pub fn get_rank(role: &str) -> RankInfo {
    match role {
        "owner" => RankInfo { prefix: "§4§l[OWNER] ", priority: 100 },
        "admin" => RankInfo { prefix: "§c§l[ADMIN] ", priority: 80 },
        "vip" => RankInfo { prefix: "§6§l[VIP] ", priority: 50 },
        _ => RankInfo { prefix: "§7[PLAYER] ", priority: 10 },
    }
}

#[no_mangle]
pub extern "C" fn init() -> i32 {
    info!("KRONOS Tab & Ranks Plugin v0.1.0 loaded");
    0
}
`,
      'data/ranks.json': `{
  "unclebug": "owner"
}`
    }
  }
];
