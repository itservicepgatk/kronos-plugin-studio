use crate::models::PlayerPos;

#[cfg(not(test))]
mod ffi {
    #[link(wasm_import_module = "pumpkin_host")]
    extern "C" {
        pub fn host_send_chat(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
        pub fn host_broadcast_chat(msg_ptr: *const u8, msg_len: usize);
        pub fn host_send_action_bar(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
        pub fn host_send_title(
            pid_ptr: *const u8,
            pid_len: usize,
            title_ptr: *const u8,
            title_len: usize,
            sub_ptr: *const u8,
            sub_len: usize,
            fade_in: i32,
            stay: i32,
            fade_out: i32,
        );
        pub fn host_play_sound(
            pid_ptr: *const u8,
            pid_len: usize,
            sound_ptr: *const u8,
            sound_len: usize,
            volume: f32,
            pitch: f32,
        );
        pub fn host_kick_player(pid_ptr: *const u8, pid_len: usize, reason_ptr: *const u8, reason_len: usize);
        pub fn host_set_vanished(pid_ptr: *const u8, pid_len: usize, vanished: i32, viewer_min_weight: u32);
        pub fn host_set_frozen(pid_ptr: *const u8, pid_len: usize, frozen: i32);
        pub fn host_give_item(
            pid_ptr: *const u8,
            pid_len: usize,
            item_ptr: *const u8,
            item_len: usize,
            name_ptr: *const u8,
            name_len: usize,
            lore_ptr: *const u8,
            lore_len: usize,
        );
        pub fn host_open_chest_menu(
            pid_ptr: *const u8,
            pid_len: usize,
            menu_id_ptr: *const u8,
            menu_id_len: usize,
            title_ptr: *const u8,
            title_len: usize,
            rows: u8,
            items_json_ptr: *const u8,
            items_json_len: usize,
        );
        pub fn host_get_player_rank(
            pid_ptr: *const u8,
            pid_len: usize,
            out_ptr: *mut u8,
            out_cap: usize,
        ) -> usize;
        pub fn host_get_player_pos(
            pid_ptr: *const u8,
            pid_len: usize,
            out_ptr: *mut u8,
            out_cap: usize,
        ) -> usize;
        pub fn host_set_block(
            world_ptr: *const u8,
            world_len: usize,
            x: i32,
            y: i32,
            z: i32,
            block_ptr: *const u8,
            block_len: usize,
        ) -> i32;
        pub fn host_now_ms() -> u64;
        pub fn host_read_file(
            path_ptr: *const u8,
            path_len: usize,
            out_ptr: *mut u8,
            out_cap: usize,
        ) -> i32;
        pub fn host_write_file(
            path_ptr: *const u8,
            path_len: usize,
            data_ptr: *const u8,
            data_len: usize,
        ) -> i32;
        pub fn host_rename_file(
            from_ptr: *const u8,
            from_len: usize,
            to_ptr: *const u8,
            to_len: usize,
        ) -> i32;
    }
}

pub fn send_chat(player_id: &str, msg: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_send_chat(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(test)]
    {
        let _ = (player_id, msg);
    }
}

pub fn broadcast_chat(msg: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_broadcast_chat(msg.as_ptr(), msg.len());
    }
    #[cfg(test)]
    {
        let _ = msg;
    }
}

pub fn send_action_bar(player_id: &str, msg: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_send_action_bar(player_id.as_ptr(), player_id.len(), msg.as_ptr(), msg.len());
    }
    #[cfg(test)]
    {
        let _ = (player_id, msg);
    }
}

pub fn send_title(player_id: &str, title: &str, sub: &str, fade_in: i32, stay: i32, fade_out: i32) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_send_title(
            player_id.as_ptr(),
            player_id.len(),
            title.as_ptr(),
            title.len(),
            sub.as_ptr(),
            sub.len(),
            fade_in,
            stay,
            fade_out,
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, title, sub, fade_in, stay, fade_out);
    }
}

pub fn play_sound(player_id: &str, sound: &str, volume: f32, pitch: f32) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_play_sound(
            player_id.as_ptr(),
            player_id.len(),
            sound.as_ptr(),
            sound.len(),
            volume,
            pitch,
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, sound, volume, pitch);
    }
}

pub fn kick_player(player_id: &str, reason: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_kick_player(player_id.as_ptr(), player_id.len(), reason.as_ptr(), reason.len());
    }
    #[cfg(test)]
    {
        let _ = (player_id, reason);
    }
}

