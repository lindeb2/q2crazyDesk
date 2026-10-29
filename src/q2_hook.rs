//! Low-level mouse and keyboard hooks for Beside capture (R-INPUT-1, R-INPUT-2).
//!
//! q2 installs its own hooks instead of using `rdev::grab`, because `rdev`
//! keeps a single process-wide callback that RustDesk's own keyboard grab already
//! owns (docs/DECISIONS.md D-012).
//!
//! On Windows, low-level hooks (`WH_MOUSE_LL` / `WH_KEYBOARD_LL`) are used.
//! On macOS, `CGEventTapCreate` on a dedicated run loop is used.
//!
//! The handler runs on the hook thread inside the system's input path, so it
//! must return quickly. It returns `true` to swallow the event. Events that
//! other programs injected (for example RustDesk's server moving this cursor
//! for a remote controller) are never shown to the handler and never swallowed:
//! only physical input counts (R-DEV-2).

use std::sync::Mutex;
use std::thread;

use q2_router::{Button, Input};

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

    #[cfg(windows)]
    let spawned = thread::Builder::new()
        .name("q2-hook".to_owned())
        .spawn(windows::hook_thread)
        .is_ok();

    #[cfg(target_os = "macos")]
    let spawned = thread::Builder::new()
        .name("q2-hook".to_owned())
        .spawn(macos::hook_thread)
        .is_ok();

    #[cfg(not(any(windows, target_os = "macos")))]
    let spawned = false;

    *started = spawned;
    spawned
}

fn dispatch(raw: Input) -> bool {
    match HANDLER.lock() {
        Ok(mut slot) => slot.as_mut().map(|handler| handler(raw)).unwrap_or(false),
        Err(_) => false,
    }
}

fn button(button: Button, down: bool) -> Input {
    Input::Button { button, down }
}

#[cfg(windows)]
mod windows {
    use std::ptr::null_mut;

    use hbb_common::log;
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

    use super::{button, dispatch};

    pub fn hook_thread() {
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
}

#[cfg(target_os = "macos")]
mod macos {
    use std::os::raw::c_void;

    use core_graphics::event::{
        CGEvent, CGEventFlags, CGEventType, EventField,
    };
    use hbb_common::log;
    use q2_router::{Button, Input};

    use super::dispatch;

    pub fn hook_thread() {
        unsafe {
            let mask = (1 << CGEventType::LeftMouseDown as u64)
                | (1 << CGEventType::LeftMouseUp as u64)
                | (1 << CGEventType::RightMouseDown as u64)
                | (1 << CGEventType::RightMouseUp as u64)
                | (1 << CGEventType::OtherMouseDown as u64)
                | (1 << CGEventType::OtherMouseUp as u64)
                | (1 << CGEventType::MouseMoved as u64)
                | (1 << CGEventType::LeftMouseDragged as u64)
                | (1 << CGEventType::RightMouseDragged as u64)
                | (1 << CGEventType::OtherMouseDragged as u64)
                | (1 << CGEventType::KeyDown as u64)
                | (1 << CGEventType::KeyUp as u64)
                | (1 << CGEventType::FlagsChanged as u64)
                | (1 << CGEventType::ScrollWheel as u64);

            let tap = CGEventTapCreate(
                1, // Session
                0, // HeadInsert
                0, // Default active filter
                mask,
                raw_callback,
                std::ptr::null_mut(),
            );
            if tap.is_null() {
                log::error!("q2 could not create macOS event tap (check Accessibility permissions)");
                return;
            }
            let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
            if source.is_null() {
                log::error!("q2 could not create run loop source for event tap");
                return;
            }
            let current_loop = CFRunLoopGetCurrent();
            CFRunLoopAddSource(current_loop, source, kCFRunLoopCommonModes);
            CGEventTapEnable(tap, true);
            CFRunLoopRun();
        }
    }

    unsafe extern "C" fn raw_callback(
        _proxy: *const c_void,
        event_type: CGEventType,
        cg_event: *mut c_void,
        _user_info: *mut c_void,
    ) -> *mut c_void {
        if cg_event.is_null() {
            return cg_event;
        }
        if (event_type as u32) == 0x0E {
            return cg_event;
        }
        let event = CGEvent::new_with_event(cg_event);
        if let Some(input) = convert(event_type, &event) {
            if dispatch(input) {
                event.set_type(CGEventType::Null);
            }
        }
        cg_event
    }

