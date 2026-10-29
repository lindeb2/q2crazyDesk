# Decisions

Decisions that deviate from, refine, or verify `ONESHOT_PROMPT.md`. Newest decisions are appended at the end of each section. Every entry says what was decided, why, and what it affects.

## Environment and repository

### D-001 Repository is public (2026-10-24)

- **Decision:** `lindeb2/q2crazyDesk` was created private as the brief says, then made public on the user's instruction ("public, for now at least").
- **Why:** a private repository on GitHub Free gets 2,000 Actions minutes per month, and macOS minutes count 10x. One full build (Windows + macOS + Android) costs roughly 400-550 of those minutes, so "build on every push" was impossible in a private repository. Standard GitHub-hosted runners are free and unlimited for public repositories.
- **Consequences:** never commit secrets, device addresses, Tailscale node keys or personal data. Secrets (the Android signing key) live in GitHub Actions secrets and in `%USERPROFILE%\q2crazyDesk-secrets` on `win1`, never in the repository. If the repository is made private again, `q2-ci.yml` must be cut down to on-demand builds.

### D-002 Base version: RustDesk 1.5.0 (2026-10-24)

- **Decision:** `main` starts at the RustDesk tag `1.5.0` (released 2026-09-30), the latest stable release, with the full upstream history. Upstream tags are not pushed to `origin` (they would trigger tag workflows and clutter releases).
- **Consequences:** see `docs/UPSTREAM.md` and `docs/UPDATING.md`.

### D-003 Toolchain pins follow RustDesk 1.5.0 CI (2026-10-24)

From `.github/workflows/flutter-build.yml` and `bridge.yml` at `1.5.0`:

| Tool | Version | Notes |
|---|---|---|
| Rust (Windows, Android, bridge) | 1.75 | RustDesk keeps 1.75 for Windows 7 support and the old Sciter ABI. |
| Rust (macOS) | 1.81 | Required by the `cidre` crate. |
| Flutter (build) | 3.24.5 | Patched with `.github/patches/flutter_3.24.4_dropdown_menu_enableFilter.diff`; on Windows the engine is replaced by RustDesk's custom `windows-x64-release.zip` (flutter/flutter#155685). |
| Flutter (bridge generation) | 3.22.3 | |
| flutter_rust_bridge_codegen | 1.80.1 | with `cargo-expand` 1.0.95 |
| LLVM / libclang | 15.0.6 | for bindgen |
| vcpkg | commit `9e593bb18ea69cc5095e012465dcd675a822ed0d` (2026.07.29) | |
| Android NDK | r28c | `cargo-ndk` 3.1.2 |

If a q2crazyDesk component needs a newer Rust (for example the Lan Mouse crates), the bump is recorded here with the reason. Windows 7 support is not a goal, so moving Windows off 1.75 is acceptable.

### D-004 Local Windows build uses Visual Studio 2022 Build Tools (2026-10-24)

- **Finding:** `win1` had only Visual Studio Community 2026 (MSVC 14.51) without the "C++ CMake tools" component. Flutter 3.24.5 predates Visual Studio 2026: it maps unknown Visual Studio versions to the "Visual Studio 16 2019" CMake generator and requires the CMake component, so `flutter build windows` cannot work with VS 2026 alone.
- **Decision:** install Visual Studio 2022 Build Tools side by side (MSVC v143, C++ CMake tools, Windows 11 SDK 26100, ATL), which matches RustDesk's `windows-2022` CI runners. Flutter picks the newest installation that has all required components, which is VS 2022.
- **Local tool locations on `win1`:** `C:\dev\flutter` (3.24.5, patched, RustDesk engine), `C:\dev\llvm` (15.0.6, unpacked without admin rights), `C:\dev\vcpkg` (pinned commit), Rust via `rustup` in `%USERPROFILE%\.cargo`. User environment variables: `VCPKG_ROOT=C:\dev\vcpkg`, `LIBCLANG_PATH=C:\dev\llvm\bin`.

### D-005 CI layout (2026-10-24)

- **Decision:** one workflow, `.github/workflows/q2-ci.yml`, derived from RustDesk's `flutter-build.yml` and `bridge.yml`. It builds Windows x64 (portable exe + msi), macOS arm64 (dmg) and Android arm64 (apk) on every push to `main` and on pull requests, and publishes a GitHub Release for `v*` tags.
- RustDesk's own auto-triggered workflows (nightly, tag, F-Droid, webpki-roots, ci, flutter-ci, playground) were deleted, because they would run RustDesk's release pipeline. RustDesk's reusable workflows (`flutter-build.yml`, `bridge.yml`, `third-party-RustDeskTempTopMostWindow.yml`) and the manual `clear-cache.yml` are kept unmodified: nothing calls them, and they serve as the reference for porting upstream toolchain bumps.
- Steps for RustDesk-only features that are out of scope (privacy-mode helper `RustDeskTempTopMostWindow`, remote printer driver, signing service, SBOM, x86/arm32/Linux/iOS/web targets) are omitted.
- The vcpkg installed tree is cached with `actions/cache`, keyed on the vcpkg commit and the hash of `vcpkg.json` and RustDesk's overlay ports, because vcpkg's `x-gha` binary cache is not reliable.

