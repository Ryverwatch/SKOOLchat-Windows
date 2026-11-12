// Hide the console window in release builds.
#![windows_subsystem = "windows"]

use std::net::{TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::time::Duration;

use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use wry::{WebContext, WebViewBuilder};

const APP_NAME: &str = "SKOOLchat Windows";
const SITE_URL: &str = "https://beyou.figma.site";
const SITE_HOST: &str = "beyou.figma.site:443";

#[derive(Debug)]
enum UserEvent {
    Retry,
    ShowOffline,
}

const OFFLINE_HTML: &str = r#"<!doctype html>
<html>
<head>
<meta charset="utf-8">
<title>SKOOLchat Windows</title>
<style>
  html, body { height: 100%; margin: 0; }
  body {
    display: flex; align-items: center; justify-content: center;
    font-family: "Segoe UI", sans-serif; background: #f5f5f7; color: #222;
  }
  .card { text-align: center; max-width: 420px; padding: 32px; }
  h1 { font-size: 24px; margin: 0 0 12px; }
  p { color: #555; line-height: 1.5; }
  button {
    margin-top: 16px; padding: 10px 24px; font-size: 16px;
    border: 0; border-radius: 8px; background: #4f46e5; color: #fff; cursor: pointer;
  }
  button:hover { background: #4338ca; }
</style>
</head>
<body>
  <div class="card">
    <h1>Can't connect</h1>
    <p>SKOOLchat Windows couldn't reach the website. Check your internet connection and try again.</p>
    <button onclick="window.ipc.postMessage('retry')">Try again</button>
  </div>
</body>
</html>"#;

/// Quick check that the site is reachable (DNS + TCP connect, 4 second timeout).
fn site_reachable() -> bool {
    match SITE_HOST.to_socket_addrs() {
        Ok(addrs) => addrs
            .into_iter()
            .any(|a| TcpStream::connect_timeout(&a, Duration::from_secs(4)).is_ok()),
        Err(_) => false,
    }
}

/// Where WebView2 keeps cookies/login sessions, so users stay signed in.
fn data_dir() -> PathBuf {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    base.join("SKOOLchatWindows")
}

/// Pull the host name out of a URL, e.g. "https://a.b.com:443/x" -> "a.b.com".
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://")?.1;
    let authority = rest.split(['/', '?', '#']).next()?;
    let authority = authority.rsplit('@').next()?;
    let host = authority.split(':').next()?;
    Some(host.to_lowercase())
}

/// Pages that stay inside the app. Everything else opens in the default browser.
fn is_internal(url: &str) -> bool {
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return true; // about:blank, data:, blob: etc. stay in the app
    }
    match host_of(url) {
        Some(h) => {
            h == "figma.site"
                || h.ends_with(".figma.site")
                || h == "figma.com"
                || h.ends_with(".figma.com")
        }
        None => true,
    }
}

/// Open a link in the user's default browser (no console window flashes).
fn open_in_default_browser(url: &str) {
    use std::os::windows::process::CommandExt;
    // Safety: only ever hand web, mail and phone links to Windows.
    let ok = url.starts_with("http://")
        || url.starts_with("https://")
        || url.starts_with("mailto:")
        || url.starts_with("tel:");
    if !ok {
        return;
    }
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let _ = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .creation_flags(CREATE_NO_WINDOW)
        .spawn();
}

/// Window/taskbar icon (64x64 raw RGBA baked into the exe).
fn window_icon() -> Option<tao::window::Icon> {
    const RGBA: &[u8] = include_bytes!("../icon_64.rgba");
    tao::window::Icon::from_rgba(RGBA.to_vec(), 64, 64).ok()
}


const LINK_SCRIPT: &str = r#"
(function () {
  function external(u) {
    try {
      var x = new URL(u, location.href);
      if (['http:', 'https:', 'mailto:', 'tel:'].indexOf(x.protocol) < 0) return null;
      var h = x.hostname.toLowerCase();
      if (h === 'figma.site' || h.endsWith('.figma.site') ||
          h === 'figma.com' || h.endsWith('.figma.com')) return null;
      return x.href;
    } catch (e) { return null; }
  }
  function web(u) {
    try {
      var x = new URL(u, location.href);
      if (['http:', 'https:', 'mailto:', 'tel:'].indexOf(x.protocol) < 0) return null;
      return x.href;
    } catch (e) { return null; }
  }
  document.addEventListener('click', function (e) {
    var a = e.target && e.target.closest ? e.target.closest('a[href]') : null;
    if (!a) return;
    var u = external(a.href);
    if (!u && a.target === '_blank') u = web(a.href);
    if (u) {
      e.preventDefault();
      e.stopPropagation();
      window.ipc.postMessage('open:' + u);
    }
  }, true);
  var realOpen = window.open;
  window.open = function (u) {
    var x = u ? web(u) : null;
    if (x) { window.ipc.postMessage('open:' + x); return null; }
    return null;
  };
})();
"#;

/// Check the connection in the background; show the offline page only if it fails.
fn check_in_background(proxy: tao::event_loop::EventLoopProxy<UserEvent>) {
    std::thread::spawn(move || {
        if !site_reachable() {
            let _ = proxy.send_event(UserEvent::ShowOffline);
        }
    });
}

fn main() {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    let ipc_proxy = proxy.clone();

    // Start hidden so the user never sees an empty white window while things load.
    let window = WindowBuilder::new()
        .with_title(APP_NAME)
        .with_maximized(true)
        .with_resizable(true)
        .with_visible(false)
        .with_window_icon(window_icon())
        .with_min_inner_size(tao::dpi::LogicalSize::new(480.0, 360.0))
        .build(&event_loop)
        .expect("Failed to create window");

    let mut context = WebContext::new(Some(data_dir()));

    // Load the site immediately (no blocking network check before startup).
    let webview = WebViewBuilder::with_web_context(&mut context)
        .with_url(SITE_URL)
        .with_background_color((255, 255, 255, 255))
        .with_navigation_handler(|url: String| {
            if is_internal(&url) {
                true
            } else {
                open_in_default_browser(&url);
                false
            }
        })
        .with_new_window_req_handler(|url: String| {
            // Never open a second app window: send it to the default browser.
            open_in_default_browser(&url);
            false
        })
        .with_initialization_script(LINK_SCRIPT)
        .with_ipc_handler(move |req| {
            let body = req.body();
            if body == "retry" {
                let _ = ipc_proxy.send_event(UserEvent::Retry);
            } else if let Some(url) = body.strip_prefix("open:") {
                open_in_default_browser(url);
            }
        })
        .build(&window)
        .expect(
            "Failed to start WebView2. Install the WebView2 Runtime from \
             https://developer.microsoft.com/microsoft-edge/webview2/",
        );

    window.set_visible(true);
    window.set_maximized(true);

    // Connection check runs in parallel while the page loads.
    check_in_background(proxy.clone());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Retry) => {
                let _ = webview.load_url(SITE_URL);
                check_in_background(proxy.clone());
            }
            Event::UserEvent(UserEvent::ShowOffline) => {
                let _ = webview.load_html(OFFLINE_HTML);
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            _ => {}
        }
    });
}
