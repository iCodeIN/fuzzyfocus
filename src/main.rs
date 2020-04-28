use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use winapi::{
    shared::{
        minwindef::{BYTE, FALSE, LPARAM, PBYTE, TRUE},
        ntdef::NULL,
        windef::HWND,
    },
    um::{
        processthreadsapi,
        winnt::{LPWSTR, PVOID},
        winuser::{self, EnumWindows, GetWindowTextW, IsWindowVisible, SetWindowPos},
    },
};

struct Window {
    pub hwnd: HWND,
    pub name: String,
}

#[derive(Default)]
struct State {
    pub windows: Vec<Window>,
}

fn main() {
    unsafe { run() }
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

unsafe fn run() {
    unsafe extern "system" fn enum_windows_proc(hwnd: HWND, state_raw: LPARAM) -> i32 {
        let state = &mut *(state_raw as *mut State);

        let name = match get_window_name(hwnd) {
            Some(s) => s,
            None => return TRUE,
        };

        if !name.is_empty() && IsWindowVisible(hwnd) == TRUE {
            state.windows.push(Window { hwnd, name });
        }

        TRUE
    }

    let mut state = State::default();
    EnumWindows(Some(enum_windows_proc), &mut state as *mut State as LPARAM);

    for (i, window) in state.windows.iter().enumerate() {
        println!("{} {}", i, window.name);
    }

    let mut buffer = String::new();
    std::io::stdin().read_line(&mut buffer).expect("failed");
    let index = buffer
        .trim()
        .parse::<usize>()
        .expect("could not read number");

    let window = &state.windows[index];
    println!("setting forcus for {}", window.name);

    let keystate = [0 as BYTE; 256];
    if winuser::GetKeyboardState(&keystate as *const BYTE as PBYTE) == TRUE {
        if keystate[winuser::VK_MENU as usize] & 0x80 == 0 {
            winuser::keybd_event(winuser::VK_MENU as u8, 0, winuser::KEYEVENTF_EXTENDEDKEY | 0, 0);
        }
    }
    if winuser::GetKeyboardState(&keystate as *const BYTE as PBYTE) == TRUE {
        if keystate[winuser::VK_MENU as usize] & 0x80 == 0 {
            winuser::keybd_event(winuser::VK_MENU as u8, 0, winuser::KEYEVENTF_EXTENDEDKEY | winuser::KEYEVENTF_KEYUP, 0);
        }
    }
    winuser::SetForegroundWindow(window.hwnd);

    /*
    let local_process_id = processthreadsapi::GetCurrentThreadId();
    let window_process_id = winuser::GetWindowThreadProcessId(window.hwnd, 0 as *mut u32);

    dbg!(winuser::AttachThreadInput(
        local_process_id,
        window_process_id,
        TRUE
    ));

    let mut lock_timeout = 0;
    winuser::SystemParametersInfoW(
        winuser::SPI_GETFOREGROUNDLOCKTIMEOUT,
        0,
        &mut lock_timeout as *mut i32 as PVOID,
        0
    );
    winuser::SystemParametersInfoW(
        winuser::SPI_SETFOREGROUNDLOCKTIMEOUT,
        0,
        NULL,
        winuser::SPIF_UPDATEINIFILE | winuser::SPIF_SENDWININICHANGE,
    );

    dbg!(winuser::AllowSetForegroundWindow(winuser::ASFW_ANY));
    dbg!(winuser::SetForegroundWindow(window.hwnd));

    winuser::SystemParametersInfoW(
        winuser::SPI_SETFOREGROUNDLOCKTIMEOUT,
        0,
        lock_timeout as PVOID,
        winuser::SPIF_UPDATEINIFILE | winuser::SPIF_SENDWININICHANGE,
    );
    dbg!(winuser::AttachThreadInput(
        local_process_id,
        window_process_id,
        FALSE
    ));
    */

    /*
    SetWindowPos(
        window.hwnd,
        winuser::HWND_TOP,
        0,
        0,
        0,
        0,
        winuser::SWP_ASYNCWINDOWPOS
            | winuser::SWP_SHOWWINDOW
            | winuser::SWP_NOSIZE
            | winuser::SWP_NOMOVE
            | winuser::SWP_NOACTIVATE,
    );
    */

    //dbg!(winuser::SetFocus(window.hwnd));

    /*
    winuser::SetFocus(window.hwnd);
    winuser::SetForegroundWindow(window.hwnd);
    winuser::BringWindowToTop(window.hwnd);
    winuser::SetActiveWindow(window.hwnd);
    */
}
