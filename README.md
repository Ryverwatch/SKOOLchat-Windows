# SKOOLchat — Your school. Your conversations.

SKOOLchat is for chatting with your friends at school. SKOOLchat Windows puts it
right on your desktop. Open it like any other app, with no browser tabs and no
address bar, and jump straight into the conversation.

Website: https://beyou.figma.site

---

## Get the app

1. Open the **Releases** page of this repository (right side of the main page).
2. Download **SKOOLchat-Windows.exe** from the newest release.
3. Double-click it. That's it. There's nothing to install.

**First time on Windows?** You may see "Windows protected your PC". Click
**More info**, then **Run anyway**. The app is safe; Windows just doesn't
recognize new apps yet.

**Needs:** Windows 10 or 11 with the Microsoft WebView2 Runtime. It's already
included on Windows 11 and up-to-date Windows 10. If it's missing, you can get it
free from https://developer.microsoft.com/microsoft-edge/webview2/

## What you get

- Opens full screen, ready to chat
- Resize it, minimize it, or snap it next to whatever else you're doing
- Stays signed in, so you don't log in every time
- Links to other websites open in your normal browser, so your chats stay put
- A friendly "Try again" screen if your internet drops

---

## For developers

SKOOLchat Windows is a small Rust app that shows the website in a WebView2 window.

**Build in the cloud (no setup):** every time you save a change to this repo,
GitHub Actions builds the app and publishes a new numbered release.

**Build on your own PC:** install Rust (https://rustup.rs) and run
`cargo build --release`, or double-click `build.bat`.

| File | What it does |
| --- | --- |
| `src/main.rs` | The app itself |
| `Cargo.toml` | Dependencies and release settings |
| `build.rs`, `app.rc`, `icon.ico` | Put the SKOOLchat icon on the exe |
| `icon_64.rgba` | Window and taskbar icon |
| `.github/workflows/build.yml` | The cloud build that makes each release |

Login data is stored in `%LOCALAPPDATA%\SKOOLchatWindows`.

**Your school. Your conversations.**
