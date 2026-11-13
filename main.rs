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
    WinCmd(String),
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


const DEVICE_SCRIPT: &str = r#"
(function () {
  if (window.top !== window) return;
  var MIC = 'skc_mic', SPK = 'skc_spk';
  function get(k) { try { return localStorage.getItem(k) || ''; } catch (e) { return ''; } }
  function put(k, v) { try { localStorage.setItem(k, v); } catch (e) {} }

  // Use the chosen microphone whenever the site asks for audio input.
  var md = navigator.mediaDevices;
  var realGUM = null;
  if (md && md.getUserMedia) {
    realGUM = md.getUserMedia.bind(md);
    md.getUserMedia = function (orig) {
      var c = orig;
      try {
        var id = get(MIC);
        if (id && c && c.audio) {
          var a = c.audio === true ? {} : Object.assign({}, c.audio);
          if (!a.deviceId) a.deviceId = { exact: id };
          c = Object.assign({}, c, { audio: a });
        }
      } catch (e) {}
      return realGUM(c).catch(function (err) {
        if (err && err.name === 'OverconstrainedError') return realGUM(orig);
        throw err;
      });
    };
  }

  // Send sound to the chosen speakers.
  function applySink(el) {
    var id = get(SPK);
    if (id && el && el.setSinkId && el.sinkId !== id) {
      try { el.setSinkId(id).catch(function () {}); } catch (e) {}
    }
  }
  ['play', 'loadedmetadata'].forEach(function (ev) {
    document.addEventListener(ev, function (e) { applySink(e.target); }, true);
  });
  var AC = window.AudioContext;
  if (AC && AC.prototype.setSinkId) {
    var realResume = AC.prototype.resume;
    AC.prototype.resume = function () {
      var id = get(SPK);
      if (id) { try { this.setSinkId(id); } catch (e) {} }
      return realResume.apply(this, arguments);
    };
  }

  function build() {
    if (document.getElementById('skc-bar') || !document.body) return;
    var H = 34;

    // Top bar with the app name and the two audio buttons.
    var bar = document.createElement('div');
    bar.id = 'skc-bar';
    bar.style.cssText = 'position:fixed;top:0;left:0;right:0;height:' + H + 'px;z-index:2147483647;' +
      'display:flex;align-items:center;gap:10px;padding:0 12px;box-sizing:border-box;' +
      'background:#111;color:#fff;font:600 14px Segoe UI,Arial,sans-serif;';
    var title = document.createElement('span');
    title.textContent = 'SKOOLchat Windows';
    title.style.cssText = 'margin-right:6px;';
    function mk(text) {
      var b = document.createElement('button');
      b.textContent = text;
      b.style.cssText = 'border:0;border-radius:8px;padding:5px 10px;background:#2b2b2b;color:#fff;' +
        'font:13px Segoe UI,Arial,sans-serif;cursor:pointer;';
      return b;
    }
    var micBtn = mk('\u{1F3A4} Microphone');
    var spkBtn = mk('\u{1F50A} Speakers');
    bar.appendChild(title);
    bar.appendChild(micBtn);
    bar.appendChild(spkBtn);

    // Window controls (the native title bar is hidden).
    function send(m) { try { window.ipc.postMessage(m); } catch (e) {} }
    var ctl = document.createElement('div');
    ctl.style.cssText = 'margin-left:auto;margin-right:-12px;display:flex;height:100%;';
    function cbtn(text, cmd, hover) {
      var b = document.createElement('button');
      b.textContent = text;
      b.style.cssText = 'width:46px;height:100%;border:0;background:transparent;color:#fff;' +
        'font:14px Segoe UI,Arial,sans-serif;cursor:pointer;';
      b.addEventListener('mouseenter', function () { b.style.background = hover; });
      b.addEventListener('mouseleave', function () { b.style.background = 'transparent'; });
      b.addEventListener('click', function () { send('win:' + cmd); });
      ctl.appendChild(b);
    }
    cbtn('–', 'min', '#333');
    cbtn('□', 'max', '#333');
    cbtn('✕', 'close', '#c42b1c');
    bar.appendChild(ctl);

    // Drag the window by the bar, double-click to maximize/restore.
    bar.addEventListener('mousedown', function (e) {
      if (e.button === 0 && (e.target === bar || e.target === title)) send('win:drag');
    });
    bar.addEventListener('dblclick', function (e) {
      if (e.target === bar || e.target === title) send('win:max');
    });
    document.body.appendChild(bar);

    // Invisible edge handles so the window can still be resized.
    function isMax() {
      return window.innerWidth >= screen.availWidth - 2 && window.innerHeight >= screen.availHeight - 2;
    }
    [['n', 'top:0;left:8px;right:8px;height:5px;cursor:ns-resize'],
     ['s', 'bottom:0;left:8px;right:8px;height:5px;cursor:ns-resize'],
     ['w', 'left:0;top:8px;bottom:8px;width:5px;cursor:ew-resize'],
     ['e', 'right:0;top:8px;bottom:8px;width:5px;cursor:ew-resize'],
     ['nw', 'top:0;left:0;width:8px;height:8px;cursor:nwse-resize'],
     ['se', 'bottom:0;right:0;width:8px;height:8px;cursor:nwse-resize'],
     ['ne', 'top:0;right:0;width:8px;height:8px;cursor:nesw-resize'],
     ['sw', 'bottom:0;left:0;width:8px;height:8px;cursor:nesw-resize']
    ].forEach(function (h) {
      var d = document.createElement('div');
      d.style.cssText = 'position:fixed;z-index:2147483647;' + h[1];
      d.addEventListener('mousedown', function (e) {
        if (e.button === 0 && !isMax()) { e.preventDefault(); send('win:resize:' + h[0]); }
      });
      document.body.appendChild(d);
    });

    // Push the page down so the bar doesn't cover it.
    document.documentElement.style.setProperty('padding-top', H + 'px', 'important');

    var menu = document.createElement('div');
    menu.style.cssText = 'position:fixed;top:' + (H + 4) + 'px;z-index:2147483647;min-width:260px;' +
      'max-width:380px;max-height:60vh;overflow:auto;padding:6px;border-radius:10px;background:#fff;' +
      'color:#111;font:14px Segoe UI,Arial,sans-serif;box-shadow:0 6px 24px rgba(0,0,0,.35);display:none;';
    document.body.appendChild(menu);
    var current = null;

    function closeMenu() { menu.style.display = 'none'; current = null; }

    function openMenu(kind, anchor) {
      if (current === kind) { closeMenu(); return; }
      current = kind;
      var isMic = kind === 'mic';
      var key = isMic ? MIC : SPK;
      menu.style.left = Math.max(4, anchor.getBoundingClientRect().left) + 'px';
      menu.style.display = 'block';
      menu.textContent = 'Loading devices...';

      // Ask for mic access once so Windows shows the real device names.
      var unlock = realGUM ? realGUM({ audio: true }).then(function (st) {
        st.getTracks().forEach(function (t) { t.stop(); });
      }).catch(function () {}) : Promise.resolve();

      unlock.then(function () {
        return md && md.enumerateDevices ? md.enumerateDevices() : [];
      }).then(function (devs) {
        if (current !== kind) return;
        var list = devs.filter(function (d) { return d.kind === (isMic ? 'audioinput' : 'audiooutput'); });
        var saved = get(key);
        menu.textContent = '';
        function item(label, id) {
          var b = document.createElement('button');
          var on = id === saved;
          b.textContent = (on ? '\u2713  ' : '\u00A0\u00A0\u00A0\u00A0') + label;
          b.style.cssText = 'display:block;width:100%;text-align:left;border:0;border-radius:6px;' +
            'padding:8px 10px;cursor:pointer;font:14px Segoe UI,Arial,sans-serif;color:#111;' +
            'background:' + (on ? '#e8e8ff' : 'transparent') + ';';
          b.addEventListener('mouseenter', function () { if (!on) b.style.background = '#f1f1f1'; });
          b.addEventListener('mouseleave', function () { if (!on) b.style.background = 'transparent'; });
          b.addEventListener('click', function () {
            put(key, id);
            if (!isMic) document.querySelectorAll('audio,video').forEach(applySink);
            closeMenu();
          });
          menu.appendChild(b);
        }
        item('System default', '');
        list.forEach(function (d, i) {
          item(d.label || ((isMic ? 'Microphone ' : 'Speaker ') + (i + 1)), d.deviceId);
        });
        if (!list.length) {
          var n = document.createElement('div');
          n.textContent = isMic ? 'No microphones found. Allow microphone access and try again.'
                                : 'No speaker list available.';
          n.style.cssText = 'padding:8px 10px;color:#666;font-size:12px;';
          menu.appendChild(n);
        }
        if (isMic) {
          var t = document.createElement('div');
          t.textContent = 'Microphone changes apply the next time a call starts.';
          t.style.cssText = 'padding:6px 10px 2px;color:#666;font-size:12px;';
          menu.appendChild(t);
        }
      });
    }

    micBtn.addEventListener('click', function () { openMenu('mic', micBtn); });
    spkBtn.addEventListener('click', function () { openMenu('spk', spkBtn); });
    document.addEventListener('mousedown', function (e) {
      if (current && !menu.contains(e.target) && e.target !== micBtn && e.target !== spkBtn) closeMenu();
    }, true);
  }

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', build);
  else build();
})();
"#;

