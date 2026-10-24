# Architecture

q2crazyDesk is a RustDesk fork. New logic lives in the `q2/` crates. Upstream crate names stay unchanged so RustDesk releases can be merged (see `docs/UPDATING.md`).

## Components

```
Flutter UI (canvas editor, device list, tray)
        |
        | flutter_rust_bridge
        v
rustdesk (src/)  ---- peer connection (input, clipboard, files, audio, video)
        |
        +-- q2-layout   millimetre canvas, edge links, profiles
        +-- q2-router   who is controller, where the cursor is, guards, hotkeys
        +-- q2-keys     modifier maps and Swedish layout translation
        +-- q2-sync     last-writer-wins layout/settings documents
        +-- q2-net      Tailscale peers, 100.64.0.0/10 filter
        +-- q2-wake     magic packet + relay request
        +-- q2-display  virtual displays (Windows VDD, macOS CGVirtualDisplay)
        +-- q2-dnd      cross-machine file drag protocol
```

Third-party trees (phase 2 and 5): `third_party/lan-mouse`, `third_party/macos-virtual-display`.

## Data flow

1. **Discovery.** `q2-net` reads Tailscale (`tailscale status --json` or the LocalAPI) and matches `100.x` addresses to paired devices.
2. **Session.** Each pair keeps one RustDesk peer connection on the direct-IP port (21118), allowed only from `100.64.0.0/10`.
3. **Beside.** Video is unsubscribed (`Misc::CaptureDisplays` with an empty set). The controller captures the cursor at linked edges (`input-capture`) and forwards input through the same connection. The target injects via RustDesk's existing input path.
4. **Inside.** Video is turned on in that connection. The remote desktop is a window, fullscreen on a chosen monitor, or on a specific monitor. The region's border is an exit back to the host.
5. **Lent.** The borrower creates a virtual display (`q2-display`) sized to the lender's monitor, places it in the OS arrangement, and the lender opens a fullscreen Inside view of that display only.
6. **Sync.** Layout edits bump a version counter (`q2-sync`). Peers apply a document only when its version is newer.

## Transport decision (R-INPUT-3)

**Intended:** one RustDesk connection, video off for Beside. Target added latency on the home network: under 10 ms from controller input to target cursor motion.

**Fallback:** if that path cannot meet the budget or cannot express edge-crossing cleanly, Beside input moves to Lan Mouse's DTLS protocol. RustDesk keeps clipboard, files, audio, and video.

Latency numbers will be written here after the phase 2 measurement on `win1` ↔ `win2`. Until then the decision is "RustDesk input, measure before changing".

## Drag-and-drop design (R-FILES-2)

No mature open-source KVM still does live file drag well (Deskflow removed it; Lan Mouse never shipped it). ShareCursor (MIT/Apache, Rust, pre-release) is the first new candidate; we will read its OLE / `NSDraggingSession` path in phase 6 before writing our own.

Intended pipeline, if we build it:

1. Detect a drag when the cursor leaves a linked edge with a button held.
2. Collect paths (OLE on Windows, drag pasteboard on macOS).
3. Stream the files on the existing RustDesk file-transfer channel.
4. Start a synthetic drag on the target from an invisible window under the cursor (`DoDragDrop` / `NSDraggingSession`).

Protocol types live in `q2-dnd`.

## Lent pipeline (R-PLACE-3)

1. Borrower Y creates a virtual display matching lender monitor M in resolution and physical size.
2. Y positions it with `SetDisplayConfig` (Windows) or `CGConfigureDisplayOrigin` (macOS) to match the canvas.
3. Lender X opens a fullscreen Inside session that shows only that virtual display, on M.
4. Input: if the user is at Y, the cursor walks onto the virtual display as part of Y's desktop. If the user touches X, X becomes the controller (R-DEV-2) and its input goes to Y.
5. Tear-down removes the virtual display and gives M back to X.

macOS constraint: do not change a `CGVirtualDisplay`'s mode after create, or the display can survive process teardown until exit.

## Services

Windows: RustDesk Session 0 service plus the active-session helper, reused so Beside input can reach the login screen and UAC (R-LOGIN-1). The router runs in the privileged helper.

macOS: LaunchDaemon / LaunchAgent as RustDesk already installs, plus menu bar UI.
