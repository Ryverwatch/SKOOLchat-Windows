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
        .with_min_inner_size(tao::dpi::LogicalSize::new(480.0, 360.0))
        .build(&event_loop)
        .expect("Failed to create window");

    let mut context = WebContext::new(Some(data_dir()));

    // Load the site immediately (no blocking network check before startup).
    let webview = WebViewBuilder::new_with_web_context(&mut context)
        .with_url(SITE_URL)
        .with_background_color((255, 255, 255, 255))
        .with_ipc_handler(move |req| {
            if req.body() == "retry" {
                let _ = ipc_proxy.send_event(UserEvent::Retry);
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
