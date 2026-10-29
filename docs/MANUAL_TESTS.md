# Manual tests

Every R-ID has at least one physical test. Run the subset for the phase that just finished, then record the result here.

Result values: `pass` / `fail` / `blocked` / `not run`.

## Phase 0 — bootstrap

### T-0-1 Local Windows build (`win1`)
- **Covers:** phase 0 acceptance
- **Steps:**
  1. On `win1`, open `flutter\build\windows\x64\runner\Release\rustdesk.exe` (or the portable exe from a successful `python build.py --flutter --hwcodec --vram`).
  2. Confirm the main window opens and the tray icon appears.
- **Expected:** the unmodified RustDesk UI runs.
- **Result:** pass
- **Date / notes:** 2026-10-24, `win1`. User started `flutter\build\windows\x64\runner\Release\rustdesk.exe`. The window and the tray icon appeared.

### T-0-2 CI artifacts
- **Covers:** phase 0 acceptance
- **Steps:**
  1. Open the latest green `q2crazyDesk CI` run on `main`.
  2. Confirm artifacts `q2crazydesk-windows-x86_64`, `q2crazydesk-macos-aarch64`, `q2crazydesk-android-aarch64` are present.
- **Expected:** all three jobs succeeded and uploaded artifacts.
- **Result:** pass
- **Date / notes:** 2026-10-25, run 40147278687. Jobs Windows x64, macOS arm64, and Android arm64 succeeded and uploaded `q2crazydesk-windows-x86_64`, `q2crazydesk-macos-aarch64`, and `q2crazydesk-android-aarch64`.

## Phase 1 — devices, network, service

User confirmed on 2026-10-25, after run 40154488523: one card per computer, the Mac icon, the status line, and connecting both ways. The checks below are still open.

### T-1-1 Online after cold boot (R-DEV-4, R-SVC-1, R-SVC-2, R-NET-4)
- **Steps:**
  1. Install the Windows build on `win1` and `win2` with the service.
  2. Sign both into the same Tailscale tailnet, unattended mode on.
  3. Shut down both, power on, do **not** log in on `win2`.
  4. Log in on `win1` and open the device list.
- **Expected:** `win2` is online (or at least reachable) before anyone logs in there.
- **Result:** not run

### T-1-2 Reconnect after unplugging Ethernet (R-SVC-3)
- **Steps:** unplug `win2`'s cable for 30 seconds, plug it back in. Watch the device list on `win1`.
- **Expected:** `win2` goes offline, then online again without a click.
- **Result:** not run

### T-1-3 Pairing (R-DEV-3)
- **Steps:**
  1. On `win1` and the Mac, open Discovered, right-click the other computer, and choose **Pair device**. Do this on both.
  2. Connect from `win1` to the Mac.
  3. On `win1`, choose **Unpair device** for the Mac and try to connect again.
- **Expected:** the paired connection is accepted. After unpairing, the Mac rejects `win1`.
- **Result:** not run
- **Date / notes:** 2026-10-25. On the Mac, Pair device and Unpair device work. A connection from that build reached the one-time password prompt. Passwordless login is in run 40147278687, which was not the build under test.

### T-1-4 Tailscale missing (R-NET-5)
- **Steps:** stop Tailscale on `win1` and open the app.
- **Expected:** a clear error that says how to start Tailscale. LAN fallback stays off.
- **Result:** not run

## Phase 2 — Beside

### T-2-1 Local crossing on `win1` (R-LAYOUT-4)
- **Steps:** place `win1`'s two monitors at their real physical positions. Move the cursor across the inner edge at the top, middle, and bottom.
- **Expected:** the cursor arrives at the matching physical height on the other monitor.
- **Result:** not run

### T-2-2 Cross to `win2` and the Mac (R-PLACE-1, R-LAYOUT-3)
- **Steps:** sit at `win1`. Cross into `win2`, then into the Mac, at a marked physical height (for example 100 mm from the top). While captured, scroll up and down one notch and press Ctrl+Alt+Shift+Esc.
- **Expected:** the cursor appears 100 mm from the top on the target. No video window opens. Scrolling up scrolls up on the target (the wheel sign convention in `src/q2_beside.rs` was read from the server code and has not been run). The rescue combination returns control to `win1` and releases held keys.
- **Result:** not run

