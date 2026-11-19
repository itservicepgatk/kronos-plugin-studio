use crate::models::{ChestMenuDescriptor, Location};
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
    fn host_apply_effect(
        pid_ptr: *const u8,
        pid_len: usize,
        effect_ptr: *const u8,
        effect_len: usize,
        duration_ticks: u32,
        amplifier: u8,
    );
    fn host_open_chest_menu(pid_ptr: *const u8, pid_len: usize, json_ptr: *const u8, json_len: usize);
    fn host_close_menu(pid_ptr: *const u8, pid_len: usize);
    fn host_get_player_location(
        pid_ptr: *const u8,
        pid_len: usize,
        out_ptr: *mut u8,
        out_cap: usize,
    ) -> i32;
    fn host_is_location_safe(
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        y: f64,
        z: f64,
    ) -> i32;
    fn host_find_safe_surface_y(
        world_ptr: *const u8,
        world_len: usize,
        x: f64,
        z: f64,
    ) -> f64;
    fn host_now_ms() -> u64;
}

pub fn send_chat(player_id: &str, message: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_chat(player_id.as_ptr(), player_id.len(), message.as_ptr(), message.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[CHAT -> {}] {}", player_id, message);
}

pub fn send_action_bar(player_id: &str, text: &str) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_send_action_bar(player_id.as_ptr(), player_id.len(), text.as_ptr(), text.len());
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[ACTIONBAR -> {}] {}", player_id, text);
}

pub fn send_title(
    player_id: &str,
    title: &str,
    subtitle: &str,
    fade_in: u32,
    stay: u32,
    fade_out: u32,
) {
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

pub fn teleport_player(player_id: &str, loc: &Location) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_teleport(
            player_id.as_ptr(),
            player_id.len(),
            loc.world.as_ptr(),
            loc.world.len(),
            loc.x,
            loc.y,
            loc.z,
            loc.yaw,
            loc.pitch,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    info!("[TELEPORT -> {}] {}", player_id, loc.coords_label());
}

pub fn spawn_particle(
    world: &str,
    particle: &str,
    x: f64,
    y: f64,
    z: f64,
    count: u32,
    speed: f32,
) {
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
        host_play_sound(
            player_id.as_ptr(),
            player_id.len(),
            sound.as_ptr(),
            sound.len(),
            volume,
            pitch,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, sound, volume, pitch);
}

pub fn apply_effect(player_id: &str, effect: &str, duration_ticks: u32, amplifier: u8) {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        host_apply_effect(
            player_id.as_ptr(),
            player_id.len(),
            effect.as_ptr(),
            effect.len(),
            duration_ticks,
            amplifier,
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (player_id, effect, duration_ticks, amplifier);
}

pub fn open_chest_menu(player_id: &str, descriptor: &ChestMenuDescriptor) {
    if let Ok(json) = serde_json::to_string(descriptor) {
        #[cfg(target_arch = "wasm32")]
        unsafe {
            host_open_chest_menu(
                player_id.as_ptr(),
                player_id.len(),
                json.as_ptr(),
                json.len(),
            );
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

pub fn get_player_location(player_id: &str) -> Location {
    #[cfg(target_arch = "wasm32")]
    {
        let mut buf = [0u8; 512];
        let written = unsafe {
            host_get_player_location(
                player_id.as_ptr(),
                player_id.len(),
                buf.as_mut_ptr(),
                buf.len(),
            )
        };
        if written > 0 {
            if let Ok(loc) = serde_json::from_slice::<Location>(&buf[..written as usize]) {
                return loc;
            }
        }
    }

    Location {
        world: "world".to_string(),
        x: 0.5,
        y: 65.0,
        z: 0.5,
        yaw: 0.0,
        pitch: 0.0,
    }
}

/// Checks if a destination block has solid ground, 2 blocks of breathing room, and no lava/void.
pub fn is_location_safe(loc: &Location) -> bool {
    if loc.y < -60.0 || loc.y > 320.0 {
        return false;
    }
    #[cfg(target_arch = "wasm32")]
    unsafe {
        return host_is_location_safe(loc.world.as_ptr(), loc.world.len(), loc.x, loc.y, loc.z) != 0;
    }
    #[cfg(not(target_arch = "wasm32"))]
    true
}

/// Finds a safe solid surface Y coordinate for /rtp (returns None for ocean/lava/void).
pub fn find_safe_surface_y(world: &str, x: f64, z: f64) -> Option<f64> {
    #[cfg(target_arch = "wasm32")]
    unsafe {
        let y = host_find_safe_surface_y(world.as_ptr(), world.len(), x, z);
        if y >= -60.0 {
            return Some(y);
        }
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = (world, x, z);
        Some(72.0)
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
