# SKOOLchat Windows

A small Windows desktop app (Rust + WebView2) that shows https://beyou.figma.site
inside its own window. It does not open Chrome or Edge.

## Requirements

- Windows 10 or 11
- **WebView2 Runtime** (already included in Windows 11 and most up-to-date Windows 10).
  If missing: https://developer.microsoft.com/microsoft-edge/webview2/
- To build: **Rust** (https://rustup.rs) and the **Visual Studio Build Tools**
  with the "Desktop development with C++" workload (rustup will prompt for this).

## Build

1. Open the `SKOOLchat-Windows` folder.
2. Double-click `build.bat`.
3. Your app appears at `dist\SKOOLchat-Windows.exe`.

Or from a terminal:

```
cargo build --release
```

The exe is at `target\release\SKOOLchat-Windows.exe`.

## Install / run

The exe is standalone. Copy it anywhere and double-click it. To make a desktop
shortcut: right-click the exe, choose **Send to > Desktop (create shortcut)**.

## Behavior

- Opens maximized; resize by dragging edges; standard minimize/maximize/close buttons.
- The site resizes with the window.
- Login sessions are kept in `%LOCALAPPDATA%\SKOOLchatWindows`.
- If the site can't be reached at startup, an offline page with a **Try again** button is shown.

## Test checklist

- [ ] App starts with no console window
- [ ] Opens maximized
- [ ] Restore down, resize by dragging edges, re-maximize
- [ ] Site loads, scrolls, links/buttons/text fields work
- [ ] Disconnect internet, launch: offline page shows; reconnect, click Try again
- [ ] Close and reopen; you're still signed in (if the site supports it)

## Optional improvements (not in the basic build)

- **Icon / metadata:** add `app.ico`, add the `winres` crate as a build dependency and a `build.rs`
  that sets the icon, product name and version.
- **Installer:** use Inno Setup or WiX to wrap the exe and create a Start Menu/desktop shortcut.
- **Loading/splash screen:** show a small borderless window or HTML page while the site loads.