/// Map the short names sent by the page to window resize directions.
fn resize_dir(s: &str) -> Option<tao::window::ResizeDirection> {
    use tao::window::ResizeDirection as R;
    Some(match s {
        "n" => R::North,
        "s" => R::South,
        "e" => R::East,
        "w" => R::West,
        "ne" => R::NorthEast,
        "nw" => R::NorthWest,
        "se" => R::SouthEast,
        "sw" => R::SouthWest,
        _ => return None,
    })
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
    // The build number is stamped in by the cloud build so you can tell which version is running.
    let title = format!(
        "{} (build {})",
        APP_NAME,
        option_env!("BUILD_NUMBER").unwrap_or("local")
    );

    let window = WindowBuilder::new()
        .with_title(title)
        .with_maximized(true)
        .with_resizable(true)
        .with_visible(false)
        .with_decorations(false)
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
        .with_initialization_script(DEVICE_SCRIPT)
        .with_ipc_handler(move |req| {
            let body = req.body();
            if body == "retry" {
                let _ = ipc_proxy.send_event(UserEvent::Retry);
            } else if let Some(url) = body.strip_prefix("open:") {
                open_in_default_browser(url);
            } else if let Some(cmd) = body.strip_prefix("win:") {
                let _ = ipc_proxy.send_event(UserEvent::WinCmd(cmd.to_string()));
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
            Event::UserEvent(UserEvent::WinCmd(cmd)) => match cmd.as_str() {
                "min" => window.set_minimized(true),
                "max" => window.set_maximized(!window.is_maximized()),
                "close" => *control_flow = ControlFlow::Exit,
                "drag" => {
                    let _ = window.drag_window();
                }
                c if c.starts_with("resize:") => {
                    if let Some(d) = resize_dir(&c[7..]) {
                        let _ = window.drag_resize_window(d);
                    }
                }
                _ => {}
            },
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
