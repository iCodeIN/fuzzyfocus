use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use winapi::{
    shared::{
        minwindef::{BYTE, LPARAM, PBYTE, TRUE},
        windef::HWND,
    },
    um::{
        winnt::LPWSTR,
        winuser::{self, EnumWindows, GetWindowTextW, IsWindowVisible},
    },
};

struct Window {
    pub hwnd: HWND,
    pub name: String,
}

unsafe fn to_string(raw: &[u16]) -> Option<String> {
    let mut len = raw.len();
    for i in 0..raw.len() {
        if raw[i] == 0 {
            len = i;
            break;
        }
    }

    match OsString::from_wide(&raw[..len]).into_string() {
        Ok(s) => Some(s),
        Err(_) => None,
    }
}

unsafe fn get_window_name(hwnd: HWND) -> Option<String> {
    let raw = [0 as u16; 128];
    GetWindowTextW(hwnd, &raw as *const u16 as LPWSTR, raw.len() as i32);
    to_string(&raw[..])
}

unsafe fn list_windows(window_list: &mut Vec<Window>) {
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, list_raw: LPARAM) -> i32 {
        let list = &mut *(list_raw as *mut Vec<Window>);

        let name = match get_window_name(hwnd) {
            Some(s) => s,
            None => return TRUE,
        };

        if !name.is_empty() && IsWindowVisible(hwnd) == TRUE {
            list.push(Window { hwnd, name });
        }

        TRUE
    }

    EnumWindows(
        Some(enum_windows_proc),
        window_list as *mut Vec<Window> as LPARAM,
    );

    window_list.sort_by(|a, b| a.name.cmp(&b.name));
    window_list.dedup_by(|a, b| a.name.eq(&b.name));
}

unsafe fn focus_window(hwnd: HWND) {
    // https://www.codeproject.com/Tips/76427/How-to-bring-window-to-top-with-SetForegroundWindo
    let keystate = [0 as BYTE; 256];
    if winuser::GetKeyboardState(&keystate as *const BYTE as PBYTE) == TRUE {
        if keystate[winuser::VK_MENU as usize] & 0x80 == 0 {
            winuser::keybd_event(
                winuser::VK_MENU as u8,
                0,
                winuser::KEYEVENTF_EXTENDEDKEY | 0,
                0,
            );
        }
    }
    winuser::SetForegroundWindow(hwnd);
    if winuser::GetKeyboardState(&keystate as *const BYTE as PBYTE) == TRUE {
        if keystate[winuser::VK_MENU as usize] & 0x80 == 0 {
            winuser::keybd_event(
                winuser::VK_MENU as u8,
                0,
                winuser::KEYEVENTF_EXTENDEDKEY | winuser::KEYEVENTF_KEYUP,
                0,
            );
        }
    }
}

unsafe fn unsafe_main() {
    let mut window_list = Vec::new();
    list_windows(&mut window_list);

    for (i, window) in window_list.iter().enumerate() {
        println!("{} {}", i, window.name);
    }

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).expect("failed");
    let index = buffer
        .trim()
        .parse::<usize>()
        .expect("could not read number");

    let window = &window_list[index];
    println!("setting forcus for {}", window.name);

    focus_window(window.hwnd);
}

fn main() {
    unsafe { unsafe_main() }
}
