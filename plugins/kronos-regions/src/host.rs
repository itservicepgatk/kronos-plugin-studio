use crate::models::{BlockPos, ChestMenuDescriptor};
use log::info;

/// Pumpkin 26.3 WASM Host Imports (`pumpkin_host` namespace).
#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "pumpkin_host")]
extern "C" {
    fn host_send_chat(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
    fn host_send_action_bar(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
    fn host_send_title(
        pid_ptr: *const u8,
        pid_len: usize,
        title_ptr: *const u8,
        title_len: usize,
        sub_ptr: *const u8,
        sub_len: usize,
        fade_in: u32,
        stay: u32,
        fade_out: u32,
    );
    fn host_spawn_particle(
        world_ptr: *const u8,
        world_len: usize,
        particle_ptr: *const u8,
        particle_len: usize,
        x: f64,
        y: f64,
        z: f64,
        count: u32,
        speed: f32,
    );
    fn host_play_sound(
        pid_ptr: *const u8,
        pid_len: usize,
        sound_ptr: *const u8,
        sound_len: usize,
        volume: f32,
        pitch: f32,
    );
    fn host_give_custom_item(
        pid_ptr: *const u8,
        pid_len: usize,
        json_ptr: *const u8,
        json_len: usize,
    );
    fn host_teleport(
        pid_ptr: *const u8,
        pid_len: usize,
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        y: f64,
        z: f64,
        yaw: f32,
        pitch: f32,
    );
    fn host_open_chest_menu(pid_ptr: *const u8, pid_len: usize, json_ptr: *const u8, json_len: usize);
    fn host_close_menu(pid_ptr: *const u8, pid_len: usize);
    fn host_get_player_block_pos(
        pid_ptr: *const u8,
        pid_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn host_now_ms() -> u64;
}

pub fn send_chat(player_id: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_chat(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[CHAT -> {}] {}", player_id, msg);
}

pub fn send_action_bar(player_id: &str, msg: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_action_bar(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, msg);
}

pub fn send_title(player_id: &str, title: &str, subtitle: &str, fade_in: u32, stay: u32, fade_out: u32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_title(
            player_id.as_ptr(),
            player_id.len(),
            title.as_ptr(),
            title.len(),
            subtitle.as_ptr(),
            subtitle.len(),
            fade_in,
            stay,
            fade_out,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TITLE -> {}] {} | {}", player_id, title, subtitle);
}

pub fn spawn_particle(world: &str, particle: &str, x: f64, y: f64, z: f64, count: u32, speed: f32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_spawn_particle(
            world.as_ptr(),
            world.len(),
            particle.as_ptr(),
            particle.len(),
            x,
            y,
            z,
            count,
            speed,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (world, particle, x, y, z, count, speed);
}

pub fn play_sound(player_id: &str, sound: &str, volume: f32, pitch: f32) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_play_sound(player_id.as_ptr(), player_id.len(), sound.as_ptr(), sound.len(), volume, pitch);
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, sound, volume, pitch);
}

pub fn give_claim_wand(player_id: &str, item_id: &str, display_name: &str, tag: &str) {
    let payload = serde_json::json!({
        "item": item_id,
        "name": display_name,
        "tag": tag,
        "enchanted": true,
        "lore": [
            "§7Специальный инструмент привата территории",
            "§8────────────────────────",
            "§a▸ ЛКМ по блоку §8— §fТочка #1",
            "§e▸ ПКМ по блоку §8— §fТочка #2 + 3D-превью",
            "§b▸ Shift+ПКМ §8— §fОткрыть GUI меню (/rg)",
            "§8────────────────────────",
            "§7Создать приват: §e/rg claim <название>"
        ]
    });
    let json = payload.to_string();
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_give_custom_item(player_id.as_ptr(), player_id.len(), json.as_ptr(), json.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[GIVE WAND -> {}] {}", player_id, json);
}

pub fn teleport_player(player_id: &str, world: &str, x: f64, y: f64, z: f64) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_teleport(player_id.as_ptr(), player_id.len(), world.as_ptr(), world.len(), x, y, z, 0.0, 0.0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TP -> {}] {} ({:.1}, {:.1}, {:.1})", player_id, world, x, y, z);
}

pub fn open_chest_menu(player_id: &str, descriptor: &ChestMenuDescriptor) {
    if let Ok(json) = serde_json::to_string(descriptor) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            host_open_chest_menu(player_id.as_ptr(), player_id.len(), json.as_ptr(), json.len());
        }
        #[cfg(not(target_arch = "wasm32"))]
        info!("[MENU -> {}] {} ({} slots)", player_id, descriptor.title, descriptor.slots.len());
    }
}

pub fn close_menu(player_id: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_close_menu(player_id.as_ptr(), player_id.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = player_id;
}

pub fn get_player_block_pos(player_id: &str) -> BlockPos {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 256];
        let n = unsafe { host_get_player_block_pos(player_id.as_ptr(), player_id.len(), buf.as_mut_ptr(), buf.len()) };
        if n > 0 {
            if let Ok(pos) = serde_json::from_slice::<BlockPos>(&buf[..n as usize]) {
                return pos;
            }
        }
    }
    BlockPos {
        world: "world".to_string(),
        x: 0,
        y: 65,
        z: 0,
    }
}

pub fn now_ms() -> u64 {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_now_ms()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0)
    }
}
