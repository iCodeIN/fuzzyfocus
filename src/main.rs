use std::{
    ffi::OsString,
    mem::{size_of, zeroed},
    os::windows::ffi::OsStringExt,
};

use winapi::{
    shared::{minwindef::*, windef::*},
    um::{libloaderapi::*, wincon::*, wingdi::*, winnt::*, winuser::*},
};

use rustyline::{completion::Completer, Context, Editor, Helper, Result};
use rustyline_derive::{Highlighter, Hinter, Validator};

use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

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
    GetWindowTextW(hwnd, raw.as_ptr() as LPWSTR, raw.len() as i32);
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

    EnumWindows(Some(enum_windows_proc), window_list.as_mut_ptr() as LPARAM);

    window_list.sort_by(|a, b| a.name.cmp(&b.name));
    window_list.dedup_by(|a, b| a.name.eq(&b.name));
}

unsafe fn focus_window(hwnd: HWND) {
    // https://www.codeproject.com/Tips/76427/How-to-bring-window-to-top-with-SetForegroundWindo
    let keystate = [0 as BYTE; 256];
    if GetKeyboardState(keystate.as_ptr() as PBYTE) == TRUE {
        if keystate[VK_MENU as usize] & 0x80 == 0 {
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_EXTENDEDKEY | 0, 0);
        }
    }
    SetForegroundWindow(hwnd);
    if GetKeyboardState(keystate.as_ptr() as PBYTE) == TRUE {
        if keystate[VK_MENU as usize] & 0x80 == 0 {
            keybd_event(VK_MENU as u8, 0, KEYEVENTF_EXTENDEDKEY | KEYEVENTF_KEYUP, 0);
        }
    }
}

#[derive(Hinter, Highlighter, Validator)]
struct ReadlineHelper {
    pub window_names: Vec<String>,
    pub fuzzy_matcher: SkimMatcherV2,
}

impl ReadlineHelper {
    pub fn get_matches(&self, input: &str) -> Vec<String> {
        let mut matches = Vec::new();
        for name in &self.window_names {
            let name = &name[..];
            if let Some(score) = self.fuzzy_matcher.fuzzy_match(name, input) {
                matches.push((score, name));
            }
        }

        matches.sort_by_key(|m| -m.0);
        matches.iter().map(|m| m.1.to_owned()).collect()
    }
}

impl Helper for ReadlineHelper {}

impl Completer for ReadlineHelper {
    type Candidate = String;
    fn complete(
        &self,
        line: &str,
        _pos: usize,
        _ctx: &Context,
    ) -> Result<(usize, Vec<Self::Candidate>)> {
        Ok((0, self.get_matches(line)))
    }
}

unsafe fn unsafe_main() {
    let mut window_list = Vec::new();
    list_windows(&mut window_list);
    let window_names: Vec<_> = window_list.iter().map(|w| w.name.clone()).collect();

    let mut readline = Editor::new();
    let readline_helper = ReadlineHelper {
        window_names,
        fuzzy_matcher: SkimMatcherV2::default(),
    };
    readline.set_helper(Some(readline_helper));
    let input = match readline.readline("") {
        Ok(line) => line,
        Err(_) => return,
    };

    let helper = readline.helper().unwrap();
    let matches = helper.get_matches(&input[..]);
    let window_name = match matches.first() {
        Some(window_name) => window_name,
        None => return,
    };

    for window in &window_list {
        if &window.name[..] == window_name {
            focus_window(window.hwnd);
            break;
        }
    }
}

unsafe fn run_daemon() {
    unsafe extern "system" fn keyboard_hook_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        let mut consume_key = false;
        if n_code == HC_ACTION {
            match w_param as u32 {
                WM_KEYDOWN | WM_SYSKEYDOWN | WM_KEYUP | WM_SYSKEYUP => {
                    let p = &*(l_param as PKBDLLHOOKSTRUCT);
                    let alt_down = (p.flags & LLKHF_ALTDOWN) != 0;
                    let pressed_escape = p.vkCode as i32 == VK_ESCAPE;
                    consume_key = alt_down && pressed_escape;
                    //let pressing_win_key = GetKeyState(VK_LWIN) != 0;
                    //let pressed_space = p.vkCode as i32 == VK_SPACE;
                    //consume_key = pressing_win_key && pressed_space;

                    if consume_key {
                        match w_param as u32 {
                            WM_KEYUP | WM_SYSKEYUP => {
                                show_dialog();
                            }
                            _ => (),
                        }
                    }
                }
                _ => (),
            }
        }

        if consume_key {
            1
        } else {
            CallNextHookEx(0 as HHOOK, n_code, w_param, l_param)
        }
    }

    FreeConsole();
    let h_instance = GetModuleHandleA(0 as LPCSTR);
    let hook = SetWindowsHookExA(WH_KEYBOARD_LL, Some(keyboard_hook_proc), h_instance, 0);

    let mut msg: MSG = zeroed();
    while GetMessageA(&mut msg, 0 as HWND, 0, 0) > 0 {
        TranslateMessage(&msg);
        DispatchMessageW(&msg);
    }

    UnhookWindowsHookEx(hook);
}

unsafe fn show_dialog() {
    let text = std::ffi::CString::new("click to reenable").unwrap();
    let caption = std::ffi::CString::new("disable low level keys").unwrap();
    MessageBoxA(
        0 as HWND,
        text.as_ptr() as LPCSTR,
        caption.as_ptr() as LPCSTR,
        MB_OK,
    );
}

fn main() {
    //unsafe { unsafe_main() }
    unsafe { run_daemon() }
}
