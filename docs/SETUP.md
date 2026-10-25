# Setup

Per-device setup for q2crazyDesk. The app is for one person, on their own machines, over Tailscale.

## Devices

| ID | Machine | OS | Notes |
|---|---|---|---|
| `win1` | Main PC | Windows 11 Home | Two monitors, Ethernet, physical keyboard and mouse |
| `win2` | Second PC | Windows 10 Pro | One monitor, Ethernet, **no** keyboard or mouse |
| `mac` | MacBook Air M2 | macOS 26 Tahoe | Built-in display, Wi-Fi, built-in keyboard and trackpad |
| `phone` | Android | — | Controller only (Inside sessions) |

Keyboard layout: Swedish on every desktop.

## Tailscale

1. Create or sign in to a **personal** Tailscale account (Gmail or GitHub). A custom domain can be treated as a company account and leave the free Personal plan.
2. Install Tailscale on every device from https://tailscale.com/download and sign in to the same tailnet.
3. On Windows, enable **unattended mode** so Tailscale starts before login: Tailscale menu → Preferences → unattended / start at boot. Approve the UAC prompt.
4. In the [Machines](https://login.tailscale.com/admin/machines) admin console, open the menu on `win1` and `win2` and choose **Disable key expiry**. Keys otherwise expire after 180 days and you would have to sit at the PC to renew them. Do this on the always-on PCs only.
5. Confirm each desktop has a `100.x` address: `tailscale ip -4`.

If Tailscale is not running, q2crazyDesk shows an error and how to start it. An optional unencrypted LAN fallback exists and is **off** by default.

## Installing the app

Download the latest artifacts from the [q2crazyDesk CI](https://github.com/lindeb2/q2crazyDesk/actions/workflows/q2-ci.yml) run on `main`. There is no GitHub Release until `v1.0.0`. The green run `40147278687` has `q2crazydesk-windows-x86_64` (msi and portable exe), `q2crazydesk-macos-aarch64` (dmg), and `q2crazydesk-android-aarch64` (apk).

### Windows

- Prefer `q2crazydesk-<version>-x86_64.msi`. The installer is **unsigned**: SmartScreen will warn; choose More info → Run anyway.
- The portable `.exe` is for trying the UI without installing the service.
- The MSI installs the Windows service (needed for the login screen, UAC, and start-before-login) and, from phase 5, the signed Virtual-Display-Driver. Driver install needs administrator rights.

### macOS

1. Open `q2crazydesk-<version>-aarch64.dmg` and drag the app to `/Applications`.
2. Clear quarantine and give the bundle one signature. Without the second command, macOS aborts on launch with `different Team IDs` for `FlutterMacOS.framework`:

   ```bash
   xattr -cr /Applications/q2crazyDesk.app
   codesign --force --deep --sign - /Applications/q2crazyDesk.app
   open /Applications/q2crazyDesk.app
   ```

3. Grant permissions under System Settings → Privacy & Security:
   - Accessibility
   - Input Monitoring
   - Screen Recording
4. There is no paid Apple Developer account. Builds are ad-hoc or self-signed. After an update, macOS may forget the permissions: remove the app from each list and add it again.
5. Optional Wi-Fi stutter workaround (AWDL): `sudo ifconfig awdl0 down`. Documented because Synergy and Deskflow hit the same issue.

### Android

Install `q2crazydesk-<version>-aarch64.apk`. The APK is signed with the q2crazyDesk release key, so updates overwrite in place. Enable Tailscale on the phone (Wi-Fi or mobile data) before opening a session.

## Pairing

1. Start q2crazyDesk on two devices. Both must be signed in to the same Tailscale tailnet.
2. Open **Discovered** on each device. Right-click the other computer and choose **Pair device**.
3. Do that on **both** sides. A device that is not paired on the target is rejected. Remove a pair with **Unpair device**.
4. `win2` has no keyboard or mouse. Plug in a mouse once, pair from its Discovered menu, then unplug it.

## Wake-on-LAN

`win1` and `win2` are on Ethernet and must be wakeable from sleep, and from shutdown where the hardware allows. The Mac is on Wi-Fi: waking from sleep is best-effort; waking from shutdown is not possible.

### BIOS / UEFI (`win1`, `win2`)

Enable **Wake on LAN** / **PME** / **Power on by PCI-E**. The exact name varies by board.

### Windows adapter

1. Device Manager → Network adapters → the Ethernet adapter → Properties → Power Management.
2. Allow the computer to turn off the device to save power: optional.
3. Allow this device to wake the computer: on.
4. Only allow a magic packet to wake the computer: on.

### Windows Fast Startup

Fast Startup often blocks waking from shutdown. Turn it off:

1. Control Panel → Power Options → Choose what the power buttons do.
2. Change settings that are currently unavailable.
3. Uncheck **Turn on fast startup**.

### Mac energy setting

System Settings → Battery / Energy → enable **Wake for network access**. This only helps from sleep, on Wi-Fi, and is not reliable without a sleep proxy.

The GUI stores each device's Ethernet/Wi-Fi MAC address and offers a **Wake** action. On the home network the requesting device sends the magic packet. Away from home it asks an online home device to send it, over Tailscale.

## Building from source (`win1`)

Toolchain pins match RustDesk 1.5.0 CI (`docs/DECISIONS.md` D-003, D-004):

| Tool | Location |
|---|---|
| Rust 1.75 | `rustup` (`%USERPROFILE%\.cargo`) |
| Flutter 3.24.5 (patched, RustDesk engine) | `C:\dev\flutter` |
| LLVM 15.0.6 | `C:\dev\llvm` (`LIBCLANG_PATH=C:\dev\llvm\bin`) |
| vcpkg (commit `9e593bb18ea69cc5095e012465dcd675a822ed0d`) | `C:\dev\vcpkg` (`VCPKG_ROOT`) |
| Visual Studio 2022 Build Tools | `C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools` |

```bat
call "C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
set VCPKG_ROOT=C:\dev\vcpkg
set LIBCLANG_PATH=C:\dev\llvm\bin
python build.py --flutter --hwcodec --vram --skip-portable-pack
```

The app is `flutter\build\windows\x64\runner\Release\rustdesk.exe` until rebranding lands.

q2 unit tests (no Flutter):

```bat
cargo test --manifest-path q2/Cargo.toml --workspace
```

macOS and Android binaries come from GitHub Actions unless you set up those toolchains locally.
