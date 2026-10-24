# Sourcing

Open-source survey for every component. Ladder (stop at the first rung that works):

1. Use an existing project as-is.
2. Configure it.
3. Modify it (fork, patch, extend).
4. Port code from another open-source project (keep attribution).
5. Write new code.

Researched 2026-10-24.

### Base app: remote sessions, video, clipboard, files, audio, Windows service, Android, Flutter UI (R-DEV, R-PLACE-2, R-CLIP, R-FILES-1, R-AUDIO, R-LOGIN, R-SVC, R-VIDEO, R-PHONE)
- Candidates considered: RustDesk (AGPL-3.0, Rust+Flutter, actively maintained), Sunshine+Moonlight (GPL-3.0, games, not a desktop product), Deskflow (GPL-2.0-only, input only).
- Chosen: RustDesk 1.5.0, rung 3 (fork as the repository base).
- Why: already covers Inside sessions, clipboard, file transfer, audio, login/UAC via a Session 0 service, Wake-on-LAN, Android, and a Flutter UI. License is the project license.
- What we changed: product docs, CI (`q2-ci.yml`), and q2 crates beside the upstream tree. See `docs/UPSTREAM.md`.

### Edge capture and input emulation (R-INPUT, R-PLACE-1)
- Candidates considered: Lan Mouse crates `input-capture`, `input-emulation`, `input-event` (GPL-3.0-or-later, Rust, v0.11.0 / crates 0.4.0, June 2026, maintained); RustDesk `rdev`/`enigo` (already in-tree); Deskflow (GPL-2.0-only, study only).
- Chosen: Lan Mouse crates, rung 3 (vendor with `git subtree` into `third_party/lan-mouse` in phase 2).
- Why: dedicated edge-capture backends for Windows 11 and macOS, released as libraries. RustDesk already injects input on the target; we will pick capture from Lan Mouse and keep RustDesk injection unless measurements fail R-INPUT-3.
- What we changed: not vendored yet (phase 2). macOS 26 capture is untested on the user's Mac.

