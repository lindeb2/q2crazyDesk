# Third-party notices

q2crazyDesk is a fork of RustDesk and includes other open-source components. This file lists the licenses of borrowed code. Versions, paths and local patches are in `docs/UPSTREAM.md`.

## RustDesk

Copyright (c) Purslane Ltd. and RustDesk contributors.

Licensed under the GNU Affero General Public License, version 3 (`LICENCE` in this repository). The full text is also at https://www.gnu.org/licenses/agpl-3.0.html.

## RustDesk dependencies

RustDesk's own `Cargo.toml`, `flutter/pubspec.yaml`, and vendored crates under `libs/` carry their original licenses (MIT, Apache-2.0, BSD, Zlib, MPL-2.0, and others). Those notices stay with the files.

## LittleBigMouse (ported geometry)

Copyright (c) LittleBigMouse contributors.

The millimetre canvas and edge-crossing model in `q2/layout` is a port of ideas and algorithms from https://github.com/mgth/LittleBigMouse (GPL-3.0), in particular the Rust hook daemon under `rust/crates`. GPL-3.0 is compatible with AGPL-3.0. The original license text: https://github.com/mgth/LittleBigMouse/blob/master/LICENSE.

## Mouse Without Borders (ported behaviour)

Copyright (c) Microsoft Corporation.

Jump hotkeys, corner dead zones, the fullscreen crossing guard, and "draw the cursor when no mouse is attached" follow PowerToys Mouse Without Borders (MIT). https://github.com/microsoft/PowerToys

## Lan Mouse (vendored in phase 2)

Copyright (c) Ferdinand Schober and contributors.

`input-capture`, `input-emulation`, and `input-event` are GPL-3.0-or-later. They will live in `third_party/lan-mouse` once vendored. https://github.com/feschber/lan-mouse

## Virtual-Display-Driver (shipped signed binary, phase 5)

Copyright (c) VirtualDrivers contributors.

MIT. The signed release is downloaded by the installer / CI, not stored in git. https://github.com/VirtualDrivers/Virtual-Display-Driver

## DeskPad / VirtualDisplayKit (macOS shim, phase 5)

Copyright (c) DeskPad and VirtualDisplayKit contributors.

MIT. The Swift shim will live in `third_party/macos-virtual-display`. https://github.com/Stengo/DeskPad https://github.com/xocialize/VirtualDisplayKit

## ShareCursor (study for drag-and-drop, phase 6)

Copyright (c) ShareCursor contributors.

MIT or Apache-2.0. Studied for the live file-drag path; not vendored unless a later decision copies specific files. https://github.com/phun333/ShareCursor
