//! Low-level mouse and keyboard hooks for Beside capture (R-INPUT-1, R-INPUT-2).
//!
//! q2 installs its own `WH_MOUSE_LL` and `WH_KEYBOARD_LL` hooks instead of using
//! `rdev::grab`, because `rdev` keeps a single process-wide callback that
//! RustDesk's own keyboard grab already owns (docs/DECISIONS.md D-012).
//!
//! The handler runs on the hook thread inside the system's input path, so it
//! must return quickly. It returns `true` to swallow the event. Events that
//! other programs injected (for example RustDesk's server moving this cursor
//! for a remote controller) are never shown to the handler and never swallowed:
//! only physical input counts (R-DEV-2).

use std::ptr::null_mut;
use std::sync::Mutex;
use std::thread;

use q2_router::{Button, Input};

use winapi::{
    shared::minwindef::{LPARAM, LRESULT, WPARAM},
    um::winuser::{
        CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
        HC_ACTION, KBDLLHOOKSTRUCT, LLKHF_EXTENDED, LLKHF_INJECTED, LLMHF_INJECTED, MSG,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN,
        WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_MOUSEHWHEEL, WM_MOUSEMOVE, WM_MOUSEWHEEL,
        WM_RBUTTONDOWN, WM_RBUTTONUP, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_XBUTTONDOWN, WM_XBUTTONUP,
    },
};

pub type Handler = Box<dyn FnMut(Input) -> bool + Send>;

static HANDLER: Mutex<Option<Handler>> = Mutex::new(None);
static STARTED: Mutex<bool> = Mutex::new(false);

/// Set the handler and start the hook thread the first time.
pub fn install(handler: Handler) -> bool {
    if let Ok(mut slot) = HANDLER.lock() {
        *slot = Some(handler);
    } else {
        return false;
    }
    let Ok(mut started) = STARTED.lock() else {
        return false;
    };
    if *started {
        return true;
    }
    let spawned = thread::Builder::new()
        .name("q2-hook".to_owned())
        .spawn(hook_thread)
        .is_ok();
    *started = spawned;
    spawned
}

fn hook_thread() {
    unsafe {
        let mouse = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_proc), null_mut(), 0);
        let keyboard = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), null_mut(), 0);
        if mouse.is_null() || keyboard.is_null() {
            log::error!("q2 could not install the input hooks");
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn dispatch(raw: Input) -> bool {
    match HANDLER.lock() {
        Ok(mut slot) => slot.as_mut().map(|handler| handler(raw)).unwrap_or(false),
        Err(_) => false,
    }
}

unsafe extern "system" fn mouse_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION {
        let info = &*(lparam as *const MSLLHOOKSTRUCT);
        if info.flags & LLMHF_INJECTED == 0 {
            if let Some(raw) = mouse_event(wparam as u32, info) {
                if dispatch(raw) {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

fn mouse_event(message: u32, info: &MSLLHOOKSTRUCT) -> Option<Input> {
    let high = (info.mouseData >> 16) as u16;
    let wheel = high as i16 as i32;
    let side = if high == 2 {
        Button::Forward
    } else {
        Button::Back
    };
    Some(match message {
        WM_MOUSEMOVE => Input::Move {
            x: info.pt.x,
            y: info.pt.y,
        },
        WM_LBUTTONDOWN => button(Button::Left, true),
        WM_LBUTTONUP => button(Button::Left, false),
        WM_RBUTTONDOWN => button(Button::Right, true),
        WM_RBUTTONUP => button(Button::Right, false),
        WM_MBUTTONDOWN => button(Button::Middle, true),
        WM_MBUTTONUP => button(Button::Middle, false),
        WM_XBUTTONDOWN => button(side, true),
        WM_XBUTTONUP => button(side, false),
        WM_MOUSEWHEEL => Input::Wheel {
            vertical: true,
            delta: wheel,
        },
        WM_MOUSEHWHEEL => Input::Wheel {
            vertical: false,
            delta: wheel,
        },
        _ => return None,
    })
}

fn button(button: Button, down: bool) -> Input {
    Input::Button { button, down }
}

unsafe extern "system" fn keyboard_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION {
        let info = &*(lparam as *const KBDLLHOOKSTRUCT);
        if info.flags & LLKHF_INJECTED == 0 {
            let message = wparam as u32;
            let down = message == WM_KEYDOWN || message == WM_SYSKEYDOWN;
            let up = message == WM_KEYUP || message == WM_SYSKEYUP;
            if down || up {
                let raw = Input::Key {
                    vk: info.vkCode,
                    scan: info.scanCode,
                    extended: info.flags & LLKHF_EXTENDED != 0,
                    down,
                };
                if dispatch(raw) {
                    return 1;
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}
