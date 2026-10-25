//! Local millimetre crossing on this computer's own monitors (R-LAYOUT-4).
//!
//! The main window process watches the cursor. When it enters another monitor,
//! the landing point is moved so a physical height is preserved. The service,
//! the tray, and an incoming session's helper do not run this.

use std::thread;
use std::time::Duration;

pub fn start() {
    #[cfg(windows)]
    start_windows();
}

#[cfg(windows)]
fn start_windows() {
    if !is_main_window() {
        return;
    }
    thread::Builder::new()
        .name("q2-local-cross".to_owned())
        .spawn(watch_cursor)
        .ok();
}

#[cfg(windows)]
fn is_main_window() -> bool {
    !std::env::args().any(|arg| {
        matches!(
            arg.as_str(),
            "--service" | "--server" | "--tray" | "--cm" | "--install" | "--uninstall"
        )
    })
}

#[cfg(windows)]
fn watch_cursor() {
    let mut panels: Vec<q2_layout::OsPanel> = Vec::new();
    let mut canvas = q2_layout::Canvas::new();
    let mut prev: Option<(u32, i32, i32)> = None;
    let mut ticks = 0u32;
    loop {
        thread::sleep(Duration::from_millis(8));
        ticks = ticks.wrapping_add(1);
        if panels.is_empty() || ticks % 250 == 0 {
            let next = read_panels();
            if next != panels {
                canvas = q2_layout::canvas_from_os_panels("local", &next);
                panels = next;
            }
        }
        if panels.len() < 2 {
            prev = None;
            continue;
        }
        let Some((x, y)) = crate::platform::windows::get_cursor_pos() else {
            continue;
        };
        let Some(index) = hit_panel(&panels, x, y) else {
            continue;
        };
        let Some((from_index, from_x, from_y)) = prev else {
            prev = Some((index, x, y));
            continue;
        };
        prev = Some((index, x, y));
        if from_index == index {
            continue;
        }
        let Some((nx, ny)) = q2_layout::crossing_correction(
            &canvas,
            &panels,
            from_index,
            (from_x, from_y),
            index,
            (x, y),
        ) else {
            continue;
        };
        if crate::platform::windows::set_cursor_pos(nx, ny) {
            prev = Some((index, nx, ny));
        }
    }
}

#[cfg(windows)]
fn hit_panel(panels: &[q2_layout::OsPanel], x: i32, y: i32) -> Option<u32> {
    panels.iter().find_map(|panel| {
        let right = panel.x.saturating_add(panel.width as i32);
        let bottom = panel.y.saturating_add(panel.height as i32);
        if x >= panel.x && x < right && y >= panel.y && y < bottom {
            Some(panel.index)
        } else {
            None
        }
    })
}

#[cfg(windows)]
fn read_panels() -> Vec<q2_layout::OsPanel> {
    use std::mem::size_of;
    use winapi::{
        shared::{
            minwindef::{BOOL, DWORD, FALSE, LPARAM, TRUE},
            ntdef::WCHAR,
            windef::{HDC, HMONITOR, LPRECT, RECT},
        },
        um::{
            wingdi::{CreateDCW, DeleteDC, GetDeviceCaps, HORZSIZE, LOGPIXELSX, VERTSIZE},
            winuser::{
                EnumDisplayMonitors, GetMonitorInfoW, CCHDEVICENAME, LPMONITORINFO, MONITORINFOEXW,
            },
        },
    };

    struct Found {
        rect: RECT,
        device: [WCHAR; CCHDEVICENAME],
    }

    unsafe extern "system" fn each(
        hmon: HMONITOR,
        _hdc: HDC,
        _rect: LPRECT,
        data: LPARAM,
    ) -> BOOL {
        let found = &mut *(data as *mut Vec<Found>);
        let mut info: MONITORINFOEXW = unsafe { std::mem::zeroed() };
        info.cbSize = size_of::<MONITORINFOEXW>() as DWORD;
        if GetMonitorInfoW(hmon, &mut info as *mut MONITORINFOEXW as LPMONITORINFO) == FALSE {
            return TRUE;
        }
        found.push(Found {
            rect: info.rcMonitor,
            device: info.szDevice,
        });
        TRUE
    }

    let mut found: Vec<Found> = Vec::new();
    unsafe {
        EnumDisplayMonitors(
            std::ptr::null_mut(),
            std::ptr::null(),
            Some(each),
            &mut found as *mut Vec<Found> as LPARAM,
        );
    }
    found
        .into_iter()
        .enumerate()
        .map(|(i, item)| {
            let width = (item.rect.right - item.rect.left).max(1) as u32;
            let height = (item.rect.bottom - item.rect.top).max(1) as u32;
            let (mut width_mm, mut height_mm, mut scale) = fallback_mm(width, height);
            unsafe {
                let dc = CreateDCW(std::ptr::null(), item.device.as_ptr(), std::ptr::null(), std::ptr::null());
                if !dc.is_null() {
                    let w = GetDeviceCaps(dc, HORZSIZE);
                    let h = GetDeviceCaps(dc, VERTSIZE);
                    let dpi = GetDeviceCaps(dc, LOGPIXELSX);
                    if w > 0 && h > 0 {
                        width_mm = w as f64;
                        height_mm = h as f64;
                    }
                    if dpi > 0 {
                        scale = dpi as f64 / 96.0;
                    }
                    DeleteDC(dc);
                }
            }
            q2_layout::OsPanel {
                index: i as u32,
                x: item.rect.left,
                y: item.rect.top,
                width,
                height,
                width_mm,
                height_mm,
                scale,
            }
        })
        .collect()
}

#[cfg(windows)]
fn fallback_mm(width: u32, height: u32) -> (f64, f64, f64) {
    (width as f64 * 25.4 / 96.0, height as f64 * 25.4 / 96.0, 1.0)
}