### T-2-3 Last input wins (R-DEV-2)
- **Steps:** from `win1`, control `win2`. Touch the Mac trackpad.
- **Expected:** control moves to the Mac at once. Keys held on `win1` are released on `win2`.
- **Result:** not run

### T-2-4 Swedish characters (R-KEYS-3)
- **Steps:** from every controller into every target, type `å ä ö Å Ä Ö @ £ $ € { } [ ] | \ ~ ^ ´ ¨` and a dead-key combination (`¨` then `o` → `ö`).
- **Expected:** the right character appears in Notepad / TextEdit.
- **Result:** not run

### T-2-5 Jump hotkey and guards (R-INPUT-4, R-INPUT-5)
- **Steps:** set a jump hotkey for `win1`. Cross with a corner dead zone on, then with "require modifier" on, then with a fullscreen app in front.
- **Expected:** jump returns the cursor to `win1`. Guards block or allow as configured.
- **Result:** not run

### T-2-6 Cursor on `win2` without a mouse (R-INPUT-6)
- **Steps:** leave `win2` without a mouse. Cross into it from `win1`.
- **Expected:** a cursor is visible on `win2`.
- **Result:** not run

### T-2-7 Latency (R-INPUT-3)
- **Steps:** measure added latency `win1` → `win2` on Ethernet (Diagnostics view or the recorded method in `docs/ARCHITECTURE.md`).
- **Expected:** under 10 ms added.
- **Result:** not run

### T-2-8 Beside settings GUI (R-INPUT-4, R-INPUT-5, R-INPUT-6, R-UI-2)
- **Steps:** on `win1`, open Settings -> Beside tab. Change corner dead zone, toggle "Require modifier to cross", set a hotkey, and change "Draw cursor on mouseless target".
- **Expected:** settings apply live without restarting the application; inputs reflect the new settings immediately.
- **Result:** not run

### T-2-9 Mac controlling Windows (R-DEV-1, R-DEV-2, R-KEYS-2)
- **Steps:** sit at the Mac. Move the cursor across the screen edge toward `win1`. Type Cmd+C, Cmd+V, Control, and Swedish characters (`å ä ö @`).
- **Expected:** cursor captures and appears on `win1`. Cmd+C triggers Ctrl+C on Windows (copy), Control triggers Windows key, and `@` (Option+2 on Mac) produces `@` on Windows. Touching `win1`'s physical mouse takes control back immediately (last-input-wins).
- **Result:** not run

## Phase 3 — Inside

### T-3-1 Window enter/leave (R-PLACE-2, R-INPUT-2)
- **Steps:** switch `win2` to Inside (windowed) on `win1`. Move into the window without clicking, then out through any of the four borders.
- **Expected:** control follows the mouse. No click required. Moving past any border exits back to host.
- **Result:** not run (Capture `ExitRule::AllBorders` unit tested in `q2-router`).

### T-3-2 Fullscreen exits (R-LAYOUT-6)
- **Steps:** fullscreen Inside of `win2` on one `win1` monitor. Leave through a linked edge toward `win1`'s other monitor and toward the Mac.
- **Expected:** those edges exit; unlinked edges stay walls.
- **Result:** not run (Capture `ExitRule::LinkedEdges` unit tested in `q2-router`).

### T-3-3 Film (R-VIDEO-1, R-VIDEO-2, R-AUDIO-2)
- **Steps:** play a 1080p film on `win2`, watch it in an Inside window on `win1` with 4:4:4 on.
- **Expected:** smooth enough to watch, sound in sync, text sharp.
- **Result:** not run

## Phase 4 — editor, profiles, live switch

### T-4-1 Sync (R-LAYOUT-7)
- **Steps:** move a monitor on the Mac editor. Check `win1`.
- **Expected:** `win1` updates. Last writer wins if both edit.
- **Result:** not run (Sync protocol and last-writer-wins unit tested in `q2-sync`).

### T-4-2 Live placement switch (R-PLACE-5)
- **Steps:** switch `win2` Beside → Inside → Beside in one action each using the canvas editor or quick placement dropdown.
- **Expected:** under about one second, no new pairing or reconnection delay.
- **Result:** not run (Live video stream toggle on existing session tested).

### T-4-3 Profile restore (R-LAYOUT-8)
- **Steps:** unplug the Mac (or turn Tailscale off on it). Use `win1`+`win2`. Plug the Mac back in.
- **Expected:** the three-device profile returns.
- **Result:** not run (Profile store exact match and closest subset selection unit tested in `q2-layout`).

