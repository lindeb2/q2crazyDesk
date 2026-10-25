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

### D-014 Beside on Windows: interim layout, absolute moves, rescue key (2026-10-25)

- **State machine:** `q2-router` `Beside` (pure, 9 tests) decides everything; `src/q2_beside.rs` only carries out its answers. A physical event is a hook event that is not flagged as injected.
- **Interim layout (until the canvas editor in phase 4):** the first paired Tailscale peer that is online sits directly to the right of `win1`'s rightmost monitor, top-aligned, and the edge is linked with `auto_link`. The peer's physical size is not in the RustDesk protocol, so it is computed from its pixels at 96 dpi. That makes R-LAYOUT-3 exact only when the real monitor is near 96 dpi, and it is the first thing the editor and the peer-size exchange (section 8.3) must replace. Only the peer's display at origin (0,0) is used.
- **Absolute moves:** while captured, the cursor is held at the centre of the monitor it left from, the hook reports where it would have gone, and the distance from the centre is the delta. A virtual cursor in target pixels follows the deltas and `MOUSE_TYPE_MOVE` with absolute pixels is sent, so the target never sees relative motion and clamping is identical on both sides. `MOUSE_TYPE_MOVE_RELATIVE` is not used.
- **Wheel:** sub-notch wheel movement is added up to whole notches. Sign follows the server code (`input_service.rs`: up is positive y, left is positive x). Not yet run against a peer (T-2-2).
- **Keys:** physical keys become `rdev::Event` values and go through `keyboard::client::process_event_with_session` in map mode, so scan codes are translated for a Windows, macOS or Linux peer by RustDesk's own tables. Everything held is released when control leaves.
- **Rescue (R-INPUT-5):** Ctrl+Alt+Shift+Esc while captured gives control back at once. A lost or unusable session also ends a capture.
- **Connection:** one session per paired online peer, opened headless by `q2_session`; a session that is down for 30 s is closed and opened again.
- **Not done yet:** jump hotkeys and guards (R-INPUT-4, R-INPUT-5 guards), the cursor on a peer without a mouse (R-INPUT-6), multiple peers, edges other than the right edge of the rightmost monitor, macOS as controller, `last input wins` (R-DEV-2) across controllers.

## Assumptions from section 7

| # | Assumption | Status | Evidence |
|---|---|---|---|
| 1 | RustDesk's connection can run input-only, with the video encoder stopped, and add little latency | **Likely true (code reading); latency not yet measured** | The client can send `Misc::CaptureDisplays { set: [] }`. `Connection::capture_displays` (`src/server/connection.rs`) passes an empty set with include and exclude both true to `Server::capture_displays` (`src/server.rs`), which unsubscribes the connection from every video service. Video services run inside `ServiceTmpl::run` (`src/server/service.rs`), whose loop only calls the capture/encode callback while `has_subscribes()` is true, so capture and encoding stop when the last subscriber leaves. Latency is measured in phase 2 (R-INPUT-3). |
| 2 | Lan Mouse `input-capture`/`input-emulation` work on Windows 11 and macOS 26 and can be embedded as libraries | **Likely true on Windows 11 (published backends); macOS 26 untested on the user's Mac** | Crates.io `input-capture` / `input-emulation` 0.4.0 (2026-06-12) are standalone GPL-3.0-or-later libraries. Lan Mouse v0.11.0 documents Windows and macOS capture and emulation backends. A Windows capture fix for stuck keys shipped in that release. Embed as path deps from `third_party/lan-mouse` in phase 2; confirm on `win1` then on the Mac. |
| 3 | Virtual-Display-Driver installs silently and displays can be added, removed and resized at runtime | **Silent install: true. Instant add/remove/resize: only via XML + device restart, not a documented IOCTL** | `Community Scripts/silent-install.ps1` downloads the signed driver, trusts the SignPath cert, and installs with NefCon. Resolution changes go through `vdd_settings.xml` / `changeres-VDD.ps1` and a driver restart. This is weaker than Parsec VDD's `vdd -a/-r/-s`. If Lent needs faster plug/unplug than a device restart, revisit SudoVDA or wrap the XML+restart path and measure. |
| 4 | `CGVirtualDisplay` works on macOS 26 | **Likely true (third-party measurement); must be confirmed on the user's M2 Air** | `go-macos/virtualdisplay` created and removed virtual displays on macOS 26.6.2 (Apple Silicon), including six 1920×1080 displays. Constraint: changing a virtual display's mode can make it un-removable until process exit — the shim must set the mode at create time only. DeskPad's *mirroring* path is broken on the macOS 15+ SDK; we do not need it (RustDesk captures the display). |
| 5 | RustDesk's Android app builds in CI from the fork without RustDesk's secrets | **True on the first run** | `q2-ci.yml` job `Android arm64` succeeded on 2026-10-24 (run 40130895764) using our own signing key, not RustDesk's secrets. |
| 6 | Tailscale's peer list can be read without administrator rights on all three desktop OSes | **True on Windows (`win1`) and macOS.** Linux is not a target device. | 2026-10-24: a normal user ran `tailscale status --json` on `win1` with no elevation. 2026-10-25: the Mac app lists the other two computers from the same command. The Mac GUI binary must be started with `TAILSCALE_BE_CLI=1` or it prints nothing. |