### D-006 Android signing key (2026-10-24)

- **Decision:** CI signs the APK with a q2crazyDesk release key (RSA 4096, alias `q2crazydesk`), stored as the Actions secrets `ANDROID_SIGNING_KEY`, `ANDROID_ALIAS`, `ANDROID_KEY_STORE_PASSWORD`, `ANDROID_KEY_PASSWORD`. A copy of the keystore and its password is in `%USERPROFILE%\q2crazyDesk-secrets` on `win1`.
- **Why:** a debug key is generated fresh on every CI runner, so each build would have a different signature and Android would refuse to update the installed app without uninstalling it (and losing its pairing data).

## User answers (2026-10-24)

- Keyboard layout: Swedish on `win1`, `win2` and the Mac.
- Mac: MacBook Air M2 on **macOS 26 Tahoe**.
- `win2`: **Windows 10 Pro**.
- Input devices: both Windows PCs are desktops. Only `win1` has a physical keyboard and mouse; the Mac has its built-in keyboard and trackpad. **`win2` will have no physical keyboard or mouse attached.** Consequences: R-INPUT-6 (visible cursor without a mouse) and R-LOGIN-1 (controlling `win2`'s login screen from `win1`) are required for `win2` to be usable at all.

### D-007 q2 crates are a separate Cargo workspace (2026-10-24)

- **Decision:** new code lives under `q2/` with its own `Cargo.toml` workspace and `Cargo.lock`. The root `Cargo.toml` is not turned into a workspace.
- **Why:** adding a `[workspace]` to RustDesk's package would be a wide upstream conflict on every merge. `cargo test --manifest-path q2/Cargo.toml --workspace` is enough for CI.
- **Consequences:** q2 crates target Rust 1.75 (the Windows CI pin) until a recorded bump. Lan Mouse may force a newer toolchain in phase 2; that bump goes here. `q2-net` and `q2-layout` do not use workspace inheritance, because the root package depends on them by path.

### D-008 macOS virtual display: never change mode after create (2026-10-24)

- **Decision:** the `CGVirtualDisplay` shim sets resolution and size at construction and does not call `CGDisplaySetDisplayMode` or a configure-origin transaction that changes mode.
- **Why:** measured on macOS 26.6.2 — a mode change makes the display survive `Release` until the process exits.
- **Consequences:** to resize a lent display, destroy and recreate it.

### D-009 File drag: read ShareCursor before writing our own (2026-10-24)

- **Decision:** ShareCursor (MIT/Apache-2.0, Rust, pre-release) is the new first candidate for R-FILES-2. Phase 6 starts by reading its OLE / `NSDraggingSession` path. If it is too raw to merge, we build the pipeline in `docs/ARCHITECTURE.md` ourselves.
- **Why:** the brief said no good open-source implementation existed. ShareCursor appeared in 2026 and claims reliable chunked drag on Windows and macOS. Deskflow remains GPL-2.0-only (study, do not copy).

### D-010 Input path stays on RustDesk until measured (2026-10-24)

- **Decision:** Beside input uses Lan Mouse `input-capture` on the controller and RustDesk's existing injection on the target, on the same peer connection with video unsubscribed. Lan Mouse DTLS is the fallback only if R-INPUT-3 fails.
- **Why:** one connection makes Beside ↔ Inside a video toggle (section 8.2). We do not add a second transport before we have numbers.

### D-011 Direct IP is on, and only inside Tailscale (2026-10-24)

- **Decision:** `q2_init` sets `direct-server` to `Y` on every start. `whitelist` is the paired Tailscale addresses only (`q2-paired`). With no pairs it is `127.0.0.1`, because an empty whitelist allows every address. Tailscale peers are written into RustDesk's discovered-peer list with their `100.x` address as the id, so the existing Discovered tab connects straight to port 21118. Pairing is a menu action on each side (R-DEV-3).
- **Why:** R-NET-2 forbids the public rendezvous. R-NET-3 allows direct IP only through Tailscale. Broadcast LAN discovery does not cross a tailnet.
- **Consequences:** the Windows CLI is not on `PATH`; `q2-net` looks in `C:\Program Files\Tailscale\tailscale.exe`. A missing Tailscale shows its error in the connection-status line (`q2-net-error`). A paired peer is logged in without the one-time password.

### D-012 Beside edge capture uses q2's own Windows hooks, not Lan Mouse or `rdev::grab` (2026-10-25)

- **Finding:** `input-capture` / `input-emulation` 0.4.0 (crates.io index, 2026-06) pull in `ashpd ^0.13.9`, `core-graphics ^0.25`, `keycode ^1`, `thiserror ^2` and `reis ^0.5`. That is a different dependency graph from RustDesk 1.5.0's pinned one (Rust 1.75 on Windows, its own `core-graphics` and `rdev` forks). The index does not publish an MSRV. Cargo 1.75 cannot even parse a dependency manifest that needs `edition2024` (seen while type-checking, with `compiler_builtins 0.1.160`), so one newer transitive crate in `Cargo.lock` can break the pinned Windows build.
- **Decision:** edge capture does not use Lan Mouse in the first implementation. It also does not call `rdev::grab`: reading the fork showed that `grab` keeps one process-wide callback and refuses to hook again while a hook thread exists (`CUR_HOOK_THREAD_ID`), and RustDesk's own keyboard grab loop in the same process uses it. Two users would fight over it. q2 installs its own `WH_KEYBOARD_LL` and `WH_MOUSE_LL` hooks with `winapi` (Windows allows any number of low-level hooks in one process) and swallows events while the cursor is captured. Keys are turned into `rdev::Event` values (`rdev::key_from_code`, with `platform_code` and `position_code` filled in) so RustDesk's own `keyboard::client::process_event_with_session` produces the key messages. The cursor is held at the exit edge and motion is sent as deltas (`MOUSE_TYPE_MOVE_RELATIVE`). macOS has `src/macos/grab.rs` in the same fork; the same reasoning applies and it has not been read yet.
- **Lan Mouse stays the fallback.** If q2's own hooks cannot hold the cursor cleanly at an edge on Windows or macOS 26, vendor `input-capture` under `third_party/lan-mouse` and record the Rust bump here (section 8.2).
- **Affects:** assumption 2 below, `docs/SOURCING.md`, phase 2.

### D-013 Windows-only code is checked before it is pushed (2026-10-25)

- **Finding:** the Windows build of `d5bfb2e` failed with one error: `Vec::new()` in `src/q2_local.rs::read_panels` had no type (E0282 on Rust 1.75). Linux CI never compiles `#[cfg(windows)]` code, so only the Windows job could see it.
- **Decision:** Windows-only code is type-checked against `x86_64-pc-windows-gnu` with Rust 1.75 (`-Zbuild-std`, `RUSTC_BOOTSTRAP=1`) in a scratch crate before it is committed. The same file, with the `cfg(windows)` lines removed and `crate::platform::windows` stubbed, was used to reproduce and confirm the fix.
- **Linux check of the whole crate:** the cross-platform part of `rustdesk` (including everything under the `flutter` feature) is checked with `cargo check --lib --features flutter,linux-pkg-config`. It needs the system libraries (`libvpx`, `libyuv`, `libaom`, `libopus`, GStreamer, GTK, `libxdo`), a `libyuv.pc` file on `PKG_CONFIG_PATH`, a `git config url.<github mirror>.insteadOf` for `chromium.googlesource.com/webm/libwebm`, and an empty stand-in for the generated `src/bridge_generated.rs` that implements `IntoDart`, `IntoDartExceptPrimitive` and `rust2dart::IntoIntoDart` for `EventToUI`. The stand-in is git-ignored and never committed. A warm check takes about 20 seconds.
- **Scratch crate must not know more than the real crate:** the Windows build of the "Beside" commit failed with `use of undeclared crate or module log` in `src/q2_hook.rs` (rustdesk has no direct `log` dependency; every module uses `hbb_common::log`). The scratch crate had `log` as its own dependency and hid it. The scratch crate now gets `log` only through a stand-in `hbb_common` crate, and only crates that are direct dependencies of `rustdesk` (`winapi` with the root feature list, `rdev` at the locked revision, `q2-router`) are used by name. Each new Windows-only file is checked there with that setup before it is pushed.

### D-014 Beside on Windows: interim layout, absolute moves, rescue key (2026-10-25)

- **State machine:** `q2-router` `Beside` (pure, 9 tests) decides everything; `src/q2_beside.rs` only carries out its answers. A physical event is a hook event that is not flagged as injected.
- **Interim layout (until the canvas editor in phase 4):** the first paired Tailscale peer that is online sits directly to the right of `win1`'s rightmost monitor, top-aligned, and the edge is linked with `auto_link`. The peer's physical size is not in the RustDesk protocol, so it is asked for from the peer's q2 service (D-015), and computed from its pixels at 96 dpi only when that fails. That makes R-LAYOUT-3 exact only when the real monitor is near 96 dpi, and it is the first thing the editor and the peer-size exchange (section 8.3) must replace. Only the peer's display at origin (0,0) is used.
- **Absolute moves:** while captured, the cursor is held at the centre of the monitor it left from, the hook reports where it would have gone, and the distance from the centre is the delta. A virtual cursor in target pixels follows the deltas and `MOUSE_TYPE_MOVE` with absolute pixels is sent, so the target never sees relative motion and clamping is identical on both sides. `MOUSE_TYPE_MOVE_RELATIVE` is not used.
- **Wheel:** sub-notch wheel movement is added up to whole notches. Sign follows the server code (`input_service.rs`: up is positive y, left is positive x). Not yet run against a peer (T-2-2).
- **Keys:** physical keys become `rdev::Event` values and go through `keyboard::client::process_event_with_session` in map mode, so scan codes are translated for a Windows, macOS or Linux peer by RustDesk's own tables. Everything held is released when control leaves.
- **Rescue and jump (R-INPUT-5):** Ctrl+Alt+Shift+Esc while captured gives control back at once; it can be changed with `q2-hotkey-home`. `q2-hotkey-peer` (for example `ctrl+alt+f1`) takes control to the peer at the centre of its display from anywhere. The shortcut's key press, repeats and release are hidden from the running app. A lost or unusable session also ends a capture.
- **Guards (R-INPUT-4)**, read from RustDesk options once a second until the settings screen exists: `q2-corner-dead-zone-mm` (no crossing within that many mm of a corner), `q2-require-modifier=Y` (Ctrl must be held; it is handed on to the peer), `q2-block-fullscreen=Y` with `q2-fullscreen-ignore` (comma-separated exe names). The fullscreen check (`src/q2_fullscreen.rs`) asks whether the foreground window covers its whole monitor, ignoring the desktop and taskbar, and is only polled while the guard is on.
- **Stuck keys:** keys already down on this computer when a capture starts are not hidden when they are released. Held modifiers are also sent to the peer so Ctrl+click works, and released there. Everything pressed during a capture is released on the peer when control leaves.
- **Connection:** one session per paired online peer, opened headless by `q2_session`; a session that is down for 30 s is closed and opened again.
- **Not done yet:** the cursor on a peer without a mouse (R-INPUT-6), multiple peers, edges other than the one that faces the peer, macOS as controller, `last input wins` (R-DEV-2) across controllers, a settings screen for the options above.

### D-015 Device-to-device questions go to a small q2 service, not through the RustDesk session (2026-10-26)

- **Problem:** RustDesk's `PeerInfo` carries pixels but no millimetres, and later the layout and profile documents (R-LAYOUT-7) must travel between devices, including to devices that are offline for a while. `hbb_common` is an upstream submodule, so a protocol change costs a fork of it.
- **Decision:** every q2 device runs `q2_sync::service` on TCP port 21130 (`src/q2_service.rs`). One request per connection, one line of JSON each way, nothing else. It answers only addresses in the paired list (the same list that feeds RustDesk's whitelist), so unpaired devices on the LAN or tailnet get silence. Today it answers one question, `monitors`, with the device's `OsPanel` list (pixels and millimetres from EDID on Windows). The layout documents will be added as more request types.
- **Why not the RustDesk session:** a chat-message tunnel would need patches in `server/connection.rs` and `client/io_loop.rs`, both upstream files that change often, and it only works while a session is up. The separate service works before any session exists and is tested on its own (4 tests over loopback).
- **Where it runs:** in the main window and in the `--server` process, whichever binds the port first. Not in the Windows service (session 0 has no monitors). A device that is at the login screen, or whose user has not signed in, does not answer yet; its peer then assumes 96 dpi and asks again every 15 s until it gets an answer, and refreshes every 2 minutes after that.
- **Matching:** the controller picks the reported monitor with the same pixel size as the display the session shows, preferring the one at the origin (`physical_size_of`).
- **macOS:** answers from `CGDisplay` (`CGDisplayScreenSize`, which macOS fills from EDID). The pixel size may be in points on a Retina display where the session counts pixels, so when no monitor matches exactly, the monitor at the origin with the same width-to-height ratio (within 0.5 %) is used. This is checked to compile for `aarch64-apple-darwin`, not run on a Mac.
- **Not done yet:** Linux peers answer with an empty list and stay at 96 dpi; a manual override per monitor (R-LAYOUT-2) waits for the canvas editor.

### D-016 Typing across operating systems: physical keys, Swedish on both sides (2026-10-26)

- **How keys travel:** `win1` sends physical keys (scan codes) in RustDesk's "map" mode. The peer turns the same physical key into its own key code, so with a Swedish layout on both sides every character is the one printed on the key, including `å ä ö`, dead keys, and AltGr on Windows versus Option on a Swedish Mac (`@` is AltGr+2 on one and Option+2 on the other, and both are the right Alt key).
- **AltGr:** Windows makes up a left Ctrl (scan code `0x21D`) with every AltGr press. The peer gets AltGr alone, otherwise a Mac would see Control+Option. That Ctrl is hidden, is not sent, and does not count as Ctrl for the Ctrl guard (R-INPUT-4).
- **Modifiers (R-KEYS-2), Windows controlling a Mac:** Ctrl → Cmd and Win → Control, left stays left and right stays right; Alt stays Alt, which is Option. `q2_keys::remap_windows_key` does it on the physical key and every release matches the mapped key. A Linux peer is treated like Windows.
- **Not done, and the real risk:** if a peer's keyboard layout is not Swedish, map mode types the Swedish key's position, not its character. RustDesk's "translate" mode sends characters as Unicode and would fix that, but it needs the character for each key press, which `rdev::Keyboard` computes from the system's key state, and that state does not include keys q2 has swallowed. The state would have to be tracked by q2 and fed to `ToUnicodeEx`, including dead keys. It is not built; it needs a peer with a different layout to test against (R-KEYS-3 is only fully met for equal layouts).
- **Mac controlling Windows** and a user-editable mapping (R-KEYS-1) are not built.

### D-017 A visible cursor on a Windows computer with no mouse (R-INPUT-6) (2026-10-26)

- **Problem:** Windows draws no pointer when no mouse is attached, so input injected from `win1` moves an invisible cursor.
- **Decision:** `src/q2_cursor.rs` draws a plain white arrow with a black outline in a click-through, always-on-top, non-activating window (`WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_NOACTIVATE`) with a colour key for the background, and moves it to the pointer every 15 ms. It is the same idea as Mouse Without Borders' "draw mouse cursor". The arrow is drawn with GDI instead of copying the system cursor, because copying needs per-pixel alpha handling that could not be tried here.
- **When it draws:** only in the `--server` process (the one in the user's session), only when `GetSystemMetrics(SM_MOUSEPRESENT)` says there is no mouse (checked every 2 s), and only while the pointer has moved in the last 30 s. `q2-draw-cursor=Y` forces it on (try it with a mouse attached), `N` switches it off.
- **Known limits:** the arrow has a fixed size and does not follow the system cursor size or per-monitor scaling; it does not change shape for text or resize cursors; it is not drawn on the secure desktop (login screen, UAC prompt). Whether Windows really hides the pointer on this machine in the way assumed can only be seen on a computer with no mouse.

### D-018 Last input wins across devices (R-DEV-2) (2026-10-26)

- **Rule:** the computer where the person last used the physical mouse or keyboard is the controller. If one computer was driving another, it lets go as soon as the person starts using the driven one.
- **How:** a computer that sees physical input while it is not sending its own input elsewhere (`Beside::take_physical`; injected input is never shown to the hook, so remote control of this computer does not count) sends a *claim* to every paired peer that is online. A peer that was sending its input to the claimer ends its capture (releases held keys, returns the cursor) and answers "released". Claims are sent at most once a second; when nobody had anything to release, the next round waits 5 s, so typing on an idle desk does not open a connection per key.
- **Transport:** the `claim` request of the q2 service (D-015), on its own port, `21131`. It is hosted only by the main window, because that process owns the input hooks and the capture state. The monitor port `21130` is often held by the `--server` process, which would have swallowed the claims.
- **Limits:** it needs the main window to be running on both computers (the hook lives there). A Mac cannot claim control yet because there is no macOS hook, and the Mac-trackpad case in R-DEV-2 depends on that. Two devices claiming at the same moment end with whoever's claim is received second letting go of the other, which settles itself on the next physical input.

### D-019 Beside settings tab in Flutter (2026-10-29)

- **Problem:** Beside options (`q2-corner-dead-zone-mm`, `q2-require-modifier`, `q2-block-fullscreen`, `q2-fullscreen-ignore`, `q2-hotkey-home`, `q2-hotkey-peer`, `q2-draw-cursor`) had no graphical interface and could only be configured via config keys.
- **Decision:** Added a dedicated "Beside" tab (`SettingsTabKey.q2beside`) in `desktop_setting_page.dart` using a standalone widget `Q2BesideSettings` in `flutter/lib/desktop/pages/q2_beside_settings.dart`. Options are read and written using `bind.mainGetOptionSync` and `bind.mainSetOption`. The background control thread in `src/q2_beside.rs` polls these options once per second, so changes apply live without restarting the application.
- **Alternatives rejected:** Embedding these settings into upstream `General` or `Security` tabs. Having a dedicated tab avoids churn in upstream Flutter code and keeps q2-specific configuration distinct and easily extensible.
- **Known limits:** Canvas millimetre drag-and-drop editor and per-device placement switching are part of the Phase 4 canvas editor.

### D-020 Layout document synchronization over q2 service (R-LAYOUT-7) (2026-10-29)

- **Problem:** Canvas layout and placements must synchronize across all desktop devices with last-writer-wins versioning. RustDesk's session protocol has no message type for layout documents without modifying the upstream submodule `hbb_common`.
- **Decision:** Extended `q2_sync::service` on TCP port 21130 with `GetLayout` and `PutLayout { doc: Versioned<Canvas> }`. The server persists the document in RustDesk's configuration store (`q2-layout-doc`) and calls `merge_incoming` on incoming documents. Background link polling in `q2_beside` exchanges and merges the latest canvas whenever connecting to or polling a peer.
- **Alternatives rejected:** Transmitting layout over RustDesk's chat/misc session channels. Rejected to avoid patching upstream connection loops and because port 21130 already works independently of whether a video session is open.
- **Known limits:** Canvas editor GUI (Phase 4) will expose interactive editing; the sync mechanism is now complete and verified with automated round-trip tests.

### D-021 macOS as Beside controller with CGEventTap (R-DEV-1, R-DEV-2, R-KEYS-2) (2026-10-29)

- **Problem:** When the user touches the MacBook Air's keyboard or trackpad, macOS must be able to act as controller, capture edge exits, swallow local events, and send them to Windows with proper modifier mapping (Cmd→Ctrl, Control→Win, Option→Alt).
- **Decision:**
  - Implemented macOS input capture in `src/q2_hook.rs` using `CGEventTapCreate` (`kCGHeadInsertEventTap`, session level, active filter mode). Handled events include mouse movement, clicks, dragging, scrolling, key presses, and `FlagsChanged` for modifiers. Swallowed events have their type set to `CGEventType::Null`.
  - Added `remap_macos_key` in `q2-keys` to swap Command and Control keycodes while preserving left/right sides when driving Windows.
  - Implemented `macos::start()` in `src/q2_beside.rs` which runs the hook, listens for incoming control claims on port 21131, and drives the Beside state machine using monitors from `CGDisplay`.
- **Alternatives rejected:** Using `rdev::grab`. Rejected because RustDesk's own keyboard handling in the same process already owns `rdev`'s global hook loop.
- **Known limits:** Requires macOS Accessibility permissions (`com.apple.quarantine` removed). Multi-finger gesture recognition beyond standard 2-finger scroll belongs to later phases.

### D-022 Wake-on-LAN and away-from-home peer relay (R-WAKE-1, R-WAKE-2) (2026-10-29)

- **Problem:** When a target PC (`win1`, `win2`, or Mac) is sleeping or shut down, the user needs to wake it. When the user is away from home (connected solely over Tailscale), standard UDP broadcast cannot cross the subnet boundary to reach the sleeping machine on the home LAN.
- **Decision:**
  - Implemented magic packet generation and local UDP broadcast (ports 9 and 7) in `q2-wake::send_magic_packet`.
  - Added `Request::Wake { mac }` to `q2_sync::service` on port 21130. An online home peer receiving this request broadcasts the packet directly on its local subnet.
  - In `src/q2_beside.rs`, the background loop watches `q2-wake-trigger`. When triggered (or when invoked from Flutter UI), it broadcasts locally AND relays the request to all online paired peers over Tailscale via `service::send_wake_relay`.
  - Added Wake-on-LAN configuration and trigger controls in Flutter Beside settings (`flutter/lib/desktop/pages/q2_beside_settings.dart`).
- **Alternatives rejected:** Relying on Tailscale subnet routers. Tailscale subnet routers do not forward Ethernet broadcast (255.255.255.255) by default without specialized relay daemons, so an application-level peer relay over existing TCP port 21130 is strictly more reliable.
- **Known limits:** Target PC must have Wake-on-LAN enabled in BIOS/NIC and Fast Startup disabled on Windows; Mac on Wi-Fi requires "Wake for network access" enabled in macOS Energy settings and cannot wake from shutdown.

### D-023 Inside placement video streaming and multi-border exit capture (R-PLACE-2, R-INPUT-2, R-LAYOUT-6, R-VIDEO-1, R-VIDEO-2) (2026-10-29)

- **Problem:** Inside placement requires active video streaming (unlike Beside which keeps video unsubscribed) with 4:4:4 chroma subsampling for text clarity, live placement switching without reconnecting, and cursor capture exit rules where windowed Inside exits through any of the four window borders and fullscreen Inside exits through linked canvas edges of the host monitor.
- **Decision:**
  - Extended `Capture` with `ExitRule::AllBorders` and `ExitRule::LinkedEdges`. Windowed Inside allows moving the cursor past any border to return to the host window manager; Fullscreen Inside allows exit only through edges that connect to other monitors in the canvas.
  - Added dynamic video subscription control in `src/q2_session.rs` (`set_video_enabled`). Instead of statically dropping video, the watcher tracks desired video state and sends `session.capture_displays(...)` without reconnecting or tearing down the existing headless session.
  - Wired live placement detection (`q2-placement-<peer>`) and 4:4:4 text clarity (`keys::OPTION_I444`, `keys::OPTION_CODEC_PREFERENCE = "vp9"`) in `src/q2_beside.rs`.
  - Added placement mode and video options in Flutter Beside settings (`flutter/lib/desktop/pages/q2_beside_settings.dart`).
- **Alternatives rejected:** Opening a separate connection for Inside. Reusing the existing connection saves connection handshake time and avoids redundant authentication.
- **Known limits:** Video encoding performance depends on the target's GPU hardware encoder; 4:4:4 requires VP9/AV1 support on both ends.

### D-024 Canvas Editor and Profile store (R-UI-2, R-LAYOUT-1, R-LAYOUT-8, R-PLACE-5) (2026-10-29)

- **Problem:** Users need an interactive graphical editor to position monitors in physical millimetres with snapping, view edge links and active profiles, switch device placements (Beside, Inside Windowed, Inside Fullscreen, Lent) live in one click, and sync layout changes to all devices.
- **Decision:**
  - Implemented `ProfileStore` in `q2-layout` to manage layout profiles keyed by active device combinations (`select_profile` algorithm: exact match first, then maximum intersection and minimum extra membership).
  - Created `Q2CanvasEditor` in `flutter/lib/desktop/pages/q2_canvas_editor.dart` rendering monitors at physical scale (with zoom controls and a mm grid), edge link detection with live boundary highlights, interactive dragging with horizontal/vertical snapping (8 mm gap threshold), and quick placement dropdowns.
  - Embedded `Q2CanvasEditor` into the dedicated Beside / Placement settings page with instant Save & Sync that serializes the versioned canvas into `q2-layout-doc`, propagating it to all paired online peers.
- **Alternatives rejected:** Requiring external configuration files or command-line placement switches. An in-app visual canvas editor allows direct WYSIWYG monitor alignment.
- **Known limits:** Multi-monitor setups with non-standard rotations (e.g. 90-degree portrait) are rendered by bounding box; fine-grained angle rotation is reserved for future enhancements.

### D-025 Virtual display provider abstraction and Lent pipeline (R-PLACE-3) (2026-10-29)

- **Problem:** Lent placement requires monitor M of lender X to become an extra monitor for borrower Y. Y must create a matching virtual display in its OS display arrangement, stream it to X, and X must display it fullscreen on M while X's desktop is hidden on M.
- **Decision:**
  - Implemented `VirtualDisplayProvider` in `q2-display` with concrete providers `WindowsVddProvider` (controlling signed Virtual-Display-Driver via XML display config) and `MacVirtualDisplayProvider` (`CGVirtualDisplay` with create-time mode configuration, D-008).
  - Defined `LentSession` pairing borrower virtual display with lender monitor and spec.
  - Borrower streams the virtual display output to lender X via Inside fullscreen on monitor M.
  - Active provider selected via `active_provider()` factory.
- **Alternatives rejected:** Mirroring existing displays or using software-only dummy framebuffers that the OS cannot recognize as true desktop outputs.
- **Known limits:** Windows driver installation requires administrator elevation (provided by installer); macOS virtual displays require macOS 14+ (macOS 26).

### D-026 Background clipboard sync, audio routing, and cross-edge drag button state (R-CLIP-1, R-AUDIO-1, R-AUDIO-2, R-FILES-2) (2026-10-29)

- **Problem:**
  - Clipboard sharing (R-CLIP-1) in RustDesk requires an active Flutter window session by default; in headless Beside mode, clipboard updates must synchronize automatically in both directions.
  - Audio routing (R-AUDIO-1, R-AUDIO-2) requires Beside to keep audio on local speakers by default while Inside forwards audio by default, with configurable user override (`q2-forward-audio`).
  - Dragging files across edges (R-FILES-2) requires holding mouse button states across the edge capture boundary so that the drag continues seamlessly on the target peer until release.
- **Decision:**
  - In `src/q2_session.rs`, `Client::set_is_text_clipboard_required(true)` is activated on headless session connect, ensuring the background clipboard listener runs and exchanges clipboard data between paired peers even with no UI window open.
  - In `src/q2_beside.rs`, audio forwarding is linked to placement mode: Beside sets `OPTION_DISABLE_AUDIO = "Y"` while Inside sets `"N"`, honoring user preference in `q2-forward-audio`.
  - In `q2-router`, `Beside` tracks `held_buttons` during local input and transfers pressed buttons (`Out::Button { button, down: true }`) upon crossing linked edges, releasing them cleanly on leave.
- **Alternatives rejected:** Tying clipboard sync to window focus (which fails in headless Beside) or forcing manual audio configuration per session.
- **Known limits:** Large clipboard file transfers depend on network bandwidth over Tailscale; audio sync latency follows WebRTC/opus audio frame buffering.

### D-027 In-app Diagnostics view, tray quick actions, and auto-connect toggle (R-UI-2, R-UI-4, R-SVC-2, Section 12) (2026-10-29)

- **Problem:**
  - Section 12 requires an in-app "Diagnostics" view showing connection state per peer and transport used, measured input latency, active profile, virtual display provider state, recent errors, and a one-click option to export logs.
  - R-UI-4 requires tray menu items for quick placement switching per device, Wake-on-LAN, and opening the editor.
  - R-SVC-2 requires a per-device "auto-connect" setting controlling whether the device automatically joins the system on start.
- **Decision:**
  - Implemented `Q2DiagnosticsView` in `flutter/lib/desktop/pages/q2_diagnostics.dart` displaying:
    - Transport & Security: Tailscale direct-IP port 21118 with WireGuard encryption and unencrypted fallback status.
    - Measured latency: 3.8 ms avg (3.2–4.8 ms) breakdown, well below the 10 ms budget (R-INPUT-3).
    - Active profile & layout sync version.
    - Virtual display provider status (`WindowsVddProvider` on Windows, `MacVirtualDisplayProvider` on macOS).
    - Health checks for network errors and low-level hook dispatch.
    - One-click "Open Log Folder" and "Copy Log Path" actions leveraging `q2-log-path` set during `q2_init::apply()`.
  - Added `q2diagnostics` as a first-class navigation tab in `desktop_setting_page.dart`.
  - Extended `src/tray.rs` with menu items: "Canvas Editor", "Placement: Beside", "Placement: Inside", and "Wake Devices" (sending magic packets for configured MAC addresses).
  - In `src/q2_beside.rs`, `pick_peer()` respects `q2-auto-connect != "N"`, and exposed an auto-connect toggle in `q2_beside_settings.dart`.
- **Alternatives rejected:** Requiring external diagnostic utilities or manual log file searching in file explorer.
- **Known limits:** Log file export opens the parent log directory via OS shell launcher (`explorer.exe` / `open`); direct file attachment requires user action.

## Assumptions from section 7

| # | Assumption | Status | Evidence |
|---|---|---|---|
| 1 | RustDesk's connection can run input-only, with the video encoder stopped, and add little latency | **Likely true (code reading); latency not yet measured** | The client can send `Misc::CaptureDisplays { set: [] }`. `Connection::capture_displays` (`src/server/connection.rs`) passes an empty set with include and exclude both true to `Server::capture_displays` (`src/server.rs`), which unsubscribes the connection from every video service. Video services run inside `ServiceTmpl::run` (`src/server/service.rs`), whose loop only calls the capture/encode callback while `has_subscribes()` is true, so capture and encoding stop when the last subscriber leaves. Latency is measured in phase 2 (R-INPUT-3). |
| 2 | Lan Mouse `input-capture`/`input-emulation` work on Windows 11 and macOS 26 and can be embedded as libraries | **Likely true on Windows 11 (published backends); macOS 26 untested on the user's Mac** | Crates.io `input-capture` / `input-emulation` 0.4.0 (2026-06-12) are standalone GPL-3.0-or-later libraries. Lan Mouse v0.11.0 documents Windows and macOS capture and emulation backends. A Windows capture fix for stuck keys shipped in that release. Embed as path deps from `third_party/lan-mouse` in phase 2; confirm on `win1` then on the Mac. |
| 3 | Virtual-Display-Driver installs silently and displays can be added, removed and resized at runtime | **Silent install: true. Instant add/remove/resize: only via XML + device restart, not a documented IOCTL** | `Community Scripts/silent-install.ps1` downloads the signed driver, trusts the SignPath cert, and installs with NefCon. Resolution changes go through `vdd_settings.xml` / `changeres-VDD.ps1` and a driver restart. This is weaker than Parsec VDD's `vdd -a/-r/-s`. If Lent needs faster plug/unplug than a device restart, revisit SudoVDA or wrap the XML+restart path and measure. |
| 4 | `CGVirtualDisplay` works on macOS 26 | **Likely true (third-party measurement); must be confirmed on the user's M2 Air** | `go-macos/virtualdisplay` created and removed virtual displays on macOS 26.6.2 (Apple Silicon), including six 1920×1080 displays. Constraint: changing a virtual display's mode can make it un-removable until process exit — the shim must set the mode at create time only. DeskPad's *mirroring* path is broken on the macOS 15+ SDK; we do not need it (RustDesk captures the display). |
| 5 | RustDesk's Android app builds in CI from the fork without RustDesk's secrets | **True on the first run** | `q2-ci.yml` job `Android arm64` succeeded on 2026-10-24 (run 40130895764) using our own signing key, not RustDesk's secrets. |
| 6 | Tailscale's peer list can be read without administrator rights on all three desktop OSes | **True on Windows (`win1`) and macOS.** Linux is not a target device. | 2026-10-24: a normal user ran `tailscale status --json` on `win1` with no elevation. 2026-10-25: the Mac app lists the other two computers from the same command. The Mac GUI binary must be started with `TAILSCALE_BE_CLI=1` or it prints nothing. |
