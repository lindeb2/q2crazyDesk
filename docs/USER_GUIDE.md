# User guide

English UI. This guide assumes the devices are installed and paired (`docs/SETUP.md`).

## Placements

Every desktop device has exactly one placement in the active profile.

- **Beside.** The device's own monitors sit on the shared canvas. The mouse and keyboard cross linked edges onto it. It keeps showing its own desktop. No video is streamed.
- **Inside.** The device's screens are shown on another computer: a movable window (the default), fullscreen on a chosen host monitor, or on a specific host monitor. Moving the mouse into the region controls that device; leaving through the border returns control to the host. In fullscreen Inside, the exits are the linked edges of that host monitor.
- **Lent.** A physical monitor on device X becomes an extra monitor for device Y. Y creates a matching virtual display; X shows only that display, fullscreen, on the lent monitor.

Change a placement with one action in the editor or the tray. The existing connection is reused: Beside turns video off, Inside turns it on.

## Canvas editor

Monitors are drawn at their real physical size, in millimetres. Drag them to where they sit on the desk, including offsets, gaps, and different heights. Snapping helps; free placement always works.

Physical size is read from EDID (Windows) or `CGDisplayScreenSize` (macOS). Override a size if the reading is wrong.

Only edge segments that touch another screen are exits. Everything else is a wall. An Inside region is a rectangle inside its host monitor; its border is the way out.

Suggested gestures:

- Drop a device tile next to a monitor → Beside.
- Drop it into a monitor → Inside.
- Drop it onto "lend to" → Lent.

The editor can be opened from any desktop. The last writer wins. A device that was offline receives the latest layout when it reconnects.

## Profiles

A profile is the canvas and the placements for one exact set of online devices. When that combination appears again, the last layout for it is restored. A new combination starts from the closest earlier profile and is then saved on its own.

The main window shows which profile is active.

## Device list

Each device shows online, offline, or asleep (wakeable). Asleep or offline devices have a **Wake** action if a MAC address is stored.

## Per-device settings

- Placement and Inside mode.
- Audio: play on the device's own speakers (default for Beside), or forward to the controller. Inside forwards audio by default.
- Key mapping for each controller/target pair. Defaults:

  | Direction | Mapping |
  |---|---|
  | Mac → Windows | Cmd → Ctrl, Control → Win, Option → Alt |
  | Windows → Mac | Ctrl → Cmd, Win → Control, Alt → Option |

- Auto-connect after every start.
- MAC address and Wake.
- Jump hotkey: a shortcut that moves control to the centre of that device's primary monitor (also the way out if the cursor is stuck).
- Switch guards (each optional): corner dead zones, hold a modifier to cross, block crossing while the foreground app is fullscreen (with an ignore list).

## Tray / menu bar

Quick placement switching, Wake, and Open editor.

## Away from home

All traffic stays on Tailscale `100.x` addresses. From the Mac on another network, Inside sessions to `win1` and `win2` work the same as at home. From the phone on mobile data, wake `win1` if needed, then open it as a session.

## Android

The phone is a controller only. It lists devices and opens a fullscreen Inside session with RustDesk's touch or trackpad input. It is not part of the Beside canvas.

## Diagnostics

The Diagnostics view shows per-peer connection state and transport, measured input latency, the active profile, virtual display state, and recent errors. Export logs from there when something fails.
