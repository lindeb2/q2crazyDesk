# q2crazyDesk: one-shot build prompt

You are an AI coding agent working in Cursor on the user's main Windows PC, inside the folder `C:\Users\Johan\PycharmProjects\q2crazyDesk`. Your job is to take this project from an empty folder to a finished, working, installable application on Windows, macOS, and Android, following this document. Work through every phase until the Definition of Done (section 14) is met. Do not stop after scaffolding or a prototype.

---

## 0. Prime directive: open source first

The user explicitly wants this app assembled mainly from existing open-source projects. Cherry-pick a little here and a little there, merge the parts, and modify them where needed, rather than writing code from scratch. The user believes, and you should assume, that battle-tested open-source code will usually be better than new code. Writing your own code is completely fine where nothing suitable exists, but it is the last resort, not the default.

For every component, follow this ladder and stop at the first rung that works:

1. Use an existing open-source project or library as-is.
2. Configure it, for example through options, flags, or settings.
3. Modify it: fork, patch, or extend it.
4. Port code from another open-source project, including from another language (for example C# or Swift into Rust), keeping attribution.
5. Write new code yourself.

Rules that follow from this:

- **Search before you build.** Before implementing any non-trivial piece, search GitHub, crates.io, pub.dev, and the web for existing implementations, whether that is a whole project, a crate or package, or a single file or function. Record what you found in `docs/SOURCING.md` (format in section 9.4), including when you found nothing.
- **Prefer code that merges easily.** Prefer components in Rust, the language of the base project. Components in other languages are fine as prebuilt binaries, as small FFI shims, or as code ported to Rust.
- **License compatibility.** The app is for the user's personal use only and will not be distributed, so licenses do not block anything. Still keep the codebase cleanly compatible with AGPL-3.0, the license of the base project, in case the user ever shares it:
  - Allowed to copy or merge: MIT, BSD, Apache-2.0, ISC, Zlib, MPL-2.0, LGPL, GPL-3.0(-or-later), and AGPL-3.0.
  - GPL-2.0-only code, such as Deskflow, may be studied for ideas but not copied.
  - Proprietary products may only inspire behaviour.
- **Keep attribution.** Preserve original license headers, add a `THIRD_PARTY_NOTICES.md`, and list every borrowed piece in `docs/UPSTREAM.md` (section 9.3).
- **Keep upstream diffs small and isolated** so that new upstream versions can be merged later (section 9).

---

## 1. The user and how to communicate

- The user writes Swedish. Talk to the user in short, plain Swedish. Write all code, comments, commit messages, UI text, and docs in English.
- The user can read and change code, but is not an expert. Explain decisions briefly when they affect the user.
- The app is for the user only: one person, on their own devices, at home and sometimes away from home.
- Only ask the user for things you cannot do or find out yourself:
  - physical tests at the desk
  - actions on the Mac, the second Windows PC, or the phone
  - signing in to accounts, such as Tailscale
  - approving administrator (UAC) prompts
  - decisions that change the scope
- When you need a physical test, give exact numbered steps and say what result to report back.

---

## 2. Product summary

q2crazyDesk lets one person use several computers as if they were one desk. Every computer has a placement in a shared layout, and the user can switch a computer's placement with one action. There are three placements:

- **Beside** (software KVM, keyboard and mouse sharing): the computer's own monitors sit next to the others. The mouse and keyboard move across screen edges to it, and each computer keeps showing its own desktop. No video is streamed.
- **Inside** (remote desktop): the computer's screen is shown inside another computer's monitor, as a movable window, in fullscreen on that monitor, or on a monitor the user picks. Moving the mouse into it controls that computer, and moving the mouse out through an edge leaves it.
- **Lent** (extra monitor): a computer's physical monitor becomes an extra monitor for another computer. The borrowing computer creates a virtual display, extends its desktop onto it, and the lending computer shows that virtual display in fullscreen. This works in every direction: Windows to Windows, Windows to Mac, and Mac to Windows.

Clipboard, files, and audio work across all of this. The app starts before login and reconnects by itself, can wake sleeping PCs, and works both on the home network and away from home over Tailscale.

### Glossary used in this document

- **Device:** one computer or phone running q2crazyDesk.
- **Controller:** the device whose physical keyboard, mouse, or trackpad the user is touching right now.
- **Target:** the device that currently receives the input.
- **Canvas:** the shared layout of all monitors in millimetres.
- **Placement:** Beside, Inside, or Lent, set per device in the active profile.
- **Profile:** the saved canvas and placements for one combination of online devices.

---

## 3. Hardware, accounts, and environment

### 3.1 Devices

| ID | Device | OS | Monitors | Network | Input |
|---|---|---|---|---|---|
| `win1` | Main PC, where you run | Windows 11 Home, build 26200 | 2 | Ethernet | Physical keyboard and mouse |
| `win2` | Second PC | Windows; check the edition and version with the user | 1 | Ethernet | Has its own; check |
| `mac` | MacBook Air M2 (2022) | macOS; check the version with the user | 1 built-in | Wi-Fi | Built-in keyboard and trackpad |
| `phone` | Android phone | Android | Built-in | Wi-Fi or mobile data | Touch; controller only, lowest priority |

The number of monitors will change. Plugging monitors in and out must be handled while the app runs.

Keyboard layout: assume Swedish on every device, but confirm with the user. Swedish characters must work across platforms (R-KEYS).

### 3.2 State of `win1` when this prompt was written

- **Present:** Git 2.47, the GitHub CLI `gh` (logged in as GitHub user `lindeb2`), Visual Studio Community 2026, CMake, Python 3.11, .NET, and Java.
- **Missing:** Rust (`rustup`, `cargo`), Flutter, vcpkg, LLVM/Clang, and Tailscale.

Install what the build needs. Prefer the versions that RustDesk's own CI pins (section 10). Tell the user before large installs, but do not wait for permission for standard developer toolchains.

### 3.3 Accounts

- **GitHub:** the user has an account, and `gh` is authenticated on `win1`. Create a private repository `lindeb2/q2crazyDesk`.
- **Tailscale:** the user will create or sign in to a personal account when you ask. Use a personal identity, such as a Gmail or GitHub login, so the tailnet stays on the free Personal plan.

---

## 4. Component map: where to start

This is the result of research done before writing this prompt. Verify each item, because versions and details move, and look for better options before committing to one. Every row needs an entry in `docs/SOURCING.md`.

| Need | Starting candidate | License | Language | How to use it |
|---|---|---|---|---|
| Base app: remote sessions, video, clipboard, file transfer, audio, Windows service with login-screen and UAC control, LAN discovery, Wake-on-LAN, 4:4:4 colour, remote monitors as separate windows, multiple sessions, Android app, Flutter UI | [RustDesk](https://github.com/rustdesk/rustdesk) | AGPL-3.0 | Rust and Flutter | **Fork it as the base of this repository**, keeping its git history |
| Capturing the cursor at screen edges and emulating input on Windows and macOS; DTLS input transport | [Lan Mouse](https://github.com/feschber/lan-mouse) crates `input-capture`, `input-emulation`, `input-event` | GPL-3.0-or-later | Rust | Vendor with `git subtree` into `third_party/lan-mouse` and adapt |
| Physical millimetre layout; DPI-aware edge crossing; reading the monitor's physical size from EDID; border resistance; rescue shortcut | [LittleBigMouse](https://github.com/mgth/LittleBigMouse) | GPL-3.0 | C#, with a newer Rust port of its hook daemon (PR #491) | Port the geometry and crossing logic to Rust; check the Rust hook-daemon code first |
| KVM usability: hotkey per machine, blocking corners, guard when an app is fullscreen, drawing the cursor on machines without a mouse | PowerToys [Mouse Without Borders](https://github.com/microsoft/PowerToys) | MIT | C# | Port the logic or ideas |
| Virtual display on Windows (Lent placement) | [Virtual-Display-Driver](https://github.com/VirtualDrivers/Virtual-Display-Driver) (IddCx 1.10, signed through SignPath) | MIT | C++ driver | **Ship its signed release.** Do not build the driver yourself, because unsigned drivers will not load. Control it through its documented interface. Compare it with [SudoVDA](https://github.com/SudoMaker/SudoVDA) (MIT/CC0) and RustDesk's own IddCx driver |
| Virtual display on macOS (Lent placement) | [DeskPad](https://github.com/Stengo/DeskPad) or [VirtualDisplayKit](https://github.com/xocialize/VirtualDisplayKit), which uses the private `CGVirtualDisplay` API | MIT | Swift | Small Swift shim called through FFI. Reference only: SimpleDisplay (GPL-3.0) |
| Encrypted network home and away; finding peers | Tailscale, installed on every device | Separate installed app | n/a | Read peers from `tailscale status --json` or the LocalAPI; connect to `100.x` addresses |
| Dragging files across machines | No good open-source implementation found: Deskflow removed drag-and-drop in May 2025, and Lan Mouse removed its half-finished version | n/a | n/a | Research again, including old Synergy 1 and Input Leap code (GPL-2.0, study only). Most likely you build this yourself (R-FILES) |
| Capturing system audio on macOS | RustDesk's implementation | AGPL-3.0 | Rust | If it fails on newer macOS, research ScreenCaptureKit audio or Core Audio process taps |
| USB redirection (*later, do not build now*) | [usbip-win2](https://github.com/vadimgrn/usbip-win2) | BSD-2 | C++ | Only make the design able to take it later |

**Explicitly not used:**

- Deskflow, Input Leap, and Barrier code: GPL-2.0, so ideas only.
- Sunshine and Moonlight: games are not a goal, and RustDesk covers remote desktop.
- Parsec, AnyDesk, TeamViewer, Splashtop, Jump Desktop, Synergy 3, ShareMouse, and Multiplicity: proprietary, so behaviour may only inspire the design.

---

## 5. Functional requirements

Every requirement has an ID. Refer to these IDs in commits, tests, and `docs/MANUAL_TESTS.md`.

### R-DEV: devices and roles

- **R-DEV-1:** `win1`, `win2`, and `mac` can each be both a controller and a target. `phone` is only a controller and opens Inside-style sessions (R-PHONE).
- **R-DEV-2:** Only one person uses the system. Exactly one controller is active at a time: the device with the most recent physical input event wins. If the user touches the Mac trackpad while `win1` is driving `win2`, control moves to the Mac at once.
- **R-DEV-3:** Devices pair once. The user approves the pairing on both sides, and devices are trusted from then on. Unpaired devices can never connect.
- **R-DEV-4:** The GUI shows every device's status: online, offline, or asleep (wakeable).

### R-PLACE: placements

- **R-PLACE-1, Beside:** the device's monitors are part of the canvas. The cursor crosses into them through linked edges, and the device shows its own desktop. No video is streamed.
- **R-PLACE-2, Inside:** the device's screen or screens are rendered on a monitor of another device. Three modes can be chosen per placement:
  1. a movable, resizable window
  2. fullscreen on a chosen monitor of the host
  3. on a specific monitor the user picks

  Pick a sensible default (windowed), and make it configurable.
- **R-PLACE-3, Lent:** monitor M of device X becomes an extra monitor for device Y.
  - Y creates a virtual display that matches M's resolution and physical size, and places it in Y's OS display arrangement according to the canvas.
  - X shows only that virtual display in fullscreen on M.
  - The cursor reaches it as an ordinary part of Y's desktop, and windows can be dragged onto it.
  - Every direction must work: Windows to Windows, Windows to Mac, and Mac to Windows.
  - While M is lent, X's own desktop is not visible on M. X's other monitors, if any, keep working.
- **R-PLACE-4:** A device has exactly one placement in the active profile at any time.
- **R-PLACE-5:** Changing a placement is a single action in the GUI and applies live. Reuse the existing connection where possible (section 8.2) and avoid reconnect delays.

### R-LAYOUT: canvas in millimetres

- **R-LAYOUT-1:** The canvas is in millimetres. Every physical monitor is drawn at its true physical size and dragged to its real position, including offsets, gaps, and different heights. Snapping helps, but free placement must be possible. This is LittleBigMouse's model, extended across computers.
- **R-LAYOUT-2:** The physical size is detected automatically: from EDID on Windows, and from `CGDisplayScreenSize` or similar on macOS. The user can override it manually per monitor.
- **R-LAYOUT-3:** Edge crossing maps by physical position. A point 100 mm from the top of one monitor arrives 100 mm from the top of the neighbour. This is aware of DPI and scaling (Retina points versus pixels, and Windows scale factors), and the cursor speed in mm/s stays consistent across monitors as far as practical.
- **R-LAYOUT-4:** The same physically correct crossing also applies between the monitors of a single computer, at least between `win1`'s two monitors. This is what LittleBigMouse does locally.
- **R-LAYOUT-5:** Only edge segments that touch another screen in the canvas are exits. All other edges are walls.
- **R-LAYOUT-6:** An Inside placement is drawn as a rectangle inside its host monitor. Its border is an exit back to the host. In fullscreen Inside on monitor M, the exits are exactly the linked edges of M in the canvas: towards other devices, or towards the host's other monitors.
- **R-LAYOUT-7:** The layout can be edited from any desktop device and is synced to all of them.
  - Conflicts: last writer wins, using a version counter.
  - A device that was offline receives the latest layout when it reconnects.
- **R-LAYOUT-8, profiles:** a profile is keyed by the set of devices that take part.
  - When a combination appears, the layout and placements last used for exactly that combination are restored.
  - For a combination never seen before, start from the closest earlier profile and save it.

### R-INPUT: input routing

- **R-INPUT-1:** Forward the following, with no stuck keys or buttons: relative and absolute motion, all mouse buttons, vertical and horizontal wheel including smooth high-resolution scrolling, the full keyboard including modifiers, media keys and key repeat, and Ctrl+Alt+Del or the secure attention sequence to Windows targets. Release everything held whenever control switches or a connection drops.
- **R-INPUT-2:** Entering an Inside region forwards input to the target, with absolute coordinates mapped onto the target's screen, and hides the local cursor. Leaving through the region's border returns control to the host.
- **R-INPUT-3:** Latency budget in Beside mode on the home network: aim for less than 10 ms added from the controller's input to the target's cursor movement. Measure it and record the numbers in `docs/ARCHITECTURE.md`.
- **R-INPUT-4, switch guards** (each optional, user-configurable):
  - dead zones in the corners
  - require holding a modifier key to cross
  - block crossing while the foreground app is fullscreen, with an ignore list
- **R-INPUT-5, jump hotkeys:** a configurable shortcut per device moves control straight to that device, to the centre of its primary monitor. This also works as the way out if the cursor is ever stuck.
- **R-INPUT-6:** A target with no physical mouse must still show a cursor. On Windows the cursor can be invisible without a mouse, as Lan Mouse documents; see Mouse Without Borders' "draw mouse cursor" for one way to solve it.

### R-KEYS: keyboard mapping

- **R-KEYS-1:** Key mapping is configurable per controller and target pair.
- **R-KEYS-2:** Default mapping:

  | Direction | Mapping |
  |---|---|
  | Mac controlling Windows | Cmd → Ctrl, Control → Win, Option → Alt |
  | Windows controlling Mac | Ctrl → Cmd, Win → Control, Alt → Option |

  Every mapping can be edited in the GUI.
- **R-KEYS-3:** Swedish layout correctness. From every controller into every target, typing `å ä ö Å Ä Ö @ £ $ € { } [ ] | \ ~ ^ ´ ¨` and dead-key combinations must produce the right characters.
  - This is hard because, for example, `@` is AltGr+2 on Windows but Option+2 on a Swedish Mac.
  - Design the mapping so that it works, for example with a layout-aware translation layer or an optional text-forwarding mode, and cover it with tests.

### R-GEST: trackpad gestures (simple form)

- **R-GEST-1:** Two-finger scrolling from the Mac trackpad is smooth on every target, horizontally as well as vertically.
- **R-GEST-2:** On Windows targets, pinch becomes Ctrl+wheel. On Mac targets, pinch stays a pinch where possible.
- **R-GEST-3:** Other multi-finger gestures are *later* (section 6). Do not build them, but do not design them out.

### R-CLIP and R-FILES: clipboard and files

- **R-CLIP-1:** The clipboard is shared in both directions between all desktop devices: plain text, rich text, images, and files.
  - Large content is transferred lazily, when it is pasted, or in the background without blocking input.
  - The size limit is configurable, with a generous default.
- **R-FILES-1:** Copying and pasting files works between every pair of desktop devices, with progress shown for large files.
- **R-FILES-2:** **Dragging files** across a Beside edge onto the other computer's desktop or into an app window works in every direction between `win1`, `win2`, and `mac`. Dragging into and out of Inside windows works too.
  - This is the hardest custom feature: no open-source tool does it well today. Research first; if nothing fits, build it.
  - Likely approach: detect a drag in progress when the cursor leaves through an edge with a button held. Collect the file paths with OLE drag-and-drop on Windows or the drag pasteboard on macOS. Stream the files through the session's file-transfer channel. Then start a synthetic drag on the target from an invisible window under the cursor (`DoDragDrop` on Windows, `NSDraggingSession` on macOS).
  - Record the design in `docs/ARCHITECTURE.md`.

### R-AUDIO: audio routing

- **R-AUDIO-1:** For each device, the user chooses where its audio plays:
  - on its own speakers or headphones (the default for Beside), or
  - forwarded to the device the user is currently at (the controller).
- **R-AUDIO-2:** Inside sessions forward audio by default, configurable, with audio and video kept in sync well enough to watch a 1080p film.
- **R-AUDIO-3:** Audio capture on macOS must work on the user's macOS version. RustDesk has known problems with newer macOS (section 13).

### R-LOGIN: login screen and administrator prompts

- **R-LOGIN-1:** On Windows targets, the user can control the login screen, the lock screen, and UAC prompts on the secure desktop, in both Beside and Inside placements. RustDesk's service architecture (a service in Session 0 that starts a helper in the active session) already does this for remote sessions. Reuse it for Beside input as well.
- **R-LOGIN-2:** macOS login window: best effort, nice to have.

### R-SVC: start-up and automatic connection

- **R-SVC-1:** On Windows the app runs as a service that starts at boot, before anyone logs in. On macOS it runs as a LaunchDaemon or LaunchAgent, as appropriate. The UI is a tray (Windows) or menu bar (macOS) app plus a main window.
- **R-SVC-2:** A per-device setting "auto-connect" decides whether, after every start (not only after a restart), the device joins the system by itself and takes its placement from the matching profile.
- **R-SVC-3:** The app reconnects automatically, with backoff, after network loss, sleep and resume, or a peer restarting.

### R-WAKE: Wake-on-LAN

- **R-WAKE-1:** The app stores each device's network MAC address. The GUI has a "Wake" action on asleep or offline devices.
- **R-WAKE-2:** If the requesting device is on the same home network, it sends the magic packet directly. If it is away, it asks an online device at home to send it, over Tailscale.
- **R-WAKE-3:** `win1` and `win2` are on Ethernet and must be wakeable from sleep, and from shutdown where the hardware allows. The Mac is on Wi-Fi: support waking it from sleep through macOS "Wake for network access" as best effort, and say clearly in the docs that waking it from shutdown is not possible.
- **R-WAKE-4:** Setup docs explain:
  - enabling Wake-on-LAN in the BIOS/UEFI
  - "Wake on Magic Packet" in the network adapter settings
  - turning off Windows Fast Startup, which often blocks waking from shutdown
  - the Mac energy setting

### R-NET: networking and security

- **R-NET-1:** All traffic between devices goes over Tailscale `100.x` addresses, both at home and away. Tailscale provides the encryption.
- **R-NET-2:** Do not depend on RustDesk's public ID or relay servers, and do not require port forwarding.
- **R-NET-3:** Use RustDesk's direct IP access (default port 21118), restricted with its IP whitelist to Tailscale's range `100.64.0.0/10`. RustDesk's direct LAN connections are unencrypted, which is why they are allowed only through Tailscale.
- **R-NET-4:** Discover peers automatically through Tailscale (`tailscale status --json` or the LocalAPI) and match them to paired devices. MagicDNS names are fine.
- **R-NET-5:** If Tailscale is not running, show a clear error that says how to fix it. Offer an optional "allow unencrypted LAN fallback" setting, off by default.
- **R-NET-6:** Setup docs explain:
  - installing Tailscale on every device
  - running it as a service that starts before login (unattended mode on Windows)
  - turning off key expiry for the always-on PCs in the Tailscale admin console

### R-VIDEO: Inside sessions

- **R-VIDEO-1:** 1080p at 60 fps is the target quality. 4K and high frame rates are not goals.
- **R-VIDEO-2:** 4:4:4 colour is on by default for sharp text, using a codec that supports it (in RustDesk: VP9 or AV1).
- **R-VIDEO-3:** All monitors of a remote computer can be viewed at once, either as separate windows or spread across local monitors. RustDesk has the settings `displays-as-individual-windows` and `use-all-my-displays-for-the-remote-session`.
- **R-VIDEO-4:** Several remote devices can be open as Inside at the same time.

### R-PHONE: Android

- **R-PHONE-1:** The Android app lists devices and can open a fullscreen session to any computer, with RustDesk's touch and trackpad input modes. It is not part of the Beside canvas.
- **R-PHONE-2:** It works at home and away over Tailscale.

### R-UI: interface

- **R-UI-1:** The interface is in English.
- **R-UI-2:** The main window contains:
  - **Canvas editor:** monitors drawn to physical scale, drag to position, edge links shown, Inside rectangles, and Lent links.
  - **Device list** with status.
  - **Per-device settings:** placement and its mode, audio routing, key mapping, auto-connect, MAC address and the Wake action, jump hotkey, and switch guards.
  - **Active profile indicator.**
- **R-UI-3:** Suggested interaction, which may be refined: drag a device's tile next to a monitor for Beside, drop it into a monitor for Inside, or drop it onto "lend to" for Lent.
- **R-UI-4:** The tray or menu bar menu offers quick placement switching per device, Wake, and opening the editor.

---

## 6. Out of scope and later

**Out of scope.** Do not build these:

- matching virtual display size for remote sessions
- 4K or high frame rates
- privacy mode (blanking the remote screen)
- remote printing
- session recording
- locking the cursor to one computer
- locking all computers at once
- dimming inactive screens
- sending the same input to all computers at once
- switching a shared monitor's input over DDC
- every team or business feature: user management, SSO, chat, whiteboard, sharing over a web link, branding
- gamepad support
- iPhone support

**Later.** Do not build these now, but design so that they can be added:

- USB redirection (usbip-win2 on Windows; on Mac it is blocked by Apple's driver entitlements)
- microphone passthrough to the remote device (needs a virtual microphone driver on Windows)
- full multi-finger trackpad gestures on Windows targets

---

## 7. Assumptions you must verify early

Check each of these in phase 0 or 1, and record the result in `docs/DECISIONS.md`. If one is false, choose the best alternative that fits the user's priorities, record it, and tell the user briefly.

1. RustDesk's connection can run in an input-only mode with the video encoder stopped, and add little latency on the home network. If it cannot, see the fallback in section 8.2.
2. Lan Mouse's `input-capture` and `input-emulation` crates work on Windows 11 and on the user's macOS version, and can be embedded as libraries rather than as the full app.
3. Virtual-Display-Driver can be installed silently, and displays can be added, removed, and resized from code at runtime.
4. `CGVirtualDisplay` works on the user's macOS version.
5. RustDesk's Android app builds in CI from the fork without RustDesk's private secrets.
6. Tailscale's peer list can be read without administrator rights on all three desktop operating systems.

---

## 8. Target architecture

### 8.1 Repository layout (suggested)

- **The repository is a fork of RustDesk with its history.** Leave RustDesk's internal crate and module names unchanged, which keeps upstream merges easy. Change the product name, app IDs, icons, and installer names to q2crazyDesk through RustDesk's existing branding and config mechanisms, with as few edits to upstream files as possible.
- **New code goes in its own crates** under `q2/`:

  | Crate | Purpose |
  |---|---|
  | `q2-layout` | Millimetre canvas, monitor model, edge links, physical crossing maths, Inside regions, Lent links, profiles. Pure logic with heavy unit tests. |
  | `q2-router` | State machine for input routing: who the controller is, where the cursor is, transitions, guards, hotkeys, key release on switch. |
  | `q2-keys` | Key mapping profiles and layout-aware translation (R-KEYS). |
  | `q2-sync` | Syncing layout, profiles, and settings between peers. |
  | `q2-display` | Virtual display providers: Windows VDD control, and macOS through a Swift shim. |
  | `q2-net` | Tailscale peer discovery and pairing glue. |
  | `q2-wake` | Wake-on-LAN, including the relay. |
  | `q2-dnd` | Cross-machine drag-and-drop of files. |
- **Third-party code:** `third_party/lan-mouse` (git subtree), and `third_party/macos-virtual-display` for the Swift shim based on DeskPad or VirtualDisplayKit.
- **UI:** new Flutter pages (the canvas editor and device settings) inside RustDesk's Flutter app, under a separate folder.
- **Docs:** `docs/`, see section 14.

### 8.2 One connection, two modes (intended design)

- Each pair of devices keeps one RustDesk peer connection, carrying input, clipboard, files, and audio.
- **Beside** uses this connection with no video. **Inside** turns video on in the same connection. Switching placement only toggles video and changes the routing, so it is instant.
- Edge capture on the controller uses Lan Mouse's `input-capture`. Input injection on the target uses RustDesk's existing input path, or Lan Mouse's `input-emulation`, whichever proves better. Document the choice.
- **Fallback:** if measurements show that RustDesk's input path cannot meet R-INPUT-3, or cannot express edge-crossing semantics cleanly, carry Beside input over Lan Mouse's DTLS protocol. RustDesk keeps clipboard, files, audio, and video. Record the decision with numbers.

### 8.3 Lent screen pipeline

1. The borrower Y creates a virtual display through `q2-display`, sized to the lender's monitor M in both resolution and physical size.
2. Y positions the virtual display in its OS display arrangement to match the canvas: `SetDisplayConfig` on Windows, `CGConfigureDisplayOrigin` on macOS.
3. The lender X opens a fullscreen Inside-type session to Y that shows only that virtual display, on M.
4. Input: if the user is at Y's own keyboard, the cursor simply moves onto the virtual display. If the user touches X's own input, X becomes the controller (R-DEV-2) and its input goes to Y, because M shows Y.
5. Tear-down removes the virtual display and gives M back to X.

### 8.4 Services and processes

- Reuse RustDesk's Windows service with its active-session helper, and its macOS launch setup.
- The input router runs in the privileged helper, so it can reach the secure desktop on Windows.

---

## 9. Repository and upstream management

### 9.1 Bootstrapping the fork

The folder already holds a git repository containing only this file. Bring RustDesk's history in, for example:

```bash
git remote add upstream https://github.com/rustdesk/rustdesk.git
git fetch upstream --tags
git checkout -B main <latest stable RustDesk tag>
git submodule update --init --recursive
```

Put `ONESHOT_PROMPT.md` back at the repository root and commit it. Then create the private GitHub repository `lindeb2/q2crazyDesk` with `gh` and push.

### 9.2 Vendoring

- Lan Mouse: `git subtree add --prefix third_party/lan-mouse https://github.com/feschber/lan-mouse.git <tag or commit> --squash`.
- Code ported from LittleBigMouse, Mouse Without Borders, and DeskPad or VirtualDisplayKit: keep a header in every ported file that names the source repository, commit, path, and license.
- Prebuilt binaries, such as the signed Virtual-Display-Driver release: download them in CI at a pinned version and verify a checksum. Do not commit large binaries.

### 9.3 `docs/UPSTREAM.md`

For each borrowed component, list:

- the source repository
- the version or commit
- the license
- what was taken
- where it lives in this repository
- a short list of local modifications, the "patch list"

Also write `docs/UPDATING.md`, a step-by-step procedure for merging a new RustDesk release and updating the subtree. Keep edits to upstream files minimal and grouped, so that merges stay small.

### 9.4 `docs/SOURCING.md`

One entry per feature or component, in this format:

```
### <Feature / component> (R-IDs)
- Candidates considered: <name, link, license, language, maintenance status> ...
- Chosen: <candidate or "own code">, rung <1-5> of the ladder
- Why: <short reasoning>
- What we changed: <short>
```

---

## 10. Build and CI

- **GitHub Actions builds every platform.** Start from RustDesk's own workflows (for example `.github/workflows/flutter-build.yml`) and trim them to:
  - **Windows x64:** installer and portable build.
  - **macOS arm64:** a `.dmg`.
  - **Android arm64:** an APK.
- Remove or disable steps that need RustDesk's private secrets or upload to their servers. Builds run on every push to `main` and on pull requests. Pushing a `v*` tag creates a GitHub Release with all artifacts attached.
- **Pin toolchains** to what RustDesk's CI uses: the Flutter version, the Rust toolchain, the vcpkg commit, and the NDK.
- **Run the q2 crates' unit tests in CI** on Windows, macOS, and Linux.
- **macOS signing:** there is no paid Apple developer account.
  - Ad-hoc signing changes the app's code identity on every build, so macOS privacy permissions may have to be granted again after every update.
  - Prefer a stable self-signed certificate that the user creates once on the Mac, or document the re-grant steps clearly.
  - The setup docs must include the quarantine fix: `xattr -d com.apple.quarantine /Applications/q2crazyDesk.app`.
- **Windows:** the installer is unsigned, so expect a SmartScreen warning and document it. The installer installs the service and the signed Virtual-Display-Driver.
- **Local builds:** set up a local Windows build on `win1` for fast iteration. Mac and Android builds come from CI unless the user sets up a local Mac build.

---

## 11. Phased plan with acceptance criteria

Keep this order, which the user agreed to. Run the automated tests at the end of every phase, push, and make sure CI is green. Then give the user the matching manual test steps and wait for the results where a physical test is needed. Fix what fails before moving on, unless a later phase does not depend on it.

**Phase 0: bootstrap and survey**
- Toolchains installed. The fork created as described in section 9.1. RustDesk unmodified builds locally on `win1` and in CI for all three targets.
- `docs/` skeleton created. `docs/SOURCING.md` contains the first survey of every row in section 4. The assumptions in section 7 are checked as far as possible without the other devices.
- *Done when:* CI is green with three artifacts and the local Windows build runs.

**Phase 1: devices, network, service**
- Rebranding. Tailscale discovery, pairing, and direct IP restricted to Tailscale. Auto-start before login, auto-connect, reconnect.
- *Done when:* all three desktop devices see each other as online in the GUI after a cold boot without anyone logging in, on Windows at least, and reconnect after unplugging the network cable.

**Phase 2: Beside across all three computers**
- `q2-layout` with the millimetre model, physical crossing, and `win1`'s local multi-monitor crossing. `q2-router`, `q2-keys` including the Swedish layout, simple gestures, switch guards, jump hotkeys, the clipboard, and key release on switch.
- *Done when:* the cursor crosses between `win1`'s two monitors, `win2`, and the Mac at physically correct positions. Typing works in every direction, including Swedish characters. Last input wins. The latency is measured and under budget.

**Phase 3: Inside**
- Window, fullscreen, and chosen-monitor modes. Entering and leaving by moving the mouse, with fullscreen exits through linked edges. All remote monitors at once, several sessions at once, and 4:4:4.
- *Done when:* the user can move from a Beside layout into an Inside window and back without clicking, and a 1080p film plays smoothly with sound in an Inside window.

**Phase 4: canvas editor, profiles, sync, live switching**
- Full editor UI as in R-UI. Profiles per device combination. Syncing. A placement change in one action.
- *Done when:* editing on any device updates all devices; switching a device between Beside and Inside takes one action and under about 1 second; and unplugging and reconnecting a device restores the right profile.

**Phase 5: Lent**
- Virtual displays on Windows and macOS, and the Lent pipeline in every direction.
- *Done when:* the Mac's screen works as an extra monitor for `win1`, `win1`'s monitor works as an extra monitor for the Mac, and `win2` works as an extra monitor for `win1`. Windows can be dragged onto the lent screen, and tear-down restores everything.

**Phase 6: files, audio, login, wake**
- Copying and pasting files, dragging files across edges and Inside windows, audio routing, the login screen and UAC, Wake-on-LAN with the relay.
- *Done when:* every R-FILES, R-AUDIO, R-LOGIN, and R-WAKE requirement passes its manual test.

**Phase 7: away from home and Android**
- Hardening of remote use over Tailscale. The Android app.
- *Done when:* from the phone on mobile data, the user can wake `win1`, open it as a session, and use it; and from the Mac on another network, Inside sessions to `win1` and `win2` work.

**Phase 8: polish and release**
- Docs complete, `docs/MANUAL_TESTS.md` run in full by the user, a `v1.0.0` tag with installers.

---

## 12. Testing strategy

- **Unit tests:**
  - `q2-layout`: property-based tests for the geometry, including round-trips, continuity across edges, and DPI cases.
  - `q2-router`: transitions, guards, key release, last-input-wins.
  - `q2-keys`: tables for every controller and target pair and for the Swedish characters.
  - Profile selection and sync merging.
- **Integration tests on one machine:** run two or more app instances in a headless test mode with fake input-capture and input-emulation backends and loopback networking. Test the protocol, routing, clipboard, file transfer, and sync. Run these in CI.
- **Manual tests:** write `docs/MANUAL_TESTS.md`, a numbered checklist where every R-ID has at least one physical test, with exact steps and the expected result. At the end of each phase, give the user the relevant subset in Swedish in chat, and record the results in the file.
- **Diagnostics:** an in-app "Diagnostics" view showing:
  - connection state per peer and the transport used
  - measured input latency
  - the active profile
  - the virtual display state
  - recent errors

  Add a one-click option to export logs, which makes remote debugging with the user possible.

---

## 13. Known pitfalls

**macOS**
- **Permissions:** the app needs Accessibility, Input Monitoring, and Screen Recording. Updates can silently break these permissions; the fix is to remove and re-add the app in System Settings. Detect missing permissions and guide the user.
- **Unsigned app:** quarantine and Gatekeeper; see section 10 for code identity and permissions.
- **Wi-Fi stutter:** AWDL can cause stuttering on a Mac on Wi-Fi. Synergy and Deskflow both document this; one workaround is `sudo ifconfig awdl0 down`. Offer a toggle or document it.
- **Virtual displays:** `CGVirtualDisplay` is private. It can break with macOS updates and may be capped at 60 Hz. Handle failure gracefully.
- **Audio capture:** RustDesk has had system audio issues on newer macOS. Verify early on the user's version.

**Windows**
- **No mouse attached:** the cursor can be invisible on a target with no physical mouse (R-INPUT-6).
- **Secure desktop:** UAC prompts and the login screen need the Session 0 service plus an active-session helper.
- **Virtual-Display-Driver:** installing it needs administrator rights. Use the signed release only.

**Cross-platform**
- **One rectangle per computer is wrong:** do not treat a computer with several monitors as a single rectangle, as Deskflow and Synergy do. This app handles monitors individually.
- **Swedish layout:** AltGr versus Option for `@ { } [ ] | \ ~ $ €` (R-KEYS-3).
- **Stuck keys:** modifiers stuck after a switch or disconnect. Always send releases.
- **Scaling:** Retina points versus pixels, and Windows per-monitor DPI. Every coordinate conversion goes through `q2-layout`.

**Network and upstream**
- **Tailscale key expiry:** keys expire after 180 days by default. The docs must tell the user to disable expiry on the always-on PCs.
- **Unencrypted direct IP:** RustDesk's direct IP is unencrypted, so allow it only through Tailscale (R-NET-3).
- **Large upstream merges:** keep the patch list current so that RustDesk upgrades stay feasible.

---

## 14. Deliverables and Definition of Done

**Deliverables in the repository**

- The working app for Windows x64, macOS arm64, and Android, built by CI and attached to a GitHub Release.
- `README.md`: what the app is and a quick start.
- `docs/SETUP.md`: per-device setup for Tailscale, installing the app, macOS permissions and the quarantine fix, Wake-on-LAN in the BIOS and network adapter, Fast Startup, the Mac energy setting, and pairing.
- `docs/USER_GUIDE.md`: placements, the canvas editor, profiles, hotkeys, audio routing, key mapping, Wake, and remote use away from home.
- `docs/ARCHITECTURE.md`: components, data flow, the transport decision with latency numbers, the drag-and-drop design, and the Lent pipeline.
- `docs/SOURCING.md`, `docs/UPSTREAM.md`, `docs/UPDATING.md`, `docs/DECISIONS.md`, `docs/MANUAL_TESTS.md` with results, and `THIRD_PARTY_NOTICES.md`.

**Definition of Done.** The work is finished when all of these hold:

1. Every requirement in section 5 is implemented, with no stubs and no placeholder TODOs in shipped code paths.
2. Every requirement is covered by an automated test where that is feasible, and by a manual test in every case.
3. CI is green on every platform, and the `v1.0.0` release contains all three artifacts.
4. The user has run `docs/MANUAL_TESTS.md` in full, and every test passes.
5. `docs/SOURCING.md` has an entry for every component, showing that open source was checked first.
6. `docs/UPSTREAM.md` lists every piece of third-party code with its version and license.

---

## 15. Working rules

- Plan with a todo list. Work phase by phase. Commit small, meaningful steps with clear messages that reference R-IDs, push often, and keep CI green.
- Research before building (section 0). When this document is wrong or out of date, trust what you verify, record the correction in `docs/DECISIONS.md`, and continue.
- Do not reduce the scope on your own. If something proves infeasible, present the alternatives to the user in Swedish with your recommendation, and continue with other work while you wait.
- Keep the user informed with short Swedish progress notes at the end of each phase: what works, what to test, and what comes next.
- Never expose the system outside Tailscale. Never commit secrets.
