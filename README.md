# q2crazyDesk

q2crazyDesk lets one person use several computers as if they were one desk. Every computer has a placement in a shared layout of all monitors, drawn in millimetres at their real physical size and position, and a computer's placement can be switched with one action:

- **Beside** (software KVM): the computer's own monitors sit next to the others. The mouse and keyboard cross screen edges to it, and it keeps showing its own desktop.
- **Inside** (remote desktop): the computer's screen is shown inside another computer's monitor, as a window, in fullscreen, or on a monitor you pick. Moving the mouse into it controls that computer; moving out through an edge leaves it.
- **Lent** (extra monitor): a computer's physical monitor becomes an extra monitor for another computer, through a virtual display.

Clipboard, files (including drag-and-drop across edges) and audio work across all of this. The app starts before login, reconnects by itself, can wake sleeping PCs, and works at home and away over [Tailscale](https://tailscale.com).

Platforms: Windows 10/11 (x64), macOS (Apple silicon) and Android (arm64, as a controller that opens remote sessions).

q2crazyDesk is a personal project. It is a fork of [RustDesk](https://github.com/rustdesk/rustdesk) and is assembled mainly from existing open-source projects; see [docs/SOURCING.md](docs/SOURCING.md) and [docs/UPSTREAM.md](docs/UPSTREAM.md).

## Quick start

1. Install Tailscale on every device and sign in to the same tailnet ([docs/SETUP.md](docs/SETUP.md#tailscale)).
2. Download the installer for each device from the [latest release](https://github.com/lindeb2/q2crazyDesk/releases/latest):
   - Windows: `q2crazydesk-<version>-x86_64.msi` (or the portable `.exe`).
   - macOS: `q2crazydesk-<version>-aarch64.dmg`, then follow the quarantine and permission steps in [docs/SETUP.md](docs/SETUP.md#macos).
   - Android: `q2crazydesk-<version>-aarch64.apk`.
3. Start q2crazyDesk on two devices and pair them; approve the pairing on both ([docs/SETUP.md](docs/SETUP.md#pairing)).
4. Open the canvas editor, drag the monitors to where they physically are, and choose a placement for each device ([docs/USER_GUIDE.md](docs/USER_GUIDE.md)).

## Documentation

| Document | Contents |
|---|---|
| [docs/SETUP.md](docs/SETUP.md) | Per-device setup: Tailscale, installing, macOS permissions, Wake-on-LAN, pairing |
| [docs/USER_GUIDE.md](docs/USER_GUIDE.md) | Placements, canvas editor, profiles, hotkeys, audio, key mapping, Wake, remote use |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Components, data flow, transport decision and latency, drag-and-drop, Lent pipeline |
| [docs/SOURCING.md](docs/SOURCING.md) | Which open-source projects were considered and chosen for each feature |
| [docs/UPSTREAM.md](docs/UPSTREAM.md) | Every piece of third-party code, its version, license and local patches |
| [docs/UPDATING.md](docs/UPDATING.md) | How to merge a new RustDesk release and update vendored code |
| [docs/DECISIONS.md](docs/DECISIONS.md) | Decisions and verified assumptions |
| [docs/MANUAL_TESTS.md](docs/MANUAL_TESTS.md) | Physical test checklist and results |
| [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) | Licenses of borrowed code |

## Building

CI builds all three platforms on every push (`.github/workflows/q2-ci.yml`). For a local Windows build, see [docs/SETUP.md](docs/SETUP.md#building-from-source).

## License

AGPL-3.0, like RustDesk (see [LICENCE](LICENCE)). Borrowed code keeps its original license; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