## Phase 5 — Lent

### T-5-1 Mac as extra monitor for `win1` (R-PLACE-3)
- **Steps:** lend the Mac display to `win1`. Drag a window from `win1` onto it. Tear down.
- **Expected:** the Mac shows `win1`'s extra desktop. Tear-down restores the Mac desktop.
- **Result:** not run

### T-5-2 `win1` monitor lent to the Mac
- **Result:** not run

### T-5-3 `win2` lent to `win1`
- **Result:** not run

## Phase 6 — files, audio, login, wake

### T-6-1 Copy/paste files (R-FILES-1, R-CLIP-1)
- **Steps:** copy a folder on `win1`, paste on the Mac and on `win2`. Reverse.
- **Expected:** files arrive; large files show progress. Background clipboard listener syncs across headless Beside sessions.
- **Result:** not run (Session clipboard listener activation wired in `q2_session`).

### T-6-2 Drag files across a Beside edge (R-FILES-2)
- **Steps:** drag a file from `win1`'s desktop across the edge onto `win2` and onto the Mac, and into an Inside window.
- **Expected:** the file lands in the drop target. Left mouse button stays held across the edge boundary until dropped.
- **Result:** not run (`held_buttons` tracking and cross-edge button transfer unit tested in `q2-router`).

### T-6-3 Audio routing (R-AUDIO-1, R-AUDIO-3)
- **Steps:** play sound on `win2` in Beside (own speakers), then forward to `win1`. Repeat with the Mac as source.
- **Expected:** sound follows the setting (`q2-forward-audio`). macOS capture works on macOS 26.
- **Result:** not run (Audio option link to placement wired in `q2_beside`).

### T-6-4 Login and UAC on `win2` (R-LOGIN-1)
- **Steps:** lock `win2`. From `win1`, in Beside, type the password. Trigger a UAC prompt on `win2` and accept it.
- **Expected:** both work without a local keyboard on `win2`.
- **Result:** not run

### T-6-5 Wake (R-WAKE-1, R-WAKE-2)
- **Steps:** configure target MAC in Beside settings (`q2-device-macs` or input field). Sleep `win2`. From `win1` on Ethernet, trigger Wake. Then sleep `win2` again and trigger Wake from the Mac on another network (via Tailscale).
- **Expected:** `win2` wakes. Direct broadcast sends magic packet on home LAN; away-from-home relays request to online home peer on TCP port 21130.
- **Result:** not run (unit tests for packet layout, UDP broadcast, TCP wake relay, and settings trigger pass).

## Phase 7 — away and Android

### T-7-1 Phone on mobile data (R-PHONE-1, R-PHONE-2, R-WAKE)
- **Steps:** on the phone, Tailscale on, Wi-Fi off. Wake `win1` if asleep. Open a fullscreen session.
- **Expected:** session works.
- **Result:** not run

### T-7-2 Mac on another network (R-NET-1)
- **Steps:** take the Mac off the home Wi-Fi. Open Inside sessions to `win1` and `win2`.
- **Expected:** both sessions work over Tailscale.
- **Result:** not run

## Phase 8 — diagnostics and quick tray actions

### T-8-1 In-app diagnostics and log export (R-UI-2, Section 12)
- **Steps:** on `win1` (or the Mac), open Settings -> Diagnostics tab. Verify that peer connection status, measured input latency (< 10 ms budget), active profile, virtual display status, and error logs are shown. Click "Open Log Folder" and "Copy Log Path".
- **Expected:** diagnostics are rendered cleanly with live status indicators; clicking "Open Log Folder" launches the log directory in Explorer/Finder, and "Copy Log Path" copies the path to clipboard.
- **Result:** not run

### T-8-2 Tray quick menu placement, wake, editor (R-UI-4)
- **Steps:** right-click the q2crazyDesk tray icon in the Windows taskbar or macOS menu bar. Click "Placement: Beside", then "Placement: Inside", then "Wake Devices", then "Canvas Editor".
- **Expected:** placement options switch the default placement live without opening the full UI; "Wake Devices" dispatches magic packets to configured MAC addresses; "Canvas Editor" brings up the physical canvas editor window.
- **Result:** not run

## Smoke (after an upstream merge)

Run T-0-1, T-2-2, T-3-1, T-4-2, T-6-4.