    fn convert(event_type: CGEventType, event: &CGEvent) -> Option<Input> {
        let loc = event.location();
        let x = loc.x as i32;
        let y = loc.y as i32;

        match event_type {
            CGEventType::MouseMoved
            | CGEventType::LeftMouseDragged
            | CGEventType::RightMouseDragged
            | CGEventType::OtherMouseDragged => Some(Input::Move { x, y }),
            CGEventType::LeftMouseDown => Some(Input::Button { button: Button::Left, down: true }),
            CGEventType::LeftMouseUp => Some(Input::Button { button: Button::Left, down: false }),
            CGEventType::RightMouseDown => Some(Input::Button { button: Button::Right, down: true }),
            CGEventType::RightMouseUp => Some(Input::Button { button: Button::Right, down: false }),
            CGEventType::OtherMouseDown => {
                let num = event.get_integer_value_field(EventField::MouseEventButtonNumber);
                let button = if num == 2 {
                    Button::Middle
                } else if num == 3 {
                    Button::Back
                } else {
                    Button::Forward
                };
                Some(Input::Button { button, down: true })
            }
            CGEventType::OtherMouseUp => {
                let num = event.get_integer_value_field(EventField::MouseEventButtonNumber);
                let button = if num == 2 {
                    Button::Middle
                } else if num == 3 {
                    Button::Back
                } else {
                    Button::Forward
                };
                Some(Input::Button { button, down: false })
            }
            CGEventType::ScrollWheel => {
                let dy = event.get_integer_value_field(EventField::ScrollWheelEventDeltaAxis1) as i32;
                let dx = event.get_integer_value_field(EventField::ScrollWheelEventDeltaAxis2) as i32;
                if dy != 0 {
                    Some(Input::Wheel { vertical: true, delta: dy * 120 })
                } else if dx != 0 {
                    Some(Input::Wheel { vertical: false, delta: dx * 120 })
                } else {
                    None
                }
            }
            CGEventType::KeyDown => {
                let code = event.get_integer_value_field(EventField::KeyboardEventKeycode) as u32;
                Some(Input::Key { vk: code, scan: code, extended: false, down: true })
            }
            CGEventType::KeyUp => {
                let code = event.get_integer_value_field(EventField::KeyboardEventKeycode) as u32;
                Some(Input::Key { vk: code, scan: code, extended: false, down: false })
            }
            CGEventType::FlagsChanged => {
                let code = event.get_integer_value_field(EventField::KeyboardEventKeycode) as u32;
                let flags = event.get_flags();
                let down = match code {
                    0x37 | 0x36 => flags.contains(CGEventFlags::CGEventFlagCommand),
                    0x3B | 0x3E => flags.contains(CGEventFlags::CGEventFlagControl),
                    0x3A | 0x3D => flags.contains(CGEventFlags::CGEventFlagAlternate),
                    0x38 | 0x3C => flags.contains(CGEventFlags::CGEventFlagShift),
                    _ => false,
                };
                Some(Input::Key { vk: code, scan: code, extended: false, down })
            }
            _ => None,
        }
    }

    #[link(name = "CoreGraphics", kind = "framework")]
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        static kCFRunLoopCommonModes: *const c_void;
        fn CGEventTapCreate(
            tap: u32,
            place: u32,
            options: u32,
            events_of_interest: u64,
            callback: unsafe extern "C" fn(*const c_void, CGEventType, *mut c_void, *mut c_void) -> *mut c_void,
            user_info: *mut c_void,
        ) -> *const c_void;
        fn CFMachPortCreateRunLoopSource(
            allocator: *const c_void,
            tap: *const c_void,
            order: i64,
        ) -> *const c_void;
        fn CFRunLoopGetCurrent() -> *const c_void;
        fn CFRunLoopAddSource(rl: *const c_void, source: *const c_void, mode: *const c_void);
        fn CGEventTapEnable(tap: *const c_void, enable: bool);
        fn CFRunLoopRun();
    }
}
