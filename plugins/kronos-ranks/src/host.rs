#[cfg(not(test))]
mod ffi {
    #[link(wasm_import_module = "pumpkin_host")]
    extern "C" {
        pub fn host_send_chat(pid_ptr: *const u8, pid_len: usize, msg_ptr: *const u8, msg_len: usize);
        pub fn host_broadcast_chat(msg_ptr: *const u8, msg_len: usize);
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
        pub fn host_set_player_tablist(
            pid_ptr: *const u8,
            pid_len: usize,
            prefix_ptr: *const u8,
            prefix_len: usize,
            suffix_ptr: *const u8,
            suffix_len: usize,
            sort_weight: u32,
        );
        pub fn host_set_tab_header_footer(
            pid_ptr: *const u8,
            pid_len: usize,
            header_ptr: *const u8,
            header_len: usize,
            footer_ptr: *const u8,
            footer_len: usize,
        );
        pub fn host_set_player_rank(
            pid_ptr: *const u8,
            pid_len: usize,
            rank_ptr: *const u8,
            rank_len: usize,
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

pub fn set_player_tablist(player_id: &str, prefix: &str, suffix: &str, sort_weight: u32) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_player_tablist(
            player_id.as_ptr(),
            player_id.len(),
            prefix.as_ptr(),
            prefix.len(),
            suffix.as_ptr(),
            suffix.len(),
            sort_weight,
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, prefix, suffix, sort_weight);
    }
}

pub fn set_tab_header_footer(player_id: &str, header: &str, footer: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_tab_header_footer(
            player_id.as_ptr(),
            player_id.len(),
            header.as_ptr(),
            header.len(),
            footer.as_ptr(),
            footer.len(),
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, header, footer);
    }
}

/// Syncs player's primary rank ID into the Pumpkin 26.3 host runtime
/// so other WASM plugins calling `host_get_player_rank` receive the exact live rank.
pub fn sync_host_rank(player_id: &str, primary_rank: &str) {
    #[cfg(not(test))]
    unsafe {
        ffi::host_set_player_rank(
            player_id.as_ptr(),
            player_id.len(),
            primary_rank.as_ptr(),
            primary_rank.len(),
        );
    }
    #[cfg(test)]
    {
        let _ = (player_id, primary_rank);
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
        let mut buf = vec![0u8; 1024 * 1024 * 2];
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
