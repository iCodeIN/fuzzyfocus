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
    let input = match readline.readline("window > ") {
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

fn main() {
    unsafe { unsafe_main() }
}
