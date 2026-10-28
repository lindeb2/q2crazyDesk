//! A visible cursor on a Windows computer with no mouse (R-INPUT-6).
//!
//! Windows draws no cursor when no mouse is attached, so a person who moves
//! this computer's pointer from another computer sees nothing. This draws an
//! arrow in a click-through window that follows the pointer. Mouse Without
//! Borders solves the same problem the same way ("draw mouse cursor").
//!
//! It only runs in the `--server` process, which lives in the signed-in user's
//! session, and only draws while the pointer has moved in the last 30 seconds.
//! The option `q2-draw-cursor` can force it (`Y`, useful with a mouse attached,
//! to try it) or switch it off (`N`).

use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use std::sync::Once;
use std::thread;
use std::time::{Duration, Instant};

use hbb_common::{config::Config, log};
use winapi::{
    shared::{
        minwindef::{LPARAM, LRESULT, UINT, WPARAM},
        windef::{COLORREF, HWND, POINT},
    },
    um::{
        libloaderapi::GetModuleHandleW,
        wingdi::{CreatePen, CreateSolidBrush, DeleteObject, Polygon, SelectObject, PS_SOLID},
        winuser::{
            BeginPaint, CreateWindowExW, DefWindowProcW, DispatchMessageW, EndPaint, GetCursorPos,
            GetSystemMetrics, PeekMessageW, RegisterClassExW, SetLayeredWindowAttributes,
            SetWindowPos, ShowWindow, TranslateMessage, HWND_TOPMOST, LWA_COLORKEY, MSG,
            PAINTSTRUCT, PM_REMOVE, SM_MOUSEPRESENT, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
            WM_PAINT, WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
            WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
        },
    },
};

/// Drawn as transparent by the window, so it never shows.
const KEY: COLORREF = 0x00FF_00FF;
const WHITE: COLORREF = 0x00FF_FFFF;
const BLACK: COLORREF = 0x0000_0000;
const WIDTH: i32 = 16;
const HEIGHT: i32 = 24;
const TICK: Duration = Duration::from_millis(15);
const MOUSE_CHECK: Duration = Duration::from_secs(2);
const IDLE_HIDE: Duration = Duration::from_secs(30);

/// The classic arrow, with its tip at the top left corner of the window.
const ARROW: [(i32, i32); 7] = [(0, 0), (0, 17), (4, 13), (7, 20), (10, 19), (7, 12), (12, 12)];

pub fn start() {
    static START: Once = Once::new();
    START.call_once(|| {
        let server = std::env::args().skip(1).any(|arg| arg == "--server");
        if !server || Config::get_option("q2-draw-cursor") == "N" {
            return;
        }
        let spawned = thread::Builder::new()
            .name("q2-cursor".to_owned())
            .spawn(run);
        if let Err(err) = spawned {
            log::error!("q2 cursor overlay did not start: {err}");
        }
    });
}

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// True when the overlay should be drawn at all: no mouse is attached, or the
/// option forces it.
fn wanted() -> bool {
    match Config::get_option("q2-draw-cursor").as_str() {
        "Y" => true,
        "N" => false,
        _ => unsafe { GetSystemMetrics(SM_MOUSEPRESENT) == 0 },
    }
}

fn run() {
    unsafe {
        let instance = GetModuleHandleW(null_mut());
        let class_name = wide("q2CursorOverlay");
        let mut class: WNDCLASSEXW = zeroed();
        class.cbSize = size_of::<WNDCLASSEXW>() as UINT;
        class.lpfnWndProc = Some(window_proc);
        class.hInstance = instance;
        class.hbrBackground = CreateSolidBrush(KEY);
        class.lpszClassName = class_name.as_ptr();
        if RegisterClassExW(&class) == 0 {
            log::error!("q2 cursor overlay: the window class was refused");
            return;
        }
        let window = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            class_name.as_ptr(),
            null_mut(),
            WS_POPUP,
            0,
            0,
            WIDTH,
            HEIGHT,
            null_mut(),
            null_mut(),
            instance,
            null_mut(),
        );
        if window.is_null() {
            log::error!("q2 cursor overlay: the window was not created");
            return;
        }
        SetLayeredWindowAttributes(window, KEY, 0, LWA_COLORKEY);

        let mut last = POINT { x: -1, y: -1 };
        let mut moved_at: Option<Instant> = None;
        let mut checked_at = Instant::now() - MOUSE_CHECK;
        let mut want = false;
        let mut shown = false;
        loop {
            let mut message: MSG = zeroed();
            while PeekMessageW(&mut message, null_mut(), 0, 0, PM_REMOVE) != 0 {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            if checked_at.elapsed() >= MOUSE_CHECK {
                want = wanted();
                checked_at = Instant::now();
            }
            let mut at = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut at) != 0 && (at.x != last.x || at.y != last.y) {
                last = at;
                moved_at = Some(Instant::now());
            }
            let active = want && moved_at.is_some_and(|t| t.elapsed() < IDLE_HIDE);
            if active {
                SetWindowPos(
                    window,
                    HWND_TOPMOST,
                    last.x,
                    last.y,
                    WIDTH,
                    HEIGHT,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
                shown = true;
            } else if shown {
                ShowWindow(window, SW_HIDE);
                shown = false;
            }
            thread::sleep(TICK);
        }
    }
}

unsafe extern "system" fn window_proc(
    window: HWND,
    message: UINT,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message == WM_PAINT {
        paint(window);
        return 0;
    }
    DefWindowProcW(window, message, wparam, lparam)
}

unsafe fn paint(window: HWND) {
    let mut paint: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(window, &mut paint);
    if dc.is_null() {
        return;
    }
    let fill = CreateSolidBrush(WHITE);
    let outline = CreatePen(PS_SOLID as i32, 1, BLACK);
    let old_brush = SelectObject(dc, fill as _);
    let old_pen = SelectObject(dc, outline as _);
    let points: Vec<POINT> = ARROW.iter().map(|&(x, y)| POINT { x, y }).collect();
    Polygon(dc, points.as_ptr(), points.len() as i32);
    SelectObject(dc, old_brush);
    SelectObject(dc, old_pen);
    DeleteObject(fill as _);
    DeleteObject(outline as _);
    EndPaint(window, &paint);
}