### Millimetre canvas and physical edge crossing (R-LAYOUT)
- Candidates considered: LittleBigMouse (GPL-3.0, C#, with a merged Rust hook daemon under `rust/crates`, maintained); Lan Mouse (pixel edges, one rectangle per computer); Deskflow (GPL-2.0-only, study only).
- Chosen: port the millimetre geometry and crossing model from LittleBigMouse's Rust daemon, rung 4, into `q2-layout`.
- Why: it is the only maintained model that places monitors in real millimetres, reads EDID physical size, and maps a crossing by physical position rather than by stretching one rectangle per computer.
- What we changed: a new pure-Rust crate with the crossing maths and tests. The Win32 hook itself is not copied; capture stays in Lan Mouse / RustDesk.

### KVM usability: jump hotkeys, corner dead zones, fullscreen guard, cursor without a mouse (R-INPUT-4, R-INPUT-5, R-INPUT-6)
- Candidates considered: PowerToys Mouse Without Borders (MIT, C#, maintained); Lan Mouse (no cursor-draw helper); RustDesk (no KVM guards).
- Chosen: port the behaviour (not the C# UI) from Mouse Without Borders, rung 4, into `q2-router`.
- Why: MIT, documents the Windows "invisible cursor without a mouse" problem, and already has corner blocking, a fullscreen guard, and per-machine hotkeys.
- What we changed: Rust state machine in `q2-router`.

### Virtual display on Windows (R-PLACE-3)
- Candidates considered: Virtual-Display-Driver (MIT, C++ IddCx 1.10, signed via SignPath, silent-install script, XML/PowerShell for resolution); SudoVDA (MIT/CC0); RustDesk's bundled Amyuni `usbmmidd_v2` and `RustDeskIddDriver`; Parsec VDD (has a clean add/remove/resize CLI, but is not the signed driver we will ship).
- Chosen: ship the signed Virtual-Display-Driver release, rung 1–2, controlled from `q2-display`. Keep RustDesk's existing driver path untouched for Inside sessions.
- Why: signed, silent-installable, IddCx 1.10. Runtime add/remove/resize is done by editing `vdd_settings.xml` and restarting the device, or through the project's PowerShell scripts — not as instant as Parsec's `vdd -a`, which we record as a risk (assumption 3).
- What we changed: none yet (phase 5). CI will download a pinned release and verify a checksum; binaries are not committed.

### Virtual display on macOS (R-PLACE-3)
- Candidates considered: DeskPad (MIT, Swift, `CGVirtualDisplay`); VirtualDisplayKit (MIT, Swift package forked from DeskPad); go-macos/virtualdisplay (measured on macOS 26.6.2); SimpleDisplay (GPL-3.0, reference only).
- Chosen: small Swift shim based on VirtualDisplayKit / DeskPad, rung 3–4, in `third_party/macos-virtual-display` (phase 5).
- Why: `CGVirtualDisplay` still creates and removes displays on macOS 26. DeskPad's mirroring path needs ScreenCaptureKit on the macOS 15+ SDK, but we only need the virtual display itself — RustDesk already captures it. Mode changes can make a display un-removable until process exit; the shim must never change mode after create (recorded in `docs/DECISIONS.md`).
- What we changed: none yet. Needs a physical test on the user's MacBook Air M2.

### Encrypted network and peer discovery (R-NET)
- Candidates considered: Tailscale (separate installed app, LocalAPI + `tailscale status --json`); RustDesk public ID/relay (rejected by R-NET-2); a self-hosted RustDesk server (more work, no gain once Tailscale is there).
- Chosen: Tailscale, rung 1, with glue in `q2-net`.
- Why: encryption at home and away, no port forwarding, free Personal plan. The `tailscale-localapi` crate (MIT) talks to the LocalAPI; the CLI works as a fallback. Admin rights are not required for `status` on a logged-in Windows session; macOS/Linux use the local socket. Confirmed in code reading; live check waits for Tailscale to be installed.
- What we changed: parser and address filter (`100.64.0.0/10`) in `q2-net`.

### Dragging files across machines (R-FILES-2)
- Candidates considered: ShareCursor (MIT/Apache-2.0, Rust, pre-release 2026, chunked drag-and-drop on Windows and macOS); ShareFlow (MIT, Windows/macOS); MyKVM (MIT, clipboard file paste, not live drag); Deskflow / Input Leap / Synergy 1 (GPL-2.0, study only; Deskflow removed drag-and-drop in May 2025); Lan Mouse (none); RustDesk file-transfer channel (copy/paste, not a synthetic drag).
- Chosen: study ShareCursor's drag path (rung 4 if it holds up), otherwise own code over RustDesk's file-transfer channel (rung 5). Decision locked in phase 6 after reading ShareCursor's OLE / `NSDraggingSession` code.
- Why: ShareCursor is the first MIT/Apache Rust project that claims reliable cross-machine drag. It is pre-release, so we do not depend on it as a binary. Deskflow code cannot be copied.
- What we changed: protocol types in `q2-dnd` only, so far.

### System audio on macOS (R-AUDIO-3)
- Candidates considered: RustDesk `screencapturekit` feature (AGPL-3.0, in-tree); ScreenCaptureKit audio; Core Audio process taps.
- Chosen: RustDesk's implementation first, rung 1–2. Revisit if it fails on macOS 26.
- Why: already compiled by `--screencapturekit` in our macOS CI job.
- What we changed: none yet. Needs a physical test on the Mac.

### USB redirection (later, do not build)
- Candidates considered: usbip-win2 (BSD-2).
- Chosen: not built. Design leaves a process/driver slot.
- Why: out of scope (section 6).
- What we changed: none.

### Wake-on-LAN (R-WAKE)
- Candidates considered: RustDesk's existing `wol-rs` usage; a new crate.
- Chosen: RustDesk's magic-packet path plus a relay message in `q2-wake`, rung 2–5.
- Why: the packet format is standard; the new part is "ask a home peer over Tailscale to send it".
- What we changed: packet builder and relay types in `q2-wake`.

### Key mapping and Swedish layout (R-KEYS)
- Candidates considered: Lan Mouse macOS keycode translation; RustDesk keyboard map; own tables.
- Chosen: own tables in `q2-keys` informed by Lan Mouse's macOS translation (rung 4/5).
- Why: no existing crate maps Swedish Windows AltGr versus Mac Option onto each other for a KVM.
- What we changed: `q2-keys` with default Mac↔Windows modifier maps and Swedish character tables.
