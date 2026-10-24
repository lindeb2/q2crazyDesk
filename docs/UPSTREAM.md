# Upstream components

Every piece of third-party code in this repository, with its version, license, location and the local modifications ("patch list"). Keep this file current with every change to borrowed code; `docs/UPDATING.md` relies on it.

## RustDesk (base of the repository)

- **Source:** https://github.com/rustdesk/rustdesk
- **Version:** tag `1.5.0` (commit `fada664df`, 2026-09-30), full history kept; remote `upstream`.
- **Submodule:** `libs/hbb_common` at `229b904508364c8997aad0fb5af57effac859f60` (https://github.com/rustdesk/hbb_common).
- **License:** AGPL-3.0 (`LICENCE`).
- **What was taken:** everything; q2crazyDesk is a fork.
- **Where it lives:** the repository root (`src/`, `libs/`, `flutter/`, `res/`, ...). q2crazyDesk code lives in `q2/`, `third_party/` and `flutter/lib/q2/`.

### Patch list (edits to upstream files)

Keep every edit to an upstream file listed here, grouped by purpose. New q2crazyDesk files are not listed.

| # | Files | Change | Why |
|---|---|---|---|
| U-1 | `.github/workflows/{flutter-nightly,update-webpki-roots,fdroid,flutter-tag,flutter-ci,ci,playground}.yml` | Deleted | They would run RustDesk's release pipeline on this fork. Replaced by the new `.github/workflows/q2-ci.yml`. |
| U-2 | `README.md` | Replaced with the q2crazyDesk README | Product description and quick start. RustDesk's README is at https://github.com/rustdesk/rustdesk/blob/1.5.0/README.md. |
| U-3 | `.github/workflows/q2-ci.yml` | Added (new file; listed so updates are not mistaken for an upstream file) | Trimmed build of Windows x64, macOS arm64, Android arm64, plus `test-q2`. |
| U-4 | `src/lib.rs` | `mod q2_init;` | Load the q2 startup hook. |
| U-5 | `src/common.rs` | `q2_init::apply()` at the start of `global_init` | Set the product name and the default Tailscale IP whitelist. |
| U-6 | `flutter/android/app/src/main/res/values/strings.xml`, `flutter/macos/Runner/Configs/AppInfo.xcconfig` | Display name `q2crazyDesk` | Visible product name. Bundle id unchanged so updates keep data. |
| U-7 | `Cargo.toml` | Path dependency `q2-net` | Tailscale status parser. |
| U-8 | `src/lib.rs` | `mod q2_peers;` | Peer refresh module. |
| U-9 | `src/lan.rs` | `q2_peers::refresh()` at the start of `discover` | Discovered list includes Tailscale peers. |
| U-10 | `src/ui_interface.rs` | `online` field on lan peer maps | The peer card can show Tailscale online state. |
| U-11 | `flutter/lib/models/peer_model.dart`, `flutter/lib/desktop/pages/connection_page.dart` | Honour a payload `online` flag; show `q2-net-error` in the status line | R-DEV-4 and R-NET-5. |
| U-12 | `build.py` | `build_flutter_dmg` copies `service` into `*.app` | The macOS bundle name follows `PRODUCT_NAME` (`q2crazyDesk.app`). |

## LittleBigMouse (ported into `q2/layout`)

- **Source:** https://github.com/mgth/LittleBigMouse
- **Version:** master as of 2026-10-24; Rust daemon under `rust/crates` (merged via PR #491 in v5.4.0).
- **License:** GPL-3.0.
- **What was taken:** the millimetre canvas model and physical edge-crossing algorithm (ideas and structure, rewritten).
- **Where it lives:** `q2/layout`.
- **Patch list:** not a vendor tree; see file headers in that crate.

## Mouse Without Borders (ported behaviour into `q2/router`)

- **Source:** https://github.com/microsoft/PowerToys (Mouse Without Borders module)
- **Version:** studied 2026-10-24.
- **License:** MIT.
- **What was taken:** jump-hotkey, corner dead zone, fullscreen guard, and "draw cursor without a mouse" behaviour.
- **Where it lives:** `q2/router`.

## Not yet vendored

| Component | Planned path | When |
|---|---|---|
| Lan Mouse v0.11.0 | `third_party/lan-mouse` (git subtree) | Phase 2 |
| Virtual-Display-Driver signed release | CI download, checksum in this file | Phase 5 |
| DeskPad / VirtualDisplayKit shim | `third_party/macos-virtual-display` | Phase 5 |