pub fn set_vanished(player_id: &str, vanished: bool, viewer_min_weight: u32) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_vanished(
            player_id.as_ptr(),
            player_id.len(),
            if vanished { 1 } else { 0 },
            viewer_min_weight,
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, vanished, viewer_min_weight);
    }
}

pub fn set_frozen(player_id: &str, frozen: bool) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_frozen(player_id.as_ptr(), player_id.len(), if frozen { 1 } else { 0 });
    }
    #[cfg(test)]
    {
        let _ = (player_id, frozen);
    }
}

pub fn give_item(player_id: &str, item: &str, display_name: &str, lore: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_give_item(
            player_id.as_ptr(),
            player_id.len(),
            item.as_ptr(),
            item.len(),
            display_name.as_ptr(),
            display_name.len(),
            lore.as_ptr(),
            lore.len(),
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, item, display_name, lore);
    }
}

pub fn open_chest_menu(player_id: &str, menu_id: &str, title: &str, rows: u8, items_json: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_open_chest_menu(
            player_id.as_ptr(),
            player_id.len(),
            menu_id.as_ptr(),
            menu_id.len(),
            title.as_ptr(),
            title.len(),
            rows,
            items_json.as_ptr(),
            items_json.len(),
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, menu_id, title, rows, items_json);
    }
}

pub fn get_player_rank(player_id: &str) -> String {
    #[cfg(not(test))]
    unsafe {
        let mut buf = vec![0u8; 64];
        let written = ffi::host_get_player_rank(player_id.as_ptr(), player_id.len(), buf.as_mut_ptr(), buf.len());
        if written > 0 && written <= buf.len() {
            return String::from_utf8_lossy(&buf[..written]).to_string();
        }
        "player".into()
    }
    #[cfg(test)]
    {
        match player_id {
            "owner" => "owner".into(),
            "admin" => "admin".into(),
            "stmoder" => "stmoder".into(),
            "moder" => "moder".into(),
            "helper" => "helper".into(),
            _ => "player".into(),
        }
    }
}

pub fn get_player_pos(player_id: &str) -> PlayerPos {
    #[cfg(not(test))]
    unsafe {
        let mut buf = vec![0u8; 128];
        let written = ffi::host_get_player_pos(player_id.as_ptr(), player_id.len(), buf.as_mut_ptr(), buf.len());
        if written > 0 && written <= buf.len() {
            let text = String::from_utf8_lossy(&buf[..written]);
            let parts: Vec<&str> = text.split(',').collect();
            if parts.len() == 4 {
                return PlayerPos {
                    world: parts[0].to_string(),
                    x: parts[1].parse().unwrap_or(0),
                    y: parts[2].parse().unwrap_or(64),
                    z: parts[3].parse().unwrap_or(0),
                };
            }
        }
        PlayerPos { world: "world".into(), x: 0, y: 64, z: 0 }
    }
    #[cfg(test)]
    {
        let _ = player_id;
        PlayerPos { world: "world".into(), x: 10, y: 64, z: 10 }
    }
}

pub fn set_block(world: &str, x: i32, y: i32, z: i32, block_id: &str) -> bool {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_block(world.as_ptr(), world.len(), x, y, z, block_id.as_ptr(), block_id.len()) == 0
    }
    #[cfg(test)]
    {
        let _ = (world, x, y, z, block_id);
        true
    }
}

pub fn now_ms() -> u64 {
    #[cfg(not(test))]
    unsafe {
        ffi::host_now_ms()
    }
    #[cfg(test)]
    {
        1_750_000_000_000
    }
}

pub fn read_file(path: &str) -> Option<String> {
    #[cfg(not(test))]
    unsafe {
        let mut buf = vec![0u8; 1024 * 1024 * 4];
        let res = ffi::host_read_file(path.as_ptr(), path.len(), buf.as_mut_ptr(), buf.len());
        if res > 0 {
            let len = (res as usize).min(buf.len());
            return String::from_utf8(buf[..len].to_vec()).ok();
        }
        None
    }
    #[cfg(test)]
    {
        let _ = path;
        None
    }
}

pub fn write_file_atomic(path: &str, content: &str) -> bool {
    #[cfg(not(test))]
    unsafe {
        let tmp = format!("{}.tmp", path);
        let ok = ffi::host_write_file(tmp.as_ptr(), tmp.len(), content.as_ptr(), content.len()) == 0;
        if !ok {
            return false;
        }
        ffi::host_rename_file(tmp.as_ptr(), tmp.len(), path.as_ptr(), path.len()) == 0
    }
    #[cfg(test)]
    {
        let _ = (path, content);
        true
    }
}
