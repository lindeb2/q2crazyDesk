//! The foreground app, when it covers a whole monitor (R-INPUT-4, fullscreen guard).

use std::mem::{size_of, zeroed};

use winapi::{
    shared::{minwindef::DWORD, windef::RECT},
    um::{
        handleapi::CloseHandle,
        processthreadsapi::OpenProcess,
        winbase::QueryFullProcessImageNameW,
        winnt::PROCESS_QUERY_LIMITED_INFORMATION,
        winuser::{
            GetClassNameW, GetForegroundWindow, GetMonitorInfoW, GetWindowRect,
            GetWindowThreadProcessId, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST,
        },
    },
};

/// Windows that always fill the screen and are not an app in the way.
const SHELL_CLASSES: [&str; 3] = ["Progman", "WorkerW", "Shell_TrayWnd"];

/// The executable name (for example `game.exe`) of the foreground window when
/// it covers the whole monitor it is on, otherwise `None`.
pub fn foreground_fullscreen_app() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut class = [0u16; 64];
        let len = GetClassNameW(hwnd, class.as_mut_ptr(), class.len() as i32);
        if len > 0 {
            let name = String::from_utf16_lossy(&class[..len as usize]);
            if SHELL_CLASSES.contains(&name.as_str()) {
                return None;
            }
        }
        let mut rect: RECT = zeroed();
        if GetWindowRect(hwnd, &mut rect) == 0 {
            return None;
        }
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = zeroed();
        info.cbSize = size_of::<MONITORINFO>() as DWORD;
        if GetMonitorInfoW(monitor, &mut info) == 0 {
            return None;
        }
        let m = info.rcMonitor;
        let covers = rect.left <= m.left
            && rect.top <= m.top
            && rect.right >= m.right
            && rect.bottom >= m.bottom;
        if !covers {
            return None;
        }
        let mut pid: DWORD = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buf = [0u16; 520];
        let mut size = buf.len() as DWORD;
        let ok = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &mut size);
        CloseHandle(process);
        if ok == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        path.rsplit(['\\', '/']).next().map(|name| name.to_owned())
    }
}
